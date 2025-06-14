use super::{ResizeCorner, ResizeData, ResizeState};
use crate::{shell::window::MappedWindow, state::State};
use smithay::{
	desktop::WindowSurface,
	input::{
		SeatHandler,
		pointer::{
			AxisFrame, ButtonEvent, CursorIcon, Focus, GestureHoldBeginEvent, GestureHoldEndEvent,
			GesturePinchBeginEvent, GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent,
			GestureSwipeEndEvent, GestureSwipeUpdateEvent, GrabStartData, MotionEvent, PointerGrab,
			PointerInnerHandle, RelativeMotionEvent,
		},
	},
	reexports::{wayland_protocols::xdg::shell::server::xdg_toplevel, wayland_server::Resource},
	utils::{IsAlive, Logical, Point, Serial, Size},
	wayland::seat::WaylandFocus,
};

struct MoveGrab {
	start_data: GrabStartData<State>,
	window: MappedWindow,
	window_offset: Point<i32, Logical>,
}

impl PointerGrab<State> for MoveGrab {
	fn motion(
		&mut self,
		state: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		_focus: Option<(<State as SeatHandler>::PointerFocus, Point<f64, Logical>)>,
		event: &MotionEvent,
	) {
		// no client has pointer focus while grab is active
		handle.motion(state, None, event);

		let new_location = event.location.to_i32_round() + self.window_offset;

		let Some(workspace) = state.mayland.workspaces.workspace_mut() else {
			handle.unset_grab(self, state, event.serial, event.time, true);
			return;
		};

		workspace.floating_move(self.window.clone(), new_location);
	}

	fn relative_motion(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		_focus: Option<(<State as SeatHandler>::PointerFocus, Point<f64, Logical>)>,
		event: &RelativeMotionEvent,
	) {
		// no client has pointer focus while grab is active
		handle.relative_motion(data, None, event);
	}

	fn button(&mut self, state: &mut State, handle: &mut PointerInnerHandle<'_, State>, event: &ButtonEvent) {
		handle.button(state, event);
		if !handle.current_pressed().contains(&272) {
			handle.unset_grab(self, state, event.serial, event.time, true);
		}
	}

	fn axis(&mut self, data: &mut State, handle: &mut PointerInnerHandle<'_, State>, details: AxisFrame) {
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

	fn unset(&mut self, state: &mut State) {
		state.mayland.cursor.icon = None;
		state.mayland.queue_redraw_all();
	}
}

struct ResizeGrab {
	start_data: GrabStartData<State>,
	corner: ResizeCorner,
	window: MappedWindow,
	initial_window_size: Size<i32, Logical>,
	new_window_size: Size<i32, Logical>,
}

impl PointerGrab<State> for ResizeGrab {
	fn motion(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		_focus: Option<(<State as SeatHandler>::PointerFocus, Point<f64, Logical>)>,
		event: &MotionEvent,
	) {
		// no client has pointer focus while grab is active
		handle.motion(data, None, event);

		let (dx, dy) = <(f64, f64)>::from(event.location - self.start_data.location);
		let (dx, dy) = match self.corner {
			ResizeCorner::TopLeft => (-dx, -dy),
			ResizeCorner::TopRight => (dx, -dy),
			ResizeCorner::BottomLeft => (-dx, dy),
			ResizeCorner::BottomRight => (dx, dy),
		};

		let new_window_width = (self.initial_window_size.w as f64 + dx) as i32;
		let new_window_width = i32::max(new_window_width, 50);

		let new_window_height = (self.initial_window_size.h as f64 + dy) as i32;
		let new_window_height = i32::max(new_window_height, 50);

		self.new_window_size = Size::new(new_window_width, new_window_height);
		match self.window.underlying_surface() {
			WindowSurface::Wayland(xdg) => {
				xdg.with_pending_state(|state| {
					state.states.set(xdg_toplevel::State::Resizing);
					state.size = Some(self.new_window_size);
				});
				xdg.send_pending_configure();
			}
		}
	}

	fn relative_motion(
		&mut self,
		data: &mut State,
		handle: &mut PointerInnerHandle<'_, State>,
		_focus: Option<(<State as SeatHandler>::PointerFocus, Point<f64, Logical>)>,
		event: &RelativeMotionEvent,
	) {
		// no client has pointer focus while grab is active
		handle.relative_motion(data, None, event);
	}

