use smithay::reexports::calloop::{
	self, EventSource, Interest, Mode, Poll, PostAction, Readiness, Token, TokenFactory,
};
use std::{
	io::{ErrorKind, Read},
	os::{
		fd::{AsFd, BorrowedFd},
		unix::net::UnixListener,
	},
};

// todo rename
pub struct Action {}

pub struct MaySocket {
	socket: UnixListener,
	token: Option<Token>,
}

const SOCKET_PATH: &str = "/tmp/mayland.sock";

impl MaySocket {
	pub fn init() -> MaySocket {
		let listener = UnixListener::bind(SOCKET_PATH).unwrap();
		listener.set_nonblocking(true).unwrap();

		MaySocket {
			socket: listener,
			token: None,
		}
	}
}

impl AsFd for MaySocket {
	fn as_fd(&self) -> BorrowedFd<'_> {
		self.socket.as_fd()
	}
}

impl EventSource for MaySocket {
	type Event = Action;
	type Metadata = ();
	type Ret = ();
	type Error = std::io::Error;

	fn process_events<F>(
		&mut self,
		readiness: Readiness,
		token: Token,
		callback: F,
	) -> Result<PostAction, Self::Error>
	where
		F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
	{
		if Some(token) != self.token {
			return Ok(PostAction::Continue);
		}

		let (mut stream, _addr) = match self.socket.accept() {
			Ok(thing) => thing,
			Err(io_err) => {
				let kind = io_err.kind();
				if kind == ErrorKind::WouldBlock {
					return Ok(PostAction::Continue);
				} else {
					return Err(io_err);
				}
			}
		};

		let mut buf = String::new();
		stream.read_to_string(&mut buf).unwrap();

		println!("buf {:?}", buf);

		Ok(PostAction::Continue)
	}

	fn register(&mut self, poll: &mut Poll, factory: &mut TokenFactory) -> calloop::Result<()> {
		let token = factory.token();
		self.token = Some(token);

		// SAFETY: the fd is owned by MaySocket and cannot be dropped without unregistering
		unsafe { poll.register(self.as_fd(), Interest::BOTH, Mode::Level, token) }
	}

	fn reregister(
		&mut self,
		poll: &mut Poll,
		factory: &mut TokenFactory,
	) -> smithay::reexports::calloop::Result<()> {
		let token = factory.token();
		self.token = Some(token);

		poll.reregister(self.as_fd(), Interest::BOTH, Mode::Level, token)
	}

	fn unregister(&mut self, poll: &mut Poll) -> calloop::Result<()> {
		self.token = None;
		poll.unregister(self.as_fd())
	}
}

impl Drop for MaySocket {
	fn drop(&mut self) {
		// todo
		let _ = std::fs::remove_file(SOCKET_PATH);
	}
}
