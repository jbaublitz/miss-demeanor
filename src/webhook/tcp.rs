use std::error::Error;
use std::io;
use std::net;

use webhook::listener::Listener;

pub(crate) struct TcpListener(net::TcpListener);

impl Listener<net::TcpStream, io::Error> for TcpListener {
    fn bind(listen_addr: String) -> Result<Self, Box<Error>> {
        Ok(TcpListener(net::TcpListener::bind(listen_addr)?))
    }
}

impl Iterator for TcpListener {
    type Item = Result<net::TcpStream, io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.0.accept().map(|p| p.0))
    }
}
