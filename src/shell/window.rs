use super::{focus::KeyboardFocusTarget, grab::ResizeState};
use crate::{layout::workspace::Workspace, render::MaylandRenderElements, state::State};
use mayland_config::windowrules::WindowRule;
use smithay::{
	backend::renderer::{
		ImportAll, ImportMem, Renderer,
		element::{AsRenderElements, surface::WaylandSurfaceRenderElement, utils::CropRenderElement},
		glow::GlowRenderer,
	},
	desktop::{Window, WindowSurface, space::SpaceElement},
	input::{
		Seat,
		pointer::{
			AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent, GesturePinchBeginEvent,
			GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent, GestureSwipeEndEvent,
			GestureSwipeUpdateEvent, MotionEvent, PointerTarget, RelativeMotionEvent,
		},
	},
	output::Output,
	reexports::{
		wayland_protocols::xdg::shell::server::xdg_toplevel,
		wayland_server::{DisplayHandle, Resource, protocol::wl_surface::WlSurface},
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
	sync::{Arc, Mutex, RwLock},
};

#[derive(Debug, Clone)]
pub struct MappedWindow {
	pub window: Window,
	pub windowrules: ResolvedWindowRule,
	pub resize_state: Arc<Mutex<Option<ResizeState>>>,
}

impl PartialEq for MappedWindow {
	fn eq(&self, other: &Self) -> bool {
		self.window == other.window
	}
}

impl Eq for MappedWindow {}

impl MappedWindow {
	pub fn resize(&self, rect: Rectangle<i32, Logical>) {
		match self.underlying_surface() {
			WindowSurface::Wayland(xdg) => {
				xdg.with_pending_state(|state| {
					state.size = Some(rect.size);
				});
				xdg.send_pending_configure();
			}
		}
	}
}

impl MappedWindow {
	pub fn new(unmapped: UnmappedSurface, windowrules: WindowRule) -> Self {
		MappedWindow {
			window: unmapped.0,
			windowrules: ResolvedWindowRule::new(windowrules),
			resize_state: Arc::new(Mutex::new(None)),
		}
	}

	pub fn close(&self) {
		match self.underlying_surface() {
			WindowSurface::Wayland(xdg) => xdg.send_close(),
		}
	}

	pub fn on_commit(&self) {
		self.window.on_commit();
	}

	pub fn underlying_surface(&self) -> &WindowSurface {
		self.window.underlying_surface()
	}

	pub fn toplevel(&self) -> Option<&ToplevelSurface> {
		self.window.toplevel()
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
		rect.size = Size::new(
			rect.size.w.saturating_add(geometry.loc.x).max(0),
			rect.size.h.saturating_add(geometry.loc.y).max(0),
		);

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

	pub fn min_max_size(&self) -> (Size<i32, Logical>, Size<i32, Logical>) {
		match self.underlying_surface() {
			WindowSurface::Wayland(xdg) => with_states(xdg.wl_surface(), |states| {
				let mut data = states.cached_state.get::<SurfaceCachedState>();
				let data = data.current();

				let max_size = Size::new(
					if data.max_size.w > 0 {
						data.max_size.w
					} else {
						i32::MAX
					},
					if data.max_size.h > 0 {
						data.max_size.h
					} else {
						i32::MAX
					},
				);

				(data.min_size, max_size)
			}),
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
						// maybe kind of works mostly
						state.states.set(xdg_toplevel::State::Maximized);
					}
				});
				xdg.send_pending_configure();
			}
		}
	}
}

impl MappedWindow {
	pub fn recompute_windowrules(&self, config: &mayland_config::WindowRules) {
		let windowrules = match self.underlying_surface() {
			WindowSurface::Wayland(xdg) => with_states(xdg.wl_surface(), |states| {
				let surface_data = states
					.data_map
					.get::<XdgToplevelSurfaceData>()
					.unwrap()
					.lock()
					.unwrap();

				config.compute(surface_data.app_id.as_deref(), surface_data.title.as_deref())
			}),
		};

		self.windowrules.write(windowrules);
	}
}

impl IsAlive for MappedWindow {
	fn alive(&self) -> bool {
		self.window.alive()
	}
}

impl WaylandFocus for MappedWindow {
	fn wl_surface(&self) -> Option<Cow<'_, WlSurface>> {
		self.window.wl_surface()
	}
}

impl SpaceElement for MappedWindow {
	fn geometry(&self) -> Rectangle<i32, Logical> {
		SpaceElement::geometry(&self.window)
	}

	fn bbox(&self) -> Rectangle<i32, Logical> {
		SpaceElement::bbox(&self.window)
	}

	fn is_in_input_region(&self, point: &smithay::utils::Point<f64, Logical>) -> bool {
		SpaceElement::is_in_input_region(&self.window, point)
	}

	fn z_index(&self) -> u8 {
		SpaceElement::z_index(&self.window)
	}

