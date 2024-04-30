use crate::{shell::element::WindowElement, state::State};
use smithay::{
	desktop::{space::SpaceElement, WindowSurface},
	input::{
		pointer::{
			AxisFrame, ButtonEvent, Focus, GestureHoldBeginEvent, GestureHoldEndEvent,
			GesturePinchBeginEvent, GesturePinchEndEvent, GesturePinchUpdateEvent,
			GestureSwipeBeginEvent, GestureSwipeEndEvent, GestureSwipeUpdateEvent, GrabStartData,
			MotionEvent, PointerGrab, PointerInnerHandle, RelativeMotionEvent,
		},
		SeatHandler,
	},
	reexports::{
		wayland_protocols::xdg::shell::server::xdg_toplevel::State as TopLevelState,
		wayland_server::Resource,
	},
	utils::{IsAlive, Logical, Point, Serial, Size},
	wayland::seat::WaylandFocus,
};

struct MoveGrab {
	start_data: GrabStartData<State>,
	window: WindowElement,
	window_offset: Point<i32, Logical>,
}

impl PointerGrab<State> for MoveGrab {
	fn motion(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		focus: Option<(<State as SeatHandler>::PointerFocus, Point<i32, Logical>)>,
		event: &MotionEvent,
	) {
		handle.motion(data, focus, event);

		let new_location = event.location.to_i32_round() + self.window_offset;
		data.mayland
			.space
			.map_element(self.window.clone(), new_location, true);
	}

	fn relative_motion(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		focus: Option<(<State as SeatHandler>::PointerFocus, Point<i32, Logical>)>,
		event: &RelativeMotionEvent,
	) {
		handle.relative_motion(data, focus, event);
	}

	fn button(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		event: &ButtonEvent,
	) {
		handle.button(data, event);
		if !handle.current_pressed().contains(&272) {
			handle.unset_grab(self, data, event.serial, event.time, true);
		}
	}

	fn axis(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		details: AxisFrame,
	) {
		handle.axis(data, details);
	}

	fn frame(&mut self, data: &mut State, handle: &mut PointerInnerHandle<'_, State>) {
		handle.frame(data);
	}

	fn gesture_swipe_begin(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		event: &GestureSwipeBeginEvent,
	) {
		handle.gesture_swipe_begin(data, event);
	}

	fn gesture_swipe_update(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		event: &GestureSwipeUpdateEvent,
	) {
		handle.gesture_swipe_update(data, event);
	}

	fn gesture_swipe_end(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		event: &GestureSwipeEndEvent,
	) {
		handle.gesture_swipe_end(data, event);
	}

	fn gesture_pinch_begin(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		event: &GesturePinchBeginEvent,
	) {
		handle.gesture_pinch_begin(data, event);
	}

	fn gesture_pinch_update(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		event: &GesturePinchUpdateEvent,
	) {
		handle.gesture_pinch_update(data, event);
	}

	fn gesture_pinch_end(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		event: &GesturePinchEndEvent,
	) {
		handle.gesture_pinch_end(data, event);
	}

	fn gesture_hold_begin(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		event: &GestureHoldBeginEvent,
	) {
		handle.gesture_hold_begin(data, event);
	}

	fn gesture_hold_end(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		event: &GestureHoldEndEvent,
	) {
		handle.gesture_hold_end(data, event);
	}

	fn start_data(&self) -> &GrabStartData<State> {
		&self.start_data
	}

	fn unset(&mut self, _data: &mut State) {}
}

#[derive(Debug)]
enum ResizeCorner {
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

struct ResizeGrab {
	start_data: GrabStartData<State>,
	corner: ResizeCorner,
	window: WindowElement,
	initial_window_location: Point<i32, Logical>,
	initial_window_size: Size<i32, Logical>,
	new_window_size: Size<i32, Logical>,
}

impl PointerGrab<State> for ResizeGrab {
	fn motion(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		focus: Option<(<State as SeatHandler>::PointerFocus, Point<i32, Logical>)>,
		event: &MotionEvent,
	) {
		handle.motion(data, focus, event);

		let (dx, dy) = <(f64, f64)>::from(event.location - self.start_data.location);
		let (dx, dy) = match self.corner {
			ResizeCorner::TopLeft => (-dx, -dy),
			ResizeCorner::TopRight => (dx, -dy),
			ResizeCorner::BottomLeft => (-dx, dy),
			ResizeCorner::BottomRight => (dx, dy),
		};

		let new_window_width = (self.initial_window_size.w as f64 + dx) as i32;
		let new_window_height = (self.initial_window_size.h as f64 + dy) as i32;

		self.new_window_size = Size::from((new_window_width, new_window_height));
		match self.window.0.underlying_surface() {
			WindowSurface::Wayland(xdg) => {
				xdg.with_pending_state(|state| {
					state.states.set(TopLevelState::Resizing);
					state.size = Some(self.new_window_size);
				});
				xdg.send_pending_configure();

				// todo this doesn't work properly

				let geometry = self.window.geometry();
				let (dx, dy) = match self.corner {
					ResizeCorner::TopLeft => (
						Some(self.initial_window_size.w - geometry.size.w),
						Some(self.initial_window_size.h - geometry.size.h),
					),
					ResizeCorner::TopRight => {
						(None, Some(self.initial_window_size.h - geometry.size.h))
					}
					ResizeCorner::BottomLeft => {
						(Some(self.initial_window_size.w - geometry.size.w), None)
					}
					ResizeCorner::BottomRight => (None, None),
				};

				let mut location = data.mayland.space.element_location(&self.window).unwrap();
				if let Some(dx) = dx {
					location.x = self.initial_window_location.x + dx;
				}
				if let Some(dy) = dy {
					location.y = self.initial_window_location.y + dy;
				}

				data.mayland
					.space
					.map_element(self.window.clone(), location, true);
			}
		}
	}

