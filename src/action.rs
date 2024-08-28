use crate::{shell::focus::KeyboardFocusTarget, state::State};
use mayland_config::Action;
use std::process::{Command, Stdio};
use tracing::error;

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

				// todo serious refactor
				if let Some(location) = location {
					self.mayland.queue_redraw_all();
					self.move_cursor(location.to_f64());
				}

				self.reset_keyboard_focus();
			}
			Action::Spawn(spawn) => {
				let [command, args @ ..] = &*spawn else {
					panic!("spawn commands cannot be empty");
				};

				let mut cmd = Command::new(command);
				cmd.args(args)
					.stdin(Stdio::null())
					.stdout(Stdio::null())
					.stderr(Stdio::null());

				std::thread::spawn(move || match cmd.spawn() {
					Ok(mut child) => {
						let _ = child.wait();
					}
					Err(err) => error!("error spawning child: {:?}", err),
				});
			}
		}
	}
}
