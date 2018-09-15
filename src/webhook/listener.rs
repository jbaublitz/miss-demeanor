use std::error::Error;
use std::io::{Read,Write};

pub(crate) trait Listener<S, E>: Sized + Iterator<Item=Result<S, E>> where S: Read + Write, E: Error {
    fn bind(listen_addr: String) -> Result<Self, Box<Error>>;
}