	fn button(&mut self, state: &mut State, handle: &mut PointerInnerHandle<'_, State>, event: &ButtonEvent) {
		handle.button(state, event);

		if !handle.current_pressed().contains(&273) {
			handle.unset_grab(self, state, event.serial, event.time, true);

			if !self.window.alive() {
				return;
			}

			match self.window.underlying_surface() {
				WindowSurface::Wayland(xdg) => {
					xdg.with_pending_state(|state| {
						state.states.unset(xdg_toplevel::State::Resizing);
						state.size = Some(self.new_window_size);
					});
					xdg.send_pending_configure();

					let mut guard = self.window.resize_state.lock().unwrap();
					if let Some(ResizeState::Resizing(data)) = *guard {
						*guard = Some(ResizeState::WatingForCommit(data));
					}
				}
			}
		}
	}

	fn axis(&mut self, data: &mut State, handle: &mut PointerInnerHandle<'_, State>, details: AxisFrame) {
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

	fn unset(&mut self, state: &mut State) {
		state.mayland.cursor.icon = None;
		state.mayland.queue_redraw_all();
	}
}

impl State {
	pub fn xdg_floating_move(&mut self, window: MappedWindow, serial: Serial) {
		let pointer = self.mayland.pointer.clone();

		if !pointer.has_grab(serial) {
			return;
		}

		let start_data = pointer.grab_start_data().unwrap();
		let Some(((grab_focus, _), wl_surface)) = start_data.focus.as_ref().zip(window.wl_surface()) else {
			return;
		};

		if !grab_focus.same_client_as(&wl_surface.id()) {
			return;
		}

		let pointer_location = pointer.current_location().to_i32_round();

		let workspace = self.mayland.workspaces.workspace().unwrap();
		let window_geometry = workspace.window_geometry(&window).unwrap();
		let window_offset = window_geometry.loc - pointer_location;

		self.mayland.cursor.icon = Some(CursorIcon::Grabbing);
		self.mayland.queue_redraw_all();

		let grab = MoveGrab {
			start_data,
			window,
			window_offset,
		};
		pointer.set_grab(self, grab, serial, Focus::Clear);
	}

	pub fn xdg_floating_resize(&mut self, window: MappedWindow, serial: Serial) {
		let pointer = self.mayland.pointer.clone();

		if !pointer.has_grab(serial) {
			return;
		}

		let start_data = pointer.grab_start_data().unwrap();
		let Some(((grab_focus, _), wl_surface)) = start_data.focus.as_ref().zip(window.wl_surface()) else {
			return;
		};

		if !grab_focus.same_client_as(&wl_surface.id()) {
			return;
		}

		let workspace = self.mayland.workspaces.workspace().unwrap();
		let window_geometry = workspace.window_geometry(&window).unwrap();
		let pointer_location = pointer.current_location().to_i32_round();

		let output_location = self.mayland.workspaces.active_output_position().unwrap();
		let relative_position = pointer_location - output_location - window_geometry.loc;
		let window_size = window_geometry.size;

		let half_height = window_size.h / 2;
		let is_top = relative_position.y <= half_height;

		let half_width = window_size.w / 2;
		let is_left = relative_position.x <= half_width;

		let corner = ResizeCorner::new(is_top, is_left);

		let resize_data = ResizeData {
			corner,
			initial_window_location: window_geometry.loc,
			initial_window_size: window_size,
		};

		let mut guard = window.resize_state.lock().unwrap();
		*guard = Some(ResizeState::Resizing(resize_data));
		drop(guard);

		let cursor_icon = match corner {
			ResizeCorner::TopLeft => CursorIcon::NwResize,
			ResizeCorner::TopRight => CursorIcon::NeResize,
			ResizeCorner::BottomLeft => CursorIcon::SwResize,
			ResizeCorner::BottomRight => CursorIcon::SeResize,
		};
		self.mayland.cursor.icon = Some(cursor_icon);
		self.mayland.queue_redraw_all();

		let grab = ResizeGrab {
			start_data,
			corner,
			window,
			initial_window_size: window_size,
			new_window_size: window_size,
		};
		pointer.set_grab(self, grab, serial, Focus::Clear);
	}
}
