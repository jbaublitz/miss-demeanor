use std::error::Error;

use tokio::prelude::{AsyncRead, AsyncWrite, Stream};

pub(crate) trait Listener<S, C, E>: Sized
where
    S: Stream<Item = C>,
    C: AsyncRead + AsyncWrite,
    E: Error,
{
    fn bind(listen_addr: &str) -> Result<S, E>;
}
