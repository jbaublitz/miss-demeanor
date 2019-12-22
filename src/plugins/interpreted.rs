use std::borrow::Borrow;
use std::hash::{Hash, Hasher};
use std::io;
use std::process::Command;

use missdemeanor::CRequest;

use config::{PluginConfig, Trigger};

use super::err::PluginError;
use super::{NewPlugin, Plugin};

pub struct InterpretedPlugin {
    cmd: String,
    pub config: Trigger,
}

impl NewPlugin for InterpretedPlugin {
    fn new(config: Trigger) -> Result<Self, io::Error> {
        Ok(InterpretedPlugin {
            cmd: config.get_plugin_path().to_string(),
            config,
        })
    }
}

impl Plugin for InterpretedPlugin {
    fn run_trigger(&self, request: CRequest) -> Result<(), PluginError> {
        let mut cmd = Command::new(self.cmd.as_str())
            .arg(request.get_method().map_err(|e| {
                error!("{}", e);
                PluginError::new(400, "Bad method")
            })?)
            .arg(request.get_uri().map_err(|e| {
                error!("{}", e);
                PluginError::new(400, "Bad URI")
            })?)
            .arg(request.get_headers().map_err(|e| {
                error!("{}", e);
                PluginError::new(400, "Bad headers")
            })?)
            .arg(request.get_body().map_err(|e| {
                error!("{}", e);
                PluginError::new(400, "Bad body")
            })?)
            .spawn()
            .map_err(|e| {
                error!("{}", e);
                PluginError::new(500, "Internal server error")
            })?;
        let status = cmd.wait();
        match status.map(|s| s.code()) {
            Ok(Some(0)) => Ok(()),
            Ok(Some(_)) => {
                error!("Plugin exited unsuccessfully");
                Err(PluginError::new(500, "Internal server error"))
            }
            Ok(None) => {
                error!("No status code returned");
                Err(PluginError::new(500, "Internal server error"))
            }
            Err(e) => {
                error!("{}", e);
                Err(PluginError::new(500, "Internal server error"))
            }
        }
    }
}

impl Hash for InterpretedPlugin {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.config.hash(hasher)
    }
}

impl PartialEq for InterpretedPlugin {
    fn eq(&self, rhs: &Self) -> bool {
        self.config == rhs.config
    }
}

impl Eq for InterpretedPlugin {}

impl Borrow<String> for InterpretedPlugin {
    fn borrow(&self) -> &String {
        self.config.borrow()
    }
}
