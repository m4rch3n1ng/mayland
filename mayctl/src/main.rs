use self::cli::Cli;
use clap::Parser;
use mayland_comm::{Request, Response};
use std::{
	io::{BufRead, BufReader, Write},
	net::Shutdown,
	os::unix::net::UnixStream,
};

mod cli;

const SOCKET_PATH: &str = "/tmp/mayland.sock";

fn main() {
	let cli = Cli::parse();

	let request = Request::from(cli);
	let message = serde_json::to_string(&request).unwrap();

	let mut stream = UnixStream::connect(SOCKET_PATH).unwrap();
	stream.write_all(message.as_bytes()).unwrap();
	stream.shutdown(Shutdown::Write).unwrap();

	let mut read = BufReader::new(&mut stream);
	let mut buf = String::new();
	read.read_line(&mut buf).unwrap();

	let reply = serde_json::from_str::<Response>(&buf).unwrap();
	stream.shutdown(Shutdown::Read).unwrap();

	match request {
		Request::Dispatch(_) => {
			assert!(matches!(reply, Response::Dispatch));
			println!("ok dispatch");
		}
	}
}
