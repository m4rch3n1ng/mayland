use self::{
	cli::Cli,
	term::{ensure_matches, unexpected, Term},
};
use clap::Parser;
use mayland_comm::{Request, Response, MAYLAND_SOCKET_VAR};
use serde::Serialize;
use std::{
	fmt::Display,
	io::{BufRead, BufReader, ErrorKind, Write},
	net::Shutdown,
	os::unix::net::UnixStream,
};

mod cli;
mod term;

fn main() -> Term {
	let cli = Cli::parse();
	let Ok(socket_path) = std::env::var(MAYLAND_SOCKET_VAR) else {
		return Term::MaylandNotRunning;
	};

	let request = Request::from(cli.cmd);
	let message = serde_json::to_vec(&request).unwrap();

	let mut stream = match UnixStream::connect(&socket_path) {
		Ok(stream) => stream,
		Err(err) if matches!(err.kind(), ErrorKind::NotFound) => return Term::NotFound(socket_path),
		Err(err) => return Term::IoError(err),
	};
	stream.write_all(&message).unwrap();
	stream.write_all(b"\n").unwrap();
	stream.shutdown(Shutdown::Write).unwrap();

	let mut read = BufReader::new(&mut stream);
	let mut buf = String::new();
	read.read_line(&mut buf).unwrap();

	let response = match serde_json::from_str::<Response>(&buf) {
		Ok(response) => response,
		Err(err) => return Term::InvalidResponse(err),
	};
	stream.shutdown(Shutdown::Read).unwrap();

	if let Response::Err(err) = response {
		return Term::MaylandError(err);
	}

	match request {
		Request::Dispatch(_) => {
			ensure_matches!(response, Response::Dispatch, "dispatch");
			println!("ok dispatch");
		}
		Request::Workspaces => {
			let Response::Workspaces(workspaces) = response else {
				unexpected!(response, "workspaces")
			};

			if cli.json {
				stringify(&workspaces);
			} else {
				prettify(&workspaces);
			}
		}
	}

	Term::Ok
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
