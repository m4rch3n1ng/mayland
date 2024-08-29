use std::{io::Write, os::unix::net::UnixStream};

const SOCKET_PATH: &str = "/tmp/mayland.sock";

fn main() {
	let mut unix_stream = UnixStream::connect(SOCKET_PATH).unwrap();
	unix_stream.write_all(b"test").unwrap();
}
