//! # logging
//!
//! Engine logging built on [`tracing`], with structured categories, ANSI
//! console output, rotating file output, and (in editor builds) a queryable
//! in-memory store.
//!
//! ## Initialisation
//!
//! Call [`Logger::new`] once at startup. It installs the global
//! [`tracing`] subscriber, so calling it a second time returns
//! [`InitError::AlreadyInitialised`].
//!
//! ```rust
//! // Verbose mode enables DEBUG and TRACE levels; pass `false` for INFO+.
//! let logger = logging::Logger::new(true).expect("logger init failed");
//! ```
//!
//! ## Emitting log events
//!
//! Use standard [`tracing`] macros. Assign a category via the `target:`
//! directive (falls back to the Rust module path):
//!
//! ```rust
//! # let id = 5;
//! # let frame = 5;
//! tracing::error!(target: "Physics", "Broad-phase overflow");
//! tracing::warn!(target: "Audio",   "Buffer underrun on stream {}", id);
//! tracing::info!(target: "Rendering", "Frame {} complete", frame);
//! ```
//!
//! Alternatively use the explicit `category` field, which takes precedence
//! over `target`:
//!
//! ```rust
//! # let n = 5_usize;
//! tracing::info!(category = "Audio", "Loaded {} sounds", n);
//! ```
//!
//! ## Querying the log store (editor builds only)
//!
//! In editor builds the [`Logger`] holds a shared [`store::LogStore`] that
//! captures every event. Use [`Logger::query`] with a [`Filter`] to search it:
//!
//! ```rust
//! # let logger = logging::Logger::new(false).unwrap();
//! use logging::{Filter, LogLevel};
//!
//! // The 50 most recent warnings or worse from the Rendering category:
//! let entries = logger
//!     .query(
//!         Filter::new()
//!             .of_category("Rendering")
//!             .min_level(LogLevel::Warn),
//!     )
//!     .last(50);
//!
//! // Count all errors since the session started:
//! let error_count = logger
//!     .query(Filter::new().of_level(LogLevel::Error))
//!     .count();
//! ```
//!
//! ## Category convention
//!
//! Prefer short, stable category names that map to engine subsystems
//! (`"Rendering"`, `"Physics"`, `"Audio"`, …). Hierarchical names like
//! `"Rendering/Shadows"` work well with [`Filter::category_contains`].

mod filter;
mod layers;
mod store;
#[cfg(test)]
mod tests;

pub use filter::{LogFilter as Filter, StoreFilter};
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

/// Severity level of a log entry.
///
/// Variants are ordered by severity so that comparisons work intuitively:
/// `Error` is the **most** severe and `Trace` is the **least**.
///
/// ```rust
/// use logging::LogLevel;
///
/// assert!(LogLevel::Error < LogLevel::Warn);
/// assert!(LogLevel::Warn  < LogLevel::Info);
/// assert!(LogLevel::Info  < LogLevel::Debug);
/// assert!(LogLevel::Debug < LogLevel::Trace);
/// ```
///
/// This ordering is what powers [`Filter::min_level`]: passing
/// `LogLevel::Warn` keeps `Error` and `Warn` (both ≤ `Warn` in severity).
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

/// A single captured log event.
///
/// [`LogEntry`] values are stored in the [`store::LogStore`] (editor builds
/// only) and returned by [`StoreFilter`] query terminals.
///
/// All timestamps are in **UTC**.
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Severity of the event.
    pub level: LogLevel,
    /// Subsystem category (e.g. `"Rendering"`, `"Physics"`).
    ///
    /// Populated from (in priority order):
    /// 1. An explicit `category` field in the tracing macro call.
    /// 2. The `target:` directive.
    /// 3. The Rust module path (tracing's default target).
    pub category: String,
    /// UTC time at which the event was recorded.
    pub timestamp: OffsetDateTime,
    /// Fully formatted log message.
    pub message: String,
}

/// Handle returned after the global subscriber has been installed.
///
/// In **game** builds this is a zero-sized type; the file and console layers
/// are still active but no in-memory store is maintained.
///
/// In **editor** builds it holds an `Arc` to the shared [`store::LogStore`]
/// and exposes [`Logger::query`] for rich log-panel queries.
///
/// # Errors
///
/// [`Logger::new`] fails if:
/// - The global subscriber has already been installed ([`InitError::AlreadyInitialised`]).
/// - A log file could not be created or opened ([`InitError::Io`]).
pub struct Logger {
    #[cfg(editor)]
    store: Arc<LogStore>,
}

impl Logger {
    /// Initialise the global [`tracing`] subscriber and return a [`Logger`] handle.
    ///
    /// `verbose = true` enables `DEBUG` and `TRACE` levels in addition to
    /// `INFO`, `WARN`, and `ERROR`. Pass `false` for release / shipping builds.
    ///
    /// # Errors
    ///
    /// Returns [`InitError::AlreadyInitialised`] if called more than once in
    /// the same process. Returns [`InitError::Io`] if the log directory or
    /// files cannot be created.
    ///
    /// # Example
    ///
    /// ```rust
    /// let logger = logging::Logger::new(/* verbose = */ true)
    ///     .expect("failed to initialise logger");
    ///
    /// tracing::info!(target: "Engine", "Logger ready");
    /// ```
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

    /// Build a query against the in-memory log store.
    ///
    /// Returns a [`StoreFilter`] with terminal methods:
    /// - [`execute`](filter::StoreFilter::execute) — all matching entries,
    ///   oldest first.
    /// - [`last`](filter::StoreFilter::last) — the *n* most recent matching
    ///   entries (still in chronological order).
    /// - [`count`](filter::StoreFilter::count) — count without cloning entries.
    ///
    /// # Example
    ///
    /// ```rust
    /// # let logger = logging::Logger::new(false).unwrap();
    /// use logging::{Filter, LogLevel};
    ///
    /// // All errors across every category:
    /// let all_errors = logger
    ///     .query(Filter::new().of_level(LogLevel::Error))
    ///     .execute();
    ///
    /// // The 25 most recent Rendering warnings-or-worse from the last 60 s:
    /// let recent = logger
    ///     .query(
    ///         Filter::new()
    ///             .of_category("Rendering")
    ///             .min_level(LogLevel::Warn)
    ///             .within_last_seconds(60),
    ///     )
    ///     .last(25);
    ///
    /// // A badge count for the editor HUD:
    /// let n_errors = logger
    ///     .query(Filter::new().of_level(LogLevel::Error))
    ///     .count();
    /// ```
    #[cfg(editor)]
    pub fn query(&self, filter: Filter) -> StoreFilter {
        filter.with_store(Arc::clone(&self.store))
    }
}

/// Errors that can occur while initialising the [`Logger`].
#[derive(Debug, Error)]
pub enum InitError {
    /// [`Logger::new`] was called more than once. The global tracing
    /// subscriber can only be installed once per process.
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
