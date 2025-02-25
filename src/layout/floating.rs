use crate::{
	render::{FocusRing, MaylandRenderElements},
	shell::window::MappedWindow,
};
use smithay::{
	backend::renderer::{element::AsRenderElements, glow::GlowRenderer},
	desktop::space::SpaceElement,
	utils::{Logical, Point, Rectangle},
};

#[derive(Debug)]
struct WindowLayout {
	window: MappedWindow,
	location: Point<i32, Logical>,
}

impl WindowLayout {
	fn geometry(&self) -> Rectangle<i32, Logical> {
		let mut geometry = self.window.geometry();
		geometry.loc = self.location;
		geometry
	}

	fn bbox(&self) -> Rectangle<i32, Logical> {
		let mut bbox = self.window.bbox();
		bbox.loc += self.window.render_location(self.location);
		bbox
	}
}

#[derive(Debug)]
pub struct Floating {
	windows: Vec<WindowLayout>,
}

impl Floating {
	pub fn new() -> Self {
		Floating { windows: Vec::new() }
	}

	pub fn map_window(&mut self, window: MappedWindow, location: Point<i32, Logical>) {
		self.activate(&window);
		if let Some(window) = self.windows.iter_mut().find(|w| w.window == window) {
			window.location = location;
		} else {
			window.set_activate(true);
			let window = WindowLayout { window, location };
			self.windows.push(window);
		}
	}

	pub fn remove_window(&mut self, window: &MappedWindow) {
		if let Some(idx) = self.windows.iter().position(|w| w.window == *window) {
			self.windows.remove(idx);
		}
	}

	pub fn raise_window(&mut self, window: &MappedWindow) {
		self.activate(window);
		if let Some(idx) = self.windows.iter().position(|w| w.window == *window) {
			self.windows[idx..].rotate_left(1);
		}
	}

	fn activate(&self, window: &MappedWindow) {
		for WindowLayout { window: w, .. } in &self.windows {
			if w == window {
				w.set_activate(true);
			} else {
				w.set_activate(false);
			}
		}
	}

	pub fn windows(&self) -> impl DoubleEndedIterator<Item = &MappedWindow> {
		self.windows.iter().map(|WindowLayout { window, .. }| window)
	}

	pub fn windows_geometry(
		&self,
	) -> impl DoubleEndedIterator<Item = (&MappedWindow, Rectangle<i32, Logical>)> {
		self.windows.iter().map(|w| (&w.window, w.geometry()))
	}

	pub fn window_location(&self, window: &MappedWindow) -> Option<Point<i32, Logical>> {
		self.windows
			.iter()
			.find(|w| w.window == *window)
			.map(|w| w.location)
	}

	pub fn window_under(&self, point: Point<f64, Logical>) -> Option<(&MappedWindow, Point<i32, Logical>)> {
		(self.windows.iter().rev())
			.filter(|w| w.bbox().to_f64().contains(point))
			.find_map(|WindowLayout { window, location }| {
				// we need to offset the point to the location where the surface is actually drawn
				let render_location = window.render_location(*location);
				let relative_location = point - render_location.to_f64();
				if window.is_in_input_region(&relative_location) {
					Some((window, render_location))
				} else {
					None
				}
			})
	}

	pub fn refresh(&self) {
		for WindowLayout { window, .. } in &self.windows {
			window.refresh();
		}
	}
}

impl Floating {
	pub fn render<'a, 'b>(
		&self,
		renderer: &'a mut GlowRenderer,
		scale: f64,
		decoration: &'b mayland_config::Decoration,
		focus: Option<&'b MappedWindow>,
	) -> impl Iterator<Item = MaylandRenderElements> + use<'_, 'a, 'b> {
		self.windows_geometry().rev().flat_map(move |(window, geom)| {
			let render_location = window.render_location(geom.loc).to_physical_precise_round(1);
			let mut elements = window.render_elements(renderer, render_location, scale.into(), 1.0);

			let color = if focus == Some(window) {
				decoration.focus.active
			} else {
				decoration.focus.inactive
			};

			let focus_ring = FocusRing::element(renderer, geom, color, decoration.focus.thickness);
			elements.push(MaylandRenderElements::FocusElement(focus_ring));

			elements
		})
	}
}
