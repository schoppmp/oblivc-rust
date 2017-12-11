extern crate libobliv_sys;
#[macro_use]
extern crate lazy_static;
extern crate cc;
extern crate bindgen;

use std::env;
use std::path::PathBuf;
use std::os::raw::{c_int, c_void};
use std::mem;
use std::ffi::{NulError, CString};
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
pub enum ConnectionError<'a> {
    Nul(NulError),
    Other(&'a str),
}
impl<'a> std::error::Error for ConnectionError<'a> {
    fn description(&self) -> &str {
        match self {
            &ConnectionError::Nul(ref e) => e.description(),
            &ConnectionError::Other(ref s) => s,
            // &ConnectionError::Acc => "Accept call failed",
        }
    }
}
impl<'a> fmt::Display for ConnectionError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}
impl<'a> From<NulError> for ConnectionError<'a> {
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
        ProtocolDesc{
            c: unsafe { mem::zeroed() },
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
    pub fn accept<P: Into<Vec<u8>>>(&mut self, port: P) -> Result<&mut Self, ConnectionError> {
        let port = CString::new(port)?;
        match unsafe {
             libobliv_sys::protocolAcceptTcp2P(&mut self.c, port.as_ptr())
        } {
           0 => Ok(self),
           _ => Err(ConnectionError::Other("Accept call failed")),
       }
    }

    // Tries to connect to `host:port` for `num_tries` times, waiting `sleep_time` between
    // attempts
    pub fn connect_loop<H: Into<Vec<u8>>, P: Into<Vec<u8>>>(&mut self, host: H, port: P,
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
                match num_tries {
                    Some(n) => if i < n-1 {
                        thread::sleep(sleep_time);
                    } else {
                        break;
                    },
                    None => ()
                };
            }
        }
        Err(ConnectionError::Other("Connection attempt failed"))
    }

    // Tries to connect to `host:port` in an infinite loop
    pub fn connect<H: Into<Vec<u8>>, P: Into<Vec<u8>>>(&mut self, host: H, port: P)
            -> Result<&mut Self, ConnectionError> {
        self.connect_loop(host, port, None, None)
    }

    // Tries to connect to `host:port` once
    pub fn connect_once<H: Into<Vec<u8>>, P: Into<Vec<u8>>>(&mut self, host: H, port: P)
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
            libobliv_sys::execYaoProtocol(&mut self.c, Some(f), arg as *mut _ as *mut c_void);
        }
    }
}
// Returns a new ProtocolDesc
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
    pub split: Option<unsafe extern "C" fn (t: * mut ProtocolTransport)
        -> * mut ProtocolTransport>,
    pub send: Option<unsafe extern "C" fn (t: * mut ProtocolTransport, party: c_int,
        data: *const c_void, len: usize ) -> c_int>,
    pub recv : Option<unsafe extern "C" fn (t: * mut ProtocolTransport, party: c_int,
        buf: *mut c_void, len: usize ) -> c_int>,
    pub flush: Option<unsafe extern "C" fn (t: * mut ProtocolTransport) -> c_int>,
    pub cleanup: Option<unsafe extern "C" fn (t: * mut ProtocolTransport)> ,
    pub stream: &'a mut S,
}
impl<'a, S: 'a + Read + Write> StreamProtocolTransport<'a, S> {
    // unsafe extern "C" fn split(t: * mut ProtocolTransport) -> * mut ProtocolTransport {
    //     t // TODO
    // }
    unsafe extern "C" fn send(t: * mut ProtocolTransport, _party: c_int,
            data: *const c_void, len: usize) -> c_int {
        let stream = &mut ((*(t as *mut StreamProtocolTransport<'a, S>)).stream);
        match stream.write_all(slice::from_raw_parts(data as *const u8, len)) {
            Ok(()) => len as c_int,
            Err(_) => -1,
        }
    }
    unsafe extern "C" fn recv(t: * mut ProtocolTransport, _party: c_int,
            data: *mut c_void, len: usize) -> c_int {
        let stream = &mut ((*(t as *mut StreamProtocolTransport<'a, S>)).stream);
        match stream.read_exact(slice::from_raw_parts_mut(data as *mut u8, len)) {
            Ok(()) => len as c_int,
            Err(_) => -1,
        }
    }
    unsafe extern "C" fn flush(t: * mut ProtocolTransport) -> c_int {
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

impl ProtocolDesc {
    // Uses `stream` for communication
    pub fn use_stream<'a, S: 'a + Read + Write>(&mut self, stream: &mut S) -> &mut Self {
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
