use serde::Deserialize;
use smithay::input::keyboard::XkbConfig;
use smithay::reexports::input as libinput;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Input {
	pub keyboard: Keyboard,
	pub touchpad: Touchpad,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Keyboard {
	pub xkb_file: Option<String>,

	pub xkb_rules: String,
	pub xkb_model: String,
	pub xkb_layout: String,
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

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Touchpad {
	pub tap: bool,
	pub tap_and_drag: bool,
	pub tap_drag_lock: bool,

	pub dwt: bool,
	pub dwtp: bool,

	pub click_method: ClickMethod,

	pub middle_emulation: bool,
	pub tap_button_map: TapButtonMap,
	pub left_handed: bool,

	pub natural_scroll: bool,
	pub scroll_method: ScrollMethod,

	pub accel_speed: f64,
	pub accel_profile: AccelProfile,
}

impl Default for Touchpad {
	fn default() -> Self {
		Touchpad {
			tap: true,
			tap_and_drag: false,
			tap_drag_lock: false,

			dwt: true,
			dwtp: true,

			click_method: ClickMethod::Clickfinger,

			middle_emulation: true,
			tap_button_map: TapButtonMap::LeftRightMiddle,
			left_handed: false,

			natural_scroll: true,
			scroll_method: ScrollMethod::TwoFinger,

			accel_speed: 0.0,
			accel_profile: AccelProfile::Adaptive,
		}
	}
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClickMethod {
	ButtonAreas,
	#[default]
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

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TapButtonMap {
	#[default]
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

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollMethod {
	NoScroll,
	#[default]
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

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccelProfile {
	#[default]
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
