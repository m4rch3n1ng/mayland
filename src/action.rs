use crate::{shell::focus::KeyboardFocusTarget, state::State};
use std::process::{Command, Stdio};
use tracing::error;

#[derive(Debug)]
pub enum Action {
	Quit,
	CloseWindow,

	Workspace(usize),

	Spawn(String),
}

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
				let mut cmd = Command::new("/bin/sh");
				cmd.arg("-c")
					.stdin(Stdio::null())
					.stdout(Stdio::null())
					.stderr(Stdio::null())
					.arg(&spawn)
					.env("WAYLAND_DISPLAY", &self.mayland.socket_name);

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
