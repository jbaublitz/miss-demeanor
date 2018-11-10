use std::borrow::Borrow;
use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::hash::{Hash,Hasher};
use std::io::Read;

use serde::Deserialize;
use toml;

pub trait PluginConfig {
    fn get_plugin_path(&self) -> &str;
}

#[derive(Deserialize,PartialEq,Eq)]
#[serde(from="String")]
pub enum RetryStrategy {
    Linear,
    Exponential,
    UnknownStrategy,
}

impl From<String> for RetryStrategy {
    fn from(v: String) -> Self {
        match v.as_str() {
            "linear" => RetryStrategy::Linear,
            "exponential" => RetryStrategy::Exponential,
            _ => RetryStrategy::UnknownStrategy,
        }
    }
}

#[derive(Deserialize,PartialEq,Eq)]
#[serde(from="String")]
pub enum ServerType {
    Webhook,
    UnixSocket,
    UnknownServerType,
}

impl From<String> for ServerType {
    fn from(v: String) -> Self {
        match v.as_str() {
            "webhook" => ServerType::Webhook,
            "unix_socket" => ServerType::UnixSocket,
            _ => ServerType::UnknownServerType,
        }
    }
}

#[derive(Deserialize,PartialEq,Eq)]
pub struct Server {
    pub server_type: ServerType,
    pub listen_addr: String,
    pub use_tls: bool,
    pub endpoints: HashSet<Endpoint>,
}

#[derive(Deserialize,PartialEq,Eq)]
pub struct Endpoint {
    pub path: String,
    pub trigger_name: String,
}

impl Borrow<String> for Endpoint {
    fn borrow(&self) -> &String {
        &self.path
    }
}

impl Hash for Endpoint {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.path.hash(state)
    }
}

#[derive(Deserialize,PartialEq,Eq)]
pub struct Trigger {
    pub name: String,
    pub use_checker: bool,
    pub next_plugin: String,
    pub plugin_path: String,
}

impl PluginConfig for Trigger {
    fn get_plugin_path(&self) -> &str {
        self.plugin_path.as_str()
    }
}

impl Borrow<String> for Trigger {
    fn borrow(&self) -> &String {
        &self.name
    }
}

impl Hash for Trigger {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.name.hash(state)
    }
}

#[derive(Deserialize,PartialEq,Eq)]
pub struct Checker {
    pub name: String,
    pub next_plugin: String,
    pub retry_strategy: RetryStrategy,
    pub strict_evaluation: bool,
    pub plugin_path: String,
}

impl PluginConfig for Checker {
    fn get_plugin_path(&self) -> &str {
        self.plugin_path.as_str()
    }
}

impl Borrow<String> for Checker {
    fn borrow(&self) -> &String {
        &self.name
    }
}

impl Hash for Checker {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.name.hash(state)
    }
}

#[derive(Deserialize,PartialEq,Eq)]
pub struct Handler {
    pub name: String,
    pub retry_strategy: RetryStrategy,
    pub plugin_path: String,
}

impl PluginConfig for Handler {
    fn get_plugin_path(&self) -> &str {
        self.plugin_path.as_str()
    }
}

impl Borrow<String> for Handler {
    fn borrow(&self) -> &String {
        &self.name
    }
}

impl Hash for Handler {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.name.hash(state)
    }
}

#[derive(Deserialize)]
pub struct TomlConfig {
    pub server: Server,
    pub triggers: HashSet<Trigger>,
    pub checkers: HashSet<Checker>,
    pub handlers: HashSet<Handler>,
}

pub fn parse_config(file_path: String) -> Result<TomlConfig, Box<Error>> {
    let mut file = File::open(file_path)?;
    let mut file_string = String::new();
    file.read_to_string(&mut file_string)?;
    let mut deserializer = toml::Deserializer::new(file_string.as_str());
    let config = TomlConfig::deserialize(&mut deserializer)?;
    Ok(config)
}
