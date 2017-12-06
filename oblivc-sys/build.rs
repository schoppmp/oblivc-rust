
use std::process::Command;
use std::path::Path;

fn main() {
    // update submodules
    if !Path::new("obliv-c/.git").exists() {
        Command::new("git").args(&["submodule", "update", "--init"])
                           .status().unwrap();
    }
}
