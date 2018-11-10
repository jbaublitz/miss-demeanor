use std::borrow::Borrow;
use std::collections::HashMap;
use std::error::Error;
use std::ffi::CString;
use std::fmt::{self,Display};
use std::hash::{Hash,Hasher};
use std::io;
use std::ptr;

use hyper::{Body,Response,StatusCode};
use libc;
use libloading::Library;

use config::PluginConfig;

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

pub struct CRequest {
    pub method: CString,
    pub uri: CString,
    pub headers: HashMap<CString, CString>,
    pub body: CString,
}

pub struct Plugin<C> {
    lib: Library,
    pub config: C,
}

impl<C> Plugin<C> where C: PluginConfig {
    pub fn new(config: C) -> Result<Self, io::Error> {
        Ok(Plugin {
            lib: Library::new(config.get_plugin_path())?,
            config,
        })
    }

    pub fn run_trigger(&self, request: CRequest) -> Result<*mut libc::c_void, PluginError> {
        Ok(ptr::null_mut())
    }

    pub fn run_checker(&self, state: *mut libc::c_void) -> Result<(*mut libc::c_void, bool), PluginError> {
        Ok((ptr::null_mut(), true))
    }

    pub fn run_handler(&self, state: *mut libc::c_void, compliant: bool) -> Result<(), PluginError> {
        Ok(())
    }
}

impl<C> Hash for Plugin<C> where C: Hash {
    fn hash<H>(&self, hasher: &mut H) where H: Hasher {
        self.config.hash(hasher)
    }
}

impl<C> PartialEq for Plugin<C> where C: PartialEq {
    fn eq(&self, rhs: &Self) -> bool {
        self.config == rhs.config
    }
}

impl<C> Eq for Plugin<C> where C: Eq {}

impl<C> Borrow<String> for Plugin<C> where C: Borrow<String> {
    fn borrow(&self) -> &String {
        self.config.borrow()
    }
}
