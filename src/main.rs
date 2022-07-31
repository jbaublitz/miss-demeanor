#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

mod config;
mod err;
mod plugins;
mod webhook;

use std::{env, error::Error, fs::File, io::Read, process};

use config::TriggerType;
use err::DemeanorError;
use plugins::{CABIPlugin, InterpretedPlugin};
use webhook::UseTls;

pub struct Args {
    pub use_tls: webhook::UseTls,
    pub config_path: String,
}

fn parse_opts() -> Result<(webhook::UseTls, String), Box<dyn Error>> {
    let args = env::args().collect::<Vec<String>>();
    let mut options = getopts::Options::new();
    let matches = options
        .optopt(
            "f",
            "identity-file",
            "Path to SSL pkcs12 identity file",
            "FILE_PATH",
        )
        .optopt("c", "config-path", "Path to config file", "PATH")
        .optflag("h", "help", "Print help text and exit")
        .parse(args[1..].iter())?;
    if matches.opt_present("h") {
        println!(
            "{}",
            options.usage("USAGE: miss-demeanor [-f PASSWORD] [-f FILE_PATH] [-c PATH]")
        );
        process::exit(0);
    }
    let use_tls = match (matches.opt_str("f"), env::var("PKCS12_PASSWORD")) {
        (Some(file_path), Ok(pw)) => {
            let mut file_handle = File::open(file_path)?;
            let mut pkcs12 = Vec::new();
            file_handle.read_to_end(&mut pkcs12)?;
            webhook::UseTls::Yes(webhook::TlsIdentity::new(pkcs12, pw))
        }
        (_, _) => webhook::UseTls::No,
    };
    let args = (
        use_tls,
        matches.opt_str("c").unwrap_or_else(|| {
            let path = "/etc/miss-demeanor/config.toml";
            info!("Defaulting to {}", path);
            path.to_string()
        }),
    );
    Ok(args)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let (mut use_tls, config_path) = parse_opts()?;
    let config = config::parse_config(config_path)?;
    if config.server.use_tls && !use_tls.use_tls() {
        error!("Server requires options -f for TLS identity file and PKCS12_PASSWORD environment variable");
        return Err(
            Box::new(DemeanorError::new("Missing required options for TLS")) as Box<dyn Error>,
        );
    } else if !config.server.use_tls && use_tls.use_tls() {
        error!("Server specified no TLS but TLS CLI parameters were provided; ignoring");
        use_tls = UseTls::No;
    }

    if let TriggerType::CAbi = config.trigger_type {
        let server = webhook::WebhookServer::<CABIPlugin>::new(use_tls, config)?;
        server.serve().await
    } else if let TriggerType::Interpreted = config.trigger_type {
        let server = webhook::WebhookServer::<InterpretedPlugin>::new(use_tls, config)?;
        server.serve().await
    } else {
        error!("Unrecognized trigger type: {}", config.trigger_type);
        Err(Box::new(DemeanorError::new(format!(
            "Unrecognized trigger type: {}",
            config.trigger_type
        ))))
    }
}
