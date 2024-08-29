use super::tiling::Tiling;
use crate::{
	shell::window::MappedWindow,
	state::MaylandRenderElements,
	utils::{output_size, RectExt},
	State,
};
use smithay::{
	backend::renderer::{
		element::{surface::WaylandSurfaceRenderElement, AsRenderElements},
		glow::GlowRenderer,
	},
	desktop::{layer_map_for_output, LayerMap, LayerSurface, Space},
	input::pointer::PointerHandle,
	output::Output,
	utils::{Logical, Physical, Point, Rectangle, Scale, Size},
	wayland::shell::wlr_layer::Layer,
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

		WorkspaceManager {
			space,
			active_output,
			output_map,
			workspaces,
		}
	}
}

impl WorkspaceManager {
	#[must_use = "you have to reposition the cursor"]
	pub fn switch_to_workspace(&mut self, idx: usize) -> Option<Point<i32, Logical>> {
		let Some(active_output) = &self.active_output else {
			return None;
		};

		let current = self.output_map[active_output];
		if idx == current {
			return None;
		}

		if let Some(output) = self.output_map.iter().find(|(_, &w)| w == idx).map(|(o, _)| o) {
			self.active_output = Some(output.clone());

			let rect = self.space.output_geometry(output).unwrap();
			let center = rect.center();
			Some(center)
		} else {
			if let Some(prev) = self.workspaces.get_mut(&current) {
				prev.unmap_output(active_output);
			}

			let workspace = self.workspaces.entry(idx).or_insert_with(Workspace::new);
			workspace.map_output(active_output);
			*self.output_map.get_mut(active_output).unwrap() = idx;

			None
		}
	}

	pub fn workspace(&self) -> &Workspace {
		if let Some(output) = &self.active_output {
			let idx = self.output_map[output];
			self.workspaces.get(&idx).unwrap()
		} else {
			self.workspaces.values().next().unwrap()
		}
	}

	pub fn workspace_mut(&mut self) -> &mut Workspace {
		if let Some(output) = &self.active_output {
			let idx = self.output_map[output];
			self.workspaces.get_mut(&idx).unwrap()
		} else {
			self.workspaces.values_mut().next().unwrap()
		}
	}
}

impl WorkspaceManager {
	pub fn add_output(&mut self, output: &Output) {
		let x = self
			.space
			.outputs()
			.map(|output| self.space.output_geometry(output).unwrap())
			.map(|geom| geom.loc.x + geom.size.w)
			.max()
			.unwrap_or(0);

		self.space.map_output(output, (x, 0));

		let idx = (0..usize::MAX)
			.find(|n| self.output_map.values().all(|v| n != v))
			.expect("if you have more than usize::MAX monitors you deserve this");

		self.output_map.insert(output.clone(), idx);

		if self.active_output.is_none() {
			self.active_output = Some(output.clone());
		}

		let workspace = self.workspaces.entry(idx).or_insert_with(Workspace::new);
		workspace.map_output(output);
	}

	pub fn remove_output(&mut self, output: &Output) {
		self.space.unmap_output(output);

		let idx = self.output_map.remove(output).unwrap();
		let workspace = self.workspaces.get_mut(&idx).unwrap();
		workspace.unmap_output(output);

		if self.active_output.as_ref() == Some(output) {
			self.active_output = self.output_map.keys().next().cloned();
		}
	}

	pub fn resize_output(&mut self, output: &Output) {
		let output_size = output_size(output);

		let idx = &self.output_map[output];
		let workspace = self.workspaces.get_mut(idx).unwrap();
		workspace.resize_output(output_size);
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
		let idx = &self.output_map[output];
		let workspace = &self.workspaces[idx];
		workspace.render_elements(renderer, output)
	}
}

impl WorkspaceManager {
	pub fn update_active_output(&mut self, location: Point<f64, Logical>) {
		if let Some(output) = self.space.output_under(location).next() {
			if self
				.active_output
				.as_ref()
				.map_or(true, |active| active != output)
			{
				self.active_output = Some(output.clone());
			}
		}
	}

	pub fn is_floating(&mut self, window: &MappedWindow) -> bool {
		let workspace = self.workspace();
		workspace.is_floating(window)
	}

	pub fn is_active_output(&self, output: &Output) -> bool {
		self.active_output.as_ref().is_some_and(|active| active == output)
	}

