use std::sync::mpsc::Sender;

use libc;
use serde_json::Value;

use super::{Plugin,PluginError};
use config::{self,PluginConfig};

extern "C" {
    fn rb_intern(string: *const libc::c_char) -> libc::c_ulong;
    fn rb_str_new(string: *const libc::c_char, len: libc::c_long) -> libc::c_ulong;
    fn rb_load_protect(string: libc::c_ulong, namespace: libc::c_int, state: *mut libc::c_int);
    fn rb_protect(callback: unsafe extern "C" fn(libc::c_ulong) -> libc::c_ulong,
                  args: libc::c_ulong,
                  state: *mut libc::c_int)
        -> libc::c_ulong;
}

pub struct RubyPlugin<C> {
    pub config: C,
}

impl<C> RubyPlugin<C> where C: PluginConfig {
    pub fn new(config: C) -> Self {
        RubyPlugin {
            config,
        }
    }
}

impl<C> Plugin for RubyPlugin<C> where C: PluginConfig + Send + Sync {
    type Request = Value;
    type State = libc::c_ulong;

    fn run_trigger(&self, json: Value) -> Result<libc::c_ulong, PluginError> {
        let string = unsafe { rb_str_new(self.config.get_plugin_path().as_ptr() as *const libc::c_char,
                                         self.config.get_plugin_path().len() as libc::c_long) };
        let mut state: libc::c_int = 0;
        unsafe { rb_load_protect(string, 0, &mut state) };
        Ok(0)
    }

    fn run_checker(&self, state: libc::c_ulong) -> Result<(libc::c_ulong, bool), PluginError> {
        Ok((0, true))
    }

    fn run_handler(&self, state: libc::c_ulong, compliant: bool) -> Result<(), PluginError> {
        Ok(())
    }
}
