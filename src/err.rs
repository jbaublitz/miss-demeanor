use std::error::Error;
use std::fmt::{self, Display};

#[derive(Debug)]
pub struct DemeanorError(String);

impl DemeanorError {
    pub fn new<T>(msg: T) -> Self
    where
        T: Display,
    {
        DemeanorError(msg.to_string())
    }
}

impl Display for DemeanorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for DemeanorError {}
