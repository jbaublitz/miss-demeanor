use std::borrow::Borrow;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{self,Display};
use std::hash::{Hash,Hasher};
use std::io;

use hyper::{Body,Request,Response,StatusCode};

use config::{self,TomlConfig};

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

pub trait Plugin: Sized + Send + Sync {
    type State;

    fn new(&str) -> Result<Self, io::Error>;
    fn run_trigger(&self, Request<Body>) -> Result<Self::State, PluginError>;
    fn run_checker(&self, Self::State) -> Result<(bool, Self::State), PluginError>;
    fn run_handler(&self, bool, Self::State) -> Result<(), PluginError>;
}

pub struct GenericPlugin<C, P> {
    pub config: C,
    pub plugin: P,
}

impl<C, P> PartialEq for GenericPlugin<C, P> where C: PartialEq {
    fn eq(&self, rhs: &GenericPlugin<C, P>) -> bool {
        self.config == rhs.config
    }
}

impl<C, P> Eq for GenericPlugin<C, P> where C: Eq {}

impl<C, P> Hash for GenericPlugin<C, P> where C: Hash {
    fn hash<H>(&self, hasher: &mut H) where H: Hasher {
        self.config.hash(hasher)
    }
}

impl<C, P> Borrow<String> for GenericPlugin<C, P> where C: Borrow<String> {
    fn borrow(&self) -> &String {
        self.config.borrow()
    }
}

impl<C, P> Plugin for GenericPlugin<C, P> where C: Send + Sync, P: Plugin {
    type State = P::State;

    fn new(_path: &str) -> Result<Self, io::Error> {
        unimplemented!()
    }

    fn run_trigger(&self, req: Request<Body>) -> Result<Self::State, PluginError> {
        self.plugin.run_trigger(req)
    }

    fn run_checker(&self, state: Self::State) -> Result<(bool, Self::State), PluginError> {
        self.plugin.run_checker(state)
    }

    fn run_handler(&self, compliant: bool, state: Self::State) -> Result<(), PluginError> {
        self.plugin.run_handler(compliant, state)
    }
}

pub struct PluginManager<P> {
    pub trigger_plugins: HashSet<GenericPlugin<config::Trigger, P>>,
    pub checker_plugins: HashSet<GenericPlugin<config::Checker, P>>,
    pub handler_plugins: HashSet<GenericPlugin<config::Handler, P>>,
}

impl<P> PluginManager<P> where P: Plugin {
    pub fn new(config: &mut TomlConfig) -> Result<Self, io::Error> {
        let mut trigger_hs = HashSet::new();
        for trigger in config.triggers.drain() {
            let plugin = P::new(&trigger.plugin_path)?;
            trigger_hs.insert(GenericPlugin { config: trigger, plugin });
        }

        let mut checker_hs = HashSet::new();
        for checker in config.checkers.drain() {
            let plugin = P::new(&checker.plugin_path)?;
            checker_hs.insert(GenericPlugin { config: checker, plugin });
        }

        let mut handler_hs = HashSet::new();
        for handler in config.handlers.drain() {
            let plugin = P::new(&handler.plugin_path)?;
            handler_hs.insert(GenericPlugin { config: handler, plugin });
        }

        Ok(PluginManager {
            trigger_plugins: trigger_hs,
            checker_plugins: checker_hs,
            handler_plugins: handler_hs,
        })
    }

    pub fn exec_trigger_plugin(&self, name: &String, req: Request<Body>)
            -> Result<(&String, P::State), PluginError> {
        let plugin = self.trigger_plugins.get(name)
            .ok_or(PluginError::new(500, format!("Failed to find trigger plugin {}",
                                                 name)))?;
        let state = plugin.run_trigger(req)?;
        Ok((&plugin.config.next_plugin, state))
    }

    pub fn exec_checker_plugin(&self, name: &String, state: P::State)
            -> Result<(&String, bool, P::State), PluginError> {
        let plugin = self.checker_plugins.get(name)
            .ok_or(PluginError::new(500, format!("Failed to find checker plugin {}",
                                                  name)))?;
        let (comp, state) = plugin.run_checker(state)?;
        Ok((&plugin.config.next_plugin, comp, state))
    }

    pub fn exec_handler_plugin(&self, name: &String, compliant: bool, state: P::State)
            -> Result<Response<Body>, PluginError> {
        let plugin = self.handler_plugins.get(name)
            .ok_or(PluginError::new(500, format!("Failed to find trigger plugin {}",
                                                  name)))?;
        plugin.run_handler(compliant, state)?;
        Ok(Response::new(Body::from("Success!")))
    }
}
