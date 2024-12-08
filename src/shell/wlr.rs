use crate::state::Mayland;
use smithay::{
	desktop::{layer_map_for_output, WindowSurfaceType},
	reexports::wayland_server::protocol::wl_surface::WlSurface,
	wayland::{compositor::with_states, shell::wlr_layer::LayerSurfaceData},
};

impl Mayland {
	pub fn handle_layer_surface_commit(&mut self, surface: &WlSurface) {
		let Some(output) = self.workspaces.outputs().find(|output| {
			let layer_map = layer_map_for_output(output);
			let layer = layer_map.layer_for_surface(surface, WindowSurfaceType::TOPLEVEL);
			layer.is_some()
		}) else {
			return;
		};

		let initial_configure_sent = with_states(surface, |states| {
			states
				.data_map
				.get::<LayerSurfaceData>()
				.unwrap()
				.lock()
				.unwrap()
				.initial_configure_sent
		});

		let mut layer_map = layer_map_for_output(output);
		layer_map.arrange();

		if !initial_configure_sent {
			let layer = layer_map
				.layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
				.unwrap();
			layer.layer_surface().send_configure();
		}

		drop(layer_map);
		self.queue_redraw(output.clone());
	}
}
