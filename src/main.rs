extern crate env_logger;
extern crate getopts;
extern crate hyper;
extern crate hyper_tls;
extern crate libc;
extern crate libloading;
#[macro_use]
extern crate log;
extern crate native_tls;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate tokio;
extern crate tokio_signal;
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
use std::sync::mpsc::channel;

use tokio::prelude::{Future,IntoFuture,Stream};
use tokio::prelude::future::lazy;
use tokio_signal::unix::{Signal,SIGINT};

use config::PluginType;
use plugins::CABIPluginManager;
#[cfg(feature = "ruby")]
use plugins::RubyPluginManager;

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

#[cfg(feature = "ruby")]
extern "C" {
    fn ruby_setup() -> libc::c_int;
    fn ruby_default_signal(sig: libc::c_int);
    fn ruby_finalize();
}

fn main() {
    let mut runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    };

    #[cfg(feature = "ruby")]
    {
        runtime.spawn(Signal::new(SIGINT).flatten_stream().for_each(|_| {
            unsafe { ruby_default_signal(libc::SIGINT) };
            Ok(())
        }).map_err(|e| {
            error!("{}", e);
            ()
        }));

        let state = unsafe { ruby_setup() };
        if state != 0 {
            error!("Failed to initialize Ruby");
            process::exit(1);
        }
    }

    env_logger::Builder::new().filter_level(log::LevelFilter::Info).init();
    let (use_tls, config_path) = match parse_opts() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    };
    let mut config = match config::parse_config(config_path) {
        Ok(c) => c,
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    };

    let (sender, receiver) = channel();
    let server = match webhook::WebhookServer::new(use_tls) {
        Ok(ws) => ws,
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        },
    };
    match config.plugin_type {
        #[cfg(feature = "ruby")]
        PluginType::Ruby => {
            let pm = match RubyPluginManager::new(&mut config) {
                Ok(pm) => pm,
                Err(e) => {
                    error!("{}", e);
                    process::exit(1);
                },
            };
        },
        PluginType::CABI => {
            let pm = match CABIPluginManager::new(&mut runtime, &mut config) {
                Ok(pm) => pm,
                Err(e) => {
                    error!("{}", e);
                    process::exit(1);
                },
            };
        },
        PluginType::UnknownPluginType => {
            error!("Unknown plugin type - exiting...");
            process::exit(1);
        }
    };
    match server.serve(&mut runtime, sender, config.server) {
        Ok(()) => (),
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    };
    #[cfg(feature = "ruby")]
    unsafe { ruby_finalize() };
}
