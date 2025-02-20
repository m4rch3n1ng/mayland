use chrono::Utc;
use std::fs::{self, File};
use tracing::Subscriber;
use tracing_journald as journald;
use tracing_subscriber::{
	EnvFilter, Layer, fmt,
	layer::{Filter, SubscriberExt},
	registry::LookupSpan,
	util::SubscriberInitExt,
};

fn iso8601() -> String {
	let time = Utc::now();
	time.format("%Y-%m-%dT%H-%M-%S mayland").to_string()
}

#[cfg(not(feature = "debug"))]
const DEFAULT_LOG_FILTER: &str = "warn,mayland=debug,tracing_panic";
#[cfg(feature = "debug")]
const DEFAULT_LOG_FILTER: &str = "debug";

fn default_filter<F>() -> impl Filter<F> {
	let directives = std::env::var("RUST_LOG").unwrap_or_else(|_| DEFAULT_LOG_FILTER.to_owned());
	EnvFilter::builder().parse_lossy(directives)
}

#[cfg(feature = "trace")]
fn only_trace<F>() -> impl Filter<F> {
	tracing_subscriber::filter::filter_fn(|meta| meta.level() == &tracing::Level::TRACE)
}

fn log_file(ext: &str) -> File {
	let local = dirs::data_dir().unwrap_or_else(std::env::temp_dir);

	let dir = local.join("mayland");
	fs::create_dir_all(&dir).unwrap();

	let date = iso8601();
	let path = dir.join(date + ext);

	// todo maybe handle
	File::create(path).unwrap()
}

fn with_file<F>() -> impl Layer<F>
where
	F: Subscriber + for<'span> LookupSpan<'span>,
{
	let file = log_file(".log");

	fmt::layer()
		.with_writer(file)
		.with_ansi(false)
		.with_filter(default_filter())
}

#[cfg(feature = "trace")]
fn with_trace_file<F>() -> impl Layer<F>
where
	F: Subscriber + for<'span> LookupSpan<'span>,
{
	let file = log_file(".trace.log");

	fmt::layer()
		.with_writer(file)
		.with_ansi(false)
		.with_filter(only_trace())
}

pub fn setup() {
	let registry = tracing_subscriber::registry();

	let stderr = fmt::layer()
		.with_writer(std::io::stderr)
		.with_filter(default_filter());
	let registry = registry.with(stderr);

	let file = with_file();
	let registry = registry.with(file);

	#[cfg(feature = "trace")]
	let trace_file = with_trace_file();
	#[cfg(feature = "trace")]
	let registry = registry.with(trace_file);

	match journald::layer() {
		Ok(journald) => {
			let journald = journald.with_filter(default_filter());
			let registry = registry.with(journald);

			registry.init();
		}
		Err(_) => registry.init(),
	}

	log_panics::init();
}
