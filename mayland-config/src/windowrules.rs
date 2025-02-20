use regex::{Regex, RegexBuilder};
use serde::{Deserialize, de::Visitor};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct WindowRules(Vec<(Matcher, WindowRule)>);

impl<'de> Deserialize<'de> for WindowRules {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		deserializer.deserialize_map(WindowRulesVis)
	}
}

struct WindowRulesVis;

impl<'v> Visitor<'v> for WindowRulesVis {
	type Value = WindowRules;

	fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct WindowRule {
	// * rules applied at initial configure * //
	pub floating: Option<bool>,
	// * rules applied at render * //
	pub opacity: Option<f32>,
}

/// all values are parsed with [`mayfig`],
/// which does not support nan floats
impl Eq for WindowRule {}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Matcher {
	AppId(Match),
	Title(Match),
	Match(Match, Match),
}

impl Matcher {
	fn r#match(&self, app_id: Option<&str>, title: Option<&str>) -> bool {
		match self {
			Matcher::AppId(a) => app_id.is_some_and(|app_id| a.r#match(app_id)),
			Matcher::Title(t) => title.is_some_and(|title| t.r#match(title)),
			Matcher::Match(a, t) => {
				app_id.is_some_and(|app_id| a.r#match(app_id)) && title.is_some_and(|title| t.r#match(title))
			}
		}
	}
}

#[derive(Debug)]
pub enum Match {
	Regex {
		regex: Regex,
		inverted: bool,
		case_insensitive: bool,
	},
	Plain(String),
}

impl Match {
	fn r#match(&self, haystack: &str) -> bool {
		match self {
			Match::Regex { regex, inverted, .. } => regex.is_match(haystack) ^ inverted,
			Match::Plain(plain) => plain == haystack,
		}
	}
}

impl<'de> Deserialize<'de> for Match {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		deserializer.deserialize_str(MatchVis)
	}
}

struct MatchVis;

impl Visitor<'_> for MatchVis {
	type Value = Match;

	fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("a valid matcher")
	}

	fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
		if let Some(regex_opts) = parse_regex_windowrules(v) {
			let regex_opts = regex_opts.map_err(serde::de::Error::custom)?;

			// first check if the regex itself is valid, before wrapping
			// it in ^(?:)$, to disallow regexes like /)(/
			if let Err(err) = regex_syntax::parse(regex_opts.pattern) {
				return Err(serde::de::Error::custom(err));
			}

			// add an implicit `^(?:)$` around the regex, so you have a full-word match
			// by default, which is, i think, what you usually want, and makes the matching more
			// consistent with non-regex matching, which already is a full word match
			let regex = format!("^(:?{})$", regex_opts.pattern);
			let regex = RegexBuilder::new(&regex)
				.case_insensitive(regex_opts.case_insensitive)
				.build()
				.map_err(serde::de::Error::custom)?;

			Ok(Match::Regex {
				regex,
				inverted: regex_opts.inverted,
				case_insensitive: regex_opts.case_insensitive,
			})
		} else {
			let plain = Match::Plain(v.to_owned());
			Ok(plain)
		}
	}
}

#[derive(Debug, thiserror::Error)]
enum RegexError {
	#[error("duplicate regex flag {0:?}")]
	DuplicateFlag(char),
	#[error("unknown regex flag {0:?}")]
	UnknownFlag(char),
}

struct RegexOptions<'a> {
	/// regex body
	pattern: &'a str,
	// invert the regex match
	inverted: bool,
	/// make the regex match case-insensitive
	case_insensitive: bool,
}

fn parse_regex_windowrules(v: &str) -> Option<Result<RegexOptions<'_>, RegexError>> {
	let v = v.strip_prefix('/')?;
	let (pattern, flags) = v.rsplit_once('/')?;

	let opts = RegexOptions {
		pattern,
		inverted: false,
		case_insensitive: false,
	};

	let opts = flags.chars().try_fold(opts, |mut opts, f| match f {
		'v' if opts.inverted => Err(RegexError::DuplicateFlag('v')),
		'v' => {
			opts.inverted = true;
			Ok(opts)
		}
		'i' if opts.case_insensitive => Err(RegexError::DuplicateFlag('i')),
		'i' => {
			opts.case_insensitive = true;
			Ok(opts)
		}
		c => Err(RegexError::UnknownFlag(c)),
	});

	Some(opts)
}

impl PartialEq for Match {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(
				Match::Regex {
					regex: r1,
					inverted: v1,
					case_insensitive: i1,
				},
				Match::Regex {
					regex: r2,
					inverted: v2,
					case_insensitive: i2,
				},
			) => r1.as_str() == r2.as_str() && v1 == v2 && i1 == i2,
			(Match::Plain(p1), Match::Plain(p2)) => p1 == p2,
			_ => false,
		}
	}
}

impl Eq for Match {}
