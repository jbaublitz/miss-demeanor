use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::ffi::CString;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::io;
use std::sync::Arc;

use hyper::server::conn::Http;
use hyper::service;
use hyper::{Body, Request, Response};
use missdemeanor::CRequest;
use native_tls;
use tokio;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{self, TcpStream, UnixStream};
use tokio::prelude::{Future, Stream};
use tokio_tls::TlsAcceptor;

use config::{self, Server, TomlConfig};
use err::DemeanorError;
use plugins::{NewPlugin, Plugin, PluginError};

mod listener;
use self::listener::Listener;

mod tcp;
use self::tcp::TcpListener;

mod unix;
use self::unix::UnixListener;

pub enum UseTls {
    Yes(TlsIdentity),
    No,
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

    fn service(
        req: Request<Body>,
        server_box: Arc<Server>,
        trigger_plugins_box: Arc<HashSet<P>>,
    ) -> Box<dyn Future<Item = Response<Body>, Error = PluginError> + Send> {
        let (parts, body) = req.into_parts();
        Box::new(
            body.concat2()
                .map_err(|e| PluginError::new(500, e))
                .and_then(move |b| {
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

                    let body = CString::new(b.to_vec()).map_err(|e| {
                        error!("{}", e);
                        PluginError::new(400, "Invalid body")
                    })?;

                    let crequest = CRequest {
                        method,
                        uri: uri_cstring,
                        headers,
                        body,
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
                })
                .or_else(|e| Ok(e.into_response())),
        )
    }

    fn listen<L, S, C, E>(self) -> Result<(), Box<dyn Error>>
    where
        L: Listener<S, C, E>,
        C: 'static + AsyncRead + AsyncWrite + Debug + Send,
        S: 'static + Stream<Item = C> + Send,
        S::Error: Display + Send,
        E: 'static + Error + Send,
    {
        let mut tls_acceptor = None;
        if let Some(ident) = self.identity.as_ref() {
            let acceptor = native_tls::TlsAcceptor::new(ident.to_identity()?)?;
            tls_acceptor = Some(TlsAcceptor::from(acceptor));
        }

        let listener = L::bind(&self.server.listen_addr)?;

        tokio::run(
            listener
                .for_each(move |sock| {
                    let server_spawn = Arc::clone(&self.server);
                    let trigger_plugins_spawn = Arc::clone(&self.triggers);
                    if let Some(ref mut acceptor) = tls_acceptor {
                        tokio::spawn(
                            acceptor
                                .accept(sock)
                                .map_err(|e| {
                                    error!("{}", e);
                                })
                                .and_then(|tls_stream| {
                                    Http::new()
                                        .serve_connection(
                                            tls_stream,
                                            service::service_fn(move |req| {
                                                let server_box = Arc::clone(&server_spawn);
                                                let trigger_plugins_box =
                                                    Arc::clone(&trigger_plugins_spawn);

                                                Self::service(req, server_box, trigger_plugins_box)
                                            }),
                                        )
                                        .map_err(|e| {
                                            error!("{}", e);
                                        })
                                }),
                        );
                    } else {
                        tokio::spawn(
                            Http::new()
                                .serve_connection(
                                    sock,
                                    service::service_fn(move |req| {
                                        let server_box = Arc::clone(&server_spawn);
                                        let trigger_plugins_box =
                                            Arc::clone(&trigger_plugins_spawn);

                                        Self::service(req, server_box, trigger_plugins_box)
                                    }),
                                )
                                .map_err(|e| {
                                    error!("{}", e);
                                }),
                        );
                    }
                    Ok(())
                })
                .map_err(|e| {
                    error!("{}", e);
                }),
        );
        Ok(())
    }

    pub fn serve(self) -> Result<(), Box<dyn Error>> {
        match self.server.server_type {
            config::ServerType::Webhook => {
                self.listen::<TcpListener, net::tcp::Incoming, TcpStream, io::Error>()?
            }
            config::ServerType::UnixSocket => {
                self.listen::<UnixListener, net::unix::Incoming, UnixStream, io::Error>()?
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
