//! # logging
//!
//! Engine logging built on [`tracing`].
//!
//! ## Usage
//!
//! **Game build** (no `editor` feature):
//! ```rust,ignore
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
//! ```rust,ignore
//! let logger = logging::Logger::builder()
//!     .verbose(true)
//!     .with_files("logs/")
//!     .init_editor()
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
//! ```rust,ignore
//! tracing::error!(target: "Physics", "Broad-phase overflow");
//! ```
//! Alternatively, set a `category` field (takes precedence over `target`):
//! ```rust,ignore
//! tracing::info!(category = "Audio", "Loaded {} sounds", n);
//! ```
//! If neither is set the Rust module path is used as a fallback.

mod entry;
mod layers;
mod query;
mod store;

pub use entry::{LogEntry, LogLevel};
pub use query::QueryBuilder;

use layers::{console::ConsoleLayer, file::FileLayer};
use thiserror::Error;
#[cfg(editor)]
use {layers::storage::StorageLayer, store::LogStore};

#[cfg(editor)]
use std::sync::Arc;

use tracing_subscriber::{
    Registry, filter::LevelFilter, layer::SubscriberExt, util::SubscriberInitExt,
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// LoggerBuilder
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Fluent builder for the engine logger. Obtain one via [`Logger::builder`].
#[derive(Default)]
pub struct LoggerBuilder {
    verbose: bool,
    log_dir: Option<String>,
}

impl LoggerBuilder {
    /// Enable `TRACE` / `DEBUG` levels (default: only `INFO` and above).
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Write log files into `log_dir` (created automatically if absent).
    /// If not set no files are written.
    pub fn with_files(mut self, log_dir: impl Into<String>) -> Self {
        self.log_dir = Some(log_dir.into());
        self
    }

    // ── Shared setup ──────────────────────────────────────────────────────────

    fn max_level(&self) -> LevelFilter {
        if self.verbose {
            LevelFilter::TRACE
        } else {
            LevelFilter::INFO
        }
    }

    fn build_file_layer(&self) -> Result<Option<FileLayer>, std::io::Error> {
        self.log_dir.as_deref().map(FileLayer::new).transpose()
    }

    /// Initialise the global tracing subscriber.
    ///
    /// Installs:
    /// - [`ConsoleLayer`] – colored terminal output
    /// - [`FileLayer`] – dual-file plain-text output (if [`with_files`](Self::with_files) was set)
    /// - [`StorageLayer`] – captures every event into an in-memory store
    pub fn init(self) -> Result<Logger, InitError> {
        let file_layer = self.build_file_layer()?;
        #[cfg(editor)]
        let store = Arc::new(LogStore::new());

        Registry::default()
            .with(self.max_level())
            .with(ConsoleLayer)
            .with(file_layer)
            // TODO: only add this layer in editor builds
            .with(StorageLayer::new(Arc::clone(&store)))
            .try_init()
            .map_err(|_| InitError::AlreadyInitialised)?;

        Ok(Logger {
            #[cfg(editor)]
            store,
        })
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Logger
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

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
    /// ```rust,ignore
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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Error types
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Error)]
pub enum InitError {
    /// `init` / `init_editor` was called more than once. The global subscriber
    /// can only be set once per process.
    #[error("logger has already been initialised")]
    AlreadyInitialised,
    /// A log-file directory could not be created, or a log file could not be
    /// opened.
    #[error("log-file I/O error: {0}")]
    Io(
        #[from]
        #[source]
        std::io::Error,
    ),
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

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
