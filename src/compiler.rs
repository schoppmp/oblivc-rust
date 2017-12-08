extern crate cc;

use std::env;
use std::path::PathBuf;

pub fn new_builder() -> cc::Build {
    let oblivc_root_path = PathBuf::from(env!("DEP_OBLIV_ROOT"));
    let oblivc_includes = env::split_paths(env!("DEP_OBLIV_INCLUDE"));
    let mut builder = cc::Build::new();
    oblivc_includes.fold(
        builder.compiler(oblivc_root_path.join("bin/oblivcc")),
        |builder, path| builder.include(path)
    );
    builder
}
