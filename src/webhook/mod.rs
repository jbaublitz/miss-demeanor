use std::collections::HashSet;
use std::error::Error;
use std::fmt::Debug;
use std::io::{self,Read,Write};
use std::net::TcpStream;
use std::os::unix::net::UnixStream;
use std::process;
use std::sync::Arc;

use http;
use hyper;
use hyper::{Body,Request,Response};
use hyper::server::conn::Http;
use hyper_tls;
use native_tls;
use tokio;
use tokio::prelude::Future;
use tokio::prelude::future::lazy;
use tokio_io::{AsyncRead,AsyncWrite};
use tokio_io::io::AllowStdIo;
use tokio_threadpool::ThreadPool;

use config::{self,TomlConfig};
use err::DemeanorError;
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

pub struct WebhookServer {
    pool: ThreadPool,
    identity: Option<TlsIdentity>,
    plugin_manager: Arc<PluginManager>,
}

fn spawn_server<S>(manager: Arc<PluginManager>, endpoints: Arc<HashSet<config::Endpoint>>, stream: S)
        where S: 'static + AsyncRead + AsyncWrite + Send {
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

fn listen<L, S, E>(plugin_manager: Arc<PluginManager>,
                   mut tls_acceptor: Option<Arc<native_tls::TlsAcceptor>>,
                   server: config::Server) -> Result<(), Box<Error>>
        where L: Listener<S, E>, S: 'static + Read + Write + Debug + Send, E: 'static + Error {
    let server_addr = server.listen_addr;
    let listener = L::bind(server_addr)?;
    let endpoints = Arc::new(server.endpoints);
    for sock_result in listener {
        let sock = sock_result?;
        let stream = if let Some(ref mut acceptor) = tls_acceptor.as_mut() {
            let tls_stream = acceptor.accept(AllowStdIo::new(sock))?;
            hyper_tls::MaybeHttpsStream::from(tls_stream)
        } else {
            hyper_tls::MaybeHttpsStream::from(AllowStdIo::new(sock))
        };
        spawn_server(Arc::clone(&plugin_manager), Arc::clone(&endpoints), stream)
    }
    Ok(())
}

impl WebhookServer {
    pub fn new(use_tls: UseTls, plugin_manager: PluginManager) -> Result<Self, Box<Error>> {
        let identity = match use_tls {
            UseTls::Yes(identity) => Some(identity),
            UseTls::No => None,
        };
        Ok(WebhookServer { pool: ThreadPool::new(), identity,
            plugin_manager: Arc::new(plugin_manager) })
    }

    fn spawn_listener<L, S, E>(&self, server: config::Server) -> Result<(), Box<Error>>
            where L: Listener<S, E>, S: 'static + Read + Write + Debug + Send, E: 'static + Error {
        let mut tls_acceptor = None;
        if let Some(ref ident) = self.identity {
            tls_acceptor = native_tls::TlsAcceptor::new(ident.into_identity()?).map(Arc::new).ok();
        }
        let plugin_manager = Arc::clone(&self.plugin_manager); 

        self.pool.spawn(lazy(move || {
            listen::<L, S, E>(plugin_manager, tls_acceptor, server).map_err(|e| {
                error!("{}", e);
                ()
            })
        }));

        Ok(())
    }

    pub fn serve<'a>(self, config: TomlConfig) -> Result<(), Box<Error>> {
        for server in config.servers.into_iter() {
            match server.server_type {
                config::ServerType::Webhook => {
                    self.spawn_listener::<TcpListener, TcpStream, io::Error>(server)?
                }
                config::ServerType::UnixSocket => {
                    self.spawn_listener::<UnixListener, UnixStream, io::Error>(server)?
                },
                config::ServerType::UnknownServerType => {
                    error!("Server type not recognized - exiting");
                    process::exit(1);
                }
            };
        }
        self.pool.shutdown_on_idle().wait().map_err(|_| {
            DemeanorError::new("Tokio runtime shutdown failed")
        })?;
        Ok(())
    }
}
