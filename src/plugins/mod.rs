use std::borrow::Borrow;
use std::hash::{Hash,Hasher};
use std::io;

use libc;
use libloading::{Library,Symbol};
use missdemeanor::CRequest;

use config::Trigger;

mod err;
pub use self::err::*;

pub struct Plugin {
    lib: Library,
    pub config: Trigger,
}

impl Plugin {
    pub fn new(config: Trigger) -> Result<Self, io::Error> {
        Ok(Plugin {
            lib: Library::new(&config.plugin_path)?,
            config,
        })
    }

    pub fn run_trigger(&self, request: CRequest) -> Result<(), PluginError> {
        let func: Symbol<unsafe extern fn(*const CRequest) -> libc::c_int> = unsafe { self.lib.get(b"trigger\0") }.map_err(|e| {
            error!("{}", e);
            PluginError::new(500, "Failed to find handler")
        })?;
        match unsafe { func(&request as *const CRequest) } {
            i if i == 0 => Ok(()),
            _ => Err(PluginError::new(500, "Plugin exited unsuccessfully")),
        }
    }
}

impl Hash for Plugin {
    fn hash<H>(&self, hasher: &mut H) where H: Hasher {
        self.config.hash(hasher)
    }
}

impl PartialEq for Plugin {
    fn eq(&self, rhs: &Self) -> bool {
        self.config == rhs.config
    }
}

impl Eq for Plugin {}

impl Borrow<String> for Plugin {
    fn borrow(&self) -> &String {
        self.config.borrow()
    }
}
