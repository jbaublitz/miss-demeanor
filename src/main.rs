extern crate env_logger;
extern crate futures;
extern crate getopts;
extern crate hyper;
extern crate hyper_tls;
#[macro_use]
extern crate log;
extern crate native_tls;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate tokio;
extern crate tokio_io;
extern crate toml;

mod config;
mod webhook;

use std::env;
use std::error::Error;
use std::process;

pub struct Args {
    pub use_tls: webhook::UseTls,
    pub config_path: String,
}

fn parse_opts() -> Result<Args, Box<Error>> {
    let args = env::args().collect::<Vec<String>>();
    let mut options = getopts::Options::new();
    let matches = options.optopt("p", "identity-pass", "Password for SSL identity", "PASSWORD")
        .optopt("f", "identity-file", "Path to SSL pkcs12 identity file", "FILE_PATH")
        .optopt("c", "config-path", "Path to config file", "PATH")
        .parse(args[1..].iter())?;
    let use_tls = match (matches.opt_str("f"), matches.opt_str("p")) {
        (Some(file), Some(pw)) => webhook::UseTls::Yes(file, pw),
        (_, _) => webhook::UseTls::No,
    };
    let args = Args {
        use_tls,
        config_path: matches.opt_str("c").unwrap_or_else(|| {
            let path = "/etc/miss-demeanor/config.toml";
            info!("Defaulting to {}", path);
            path.to_string()
        }),
    };
    Ok(args)
}

fn main() {
    env_logger::Builder::new().filter_level(log::LevelFilter::Info).init();
    let args = match parse_opts() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    };
    let config = match config::parse_config(args.config_path) {
        Ok(c) => c,
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    };
    let server = match webhook::WebhookServer::new(config, args.use_tls, |_req| {
        Ok(hyper::Response::new(hyper::Body::from("This is a test")))
    }) {
        Ok(s) => s,
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    };
    match server.serve() {
        Ok(()) => (),
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    };
}
