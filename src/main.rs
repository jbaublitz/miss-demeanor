extern crate env_logger;
extern crate getopts;
extern crate hyper;
extern crate hyper_tls;
extern crate libc;
extern crate libloading;
#[macro_use]
extern crate log;
extern crate miss_demeanor_pluginutils;
extern crate native_tls;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tokio;
extern crate toml;

mod config;
mod err;
mod plugins;
mod webhook;

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::process;

pub struct Args {
    pub use_tls: webhook::UseTls,
    pub config_path: String,
}

fn parse_opts() -> Result<(webhook::UseTls, String), Box<Error>> {
    let args = env::args().collect::<Vec<String>>();
    let mut options = getopts::Options::new();
    let matches = options.optopt("p", "identity-pass", "Password for SSL identity", "PASSWORD")
        .optopt("f", "identity-file", "Path to SSL pkcs12 identity file", "FILE_PATH")
        .optopt("c", "config-path", "Path to config file", "PATH")
        .optflag("h", "help", "Print help text and exit")
        .parse(args[1..].iter())?;
    if matches.opt_present("h") {
        println!("{}", options.usage("USAGE: miss-demeanor [-f PASSWORD] [-f FILE_PATH] [-c PATH]"));
        process::exit(0);
    }
    let use_tls = match (matches.opt_str("f"), matches.opt_str("p")) {
        (Some(file_path), Some(pw)) => {
            let mut file_handle = File::open(file_path)?;
            let mut pkcs12 = Vec::new();
            file_handle.read_to_end(&mut pkcs12)?;
            webhook::UseTls::Yes(webhook::TlsIdentity::new(pkcs12, pw))
        },
        (_, _) => webhook::UseTls::No,
    };
    let args = (
        use_tls,
        matches.opt_str("c").unwrap_or_else(|| {
            let path = "/etc/miss-demeanor/config.toml";
            info!("Defaulting to {}", path);
            path.to_string()
        })
    );
    Ok(args)
}

fn main() {
    env_logger::Builder::new().filter_level(log::LevelFilter::Info).init();
    let (use_tls, config_path) = match parse_opts() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    };
    let config = match config::parse_config(config_path) {
        Ok(c) => c,
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    };

    let server = match webhook::WebhookServer::new(use_tls, config) {
        Ok(ws) => ws,
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        },
    };
    match server.serve() {
        Ok(()) => (),
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    };
}
