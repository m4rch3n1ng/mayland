use self::cli::Cli;
use clap::Parser;
use mayland_comm::{Request, Response, MAYLAND_SOCKET_VAR};
use std::{
	io::{BufRead, BufReader, Write},
	net::Shutdown,
	os::unix::net::UnixStream,
};

mod cli;

fn main() {
	let cli = Cli::parse();
	let socket_path = std::env::var(MAYLAND_SOCKET_VAR).expect("not running in a mayland instance");

	let request = Request::from(cli);
	let message = serde_json::to_string(&request).unwrap();

	let mut stream = UnixStream::connect(socket_path).unwrap();
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
