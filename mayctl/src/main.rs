use mayland::comm::{Event, Info};
use std::{
	io::{Read, Write},
	net::Shutdown,
	os::unix::net::UnixStream,
};

const SOCKET_PATH: &str = "/tmp/mayland.sock";

fn main() {
	let event = Event::Info;

	let v = postcard::to_stdvec(&event).unwrap();

	let mut stream = UnixStream::connect(SOCKET_PATH).unwrap();
	stream.write_all(&v).unwrap();

	stream.shutdown(Shutdown::Write).unwrap();

	let mut buffer = Vec::new();
	stream.read_to_end(&mut buffer).unwrap();

	let info = postcard::from_bytes::<Info>(&buffer);
	println!("info {:?}", info);
}
