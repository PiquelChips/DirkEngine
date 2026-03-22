use log::{Level, Metadata, Record};
use time::{self, OffsetDateTime};

pub struct Logger {
    enable: bool,
    max_level: Level,
    prefix: bool,
}

impl Logger {
    pub fn new(enable: bool, verbose: bool, prefix: bool) -> Box<Logger> {
        let max_level = if verbose { Level::Trace } else { Level::Info };
        Box::new(Logger {
            enable,
            max_level,
            prefix,
        })
    }
    pub fn max_level(&self) -> Level {
        self.max_level
    }
}

pub fn init(logger: Box<Logger>) {
    let max_level = logger.max_level().to_level_filter();
    log::set_boxed_logger(logger).unwrap(); // logger should not have already been set
    log::set_max_level(max_level);
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        if !self.enable {
            return false;
        }

        // TODO: add configuration for logging of external libraries
        if !metadata.target().starts_with("piquel") {
            return false;
        }

        metadata.level() <= self.max_level
    }
    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let now = OffsetDateTime::now_utc();
        let fmt = date_time_format();

        let timestamp = now.format(&fmt).unwrap();
        let log_level = format_log_level(record.level());

        let prefix = format!("{} {}", timestamp, log_level);

        let msg = if self.prefix {
            format!("{} {}", prefix, record.args())
        } else {
            record.args().to_string()
        };

        if record.level() == Level::Error {
            println!("{msg}");
        } else {
            eprintln!("{msg}");
        }
    }
    fn flush(&self) {}
}

fn format_log_level(level: Level) -> String {
    format!(
        "[{}]",
        make_colored_message(level.into(), &level.to_string())
    )
}

impl From<Level> for Color {
    fn from(level: Level) -> Self {
        match level {
            Level::Error => Color::Red,
            Level::Warn => Color::Yellow,
            Level::Info => Color::Green,
            Level::Debug => Color::Blue,
            Level::Trace => Color::Cyan,
        }
    }
}

enum Color {
    Cyan = 36,
    //Magenta = 35,
    Blue = 34,
    Yellow = 33,
    Green = 32,
    Red = 31,
}

fn make_colored_message(color: Color, message: &str) -> String {
    format!("\x1b[{}m{}\x1b[0m", color as usize, message)
}

fn date_time_format() -> Vec<time::format_description::BorrowedFormatItem<'static>> {
    time::format_description::parse("[year]/[month]/[day] [hour]:[minute]:[second]").unwrap()
}

#[cfg(test)]
mod tests {
    /// Making sure the unwrap doesn't crash
    #[test]
    fn date_time_format() {
        super::date_time_format();
    }
}
