use crate::{shell::focus::KeyboardFocusTarget, state::State};

#[derive(Debug)]
pub enum Action {
	Quit,
	CloseWindow,
}

impl State {
	pub fn handle_action(&mut self, action: Action) {
		match action {
			Action::CloseWindow => {
				let Some(focus) = self.mayland.keyboard.current_focus() else {
					return;
				};

				if let KeyboardFocusTarget::Window(window) = focus {
					window.close();
				}
			}
			Action::Quit => {
				self.mayland.loop_signal.stop();
				self.mayland.loop_signal.wakeup();
			}
		}
	}
}
