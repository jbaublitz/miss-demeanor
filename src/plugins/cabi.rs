use std::borrow::Borrow;
use std::hash::{Hash,Hasher};
use std::io;

use libc;
use libloading::{Library,Symbol};
use missdemeanor::CRequest;

use super::{NewPlugin,Plugin};
use super::err::PluginError;
use config::Trigger;

pub struct CABIPlugin {
    lib: Library,
    pub config: Trigger,
}

impl NewPlugin for CABIPlugin {
    fn new(config: Trigger) -> Result<Self, io::Error> {
        Ok(CABIPlugin {
            lib: Library::new(&config.plugin_path)?,
            config,
        })
    }
}

impl Plugin for CABIPlugin {
    fn run_trigger(&self, request: CRequest) -> Result<(), PluginError> {
        let func: Symbol<unsafe extern fn(*const CRequest) -> libc::c_int> = unsafe { self.lib.get(b"trigger\0") }.map_err(|e| {
            error!("{}", e);
            PluginError::new(500, "Failed to find handler")
        })?;
        match unsafe { func(&request as *const CRequest) } {
            i if i == 0 => Ok(()),
            _ => {
                error!("Plugin exited unsuccessfully");
                Err(PluginError::new(500, "Internal server error"))
            },
        }
    }
}

impl Hash for CABIPlugin {
    fn hash<H>(&self, hasher: &mut H) where H: Hasher {
        self.config.hash(hasher)
    }
}

impl PartialEq for CABIPlugin {
    fn eq(&self, rhs: &Self) -> bool {
        self.config == rhs.config
    }
}

impl Eq for CABIPlugin {}

impl Borrow<String> for CABIPlugin {
    fn borrow(&self) -> &String {
        self.config.borrow()
    }
}
