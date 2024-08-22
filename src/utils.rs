use smithay::{
	output::Output,
	utils::{Coordinate, Logical, Point, Rectangle, Size},
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
