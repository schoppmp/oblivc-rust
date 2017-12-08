extern crate libobliv_sys;
#[macro_use]
extern crate lazy_static;
extern crate cc;
extern crate bindgen;

use std::env;
use std::path::PathBuf;
use std::os::raw::{c_int, c_void};
use std::mem;
use std::ffi::{NulError,CString};
use std::time::Duration;
use std::thread;
use std::fmt;
use std::error::Error;
use std::ops::Drop;

lazy_static! {
    // The root folder of the native Obliv-C installation
    static ref OBLIVC_ROOT : PathBuf = PathBuf::from(env!("DEP_OBLIV_ROOT"));
    // A list of paths needed for compiling Obliv-C files
    static ref OBLIVC_INCLUDE : Vec<PathBuf> =
        env::split_paths(env!("DEP_OBLIV_INCLUDE")).collect();
}

// Returns a new `cc::Build` that uses `oblivcc` as compiler and includes all necessary headers
pub fn compiler() -> cc::Build {
    let mut builder = cc::Build::new();
    OBLIVC_INCLUDE.iter().fold(
        builder.compiler(OBLIVC_ROOT.join("bin/oblivcc")),
        |builder, path| builder.include(path)
    );
    builder
}

// Returns a new `bindgen::Builder` that already includes all headers needed to generate
// bindings for Obliv-C sources
pub fn bindings() -> bindgen::Builder {
    bindgen::builder()
        .clang_args(OBLIVC_INCLUDE.iter().map(|p| format!("-I{}", p.display())))
}



// Error returned by `ProtocolDesc::connect`, `ProtocolDesc::connect_loop` and
// `ProtocolDesc::accept`
#[derive(Debug)]
pub enum ConnectionError {
    Nul(NulError),
    Time,
}
impl std::error::Error for ConnectionError {
    fn description(&self) -> &str {
        match self {
            &ConnectionError::Nul(ref e) => e.description(),
            &ConnectionError::Time => "Connection attempt timed out",
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

// Wraps the C ProtocolDesc struct
pub struct ProtocolDesc {
    c: libobliv_sys::ProtocolDesc,
}
type ProtocolFn = unsafe extern "C" fn ( arg1 : * mut :: std :: os :: raw :: c_void );
impl ProtocolDesc {
    // Returns a new ProtocolDesc
    pub fn new() -> Self {
        unsafe {
            let mut pd = ProtocolDesc{ c: mem::uninitialized() };
            pd.c.thisParty = 0;
            pd.c.trans = std::ptr::null_mut();
            pd
        }
    }

    // Sets the party id
    pub fn party(&mut self, party: c_int) -> &mut Self {
        unsafe {
            libobliv_sys::setCurrentParty(&mut self.c, party);
        }
        self
    }

    // Accepts an incoming connection using Obliv-C's networking stack
    pub fn accept<S: Into<Vec<u8>>>(&mut self, port: S) -> Result<&mut Self, ConnectionError> {
        let port = CString::new(port)?;
        unsafe {
            libobliv_sys::protocolAcceptTcp2P(&mut self.c, port.as_ptr());
        }
        Ok(self)
    }

    // Tries to connect to `host:port` for `num_tries` times, waiting `sleep_time` between
    // attempts
    pub fn connect_loop<S: Into<Vec<u8>>>(&mut self, host: S, port: S,
            sleep_time: Option<Duration>, num_tries: Option<usize>) ->
            Result<&mut Self, ConnectionError> {
        let host = CString::new(host)?;
        let port = CString::new(port)?;
        let sleep_time = sleep_time.unwrap_or(Duration::from_millis(100));
        unsafe {
            for i in 0.. {
                let status = libobliv_sys::protocolConnectTcp2P(
                    &mut self.c, host.as_ptr(), port.as_ptr()
                );
                if status == 0 {
                    return Ok(self);
                }
                match &num_tries {
                    &Some(n) => if i < n-1 {
                        thread::sleep(sleep_time);
                    } else {
                        break;
                    },
                    &None => ()
                };
            }
        }
        Ok(self)
    }

    // Tries to connect to `host:port` in an infinite loop
    pub fn connect<S: Into<Vec<u8>>>(&mut self, host: S, port: S)
            -> Result<&mut Self, ConnectionError> {
        self.connect_loop(host, port, None, None)
    }

    // Tries to connect to `host:port` once
    pub fn connect_once<S: Into<Vec<u8>>>(&mut self, host: S, port: S)
            -> Result<&mut Self, ConnectionError> {
        self.connect_loop(host, port, None, Some(1))
    }

    // Executes `f` with argument `arg` as a two-party Yao protocol
    // Panics if not connected or party not set
    pub fn exec_yao_protocol<Arg>(&mut self, f: ProtocolFn, arg: &mut Arg) {
        if self.c.thisParty == 0 {
            panic!("Party must be set before calling `exec_yao_protocol`");
        }
        if self.c.trans == std::ptr::null_mut() {
            panic!("Connection must be established before calling `exec_yao_protocol`");
        }
        unsafe {
            libobliv_sys::execYaoProtocol(
                &mut self.c, Some(f), arg as *mut _ as *mut c_void
            );
        }
    }
}
// Returns a new ProtocolDesc
pub fn protocol_desc() -> ProtocolDesc {
    ProtocolDesc::new()
}
impl Drop for ProtocolDesc {
    fn drop(&mut self) {
        unsafe {
            libobliv_sys::cleanupProtocol(&mut self.c);
        }
    }
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
