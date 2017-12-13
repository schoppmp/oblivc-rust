#![doc(html_root_url = "https://schoppmp.github.io/doc/oblivc-rust/")]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

extern crate libgpg_error_sys;
extern crate libgcrypt_sys;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod test {
    use super::*;
    use std::os::raw::{c_int,c_void};
    use std::ffi::CString;
    use std::thread;
    use std::time::Duration;

    #[repr(C)]
    struct TestMillionaireArgs {
        input: c_int,
        output: i8,
    }
    #[link(name="test_oblivc", kind="static")]
    extern "C" { fn millionaire(arg: *mut c_void); }

    #[test]
    fn test_millionaire() {
        // spawn two threads, one for the server and one for the client
        let handles : Vec<_> = (1..3).map(|party| {
            thread::spawn(move || {
                let mut args = TestMillionaireArgs{
                    input: (10000 + 100 * party) as c_int,
                    output: 0i8,
                };
                let host = CString::new("localhost").unwrap();
                let port = CString::new("45623").unwrap();
                unsafe {
                    // allocate an uninitialized ProtocolDesc
                    let mut pd: ProtocolDesc = std::mem::uninitialized();
                    // connect
                    setCurrentParty(&mut pd, party);
                    if party == 1 {
                        protocolAcceptTcp2P(&mut pd, port.as_ptr());
                    } else {
                        while protocolConnectTcp2P(&mut pd, host.as_ptr(), port.as_ptr()) != 0 {
                            thread::sleep(Duration::from_millis(100));
                        }
                    }
                    // run millionnaire's problem
                    execYaoProtocol(&mut pd,
                        Some(millionaire),
                        &mut args as *mut _ as *mut c_void
                    );
                    cleanupProtocol(&mut pd);
                }
                // party 1 input < party 2 input>
                assert!(args.output < 0);
            })
        }).collect();
        for handle in handles {
            handle.join().unwrap();
        }
    }
}
