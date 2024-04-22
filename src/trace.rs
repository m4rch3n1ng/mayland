use std::{
	env,
	fs::{self, File},
};
use time::{macros::format_description, OffsetDateTime};
use tracing::Level;
use tracing_subscriber::{
	filter, fmt,
	layer::{Filter, SubscriberExt},
	util::SubscriberInitExt,
	Layer,
};

fn iso8601() -> String {
	let format = format_description!("[year]-[month]-[day] [hour]-[minute]-[second].log");
	let offset = OffsetDateTime::now_utc();
	offset.format(&format).unwrap()
}

fn trace_filter<F>() -> impl Filter<F> {
	filter::filter_fn(|meta| meta.level() != &Level::TRACE)
}

pub fn with_file() {
	let dir = env::current_dir().unwrap();
	let date = iso8601();

	let tmp = dir.join(".tmp");
	fs::create_dir_all(&tmp).unwrap();

	let file = tmp.join(date);
	let file = File::create(file).unwrap();
	let file = fmt::layer()
		.with_writer(file)
		.with_ansi(false)
		.with_filter(trace_filter());

	let stderr = fmt::layer()
		.with_writer(std::io::stderr)
		.with_filter(trace_filter());

	tracing_subscriber::registry()
		.with(stderr)
		.with(file)
		.init();
}

pub fn stderr() {
	let stderr = fmt::layer()
		.with_writer(std::io::stderr)
		.with_filter(trace_filter());

	tracing_subscriber::registry().with(stderr).init();
}
