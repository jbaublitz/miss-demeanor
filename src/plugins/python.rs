use std::io;

use hyper::{Body,Request,Response};
use libc;

use super::{PluginAPI,PluginError};

pub struct PythonPlugin;

impl PythonPlugin {
    pub fn new(path: &str) -> Result<Self, io::Error> {
        Ok(PythonPlugin)
    }
}

impl PluginAPI for PythonPlugin {
    fn run_trigger(&self, mut req: Request<Body>)
            -> Result<(Response<Body>, *mut libc::c_void), PluginError> {
        unimplemented!()
    }

    fn run_checker(&self, resp: Response<Body>, state: *mut libc::c_void)
            -> Result<(Response<Body>, bool, *mut libc::c_void), PluginError> {
        unimplemented!()
    }


    fn run_handler(&self, resp: Response<Body>, compliant: bool, state: *mut libc::c_void)
            -> Result<Response<Body>, PluginError> {
        unimplemented!()
    }
}