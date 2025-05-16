use bitflags::bitflags;
use serde::{Deserialize, Deserializer, de::Error as _, de::Visitor};
use smithay::input::keyboard::{
	Keysym, ModifiersState,
	keysyms::KEY_NoSymbol,
	xkb::{KEYSYM_CASE_INSENSITIVE, keysym_from_name},
};
use std::{collections::HashMap, fmt::Debug};

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Binds(HashMap<Mapping, Action>);

#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
	Quit,

	#[serde(alias = "close")]
	CloseWindow,
	ToggleFloating,
	Cycle(CycleDirection),

	Workspace(usize),

	Spawn(#[serde(deserialize_with = "deserialize_spawn")] Vec<String>),
}

fn deserialize_spawn<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
	let args = Vec::<String>::deserialize(deserializer)?;
	if args.is_empty() {
		Err(D::Error::invalid_length(0, &"at least one"))
	} else {
		Ok(args)
	}
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CycleDirection {
	Next,
	Prev,
}

impl From<Action> for mayland_comm::Action {
	/// this implementation is not strictly necessary and should
	/// probably not be used, but it exists so that the compiler warns
	/// if the two enums are out of sync
	fn from(action: Action) -> Self {
		match action {
			Action::Quit => mayland_comm::Action::Quit,

			Action::CloseWindow => mayland_comm::Action::CloseWindow,
			Action::ToggleFloating => mayland_comm::Action::ToggleFloating,
			Action::Cycle(direction) => mayland_comm::Action::Cycle(direction.into()),

			Action::Workspace(workspace) => mayland_comm::Action::Workspace(workspace),

			Action::Spawn(spawn) => mayland_comm::Action::Spawn(spawn),
		}
	}
}

impl From<mayland_comm::Action> for Action {
	fn from(action: mayland_comm::Action) -> Self {
		match action {
			mayland_comm::Action::Quit => Action::Quit,

			mayland_comm::Action::CloseWindow => Action::CloseWindow,
			mayland_comm::Action::ToggleFloating => Action::ToggleFloating,
			mayland_comm::Action::Cycle(direction) => Action::Cycle(direction.into()),

			mayland_comm::Action::Workspace(workspace) => Action::Workspace(workspace),

			mayland_comm::Action::Spawn(spawn) => Action::Spawn(spawn),
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

impl Default for Binds {
	fn default() -> Self {
		let mut binds = HashMap::new();

		// quit the compositor
		binds.insert(
			Mapping {
				mods: Modifiers::MOD,
				key: Keysym::Escape,
			},
			Action::Quit,
		);

		// close the active window
		binds.insert(
			Mapping {
				mods: Modifiers::MOD,
				key: Keysym::q,
			},
			Action::CloseWindow,
		);

		// toggle the active window between floating and tiling
		binds.insert(
			Mapping {
				mods: Modifiers::MOD,
				key: Keysym::v,
			},
			Action::ToggleFloating,
		);

		// cycle windows
		binds.insert(
			Mapping {
				mods: Modifiers::MOD,
				key: Keysym::Tab,
			},
			Action::Cycle(CycleDirection::Next),
		);
		binds.insert(
			Mapping {
				mods: Modifiers::MOD | Modifiers::SHIFT,
				key: Keysym::Tab,
			},
			Action::Cycle(CycleDirection::Prev),
		);

		// spawn kitty
		binds.insert(
			Mapping {
				mods: Modifiers::MOD,
				key: Keysym::t,
			},
			Action::Spawn(vec!["kitty".to_owned()]),
		);

		// spawn nautilus
		binds.insert(
			Mapping {
				mods: Modifiers::MOD,
				key: Keysym::e,
			},
			Action::Spawn(vec!["nautilus".to_owned()]),
		);

		// spawn firefox
		binds.insert(
			Mapping {
				mods: Modifiers::MOD,
				key: Keysym::n,
			},
			Action::Spawn(vec!["firefox".to_owned()]),
		);

		// spawn fuzzel
		binds.insert(
			Mapping {
				mods: Modifiers::MOD,
				key: Keysym::space,
			},
			Action::Spawn(vec!["fuzzel".to_owned()]),
		);

		// workspaces
		binds.insert(
			Mapping {
				mods: Modifiers::MOD,
				key: Keysym::_1,
			},
			Action::Workspace(0),
		);
		binds.insert(
			Mapping {
				mods: Modifiers::MOD,
				key: Keysym::_2,
			},
			Action::Workspace(1),
		);
		binds.insert(
			Mapping {
				mods: Modifiers::MOD,
				key: Keysym::_3,
			},
			Action::Workspace(2),
		);
		binds.insert(
			Mapping {
				mods: Modifiers::MOD,
				key: Keysym::_4,
			},
			Action::Workspace(3),
		);
		binds.insert(
			Mapping {
				mods: Modifiers::MOD,
				key: Keysym::_5,
			},
			Action::Workspace(4),
		);
		binds.insert(
			Mapping {
				mods: Modifiers::MOD,
				key: Keysym::_6,
			},
			Action::Workspace(5),
		);

		Binds(binds)
	}
}

impl Binds {
	pub fn find_action(&self, modifiers: &ModifiersState, keysym: Keysym) -> Option<Action> {
		let mapping = Mapping::from_xkb(modifiers, keysym);
		self.0.get(&mapping).cloned()
	}

	pub(crate) fn flatten_mod(mut self, comp: CompMod) -> Self {
		self.0 = self
			.0
			.into_iter()
			.map(|(key, val)| (key.flatten_mod(comp), val))
			.collect();

		self
	}
}

bitflags! {
	#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
	struct Modifiers: u8 {
		const META = 1 << 0;
		const ALT = 1 << 1;
		const CTRL = 1 << 2;
		const SHIFT = 1 << 3;
		const MOD = 1 << 4;
	}
}

impl Debug for Modifiers {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Debug::fmt(&self.0, f)
	}
}

impl Modifiers {
	fn from_xkb(modifiers_state: &ModifiersState) -> Modifiers {
		let mut modifiers = Modifiers::empty();

		if modifiers_state.ctrl {
			modifiers |= Modifiers::CTRL;
		}
		if modifiers_state.alt {
			modifiers |= Modifiers::ALT;
		}
		if modifiers_state.shift {
			modifiers |= Modifiers::SHIFT;
		}
		if modifiers_state.logo {
			modifiers |= Modifiers::META;
		}

		modifiers
	}

	fn add(&mut self, r#mod: &str) -> bool {
		let modifier = if r#mod.eq_ignore_ascii_case("ctrl") {
			Modifiers::CTRL
		} else if r#mod.eq_ignore_ascii_case("alt") {
			Modifiers::ALT
		} else if r#mod.eq_ignore_ascii_case("shift") {
			Modifiers::SHIFT
		} else if r#mod.eq_ignore_ascii_case("meta")
			|| r#mod.eq_ignore_ascii_case("logo")
			|| r#mod.eq_ignore_ascii_case("super")
		{
			Modifiers::META
		} else if r#mod.eq_ignore_ascii_case("mod") {
			Modifiers::MOD
		} else {
			return false;
		};

		*self |= modifier;
		true
	}
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Mapping {
	mods: Modifiers,
	key: Keysym,
}

