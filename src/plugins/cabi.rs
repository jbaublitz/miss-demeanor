use std::io;

use hyper::{Body,Request};
use libc;
use libloading::{Library,Symbol};

use super::{PluginAPI,PluginError};

pub struct CABIPlugin {
    lib: Library,
}

impl CABIPlugin {
    pub fn new(path: &str) -> Result<Self, io::Error> {
        Ok(CABIPlugin { lib: Library::new(path)?, })
    }
}

impl PluginAPI for CABIPlugin {
    fn run_trigger(&self, mut req: Request<Body>)
            -> Result<*mut libc::c_void, PluginError> {
        let callback: Symbol<unsafe extern fn(*mut Request<Body>) -> *mut libc::c_void> =
            unsafe { self.lib.get(b"trigger") }.map_err(|e| PluginError::new(500, e))?;

        let state = unsafe {
            callback(&mut req as *mut Request<Body>)
        };
        if state.is_null() {
            return Err(PluginError::new(500, format!("Trigger plugin returned an error")));
        }

        Ok(state)
    }

    fn run_checker(&self, state: *mut libc::c_void) -> Result<(bool, *mut libc::c_void), PluginError> {
        let callback: Symbol<unsafe extern fn(*mut libc::c_void) -> bool> =
            unsafe { self.lib.get(b"checker") }.map_err(|e| PluginError::new(500, e))?;

        let compliant = unsafe { callback(state) };

        Ok((compliant, state))
    }

    fn run_handler(&self, compliant: bool, state: *mut libc::c_void)
            -> Result<(), PluginError> {
        let callback: Symbol<unsafe extern fn(bool, *mut libc::c_void) -> libc::c_int> =
            unsafe { self.lib.get(b"handler") }.map_err(|e| PluginError::new(500, e))?;

        match unsafe { callback(compliant, state) } {
            i if i == 0 => Ok(()),
            _ => Err(PluginError::new(500, "Handler plugin returned an error")),
        }
    }
}
