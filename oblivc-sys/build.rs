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
    let oblivc_path_env = env::var("OBLIVC_PATH").ok().and_then(|s| {
        if s == "" {None} else {Some(s)}
    });
    let oblivc_path = match oblivc_path_env {
        Some(s) => PathBuf::from(s),
        None => {
            // update submodule
            if !Path::new("obliv-c/.git").exists() {
                let status = t!(Command::new("git").args(&["submodule", "update", "--init"])
                                     .status());
                if !status.success() {
                    panic!("Updating submodules failed");
                };
            }
            PathBuf::from("obliv-c")
        },
    };
    // make oblivc_path absolute
    let oblivc_path = if oblivc_path.is_absolute() {
        oblivc_path
    } else {
        env::current_dir().unwrap().join(oblivc_path)
    };

    // build obliv-c
    if !oblivc_path.join("Makefile").exists() {
        let status = t!(Command::new("./configure").current_dir(&oblivc_path).status());
        if !status.success() {
            panic!("Configuring Obliv-C failed");
        }
    }
    let status = t!(Command::new("make").current_dir(&oblivc_path).status());
    if !status.success() {
        panic!("Building Obliv-C failed");
    }
    // link oblivcc and libobliv.a to OUT_DIR
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_libobliv_path = out_path.join("libobliv.a");
    let out_oblivcc_path = out_path.join("oblivcc");
    t!(std::fs::remove_file(&out_libobliv_path));
    t!(std::fs::remove_file(&out_oblivcc_path));
    t!(std::os::unix::fs::symlink(oblivc_path.join("_build/libobliv.a"), &out_libobliv_path));
    t!(std::os::unix::fs::symlink(oblivc_path.join("bin/oblivcc"), &out_oblivcc_path));

    // tell cargo to tell rustc to link libobliv.a
    println!("cargo:rustc-link-search=native={}", out_path.to_str().unwrap());
    println!("cargo:rustc-link-lib=static=obliv");

    // register to rebuild when something changes
    for file in WalkDir::new(oblivc_path.join("src"))
                        .into_iter()
                        .filter_map(|e| e.ok()) {
        if let Some(s) = file.path().to_str() {
            println!("cargo:rerun-if-changed={}", s);
        }
    }
    println!("cargo:rerun-if-changed={}", oblivc_path.join("_build/libobliv.a").to_str().unwrap());

    // add obliv-c source directory to CPATH for bindgen to find them
    let mut paths = Vec::<PathBuf>::new();
    if let Some(path) = env::var_os("CPATH") {
        paths = env::split_paths(&path).collect();
    }
    paths.push(oblivc_path.join("src/ext/oblivc"));
    let new_path = env::join_paths(paths).unwrap();
    env::set_var("CPATH", &new_path);

    // all functions in "obliv.h", but not in included headers
    let bind_functions = vec![
        "protocolUseStdio",
        "protocolUseTcp2P",
        "protocolUseTcp2PProfiled",
        "protocolUseTcp2PKeepAlive",
        "protocolAddSizeCheck",
        "protocolConnectTcp2P",
        "protocolAcceptTcp2P",
        "protocolConnectTcp2PProfiled",
        "protocolAcceptTcp2PProfiled",
        "cleanupProtocol",
        "setCurrentParty",
        "execDebugProtocol",
        "execNetworkStressProtocol",
        "execYaoProtocol",
        "execYaoProtocol_noHalf",
        "execDualexProtocol",
        "execNpProtocol",
        "execNpProtocol_Bcast1",
        "execNnobProtocol",
        "tcp2PBytesSent",
        "tcp2PFlushCount",
    ];
    // generate bindings
    let bindings = bind_functions.iter().fold(
            bindgen::Builder::default()
                .header(oblivc_path.join("src/ext/oblivc/obliv.h").to_str().unwrap()),
            |builder, func| builder.whitelisted_function(func)
        )
         .generate()
         .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
