use std::io;

use tokio::net;

use webhook::listener::Listener;

pub(crate) struct UnixListener(net::UnixListener);

impl Listener<net::unix::Incoming, net::UnixStream, io::Error> for UnixListener {
    fn bind(listen_addr: String) -> Result<net::unix::Incoming, io::Error> {
        net::UnixListener::bind(listen_addr).map(|list| list.incoming())
    }
}
