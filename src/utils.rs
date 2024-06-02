use smithay::{
	output::Output,
	reexports::rustix::time::{clock_gettime, ClockId},
	utils::{Logical, Point, Rectangle, Size},
};
use std::time::Duration;

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

pub fn output_size(output: &Output) -> Size<i32, Logical> {
	let output_scale = output.current_scale().integer_scale();
	let output_mode = output.current_mode().unwrap();
	let output_transform = output.current_transform();

	output_transform
		.transform_size(output_mode.size)
		.to_logical(output_scale)
}

pub fn get_monotonic_time() -> Duration {
	let timespec = clock_gettime(ClockId::Monotonic);
	Duration::new(timespec.tv_sec as u64, timespec.tv_nsec as u32)
}
