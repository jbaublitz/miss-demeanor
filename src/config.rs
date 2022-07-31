use std::borrow::Borrow;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{self, Display};
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::Read;

use serde::Deserialize;

pub trait PluginConfig {
    fn get_plugin_path(&self) -> &str;
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(from = "String")]
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

#[derive(Deserialize, PartialEq, Eq)]
pub struct Server {
    pub server_type: ServerType,
    pub listen_addr: String,
    pub use_tls: bool,
    pub endpoints: HashSet<Endpoint>,
}

#[derive(Deserialize, Eq)]
pub struct Endpoint {
    pub path: String,
    pub trigger_name: String,
}

impl Borrow<String> for Endpoint {
    fn borrow(&self) -> &String {
        &self.path
    }
}

impl PartialEq<Endpoint> for Endpoint {
    fn eq(&self, rhs: &Endpoint) -> bool {
        self.path == rhs.path
    }
}

impl Hash for Endpoint {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.path.hash(state)
    }
}

#[derive(Deserialize, Eq)]
pub struct Trigger {
    pub name: String,
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

impl PartialEq<Trigger> for Trigger {
    fn eq(&self, rhs: &Trigger) -> bool {
        self.name == rhs.name
    }
}

impl Hash for Trigger {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.name.hash(state)
    }
}

impl From<String> for TriggerType {
    fn from(v: String) -> Self {
        match v.as_str() {
            "c_abi" => TriggerType::CAbi,
            "interpreted" => TriggerType::Interpreted,
            _ => TriggerType::UnknownTriggerType(v),
        }
    }
}

impl Display for TriggerType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TriggerType::CAbi => write!(f, "C ABI"),
            TriggerType::Interpreted => write!(f, "Interpreted"),
            TriggerType::UnknownTriggerType(ref s) => write!(f, "{}", s),
        }
    }
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(from = "String")]
pub enum TriggerType {
    CAbi,
    Interpreted,
    UnknownTriggerType(String),
}

#[derive(Deserialize)]
pub struct TomlConfig {
    pub trigger_type: TriggerType,
    pub server: Server,
    pub triggers: HashSet<Trigger>,
}

pub fn parse_config(file_path: String) -> Result<TomlConfig, Box<dyn Error>> {
    let mut file = File::open(file_path)?;
    let mut file_string = String::new();
    file.read_to_string(&mut file_string)?;
    let mut deserializer = toml::Deserializer::new(file_string.as_str());
    let config = TomlConfig::deserialize(&mut deserializer)?;
    Ok(config)
}
