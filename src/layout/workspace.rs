use super::tiling::Tiling;
use crate::{
	render::{FocusRing, MaylandRenderElements},
	shell::window::MappedWindow,
	utils::{output_size, RectExt, SizeExt},
};
use smithay::{
	backend::renderer::{
		element::{surface::WaylandSurfaceRenderElement, AsRenderElements},
		glow::GlowRenderer,
	},
	desktop::{layer_map_for_output, space::SpaceElement, LayerMap, LayerSurface, Space},
	output::Output,
	utils::{Logical, Physical, Point, Rectangle, Scale},
	wayland::shell::wlr_layer::Layer,
};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug)]
pub struct WorkspaceManager {
	/// output space
	///
	/// only used to map the outputs to keep track
	/// of their position
	pub output_space: Space<MappedWindow>,

	pub active_output: Option<Output>,
	output_map: HashMap<Output, usize>,
	pub workspaces: BTreeMap<usize, Workspace>,

	decoration: mayland_config::Decoration,
}

impl WorkspaceManager {
	pub fn new(decoration: &mayland_config::Decoration) -> Self {
		let output_space = Space::default();

		let active_output = None;
		let output_map = HashMap::new();

		let workspace = Workspace::new(0, decoration);
		let workspaces = BTreeMap::from([(0, workspace)]);

		WorkspaceManager {
			output_space,

			active_output,
			output_map,
			workspaces,

			decoration: *decoration,
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

		if let Some(output) = self.output_map.iter().find(|&(_, &w)| w == idx).map(|(o, _)| o) {
			self.active_output = Some(output.clone());

			let rect = self.output_space.output_geometry(output).unwrap();
			let center = rect.center();
			Some(center)
		} else {
			if let Some(prev) = self.workspaces.get_mut(&current) {
				prev.unmap_output(active_output);
			}

			let workspace = self
				.workspaces
				.entry(idx)
				.or_insert_with(|| Workspace::new(idx, &self.decoration));
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
	#[must_use = "you have to reposition the cursor"]
	pub fn add_output(&mut self, output: &Output) -> Option<Point<i32, Logical>> {
		let x = self
			.output_space
			.outputs()
			.map(|output| self.output_space.output_geometry(output).unwrap())
			.map(|geom| geom.loc.x + geom.size.w)
			.max()
			.unwrap_or(0);

		self.output_space.map_output(output, (x, 0));

		let idx = (0..usize::MAX)
			.find(|n| self.output_map.values().all(|v| n != v))
			.expect("if you have more than usize::MAX monitors you deserve this");

		self.output_map.insert(output.clone(), idx);

		let workspace = self
			.workspaces
			.entry(idx)
			.or_insert_with(|| Workspace::new(idx, &self.decoration));
		workspace.map_output(output);

		if self.active_output.is_none() {
			self.active_output = Some(output.clone());

			let output_geometry = self.output_space.output_geometry(output).unwrap();
			Some(output_geometry.loc + output_geometry.size.center())
		} else {
			None
		}
	}

	pub fn remove_output(&mut self, output: &Output) {
		self.output_space.unmap_output(output);

		let idx = self.output_map.remove(output).unwrap();
		let workspace = self.workspaces.get_mut(&idx).unwrap();
		workspace.unmap_output(output);

		if self.active_output.as_ref() == Some(output) {
			self.active_output = self.output_map.keys().next().cloned();
		}
	}

	pub fn resize_output(&mut self, output: &Output) {
		let idx = &self.output_map[output];
		let workspace = self.workspaces.get_mut(idx).unwrap();
		workspace.resize_output(output);
	}

	pub fn refresh(&mut self) {
		self.output_space.refresh();

		let workspace = self.workspace_mut();
		workspace.refresh();
	}

	pub fn render_elements(
		&self,
		renderer: &mut GlowRenderer,
		output: &Output,
		focus: Option<MappedWindow>,
	) -> impl Iterator<Item = MaylandRenderElements> + use<> {
		let idx = &self.output_map[output];
		let workspace = &self.workspaces[idx];
		workspace.render_elements(renderer, output, &self.decoration, focus)
	}
}

impl WorkspaceManager {
	#[must_use = "you have to reset keyboard focus on active output change"]
	pub fn update_active_output(&mut self, location: Point<f64, Logical>) -> bool {
		if let Some(output) = self.output_space.output_under(location).next() {
			if self.active_output.as_ref().is_none_or(|active| active != output) {
				self.active_output = Some(output.clone());
				return true;
			}
		}

		false
	}

	/// is the [`MappedWindow`] in the floating space of the currently
	/// active [`Workspace`]?
	pub fn is_floating(&mut self, window: &MappedWindow) -> bool {
		let workspace = self.workspace();
		workspace.is_floating(window)
	}

	pub fn is_active_output(&self, output: &Output) -> bool {
		self.active_output.as_ref().is_some_and(|active| active == output)
	}
}

impl WorkspaceManager {
	pub fn outputs(&self) -> impl Iterator<Item = &Output> {
		self.output_space.outputs()
	}

	pub fn outputs_for_window(&self, window: &MappedWindow) -> Vec<Output> {
		let workspace = self.workspace();
		workspace.outputs_for_window(window)
	}

	pub fn output_geometry(&self, output: &Output) -> Option<Rectangle<i32, Logical>> {
		self.output_space.output_geometry(output)
	}

	pub fn output_under(&self, point: Point<f64, Logical>) -> impl Iterator<Item = &Output> {
		debug_assert!(self.output_space.output_under(point).count() <= 1);
		self.output_space.output_under(point)
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
		if let Some(active) = &self.active_output {
			let output_geo = self.output_space.output_geometry(active).unwrap();
			let pointer = pointer - output_geo.loc.to_f64();

			let workspace = self.workspace_mut();
			workspace.add_window(window, pointer);
		} else {
			let workspace = self.workspace_mut();
			workspace.add_window(window, pointer);
		}
	}

	pub fn floating_move(&mut self, window: MappedWindow, location: Point<i32, Logical>) {
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

	/// activate [`MappedWindow`] with [`MappedWindow::set_activate`],
	/// and raise it to the top if floating.
	pub fn activate_window(&mut self, window: &MappedWindow) {
		let workspace = self.workspace_mut();
		workspace.activate_window(window);
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
		if let Some(output) = self.output_under(location).next() {
			let output_geometry = self.output_space.output_geometry(output).unwrap();
			let location = location - output_geometry.loc.to_f64();

			let workspace = &self.output_map[output];
			let workspace = &self.workspaces[workspace];

			let (window, location) = workspace.window_under(location)?;
			Some((window, location + output_geometry.loc))
		} else {
			None
		}
	}
}

#[derive(Debug)]
pub struct Workspace {
	idx: usize,
	output: Option<Output>,

	tiling: Tiling,
	floating: Space<MappedWindow>,
}

impl Workspace {
	fn new(idx: usize, decoration: &mayland_config::Decoration) -> Self {
		let tiling = Tiling::new(decoration);
		let floating = Space::default();

		Workspace {
			idx,
			output: None,

			tiling,
			floating,
		}
	}
}

impl Workspace {
	fn map_output(&mut self, output: &Output) {
		self.output = Some(output.clone());

		self.tiling.map_output(output);
		self.floating.map_output(output, (0, 0));
	}

	fn unmap_output(&mut self, output: &Output) {
		self.output = None;
		self.floating.unmap_output(output);
	}

	fn resize_output(&mut self, output: &Output) {
		self.tiling.resize_output(output);
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
	pub fn add_window(&mut self, window: MappedWindow, pointer: Point<f64, Logical>) {
		let center = if let Some(output) = &self.output {
			let window_geom = window.geometry();
			let window_center = window_geom.size.center();

			let output_center = output_size(output).center();
			output_center - window_center
		} else {
			Point::from((0, 0))
		};

		if window.is_non_resizable() {
			self.floating.map_element(window, center, true);
		} else if let Some(window) = self.tiling.add_window(window, pointer) {
			self.floating.map_element(window, center, true);
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

	pub fn floating_move(&mut self, window: MappedWindow, location: Point<i32, Logical>) {
		if self.is_floating(&window) {
			self.floating.map_element(window, location, true);
		}
	}

	pub fn activate_window(&mut self, window: &MappedWindow) {
		if self.is_floating(window) {
			self.floating.raise_element(window, true);
		} else if self.tiling.has_window(window) {
			self.tiling.activate_window(window);
		}
	}

	pub fn has_window(&self, window: &MappedWindow) -> bool {
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

type LayerSurfacePoint<'a> = (&'a LayerSurface, Point<i32, Physical>);

impl Workspace {
	fn render_elements(
		&self,
		renderer: &mut GlowRenderer,
		output: &Output,
		decoration: &mayland_config::Decoration,
		focus: Option<MappedWindow>,
	) -> impl Iterator<Item = MaylandRenderElements> + use<> {
		let mut render_elements = Vec::new();

		let layer_map = layer_map_for_output(output);
		let output_scale = output.current_scale().fractional_scale();

		let (lower, upper) = Workspace::layer_elements(&layer_map, output_scale);

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

		for window in self.floating.elements().rev() {
			let geometry = self.floating.element_geometry(window).unwrap();

			let render_location = window.render_location(geometry.loc);

			let window_render_location = render_location.to_physical_precise_round(1.0);
			let elements = window.render_elements(renderer, window_render_location, 1.0.into(), 1.0);
			render_elements.extend(elements);

			let color = if focus.as_ref().is_some_and(|focus| focus == window) {
				decoration.focus.active
			} else {
				decoration.focus.inactive
			};

			let focus_ring =
				FocusRing::element(renderer, geometry, color.as_f32s(), decoration.focus.thickness);
			render_elements.push(MaylandRenderElements::FocusElement(focus_ring));
		}

		render_elements.extend(self.tiling.render(renderer, output_scale, decoration, focus));

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

	fn layer_elements(
		layer_map: &LayerMap,
		output_scale: f64,
	) -> (
		impl Iterator<Item = LayerSurfacePoint>,
		impl Iterator<Item = LayerSurfacePoint>,
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
	/// get info for [`mayland_comm::Workspace`]
	pub fn comm_info(&self, workspaces: &WorkspaceManager) -> mayland_comm::Workspace {
		let windows = self
			.windows()
			.map(mayland_comm::workspace::Window::from)
			.collect();
		let output = self.output.as_ref().map(|output| output.name());

		let active = match &self.output {
			Some(output) => workspaces.is_active_output(output),
			None => false,
		};

		mayland_comm::Workspace {
			idx: self.idx,
			output,

			active,
			windows,
		}
	}
}
