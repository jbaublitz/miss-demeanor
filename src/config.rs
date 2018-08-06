use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::hash::{Hash,Hasher};
use std::io::Read;

use serde::Deserialize;
use toml;

#[derive(Deserialize,PartialEq,Eq)]
#[serde(from="String")]
pub enum TriggerType {
    Webhook,
    UnixSocket,
    UnknownTriggerType,
}

impl From<String> for TriggerType {
    fn from(v: String) -> Self {
        match v.as_str() {
            "webhook" => TriggerType::Webhook,
            "unix_socket" => TriggerType::UnixSocket,
            _ => TriggerType::UnknownTriggerType,
        }
    }
}

#[derive(Deserialize,PartialEq,Eq)]
pub struct Trigger {
    pub name: String,
    #[serde(rename = "type")]
    pub trigger_type: TriggerType,
    pub url_path: String,
}

impl Hash for Trigger {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.name.hash(state)
    }
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
pub struct Checker {
    pub name: String,
    pub retry_strategy: RetryStrategy,
    pub strict_evaluation: bool,
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
}

impl Hash for Handler {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.name.hash(state)
    }
}

#[derive(Deserialize)]
pub struct TomlConfig {
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
