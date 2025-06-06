use self::{
	cli::Cli,
	term::{Term, ensure_matches, unexpected},
};
use clap::{CommandFactory, Parser};
use mayland_comm::{MAYLAND_SOCKET_VAR, Request, Response};
use serde::Serialize;
use std::{
	fmt::Display,
	io::{BufRead, BufReader, ErrorKind, Write},
	net::Shutdown,
	os::unix::net::UnixStream,
	path::PathBuf,
};

mod cli;
mod term;

fn main() -> Term {
	clap_complete::CompleteEnv::with_factory(Cli::command).complete();
	let cli = Cli::parse();
	let Some(socket_path) = std::env::var_os(MAYLAND_SOCKET_VAR).map(PathBuf::from) else {
		return Term::MaylandNotRunning;
	};

	let request = Request::from(cli.cmd);
	let message = serde_json::to_vec(&request).unwrap();

	let mut stream = match UnixStream::connect(&socket_path) {
		Ok(stream) => stream,
		Err(err) if matches!(err.kind(), ErrorKind::NotFound) => return Term::NotFound(socket_path),
		Err(err) => return Term::IoError(socket_path, err),
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
		return Term::from(err);
	}

	match request {
		Request::Dispatch(_) => {
			ensure_matches!(response, Response::Dispatch, "dispatch");
			println!("ok dispatch");
		}
		Request::Reload => {
			ensure_matches!(response, Response::Reload, "reload");
			println!("ok reload");
		}
		Request::Devices => {
			let Response::Devices(devices) = response else {
				unexpected!(response, "devices")
			};

			if cli.json {
				stringify(&devices);
			} else {
				prettify(&devices);
			}
		}
		Request::Outputs => {
			let Response::Outputs(outputs) = response else {
				unexpected!(response, "outputs")
			};

			if cli.json {
				stringify(&outputs);
			} else {
				prettify(&outputs);
			}
		}
		Request::Windows => {
			let Response::Windows(windows) = response else {
				unexpected!(response, "windows")
			};

			if cli.json {
				stringify(&windows);
			} else {
				prettify(&windows);
			}
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

		print!("{t}");
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
