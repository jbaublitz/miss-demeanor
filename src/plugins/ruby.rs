use std::ffi::CString;
use std::io;

use hyper::{Body,Request};
use libc;

use {is_nil,run_ruby_trigger};
use super::{Plugin,PluginError};

pub struct RubyPlugin{
    path: CString,
}

impl Plugin for RubyPlugin {
    type State = libc::c_ulong;

    fn new(path: &str) -> Result<Self, io::Error> {
        let cstring = CString::new(path.as_bytes()).map_err(|e| {
            error!("{}", e);
            io::Error::from(io::ErrorKind::InvalidInput)
        })?;
        Ok(RubyPlugin {
            path: cstring
        })
    }

    fn run_trigger(&self, req: Request<Body>) -> Result<libc::c_ulong, PluginError> {
        let id = unsafe { run_ruby_trigger(&req as *const Request<Body> as *const libc::c_void) };
        if unsafe { is_nil(id) } != 0 {
            return Err(PluginError::new(500, "Ruby plugin returned an error"));
        }

        Ok(id)
    }

    fn run_checker(&self, state: libc::c_ulong) -> Result<(bool, libc::c_ulong), PluginError> {
        unimplemented!()
    }


    fn run_handler(&self, compliant: bool, state: libc::c_ulong) -> Result<(), PluginError> {
        unimplemented!()
    }
}
