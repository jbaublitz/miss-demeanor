use std::error::Error;
use std::fmt::{self,Display};

use hyper::{Body,Response,StatusCode};

#[derive(Debug)]
pub struct PluginError(u16, String);

impl PluginError {
    pub fn new<S>(code: u16, body: S) -> Self where S: Display {
        PluginError(code, body.to_string())
    }

    pub fn to_response(self) -> Response<Body> {
        let mut response = Response::new(Body::from(self.1));
        *response.status_mut() = StatusCode::from_u16(self.0)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        response
    }
}

impl Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self)
    }
}

impl Error for PluginError {}
