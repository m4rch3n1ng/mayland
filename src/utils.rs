use smithay::{
	reexports::rustix::time::{clock_gettime, ClockId},
	utils::{Point, Rectangle},
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

pub fn get_monotonic_time() -> Duration {
	let timespec = clock_gettime(ClockId::Monotonic);
	Duration::new(timespec.tv_sec as u64, timespec.tv_nsec as u32)
}
