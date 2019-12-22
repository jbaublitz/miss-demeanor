use std::io;
use std::net::ToSocketAddrs;

use tokio::net;

use webhook::listener::Listener;

pub(crate) struct TcpListener(net::TcpListener);

impl Listener<net::tcp::Incoming, net::TcpStream, io::Error> for TcpListener {
    fn bind(listen_addr: &str) -> Result<net::tcp::Incoming, io::Error> {
        let sock_addr = listen_addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::from(io::ErrorKind::AddrNotAvailable))?;
        net::TcpListener::bind(&sock_addr).map(|l| l.incoming())
    }
}
