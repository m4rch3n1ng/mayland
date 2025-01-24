use super::{Relocate, outputs::OutputSpace, tiling::Tiling};
use crate::{
	backend::udev::UdevOutputState,
	render::{FocusRing, MaylandRenderElements},
	shell::window::MappedWindow,
	utils::{RectExt, SizeExt, output_size},
};
use smithay::{
	backend::renderer::{
		element::{AsRenderElements, surface::WaylandSurfaceRenderElement},
		glow::GlowRenderer,
	},
	desktop::{LayerMap, LayerSurface, Space, layer_map_for_output, space::SpaceElement},
	output::Output,
	reexports::{drm::control::crtc, rustix::fs::Dev as dev_t},
	utils::{Logical, Physical, Point, Rectangle, Scale, Size},
	wayland::shell::wlr_layer::Layer,
};
use std::collections::{BTreeMap, HashMap};
use tracing::instrument;

#[derive(Debug)]
pub struct WorkspaceManager {
	/// output space
	outputs: OutputSpace,

	output_map: HashMap<Output, usize>,
	pub workspaces: BTreeMap<usize, Workspace>,

	/// layout config
	layout: mayland_config::layout::Layout,
	/// decoration config
	decoration: mayland_config::Decoration,
}

impl WorkspaceManager {
	pub fn new(config: &mayland_config::Config) -> Self {
		let outputs = OutputSpace::new();

		let output_map = HashMap::new();

		let workspace = Workspace::new(0, &config.layout, &config.decoration);
		let workspaces = BTreeMap::from([(0, workspace)]);

		WorkspaceManager {
			outputs,

			output_map,
			workspaces,

			decoration: config.decoration,
			layout: config.layout,
		}
	}

	pub fn reload_config(&mut self, config: &mayland_config::Config) {
		self.layout = config.layout;
		self.decoration = config.decoration;

		for workspace in self.workspaces.values_mut() {
			workspace.reload_config(&self.layout, &self.decoration);
		}
	}
}

