extern crate oblivc;
extern crate test_oblivc;

use test_oblivc::{millionaire,millionaire_args};
use std::thread;
use std::net::{TcpListener,TcpStream};
use std::io::{Read,Write};

fn run_server() {
    let mut args = millionaire_args {
        input: 10,
        output: 0,
    };
    // Listen for connections and accept the first one
    let listener = TcpListener::bind("0.0.0.0:56735").unwrap();
    let (mut stream, _) = listener.accept().unwrap();
    // Use this connection for our ProtocolDesc
    let pd = oblivc::protocol_desc()
        .use_stream(&mut stream)
        .party(1);
    unsafe { pd.exec_yao_protocol(millionaire, &mut args); }

    // Read non-oblivc data sent by the client
    stream.read_exact(&mut [0; 4]).unwrap();

    // run again with roles reversed
    let pd = oblivc::protocol_desc()
        .use_stream(&mut stream)
        .party(2);
    unsafe { pd.exec_yao_protocol(millionaire, &mut args); }
}

fn run_client() {
    let mut args = millionaire_args {
        input: 20,
        output: 0,
    };
    // try connecting until successful
    let mut stream = (0..).filter_map(|_| {
        TcpStream::connect("localhost:56735").ok().or_else(|| {
            thread::sleep(std::time::Duration::from_millis(100));
            None
        })
    }).next().unwrap();
    // use the connection once established
    let pd = oblivc::protocol_desc()
        .party(2)
        .use_stream(&mut stream);
    unsafe { pd.exec_yao_protocol(millionaire, &mut args); }
    assert!(args.output == -1);

    // we can use the same stream outside obliv-c!
    stream.write_all(b"blah").unwrap();

    // use it for obliv-c again
    let pd = oblivc::protocol_desc()
        .party(1)
        .use_stream(&mut stream);
    unsafe { pd.exec_yao_protocol(millionaire, &mut args); }
    assert!(args.output == 1);
}

#[test]
/// Runs a two-party protocol using [`TcpStream`][1]s
///
/// [1]: https://doc.rust-lang.org/std/net/struct.TcpStream.html
fn test_tcp_stream() {
    let server = thread::spawn(run_server);
    run_client();
    server.join().unwrap();
}
