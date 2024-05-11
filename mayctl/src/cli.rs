use clap::{Parser, Subcommand};
use mayland::{action::Action, comm::Event};

#[derive(Debug, Subcommand)]
enum CliDispatch {
	#[command(about = "quit compositor")]
	Quit,
	#[command(name = "close", alias = "close-window", about = "close window")]
	CloseWindow,

	#[command(about = "switch workspace")]
	Workspace { workspace: usize },

	#[command(about = "spawn a command", alias = "exec")]
	Spawn { command: String },
}

impl From<CliDispatch> for Action {
	fn from(value: CliDispatch) -> Self {
		match value {
			CliDispatch::Quit => Action::Quit,
			CliDispatch::CloseWindow => Action::CloseWindow,

			CliDispatch::Workspace { workspace } => Action::Workspace(workspace),

			CliDispatch::Spawn { command } => Action::Spawn(command),
		}
	}
}

#[derive(Debug, Subcommand)]
enum CliEvent {
	#[command(about = "issue a dispatch to the compositor")]
	Dispatch {
		#[command(subcommand)]
		dispatch: CliDispatch,
	},
	#[command(about = "request information from the compositor")]
	Info,
}

#[derive(Debug, Parser)]
pub struct Cli {
	#[clap(subcommand, help = "dispatch events")]
	command: CliEvent,
}

impl From<Cli> for Event {
	fn from(value: Cli) -> Self {
		match value.command {
			CliEvent::Dispatch { dispatch } => {
				let dispatch = Action::from(dispatch);
				Event::Dispatch(dispatch)
			}
			CliEvent::Info => Event::Info,
		}
	}
}
