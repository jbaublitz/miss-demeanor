use std::borrow::Borrow;
use std::collections::HashSet;
use std::error::Error;
use std::hash::{Hash,Hasher};

use hyper::{Body,HeaderMap,Request,Response};
use serde_json::Value;

use config::{self,Endpoint,TomlConfig};
use err::DemeanorError;
use plugins::{PluginError,PluginManager,RubyPlugin};

pub struct RubyPluginManager {
    endpoints: HashSet<Endpoint>,
    pub trigger_plugins: HashSet<RubyPlugin<config::Trigger>>,
    pub checker_plugins: HashSet<RubyPlugin<config::Checker>>,
    pub handler_plugins: HashSet<RubyPlugin<config::Handler>>,
}

impl RubyPluginManager {
    pub fn new(config: &mut TomlConfig) -> Result<Self, Box<Error>> {
        let mut trigger_hs = HashSet::new();
        for trigger in config.triggers.drain() {
            trigger_hs.insert(RubyPlugin::new(trigger));
        }

        let mut checker_hs = HashSet::new();
        for checker in config.checkers.drain() {
            checker_hs.insert(RubyPlugin::new(checker));
        }

        let mut handler_hs = HashSet::new();
        for handler in config.handlers.drain() {
            handler_hs.insert(RubyPlugin::new(handler));
        }

        let mut endpoints = HashSet::new();
        for endpoint in config.server.endpoints.drain() {
            endpoints.insert(endpoint);
        }

        Ok(RubyPluginManager {
            endpoints,
            trigger_plugins: trigger_hs,
            checker_plugins: checker_hs,
            handler_plugins: handler_hs,
        })
    }
}

impl<C> Hash for RubyPlugin<C> where C: Hash {
    fn hash<H>(&self, hasher: &mut H) where H: Hasher {
        self.config.hash(hasher)
    }
}

impl<C> PartialEq for RubyPlugin<C> where C: PartialEq {
    fn eq(&self, rhs: &Self) -> bool {
        self.config == rhs.config
    }
}

impl<C> Eq for RubyPlugin<C> where C: Eq {}

impl<C> Borrow<String> for RubyPlugin<C> where C: Borrow<String> {
    fn borrow(&self) -> &String {
        self.config.borrow()
    }
}

impl PluginManager for RubyPluginManager {
    fn process_request(&mut self, req: Request<Body>) -> Result<(), Box<Error>> {
        Ok(())
    }
}
