use std::collections::HashMap;
use std::error::Error;

pub struct Trigger {
    pub name: String,
    pub url_path: String,
}

pub struct Checker {
    pub name: String,

}

pub struct TomlConfig {
    triggers: HashMap<String, Trigger>,
    checkers: HashMap<String, Checker>,
}

pub fn parse_config() -> Result<TomlConfig, Box<Error>> {
    Ok(TomlConfig { triggers: HashMap::new(), checkers: HashMap::new() })
}
