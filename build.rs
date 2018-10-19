#[cfg(feature = "ruby")]
extern crate cc;

#[cfg(feature = "python")]
extern crate cc;

use std::process::exit;
use std::env;

#[cfg(feature = "ruby")]
use cc::Build;

#[cfg(feature = "python")]
use cc::Build;

#[cfg(feature = "ruby")]
fn ruby() {
    Build::new().file("./ruby/miss-demeanor-ruby.c").compile("ruby.o");
}

#[cfg(feature = "python")]
fn python() {}

fn main() {
    #[cfg(not(feature = "ruby"))]
    #[cfg(not(feature = "python"))]
    exit(0);

    #[cfg(feature = "ruby")]
    ruby();

    #[cfg(feature = "python")]
    python();
}
