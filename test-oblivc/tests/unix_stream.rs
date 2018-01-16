#![cfg(unix)]

extern crate oblivc;
extern crate test_oblivc;

use test_oblivc::{millionaire,millionaire_args};
use std::thread;
use std::os::unix::net::UnixStream;

fn run_server(mut stream: UnixStream) {
    let mut args = millionaire_args {
        input: 10,
        output: 0,
    };
    let pd = oblivc::protocol_desc()
        .party(1)
        .use_stream(&mut stream);
    unsafe { pd.exec_yao_protocol(millionaire, &mut args); }
}

fn run_client(mut stream: UnixStream) {
    let mut args = millionaire_args {
        input: 20,
        output: 0,
    };
    let pd = oblivc::protocol_desc()
        .party(2)
        .use_stream(&mut stream);
    unsafe { pd.exec_yao_protocol(millionaire, &mut args); }
    // party 1 < party 2
    assert!(args.output == -1);
}

#[test]
/// Runs a two-party protocol using [`UnixStream`][1]s.
///
/// [1]: https://doc.rust-lang.org/std/os/unix/net/struct.UnixStream.html
fn test_unix_stream() {
    // create a pair of streams corresponding to an anonymous unix socket
    let (stream1, stream2) = UnixStream::pair().unwrap();
    // pass one to the server, use the other as client
    let server = thread::spawn(move || run_server(stream1));
    run_client(stream2);
    server.join().unwrap();
}
