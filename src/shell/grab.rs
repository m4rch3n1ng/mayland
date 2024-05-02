use super::element::MappedWindowElement;
use crate::state::State;
use smithay::utils::{Logical, Point, Serial, Size};

mod floating;

pub struct ResizeData {
	pub corner: ResizeCorner,
	pub initial_window_location: Point<i32, Logical>,
	pub initial_window_size: Size<i32, Logical>,
}

pub enum ResizeState {
	Resizing(ResizeData),
	WatingForCommit(ResizeData),
}

#[derive(Debug)]
pub enum ResizeCorner {
	TopLeft,
	TopRight,
	BottomLeft,
	BottomRight,
}

impl ResizeCorner {
	fn new(is_top: bool, is_left: bool) -> Self {
		match (is_top, is_left) {
			(true, true) => ResizeCorner::TopLeft,
			(true, false) => ResizeCorner::TopRight,
			(false, true) => ResizeCorner::BottomLeft,
			(false, false) => ResizeCorner::BottomRight,
		}
	}
}

impl State {
	pub fn xdg_move(&mut self, window: MappedWindowElement, serial: Serial) {
		self.xdg_floating_move(window, serial);
	}

	pub fn xdg_resize(&mut self, window: MappedWindowElement, serial: Serial) {
		self.xdg_floating_resize(window, serial);
	}
}
