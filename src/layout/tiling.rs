use crate::{render::OutputRenderElements, shell::window::MappedWindow, utils::output_size};
use smithay::{
	backend::renderer::{element::AsRenderElements, glow::GlowRenderer},
	desktop::WindowSurface,
	output::Output,
	utils::{Logical, Point, Size},
};

#[derive(Debug)]
pub struct Tiling {
	size: Option<Size<i32, Logical>>,
	border: i32,
	window: Option<MappedWindow>,
}

impl Tiling {
	#[allow(clippy::new_without_default)]
	pub fn new() -> Self {
		Tiling {
			size: None,
			border: 25,
			window: None,
		}
	}
}

impl Tiling {
	fn window_size(&self, size: Size<i32, Logical>) -> Size<i32, Logical> {
		Size::from((
			size.w.saturating_sub(self.border * 2),
			size.h.saturating_sub(self.border * 2),
		))
	}

	fn window_location(&self, _window: &MappedWindow) -> Point<i32, Logical> {
		Point::from((self.border, self.border))
	}
}

impl Tiling {
	pub fn map_output(&mut self, output: &Output) {
		let output_size = output_size(output);
		self.size = Some(output_size);
	}

	pub fn unmap_output(&mut self) {
		self.size = None;
	}

	pub fn resize_output(&mut self, size: Size<i32, Logical>) {
		self.size = Some(size);

		if let Some(mapped) = &self.window {
			let window_size = self.window_size(size);
			tracing::debug!("tiling window resize {:?}", window_size);

			match mapped.window.underlying_surface() {
				WindowSurface::Wayland(xdg) => {
					xdg.with_pending_state(|state| {
						state.size = Some(window_size);
					});
					xdg.send_pending_configure();
				}
			}
		}
	}
}

impl Tiling {
	/// add [`MappedWindow`] if the tiling space isn't full, otherwise return it again
	pub fn add_window(&mut self, mapped: MappedWindow) -> Option<MappedWindow> {
		if self.window.is_none() {
			tracing::debug!("add tiling window");

			if let Some(size) = self.size {
				let window_size = self.window_size(size);
				tracing::debug!("tiling window size {:?}", window_size);

				match mapped.window.underlying_surface() {
					WindowSurface::Wayland(xdg) => {
						xdg.with_pending_state(|state| {
							state.size = Some(window_size);
						});
						xdg.send_pending_configure();
					}
				}
			}

			self.window = Some(mapped);
			None
		} else {
			Some(mapped)
		}
	}

	/// removes a [`MappedWindow`] from the tiling space if it exists
	///
	/// returns `true` if a window was removed, `false` otherwise
	pub fn remove_window(&mut self, window: &MappedWindow) -> bool {
		if self.window.as_ref().is_some_and(|current| current == window) {
			self.window = None;
			true
		} else {
			false
		}
	}

	pub fn has_window(&self, window: &MappedWindow) -> bool {
		self.window.as_ref().is_some_and(|current| current == window)
	}

	pub fn windows(&self) -> impl DoubleEndedIterator<Item = &MappedWindow> {
		self.window.iter()
	}

	pub fn window_under(
		&self,
		_location: Point<f64, Logical>,
	) -> Option<(&MappedWindow, Point<i32, Logical>)> {
		self.window
			.as_ref()
			.map(|window| (window, self.window_location(window)))
	}
}

impl Tiling {
	pub fn render(&self, renderer: &mut GlowRenderer, scale: f64) -> Vec<OutputRenderElements<GlowRenderer>> {
		if let Some(window) = &self.window {
			let location = self.window_location(window);
			window.render_elements(
				renderer,
				location.to_physical_precise_round(scale),
				scale.into(),
				1.,
			)
		} else {
			vec![]
		}
	}
}
