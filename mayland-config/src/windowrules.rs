use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct WindowRules(IndexMap<Matcher, WindowRule>);

impl WindowRules {
	pub fn compute(&self, app_id: Option<&str>, title: Option<&str>) -> WindowRule {
		self.0
			.iter()
			.filter_map(|(matcher, rule)| matcher.r#match(app_id, title).then_some(rule))
			.fold(WindowRule::default(), |acc, cur| WindowRule {
				floating: acc.floating.or(cur.floating),
			})
	}
}

#[derive(Debug, Deserialize, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(default)]
pub struct WindowRule {
	// * rules applied at initial configure * //
	pub floating: Option<bool>,
}