	fn relative_motion(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		focus: Option<(<State as SeatHandler>::PointerFocus, Point<i32, Logical>)>,
		event: &RelativeMotionEvent,
	) {
		handle.relative_motion(data, focus, event);
	}

	fn button(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		event: &ButtonEvent,
	) {
		handle.button(data, event);

		if !handle.current_pressed().contains(&273) {
			handle.unset_grab(self, data, event.serial, event.time, true);

			if !self.window.alive() {
				return;
			}

			match self.window.0.underlying_surface() {
				WindowSurface::Wayland(xdg) => {
					xdg.with_pending_state(|state| {
						state.states.unset(TopLevelState::Resizing);
						state.size = Some(self.new_window_size);
					});
					xdg.send_pending_configure();

					let geometry = self.window.geometry();
					let (dx, dy) = match self.corner {
						ResizeCorner::TopLeft => (
							Some(self.initial_window_size.w - geometry.size.w),
							Some(self.initial_window_size.h - geometry.size.h),
						),
						ResizeCorner::TopRight => {
							(None, Some(self.initial_window_size.h - geometry.size.h))
						}
						ResizeCorner::BottomLeft => {
							(Some(self.initial_window_size.w - geometry.size.w), None)
						}
						ResizeCorner::BottomRight => (None, None),
					};

					let mut location = data.mayland.space.element_location(&self.window).unwrap();
					if let Some(dx) = dx {
						location.x = self.initial_window_location.x + dx;
					}
					if let Some(dy) = dy {
						location.y = self.initial_window_location.y + dy;
					}

					data.mayland
						.space
						.map_element(self.window.clone(), location, true);
				}
			}
		}
	}

	fn axis(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		details: AxisFrame,
	) {
		handle.axis(data, details);
	}

	fn frame(&mut self, data: &mut State, handle: &mut PointerInnerHandle<'_, State>) {
		handle.frame(data);
	}

	fn gesture_swipe_begin(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		event: &GestureSwipeBeginEvent,
	) {
		handle.gesture_swipe_begin(data, event);
	}

	fn gesture_swipe_update(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		event: &GestureSwipeUpdateEvent,
	) {
		handle.gesture_swipe_update(data, event);
	}

	fn gesture_swipe_end(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		event: &GestureSwipeEndEvent,
	) {
		handle.gesture_swipe_end(data, event);
	}

	fn gesture_pinch_begin(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		event: &GesturePinchBeginEvent,
	) {
		handle.gesture_pinch_begin(data, event);
	}

	fn gesture_pinch_update(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		event: &GesturePinchUpdateEvent,
	) {
		handle.gesture_pinch_update(data, event);
	}

	fn gesture_pinch_end(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		event: &GesturePinchEndEvent,
	) {
		handle.gesture_pinch_end(data, event);
	}

	fn gesture_hold_begin(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		event: &GestureHoldBeginEvent,
	) {
		handle.gesture_hold_begin(data, event);
	}

	fn gesture_hold_end(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		event: &GestureHoldEndEvent,
	) {
		handle.gesture_hold_end(data, event);
	}

	fn start_data(&self) -> &GrabStartData<State> {
		&self.start_data
	}

	fn unset(&mut self, _data: &mut State) {}
}

impl State {
	pub fn xdg_floating_move(&mut self, window: WindowElement, serial: Serial) {
		let pointer = self.mayland.pointer.clone();

		if !pointer.has_grab(serial) {
			return;
		}

		let start_data = pointer.grab_start_data().unwrap();

		if start_data
			.focus
			.as_ref()
			.zip(window.wl_surface())
			.map_or(true, |(focus, wl)| !focus.0.same_client_as(&wl.id()))
		{
			return;
		}

		let pointer_location = pointer.current_location().to_i32_round();
		let window_geometry = self.mayland.space.element_geometry(&window).unwrap();
		let window_offset = window_geometry.loc - pointer_location;

		let pointer_location = start_data.location.to_i32_round();
		self.mayland
			.space
			.map_element(window.clone(), pointer_location + window_offset, true);

		let grab = MoveGrab {
			start_data,
			window,
			window_offset,
		};
		pointer.set_grab(self, grab, serial, Focus::Clear);
	}

	pub fn xdg_floating_resize(&mut self, window: WindowElement, serial: Serial) {
		let pointer = self.mayland.pointer.clone();

		if !pointer.has_grab(serial) {
			return;
		}

		let start_data = pointer.grab_start_data().unwrap();

		if start_data
			.focus
			.as_ref()
			.zip(window.wl_surface())
			.map_or(true, |(focus, wl)| !focus.0.same_client_as(&wl.id()))
		{
			return;
		}

		let window_geometry = self.mayland.space.element_geometry(&window).unwrap();
		let pointer_location = pointer.current_location().to_i32_round();

		let relative_position = pointer_location - window_geometry.loc;
		let window_size = window_geometry.size;

		let half_height = window_size.h / 2;
		let is_top = relative_position.y <= half_height;

		let half_width = window_size.w / 2;
		let is_left = relative_position.x <= half_width;

		let corner = ResizeCorner::new(is_top, is_left);

		let grab = ResizeGrab {
			start_data,
			corner,
			window,
			initial_window_location: window_geometry.loc,
			initial_window_size: window_size,
			new_window_size: window_size,
		};
		pointer.set_grab(self, grab, serial, Focus::Clear);
	}
}