use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::net::TcpListener;

use futures::Future;
use hyper;
use hyper::server::conn::Http;
use hyper_tls;
use native_tls;
use tokio::executor::thread_pool::ThreadPool;
use tokio_io::io::AllowStdIo;

pub enum UseTls {
    Yes(String, String),
    No,
}

pub struct WebhookServer(ThreadPool, Option<native_tls::Identity>,
        fn(hyper::Request<hyper::Body>) -> Result<hyper::Response<hyper::Body>, hyper::Error>);

impl WebhookServer {
    pub fn new(use_tls: UseTls,
               service: fn(hyper::Request<hyper::Body>)
               -> Result<hyper::Response<hyper::Body>, hyper::Error>)
               -> Result<Self, Box<Error>> {
        let identity = match use_tls {
            UseTls::Yes(identity_path, pw) => {
                let mut file = File::open(identity_path)?;
                let mut pkcs12 = Vec::new();
                file.read_to_end(&mut pkcs12)?;
                Some(native_tls::Identity::from_pkcs12(&pkcs12, &pw)?)
            },
            UseTls::No => None,
        };
        Ok(WebhookServer(ThreadPool::new(), identity, service))
    }

    pub fn serve(self, addr_string: String) -> Result<(), Box<Error>> {
        let mut tls_acceptor = None;
        if let Some(ident) = self.1 {
            tls_acceptor = native_tls::TlsAcceptor::new(ident).ok();
        }
        for sock_result in TcpListener::bind(addr_string)?.incoming() {
            let sock = match sock_result {
                Ok(s) => s,
                Err(e) => {
                    error!("{}", e);
                    continue;
                },
            };
            let stream = if let Some(ref mut acceptor) = tls_acceptor.as_mut() {
                let tls_stream = match acceptor.accept(AllowStdIo::new(sock)) {
                    Ok(ts) => ts,
                    Err(e) => {
                        error!("{}", e);
                        continue;
                    },
                };
                hyper_tls::MaybeHttpsStream::from(tls_stream)
            } else {
                hyper_tls::MaybeHttpsStream::from(AllowStdIo::new(sock))
            };
            self.0.spawn(Http::new().serve_connection(stream, hyper::service::service_fn(self.2))
                         .map_err(|_| ()));
        }
        Ok(())
    }
}
