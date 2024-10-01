use crate::{shell::focus::KeyboardFocusTarget, state::State, utils::spawn};
use mayland_config::Action;

impl State {
	pub fn handle_action(&mut self, action: Action) {
		match action {
			Action::Quit => {
				self.mayland.loop_signal.stop();
				self.mayland.loop_signal.wakeup();
			}
			Action::CloseWindow => {
				let Some(focus) = self.mayland.keyboard.current_focus() else {
					return;
				};

				if let KeyboardFocusTarget::Window(window) = focus {
					window.close();
				}
			}
			Action::Workspace(idx) => {
				let location = self.mayland.workspaces.switch_to_workspace(idx);

				if let Some(location) = location {
					self.move_cursor(location.to_f64());
					self.mayland.queue_redraw_all();
				}

				self.reset_keyboard_focus();
			}
			Action::Spawn(command) => {
				spawn(command, &self.mayland);
			}
		}
	}
}
