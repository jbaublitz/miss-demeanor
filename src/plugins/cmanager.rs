use std::collections::{HashMap,HashSet};
use std::error::Error;
use std::ffi::CString;
use std::io;

use hyper::{Body,Request,Response};
use tokio::prelude::Stream;
use tokio::runtime::Runtime;

use config::{self,Endpoint,TomlConfig};
use err::DemeanorError;
use plugins::{CABIPlugin,CRequest,Plugin,PluginManager};

pub struct CABIPluginManager<'a> {
    runtime: &'a mut Runtime,
    endpoints: HashSet<Endpoint>,
    trigger_plugins: HashSet<CABIPlugin<config::Trigger>>,
    checker_plugins: HashSet<CABIPlugin<config::Checker>>,
    handler_plugins: HashSet<CABIPlugin<config::Handler>>,
}

impl<'a> CABIPluginManager<'a> {
    pub fn new(runtime: &'a mut Runtime, config: &mut TomlConfig) -> Result<Self, io::Error> {
        let mut trigger_hs = HashSet::new();
        for trigger in config.triggers.drain() {
            trigger_hs.insert(CABIPlugin::new(trigger)?);
        }

        let mut checker_hs = HashSet::new();
        for checker in config.checkers.drain() {
            checker_hs.insert(CABIPlugin::new(checker)?);
        }

        let mut handler_hs = HashSet::new();
        for handler in config.handlers.drain() {
            handler_hs.insert(CABIPlugin::new(handler)?);
        }

        let mut endpoints = HashSet::new();
        for endpoint in config.server.endpoints.drain() {
            endpoints.insert(endpoint);
        }

        Ok(CABIPluginManager {
            runtime,
            endpoints,
            trigger_plugins: trigger_hs,
            checker_plugins: checker_hs,
            handler_plugins: handler_hs,
        })
    }
}

impl<'a> PluginManager for CABIPluginManager<'a> {
    fn process_request(&mut self, req: Request<Body>) -> Result<(), Box<Error>> {
        let method = CString::new(req.method().as_str().to_string())?;
        let uri = req.uri().to_string();
        let name = self.endpoints.get(&uri).map(|e| &e.path).ok_or(DemeanorError::new("Endpoint not found"))?;
        let uri_cstring = CString::new(uri)?;
        let mut headers = HashMap::new();
        for (header, value) in req.headers() {
            let header_cstring = CString::new(header.to_string())?;
            let value_cstring = CString::new(value.to_str()?.to_string())?;
            headers.insert(header_cstring, value_cstring);
        }
        let body = CString::new(self.runtime.block_on(req.into_body().concat2())?.to_vec())?;
        let crequest = CRequest {
            method,
            uri: uri_cstring,
            headers,
            body,
        };

        let trigger_plugin = self.trigger_plugins.get(name)
            .ok_or(DemeanorError::new(format!("Failed to find trigger plugin {}",
                                              name)))?;
        let state = trigger_plugin.run_trigger(crequest)?;

        let checker_plugin = self.checker_plugins.get(&trigger_plugin.config.next_plugin)
            .ok_or(DemeanorError::new(format!("Failed to find checker plugin {}",
                                              name)))?;
        let (state, compliant) = checker_plugin.run_checker(state)?;

        let handler_plugin = self.handler_plugins.get(&checker_plugin.config.next_plugin)
            .ok_or(DemeanorError::new(format!("Failed to find trigger plugin {}",
                                              name)))?;
        handler_plugin.run_handler(state, compliant)?;
        Ok(())
    }
}
