use std::collections::{HashMap,HashSet};
use std::error::Error;
use std::ffi::CString;
use std::fmt::{Debug,Display};
use std::io;
use std::sync::Arc;

use hyper::{Body,Request,Response};
use hyper::server::conn::Http;
use hyper::service;
use hyper_tls;
use miss_demeanor_pluginutils::CRequest;
use native_tls;
use tokio;
use tokio::io::{AsyncRead,AsyncWrite};
use tokio::net::{self,TcpStream,UnixStream};
use tokio::prelude::{Future,Stream};

use config::{self,Server,TomlConfig};
use err::DemeanorError;
use plugins::{Plugin,PluginError};

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

    pub fn into_identity(&self) -> Result<native_tls::Identity, native_tls::Error> {
        native_tls::Identity::from_pkcs12(&self.identity, &self.pw)
    }
}

pub struct WebhookServer {
    identity: Option<TlsIdentity>,
    server: Arc<Server>,
    triggers: Arc<HashSet<Plugin>>,
}

impl WebhookServer {
    pub fn new(use_tls: UseTls, mut toml_config: TomlConfig) -> Result<Self, Box<Error>> {
        let identity = match use_tls {
            UseTls::Yes(identity) => Some(identity),
            UseTls::No => None,
        };

        let mut trigger_plugins_hs = HashSet::new();
        for trigger in toml_config.triggers.drain() {
            trigger_plugins_hs.insert(Plugin::new(trigger)?);
        }

        let trigger_plugins = Arc::new(trigger_plugins_hs);

        Ok(WebhookServer { identity, server: Arc::new(toml_config.server),
                           triggers: trigger_plugins, })
    }

    fn service(req: Request<Body>, server_box: Arc<Server>,
               trigger_plugins_box: Arc<HashSet<Plugin>>)
            -> Box<Future<Item=Response<Body>, Error=PluginError> + Send> {
        let (parts, body) = req.into_parts();
        Box::new(body.concat2().map_err(|e| {
            PluginError::new(500, e)
        }).and_then(move |b| {
            let method = CString::new(parts.method.as_str()).map_err(|e| {
                error!("{}", e);
                PluginError::new(400, "Invalid method")
            })?;

            let uri = parts.uri.to_string();
            let name = server_box.endpoints.get(&uri).map(|e| &e.trigger_name).ok_or_else(|| {
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
        }).or_else(|e| {
            Ok(e.to_response())
        }))
    }

    fn listen<L, S, C, E>(self) -> Result<(), Box<Error>>
            where L: Listener<S, C, E>, C: 'static + AsyncRead + AsyncWrite + Debug + Send,
                  S: 'static + Stream<Item=C> + Send,
                  S::Error: Display + Send,
                  E: 'static + Error + Send {
        let mut tls_acceptor = None;
        if let Some(ident) = self.identity.as_ref() {
            tls_acceptor = native_tls::TlsAcceptor::new(ident.into_identity()?).map(Arc::new).ok();
        }

        let listener = L::bind(&self.server.listen_addr)?;

        tokio::run(listener.for_each(move |sock| {
            let stream = if let Some(ref mut acceptor) = tls_acceptor.as_mut() {
                let tls_stream = match acceptor.accept(sock) {
                    Ok(st) => st,
                    Err(e) => {
                        error!("{}", e);
                        return Ok(());
                    }
                };
                hyper_tls::MaybeHttpsStream::from(tls_stream)
            } else {
                hyper_tls::MaybeHttpsStream::from(sock)
            };

            let server_spawn = Arc::clone(&self.server);
            let trigger_plugins_spawn = Arc::clone(&self.triggers);
            tokio::spawn(Http::new().serve_connection(stream, service::service_fn(move |req| {
                let server_box = Arc::clone(&server_spawn);
                let trigger_plugins_box = Arc::clone(&trigger_plugins_spawn);

                Self::service(req, server_box, trigger_plugins_box)
            })).map_err(|e| {
                error!("{}", e);
                ()
            }));
            Ok(())
        }).map_err(|e| {
            error!("{}", e);
            ()
        }));
        Ok(())
    }

    pub fn serve(self)
            -> Result<(), Box<Error>> {
        match self.server.server_type {
            config::ServerType::Webhook => {
                self.listen::<TcpListener, net::tcp::Incoming, TcpStream, io::Error>()?
            },
            config::ServerType::UnixSocket => {
                self.listen::<UnixListener, net::unix::Incoming, UnixStream, io::Error>()?
            },
            config::ServerType::UnknownServerType => {
                Err(DemeanorError::new("Server type not recognized - exiting"))?
            }
        };
        Ok(())
    }
}
