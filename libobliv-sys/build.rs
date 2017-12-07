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
        t!(env::current_dir()).join(oblivc_path)
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
    let oblivc_src_path = oblivc_path.join("src");
    let oblivc_bin_path = oblivc_path.join("bin");
    let oblivc_libobliv_path = oblivc_path.join("_build/libobliv.a");
    let out_path = PathBuf::from(t!(env::var("OUT_DIR")));
    let out_libobliv_path = out_path.join("libobliv.a");
    let out_bin_path = out_path.join("bin");
    // delete previous symlinks, ignoring errors'
    let _ = std::fs::remove_file(&out_libobliv_path);
    let _ = std::fs::remove_file(&out_bin_path);
    t!(std::os::unix::fs::symlink(&oblivc_libobliv_path, &out_libobliv_path));
    t!(std::os::unix::fs::symlink(&oblivc_bin_path, &out_bin_path));

    // tell cargo to tell rustc to link libobliv.a and gcrypt
    println!("cargo:rustc-link-search=native={}", out_path.display());
    println!("cargo:rustc-link-lib=static=obliv");

    // register to rebuild when something changes
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

    // add obliv-c source directory to CPATH for bindgen to find them
    let mut paths = Vec::<PathBuf>::new();
    if let Some(path) = env::var_os("CPATH") {
        paths = env::split_paths(&path).collect();
    }
    paths.push(oblivc_path.join("src/ext/oblivc"));
    let new_path = t!(env::join_paths(paths));
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
        .expect("Couldn't write bindings");

    // compile test_oblivc.oc
    // oblivcc doesn't properly set its exit status, so check if stderr is empty
    let output = t!(String::from_utf8(t!(Command::new(out_bin_path.join("oblivcc"))
                    .args(&["-c", "-o"]).arg(out_path.join("test_oblivc.oo"))
                    .arg("src/test_oblivc.oc")
                    .output()).stderr));
    if !output.is_empty() {
        panic!("Compiling test_oblivc.oc failed: {}", output);
    }
    // archive test_oblivc.oo
    let status = t!(Command::new("ar").args(&["crus", "libtest_oblivc.a", "test_oblivc.oo"])
                      .current_dir(&out_path)
                      .status());
    if !status.success() {
      panic!("Creating libtest_oblivc.a failed");
    }

}
