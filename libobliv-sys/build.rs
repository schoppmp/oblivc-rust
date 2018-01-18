extern crate walkdir;
extern crate bindgen;
extern crate cc;

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
        None => PathBuf::from("obliv-c"),
    };
    // make oblivc_path absolute
    let oblivc_path = if oblivc_path.is_absolute() {
        oblivc_path
    } else {
        t!(env::current_dir()).join(oblivc_path)
    };

    // update submodule, but ignore if that fails
    if !oblivc_path.join(".git").exists() {
        let _status = Command::new("git")
            .args(&["submodule", "update", "--init"])
            .status();
    }

    // build obliv-c
    if !oblivc_path.join("Makefile").exists() {
        let status = t!(Command::new("./configure").current_dir(&oblivc_path).status());
        if !status.success() {
            panic!("Configuring Obliv-C failed");
        }
    }
    let status = t!(Command::new("make")
        .current_dir(&oblivc_path)
        .arg("CFLAGS=-fPIC")
        .status());
    if !status.success() {
        panic!("Building Obliv-C failed");
    }

    let oblivc_src_path = oblivc_path.join("src");
    let oblivc_bin_path = oblivc_path.join("bin");
    let oblivc_libobliv_path = oblivc_path.join("_build/libobliv.a");
    let out_path = PathBuf::from(t!(env::var("OUT_DIR")));
    let out_libobliv_path = out_path.join("libobliv.a");
    let out_bin_path = out_path.join("bin");

    // delete previous symlinks, ignoring errors
    let _ = std::fs::remove_file(&out_libobliv_path);
    let _ = std::fs::remove_file(&out_bin_path);
    // link oblivcc and libobliv.a to OUT_DIR
    t!(std::os::unix::fs::symlink(&oblivc_libobliv_path, &out_libobliv_path));
    t!(std::os::unix::fs::symlink(&oblivc_bin_path, &out_bin_path));

    // tell cargo to tell rustc to link libobliv.a
    println!("cargo:root={}", oblivc_path.display());
    println!("cargo:rustc-link-search=native={}", out_path.display());
    println!("cargo:rustc-link-lib=static=obliv");

    // register to rebuild when Obliv-C sources change
    let register_dir_rebuild = |dir: &AsRef<Path>| {
        for file in WalkDir::new(dir)
                            .into_iter()
                            .filter_map(|e| e.ok()) {
            println!("cargo:rerun-if-changed={}", file.path().display());
        }
    };
    register_dir_rebuild(&oblivc_src_path);
    register_dir_rebuild(&oblivc_bin_path);
    register_dir_rebuild(&"src");
    println!("cargo:rerun-if-changed={}", oblivc_libobliv_path.display());
    // also rerun if OBLIVC_PATH changes
    println!("cargo:rerun-if-env-changed=OBLIVC_PATH");

    // collect include paths (including dependencies)
    let include_paths = vec![
        oblivc_path.join("src/ext/oblivc"),
        PathBuf::from(t!(env::var("DEP_GCRYPT_ROOT"))).join("include"),
        PathBuf::from(t!(env::var("DEP_GPG_ERROR_ROOT"))).join("include"),
    ];
    println!("cargo:include={}", t!(env::join_paths(&include_paths)).to_str().unwrap());

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
            |builder, func| builder.whitelist_function(func)
        )
        .clang_args(include_paths.iter().map(|p| format!("-I{}", p.display())))
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings");

    // Compile tests
    include_paths.iter().fold(
            cc::Build::new().compiler(out_bin_path.join("oblivcc")),
            |builder, path| builder.include(path)
        )
        .file("src/test_oblivc.oc")
        .compile("libtest_oblivc.a")
}
