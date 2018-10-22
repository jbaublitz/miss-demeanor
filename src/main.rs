extern crate env_logger;
extern crate futures;
extern crate getopts;
extern crate http;
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
extern crate tokio;
extern crate toml;

mod config;
mod plugins;
mod webhook;

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::process;

use config::PluginType;
use plugins::{CABIPlugin,PluginManager};
#[cfg(feature = "python")]
use plugins::PythonPlugin;
#[cfg(feature = "ruby")]
use plugins::RubyPlugin;

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

#[cfg(feature = "ruby")]
extern "C" {
    fn start_miss_demeanor_ruby() -> libc::c_int;
    fn run_ruby_trigger(request: *const libc::c_void) -> libc::c_ulong;
    fn is_nil(id: libc::c_ulong) -> libc::c_int;
    fn cleanup_miss_demeanor_ruby();
}

macro_rules! match_and_serve (
    ($plugin_ty:ty, $args: ident, $config: ident) => {{
        let plugin_manager = match PluginManager::<$plugin_ty>::new(&mut $config) {
            Ok(pm) => pm,
            Err(e) => {
                error!("{}", e);
                process::exit(1);
            }
        };
        let server = match webhook::WebhookServer::new($args.use_tls, plugin_manager) {
            Ok(s) => s,
            Err(e) => {
                error!("{}", e);
                process::exit(1);
            }
        };
        match server.serve($config) {
            Ok(()) => (),
            Err(e) => {
                error!("{}", e);
                process::exit(1);
            }
        };
    }}
);

fn main() {
    env_logger::Builder::new().filter_level(log::LevelFilter::Info).init();
    let args = match parse_opts() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    };
    let mut config = match config::parse_config(args.config_path) {
        Ok(c) => c,
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    };

    #[cfg(feature = "ruby")]
    {
        if unsafe { start_miss_demeanor_ruby() } < 0 {
            error!("Failed to start embedded Ruby");
            process::exit(1);
        };
    }

    match config.plugin_type {
        PluginType::CABI => match_and_serve!(CABIPlugin, args, config),
        #[cfg(feature = "python")]
        PluginType::Python => match_and_serve!(PythonPlugin, args, config),
        #[cfg(feature = "ruby")]
        PluginType::Ruby => match_and_serve!(RubyPlugin, args, config),
        _ => {
            error!("Unknown plugin type");
            process::exit(1);
        }
    };
}
