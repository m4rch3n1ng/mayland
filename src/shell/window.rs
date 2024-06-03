use super::grab::ResizeState;
use crate::state::State;
use smithay::{
	backend::renderer::{
		element::{surface::WaylandSurfaceRenderElement, AsRenderElements},
		ImportAll, ImportMem, Renderer, Texture,
	},
	desktop::{space::SpaceElement, Window, WindowSurface},
	input::{
		pointer::{
			AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent, GesturePinchBeginEvent,
			GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent, GestureSwipeEndEvent,
			GestureSwipeUpdateEvent, MotionEvent, PointerTarget, RelativeMotionEvent,
		},
		Seat,
	},
	output::Output,
	reexports::wayland_server::protocol::wl_surface::WlSurface,
	utils::{IsAlive, Logical, Physical, Point, Rectangle, Scale, Serial},
	wayland::seat::WaylandFocus,
};
use std::{
	borrow::Cow,
	sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub struct MappedWindow {
	pub window: Window,
	pub resize_state: Arc<Mutex<Option<ResizeState>>>,
}

impl PartialEq for MappedWindow {
	fn eq(&self, other: &Self) -> bool {
		self.window == other.window
	}
}

impl MappedWindow {
	pub fn new(window: Window) -> Self {
		MappedWindow {
			window,
			resize_state: Arc::new(Mutex::new(None)),
		}
	}

	pub fn close(&self) {
		if let Some(toplevel) = self.window.toplevel() {
			toplevel.send_close();
		}
	}

	pub fn underlying_surface(&self) -> &WindowSurface {
		self.window.underlying_surface()
	}
}

impl IsAlive for MappedWindow {
	fn alive(&self) -> bool {
		self.window.alive()
	}
}

impl SpaceElement for MappedWindow {
	fn geometry(&self) -> Rectangle<i32, Logical> {
		self.window.geometry()
	}

	fn bbox(&self) -> Rectangle<i32, Logical> {
		self.window.bbox()
	}

	fn is_in_input_region(&self, point: &smithay::utils::Point<f64, Logical>) -> bool {
		self.window.is_in_input_region(point)
	}

	fn z_index(&self) -> u8 {
		self.window.z_index()
	}

	fn set_activate(&self, activated: bool) {
		self.window.set_activate(activated);
	}

	fn output_enter(&self, output: &Output, overlap: Rectangle<i32, Logical>) {
		self.window.output_enter(output, overlap);
	}

	fn output_leave(&self, output: &Output) {
		self.window.output_leave(output);
	}

	fn refresh(&self) {
		self.window.refresh();
	}
}

impl<R> AsRenderElements<R> for MappedWindow
where
	R: Renderer + ImportAll + ImportMem,
	<R as Renderer>::TextureId: Clone + Texture + 'static,
{
	type RenderElement = WaylandSurfaceRenderElement<R>;

	fn render_elements<C: From<Self::RenderElement>>(
		&self,
		renderer: &mut R,
		location: Point<i32, Physical>,
		scale: Scale<f64>,
		alpha: f32,
	) -> Vec<C> {
		self.window.render_elements(renderer, location, scale, alpha)
	}
}

impl PointerTarget<State> for MappedWindow {
	fn enter(&self, seat: &smithay::input::Seat<State>, data: &mut State, event: &MotionEvent) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::enter(&*w, seat, data, event);
		}
	}

	fn motion(&self, seat: &smithay::input::Seat<State>, data: &mut State, event: &MotionEvent) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::motion(&*w, seat, data, event);
		}
	}

	fn relative_motion(
		&self,
		seat: &smithay::input::Seat<State>,
		data: &mut State,
		event: &RelativeMotionEvent,
	) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::relative_motion(&*w, seat, data, event);
		}
	}

	fn button(&self, seat: &Seat<State>, data: &mut State, event: &ButtonEvent) {
		let mods = data.mayland.keyboard.modifier_state();
		if mods.alt {
			let button = event.button;

			if button == 272 {
				let serial = event.serial;
				let window = self.clone();
				data.mayland
					.loop_handle
					.insert_idle(move |state| state.xdg_move(window, serial));
			} else if button == 273 {
				let serial = event.serial;
				let window = self.clone();
				data.mayland
					.loop_handle
					.insert_idle(move |state| state.xdg_resize(window, serial));
			}
		} else if let Some(w) = self.wl_surface() {
			PointerTarget::button(&*w, seat, data, event);
		}
	}

	fn axis(&self, seat: &smithay::input::Seat<State>, data: &mut State, frame: AxisFrame) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::axis(&*w, seat, data, frame);
		}
	}

	fn frame(&self, seat: &smithay::input::Seat<State>, data: &mut State) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::frame(&*w, seat, data);
		}
	}

	fn gesture_swipe_begin(
		&self,
		seat: &smithay::input::Seat<State>,
		data: &mut State,
		event: &GestureSwipeBeginEvent,
	) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::gesture_swipe_begin(&*w, seat, data, event);
		}
	}

	fn gesture_swipe_update(
		&self,
		seat: &smithay::input::Seat<State>,
		data: &mut State,
		event: &GestureSwipeUpdateEvent,
	) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::gesture_swipe_update(&*w, seat, data, event);
		}
	}

	fn gesture_swipe_end(
		&self,
		seat: &smithay::input::Seat<State>,
		data: &mut State,
		event: &GestureSwipeEndEvent,
	) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::gesture_swipe_end(&*w, seat, data, event);
		}
	}

	fn gesture_pinch_begin(
		&self,
		seat: &smithay::input::Seat<State>,
		data: &mut State,
		event: &GesturePinchBeginEvent,
	) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::gesture_pinch_begin(&*w, seat, data, event);
		}
	}

	fn gesture_pinch_update(
		&self,
		seat: &smithay::input::Seat<State>,
		data: &mut State,
		event: &GesturePinchUpdateEvent,
	) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::gesture_pinch_update(&*w, seat, data, event);
		}
	}

	fn gesture_pinch_end(
		&self,
		seat: &smithay::input::Seat<State>,
		data: &mut State,
		event: &GesturePinchEndEvent,
	) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::gesture_pinch_end(&*w, seat, data, event);
		}
	}

	fn gesture_hold_begin(
		&self,
		seat: &smithay::input::Seat<State>,
		data: &mut State,
		event: &GestureHoldBeginEvent,
	) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::gesture_hold_begin(&*w, seat, data, event);
		}
	}

	fn gesture_hold_end(
		&self,
		seat: &smithay::input::Seat<State>,
		data: &mut State,
		event: &GestureHoldEndEvent,
	) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::gesture_hold_end(&*w, seat, data, event);
		}
	}

	fn leave(&self, seat: &smithay::input::Seat<State>, data: &mut State, serial: Serial, time: u32) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::leave(&*w, seat, data, serial, time);
		}
	}
}

impl WaylandFocus for MappedWindow {
	fn wl_surface(&self) -> Option<Cow<'_, WlSurface>> {
		self.window.wl_surface()
	}
}