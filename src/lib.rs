//! # Obliv-C bindings for Rust
//! [Obliv-C](https://github.com/samee/obliv-c) is a language for expressing Multi-Party
//! Computation protocols as C-like programs.
//! This wrapper allows to develop Rust programs that call Obliv-C protocols.
//!
//! If an Obliv-C installation is passed via the `OBLIVC_PATH` environment
//! variable at build time, that installation is used.
//! Otherwise, Obliv-C is built from source.
//!
//! A small example using this library can be found
//! [here](https://schoppmp.github.io/doc/oblivc-rust/test_oblivc/index.html).

// TODO: extend documentation

#![doc(html_root_url = "https://schoppmp.github.io/doc/oblivc-rust/")]

extern crate bindgen;
extern crate cc;
#[macro_use]
extern crate lazy_static;
extern crate libobliv_sys;

use std::env;
use std::path::PathBuf;
use std::os::raw::{c_int, c_void};
use std::mem;
use std::ffi::{CString, NulError};
use std::time::Duration;
use std::thread;
use std::fmt;
use std::error::Error;
use std::ops::Drop;
use std::io::{Read, Write};
use std::slice;
use libobliv_sys::ProtocolTransport;

lazy_static! {
    // The root folder of the native Obliv-C installation
    static ref OBLIVC_ROOT : PathBuf = PathBuf::from(env!("DEP_OBLIV_ROOT"));
    // A list of paths needed for compiling Obliv-C files
    static ref OBLIVC_INCLUDE : Vec<PathBuf> =
        env::split_paths(env!("DEP_OBLIV_INCLUDE")).collect();
}

/// Returns a new [`cc::Build`](../cc/struct.Build.html) that uses `oblivcc` as compiler and
/// includes all headers needed to compile Obliv-C sources.
pub fn compiler() -> cc::Build {
    let mut builder = cc::Build::new();
    OBLIVC_INCLUDE.iter().fold(
        builder.compiler(OBLIVC_ROOT.join("bin/oblivcc")),
        |builder, path| builder.include(path),
    );
    builder
}

/// Returns a new [`bindgen::Builder`](../bindgen/struct.Builder.html) that includes all headers
/// needed to generate Rust FFI bindings for Obliv-C sources.
pub fn bindings() -> bindgen::Builder {
    bindgen::builder().clang_args(OBLIVC_INCLUDE.iter().map(|p| format!("-I{}", p.display())))
}

/// Error returned by [`ProtocolDesc`](struct.ProtocolDesc.html) native Obliv-C connection methods.
#[derive(Debug)]
pub enum ConnectionError {
    /// Indicates that either host or port contained null bytes
    Nul(NulError),
    /// Used to indicate other errors, for example timeouts
    Other(&'static str),
}
impl std::error::Error for ConnectionError {
    fn description(&self) -> &str {
        match self {
            &ConnectionError::Nul(ref e) => e.description(),
            &ConnectionError::Other(ref s) => s,
            // &ConnectionError::Acc => "Accept call failed",
        }
    }
}
impl fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}
impl From<NulError> for ConnectionError {
    fn from(e: NulError) -> Self {
        ConnectionError::Nul(e)
    }
}

/// Describes a protocol that can be executed via Obliv-C.
/// Currently, only two-party Yao protocols are supported.
///
/// Apart from native TCP sockets created via
/// [`accept`](#method.accept) or
/// [`connect`](#method.connect), applications can use
/// anything that implements
/// [`Read`](https://doc.rust-lang.org/nightly/std/io/trait.Read.html) and
/// [`Write`](https://doc.rust-lang.org/nightly/std/io/trait.Write.html) for communication by
/// calling [`use_stream`](#method.use_stream).
pub struct ProtocolDesc {
    c: libobliv_sys::ProtocolDesc,
}
pub type ProtocolFn = unsafe extern "C" fn(arg: *mut c_void);
impl ProtocolDesc {
    /// Returns a new [`ProtocolDesc`](#struct.ProtocolDesc)
    pub fn new() -> Self {
        ProtocolDesc {
            c: unsafe { mem::zeroed() },
        }
    }

