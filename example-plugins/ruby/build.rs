extern crate cc;

use std::process::Command;

fn main() {
    let cmd_output = Command::new("/usr/bin/ruby").arg("./ext.rb").output().unwrap();
    let stdout = String::from_utf8(cmd_output.stdout).unwrap();
    let stderr = String::from_utf8(cmd_output.stderr).unwrap();
    println!("STDOUT:\n{}\nSTDERR:\n{}", stdout, stderr);
}
