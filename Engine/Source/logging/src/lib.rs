//! # logging
//!
//! Engine logging built on [`tracing`].
//!
//! ## Usage
//!
//! ```rust
//! // create verbose logger
//! let logger = logging::Logger::new(true).unwrap();
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

mod layers;
mod filter;
mod query;
mod store;
#[cfg(test)]
mod tests;

pub use query::QueryBuilder;
use thiserror::Error;
use tracing_subscriber::{
    Registry, filter::LevelFilter, layer::SubscriberExt, util::SubscriberInitExt,
};

use std::fmt;
use time::OffsetDateTime;

use crate::layers::{console::ConsoleLayer, file::FileLayer, storage::StorageLayer};
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
    pub fn new(verbose: bool) -> Result<Logger, InitError> {
        #[cfg(editor)]
        let store = Arc::new(LogStore::new());

        let max_level = if verbose {
            LevelFilter::TRACE
        } else {
            LevelFilter::INFO
        };

        Registry::default()
            .with(max_level)
            .with(ConsoleLayer)
            .with(FileLayer::new()?)
            // TODO: only add this layer in editor builds
            .with(StorageLayer::new(Arc::clone(&store)))
            .try_init()
            .map_err(|_| InitError::AlreadyInitialised)?;

        Ok(Logger {
            #[cfg(editor)]
            store,
        })
    }

    /// Start building a query against the captured log entries.
    ///
    /// Only available with the `editor` feature. Panics at compile time in
    /// game builds (the method simply doesn't exist).
    ///
    /// # Example
    /// ```rust
    /// # use logging::LogLevel;
    /// # let logger = logging::Logger::new(false).unwrap();
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
