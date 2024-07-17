use super::window::MappedWindow;
use crate::state::State;
use smithay::{
	desktop::space::SpaceElement,
	utils::{Logical, Point, Serial, Size},
};

mod floating;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResizeData {
	pub corner: ResizeCorner,
	pub initial_window_location: Point<i32, Logical>,
	pub initial_window_size: Size<i32, Logical>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeState {
	Resizing(ResizeData),
	WatingForCommit(ResizeData),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

	pub fn delta(
		&self,
		initial_window_size: Size<i32, Logical>,
		window_size: Size<i32, Logical>,
	) -> Option<Point<i32, Logical>> {
		let delta = match self {
			ResizeCorner::TopLeft => Some((
				initial_window_size.w - window_size.w,
				initial_window_size.h - window_size.h,
			)),
			ResizeCorner::TopRight => Some((0, initial_window_size.h - window_size.h)),
			ResizeCorner::BottomLeft => Some((initial_window_size.w - window_size.w, 0)),
			ResizeCorner::BottomRight => None,
		};

		delta.map(Point::from)
	}
}

impl State {
	pub fn xdg_move(&mut self, window: MappedWindow, serial: Serial) {
		if self.mayland.workspaces.is_floating(&window) {
			self.xdg_floating_move(window, serial);
		} else {
			tracing::debug!("todo! tiling move");
		}
	}

	pub fn xdg_resize(&mut self, window: MappedWindow, serial: Serial) {
		if self.mayland.workspaces.is_floating(&window) {
			self.xdg_floating_resize(window, serial);
		} else {
			tracing::debug!("todo! tiling resize");
		}
	}

	pub fn handle_resize(&mut self, window: MappedWindow) {
		let mut resize_state = window.resize_state.lock().unwrap();
		if let Some(ResizeState::Resizing(data) | ResizeState::WatingForCommit(data)) = *resize_state {
			let ResizeData {
				corner,
				initial_window_location,
				initial_window_size,
			} = data;

			let window_size = window.geometry().size;

			let delta = corner.delta(initial_window_size, window_size);
			if let Some(delta) = delta {
				let location = initial_window_location + delta;
				self.mayland.workspaces.floating_move(window.clone(), location);
			}
		}

		if let Some(ResizeState::WatingForCommit(_)) = *resize_state {
			*resize_state = None;
		}
	}
}