	fn set_activate(&self, activated: bool) {
		SpaceElement::set_activate(&self.window, activated);
	}

	fn output_enter(&self, output: &Output, overlap: Rectangle<i32, Logical>) {
		SpaceElement::output_enter(&self.window, output, overlap);
	}

	fn output_leave(&self, output: &Output) {
		SpaceElement::output_leave(&self.window, output);
	}

	fn refresh(&self) {
		SpaceElement::refresh(&self.window);
	}
}

impl<R> AsRenderElements<R> for MappedWindow
where
	R::TextureId: Clone + 'static,
	R: Renderer + ImportAll + ImportMem,
{
	type RenderElement = WaylandSurfaceRenderElement<R>;

	fn render_elements<C: From<Self::RenderElement>>(
		&self,
		renderer: &mut R,
		location: Point<i32, Physical>,
		scale: Scale<f64>,
		alpha: f32,
	) -> Vec<C> {
		let opacity = self.windowrules.opacity().unwrap_or(1.).clamp(0., 1.);
		let alpha = opacity * alpha;

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
		self.render_elements(renderer, rect.loc, scale, alpha)
			.into_iter()
			.filter_map(|element| CropRenderElement::from_element(element, scale, rect))
			.map(MaylandRenderElements::CropSurface)
			.collect()
	}
}

impl PartialEq<ToplevelSurface> for MappedWindow {
	fn eq(&self, other: &ToplevelSurface) -> bool {
		match self.underlying_surface() {
			WindowSurface::Wayland(xdg) => xdg == other,
		}
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

impl PartialEq<WlSurface> for MappedWindow {
	fn eq(&self, other: &WlSurface) -> bool {
		self.wl_surface().is_some_and(|w| &*w == other)
	}
}

impl MappedWindow {
	/// get [`mayland_comm::Window`] info for [`mayland`]
	pub fn comm_info(
		&self,
		geometry: Rectangle<i32, Logical>,
		workspace: &Workspace,
		keyboard_focus: Option<&KeyboardFocusTarget>,
		display_handle: &DisplayHandle,
	) -> mayland_comm::Window {
		let active = keyboard_focus.is_some_and(|focus| focus == self);

		let absolute = workspace.output.as_ref().map(|output| {
			let output_location = output.current_location();
			let mut absolute = geometry;
			absolute.loc += output_location;

			mayland_comm::window::Geometry {
				x: absolute.loc.x,
				y: absolute.loc.y,
				w: absolute.size.w,
				h: absolute.size.h,
			}
		});

		let relative = mayland_comm::window::Geometry {
			x: geometry.loc.x,
			y: geometry.loc.y,
			w: geometry.size.w,
			h: geometry.size.h,
		};

		match self.underlying_surface() {
			WindowSurface::Wayland(xdg) => with_states(xdg.wl_surface(), |states| {
				let surface_data = states
					.data_map
					.get::<XdgToplevelSurfaceData>()
					.unwrap()
					.lock()
					.unwrap();

				let pid = (display_handle.get_client(xdg.wl_surface().id()).ok())
					.and_then(|client| client.get_credentials(display_handle).ok())
					.map(|credentials| credentials.pid);

				mayland_comm::Window {
					relative,
					absolute,

					app_id: surface_data.app_id.clone(),
					title: surface_data.title.clone(),
					pid,

					workspace: workspace.idx,
					active,

					xwayland: false,
				}
			}),
		}
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

	pub fn compute_windowrules(&self, windowrules: &mayland_config::WindowRules) -> WindowRule {
		match self.0.underlying_surface() {
			WindowSurface::Wayland(xdg) => with_states(xdg.wl_surface(), |states| {
				let surface_data = states
					.data_map
					.get::<XdgToplevelSurfaceData>()
					.unwrap()
					.lock()
					.unwrap();

				windowrules.compute(surface_data.app_id.as_deref(), surface_data.title.as_deref())
			}),
		}
	}
}

impl IsAlive for UnmappedSurface {
	fn alive(&self) -> bool {
		self.0.alive()
	}
}

impl WaylandFocus for UnmappedSurface {
	fn wl_surface(&self) -> Option<Cow<'_, WlSurface>> {
		self.0.wl_surface()
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

#[derive(Debug, Clone)]
pub struct ResolvedWindowRule(Arc<RwLock<WindowRule>>);

impl ResolvedWindowRule {
	fn new(windowrules: WindowRule) -> Self {
		let inner = Arc::new(RwLock::new(windowrules));
		ResolvedWindowRule(inner)
	}

	fn write(&self, windowrules: WindowRule) {
		*self.0.write().unwrap() = windowrules;
	}

	pub fn floating(&self) -> Option<bool> {
		self.0.read().unwrap().floating
	}

	pub fn opacity(&self) -> Option<f32> {
		self.0.read().unwrap().opacity
	}
}