	pub fn relative_cursor_location(&mut self, pointer: &PointerHandle<State>) -> Point<f64, Physical> {
		let absolute_location = pointer.current_location();
		let location = if let Some(active) = &self.active_output {
			let geometry = self.space.output_geometry(active).unwrap();
			absolute_location - geometry.loc.to_f64()
		} else {
			absolute_location
		};

		location.to_physical(1.)
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
	pub fn window_for_surface<S>(&self, surface: &S) -> Option<&MappedWindow>
	where
		MappedWindow: PartialEq<S>,
	{
		self.workspaces
			.values()
			.flat_map(|w| w.windows())
			.find(|&w| w == surface)
	}
}

impl WorkspaceManager {
	pub fn add_window(&mut self, window: MappedWindow, pointer: Point<f64, Logical>) {
		let workspace = self.workspace_mut();
		workspace.add_window(window, pointer);
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

	pub fn window_under(
		&self,
		location: Point<f64, Logical>,
	) -> Option<(&MappedWindow, Point<i32, Logical>)> {
		if let Some(active) = &self.active_output {
			let output_geo = self.space.output_geometry(active).unwrap();
			let location = location - output_geo.loc.to_f64();

			let workspace = self.workspace();
			workspace.window_under(location)
		} else {
			let workspace = self.workspace();
			workspace.window_under(location)
		}
	}
}

#[derive(Debug)]
pub struct Workspace {
	tiling: Tiling,
	floating: Space<MappedWindow>,
}

impl Workspace {
	fn new() -> Self {
		let tiling = Tiling::new();
		let floating = Space::default();

		Workspace { tiling, floating }
	}
}

impl Workspace {
	fn map_output(&mut self, output: &Output) {
		self.tiling.map_output(output);
		self.floating.map_output(output, (0, 0));
	}

	fn unmap_output(&mut self, output: &Output) {
		self.tiling.unmap_output();
		self.floating.unmap_output(output);
	}

	fn resize_output(&mut self, size: Size<i32, Logical>) {
		self.tiling.resize_output(size);
	}

	fn outputs_for_window(&self, window: &MappedWindow) -> Vec<Output> {
		self.floating.outputs_for_element(window)
	}

	fn refresh(&mut self) {
		self.floating.refresh();
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
			.map(MaylandRenderElements::Surface)
		}));

		if let Some(output_geo) = self.floating.output_geometry(output) {
			render_elements.extend(
				self.floating
					.render_elements_for_region(renderer, &output_geo, output_scale, 1.)
					.into_iter()
					.map(MaylandRenderElements::Surface),
			);
		}

		render_elements.extend(self.tiling.render(renderer, output_scale));

		render_elements.extend(lower.flat_map(|(surface, location)| {
			AsRenderElements::<_>::render_elements::<WaylandSurfaceRenderElement<_>>(
				surface,
				renderer,
				location,
				Scale::from(output_scale),
				1.,
			)
			.into_iter()
			.map(MaylandRenderElements::Surface)
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
	pub fn add_window(&mut self, window: MappedWindow, pointer: Point<f64, Logical>) {
		if let Some(window) = self.tiling.add_window(window, pointer) {
			self.floating.map_element(window, (0, 0), true);
		}
	}

	pub fn remove_window(&mut self, window: &MappedWindow) {
		if !self.tiling.remove_window(window) {
			self.floating.unmap_elem(window);
		}
	}

	/// is the [`MappedWindow`] in the floating space?
	pub fn is_floating(&self, window: &MappedWindow) -> bool {
		self.floating.elements().any(|w| w == window)
	}

	pub fn floating_move<P: Into<Point<i32, Logical>>>(&mut self, window: MappedWindow, location: P) {
		if self.is_floating(&window) {
			self.floating.map_element(window, location, true);
		}
	}

	pub fn raise_window(&mut self, window: &MappedWindow, activate: bool) {
		if !self.tiling.has_window(window) {
			self.floating.raise_element(window, activate);
		}
	}

	pub fn has_window(&mut self, window: &MappedWindow) -> bool {
		self.windows().any(|w| w == window)
	}

	pub fn windows(&self) -> impl DoubleEndedIterator<Item = &MappedWindow> {
		self.floating.elements().chain(self.tiling.windows())
	}

	pub fn window_location(&self, window: &MappedWindow) -> Option<Point<i32, Logical>> {
		self.floating.element_location(window)
	}

	pub fn window_geometry(&self, window: &MappedWindow) -> Option<Rectangle<i32, Logical>> {
		self.floating.element_geometry(window)
	}

	pub fn window_under(
		&self,
		location: Point<f64, Logical>,
	) -> Option<(&MappedWindow, Point<i32, Logical>)> {
		self.floating
			.element_under(location)
			.or_else(|| self.tiling.window_under(location))
			.or_else(|| {
				self.floating.elements().next_back().map(|w| {
					let location = self.floating.element_location(w).unwrap();
					(w, w.render_location(location))
				})
			})
	}
}
