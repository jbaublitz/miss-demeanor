mod listener;
mod tcp;
mod unix;

use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    error::Error,
    ffi::CString,
    fmt::Debug,
    hash::Hash,
    io,
    marker::Unpin,
    sync::Arc,
};

use futures::stream::StreamExt;
use hyper::{body::to_bytes, server::conn::Http, service, Body, Request, Response};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpStream, UnixStream},
};
use tokio_native_tls::TlsAcceptor;
use tokio_stream::wrappers::{TcpListenerStream, UnixListenerStream};

use missdemeanor::CRequest;

use crate::{
    config::{self, Server, TomlConfig},
    err::DemeanorError,
    plugins::{NewPlugin, Plugin, PluginError},
    webhook::listener::Listener,
};

pub enum UseTls {
    Yes(TlsIdentity),
    No,
}

impl UseTls {
    pub fn use_tls(&self) -> bool {
        matches!(self, UseTls::Yes(_))
    }
}

#[derive(Clone)]
pub struct TlsIdentity {
    identity: Vec<u8>,
    pw: String,
}

impl TlsIdentity {
    pub fn new(identity: Vec<u8>, pw: String) -> Self {
        TlsIdentity { identity, pw }
    }

    pub fn to_identity(&self) -> Result<native_tls::Identity, native_tls::Error> {
        native_tls::Identity::from_pkcs12(&self.identity, &self.pw)
    }
}

pub struct WebhookServer<P> {
    identity: Option<TlsIdentity>,
    server: Arc<Server>,
    triggers: Arc<HashSet<P>>,
}

