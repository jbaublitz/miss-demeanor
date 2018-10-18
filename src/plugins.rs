use std::borrow::Borrow;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{self,Display};
use std::hash::{Hash,Hasher};

use hyper::{Body,Request,Response,StatusCode};
use libc;
use libloading::{Library,Symbol};

use config::{self,TomlConfig};
use err::DemeanorError;

#[derive(Debug)]
pub struct PluginError(Response<Body>);

impl PluginError {
    pub fn new<S>(code: u16, body: S) -> Self where S: Display {
        let mut resp = Response::builder();
        resp.status(code);
        let resp_final = resp.body(Body::from(body.to_string())).unwrap_or_else(|e| {
            error!("{}", e);
            let mut resp = Response::new(Body::from("Whoops! Could not convert the error \
                                                     message an HTTP body - \
                                                     check the logs."));
            *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            resp
        });
        PluginError(resp_final)
    }

    pub fn to_response(self) -> Response<Body> {
        self.0
    }
}

impl Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self)
    }
}

impl Error for PluginError {
}

pub struct Plugin<T> {
    lib: Library,
    plugin_def: T,
}

impl<T> PartialEq for Plugin<T> where T: PartialEq {
    fn eq(&self, rhs: &Plugin<T>) -> bool {
        self.plugin_def == rhs.plugin_def
    }
}

impl<T> Eq for Plugin<T> where T: Eq {
}

impl<T> Hash for Plugin<T> where T: Hash {
    fn hash<H>(&self, hasher: &mut H) where H: Hasher {
        self.plugin_def.hash(hasher)
    }
}

impl<T> Borrow<String> for Plugin<T> where T: Borrow<String> {
    fn borrow(&self) -> &String {
        self.plugin_def.borrow()
    }
}

pub struct PluginManager {
    pub trigger_plugins: HashSet<Plugin<config::Trigger>>,
    pub checker_plugins: HashSet<Plugin<config::Checker>>,
    pub handler_plugins: HashSet<Plugin<config::Handler>>,
}

impl PluginManager {
    pub fn new(config: &mut TomlConfig) -> Result<Self, DemeanorError> {
        let mut trigger_hs = HashSet::new();
        for plugin_def in config.triggers.drain() {
            let plugin = Plugin {
                lib: Library::new(plugin_def.plugin_path.as_str())
                    .map_err(|e| DemeanorError::new(e))?,
                plugin_def, 
            };
            trigger_hs.insert(plugin);
        }

        let mut checker_hs = HashSet::new();
        for plugin_def in config.checkers.drain() {
            let plugin = Plugin {
                lib: Library::new(plugin_def.plugin_path.as_str())
                    .map_err(|e| DemeanorError::new(e))?,
                plugin_def,
            };
            checker_hs.insert(plugin);
        }

        let mut handler_hs = HashSet::new();
        for plugin_def in config.handlers.drain() {
            let plugin = Plugin {
                lib: Library::new(plugin_def.plugin_path.as_str())
                    .map_err(|e| DemeanorError::new(e))?,
                plugin_def,
            };
            handler_hs.insert(plugin);
        }

        Ok(PluginManager {
            trigger_plugins: trigger_hs,
            checker_plugins: checker_hs,
            handler_plugins: handler_hs,
        })
    }

    pub fn run_trigger<'a>(&self, plugin: &'a Plugin<config::Trigger>, mut req: Request<Body>)
            -> Result<(&'a String, Response<Body>, *mut libc::c_void), PluginError> {
        let callback: Symbol<unsafe extern fn(*mut Request<Body>, *mut Response<Body>)
            -> *mut libc::c_void> = unsafe { plugin.lib.get(b"trigger") }.map_err(|e| PluginError::new(500, e))?;

        let mut response = Response::new(Body::empty());
        let state = unsafe {
            callback(&mut req as *mut Request<Body>, &mut response as *mut Response<Body>)
        };
        if state.is_null() {
            return Err(PluginError::new(500, format!("Trigger plugin {} returned an error",
                                                     plugin.plugin_def.name)));
        }

        Ok((&plugin.plugin_def.next_plugin, response, state))
    }

    pub fn run_checker<'a>(&self, plugin: &'a Plugin<config::Checker>, resp: Response<Body>,
                       state: *mut libc::c_void)
            -> Result<(&'a String, Response<Body>, bool, *mut libc::c_void), PluginError> {
        let callback: Symbol<unsafe extern fn(*const libc::c_void) -> bool> =
            unsafe { plugin.lib.get(b"checker") }.map_err(|e| PluginError::new(500, e))?;

        let compliant = unsafe { callback(state) };

        Ok((&plugin.plugin_def.next_plugin, resp, compliant, state))
    }

    pub fn run_handler(&self, plugin: &Plugin<config::Handler>, resp: Response<Body>,
                       compliant: bool, state: *const libc::c_void)
            -> Result<Response<Body>, PluginError> {
        let callback: Symbol<unsafe extern fn(bool, *const libc::c_void) -> libc::c_int> =
            unsafe { plugin.lib.get(b"handler") }.map_err(|e| PluginError::new(500, e))?;

        match unsafe { callback(compliant, state) } {
            i if i == 0 => Ok(resp),
            _ => Err(PluginError::new(500, "Handler execution failed")),
        }
    }
}
