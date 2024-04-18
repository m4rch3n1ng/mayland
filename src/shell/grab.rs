use super::element::WindowElement;
use crate::state::State;
use smithay::{
	input::{
		pointer::{
			AxisFrame, ButtonEvent, Focus, GestureHoldBeginEvent, GestureHoldEndEvent,
			GesturePinchBeginEvent, GesturePinchEndEvent, GesturePinchUpdateEvent,
			GestureSwipeBeginEvent, GestureSwipeEndEvent, GestureSwipeUpdateEvent, GrabStartData,
			MotionEvent, PointerGrab, PointerInnerHandle, RelativeMotionEvent,
		},
		SeatHandler,
	},
	reexports::wayland_server::Resource,
	utils::{Logical, Point, Serial},
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
		if handle.current_pressed().is_empty() {
			handle.unset_grab(data, event.serial, event.time, true);
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
	pub fn xdg_move(&mut self, window: WindowElement, serial: Serial) {
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

		let size = window.0.geometry().size;
		let window_offset = Point::from((-size.w / 2, -size.h / 2));

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
}
