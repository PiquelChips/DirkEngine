use std::fmt;

use time::OffsetDateTime;
use tracing::field::{Field, Visit};

/// Visitor that extracts the `message` and optional `category` fields from a
/// tracing [`Event`](tracing::Event).
#[derive(Default)]
pub(crate) struct EventVisitor {
    pub message: String,
    pub category: Option<String>,
}

impl Visit for EventVisitor {
    /// Handles string fields set via `field = "value"` syntax.
    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "message" => self.message = value.to_owned(),
            "category" => self.category = Some(value.to_owned()),
            _ => {}
        }
    }

    /// Handles the implicit `message` argument from format macros
    /// (e.g. `tracing::info!("hello {}", name)`), which arrives as
    /// `fmt::Arguments` (implements both `Debug` and `Display`).
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            // fmt::Arguments' Debug impl delegates to Display, so no
            // surrounding quotes appear in the output.
            self.message = format!("{value:?}");
        }
    }
}

const TS_FORMAT: &[time::format_description::BorrowedFormatItem<'static>] =
    time::macros::format_description!("[year]/[month]/[day] [hour]:[minute]:[second]");

const TS_FILE_FORMAT: &[time::format_description::BorrowedFormatItem<'static>] =
    time::macros::format_description!("[year]-[month]-[day]_[hour]-[minute]-[second]");

/// Format a UTC timestamp as `YYYY/MM/DD HH:MM:SS`.
pub(crate) fn format_timestamp(ts: &OffsetDateTime) -> String {
    ts.format(TS_FORMAT).unwrap_or_else(|_| ts.to_string())
}

/// Format a UTC timestamp as `YYYY-MM-DD_HH-MM-SS` (safe for filenames).
pub(crate) fn format_timestamp_filename(ts: &OffsetDateTime) -> String {
    ts.format(TS_FILE_FORMAT)
        .unwrap_or_else(|_| "unknown".to_owned())
}

/// ANSI color codes for each log level.
fn level_color_code(level: &tracing::Level) -> u8 {
    match *level {
        tracing::Level::ERROR => 31, // Red
        tracing::Level::WARN => 33,  // Yellow
        tracing::Level::INFO => 32,  // Green
        tracing::Level::DEBUG => 34, // Blue
        tracing::Level::TRACE => 36, // Cyan
    }
}

fn ansi_wrap(code: u8, text: &str) -> String {
    format!("\x1b[{code}m{text}\x1b[0m")
}

/// Format the log level, optionally with ANSI color codes.
///
/// `colored = true`  → `\x1b[32mINFO\x1b[0m`
/// `colored = false` → `INFO`
pub(crate) fn format_level(level: &tracing::Level, colored: bool) -> String {
    let name = level.to_string(); // "ERROR" | "WARN" | "INFO" | "DEBUG" | "TRACE"
    if colored {
        format!("{}", ansi_wrap(level_color_code(level), &name))
    } else {
        format!("{name}")
    }
}

/// Assemble the final log line from its parts.
///
/// Format: `YYYY/MM/DD HH:MM:SS [LEVEL] [Category] message`
pub(crate) fn format_line(
    timestamp: &str,
    level_str: &str,
    category: &str,
    message: &str,
) -> String {
    format!("{timestamp} [{level_str}] [{category}] {message}")
}

/// Extract all relevant fields from a tracing event, returning
/// `(message, category, timestamp)` ready for use by any layer.
pub(crate) fn extract_event_data(event: &tracing::Event<'_>) -> (String, String, OffsetDateTime) {
    let mut visitor = EventVisitor::default();
    event.record(&mut visitor);

    // Category priority: explicit `category` field > tracing `target`
    // (target defaults to the Rust module path but can be set via
    //  `tracing::info!(target: "Rendering", "msg")`)
    let category = visitor
        .category
        .unwrap_or_else(|| event.metadata().target().to_owned());

    let timestamp = OffsetDateTime::now_utc();

    (visitor.message, category, timestamp)
}
