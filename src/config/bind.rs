use crate::action::Action;
use serde::{de::Visitor, Deserialize};
use smithay::input::keyboard::{
	keysyms::KEY_NoSymbol,
	xkb::{keysym_from_name, KEYSYM_CASE_INSENSITIVE},
	Keysym, ModifiersState,
};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct Binds(HashMap<Mapping, Action>);

impl Default for Binds {
	fn default() -> Self {
		let mut binds = HashMap::new();

		// quit the compositor
		binds.insert(
			Mapping {
				mods: Modifiers::ALT,
				key: Keysym::Escape,
			},
			Action::Quit,
		);

		// close a window
		binds.insert(
			Mapping {
				mods: Modifiers::ALT,
				key: Keysym::q,
			},
			Action::CloseWindow,
		);

		// spawn kitty
		binds.insert(
			Mapping {
				mods: Modifiers::ALT,
				key: Keysym::t,
			},
			Action::Spawn("kitty".to_owned()),
		);

		// spawn nautilus
		binds.insert(
			Mapping {
				mods: Modifiers::ALT,
				key: Keysym::e,
			},
			Action::Spawn("nautilus".to_owned()),
		);

		// spawn firefox
		binds.insert(
			Mapping {
				mods: Modifiers::ALT,
				key: Keysym::n,
			},
			Action::Spawn("firefox".to_owned()),
		);

		// workspaces
		binds.insert(
			Mapping {
				mods: Modifiers::ALT,
				key: Keysym::_0,
			},
			Action::Workspace(0),
		);
		binds.insert(
			Mapping {
				mods: Modifiers::ALT,
				key: Keysym::_1,
			},
			Action::Workspace(1),
		);
		binds.insert(
			Mapping {
				mods: Modifiers::ALT,
				key: Keysym::_2,
			},
			Action::Workspace(2),
		);
		binds.insert(
			Mapping {
				mods: Modifiers::ALT,
				key: Keysym::_3,
			},
			Action::Workspace(3),
		);
		binds.insert(
			Mapping {
				mods: Modifiers::ALT,
				key: Keysym::_4,
			},
			Action::Workspace(4),
		);
		binds.insert(
			Mapping {
				mods: Modifiers::ALT,
				key: Keysym::_5,
			},
			Action::Workspace(5),
		);

		Binds(binds)
	}
}

impl Binds {
	pub fn find_action(&self, modifiers: &ModifiersState, keysym: Keysym) -> Option<Action> {
		let mapping = Mapping::from_xkb(modifiers, keysym)?;
		self.0.get(&mapping).cloned()
	}
}

#[derive(Debug, Default, PartialEq, Eq, Hash)]
struct Modifiers {
	pub ctrl: bool,
	pub alt: bool,
	pub shift: bool,
	pub meta: bool,
}

impl Modifiers {
	const ALT: Modifiers = Modifiers {
		ctrl: false,
		alt: true,
		shift: false,
		meta: false,
	};

	fn from_xkb(modifiers: &ModifiersState) -> Modifiers {
		Modifiers {
			ctrl: modifiers.ctrl,
			alt: modifiers.alt,
			shift: modifiers.shift,
			meta: modifiers.logo,
		}
	}

	fn add(&mut self, r#mod: &str) -> bool {
		if r#mod.eq_ignore_ascii_case("ctrl") {
			self.ctrl = true;
			true
		} else if r#mod.eq_ignore_ascii_case("alt") {
			self.alt = true;
			true
		} else if r#mod.eq_ignore_ascii_case("shift") {
			self.shift = true;
			true
		} else if r#mod.eq_ignore_ascii_case("meta")
			|| r#mod.eq_ignore_ascii_case("logo")
			|| r#mod.eq_ignore_ascii_case("super")
		{
			self.meta = true;
			true
		} else {
			false
		}
	}

	fn is_empty(&self) -> bool {
		!self.ctrl && !self.alt && !self.shift && !self.meta
	}
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Mapping {
	mods: Modifiers,
	key: Keysym,
}

impl Mapping {
	/// return an [`Option`] of a [`Mapping`]
	///
	/// returns None, if the Modifiers are empty
	fn from_xkb(modifiers: &ModifiersState, key: Keysym) -> Option<Mapping> {
		let mods = Modifiers::from_xkb(modifiers);
		if mods.is_empty() {
			return None;
		}

		Some(Mapping { mods, key })
	}
}

impl<'de> Deserialize<'de> for Mapping {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		deserializer.deserialize_str(MappingVisitor)
	}
}

struct MappingVisitor;

impl<'de> Visitor<'de> for MappingVisitor {
	type Value = Mapping;

	fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.write_str("a valid key map")
	}

	fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		let mut mods = Modifiers::default();
		let mut key = None;

		for split in v.split_whitespace() {
			if !mods.add(split) {
				let keysym = keysym_from_name(split, KEYSYM_CASE_INSENSITIVE);

				if keysym.raw() == KEY_NoSymbol {
					return Err(serde::de::Error::custom(format_args!("invalid key {:?}", split)));
				} else if key.is_some() {
					return Err(serde::de::Error::custom(format_args!(
						"duplicate key definition at {:?}",
						split
					)));
				}

				key = Some(keysym);
			}
		}

		let Some(key) = key else {
			return Err(serde::de::Error::custom("missing key"));
		};

		if mods.is_empty() {
			return Err(serde::de::Error::custom("missing modifier"));
		}

		Ok(Mapping { mods, key })
	}
}
