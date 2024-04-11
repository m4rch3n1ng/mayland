use super::focus::PointerFocusTarget;
use smithay::{
	backend::renderer::{
		element::{surface::WaylandSurfaceRenderElement, AsRenderElements},
		ImportAll, ImportMem, Renderer, Texture,
	},
	desktop::{space::SpaceElement, Window, WindowSurface, WindowSurfaceType},
	output::Output,
	reexports::wayland_server::protocol::wl_surface::WlSurface,
	utils::{IsAlive, Logical, Physical, Point, Rectangle, Scale},
	wayland::seat::WaylandFocus,
};

#[derive(Debug, Clone, PartialEq)]
pub struct WindowElement(pub Window);

impl WindowElement {
	pub fn surface_under(
		&self,
		location: Point<f64, Logical>,
		window_type: WindowSurfaceType,
	) -> Option<(PointerFocusTarget, Point<i32, Logical>)> {
		let surface_under = self.0.surface_under(location, window_type);
		match self.0.underlying_surface() {
			WindowSurface::Wayland(_) => {
				surface_under.map(|(surface, loc)| (PointerFocusTarget::WlSurface(surface), loc))
			}
		}
	}

	pub fn underlying_surface(&self) -> &WindowSurface {
		self.0.underlying_surface()
	}

	pub fn wl_surface(&self) -> Option<WlSurface> {
		self.0.wl_surface()
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
		self.0.set_activate(activated)
	}

	fn output_enter(&self, output: &Output, overlap: Rectangle<i32, Logical>) {
		self.0.output_enter(output, overlap)
	}

	fn output_leave(&self, output: &Output) {
		self.0.output_leave(output)
	}

	fn refresh(&self) {
		self.0.refresh()
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
