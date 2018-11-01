use std::collections::{HashSet,VecDeque};
use std::error::Error;
use std::fmt::Debug;
use std::io;
use std::process;
use std::str;
use std::sync::Arc;
use std::sync::mpsc::Sender;

use hyper::{Body,Request,Response};
use hyper::server::conn::Http;
use hyper::service::Service;
use hyper_tls;
use native_tls;
use serde_json::Value;
use tokio;
use tokio::io::{AsyncRead,AsyncWrite};
use tokio::net::{self,TcpStream,UnixStream};
use tokio::prelude::{Future,Stream};
use tokio::prelude::future::{self,lazy};
use tokio::runtime::Runtime;

use config::{self,TomlConfig};
use err::DemeanorError;
use plugins::{Plugin,PluginError,PluginManager};

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

pub struct PluginService {
    sender: Sender<Request<Body>>,
    endpoints: Arc<HashSet<config::Endpoint>>,
}

impl Service for PluginService {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = DemeanorError;
    type Future = Box<Future<Item=Response<Body>, Error=DemeanorError> + Send>;

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        match self.sender.clone().send(req) {
            Ok(()) => (),
            Err(e) => {
                error!("{}", e);
                Box::new(PluginError::new(500, "Failed to queue request").to_response());
            },
        };
        Box::new(future::ok(Response::new(Body::from("Success!"))))
    }
}

pub struct WebhookServer {
    identity: Option<TlsIdentity>,
}

impl WebhookServer {
    pub fn new(use_tls: UseTls) -> Result<Self, Box<Error>> {
        let identity = match use_tls {
            UseTls::Yes(identity) => Some(identity),
            UseTls::No => None,
        };
        Ok(WebhookServer { identity, })
    }

    fn spawn_server<S>(sender: Sender<Request<Body>>,
                       endpoints: Arc<HashSet<config::Endpoint>>,
                       stream: S)
            where S: 'static + AsyncRead + AsyncWrite + Send {
        tokio::spawn(lazy(move || {
            Http::new().serve_connection(stream, PluginService { sender: sender,
                                                                 endpoints }).map_err(|e| {
                error!("Failed to serve HTTP request: {}", e);
                ()
            })
        }));
    }

    fn listen<L, S, C, E>(runtime: &mut Runtime, sender: Sender<Request<Body>>,
                          identity: Option<TlsIdentity>,
                          server: config::Server) -> Result<(), Box<Error>>
            where L: Listener<S, C, E>, C: 'static + AsyncRead + AsyncWrite + Debug + Send,
                  S: 'static + Stream<Item=C> + Send,
                  S::Error: Send,
                  E: 'static + Error + Send {
        let mut tls_acceptor = None;
        if let Some(ident) = identity.as_ref() {
            tls_acceptor = native_tls::TlsAcceptor::new(ident.into_identity()?).map(Arc::new).ok();
        }

        let listener = L::bind(server.listen_addr)?;
        let endpoints = Arc::new(server.endpoints);
        runtime.spawn(listener.for_each(move |sock| {
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
            Self::spawn_server(sender.clone(), Arc::clone(&endpoints), stream);
            Ok(())
        }).map_err(|_| ()));
        Ok(())
    }

    pub fn serve(self, runtime: &mut Runtime, sender: Sender<Request<Body>>, server: config::Server) -> Result<(), Box<Error>> {
        let identity = self.identity;

        match server.server_type {
            config::ServerType::Webhook => {
                Self::listen::<TcpListener, net::tcp::Incoming, TcpStream, io::Error>(
                    runtime, sender, identity, server
                )?
            },
            config::ServerType::UnixSocket => {
                Self::listen::<UnixListener, net::unix::Incoming, UnixStream, io::Error>(
                    runtime, sender, identity, server
                )?
            },
            config::ServerType::UnknownServerType => {
                Err(DemeanorError::new("Server type not recognized - exiting"))?
            }
        };
        Ok(())
    }
}
