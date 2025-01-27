use serde::Deserialize;
use smithay::input::keyboard::XkbConfig;
use smithay::reexports::input::{AccelProfile, ClickMethod, ScrollMethod, TapButtonMap};

#[derive(Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Input {
	pub keyboard: Keyboard,
	pub touchpad: Touchpad,
	pub mouse: Mouse,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Keyboard {
	#[serde(deserialize_with = "deserialize_path")]
	pub xkb_file: Option<String>,

	pub xkb_rules: String,
	pub xkb_layout: String,
	pub xkb_model: String,
	pub xkb_variant: String,
	pub xkb_options: Option<String>,

	pub repeat_delay: i32,
	pub repeat_rate: i32,
}

impl Keyboard {
	pub fn xkb_config(&self) -> XkbConfig {
		XkbConfig {
			rules: &self.xkb_rules,
			model: &self.xkb_model,
			layout: &self.xkb_layout,
			variant: &self.xkb_variant,
			options: self.xkb_options.clone(),
		}
	}
}

impl Default for Keyboard {
	fn default() -> Self {
		Keyboard {
			xkb_file: None,

			xkb_rules: String::new(),
			xkb_model: String::new(),
			xkb_layout: String::new(),
			xkb_variant: String::new(),
			xkb_options: None,

			repeat_delay: 600,
			repeat_rate: 25,
		}
	}
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(default)]
pub struct Touchpad {
	pub tap: bool,
	pub tap_and_drag: bool,
	pub tap_drag_lock: bool,

	pub dwt: bool,
	pub dwtp: bool,

	pub natural_scroll: bool,
	#[serde(with = "scroll_method")]
	pub scroll_method: Option<ScrollMethod>,

	#[serde(with = "click_method")]
	pub click_method: Option<ClickMethod>,

	pub middle_emulation: bool,
	#[serde(with = "tap_button_map")]
	pub tap_button_map: Option<TapButtonMap>,
	pub left_handed: bool,

	pub accel_speed: f64,
	#[serde(with = "accel_profile")]
	pub accel_profile: Option<AccelProfile>,
}

/// all values are parsed with [`mayfig`],
/// which does not support nan floats
impl Eq for Touchpad {}

impl Default for Touchpad {
	fn default() -> Self {
		Touchpad {
			tap: true,
			tap_and_drag: false,
			tap_drag_lock: false,

			dwt: true,
			dwtp: true,

			natural_scroll: true,
			scroll_method: None,

			click_method: None,

			middle_emulation: true,
			tap_button_map: None,
			left_handed: false,

			accel_speed: 0.0,
			accel_profile: None,
		}
	}
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(default)]
pub struct Mouse {
	pub natural_scroll: bool,

	pub middle_emulation: bool,
	pub left_handed: bool,

	pub accel_speed: f64,
	#[serde(with = "accel_profile")]
	pub accel_profile: Option<AccelProfile>,
}

/// all values are parsed with [`mayfig`],
/// which does not support nan floats
impl Eq for Mouse {}

impl Default for Mouse {
	fn default() -> Self {
		Mouse {
			natural_scroll: false,

			middle_emulation: false,
			left_handed: false,

			accel_speed: 0.0,
			accel_profile: None,
		}
	}
}

fn deserialize_path<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
	let option = Option::<String>::deserialize(deserializer)?;
	let Some(mut path) = option else { return Ok(None) };

	let path = if let Some(rest) = path.strip_prefix("~") {
		let home = dirs::home_dir().unwrap();
		let home = home.into_os_string().into_string().unwrap();
		if rest.is_empty() {
			home
		} else {
			path.replace_range(0..=0, &home);
			path
		}
	} else {
		path
	};

	Ok(Some(path))
}

mod click_method {
	use serde::{Deserialize, Deserializer};
	use smithay::reexports::input as libinput;

	#[derive(Debug, Deserialize)]
	#[serde(rename_all = "snake_case")]
	enum ClickMethod {
		ButtonAreas,
		Clickfinger,
	}

	impl From<ClickMethod> for libinput::ClickMethod {
		fn from(value: ClickMethod) -> Self {
			match value {
				ClickMethod::ButtonAreas => libinput::ClickMethod::ButtonAreas,
				ClickMethod::Clickfinger => libinput::ClickMethod::Clickfinger,
			}
		}
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<libinput::ClickMethod>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let option = Option::<ClickMethod>::deserialize(deserializer)?;
		let option = option.map(libinput::ClickMethod::from);
		Ok(option)
	}
}

mod tap_button_map {
	use serde::{Deserialize, Deserializer};
	use smithay::reexports::input as libinput;

	#[derive(Debug, Deserialize)]
	#[serde(rename_all = "snake_case")]
	enum TapButtonMap {
		LeftRightMiddle,
		LeftMiddleRight,
	}

	impl From<TapButtonMap> for libinput::TapButtonMap {
		fn from(value: TapButtonMap) -> Self {
			match value {
				TapButtonMap::LeftRightMiddle => libinput::TapButtonMap::LeftRightMiddle,
				TapButtonMap::LeftMiddleRight => libinput::TapButtonMap::LeftMiddleRight,
			}
		}
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<libinput::TapButtonMap>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let option = Option::<TapButtonMap>::deserialize(deserializer)?;
		let option = option.map(libinput::TapButtonMap::from);
		Ok(option)
	}
}

mod scroll_method {
	use serde::{Deserialize, Deserializer};
	use smithay::reexports::input as libinput;

	#[derive(Debug, Deserialize)]
	#[serde(rename_all = "snake_case")]
	enum ScrollMethod {
		NoScroll,
		TwoFinger,
		Edge,
		OnButtonDown,
	}

	impl From<ScrollMethod> for libinput::ScrollMethod {
		fn from(value: ScrollMethod) -> Self {
			match value {
				ScrollMethod::NoScroll => libinput::ScrollMethod::NoScroll,
				ScrollMethod::TwoFinger => libinput::ScrollMethod::TwoFinger,
				ScrollMethod::Edge => libinput::ScrollMethod::Edge,
				ScrollMethod::OnButtonDown => libinput::ScrollMethod::OnButtonDown,
			}
		}
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<libinput::ScrollMethod>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let option = Option::<ScrollMethod>::deserialize(deserializer)?;
		let option = option.map(libinput::ScrollMethod::from);
		Ok(option)
	}
}

mod accel_profile {
	use serde::{Deserialize, Deserializer};
	use smithay::reexports::input as libinput;

	#[derive(Debug, Deserialize)]
	#[serde(rename_all = "snake_case")]
	enum AccelProfile {
		Adaptive,
		Flat,
	}

	impl From<AccelProfile> for libinput::AccelProfile {
		fn from(value: AccelProfile) -> Self {
			match value {
				AccelProfile::Adaptive => libinput::AccelProfile::Adaptive,
				AccelProfile::Flat => libinput::AccelProfile::Flat,
			}
		}
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<libinput::AccelProfile>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let option = Option::<AccelProfile>::deserialize(deserializer)?;
		let option = option.map(libinput::AccelProfile::from);
		Ok(option)
	}
}
