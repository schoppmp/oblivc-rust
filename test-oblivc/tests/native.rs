extern crate oblivc;
extern crate test_oblivc;

use test_oblivc::{millionaire,millionaire_args};
use std::thread;

fn run_server() {
    let mut args = millionaire_args {
        input: 10,
        output: 0,
    };
    let pd = oblivc::protocol_desc()
        .party(1)
        .accept("56734").unwrap();
    unsafe { pd.exec_yao_protocol(millionaire, &mut args); }
}

fn run_client() {
    let mut args = millionaire_args {
        input: 20,
        output: 0,
    };
    let pd = oblivc::protocol_desc()
        .party(2)
        .connect("localhost", "56734").unwrap();
    unsafe { pd.exec_yao_protocol(millionaire, &mut args); }
    // party 1 < party 2
    assert!(args.output == -1);
}

#[test]
/// Runs a two-party protocol using Obliv-C's native connections
fn test_native() {
    let server = thread::spawn(run_server);
    run_client();
    server.join().unwrap();
}
