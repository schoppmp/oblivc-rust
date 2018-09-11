extern crate oblivc;

use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Compile `millionaire.oc` using oblivcc
    oblivc::compiler()
        .file("src/millionaire.oc")
        .include("src")
        .compile("millionaire");

    // Generate Rust bindings for the Obliv-C function and struct in `millionaire.h`, then
    // write them to `OUT_DIR/millionaire.rs`.
    oblivc::bindings()
        .header("src/millionaire.h")
        .generate()
        .unwrap()
        .write_to_file(out_dir.join("millionaire.rs"))
        .unwrap();

    // Rebuild if either of the files change
    println!("cargo:rerun-if-changed=src/millionaire.h");
    println!("cargo:rerun-if-changed=src/millionaire.oc");
}
