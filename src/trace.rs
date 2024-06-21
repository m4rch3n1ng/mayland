use chrono::Utc;
use std::{
	env,
	fs::{self, File},
};
use tracing_subscriber::{
	fmt,
	layer::{Filter, SubscriberExt},
	util::SubscriberInitExt,
	EnvFilter, Layer,
};

fn iso8601() -> String {
	let time = Utc::now();
	time.format("%Y-%m-%d %H-%M-%S").to_string()
}

#[cfg(not(feature = "debug"))]
const DEFAULT_LOG_FILTER: &str = "warn,mayland=debug,tracing_panic";
#[cfg(feature = "debug")]
const DEFAULT_LOG_FILTER: &str = "debug";

fn exclude_trace<F>() -> impl Filter<F> {
	let directives = std::env::var("RUST_LOG").unwrap_or_else(|_| DEFAULT_LOG_FILTER.to_owned());
	EnvFilter::builder().parse_lossy(directives)
}

#[cfg(feature = "trace")]
fn only_trace<F>() -> impl Filter<F> {
	tracing_subscriber::filter::filter_fn(|meta| meta.level() == &tracing::Level::TRACE)
}

pub fn setup() {
	let dir = env::current_dir().unwrap();
	let date = iso8601();

	let tmp = dir.join(".tmp");
	fs::create_dir_all(&tmp).unwrap();

	#[cfg(feature = "trace")]
	let trace_file = {
		let trace_file = tmp.join(date.clone() + ".trace.log");
		let trace_file = File::create(trace_file).unwrap();
		fmt::layer()
			.with_writer(trace_file)
			.with_ansi(false)
			.with_filter(only_trace())
	};

	let file = tmp.join(date + ".log");
	let file = File::create(file).unwrap();
	let file = fmt::layer()
		.with_writer(file)
		.with_ansi(false)
		.with_filter(exclude_trace());

	let stderr = fmt::layer()
		.with_writer(std::io::stderr)
		.with_filter(exclude_trace());

	let registry = tracing_subscriber::registry();
	let registry = registry.with(stderr);
	let registry = registry.with(file);

	#[cfg(feature = "trace")]
	let registry = registry.with(trace_file);

	registry.init();
}
