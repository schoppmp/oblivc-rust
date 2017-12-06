extern crate walkdir;
extern crate bindgen;


use std::process::Command;
use std::path::{Path,PathBuf};
use std::env;
use walkdir::WalkDir;

macro_rules! t {
    ($e:expr) => (match $e{
        Ok(e) => e,
        Err(e) => panic!("\"{}\" failed with \"{}\"", stringify!($e), e),
    })
}

fn main() {
    // check for OBLIVC_PATH
    let oblivc_path_string = env::var("OBLIVC_PATH").ok().and_then(|s| {
        if s == "" {None} else {Some(s)}
    });
    let oblivc_path = match oblivc_path_string.as_ref() {
        Some(s) => Path::new(s),
        None => {
            // update submodule
            if !Path::new("obliv-c/.git").exists() {
                let status = t!(Command::new("git").args(&["submodule", "update", "--init"])
                                     .status());
                if !status.success() {
                    panic!("Updating submodules failed");
                };
            }
            Path::new("obliv-c")
        },
    };

    // build obliv-c
    if !oblivc_path.join("Makefile").exists() {
        let status = t!(Command::new("./configure").current_dir(oblivc_path).status());
        if !status.success() {
            panic!("Configuring Obliv-C failed");
        }
    }
    let status = t!(Command::new("make").current_dir(oblivc_path).status());
    if !status.success() {
        panic!("Building Obliv-C failed");
    }

    // register to rebuild when something changes
    for file in WalkDir::new(oblivc_path.join("src"))
                        .into_iter()
                        .filter_map(|e| e.ok()) {
        if let Some(s) = file.path().to_str() {
            println!("cargo:rerun-if-changed={}", s);
        }
    }
    println!("cargo:rerun-if-changed={}", oblivc_path.join("_build/libobliv.a").to_str().unwrap());

    // add obliv-c source directory to CPATH
    let mut paths = Vec::<PathBuf>::new();
    if let Some(path) = env::var_os("CPATH") {
        paths = env::split_paths(&path).collect();
    }
    paths.push(oblivc_path.join("src/ext/oblivc"));
    let new_path = env::join_paths(paths).unwrap();
    env::set_var("CPATH", &new_path);

    // generate bindings
    let bindings = bindgen::Builder::default()
        .header(oblivc_path.join("src/ext/oblivc/obliv.h").to_str().unwrap())
        .whitelisted_type("ProtocolDesc") // TODO: whitelist everything directly in obliv.h
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
