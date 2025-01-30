use crate::state::Mayland;
use smithay::{
	output::Output,
	utils::{Logical, Point, Rectangle, Size, Transform},
};
use std::{
	os::unix::process::CommandExt,
	process::{Command, Stdio},
};

pub trait RectExt<N, Kind> {
	fn borderless(&self, border: N) -> Rectangle<N, Kind>;

	fn center(&self) -> Point<N, Kind>;
}

impl<Kind> RectExt<i32, Kind> for Rectangle<i32, Kind> {
	fn borderless(&self, border: i32) -> Rectangle<i32, Kind> {
		let mut rect = *self;
		rect.loc += Point::from((border, border));
		rect.size = rect.size.borderless(border);

		rect
	}

	fn center(&self) -> Point<i32, Kind> {
		let mut location = self.loc;
		location += self.size.center();

		location
	}
}

pub trait SizeExt<N, Kind> {
	fn borderless(&self, border: N) -> Size<N, Kind>;

	fn center(&self) -> Point<N, Kind>;
}

impl<Kind> SizeExt<i32, Kind> for Size<i32, Kind> {
	fn borderless(&self, border: i32) -> Size<i32, Kind> {
		*self - Size::from((2 * border, 2 * border))
	}

	fn center(&self) -> Point<i32, Kind> {
		Point::from((self.w / 2, self.h / 2))
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

pub fn logical_output(output: &Output) -> mayland_comm::output::Logical {
	let size = output_size(output);
	let point = output.current_location();

	let transform = match output.current_transform() {
		Transform::Normal => mayland_comm::output::Transform::Normal,
		Transform::_90 => mayland_comm::output::Transform::_90,
		Transform::_180 => mayland_comm::output::Transform::_180,
		Transform::_270 => mayland_comm::output::Transform::_270,
		Transform::Flipped => mayland_comm::output::Transform::Flipped,
		Transform::Flipped90 => mayland_comm::output::Transform::Flipped90,
		Transform::Flipped180 => mayland_comm::output::Transform::Flipped180,
		Transform::Flipped270 => mayland_comm::output::Transform::Flipped270,
	};

	mayland_comm::output::Logical {
		x: point.x,
		y: point.y,
		w: size.w,
		h: size.h,
		transform,
	}
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
		.envs(&mayland.environment)
		.envs(&mayland.config.environment.envs);

	for key in &mayland.config.environment.remove {
		cmd.env_remove(key);
	}

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
