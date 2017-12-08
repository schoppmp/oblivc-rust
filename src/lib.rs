extern crate libobliv_sys;
#[macro_use]
extern crate lazy_static;
extern crate cc;
extern crate bindgen;

use std::env;
use std::path::PathBuf;

lazy_static! {
    pub static ref OBLIVC_ROOT : PathBuf = PathBuf::from(env!("DEP_OBLIV_ROOT"));
    pub static ref OBLIVC_INCLUDE : Vec<PathBuf> =
        env::split_paths(env!("DEP_OBLIV_INCLUDE")).collect();
}


pub fn compiler() -> cc::Build {
    let mut builder = cc::Build::new();
    OBLIVC_INCLUDE.iter().fold(
        builder.compiler(OBLIVC_ROOT.join("bin/oblivcc")),
        |builder, path| builder.include(path)
    );
    builder
}

pub fn bindings() -> bindgen::Builder {
    bindgen::builder()
        .clang_args(OBLIVC_INCLUDE.iter().map(|p| format!("-I{}", p.display())))
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_compiler() {
        let _ = compiler();
    }
    #[test]
    fn new_bindings() {
        let _ = bindings();
    }
}
