use self::cli::Cli;
use clap::Parser;
use mayland_comm::{Request, Response, MAYLAND_SOCKET_VAR};
use serde::Serialize;
use std::{
	fmt::Display,
	io::{BufRead, BufReader, Write},
	net::Shutdown,
	os::unix::net::UnixStream,
};

mod cli;

fn main() {
	let cli = Cli::parse();
	let socket_path = std::env::var(MAYLAND_SOCKET_VAR).expect("not running in a mayland instance");

	let request = Request::from(cli.cmd);
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
		Request::Workspaces => {
			let Response::Workspaces(workspaces) = reply else { panic!() };

			if cli.json {
				stringify(&workspaces);
			} else {
				prettify(&workspaces);
			}
		}
	}
}

fn prettify<T: Display>(t: &[T]) {
	for (i, t) in t.iter().enumerate() {
		if i != 0 {
			println!();
		}

		print!("{}", t);
	}
}

fn stringify<T: Serialize>(v: &T) {
	let mut stdout = std::io::stdout().lock();

	// i would like to use a tab, but this is called depression:    vvvv
	let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
	let mut json_serializer = serde_json::Serializer::with_formatter(&mut stdout, formatter);

	v.serialize(&mut json_serializer).unwrap();
	writeln!(stdout).unwrap();
}