impl WorkspaceManager {
	#[must_use = "you have to reposition the cursor"]
	pub fn switch_to_workspace(&mut self, idx: usize) -> Option<Point<i32, Logical>> {
		let Some(active_output) = &self.outputs.active else {
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
					self.outputs.active = Some(output.clone());

					let output_geometry = self.outputs.output_geometry(output).unwrap();
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
		if let Some(output) = &self.outputs.active {
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
		if let Some(output) = &self.outputs.active {
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
	pub fn add_output(&mut self, config: &mayland_config::Outputs, output: &Output) -> Option<Relocate> {
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

		self.outputs.add_output(config, output)
	}

	#[must_use = "you have to reposition the cursor"]
	pub fn remove_output(&mut self, config: &mayland_config::Outputs, output: &Output) -> Option<Relocate> {
		let idx = self.output_map.remove(output).unwrap();
		let workspace = self.workspaces.get_mut(&idx).unwrap();
		if workspace.is_empty() {
			self.workspaces.remove(&idx);
		}

		for workspace in self.workspaces.values_mut() {
			if workspace.output.as_ref() == Some(output) {
				workspace.remove_output(output);
			}
		}

		self.outputs.remove_output(config, output)
	}

	#[must_use = "you have to reposition the cursor"]
	pub fn reconfigure_outputs(&mut self, config: &mayland_config::Outputs) -> Option<Relocate> {
		self.outputs.reconfigure(config)
	}

	pub fn output_area_changed(&mut self, output: &Output) {
		for workspace in self.workspaces.values_mut() {
			if workspace.output.as_ref() == Some(output) {
				workspace.output_area_changed(output);
			}
		}
	}

	pub fn refresh(&mut self) {
		self.outputs.refresh();

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
		if let Some(output) = self.outputs.output_under(location) {
			if self.outputs.active.as_ref().is_none_or(|active| active != output) {
				self.outputs.active = Some(output.clone());
				return true;
			}
		}

		false
	}
}

impl WorkspaceManager {
	pub fn bbox(&self) -> Option<Rectangle<i32, Logical>> {
		self.outputs
			.outputs_geometry()
			.map(|(_, g)| g)
			.reduce(Rectangle::merge)
	}

	pub fn outputs(&self) -> impl Iterator<Item = &Output> {
		self.outputs.outputs()
	}

	pub fn output_position(&self, output: &Output) -> Option<Point<i32, Logical>> {
		self.outputs.output_position(output)
	}

	pub fn active_output(&self) -> Option<&Output> {
		self.outputs.active.as_ref()
	}

	/// get the [`Output`] associated with its [`UdevOutputState`].
	///
	/// # panics
	///
	/// calling this function on a backend other than the udev backend
	/// will cause it to panic
	pub fn udev_output(&self, device_id: dev_t, crtc: crtc::Handle) -> Option<&Output> {
		self.outputs().find(|output| {
			let udev_state = output.user_data().get::<UdevOutputState>().unwrap();
			udev_state.device_id == device_id && udev_state.crtc == crtc
		})
	}

	pub fn active_output_position(&self) -> Option<Point<i32, Logical>> {
		self.outputs.active_output_position()
	}

	pub fn output_under(&self, point: Point<f64, Logical>) -> Option<&Output> {
		self.outputs.output_under(point)
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
		if let Some(active) = &self.outputs.active {
			let output_position = self.outputs.output_position(active).unwrap();
			let pointer = pointer - output_position.to_f64();

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

	pub fn windows(&self) -> impl DoubleEndedIterator<Item = &MappedWindow> {
		self.workspaces.values().flat_map(|workspace| workspace.windows())
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
			let output_position = self.outputs.output_position(output).unwrap();
			let location = location - output_position.to_f64();

			let workspace = &self.output_map[output];
			let workspace = &self.workspaces[workspace];

			let (window, location) = workspace.window_under(location)?;
			Some((window, location + output_position))
		} else {
			None
		}
	}
}

impl WorkspaceManager {
	#[instrument(skip_all)]
	pub fn toggle_floating(&mut self, window: MappedWindow, pointer: Point<f64, Logical>) {
		if let Some(active) = &self.outputs.active {
			let workspace = self.output_map[active];
			let workspace = self.workspaces.get_mut(&workspace).unwrap();

			if !workspace.has_window(&window) {
				tracing::warn!("window was not on the active workspace?");
				return;
			}

			let output_geometry = self.outputs.output_geometry(active).unwrap();
			let pointer = pointer - output_geometry.loc.to_f64();

			workspace.toggle_floating(window, pointer);
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

	fn reload_config(&mut self, layout: &mayland_config::Layout, decoration: &mayland_config::Decoration) {
		self.tiling.reload_config(&layout.tiling, decoration);
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

	fn output_area_changed(&mut self, output: &Output) {
		self.tiling.output_area_changed(output);
	}

	fn refresh(&mut self) {
		self.floating.refresh();
	}

	/// get the [`Point`], that the window would have to be mapped to
	/// to be centered.
	///
	/// calculated by subtracting the relative center of the window by the
	/// relative center of the output.
	fn relative_center(&self, window_size: Size<i32, Logical>) -> Point<i32, Logical> {
		if let Some(output) = &self.output {
			let window_center = window_size.center();
			let output_center = output_size(output).center();
			output_center - window_center
		} else {
			Point::from((0, 0))
		}
	}

	pub fn is_empty(&self) -> bool {
		self.windows().count() == 0
	}
}

impl Workspace {
	pub fn add_window(&mut self, window: MappedWindow, pointer: Point<f64, Logical>) {
		let center = self.relative_center(window.geometry().size);

		if window.is_non_resizable() || window.windowrules.floating().unwrap_or(false) {
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

	pub fn windows_geometry(
		&self,
	) -> impl DoubleEndedIterator<Item = (&MappedWindow, Rectangle<i32, Logical>)> {
		self.floating
			.elements()
			.map(|window| (window, self.floating.element_geometry(window).unwrap()))
			.chain(self.tiling.windows_geometry())
	}

	pub fn window_geometry(&self, window: &MappedWindow) -> Option<Rectangle<i32, Logical>> {
		self.windows_geometry()
			.find(|(w, _)| w == &window)
			.map(|(_, geom)| geom)
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

impl Workspace {
	fn toggle_floating(&mut self, window: MappedWindow, pointer: Point<f64, Logical>) {
		if self.tiling.remove_window(&window) {
			let (min, max) = window.min_max_size();
			let output_size = self.output.as_ref().map(output_size).unwrap_or_default();

			let size = Size::from((
				(output_size.w * 3 / 4).clamp(min.w, max.w),
				(output_size.h * 3 / 4).clamp(min.h, max.h),
			));

			window.resize(size);

			let center = self.relative_center(size);
			self.floating.map_element(window, center, true);
		} else if !self.tiling.is_full() {
			debug_assert!(self.floating.element_location(&window).is_some());

			self.floating.unmap_elem(&window);
			self.tiling.add_window(window, pointer);
		}
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

			let color = if focus.as_ref() == Some(window) {
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
		impl Iterator<Item = LayerSurfacePoint<'_>>,
		impl Iterator<Item = LayerSurfacePoint<'_>>,
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
