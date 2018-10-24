use std::collections::HashSet;
use std::error::Error;
use std::fmt::{self,Display};
use std::io::{self,Write};
use std::os::unix::io::{FromRawFd,AsRawFd};
use std::os::unix::net::UnixStream;
use std::process::{Command,Stdio};

use hyper::{Body,Request,Response,StatusCode};
use serde_json::Value;
use tokio::io::AsyncWrite;
use tokio::prelude::{future,stream,Async,Future,Stream};

use config::{self,TomlConfig};

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


fn run_trigger(config: &config::Trigger, req: Request<Body>) -> Result<Value, PluginError> {
    let mut cmd = Command::new(&config.plugin_path);
    let (mut ipc_main, ipc_proc) = UnixStream::pair().map_err(|e| PluginError::new(500, e))?;

    let mut json = json!({
        "method": req.method().as_str(),
        "uri": req.uri().to_string(),
        "headers": {}
    });
    for (header, value) in req.headers() {
        let headers = json.get_mut("headers");
        if let Some(Value::Object(map)) = headers {
            map.insert(header.to_string(), Value::from(value.to_str().map_err(|e| {
                PluginError::new(500, e)
            })?));
        }
    }
    cmd.stdin(unsafe { Stdio::from_raw_fd(ipc_proc.as_raw_fd()) });

    ipc_main.write(json.to_string().as_bytes()).map_err(|e| PluginError::new(500, e))?;
    tokio::spawn(req.into_body().concat2().map_err(|e| {
        error!("{}", e);
        ()
    }).and_then(move |b| {
        ipc_main.write(&b).map(|_| ()).map_err(|e| {
            error!("{}", e);
            ()
        })
    }));

    let output = cmd.output().map_err(|e| PluginError::new(500, e))?;
    if !cmd.status().map_err(|e| PluginError::new(500, e))?.success() {
        error!("{}", String::from_utf8(cmd.output().map_err(|e| PluginError::new(500, e))?
                                       .stderr).map_err(|e| PluginError::new(500, e))?);
        return Err(PluginError::new(500, "Checker plugin exited unsuccessfully"));
    }
    Ok(serde_json::from_slice(&output.stdout)
                   .map_err(|e| PluginError::new(500, e))?)
}

fn run_checker(config: &config::Checker, state: Value)
        -> Result<Value, PluginError> {
    let mut cmd = Command::new(&config.plugin_path);
    let (mut ipc_main, ipc_proc) = UnixStream::pair().map_err(|e| PluginError::new(500, e))?;
    cmd.stdin(unsafe { Stdio::from_raw_fd(ipc_proc.as_raw_fd()) });
    ipc_main.write(state.to_string().as_bytes()).map_err(|e| PluginError::new(500, e))?;
    if !cmd.status().map_err(|e| PluginError::new(500, e))?.success() {
        error!("{}", String::from_utf8(cmd.output().map_err(|e| PluginError::new(500, e))?
                                       .stderr).map_err(|e| PluginError::new(500, e))?);
        return Err(PluginError::new(500, "Checker plugin exited unsuccessfully"));
    }
    let output = cmd.output().map_err(|e| PluginError::new(500, e))?;
    Ok(serde_json::from_slice(&output.stdout)
                   .map_err(|e| PluginError::new(500, e))?)
}

fn run_handler(config: &config::Handler, state: Value)
        -> Result<(), PluginError> {
    let mut cmd = Command::new(&config.plugin_path);
    let (mut ipc_main, ipc_proc) = UnixStream::pair().map_err(|e| PluginError::new(500, e))?;
    cmd.stdin(unsafe { Stdio::from_raw_fd(ipc_proc.as_raw_fd()) });
    ipc_main.write(state.to_string().as_bytes()).map_err(|e| PluginError::new(500, e))?;
    if cmd.status().map_err(|e| PluginError::new(500, e))?.success() {
        Ok(())
    } else {
        error!("{}", String::from_utf8(cmd.output().map_err(|e| PluginError::new(500, e))?
                                       .stderr).map_err(|e| PluginError::new(500, e))?);
        Err(PluginError::new(500, "Handler plugin exited unsuccessfully"))
    }
}

pub struct PluginManager {
    pub trigger_plugins: HashSet<config::Trigger>,
    pub checker_plugins: HashSet<config::Checker>,
    pub handler_plugins: HashSet<config::Handler>,
}

impl PluginManager {
    pub fn new(config: &mut TomlConfig) -> Result<Self, io::Error> {
        let mut trigger_hs = HashSet::new();
        for trigger in config.triggers.drain() {
            trigger_hs.insert(trigger);
        }

        let mut checker_hs = HashSet::new();
        for checker in config.checkers.drain() {
            checker_hs.insert(checker);
        }

        let mut handler_hs = HashSet::new();
        for handler in config.handlers.drain() {
            handler_hs.insert(handler);
        }

        Ok(PluginManager {
            trigger_plugins: trigger_hs,
            checker_plugins: checker_hs,
            handler_plugins: handler_hs,
        })
    }

    pub fn exec_trigger_plugin(&self, name: &String, req: Request<Body>)
            -> Result<(&String, bool, Value), PluginError> {
        let config = self.trigger_plugins.get(name)
            .ok_or(PluginError::new(500, format!("Failed to find trigger plugin {}",
                                                 name)))?;
        let state = run_trigger(config, req)?;
        Ok((&config.next_plugin, config.use_checker, state))
    }

    pub fn exec_checker_plugin(&self, name: &String, state: Value)
            -> Result<(&String, Value), PluginError> {
        let config = self.checker_plugins.get(name)
            .ok_or(PluginError::new(500, format!("Failed to find checker plugin {}",
                                                  name)))?;
        let state = run_checker(config, state)?;
        Ok((&config.next_plugin, state))
    }

    pub fn exec_handler_plugin(&self, name: &String, state: Value)
            -> Result<Response<Body>, PluginError> {
        let config = self.handler_plugins.get(name)
            .ok_or(PluginError::new(500, format!("Failed to find trigger plugin {}",
                                                  name)))?;
        run_handler(config, state)?;
        Ok(Response::new(Body::from("Success!")))
    }
}
