use super::tiling::Tiling;
use crate::{
	render::{FocusRing, MaylandRenderElements},
	shell::window::MappedWindow,
	utils::{output_size, RectExt, SizeExt},
};
use mayland_config::outputs::OutputInfo;
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
	output_space: Space<MappedWindow>,

	active_output: Option<Output>,
	output_map: HashMap<Output, usize>,
	pub workspaces: BTreeMap<usize, Workspace>,

	/// layout config
	layout: mayland_config::layout::Layout,
	/// decoration config
	decoration: mayland_config::Decoration,
}

impl WorkspaceManager {
	pub fn new(config: &mayland_config::Config) -> Self {
		let output_space = Space::default();

		let active_output = None;
		let output_map = HashMap::new();

		let workspace = Workspace::new(0, &config.layout, &config.decoration);
		let workspaces = BTreeMap::from([(0, workspace)]);

		WorkspaceManager {
			output_space,

			active_output,
			output_map,
			workspaces,

			decoration: config.decoration,
			layout: config.layout,
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

		if let Some(workspace) = self.workspaces.get_mut(&idx) {
			if let Some(output) = workspace.output.as_ref() {
				*self.output_map.get_mut(output).unwrap() = idx;

				if active_output != output {
					self.active_output = Some(output.clone());

					let output_geometry = self.output_space.output_geometry(output).unwrap();
					let output_center = output_geometry.center();
					Some(output_center)
				} else {
					let prev = &self.workspaces[&current];
					if prev.is_empty() {
						self.workspaces.remove(&current);
					}

					None
				}
			} else {
				workspace.map_output(active_output);
				*self.output_map.get_mut(active_output).unwrap() = idx;

				let prev = &self.workspaces[&current];
				if prev.is_empty() {
					self.workspaces.remove(&current);
				}

				None
			}
		} else {
			let prev = &self.workspaces[&current];
			if prev.is_empty() {
				self.workspaces.remove(&current);
			}

			let mut workspace = Workspace::new(idx, &self.layout, &self.decoration);
			workspace.map_output(active_output);

			self.workspaces.insert(idx, workspace);
			*self.output_map.get_mut(active_output).unwrap() = idx;

			None
		}
	}

	/// get the currently active workspace, if one exists
	pub fn workspace(&self) -> Option<&Workspace> {
		if let Some(output) = &self.active_output {
			let idx = self.output_map[output];
			let workspace = &self.workspaces[&idx];
			Some(workspace)
		} else {
			None
		}
	}

	/// get mutable access to the currently active workspace,
	/// if one exists
	pub fn workspace_mut(&mut self) -> Option<&mut Workspace> {
		if let Some(output) = &self.active_output {
			let idx = self.output_map[output];
			let workspace = self.workspaces.get_mut(&idx).unwrap();
			Some(workspace)
		} else {
			None
		}
	}
}

impl WorkspaceManager {
	#[must_use = "you have to reposition the cursor"]
	pub fn add_output(
		&mut self,
		config: &mayland_config::Outputs,
		output: &Output,
	) -> Option<Point<i32, Logical>> {
		let output_info = output.user_data().get::<OutputInfo>().unwrap();
		let output_config = config.get_output(output_info);
		let new_active_position = self.position_outputs(config, Some(output));

		if let Some(workspace) = self.workspaces.values_mut().find(|ws| ws.output.is_none()) {
			workspace.map_output(output);
			self.output_map.insert(output.clone(), workspace.idx);
		} else {
			let idx = (0..usize::MAX)
				.find(|n| !self.workspaces.contains_key(n))
				.expect("if you have more than usize::MAX monitors you deserve this");
			self.output_map.insert(output.clone(), idx);

			let mut workspace = Workspace::new(idx, &self.layout, &self.decoration);
			workspace.map_output(output);

			self.workspaces.insert(idx, workspace);
		}

		if self.active_output.is_none() || output_config.is_some_and(|conf| conf.active) {
			self.active_output = Some(output.clone());

			let output_geometry = self.output_space.output_geometry(output).unwrap();
			Some(output_geometry.loc + output_geometry.size.center())
		} else {
			new_active_position
		}
	}

	pub fn remove_output(&mut self, config: &mayland_config::Outputs, output: &Output) {
		let idx = self.output_map.remove(output).unwrap();
		self.output_space.unmap_output(output);
		self.position_outputs(config, None);

		let workspace = self.workspaces.get_mut(&idx).unwrap();
		if workspace.is_empty() {
			self.workspaces.remove(&idx);
		}

		for workspace in self.workspaces.values_mut() {
			if workspace.output.as_ref().is_some_and(|wo| wo == output) {
				workspace.remove_output(output);
			}
		}

		if self.active_output.as_ref() == Some(output) {
			self.active_output = self.output_map.keys().next().cloned();
		}
	}

