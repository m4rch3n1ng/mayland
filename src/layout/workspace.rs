use crate::{
	render::{MaylandRenderElements, OutputRenderElements},
	shell::window::MappedWindow,
};
use smithay::{
	backend::renderer::{
		element::{surface::WaylandSurfaceRenderElement, AsRenderElements},
		glow::GlowRenderer,
	},
	desktop::{layer_map_for_output, LayerMap, LayerSurface, Space},
	output::Output,
	reexports::wayland_server::protocol::wl_surface::WlSurface,
	utils::{Logical, Physical, Point, Rectangle, Scale},
	wayland::{seat::WaylandFocus, shell::wlr_layer::Layer},
};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug)]
pub struct WorkspaceManager {
	space: Space<MappedWindow>,

	active_output: Option<Output>,
	output_map: HashMap<Output, usize>,

	workspaces: BTreeMap<usize, Workspace>,
}

impl WorkspaceManager {
	pub fn new() -> Self {
		let space = Space::default();

		let active_output = None;
		let output_map = HashMap::new();

		let workspace = Workspace::new();
		let workspaces = BTreeMap::from([(0, workspace)]);
		// let current = 0;

		WorkspaceManager {
			space,
			active_output,
			output_map,
			workspaces,
			// current,
		}
	}
}

impl WorkspaceManager {
	pub fn switch_to_workspace(&mut self, idx: usize) {
		let Some(active_output) = &self.active_output else {
			return;
		};

		let current = self.output_map.get_mut(active_output).unwrap();
		if &idx == current {
			return;
		}

		if let Some(prev) = self.workspaces.get_mut(current) {
			prev.unmap_output(active_output);
		}

		let workspace = self.workspaces.entry(idx).or_insert_with(Workspace::new);
		workspace.map_output(active_output);
		*current = idx;
	}

	pub fn workspace(&self) -> &Workspace {
		if let Some(output) = &self.active_output {
			let idx = self.output_map[output];
			self.workspaces.get(&idx).unwrap()
		} else {
			todo!()
		}
	}

	pub fn workspace_mut(&mut self) -> &mut Workspace {
		if let Some(output) = &self.active_output {
			let idx = self.output_map[output];
			self.workspaces.get_mut(&idx).unwrap()
		} else {
			todo!()
		}
	}
}

impl WorkspaceManager {
	pub fn add_output(&mut self, output: &Output) {
		// todo multiple outputs
		assert!(self.active_output.is_none());

		let x = self
			.space
			.outputs()
			.map(|output| self.space.output_geometry(output).unwrap())
			.map(|geom| geom.loc.x + geom.size.w)
			.max()
			.unwrap_or(0);

		self.space.map_output(output, (x, 0));

		// todo don't hard code
		let idx = 0;

		self.output_map.insert(output.clone(), idx);
		self.active_output = Some(output.clone());

		let workspace = self.workspaces.entry(idx).or_insert_with(Workspace::new);
		workspace.map_output(output);
	}

	pub fn remove_output(&mut self, output: &Output) {
		self.space.unmap_output(output);

		let workspace = self.workspace_mut();
		workspace.unmap_output(output);

		if self.active_output.as_ref() == Some(output) {
			// todo
			self.active_output = None;
		}
	}

	pub fn refresh(&mut self) {
		self.space.refresh();

		let workspace = self.workspace_mut();
		workspace.refresh();
	}

	pub fn render_elements(
		&self,
		renderer: &mut GlowRenderer,
		output: &Output,
	) -> impl Iterator<Item = MaylandRenderElements> {
		let workspace = self.workspace();
		workspace.render_elements(renderer, output)
	}
}

impl WorkspaceManager {
	pub fn outputs(&self) -> impl Iterator<Item = &Output> {
		self.space.outputs()
	}

	pub fn outputs_for_window(&self, window: &MappedWindow) -> Vec<Output> {
		let workspace = self.workspace();
		workspace.outputs_for_window(window)
	}

	pub fn output_geometry(&self, output: &Output) -> Option<Rectangle<i32, Logical>> {
		self.space.output_geometry(output)
	}

	pub fn output_under<P: Into<Point<f64, Logical>>>(&self, point: P) -> impl Iterator<Item = &Output> {
		self.space.output_under(point)
	}
}

impl WorkspaceManager {
	pub fn window_for_surface(&self, surface: &WlSurface) -> Option<&MappedWindow> {
		let window = self
			.workspaces
			.iter()
			.flat_map(|(_i, w)| w.windows())
			.find(|&w| w.wl_surface().is_some_and(|w| *w == *surface));

		window
	}
}

impl WorkspaceManager {
	pub fn add_window(&mut self, window: MappedWindow) {
		let workspace = self.workspace_mut();
		workspace.add_window(window);
	}

	pub fn floating_move<P: Into<Point<i32, Logical>>>(&mut self, window: MappedWindow, location: P) {
		let workspace = self.workspace_mut();
		workspace.floating_move(window, location);
	}

	pub fn remove_window(&mut self, window: &MappedWindow) {
		for workspace in self.workspaces.values_mut() {
			if workspace.has_window(window) {
				workspace.remove_window(window);
				return;
			}
		}
	}

