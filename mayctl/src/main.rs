use crate::cli::Cli;
use clap::Parser;
use mayland::comm::Event;
use std::{io::Write, net::Shutdown, os::unix::net::UnixStream};

mod cli;
mod event;

const SOCKET_PATH: &str = "/tmp/mayland.sock";

fn main() {
	let cli = Cli::parse();
	let event = Event::from(cli);

	let mut stream = UnixStream::connect(SOCKET_PATH).unwrap();

	let wire = postcard::to_stdvec(&event).unwrap();
	stream.write_all(&wire).unwrap();
	stream.shutdown(Shutdown::Write).unwrap();

	event::process(event, &mut stream);
	stream.shutdown(Shutdown::Read).unwrap();
}
