#[cfg(feature = "ruby")]
extern crate cc;
#[cfg(feature = "ruby")]
extern crate pkg_config;

#[cfg(feature = "python")]
extern crate pkg_config;
#[cfg(feature = "python")]
extern crate cc;

#[cfg(not(feature = "ruby"))]
#[cfg(not(feature = "python"))]
use std::process::exit;

#[cfg(feature = "ruby")]
fn ruby() {
    let lib = pkg_config::Config::new().probe(env!("CARGO_RUBY_VERSION")).unwrap();
    let mut builder = cc::Build::new();
    builder.file("./ruby/miss-demeanor-ruby.c").include(".");
    for include in lib.include_paths {
        builder.include(include);
    }
    builder.compile("ruby");
}

#[cfg(feature = "python")]
fn python() {}

fn main() {
    #[cfg(not(feature = "ruby"))]
    #[cfg(not(feature = "python"))]
    exit(0);

    #[cfg(feature = "ruby")]
    #[cfg(feature = "python")]
    panic!("Can only compile for Python or Ruby, not both - aborting...");

    #[cfg(feature = "ruby")]
    ruby();

    #[cfg(feature = "python")]
    python();
}
