use super::grab::ResizeState;
use crate::{render::MaylandRenderElements, state::State};
use smithay::{
	backend::renderer::{
		element::{surface::WaylandSurfaceRenderElement, utils::CropRenderElement, AsRenderElements},
		glow::GlowRenderer,
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
	reexports::{
		wayland_protocols::xdg::shell::server::xdg_toplevel, wayland_server::protocol::wl_surface::WlSurface,
	},
	utils::{IsAlive, Logical, Physical, Point, Rectangle, Scale, Serial, Size},
	wayland::{
		compositor::with_states,
		seat::WaylandFocus,
		shell::xdg::{SurfaceCachedState, ToplevelSurface, XdgToplevelSurfaceData},
	},
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

impl Eq for MappedWindow {}

impl MappedWindow {
	pub fn resize(&self, size: Size<i32, Logical>) {
		match self.underlying_surface() {
			WindowSurface::Wayland(xdg) => {
				xdg.with_pending_state(|state| {
					state.size = Some(size);
				});
				xdg.send_pending_configure();
			}
		}
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
		match self.underlying_surface() {
			WindowSurface::Wayland(xdg) => xdg.send_close(),
		}
	}

	pub fn underlying_surface(&self) -> &WindowSurface {
		self.window.underlying_surface()
	}

	pub fn render_location(&self, location: Point<i32, Logical>) -> Point<i32, Logical> {
		let geometry = self.geometry();
		location - geometry.loc
	}

	/// like [`MappedWindow::render_location`], but it also adds the location offset
	/// to the size of the rectangle to make it work with cropped rendering
	pub fn render_rectangle(&self, mut rect: Rectangle<i32, Logical>) -> Rectangle<i32, Logical> {
		let geometry = self.geometry();

		rect.loc -= geometry.loc;
		rect.size = (rect.size.to_point() + geometry.loc).to_size();

		assert!(rect.size.w >= 0, "size should be nonnegative");
		assert!(rect.size.h >= 0, "size should be nonnegative");

		rect
	}
}

impl MappedWindow {
	/// check if surface is non-resizable
	pub fn is_non_resizable(&self) -> bool {
		match self.underlying_surface() {
			WindowSurface::Wayland(xdg) => {
				let (min, max) = with_states(xdg.wl_surface(), |states| {
					let mut guard = states.cached_state.get::<SurfaceCachedState>();
					let data = guard.current();
					(data.min_size, data.max_size)
				});

				min.w > 0 && min.h > 0 && min == max
			}
		}
	}

	pub fn set_tiled(&self) {
		match self.underlying_surface() {
			WindowSurface::Wayland(xdg) => {
				xdg.with_pending_state(|state| {
					if xdg.version() >= 2 {
						// if the surface supports it set the tiled state
						state.states.set(xdg_toplevel::State::TiledTop);
						state.states.set(xdg_toplevel::State::TiledRight);
						state.states.set(xdg_toplevel::State::TiledBottom);
						state.states.set(xdg_toplevel::State::TiledLeft);
					} else {
						// if xdg shell version is lower than 2 the surface doesn't
						// support tiled state, so set it to maximized because that
						// kind of works mostly
						state.states.set(xdg_toplevel::State::Maximized);
					}
				});
				xdg.send_pending_configure();
			}
		}
	}

	pub fn unset_tiled(&self) {
		match self.underlying_surface() {
			WindowSurface::Wayland(xdg) => {
				xdg.with_pending_state(|state| {
					if xdg.version() >= 2 {
						state.states.unset(xdg_toplevel::State::TiledTop);
						state.states.unset(xdg_toplevel::State::TiledRight);
						state.states.unset(xdg_toplevel::State::TiledBottom);
						state.states.unset(xdg_toplevel::State::TiledLeft);
					} else {
						state.states.unset(xdg_toplevel::State::Maximized);
					}
				});
				xdg.send_pending_configure();
			}
		}
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

impl MappedWindow {
	pub fn crop_render_elements(
		&self,
		renderer: &mut GlowRenderer,
		rect: Rectangle<i32, Physical>,
		scale: Scale<f64>,
		alpha: f32,
	) -> Vec<MaylandRenderElements> {
		self.window
			.render_elements(renderer, rect.loc, scale, alpha)
			.into_iter()
			.filter_map(|element| CropRenderElement::from_element(element, scale, rect))
			.map(MaylandRenderElements::CropSurface)
			.collect()
	}
}

impl PartialEq<WlSurface> for MappedWindow {
	fn eq(&self, other: &WlSurface) -> bool {
		self.wl_surface().is_some_and(|w| &*w == other)
	}
}

impl From<UnmappedSurface> for MappedWindow {
	fn from(unmapped: UnmappedSurface) -> Self {
		MappedWindow::new(unmapped.0)
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
		if mods == data.mayland.comp_mod {
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

impl From<&MappedWindow> for mayland_comm::workspace::Window {
	fn from(window: &MappedWindow) -> Self {
		match window.underlying_surface() {
			WindowSurface::Wayland(toplevel) => {
				let wl_surface = toplevel.wl_surface();
				with_states(wl_surface, |states| {
					let surface_data = states
						.data_map
						.get::<XdgToplevelSurfaceData>()
						.unwrap()
						.lock()
						.unwrap();

					mayland_comm::workspace::Window {
						app_id: surface_data.app_id.clone(),
						title: surface_data.title.clone(),
					}
				})
			}
		}
	}
}

#[derive(Debug, PartialEq, Eq)]
pub struct UnmappedSurface(Window);

impl UnmappedSurface {
	pub fn toplevel(&self) -> Option<&ToplevelSurface> {
		self.0.toplevel()
	}
}

impl PartialEq<ToplevelSurface> for UnmappedSurface {
	fn eq(&self, other: &ToplevelSurface) -> bool {
		match self.0.underlying_surface() {
			WindowSurface::Wayland(toplevel) => toplevel == other,
		}
	}
}

impl PartialEq<WlSurface> for UnmappedSurface {
	fn eq(&self, wl_surface: &WlSurface) -> bool {
		self.0.wl_surface().is_some_and(|w| &*w == wl_surface)
	}
}

impl From<ToplevelSurface> for UnmappedSurface {
	fn from(toplevel: ToplevelSurface) -> Self {
		let window = Window::new_wayland_window(toplevel);
		UnmappedSurface(window)
	}
}

impl WaylandFocus for UnmappedSurface {
	fn wl_surface(&self) -> Option<Cow<'_, WlSurface>> {
		self.0.wl_surface()
	}
}