	fn position_outputs(
		&mut self,
		config: &mayland_config::Outputs,
		output: Option<&Output>,
	) -> Option<Point<i32, Logical>> {
		let active_position = self
			.active_output
			.as_ref()
			.and_then(|output| self.output_space.output_geometry(output));

		let mut outputs = self
			.outputs()
			.chain(output)
			.map(|output| {
				let output_info = output.user_data().get::<OutputInfo>().unwrap();
				(output, config.get_output(output_info))
			})
			.map(|(output, config)| (output.clone(), config.and_then(|conf| conf.position)))
			.collect::<Vec<_>>();

		// first sort by output info
		outputs.sort_by(|(out1, _), (out2, _)| {
			let info1 = out1.user_data().get::<OutputInfo>().unwrap();
			let info2 = out2.user_data().get::<OutputInfo>().unwrap();

			info1.cmp(info2)
		});
		// then put the outputs with an explicit position first
		outputs.sort_by_key(|(_, position)| position.is_none());

		for (output, _) in &outputs {
			self.output_space.unmap_output(output);
		}

		for (output, config) in outputs {
			if let Some(config) = config {
				let point = Point::from((config[0], config[1]));

				let size = output_size(&output);
				let rect = Rectangle { loc: point, size };

				let overlaps = self
					.output_space
					.outputs()
					.map(|output| self.output_space.output_geometry(output).unwrap())
					.find(|geom| geom.overlaps(rect));

				if let Some(overlaps) = overlaps {
					panic!("output position {:?} overlaps with {:?}", rect, overlaps);
				}

				self.output_space.map_output(&output, point);
			} else {
				let x = self
					.output_space
					.outputs()
					.map(|output| self.output_space.output_geometry(output).unwrap())
					.map(|geom| geom.loc.x + geom.size.w)
					.max()
					.unwrap_or(0);

				let point = Point::from((x, 0));
				self.output_space.map_output(&output, point);
			}
		}

		if let Some(active_position) = active_position {
			let active_output = self.active_output.as_ref().unwrap();
			let new_active_position = self.output_space.output_geometry(active_output).unwrap();

			if active_position != new_active_position {
				Some(new_active_position.center())
			} else {
				None
			}
		} else {
			None
		}
	}

	pub fn resize_output(&mut self, output: &Output) {
		let idx = &self.output_map[output];
		let workspace = self.workspaces.get_mut(idx).unwrap();
		workspace.resize_output(output);
	}

	pub fn refresh(&mut self) {
		self.output_space.refresh();

		for workspace in self.workspaces.values_mut() {
			workspace.refresh();
		}
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
}

impl WorkspaceManager {
	pub fn outputs(&self) -> impl Iterator<Item = &Output> {
		self.output_space.outputs()
	}

	pub fn output_geometry(&self, output: &Output) -> Option<Rectangle<i32, Logical>> {
		self.output_space.output_geometry(output)
	}

	pub fn active_output(&self) -> Option<&Output> {
		self.active_output.as_ref()
	}

	pub fn active_output_geometry(&self) -> Option<Rectangle<i32, Logical>> {
		let active_output = self.active_output.as_ref()?;

		let geometry = self.output_space.output_geometry(active_output).unwrap();
		Some(geometry)
	}

	pub fn output_under(&self, point: Point<f64, Logical>) -> Option<&Output> {
		debug_assert!(self.output_space.output_under(point).count() <= 1);
		self.output_space.output_under(point).next()
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

			let workspace = self.output_map[active];
			let workspace = self.workspaces.get_mut(&workspace).unwrap();

			workspace.add_window(window, pointer);
		} else {
			let workspace = self.workspaces.values_mut().next().unwrap();
			workspace.add_window(window, pointer);
		}
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
		for workspace in self.workspaces.values_mut() {
			if workspace.has_window(window) {
				workspace.activate_window(window);
				return;
			}
		}
	}

	pub fn windows_for_output(&self, output: &Output) -> impl DoubleEndedIterator<Item = &MappedWindow> {
		let workspace = self.output_map[output];
		let workspace = &self.workspaces[&workspace];
		workspace.windows()
	}

	pub fn window_under(
		&self,
		location: Point<f64, Logical>,
	) -> Option<(&MappedWindow, Point<i32, Logical>)> {
		if let Some(output) = self.output_under(location) {
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
	fn new(idx: usize, layout: &mayland_config::Layout, decoration: &mayland_config::Decoration) -> Self {
		let tiling = Tiling::new(&layout.tiling, decoration);
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

	fn remove_output(&mut self, output: &Output) {
		debug_assert!(self.output.as_ref().is_some_and(|wo| wo == output));

		self.output = None;
		self.floating.unmap_output(output);
	}

	fn resize_output(&mut self, output: &Output) {
		self.tiling.resize_output(output);
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

		let active = match workspaces.workspace() {
			Some(workspace) => workspace.idx == self.idx,
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