    /// Sets the party id of this [`ProtocolDesc`](#struct.ProtocolDesc).
    /// # Panics
    /// if `party` is not 1 or 2
    /// # Examples
    /// ```should_panic
    /// let mut pd = oblivc::protocol_desc().party(0); // panics
    /// ```
    pub fn party(mut self, party: c_int) -> Self {
        if party != 1 && party != 2 {
            panic!("Party must be either 1 or 2");
        }
        unsafe {
            libobliv_sys::setCurrentParty(&mut self.c, party);
        }
        self
    }

    /// Accepts an incoming connection on `port` using Obliv-C's networking stack.
    /// # Error
    /// * If `port` contains a null byte, a
    /// [`NulError`](https://doc.rust-lang.org/std/ffi/struct.NulError.html) is
    /// returned.
    /// * If the accept call fails, a
    /// [`ConnectionError::Other`](enum.ConnectionError.html) is returned.
    pub fn accept<P: Into<Vec<u8>>>(mut self, port: P) -> Result<Self, ConnectionError> {
        let port = CString::new(port)?;
        match unsafe { libobliv_sys::protocolAcceptTcp2P(&mut self.c, port.as_ptr()) } {
            0 => Ok(self),
            _ => Err(ConnectionError::Other("Accept call failed")),
        }
    }

    /// Tries to connect to `host:port` for `num_tries` times, waiting `sleep_time` between
    /// attempts. If `num_tries` is `None`, this function tries forever.
    /// # Errors
    /// * If either `host` or `port` contain a null byte, a
    /// [`NulError`](https://doc.rust-lang.org/std/ffi/struct.NulError.html) is
    /// returned.
    /// * If no connection could be established after trying `num_tries` times, a
    /// [`ConnectionError::Other`](enum.ConnectionError.html) is returned.
    pub fn connect_loop<H: Into<Vec<u8>>, P: Into<Vec<u8>>>(
        mut self,
        host: H,
        port: P,
        sleep_time: Duration,
        num_tries: Option<usize>,
    ) -> Result<Self, ConnectionError> {
        let host = CString::new(host)?;
        let port = CString::new(port)?;
        unsafe {
            for i in 0.. {
                let status =
                    libobliv_sys::protocolConnectTcp2P(&mut self.c, host.as_ptr(), port.as_ptr());
                if status == 0 {
                    return Ok(self);
                }
                match num_tries {
                    Some(n) => if i < n - 1 {
                        thread::sleep(sleep_time);
                    } else {
                        break;
                    },
                    None => (),
                };
            }
        }
        Err(ConnectionError::Other("Connection attempt failed"))
    }

    /// Tries to connect to `host:port` in an infinite loop, waiting 100ms between attempts.
    /// # Errors
    /// See [`connect_loop`][con]
    ///
    /// [con]: #method.connect
    pub fn connect<H: Into<Vec<u8>>, P: Into<Vec<u8>>>(
        self,
        host: H,
        port: P,
    ) -> Result<Self, ConnectionError> {
        self.connect_loop(host, port, Duration::from_millis(100), None)
    }

    /// Tries to connect to `host:port` once.
    /// # Errors
    /// See [`connect_loop`][con]
    ///
    /// [con]: #method.connect
    pub fn connect_once<H: Into<Vec<u8>>, P: Into<Vec<u8>>>(
        self,
        host: H,
        port: P,
    ) -> Result<Self, ConnectionError> {
        self.connect_loop(host, port, Duration::new(0, 0), Some(1))
    }

