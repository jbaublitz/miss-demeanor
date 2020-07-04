use std::io;

use async_trait::async_trait;
use tokio::net::{UnixListener, UnixStream};

use crate::webhook::listener::Listener;

#[async_trait]
impl Listener<UnixStream, io::Error> for UnixListener {
    async fn bind(listen_addr: &str) -> Result<Self, io::Error> {
        UnixListener::bind(listen_addr)
    }
}
