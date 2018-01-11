Rust wrapper for Obliv-C
========================
[![Crates.io](https://img.shields.io/crates/v/oblivc.svg)](https://crates.io/crates/oblivc)
[![Build Status](https://travis-ci.org/schoppmp/oblivc-rust.svg?branch=master)](https://travis-ci.org/schoppmp/oblivc-rust)

[Obliv-C](https://github.com/samee/obliv-c) is a language for expressing
Multi-Party Computation protocols as C-like programs.
This wrapper allows to develop Rust programs that call Obliv-C protocols.

If an Obliv-C installation is passed via the `OBLIVC_PATH` environment
variable at build time, that installation is used.
Otherwise, Obliv-C is built from source.

A small example using this library can be found
[here](https://github.com/schoppmp/oblivc-rust/tree/master/test-oblivc).