impl Mapping {
	/// construct a [`Mapping`] from a [`ModifiersState`] and a [`Keysym`]
	fn from_xkb(modifiers: &ModifiersState, key: Keysym) -> Mapping {
		let mods = Modifiers::from_xkb(modifiers);
		Mapping { mods, key }
	}

	/// remove [`Modifiers::MOD`] from `self`
	fn flatten_mod(mut self, comp: CompMod) -> Self {
		if self.mods.contains(Modifiers::MOD) {
			self.mods = (self.mods - Modifiers::MOD) | comp.modifiers();
		}

		self
	}
}

impl<'de> Deserialize<'de> for Mapping {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		deserializer.deserialize_str(MappingVisitor)
	}
}

struct MappingVisitor;

impl Visitor<'_> for MappingVisitor {
	type Value = Mapping;

	fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("a valid key map")
	}

	fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
		let mut mods = Modifiers::empty();
		let mut key = None;

		for split in v
			.split(|c: char| c.is_whitespace() || c == '+')
			.filter(|word| !word.is_empty())
		{
			if !mods.add(split) {
				let keysym = keysym_from_name(split, KEYSYM_CASE_INSENSITIVE);

				if keysym.raw() == KEY_NoSymbol {
					return Err(serde::de::Error::custom(format_args!("invalid key {split:?}")));
				} else if key.is_some() {
					return Err(serde::de::Error::custom(format_args!(
						"duplicate key definition at {split:?}"
					)));
				}

				key = Some(keysym);
			}
		}

		let Some(key) = key else {
			return Err(serde::de::Error::custom("missing key"));
		};

		Ok(Mapping { mods, key })
	}
}

/// defines what the modifier `"mod"` binds to
///
/// set to [`CompMod::Alt`] in winit
/// and [`CompMod::Meta`] in udev mode
#[derive(Debug, Clone, Copy)]
pub enum CompMod {
	Alt,
	Meta,
}

impl CompMod {
	fn modifiers(self) -> Modifiers {
		match self {
			CompMod::Alt => Modifiers::ALT,
			CompMod::Meta => Modifiers::META,
		}
	}
}

impl PartialEq<CompMod> for ModifiersState {
	fn eq(&self, other: &CompMod) -> bool {
		Modifiers::from_xkb(self) == other.modifiers()
	}
}