impl<P> WebhookServer<P>
where
    P: 'static + NewPlugin + Plugin + Eq + Hash + Borrow<String> + Send + Sync,
{
    pub fn new(use_tls: UseTls, mut toml_config: TomlConfig) -> Result<Self, Box<dyn Error>> {
        let identity = match use_tls {
            UseTls::Yes(identity) => Some(identity),
            UseTls::No => None,
        };

        let mut trigger_plugins_hs = HashSet::new();
        for trigger in toml_config.triggers.drain() {
            trigger_plugins_hs.insert(P::new(trigger)?);
        }

        let trigger_plugins = Arc::new(trigger_plugins_hs);

        Ok(WebhookServer {
            identity,
            server: Arc::new(toml_config.server),
            triggers: trigger_plugins,
        })
    }

    async fn service(
        req: Request<Body>,
        server_box: Arc<Server>,
        trigger_plugins_box: Arc<HashSet<P>>,
    ) -> Result<Response<Body>, PluginError> {
        let (parts, body) = req.into_parts();
        let body = to_bytes(body).await.map_err(|e| {
            error!("{}", e);
            PluginError::new(500, "Failed to get request body")
        })?;
        let method = CString::new(parts.method.as_str()).map_err(|e| {
            error!("{}", e);
            PluginError::new(400, "Invalid method")
        })?;

        let uri = parts.uri.to_string();
        let name = server_box
            .endpoints
            .get(&uri)
            .map(|e| &e.trigger_name)
            .ok_or_else(|| {
                error!("Failed to find endpoint");
                PluginError::new(404, "Endpoint not found")
            })?;
        let uri_cstring = CString::new(uri).map_err(|e| {
            error!("{}", e);
            PluginError::new(400, "Invalid path")
        })?;

        let mut headers = HashMap::new();
        for (header, value) in &parts.headers {
            let header_cstring = CString::new(header.to_string()).map_err(|e| {
                error!("{}", e);
                PluginError::new(400, "Invalid header")
            })?;
            let val_str = value.to_str().map_err(|e| {
                error!("{}", e);
                PluginError::new(400, "Invalid header value")
            })?;
            let val_cstring = CString::new(val_str).map_err(|e| {
                error!("{}", e);
                PluginError::new(400, "Invalid header value")
            })?;
            headers.insert(header_cstring, val_cstring);
        }

        let body_cstring = CString::new(body.to_vec()).map_err(|e| {
            error!("{}", e);
            PluginError::new(400, "Invalid body")
        })?;

        let crequest = CRequest {
            method,
            uri: uri_cstring,
            headers,
            body: body_cstring,
        };

        let trigger = trigger_plugins_box.get(name).ok_or_else(|| {
            error!("Trigger plugin {} not found", name);
            PluginError::new(500, "Plugin not found")
        })?;
        trigger.run_trigger(crequest).map_err(|e| {
            error!("Trigger plugin failed with error: {}", e);
            PluginError::new(500, "Trigger phase failed")
        })?;

        Ok(Response::new(Body::from("Success!")))
    }

    async fn listen<L, C, E>(self) -> Result<(), Box<dyn Error>>
    where
        L: 'static + Listener<C, E> + Send,
        C: 'static + AsyncRead + AsyncWrite + Debug + Send + Unpin,
        E: 'static + Error + Send,
    {
        let mut tls_acceptor = None;
        if let Some(ident) = self.identity.as_ref() {
            let acceptor = native_tls::TlsAcceptor::new(ident.to_identity()?)?;
            tls_acceptor = Some(TlsAcceptor::from(acceptor));
        }

        let listener = L::bind(&self.server.listen_addr).await?;

        let server_for_each = Arc::clone(&self.server);
        let trigger_plugins_for_each = Arc::clone(&self.triggers);
        let tls_acceptor_for_each = Arc::new(tls_acceptor);

        listener
            .for_each(move |sock_result| {
                let server_serve = Arc::clone(&server_for_each);
                let trigger_plugins_serve = Arc::clone(&trigger_plugins_for_each);
                let tls_acceptor_inner = Arc::clone(&tls_acceptor_for_each);

                async move {
                    let sock = match sock_result {
                        Ok(s) => s,
                        Err(e) => {
                            error!("{}", e);
                            return;
                        }
                    };

                    if let Some(ref acceptor) = *tls_acceptor_inner {
                        let tls_stream = match acceptor.accept(sock).await {
                            Ok(ts) => ts,
                            Err(e) => {
                                error!("{}", e);
                                return;
                            }
                        };
                        let _ = tokio::spawn(Http::new().serve_connection(
                            tls_stream,
                            service::service_fn(move |req| {
                                let server_service = Arc::clone(&server_serve);
                                let trigger_plugins_service = Arc::clone(&trigger_plugins_serve);
                                async move {
                                    let response: Result<Response<Body>, io::Error> =
                                        match Self::service(
                                            req,
                                            server_service,
                                            trigger_plugins_service,
                                        )
                                        .await
                                        {
                                            Ok(resp) => Ok(resp),
                                            Err(e) => Ok(e.into_response()),
                                        };
                                    response
                                }
                            }),
                        ));
                    } else {
                        let _ = tokio::spawn(Http::new().serve_connection(
                            sock,
                            service::service_fn(move |req| {
                                let server_service = Arc::clone(&server_serve);
                                let trigger_plugins_service = Arc::clone(&trigger_plugins_serve);
                                async move {
                                    let response: Result<Response<Body>, io::Error> =
                                        match Self::service(
                                            req,
                                            server_service,
                                            trigger_plugins_service,
                                        )
                                        .await
                                        {
                                            Ok(resp) => Ok(resp),
                                            Err(e) => Ok(e.into_response()),
                                        };
                                    response
                                }
                            }),
                        ));
                    }
                }
            })
            .await;
        Ok(())
    }

    pub async fn serve(self) -> Result<(), Box<dyn Error>> {
        match self.server.server_type {
            config::ServerType::Webhook => {
                self.listen::<TcpListenerStream, TcpStream, io::Error>()
                    .await?
            }
            config::ServerType::UnixSocket => {
                self.listen::<UnixListenerStream, UnixStream, io::Error>()
                    .await?
            }
            config::ServerType::UnknownServerType => {
                return Err(Box::new(DemeanorError::new(
                    "Server type not recognized - exiting",
                )));
            }
        };
        Ok(())
    }
}
