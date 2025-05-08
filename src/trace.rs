use anstyle as style;
use env_filter::Filter;
use jiff::{Timestamp, Zoned, tz::TimeZone};
use log::Log;
use std::{fs::File, io::Write};
use systemd_journal_logger::JournalLog;

fn iso8601() -> String {
	let stamp = Timestamp::now();
	let zoned = Zoned::new(stamp, TimeZone::UTC);
	zoned.strftime("%Y-%m-%dT%H-%M-%S mayland.log").to_string()
}

fn log_file() -> Result<File, std::io::Error> {
	let dir = dirs::data_dir()
		.ok_or_else(|| std::io::Error::other("$HOME not set"))?
		.join("mayland");
	std::fs::create_dir_all(&dir)?;

	let date = iso8601();
	let path = dir.join(date);

	File::create(path)
}

struct Logger {
	filter: Filter,
	file: Option<File>,
	journald: Option<JournalLog>,
}

impl Logger {
	fn new(debug: bool) -> Logger {
		let directive = if debug {
			"debug"
		} else {
			"warn,mayland=debug,tracing_panic"
		};

		let filter = std::env::var("RUST_LOG");
		let filter = filter.as_deref().unwrap_or(directive);
		let filter = env_filter::Builder::new().parse(filter).build();

		let file = log_file()
			.inspect_err(|err| eprintln!("failed to open log file ({err})"))
			.ok();

		let journald = JournalLog::new()
			.map(|journal| journal.with_syslog_identifier("mayland".to_owned()))
			.ok();

		Logger {
			filter,
			file,
			journald,
		}
	}

	fn write<W: std::io::Write>(
		&self,
		mut w: W,
		color: bool,
		record: &log::Record<'_>,
		ts: jiff::Timestamp,
	) -> Result<(), std::io::Error> {
		let brace_style = if color {
			style::AnsiColor::BrightBlack.on_default()
		} else {
			style::Style::new()
		};

		write!(w, "{brace_style}[{brace_style:#}")?;
		write!(w, "{ts:.3}")?;

		if color {
			let style = match record.level() {
				log::Level::Trace => style::AnsiColor::Cyan.on_default(),
				log::Level::Debug => style::AnsiColor::Blue.on_default(),
				log::Level::Info => style::AnsiColor::Green.on_default(),
				log::Level::Warn => style::AnsiColor::Yellow.on_default(),
				log::Level::Error => style::AnsiColor::Red.on_default().effects(style::Effects::BOLD),
			};

			write!(w, " {style}{:<5}{style:#}", record.level())?;
		} else {
			write!(w, " {:<5}", record.level())?;
		}

		if record.target() != "" {
			write!(w, " {}", record.target())?;
		}

		write!(w, "{brace_style}]{brace_style:#}")?;
		writeln!(w, " {}", record.args())?;

		Ok(())
	}
}

impl Log for Logger {
	fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
		self.filter.enabled(metadata)
	}

	fn log(&self, record: &log::Record<'_>) {
		if !self.filter.matches(record) {
			return;
		}

		let timestamp = jiff::Timestamp::now();

		let stderr = std::io::stderr().lock();
		let _ = self.write(stderr, true, record, timestamp);

		if let Some(file) = &self.file {
			let _ = self.write(file, false, record, timestamp);
		}

		if let Some(journald) = &self.journald {
			journald.log(record);
		}
	}

	fn flush(&self) {
		if let Some(mut file) = self.file.as_ref() {
			let _ = file.flush();
		}
	}
}

pub fn setup(debug: bool) {
	let logger = Logger::new(debug);
	let max_level = logger.filter.filter();

	log::set_boxed_logger(Box::new(logger)).unwrap();
	log::set_max_level(max_level);

	log_panics::init();
}
