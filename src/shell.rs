use crate::state::{ClientState, MayState};
use smithay::{
	backend::renderer::utils::on_commit_buffer_handler,
	delegate_compositor, delegate_shm,
	reexports::wayland_server::{
		protocol::{wl_buffer, wl_surface::WlSurface},
		Client,
	},
	wayland::{
		buffer::BufferHandler,
		compositor::{
			get_parent, is_sync_subsurface, CompositorClientState, CompositorHandler,
			CompositorState,
		},
		shm::{ShmHandler, ShmState},
	},
};

use self::xdg::handle_commit;

pub mod xdg;

impl CompositorHandler for MayState {
	fn compositor_state(&mut self) -> &mut CompositorState {
		&mut self.compositor_state
	}

	fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
		if let Some(state) = client.get_data::<ClientState>() {
			return &state.compositor_state
		}

		panic!("unknown client data type")
	}

	fn commit(&mut self, surface: &WlSurface) {
		on_commit_buffer_handler::<Self>(surface);
		if !is_sync_subsurface(surface) {
			let mut root = surface.clone();
			while let Some(parent) = get_parent(&root) {
				root = parent;
			}
			if let Some(window) = self
				.space
				.elements()
				.find(|w| w.toplevel().wl_surface() == &root)
			{
				window.on_commit();
			}
		};

		handle_commit(&mut self.popups, &self.space, surface);
	}
}

impl BufferHandler for MayState {
	fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl ShmHandler for MayState {
	fn shm_state(&self) -> &ShmState {
		&self.shm_state
	}
}

delegate_compositor!(MayState);
delegate_shm!(MayState);

