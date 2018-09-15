use std::collections::HashMap;
use std::error::Error;

use hyper::{Body,Request,Response};
use libc;
use libloading::{Library,Symbol};

use config::TomlConfig;
use err::DemeanorError;

pub struct PluginManager {
    plugins: HashMap<String, Library>,
}

impl PluginManager {
    pub fn new(config: &TomlConfig) -> Result<Self, Box<Error>> {
        let mut hm = HashMap::new();
        for plugin_def in config.triggers.iter() {
            let plugin = Library::new(plugin_def.plugin_path.clone())?;
            hm.insert(plugin_def.name.clone(), plugin);
        }
        Ok(PluginManager { plugins: HashMap::new() })
    }

    pub fn run_trigger(&self, name: &String, mut req: Request<Body>) -> Result<Response<Body>, Box<Error>> {
        if let Some(pi) = self.plugins.get(name) {
            let mut response = Response::new(Body::empty());
            let callback: Symbol<unsafe extern fn(*mut Request<Body>, *mut Response<Body>) -> libc::c_int> =
                unsafe {
                    pi.get(b"trigger")?
                };

            let ret = unsafe {
                callback(&mut req as *mut Request<Body>, &mut response as *mut Response<Body>)
            };
            if ret < 0 {
                return Err(Box::new(
                    DemeanorError::new(format!("Callback executed unsuccessfully for plugin {}", name))
                ));
            }

            Ok(response)
        } else {
            Err(Box::new(DemeanorError::new(format!("Requested plugin {} not found", name))))
        }
    }
}
