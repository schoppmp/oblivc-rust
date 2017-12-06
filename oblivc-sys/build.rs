extern crate walkdir;

use std::process::Command;
use std::path::Path;
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
}
