use std::error::Error;

use async_trait::async_trait;
use tokio::{
    io::{self, AsyncRead, AsyncWrite},
    stream::Stream,
};

#[async_trait]
pub(crate) trait Listener<C, E>: Sized
where
    Self: Stream<Item = io::Result<C>>,
    C: AsyncRead + AsyncWrite,
    E: Error,
{
    async fn bind(listen_addr: &str) -> Result<Self, E>;
}
