use mayland::comm::{Event, Info};
use std::{io::Read, os::unix::net::UnixStream};

pub fn process(event: Event, stream: &mut UnixStream) {
	match event {
		Event::Dispatch(_) => {
			// todo result
		}
		Event::Info => {
			let mut buffer = Vec::new();
			stream.read_to_end(&mut buffer).unwrap();

			let info = postcard::from_bytes::<Info>(&buffer);
			println!("info {:?}", info);
		}
	}
}
