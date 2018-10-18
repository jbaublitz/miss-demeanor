use std::collections::HashSet;
use std::error::Error;
use std::fmt::Debug;
use std::io;
use std::process;
use std::sync::Arc;

use http;
use hyper;
use hyper::{Body,Request,Response};
use hyper::server::conn::Http;
use hyper_tls;
use native_tls;
use tokio;
use tokio::io::{AsyncRead,AsyncWrite};
use tokio::net::{self,TcpStream,UnixStream};
use tokio::prelude::{Future,Stream};
use tokio::prelude::future::lazy;

use config::{self,TomlConfig};
use plugins::PluginManager;

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

fn spawn_server<S>(manager: Arc<PluginManager>, endpoints: Arc<HashSet<config::Endpoint>>,
                   stream: S) where S: 'static + AsyncRead + AsyncWrite + Send {
    tokio::spawn(lazy(move || {
        Http::new().serve_connection(stream,
            hyper::service::service_fn(move |req: Request<Body>| -> Result<Response<Body>, http::Error> {
                let path = req.uri().path().to_string();
                let plugin_name = match endpoints.get(&path).and_then(|epoint| Some(&epoint.trigger_name)) {
                    Some(trigger_name) => trigger_name,
                    None => {
                        let mut resp = Response::builder();
                        resp.status(404);
                        return resp.body(Body::from("Endpoint does not exist"))
                    }
                };
                manager.run_trigger(&plugin_name, req).and_then(|(name, resp, ptr)| {
                    manager.run_checker(name, resp, ptr)
                }).and_then(|(name, resp, b, ptr)| {
                    manager.run_handler(name, resp, b, ptr)
                }).or_else(|e| Ok(e.to_response()))
            })
        ).map_err(|e| {
            error!("Failed to serve HTTP request: {}", e);
            ()
        })
    }));
}

fn listen<L, S, C, E>(plugin_manager: Arc<PluginManager>,
                      mut tls_acceptor: Option<Arc<native_tls::TlsAcceptor>>,
                      server: config::Server) -> Result<(), Box<Error>>
        where L: Listener<S, C, E>, C: 'static + AsyncRead + AsyncWrite + Debug + Send,
              S: 'static + Stream<Item=C> + Send,
              S::Error: Send,
              E: 'static + Error + Send {
    let server_addr = server.listen_addr;
    let listener = L::bind(server_addr)?;
    let endpoints = Arc::new(server.endpoints);
    tokio::spawn(listener.for_each(move |sock| {
        let stream = if let Some(ref mut acceptor) = tls_acceptor.as_mut() {
            let tls_stream = match acceptor.accept(sock) {
                Ok(st) => st,
                Err(e) => {
                    error!("{}", e);
                    return Ok(())
                }
            };
            hyper_tls::MaybeHttpsStream::from(tls_stream)
        } else {
            hyper_tls::MaybeHttpsStream::from(sock)
        };
        spawn_server(Arc::clone(&plugin_manager), Arc::clone(&endpoints), stream);
        Ok(())
    }).map_err(|_| ()));
    Ok(())
}

fn spawn_listener<L, S, C, E>(server: config::Server, manager: Arc<PluginManager>,
                              identity: Arc<Option<TlsIdentity>>) -> Result<(), Box<Error>>
        where L: Listener<S, C, E>, C: 'static + AsyncRead + AsyncWrite + Debug + Send,
              S: 'static + Stream<Item=C> + Send,
              S::Error: Send,
              E: 'static + Error + Send {
    let mut tls_acceptor = None;
    if let Some(ident) = identity.as_ref() {
        tls_acceptor = native_tls::TlsAcceptor::new(ident.into_identity()?).map(Arc::new).ok();
    }

    tokio::spawn(lazy(move || {
        listen::<L, S, C, E>(manager, tls_acceptor, server).map_err(|e| {
            error!("{}", e);
            ()
        })
    }));

    Ok(())
}

pub struct WebhookServer {
    identity: Option<TlsIdentity>,
    plugin_manager: Arc<PluginManager>,
}

impl WebhookServer {
    pub fn new(use_tls: UseTls, plugin_manager: PluginManager) -> Result<Self, Box<Error>> {
        let identity = match use_tls {
            UseTls::Yes(identity) => Some(identity),
            UseTls::No => None,
        };
        Ok(WebhookServer { identity, plugin_manager: Arc::new(plugin_manager) })
    }

    pub fn serve<'a>(self, config: TomlConfig) -> Result<(), Box<Error>> {
        let identity = Arc::new(self.identity);
        let plugin_manager_clone = Arc::clone(&self.plugin_manager);

        tokio::run(lazy(move || {
            for server in config.servers.into_iter() {
                match server.server_type {
                    config::ServerType::Webhook => {
                        spawn_listener::<TcpListener, net::tcp::Incoming, TcpStream, io::Error>(
                            server, Arc::clone(&plugin_manager_clone), Arc::clone(&identity)
                        ).map_err(|e| {
                            error!("{}", e);
                            ()
                        })?
                    },
                    config::ServerType::UnixSocket => {
                        spawn_listener::<UnixListener, net::unix::Incoming, UnixStream, io::Error>(
                            server, Arc::clone(&plugin_manager_clone), Arc::clone(&identity)
                        ).map_err(|e| {
                            error!("{}", e);
                            ()
                        })?
                    },
                    config::ServerType::UnknownServerType => {
                        error!("Server type not recognized - exiting");
                        process::exit(1);
                    }
                };
            }
            Ok(())
        }));
        Ok(())
    }
}