	pub fn raise_window(&mut self, window: &MappedWindow, activate: bool) {
		let workspace = self.workspace_mut();
		workspace.raise_window(window, activate);
	}

	pub fn windows(&self) -> impl DoubleEndedIterator<Item = &MappedWindow> {
		let workspace = self.workspace();
		workspace.windows()
	}

	pub fn window_location(&self, window: &MappedWindow) -> Option<Point<i32, Logical>> {
		let workspace = self.workspace();
		workspace.window_location(window)
	}

	pub fn window_geometry(&self, window: &MappedWindow) -> Option<Rectangle<i32, Logical>> {
		let workspace = self.workspace();
		workspace.window_geometry(window)
	}

	pub fn window_under<P: Into<Point<f64, Logical>>>(
		&self,
		location: P,
	) -> Option<(&MappedWindow, Point<i32, Logical>)> {
		let workspace = self.workspace();
		workspace.window_under(location)
	}
}

impl Default for WorkspaceManager {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Debug)]
pub struct Workspace {
	space: Space<MappedWindow>,
}

impl Workspace {
	fn new() -> Self {
		let space = Space::default();

		Workspace { space }
	}
}

impl Workspace {
	fn map_output(&mut self, output: &Output) {
		self.space.map_output(output, (0, 0));
	}

	fn unmap_output(&mut self, output: &Output) {
		self.space.unmap_output(output);
	}

	fn outputs_for_window(&self, window: &MappedWindow) -> Vec<Output> {
		self.space.outputs_for_element(window)
	}

	fn refresh(&mut self) {
		self.space.refresh();
	}

	pub fn is_empty(&self) -> bool {
		self.windows().count() == 0
	}
}

impl Workspace {
	fn render_elements(
		&self,
		renderer: &mut GlowRenderer,
		output: &Output,
	) -> impl Iterator<Item = MaylandRenderElements> {
		let mut render_elements = Vec::new();

		let layer_map = layer_map_for_output(output);
		let output_scale = output.current_scale().fractional_scale();

		let (lower, upper) = self.layer_elements(&layer_map, output_scale);

		render_elements.extend(upper.flat_map(|(surface, location)| {
			AsRenderElements::<_>::render_elements::<WaylandSurfaceRenderElement<_>>(
				surface,
				renderer,
				location,
				Scale::from(output_scale),
				1.,
			)
			.into_iter()
			.map(OutputRenderElements::Surface)
		}));

		if let Some(output_geo) = self.space.output_geometry(output) {
			render_elements.extend(
				self.space
					.render_elements_for_region(renderer, &output_geo, output_scale, 1.)
					.into_iter()
					.map(OutputRenderElements::Surface),
			);
		}

		render_elements.extend(lower.flat_map(|(surface, location)| {
			AsRenderElements::<_>::render_elements::<WaylandSurfaceRenderElement<_>>(
				surface,
				renderer,
				location,
				Scale::from(output_scale),
				1.,
			)
			.into_iter()
			.map(OutputRenderElements::Surface)
		}));

		render_elements.into_iter()
	}

	fn layer_elements<'o>(
		&self,
		layer_map: &'o LayerMap,
		output_scale: f64,
	) -> (
		impl Iterator<Item = (&'o LayerSurface, Point<i32, Physical>)>,
		impl Iterator<Item = (&'o LayerSurface, Point<i32, Physical>)>,
	) {
		let upper = layer_map
			.layers()
			.filter(|surface| matches!(surface.layer(), Layer::Top | Layer::Overlay))
			.filter_map(move |surface| {
				layer_map
					.layer_geometry(surface)
					.map(|geo| (surface, geo.loc.to_physical_precise_round(output_scale)))
			});

		let lower = layer_map
			.layers()
			.filter(|surface| matches!(surface.layer(), Layer::Background | Layer::Bottom))
			.filter_map(move |surface| {
				layer_map
					.layer_geometry(surface)
					.map(|geo| (surface, geo.loc.to_physical_precise_round(output_scale)))
			});

		(lower, upper)
	}
}

impl Workspace {
	pub fn add_window(&mut self, window: MappedWindow) {
		self.space.map_element(window, (0, 0), true);
	}

	pub fn remove_window(&mut self, window: &MappedWindow) {
		self.space.unmap_elem(window);
	}

	pub fn floating_move<P: Into<Point<i32, Logical>>>(&mut self, window: MappedWindow, location: P) {
		self.space.map_element(window, location, true);
	}

	pub fn raise_window(&mut self, window: &MappedWindow, activate: bool) {
		self.space.raise_element(window, activate);
	}

	pub fn has_window(&mut self, window: &MappedWindow) -> bool {
		self.windows().any(|w| w == window)
	}

	pub fn windows(&self) -> impl DoubleEndedIterator<Item = &MappedWindow> {
		self.space.elements()
	}

	pub fn window_location(&self, window: &MappedWindow) -> Option<Point<i32, Logical>> {
		self.space.element_location(window)
	}

	pub fn window_geometry(&self, window: &MappedWindow) -> Option<Rectangle<i32, Logical>> {
		self.space.element_geometry(window)
	}

	pub fn window_under<P: Into<Point<f64, Logical>>>(
		&self,
		location: P,
	) -> Option<(&MappedWindow, Point<i32, Logical>)> {
		self.space.element_under(location)
	}
}
