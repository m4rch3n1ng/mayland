use crate::state::State;
use smithay::{
	backend::renderer::{
		element::{surface::WaylandSurfaceRenderElement, AsRenderElements},
		ImportAll, ImportMem, Renderer, Texture,
	},
	desktop::{space::SpaceElement, Window, WindowSurface},
	input::pointer::{
		AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent, GesturePinchBeginEvent,
		GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent,
		GestureSwipeEndEvent, GestureSwipeUpdateEvent, MotionEvent, PointerTarget,
		RelativeMotionEvent,
	},
	output::Output,
	reexports::wayland_server::protocol::wl_surface::WlSurface,
	utils::{IsAlive, Logical, Physical, Point, Rectangle, Scale, Serial},
	wayland::seat::WaylandFocus,
};

#[derive(Debug, Clone, PartialEq)]
pub struct WindowElement(pub Window);

impl WindowElement {
	pub fn close(&self) {
		if let Some(toplevel) = self.0.toplevel() {
			toplevel.send_close();
		}
	}

	pub fn underlying_surface(&self) -> &WindowSurface {
		self.0.underlying_surface()
	}
}

impl IsAlive for WindowElement {
	fn alive(&self) -> bool {
		self.0.alive()
	}
}

impl SpaceElement for WindowElement {
	fn geometry(&self) -> Rectangle<i32, Logical> {
		self.0.geometry()
	}

	fn bbox(&self) -> Rectangle<i32, Logical> {
		self.0.bbox()
	}

	fn is_in_input_region(&self, point: &smithay::utils::Point<f64, Logical>) -> bool {
		self.0.is_in_input_region(point)
	}

	fn z_index(&self) -> u8 {
		self.0.z_index()
	}

	fn set_activate(&self, activated: bool) {
		self.0.set_activate(activated);
	}

	fn output_enter(&self, output: &Output, overlap: Rectangle<i32, Logical>) {
		self.0.output_enter(output, overlap);
	}

	fn output_leave(&self, output: &Output) {
		self.0.output_leave(output);
	}

	fn refresh(&self) {
		self.0.refresh();
	}
}

impl<R> AsRenderElements<R> for WindowElement
where
	R: Renderer + ImportAll + ImportMem,
	<R as Renderer>::TextureId: Texture + 'static,
{
	type RenderElement = WaylandSurfaceRenderElement<R>;

	fn render_elements<C: From<Self::RenderElement>>(
		&self,
		renderer: &mut R,
		location: Point<i32, Physical>,
		scale: Scale<f64>,
		alpha: f32,
	) -> Vec<C> {
		self.0.render_elements(renderer, location, scale, alpha)
	}
}

impl PointerTarget<State> for WindowElement {
	fn enter(&self, seat: &smithay::input::Seat<State>, data: &mut State, event: &MotionEvent) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::enter(&w, seat, data, event);
		}
	}

	fn motion(&self, seat: &smithay::input::Seat<State>, data: &mut State, event: &MotionEvent) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::motion(&w, seat, data, event);
		}
	}

	fn relative_motion(
		&self,
		seat: &smithay::input::Seat<State>,
		data: &mut State,
		event: &RelativeMotionEvent,
	) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::relative_motion(&w, seat, data, event);
		}
	}

	fn button(&self, seat: &smithay::input::Seat<State>, data: &mut State, event: &ButtonEvent) {
		let mods = data.mayland.keyboard.modifier_state();
		if mods.alt {
			println!("is alt {:?}", mods.alt);
		} else if let Some(w) = self.wl_surface() {
			PointerTarget::button(&w, seat, data, event);
		}
	}

	fn axis(&self, seat: &smithay::input::Seat<State>, data: &mut State, frame: AxisFrame) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::axis(&w, seat, data, frame);
		}
	}

	fn frame(&self, seat: &smithay::input::Seat<State>, data: &mut State) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::frame(&w, seat, data);
		}
	}

	fn gesture_swipe_begin(
		&self,
		seat: &smithay::input::Seat<State>,
		data: &mut State,
		event: &GestureSwipeBeginEvent,
	) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::gesture_swipe_begin(&w, seat, data, event);
		}
	}

	fn gesture_swipe_update(
		&self,
		seat: &smithay::input::Seat<State>,
		data: &mut State,
		event: &GestureSwipeUpdateEvent,
	) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::gesture_swipe_update(&w, seat, data, event);
		}
	}

	fn gesture_swipe_end(
		&self,
		seat: &smithay::input::Seat<State>,
		data: &mut State,
		event: &GestureSwipeEndEvent,
	) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::gesture_swipe_end(&w, seat, data, event);
		}
	}

	fn gesture_pinch_begin(
		&self,
		seat: &smithay::input::Seat<State>,
		data: &mut State,
		event: &GesturePinchBeginEvent,
	) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::gesture_pinch_begin(&w, seat, data, event);
		}
	}

	fn gesture_pinch_update(
		&self,
		seat: &smithay::input::Seat<State>,
		data: &mut State,
		event: &GesturePinchUpdateEvent,
	) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::gesture_pinch_update(&w, seat, data, event);
		}
	}

	fn gesture_pinch_end(
		&self,
		seat: &smithay::input::Seat<State>,
		data: &mut State,
		event: &GesturePinchEndEvent,
	) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::gesture_pinch_end(&w, seat, data, event);
		}
	}

	fn gesture_hold_begin(
		&self,
		seat: &smithay::input::Seat<State>,
		data: &mut State,
		event: &GestureHoldBeginEvent,
	) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::gesture_hold_begin(&w, seat, data, event);
		}
	}

	fn gesture_hold_end(
		&self,
		seat: &smithay::input::Seat<State>,
		data: &mut State,
		event: &GestureHoldEndEvent,
	) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::gesture_hold_end(&w, seat, data, event);
		}
	}

	fn leave(
		&self,
		seat: &smithay::input::Seat<State>,
		data: &mut State,
		serial: Serial,
		time: u32,
	) {
		if let Some(w) = self.wl_surface() {
			PointerTarget::leave(&w, seat, data, serial, time);
		}
	}
}

impl WaylandFocus for WindowElement {
	fn wl_surface(&self) -> Option<WlSurface> {
		self.0.wl_surface()
	}
}
