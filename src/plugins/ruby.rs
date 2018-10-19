use std::ffi::CString;
use std::io;

use hyper::{Body,Request,Response};
use libc;

use super::{PluginAPI,PluginError};

pub struct RubyPlugin(CString);

impl RubyPlugin {
    pub fn new(path: &str) -> Result<Self, io::Error> {
        let cstring = CString::new(path.as_bytes()).map_err(|e| {
            error!("{}", e);
            io::Error::from(io::ErrorKind::InvalidInput)
        })?;
        Ok(RubyPlugin(cstring))
    }
}

impl PluginAPI for RubyPlugin {
    fn run_trigger(&self, mut req: Request<Body>)
            -> Result<(Response<Body>, *mut libc::c_void), PluginError> {
        if unsafe { run_ruby_trigger() }.is_null() {
            return Err(PluginError::new(500, "Ruby plugin returned an error"));
        }

        Ok(())
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
