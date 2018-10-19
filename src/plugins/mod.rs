use std::borrow::Borrow;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{self,Display};
use std::hash::{Hash,Hasher};
use std::io;

use hyper::{Body,Request,Response,StatusCode};
use libc;

use config::{self,PluginConfig,PluginType,TomlConfig};

mod cabi;
pub use self::cabi::CABIPlugin;

#[cfg(feature = "python")]
mod python;
#[cfg(feature = "python")]
pub use self::python::PythonPlugin;

#[cfg(feature = "ruby")]
mod ruby;
#[cfg(feature = "ruby")]
pub use self::ruby::RubyPlugin;

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

pub trait PluginAPI {
    fn run_trigger(&self, Request<Body>) -> Result<*mut libc::c_void, PluginError>;
    fn run_checker(&self, *mut libc::c_void) -> Result<(bool, *mut libc::c_void), PluginError>;
    fn run_handler(&self, bool, *mut libc::c_void) -> Result<(), PluginError>;
}

pub enum Plugin<T> {
    CABI(T, CABIPlugin),
    #[cfg(feature = "ruby")]
    Ruby(T, RubyPlugin),
    #[cfg(feature = "python")]
    Python(T, PythonPlugin),
}

impl<T> Plugin<T> {
    fn get_config(&self) -> &T {
        match *self {
            Plugin::CABI(ref config, _) => config,
            #[cfg(feature = "ruby")]
            Plugin::Ruby(ref config, _) => config,
            #[cfg(feature = "python")]
            Plugin::Python(ref config, _) => config,
        }
    }
}

impl<T> PartialEq for Plugin<T> where T: PartialEq {
    fn eq(&self, rhs: &Plugin<T>) -> bool {
        match (self, rhs) {
            (&Plugin::CABI(ref config1, _), &Plugin::CABI(ref config2, _)) => {
                config1 == config2
            },
            #[cfg(feature = "ruby")]
            (&Plugin::Ruby(ref config1, _), &Plugin::Ruby(ref config2, _)) => {
                config1 == config2
            },
            #[cfg(feature = "python")]
            (&Plugin::Python(ref config1, _), &Plugin::Python(ref config2, _)) => {
                config1 == config2
            },
        }
    }
}

impl<T> Eq for Plugin<T> where T: Eq {}

impl<T> Hash for Plugin<T> where T: Hash {
    fn hash<H>(&self, hasher: &mut H) where H: Hasher {
        match *self {
            Plugin::CABI(ref config, _) => config.hash(hasher),
            #[cfg(feature = "ruby")]
            Plugin::Ruby(ref config, _) => config.hash(hasher),
            #[cfg(feature = "python")]
            Plugin::Python(ref config, _) => config.hash(hasher),
        }
    }
}

impl<T> Borrow<String> for Plugin<T> where T: Borrow<String> {
    fn borrow(&self) -> &String {
        match *self {
            Plugin::CABI(ref config, _) => config.borrow(),
            #[cfg(feature = "ruby")]
            Plugin::Ruby(ref config, _) => config.borrow(),
            #[cfg(feature = "python")]
            Plugin::Python(ref config, _) => config.borrow(),
        }
    }
}

impl<T> PluginAPI for Plugin<T> {
    fn run_trigger(&self, req: Request<Body>)
            -> Result<(*mut libc::c_void), PluginError> {
        match *self {
            Plugin::CABI(_, ref plugin) => plugin.run_trigger(req),
            #[cfg(feature = "ruby")]
            Plugin::Ruby(_, ref plugin) => plugin.run_trigger(req),
            #[cfg(feature = "python")]
            Plugin::Python(_, ref plugin) => plugin.run_trigger(req),
        }
    }

    fn run_checker(&self, state: *mut libc::c_void)
            -> Result<(bool, *mut libc::c_void), PluginError> {
        match *self {
            Plugin::CABI(_, ref plugin) => plugin.run_checker(state),
            #[cfg(feature = "ruby")]
            Plugin::Ruby(_, ref plugin) => plugin.run_checker(state),
            #[cfg(feature = "python")]
            Plugin::Python(_, ref plugin) => plugin.run_checker(state),
        }
    }

