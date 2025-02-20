use crate::{State, state::Mayland};
use smithay::{
	desktop::{LayerSurface, WindowSurfaceType, layer_map_for_output},
	reexports::wayland_server::protocol::wl_surface::WlSurface,
	wayland::{
		compositor::with_states,
		shell::wlr_layer::{Layer, LayerSurfaceData},
	},
};

fn initial_configure_sent(surface: &LayerSurface) -> bool {
	let initial_configure_sent = with_states(surface.wl_surface(), |states| {
		states
			.data_map
			.get::<LayerSurfaceData>()
			.unwrap()
			.lock()
			.unwrap()
			.initial_configure_sent
	});

	initial_configure_sent
}

impl State {
	pub fn try_map_layer(&mut self, surface: &WlSurface) {
		if let Some((idx, (layer_surface, _))) = self
			.mayland
			.unmapped_layers
			.iter()
			.enumerate()
			.find(|(_, (l, _))| l.wl_surface() == surface)
		{
			if initial_configure_sent(layer_surface) {
				let (layer_surface, output) = self.mayland.unmapped_layers.remove(idx);

				let mut map = layer_map_for_output(&output);
				map.map_layer(&layer_surface).unwrap();
				map.arrange();
				drop(map);

				self.mayland.output_area_changed(&output);

				if layer_surface.can_receive_keyboard_focus()
					&& (layer_surface.layer() == Layer::Overlay || layer_surface.layer() == Layer::Top)
				{
					self.focus_layer_surface(layer_surface);
				}
			} else {
				layer_surface.layer_surface().send_configure();
			}
		}
	}
}

impl Mayland {
	pub fn handle_layer_surface_commit(&mut self, surface: &WlSurface) {
		let Some(output) = self
			.workspaces
			.outputs()
			.find(|output| {
				let layer_map = layer_map_for_output(output);
				let layer = layer_map.layer_for_surface(surface, WindowSurfaceType::TOPLEVEL);
				layer.is_some()
			})
			.cloned()
		else {
			return;
		};

		let mut layer_map = layer_map_for_output(&output);
		let has_changed = layer_map.arrange();
		drop(layer_map);

		if has_changed {
			self.output_area_changed(&output);
		}

		self.queue_redraw(output);
	}
}
