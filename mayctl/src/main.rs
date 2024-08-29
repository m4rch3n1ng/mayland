use mayland_comm::{Action, Request, Response};
use std::{
	io::{BufRead, BufReader, Write},
	net::Shutdown,
	os::unix::net::UnixStream,
};

const SOCKET_PATH: &str = "/tmp/mayland.sock";

fn main() {
	let dispatch = Request::Dispatch(Action::Spawn(vec!["kitty".to_owned()]));
	let dispatch = serde_json::to_string(&dispatch).unwrap();

	let mut stream = UnixStream::connect(SOCKET_PATH).unwrap();
	stream.write_all(dispatch.as_bytes()).unwrap();
	stream.shutdown(Shutdown::Write).unwrap();

	let mut read = BufReader::new(&mut stream);
	let mut buf = String::new();
	read.read_line(&mut buf).unwrap();

	let reply = serde_json::from_str::<Response>(&buf).unwrap();
	dbg!(reply);

	stream.shutdown(Shutdown::Read).unwrap();
}
