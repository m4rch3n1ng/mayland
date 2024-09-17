use clap::{Parser, Subcommand};
use mayland_comm::{Action, Request};

#[derive(Debug, Parser)]
#[clap(version, about)]
#[clap(disable_help_subcommand = true)]
#[clap(propagate_version = true)]
pub struct Cli {
	#[command(subcommand)]
	pub cmd: Cmd,
	/// output in json format
	#[arg(short, long, global = true)]
	pub json: bool,
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
	/// issue a dispatch to the compositor
	Dispatch {
		#[command(subcommand)]
		dispatch: Dispatch,
	},
	/// request workspace info from the compositor
	Workspaces,
}

#[derive(Debug, Subcommand)]
pub enum Dispatch {
	/// issue a dispatch to quit the compositor
	Quit,
	/// close active window
	CloseWindow,

	/// switch to another workspace
	Workspace { workspace: usize },

	/// spawn command
	Spawn {
		#[arg(required = true, trailing_var_arg = true)]
		spawn: Vec<String>,
	},
}

impl From<Cmd> for Request {
	fn from(value: Cmd) -> Self {
		match value {
			Cmd::Dispatch { dispatch: action } => Request::Dispatch(mayland_comm::Action::from(action)),
			Cmd::Workspaces => Request::Workspaces,
		}
	}
}

impl From<Dispatch> for Action {
	fn from(value: Dispatch) -> Self {
		match value {
			Dispatch::Quit => Action::Quit,
			Dispatch::CloseWindow => Action::CloseWindow,

			Dispatch::Workspace { workspace } => Action::Workspace(workspace),

			Dispatch::Spawn { spawn } => Action::Spawn(spawn),
		}
	}
}