    /// Uses `stream` for communication.
    pub fn use_stream<'a, S: 'a + Read + Write>(mut self, stream: &mut S) -> Self {
        let boxed_trans = Box::new(StreamProtocolTransport {
            maxParties: 2,
            split: None,
            send: Some(StreamProtocolTransport::<'a, S>::send),
            recv: Some(StreamProtocolTransport::<'a, S>::recv),
            flush: Some(StreamProtocolTransport::<'a, S>::flush),
            cleanup: Some(StreamProtocolTransport::<'a, S>::cleanup),
            stream: stream,
        });
        self.c.trans = Box::into_raw(boxed_trans) as *mut ProtocolTransport;
        self
    }

    /// Executes `f` with argument `arg` as a two-party Yao protocol
    ///
    /// # Panics
    /// * if not connected either via [`connect`][connect], [`connect_loop`][connect_loop],
    /// [`connect_once`][connect_once], [`accept`][accept], or [`use_stream`][use_stream]
    /// * if [`party`][party] was not called
    ///
    /// # Safety
    /// This function is unsafe, since calling arbitrary Obliv-C functions with arbitrary arguments
    /// may lead to undefined behavior. It is the caller's responsibility to ensure that the
    /// arguments match the function being executed and that `f` is safe.
    ///
    /// [connect]: #method.connect
    /// [connect_loop]: #method.connect_loop
    /// [connect_once]: #method.connect_once
    /// [accept]: #method.accept
    /// [use_stream]: #method.use_stream
    /// [party]: #method.party
    pub unsafe fn exec_yao_protocol<Arg>(mut self, f: ProtocolFn, arg: &mut Arg) {
        if self.c.thisParty == 0 {
            panic!("Party must be set before calling `exec_yao_protocol`");
        }
        if self.c.trans == std::ptr::null_mut() {
            panic!("Connection must be established before calling `exec_yao_protocol`");
        }
        libobliv_sys::execYaoProtocol(&mut self.c, Some(f), arg as *mut _ as *mut c_void);
    }
}
/// Alias for `ProtocolDesc::new()`
pub fn protocol_desc() -> ProtocolDesc {
    ProtocolDesc::new()
}
impl Drop for ProtocolDesc {
    fn drop(&mut self) {
        if self.c.trans != std::ptr::null_mut() {
            unsafe {
                libobliv_sys::cleanupProtocol(&mut self.c);
            }
        }
    }
}

// Wraps a C ProtocolTransport struct that communicates via Read/Write traits
#[repr(C)]
#[allow(non_snake_case)]
struct StreamProtocolTransport<'a, S: 'a + Read + Write> {
    pub maxParties: c_int,
    pub split: Option<unsafe extern "C" fn(t: *mut ProtocolTransport) -> *mut ProtocolTransport>,
    pub send: Option<
        unsafe extern "C" fn(
            t: *mut ProtocolTransport,
            party: c_int,
            data: *const c_void,
            len: usize,
        ) -> c_int,
    >,
    pub recv: Option<
        unsafe extern "C" fn(t: *mut ProtocolTransport, party: c_int, buf: *mut c_void, len: usize)
            -> c_int,
    >,
    pub flush: Option<unsafe extern "C" fn(t: *mut ProtocolTransport) -> c_int>,
    pub cleanup: Option<unsafe extern "C" fn(t: *mut ProtocolTransport)>,
    pub stream: &'a mut S,
}
impl<'a, S: 'a + Read + Write> StreamProtocolTransport<'a, S> {
    unsafe extern "C" fn send(
        t: *mut ProtocolTransport,
        _party: c_int,
        data: *const c_void,
        len: usize,
    ) -> c_int {
        let stream = &mut ((*(t as *mut StreamProtocolTransport<'a, S>)).stream);
        match stream.write_all(slice::from_raw_parts(data as *const u8, len)) {
            Ok(()) => len as c_int,
            Err(_) => -1,
        }
    }
    unsafe extern "C" fn recv(
        t: *mut ProtocolTransport,
        _party: c_int,
        data: *mut c_void,
        len: usize,
    ) -> c_int {
        let stream = &mut ((*(t as *mut StreamProtocolTransport<'a, S>)).stream);
        match stream.read_exact(slice::from_raw_parts_mut(data as *mut u8, len)) {
            Ok(()) => len as c_int,
            Err(_) => -1,
        }
    }
    unsafe extern "C" fn flush(t: *mut ProtocolTransport) -> c_int {
        let stream = &mut ((*(t as *mut StreamProtocolTransport<'a, S>)).stream);
        match stream.flush() {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }
    unsafe extern "C" fn cleanup(t: *mut ProtocolTransport) {
        Box::from_raw(t as *mut StreamProtocolTransport<'a, S>);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_new_compiler() {
        let _ = compiler();
    }
    #[test]
    fn test_new_bindings() {
        let _ = bindings();
    }
}
