use std::error::Error;
use std::fmt::{self, Display};

use http_body_util::Full;
use hyper::{body::Bytes, Response, StatusCode};

#[derive(Debug)]
pub struct PluginError(u16, String);

impl PluginError {
    pub fn new<S>(code: u16, body: S) -> Self
    where
        S: Display,
    {
        PluginError(code, body.to_string())
    }

    pub fn into_response(self) -> Response<Full<Bytes>> {
        let mut response = Response::new(Full::new(Bytes::from(self.1)));
        *response.status_mut() =
            StatusCode::from_u16(self.0).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        response
    }
}

impl Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.0)
    }
}

impl Error for PluginError {}
