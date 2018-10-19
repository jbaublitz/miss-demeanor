use std::ffi::CString;
use std::io;

use hyper::{Body,Request};
use libc;

use run_ruby_trigger;
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
    fn run_trigger(&self, req: Request<Body>) -> Result<*mut libc::c_void, PluginError> {
        let ptr = unsafe { run_ruby_trigger(&req as *const Request<Body> as *const libc::c_void) };
        if ptr.is_null() {
            return Err(PluginError::new(500, "Ruby plugin returned an error"));
        }

        Ok(ptr)
    }

    fn run_checker(&self, state: *mut libc::c_void) -> Result<(bool, *mut libc::c_void), PluginError> {
        unimplemented!()
    }


    fn run_handler(&self, compliant: bool, state: *mut libc::c_void) -> Result<(), PluginError> {
        unimplemented!()
    }
}
