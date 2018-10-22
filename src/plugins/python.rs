use std::io;

use hyper::{Body,Request,Response};
use libc;

use super::{Plugin,PluginError};

pub struct PythonPlugin;

impl PythonPlugin {
    pub fn new(path: &str) -> Result<Self, io::Error> {
        Ok(PythonPlugin)
    }
}

impl Plugin for PythonPlugin {
    fn run_trigger(&self, mut req: Request<Body>)
            -> Result<(), PluginError> {
        unimplemented!()
    }

    fn run_checker(&self) -> Result<bool, PluginError> {
        unimplemented!()
    }


    fn run_handler(&self, resp: Response<Body>, compliant: bool) -> Result<(), PluginError> {
        unimplemented!()
    }
}
