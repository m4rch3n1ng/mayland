use crate::state::Mayland;
use smithay::{
	output::Output,
	utils::{Coordinate, Logical, Point, Rectangle, Size},
};
use std::{
	os::unix::process::CommandExt,
	process::{Command, Stdio},
};

pub trait RectExt<N, Kind> {
	fn center(&self) -> Point<N, Kind>;
}

impl<Kind> RectExt<i32, Kind> for Rectangle<i32, Kind> {
	fn center(&self) -> Point<i32, Kind> {
		let mut location = self.loc;
		location.x += self.size.w / 2;
		location.y += self.size.h / 2;

		location
	}
}

pub trait SizeExt<N: Coordinate, Kind> {
	fn borderless(&self, border: N) -> Size<N, Kind>;
}

impl<N: Coordinate, Kind> SizeExt<N, Kind> for Size<N, Kind> {
	fn borderless(&self, border: N) -> Size<N, Kind> {
		let border = border + border;
		Size::from((self.w.saturating_sub(border), self.h.saturating_sub(border)))
	}
}

pub fn output_size(output: &Output) -> Size<i32, Logical> {
	let output_scale = output.current_scale().integer_scale();
	let output_mode = output.current_mode().unwrap();
	let output_transform = output.current_transform();

	output_transform
		.transform_size(output_mode.size)
		.to_logical(output_scale)
}

pub fn spawn(spawn: Vec<String>, mayland: &Mayland) {
	let [command, args @ ..] = &*spawn else {
		panic!("spawn commands cannot be empty");
	};

	let mut cmd = Command::new(command);
	cmd.args(args)
		.stdin(Stdio::null())
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.envs(&mayland.environment);

	// SAFETY: the pre_exec closure does not access
	// any memory of the parent process and is therefore safe to use
	unsafe {
		cmd.pre_exec(|| {
			// double fork
			match libc::fork() {
				// fork returned an error
				-1 => return Err(std::io::Error::last_os_error()),
				// fork is inside the child process
				0 => (),
				// fork is inside the intermediate parent process
				// so kill the intermediate parent
				_ => libc::_exit(0),
			};

			Ok(())
		})
	};

	std::thread::spawn(move || match cmd.spawn() {
		Ok(mut child) => {
			let _ = child.wait();
		}
		Err(err) => tracing::error!("error spawning child: {:?}", err),
	});
}
