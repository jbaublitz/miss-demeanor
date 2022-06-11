use std::io;

use async_trait::async_trait;
use tokio::net::{UnixListener, UnixStream};
use tokio_stream::wrappers::UnixListenerStream;

use crate::webhook::listener::Listener;

#[async_trait]
impl Listener<UnixStream, io::Error> for UnixListenerStream {
    async fn bind(listen_addr: &str) -> Result<Self, io::Error> {
        Ok(UnixListenerStream::new(UnixListener::bind(listen_addr)?))
    }
}
