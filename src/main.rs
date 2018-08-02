extern crate futures;
extern crate getopts;
extern crate hyper;
extern crate hyper_tls;
extern crate native_tls;
extern crate tokio;
extern crate tokio_io;

mod webhook;

use std::env;
use std::error::Error;
use std::process;

fn parse_opts() -> Result<webhook::UseTls, Box<Error>> {
    let args = env::args().collect::<Vec<String>>();
    let mut options = getopts::Options::new();
    let matches = options.optopt("p", "identity-pass", "Password for SSL identity", "PASSWORD")
        .optopt("f", "identity-file", "Path to SSL pkcs12 identity file", "FILE_PATH")
        .parse(args[1..].iter())?;
    let use_tls = match (matches.opt_str("f"), matches.opt_str("p")) {
        (Some(file), Some(pw)) => webhook::UseTls::Yes(file, pw),
        (_, _) => webhook::UseTls::No,
    };
    Ok(use_tls)
}

fn main() {
    let use_tls = match parse_opts() {
        Ok(use_tls) => use_tls,
        Err(e) => {
            println!("{}", e);
            process::exit(1);
        }
    };
    let server = match webhook::WebhookServer::new(use_tls, |_req| {
        Ok(hyper::Response::new(hyper::Body::from("This is a test")))
    }) {
        Ok(use_tls) => use_tls,
        Err(e) => {
            println!("{}", e);
            process::exit(1);
        }
    };
    server.serve("localhost:8080".to_string()).unwrap();
}
