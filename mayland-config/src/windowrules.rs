use serde::{de::Visitor, Deserialize};

#[derive(Debug, Default)]
pub struct WindowRules(Vec<(Matcher, WindowRule)>);

impl<'de> Deserialize<'de> for WindowRules {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		deserializer.deserialize_map(WindowRulesVis)
	}
}

struct WindowRulesVis;

impl<'v> Visitor<'v> for WindowRulesVis {
	type Value = WindowRules;

	fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.write_str("valid windowrules")
	}

	fn visit_map<A: serde::de::MapAccess<'v>>(self, mut map: A) -> Result<Self::Value, A::Error> {
		let mut windowrules = Vec::new();
		while let Some(entry) = map.next_entry()? {
			windowrules.push(entry);
		}

		let windowrules = WindowRules(windowrules);
		Ok(windowrules)
	}
}

impl WindowRules {
	pub fn compute(&self, app_id: Option<&str>, title: Option<&str>) -> WindowRule {
		self.0
			.iter()
			.rev()
			.filter_map(|(matcher, rule)| matcher.r#match(app_id, title).then_some(rule))
			.fold(WindowRule::default(), |acc, cur| WindowRule {
				floating: acc.floating.or(cur.floating),
				opacity: acc.opacity.or(cur.opacity),
			})
	}
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(default)]
pub struct WindowRule {
	// * rules applied at initial configure * //
	pub floating: Option<bool>,
	// * rules applied at render * //
	pub opacity: Option<f32>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Matcher {
	AppId(String),
	Title(String),
	Match(String, String),
}

impl Matcher {
	fn r#match(&self, app_id: Option<&str>, title: Option<&str>) -> bool {
		match self {
			Matcher::AppId(a) => app_id.is_some_and(|app_id| a == app_id),
			Matcher::Title(t) => title.is_some_and(|title| t == title),
			Matcher::Match(a, t) => {
				app_id.is_some_and(|app_id| a == app_id) && title.is_some_and(|title| t == title)
			}
		}
	}
}
