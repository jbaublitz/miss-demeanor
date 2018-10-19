#[cfg(any(feature = "ruby", feature = "python"))] extern crate cc;
#[cfg(any(feature = "ruby", feature = "python"))]
extern crate pkg_config;

#[cfg(not(any(feature = "ruby", feature = "python")))]
use std::process::exit;

#[cfg(feature = "ruby")]
fn ruby() {
    use std::env;

    let lib = pkg_config::Config::new().probe(env!("CARGO_RUBY_VERSION")).unwrap();

    let mut builder = cc::Build::new();
    builder.file("./ruby/miss-demeanor-ruby.c");
    for include in lib.include_paths.iter() {
        builder.include(include);
    }
    builder.compile("ruby");

    println!("cargo:rustc-link-search=native=./target/{}/deps/", env::var("PROFILE").unwrap());
    println!("cargo:rustc-link-lib=dylib=missdemeanorpluginutils");
}

#[cfg(feature = "python")]
fn python() {}

fn main() {
    #[cfg(not(any(feature = "ruby", feature = "python")))]
    exit(0);

    #[cfg(feature = "ruby")]
    ruby();

    #[cfg(feature = "python")]
    python();
}
