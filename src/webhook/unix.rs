use std::error::Error;
use std::io;
use std::os::unix::net;

use webhook::listener::Listener;

pub(crate) struct UnixListener(net::UnixListener);

impl Listener<net::UnixStream, io::Error> for UnixListener {
    fn bind(listen_addr: String) -> Result<Self, Box<Error>> {
        Ok(UnixListener(net::UnixListener::bind(listen_addr)?))
    }
}

impl Iterator for UnixListener {
    type Item = Result<net::UnixStream, io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.0.accept().map(|p| p.0))
    }
}
