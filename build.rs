use std::env;

fn main() {
    // re-export obliv-c root directory and include dirs
    for &(name, var) in &[("root", "DEP_OBLIV_ROOT"), ("include", "DEP_OBLIV_INCLUDE")] {
        let value = env::var(&var).unwrap();
        println!("cargo:rustc-env={}={}", &var, &value); // pass to rust compiler
        println!("cargo:{}={}", &name, &value); // pass to dependent packages
    }
}
