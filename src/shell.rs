use self::window::MappedWindow;
use crate::state::{ClientState, State};
use smithay::{
	backend::renderer::utils::{on_commit_buffer_handler, with_renderer_surface_state},
	delegate_compositor, delegate_shm,
	desktop::{layer_map_for_output, LayerSurface},
	output::Output,
	reexports::wayland_server::{
		protocol::{wl_buffer, wl_output::WlOutput, wl_surface::WlSurface},
		Client,
	},
	wayland::{
		buffer::BufferHandler,
		compositor::{
			get_parent, is_sync_subsurface, CompositorClientState, CompositorHandler, CompositorState,
		},
		shell::wlr_layer::{
			Layer, LayerSurface as WlrLayerSurface, WlrLayerShellHandler, WlrLayerShellState,
		},
		shm::{ShmHandler, ShmState},
	},
};
use std::collections::hash_map::Entry;

pub mod focus;
pub mod grab;
pub mod window;
pub mod xdg;

impl BufferHandler for State {
	fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl CompositorHandler for State {
	fn compositor_state(&mut self) -> &mut CompositorState {
		&mut self.mayland.compositor_state
	}

	fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
		if let Some(state) = client.get_data::<ClientState>() {
			return &state.compositor_state;
		}

		panic!("unknown client data type")
	}

	fn commit(&mut self, surface: &WlSurface) {
		on_commit_buffer_handler::<Self>(surface);

		if is_sync_subsurface(surface) {
			return;
		}

		let mut root = surface.clone();
		while let Some(parent) = get_parent(&root) {
			root = parent;
		}

		if surface == &root {
			if let Entry::Occupied(unmapped) = self.mayland.unmapped_windows.entry(surface.clone()) {
				let is_mapped =
					with_renderer_surface_state(surface, |state| state.buffer().is_some()).unwrap_or(false);

				if is_mapped {
					let unmapped = unmapped.remove();
					let mapped = MappedWindow::new(unmapped);

					mapped.window.on_commit();

					// add window to workspace
					let location = self.mayland.pointer.current_location();
					self.mayland.workspaces.add_window(mapped.clone(), location);

					// automatically focus new windows
					self.focus_window(mapped);

					return;
				}

				let window = unmapped.get();
				if let Some(toplevel) = window.toplevel() {
					if !xdg::initial_configure_sent(toplevel) {
						toplevel.send_configure();
					}
				}
			}

			if let Some(mapped) = self.mayland.workspaces.window_for_surface(surface) {
				mapped.window.on_commit();
				self.mayland.queue_redraw_all();
			}
		}

		self.mayland.handle_surface_commit(surface);

		if let Some(window) = self.mayland.workspaces.window_for_surface(surface) {
			self.handle_resize(window.clone());
		}

		self.mayland.queue_redraw_all();
	}
}

impl WlrLayerShellHandler for State {
	fn shell_state(&mut self) -> &mut WlrLayerShellState {
		&mut self.mayland.layer_shell_state
	}

	fn new_layer_surface(
		&mut self,
		surface: WlrLayerSurface,
		wl_output: Option<WlOutput>,
		_layer: Layer,
		namespace: String,
	) {
		let output = wl_output
			.as_ref()
			.and_then(Output::from_resource)
			.unwrap_or_else(|| self.mayland.workspaces.outputs().next().unwrap().clone());
		let mut map = layer_map_for_output(&output);
		map.map_layer(&LayerSurface::new(surface, namespace)).unwrap();
	}
}

impl ShmHandler for State {
	fn shm_state(&self) -> &ShmState {
		&self.mayland.shm_state
	}
}

delegate_compositor!(State);
delegate_shm!(State);
