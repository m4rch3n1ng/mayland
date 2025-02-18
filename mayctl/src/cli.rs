use clap::{Parser, Subcommand, ValueEnum};
use mayland_comm::{Action, Request};

#[derive(Debug, Parser)]
#[clap(version, about)]
#[clap(disable_help_flag = true, disable_version_flag = true)]
#[clap(disable_help_subcommand = true)]
#[clap(propagate_version = true)]
pub struct Cli {
	#[command(subcommand)]
	pub cmd: Cmd,
	/// output in json format
	#[arg(short, long, global = true)]
	pub json: bool,

	/// print help
	#[arg(long, short, action = clap::ArgAction::Help, global = true)]
	help: Option<bool>,
	/// print version
	#[arg(long, short = 'V', action = clap::ArgAction::Version, global = true)]
	version: Option<bool>,
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
	/// issue a dispatch to the compositor
	Dispatch {
		#[command(subcommand)]
		dispatch: Dispatch,
	},
	/// reload compositor config
	Reload,
	/// request device info from the compositor
	Devices,
	/// request output info from the compositor
	Outputs,
	/// request window info from the compositor
	Windows,
	/// request workspace info from the compositor
	Workspaces,
}

#[derive(Debug, Subcommand)]
pub enum Dispatch {
	/// issue a dispatch to quit the compositor
	Quit,

	/// close active window
	#[clap(name = "close", visible_alias = "close-window")]
	CloseWindow,
	/// toggle floating status of active window
	ToggleFloating,
	/// cycle through windows
	Cycle {
		#[arg(value_enum)]
		direction: CycleDirection,
	},

	/// switch to another workspace
	Workspace { workspace: usize },

	/// spawn command
	Spawn {
		#[arg(required = true, trailing_var_arg = true)]
		spawn: Vec<String>,
	},
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CycleDirection {
	Next,
	Prev,
}

impl From<Cmd> for Request {
	fn from(value: Cmd) -> Self {
		match value {
			Cmd::Dispatch { dispatch: action } => Request::Dispatch(mayland_comm::Action::from(action)),
			Cmd::Reload => Request::Reload,
			Cmd::Devices => Request::Devices,
			Cmd::Outputs => Request::Outputs,
			Cmd::Windows => Request::Windows,
			Cmd::Workspaces => Request::Workspaces,
		}
	}
}

impl From<Dispatch> for Action {
	fn from(value: Dispatch) -> Self {
		match value {
			Dispatch::Quit => Action::Quit,

			Dispatch::CloseWindow => Action::CloseWindow,
			Dispatch::ToggleFloating => Action::ToggleFloating,
			Dispatch::Cycle { direction } => {
				Action::Cycle(mayland_comm::action::CycleDirection::from(direction))
			}

			Dispatch::Workspace { workspace } => Action::Workspace(workspace),

			Dispatch::Spawn { spawn } => Action::Spawn(spawn),
		}
	}
}

impl From<Action> for Dispatch {
	/// this implementation is not strictly necessary and should
	/// probably not be used, but it exists so that the compiler warns
	/// if the two enums are out of sync
	fn from(value: Action) -> Self {
		match value {
			Action::Quit => Dispatch::Quit,

			Action::CloseWindow => Dispatch::CloseWindow,
			Action::ToggleFloating => Dispatch::ToggleFloating,
			Action::Cycle(direction) => Dispatch::Cycle {
				direction: CycleDirection::from(direction),
			},

			Action::Workspace(workspace) => Dispatch::Workspace { workspace },

			Action::Spawn(spawn) => Dispatch::Spawn { spawn },
		}
	}
}

impl From<CycleDirection> for mayland_comm::action::CycleDirection {
	fn from(value: CycleDirection) -> Self {
		match value {
			CycleDirection::Next => mayland_comm::action::CycleDirection::Next,
			CycleDirection::Prev => mayland_comm::action::CycleDirection::Prev,
		}
	}
}

impl From<mayland_comm::action::CycleDirection> for CycleDirection {
	fn from(value: mayland_comm::action::CycleDirection) -> Self {
		match value {
			mayland_comm::action::CycleDirection::Next => CycleDirection::Next,
			mayland_comm::action::CycleDirection::Prev => CycleDirection::Prev,
		}
	}
}
