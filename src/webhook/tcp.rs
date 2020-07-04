use std::{io, net::ToSocketAddrs};

use async_trait::async_trait;
use tokio::net::{TcpListener, TcpStream};

use crate::webhook::listener::Listener;

#[async_trait]
impl<'a> Listener<TcpStream, io::Error> for TcpListener {
    async fn bind(listen_addr: &str) -> Result<Self, io::Error> {
        let sock_addr = listen_addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::from(io::ErrorKind::AddrNotAvailable))?;
        TcpListener::bind(&sock_addr).await
    }
}
