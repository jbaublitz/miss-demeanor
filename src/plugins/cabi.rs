use std::borrow::Borrow;
use std::collections::HashMap;
use std::ffi::CString;
use std::hash::{Hash,Hasher};
use std::io;
use std::ptr;

use libc;
use libloading::{Library,Symbol};
use serde_json::Value;

use super::{Plugin,PluginError};
use config::{self,PluginConfig};

pub struct CRequest {
    pub method: CString,
    pub uri: CString,
    pub headers: HashMap<CString, CString>,
    pub body: CString,
}

pub struct CABIPlugin<C> {
    lib: Library,
    pub config: C,
}

impl<C> CABIPlugin<C> where C: PluginConfig {
    pub fn new(config: C) -> Result<Self, io::Error> {
        Ok(CABIPlugin {
            lib: Library::new(config.get_plugin_path())?,
            config,
        })
    }
}

impl<C> Hash for CABIPlugin<C> where C: Hash {
    fn hash<H>(&self, hasher: &mut H) where H: Hasher {
        self.config.hash(hasher)
    }
}

impl<C> PartialEq for CABIPlugin<C> where C: PartialEq {
    fn eq(&self, rhs: &Self) -> bool {
        self.config == rhs.config
    }
}

impl<C> Eq for CABIPlugin<C> where C: Eq {}

impl<C> Borrow<String> for CABIPlugin<C> where C: Borrow<String> {
    fn borrow(&self) -> &String {
        self.config.borrow()
    }
}

impl<C> Plugin for CABIPlugin<C> where C: Send + Sync {
    type Request = CRequest;
    type State = *mut libc::c_void;

    fn run_trigger(&self, request: CRequest) -> Result<*mut libc::c_void, PluginError> {
        Ok(ptr::null_mut())
    }

    fn run_checker(&self, state: *mut libc::c_void) -> Result<(*mut libc::c_void, bool), PluginError> {
        Ok((ptr::null_mut(), true))
    }

    fn run_handler(&self, state: *mut libc::c_void, compliant: bool) -> Result<(), PluginError> {
        Ok(())
    }
}