    fn run_handler(&self, compliant: bool, state: *mut libc::c_void)
            -> Result<(), PluginError> {
        match *self {
            Plugin::CABI(_, ref plugin) => plugin.run_handler(compliant, state),
            #[cfg(feature = "ruby")]
            Plugin::Ruby(_, ref plugin) => plugin.run_handler(compliant, state),
            #[cfg(feature = "python")]
            Plugin::Python(_, ref plugin) => plugin.run_handler(compliant, state),
        }
    }
}

pub struct PluginManager {
    pub trigger_plugins: HashSet<Plugin<config::Trigger>>,
    pub checker_plugins: HashSet<Plugin<config::Checker>>,
    pub handler_plugins: HashSet<Plugin<config::Handler>>,
}

impl PluginManager {
    pub fn new(config: &mut TomlConfig) -> Result<Self, io::Error> {
        let mut trigger_hs = HashSet::new();
        for trigger in config.triggers.drain() {
            trigger_hs.insert(Self::open_plugin(trigger)?);
        }

        let mut checker_hs = HashSet::new();
        for checker in config.checkers.drain() {
            checker_hs.insert(Self::open_plugin(checker)?);
        }

        let mut handler_hs = HashSet::new();
        for handler in config.handlers.drain() {
            handler_hs.insert(Self::open_plugin(handler)?);
        }

        Ok(PluginManager {
            trigger_plugins: trigger_hs,
            checker_plugins: checker_hs,
            handler_plugins: handler_hs,
        })
    }

    fn open_plugin<C>(config: C) -> Result<Plugin<C>, io::Error>
            where C: PluginConfig {
        match *config.get_plugin_type() {
            PluginType::CABI => {
                let plugin = CABIPlugin::new(config.get_plugin_path())?;
                Ok(Plugin::CABI(config, plugin))
            },
            #[cfg(feature = "python")]
            PluginType::Python => {
                let plugin = PythonPlugin::new(config.get_plugin_path())?;
                Ok(Plugin::Python(config, plugin))
            },
            #[cfg(feature = "ruby")]
            PluginType::Ruby => {
                let plugin = RubyPlugin::new(config.get_plugin_path())?;
                Ok(Plugin::Ruby(config, plugin))
            },
            PluginType::UnknownPluginType => {
                Err(io::Error::from(io::ErrorKind::InvalidInput))
            },
        }
    }

    pub fn exec_trigger_plugin(&self, name: &String, req: Request<Body>)
            -> Result<(&String, *mut libc::c_void), PluginError> {
        let plugin = self.trigger_plugins.get(name)
            .ok_or(PluginError::new(500, format!("Failed to find trigger plugin {}",
                                                 name)))?;
        let state = plugin.run_trigger(req)?;
        Ok((&plugin.get_config().next_plugin, state))
    }

    pub fn exec_checker_plugin(&self, name: &String, state: *mut libc::c_void)
            -> Result<(&String, bool, *mut libc::c_void), PluginError> {
        let plugin = self.checker_plugins.get(name)
            .ok_or(PluginError::new(500, format!("Failed to find checker plugin {}",
                                                  name)))?;
        let (comp, state) = plugin.run_checker(state)?;
        Ok((&plugin.get_config().next_plugin, comp, state))
    }

    pub fn exec_handler_plugin(&self, name: &String, compliant: bool,
                               state: *mut libc::c_void)
            -> Result<Response<Body>, PluginError> {
        let plugin = self.handler_plugins.get(name)
            .ok_or(PluginError::new(500, format!("Failed to find trigger plugin {}",
                                                  name)))?;
        plugin.run_handler(compliant, state)?;
        Ok(Response::new(Body::from("Success!")))
    }
}
