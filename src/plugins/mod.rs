use std::collections::HashSet;
use std::error::Error;
use std::fmt::{self,Display};
use std::io;
use std::marker::PhantomData;

use hyper::{Body,HeaderMap,Request,Response,StatusCode};
use serde_json::Value;

use config::{self,TomlConfig};

mod cabi;
mod cmanager;
#[cfg(feature = "ruby")]
mod ruby;
#[cfg(feature = "ruby")]
mod rubymanager;

pub use self::cabi::{CABIPlugin,CRequest};
pub use self::cmanager::CABIPluginManager;
#[cfg(feature = "ruby")]
pub use self::ruby::RubyPlugin;
#[cfg(feature = "ruby")]
pub use self::rubymanager::RubyPluginManager;

#[derive(Debug)]
pub struct PluginError(Response<Body>);

impl PluginError {
    pub fn new<S>(code: u16, body: S) -> Self where S: Display {
        let mut resp = Response::builder();
        resp.status(code);
        let resp_final = resp.body(Body::from(body.to_string())).unwrap_or_else(|e| {
            error!("{}", e);
            let mut resp = Response::new(Body::from("Whoops! Could not convert the error \
                                                     message an HTTP body - \
                                                     check the logs."));
            *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            resp
        });
        PluginError(resp_final)
    }

    pub fn to_response(self) -> Response<Body> {
        self.0
    }
}

impl Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self)
    }
}

impl Error for PluginError {}

pub trait Plugin: Send + Sync {
    type Request;
    type State;

    fn run_trigger(&self, Self::Request) -> Result<Self::State, PluginError>;
    fn run_checker(&self, Self::State)
        -> Result<(Self::State, bool), PluginError>;
    fn run_handler(&self, Self::State, bool) -> Result<(), PluginError>;
}

pub trait PluginManager: Send + Sync {
    fn process_request(&mut self, Request<Body>) -> Result<(), Box<Error>>;
}
