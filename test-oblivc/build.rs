extern crate oblivc;

use std::env;
use std::path::PathBuf;

fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    oblivc::compiler()
        .file("src/millionaire.oc")
        .include("src")
        .compile("millionaire");

    oblivc::bindings()
        .header("src/millionaire.h")
        .generate().unwrap()
        .write_to_file(out_path.join("millionaire.rs")).unwrap();

    println!("cargo:rerun-if-env-changed=src/millionaire.h");
    println!("cargo:rerun-if-env-changed=src/millionaire.oc");
}
