mod listener;
mod tcp;
mod unix;

use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    convert::Infallible,
    error::Error,
    ffi::CString,
    fmt::Debug,
    future::Future,
    hash::Hash,
    io,
    marker::Unpin,
    pin::Pin,
    sync::Arc,
};

use futures::StreamExt;
use http_body_util::{BodyExt, Full};
use hyper::{
    body::{Bytes, Incoming},
    server::conn::http2::Builder,
    service::Service,
    Request, Response,
};
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

async fn service<P>(
    req: Request<Incoming>,
    server_box: Arc<Server>,
    trigger_plugins_box: Arc<HashSet<P>>,
) -> Result<Response<Full<Bytes>>, PluginError>
where
    P: Hash + Eq + Borrow<String> + Plugin,
{
    let (parts, body) = req.into_parts();
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

    let body_cstring = CString::new(match body.collect().await {
        Ok(b) => b.to_bytes().to_vec(),
        Err(e) => {
            warn!("{e}");
            return Err(PluginError::new(400, "Failed to receive body"));
        }
    })
    .map_err(|e| {
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

    Ok(Response::new(Full::new(Bytes::from("Success!"))))
}

struct WebookService<P> {
    server: Arc<Server>,
    plugins: Arc<HashSet<P>>,
}

impl<P> Service<Request<Incoming>> for WebookService<P>
where
    P: 'static + NewPlugin + Plugin + Eq + Hash + Borrow<String> + Send + Sync,
{
    type Response = Response<Full<Bytes>>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        let server = Arc::clone(&self.server);
        let plugins = Arc::clone(&self.plugins);
        Box::pin(async {
            match service(req, server, plugins).await {
                Ok(resp) => Ok(resp),
                Err(e) => Ok(e.into_response()),
            }
        })
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
                        let _ = tokio::spawn(
                            Builder::new(hyper_util::rt::TokioExecutor::new()).serve_connection(
                                hyper_util::rt::TokioIo::new(tls_stream),
                                WebookService {
                                    plugins: Arc::clone(&trigger_plugins_serve),
                                    server: Arc::clone(&server_serve),
                                },
                            ),
                        );
                    } else {
                        let _ = tokio::spawn(
                            Builder::new(hyper_util::rt::TokioExecutor::new()).serve_connection(
                                hyper_util::rt::TokioIo::new(sock),
                                WebookService {
                                    plugins: Arc::clone(&trigger_plugins_serve),
                                    server: Arc::clone(&server_serve),
                                },
                            ),
                        );
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
