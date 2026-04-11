//! # logging
//!
//! Engine logging built on [`tracing`].
//!
//! ## Usage
//!
//! **Game build** (no `editor` feature):
//! ```rust
//! logging::Logger::builder()
//!     .verbose(false)
//!     .with_files("logs/")
//!     .init()
//!     .expect("failed to init logger");
//!
//! tracing::info!(target: "Engine", "Engine started");
//! tracing::warn!(target: "Rendering", "Shader recompile triggered");
//! ```
//!
//! **Editor build** (`editor` feature enabled):
//! ```rust
//! let logger = logging::Logger::builder()
//!     .verbose(true)
//!     .with_files("logs/")
//!     .init()
//!     .expect("failed to init logger");
//!
//! // Later, in the log-panel UI:
//! let render_errors = logger
//!     .query()
//!     .of_category("Rendering")
//!     .min_level(logging::LogLevel::Warn)
//!     .last(50);
//! ```
//!
//! ## Category convention
//!
//! Use the `target:` tracing directive to assign a category:
//! ```rust
//! tracing::error!(target: "Physics", "Broad-phase overflow");
//! ```
//! Alternatively, set a `category` field (takes precedence over `target`):
//! ```rust
//! # let n = 5;
//! tracing::info!(category = "Audio", "Loaded {} sounds", n);
//! ```
//! If neither is set the Rust module path is used as a fallback.

mod builder;
mod layers;
mod query;
mod store;

pub use builder::LoggerBuilder;
pub use query::QueryBuilder;

use std::fmt;
use time::OffsetDateTime;

#[cfg(editor)]
use crate::store::LogStore;
#[cfg(editor)]
use std::sync::Arc;

/// Mirrors `tracing::Level` with an owned, filterable representation.
/// The discriminant ordering matches severity: `Error = 0` is most severe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

impl From<&tracing::Level> for LogLevel {
    fn from(level: &tracing::Level) -> Self {
        match *level {
            tracing::Level::ERROR => Self::Error,
            tracing::Level::WARN => Self::Warn,
            tracing::Level::INFO => Self::Info,
            tracing::Level::DEBUG => Self::Debug,
            tracing::Level::TRACE => Self::Trace,
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Error => "ERROR",
                Self::Warn => "WARN",
                Self::Info => "INFO",
                Self::Debug => "DEBUG",
                Self::Trace => "TRACE",
            }
        )
    }
}

/// A single captured log event with full metadata.
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Severity level.
    pub level: LogLevel,
    /// Category (from the `target:` directive or a `category` field; falls back
    /// to the tracing target, which defaults to the Rust module path).
    pub category: String,
    /// UTC timestamp at the moment the event was recorded.
    pub timestamp: OffsetDateTime,
    /// Formatted log message.
    pub message: String,
}

/// Handle returned after the global subscriber has been installed.
///
/// In **game** builds this is a zero-sized type.
/// In **editor** builds it holds an `Arc` to the shared [`LogStore`] and
/// exposes a rich query API.
pub struct Logger {
    /// Shared with the [`StorageLayer`]; only present in editor builds.
    #[cfg(editor)]
    store: Arc<LogStore>,
}

impl Logger {
    /// Create a [`LoggerBuilder`].
    pub fn builder() -> LoggerBuilder {
        LoggerBuilder::default()
    }

    // ── Query API (editor only) ───────────────────────────────────────────────

    /// Start building a query against the captured log entries.
    ///
    /// Only available with the `editor` feature. Panics at compile time in
    /// game builds (the method simply doesn't exist).
    ///
    /// # Example
    /// ```rust
    /// # use logging::LogLevel;
    /// # let logger = logging::Logger::builder().init().unwrap();
    /// let recent_errors = logger
    ///     .query()
    ///     .min_level(LogLevel::Error)
    ///     .within_last_seconds(60)
    ///     .execute();
    /// ```
    #[cfg(editor)]
    pub fn query(&self) -> QueryBuilder {
        QueryBuilder::new(Arc::clone(&self.store))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that the timestamp format string compiles without panicking.
    #[test]
    fn timestamp_format_compiles() {
        let ts = time::OffsetDateTime::now_utc();
        let s = layers::format::format_timestamp(&ts);
        assert!(!s.is_empty());
    }

    /// Verify the level-bracket format without colors.
    #[test]
    fn level_format_plain() {
        let s = layers::format::format_level(&tracing::Level::WARN, false);
        assert_eq!(s, "[WARN]");
    }

    /// Verify the level-bracket format with colors contains the level name.
    #[test]
    fn level_format_colored() {
        let s = layers::format::format_level(&tracing::Level::ERROR, true);
        assert!(s.contains("ERROR"));
        assert!(s.contains('\x1b'));
    }

    /// Verify the assembled log line structure.
    #[test]
    fn format_line_structure() {
        let line = layers::format::format_line(
            "2024/01/15 12:34:56",
            "[INFO]",
            "Rendering",
            "Mesh loaded",
        );
        assert_eq!(line, "2024/01/15 12:34:56 [INFO] [Rendering] Mesh loaded");
    }

    /// Smoke test: `LogLevel` ordering (`Error` < `Warn` < … < `Trace`).
    #[test]
    fn log_level_ordering() {
        assert!(LogLevel::Error < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Trace);
    }
}
