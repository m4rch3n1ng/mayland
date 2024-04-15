use crate::{shell::focus::KeyboardFocusTarget, state::State};
use std::process::Command;

#[derive(Debug)]
pub enum Action {
	Quit,
	CloseWindow,

	Spawn(String),
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
			Action::Spawn(spawn) => {
				let mut cmd = Command::new("/bin/sh");
				cmd.arg("-c")
					.arg(&spawn)
					.env("WAYLAND_DISPLAY", &self.mayland.socket_name);

				std::thread::spawn(move || match cmd.spawn() {
					Ok(mut child) => {
						let _ = child.wait();
					}
					Err(err) => println!("error spawning child: {:?}", err),
				});
			}
		}
	}
}
