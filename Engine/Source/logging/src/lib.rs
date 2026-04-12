//! # logging
//!
//! Engine logging built on [`tracing`], with structured targets, ANSI
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
//! let logger = logging::Logger::new().verbose(true).init().expect("logger init failed");
//! ```
//!
//! ## Emitting log events
//!
//! Use standard [`tracing`] macros. Assign a target via the `target:`
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
//! ## Querying the log store (editor builds only)
//!
//! In editor builds the [`Logger`] holds a shared [`store::LogStore`] that
//! captures every event. Use [`Logger::query`] with a [`Filter`] to search it:
//!
//! ```rust
//! # let logger = logging::Logger::new().init().unwrap();
//! use logging::{Filter, LogLevel};
//!
//! // The 50 most recent warnings or worse from the Rendering target:
//! let entries = logger
//!     .query(
//!         Filter::new()
//!             .of_target("Rendering")
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
//! ## Target convention
//!
//! Prefer short, stable target names that map to engine subsystems
//! (`"Rendering"`, `"Physics"`, `"Audio"`, …). Hierarchical names like
//! `"Rendering::Shadows"` work well with [`Filter::target_contains`].

mod filter;
mod layers;
mod store;
#[cfg(test)]
mod tests;

pub use filter::{LogFilter as Filter, StoreFilter};
use thiserror::Error;
use tracing_subscriber::{
    Registry,
    filter::{LevelFilter, filter_fn},
    layer::SubscriberExt,
    util::SubscriberInitExt,
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

impl From<LogLevel> for LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => Self::ERROR,
            LogLevel::Warn => Self::WARN,
            LogLevel::Info => Self::INFO,
            LogLevel::Debug => Self::DEBUG,
            LogLevel::Trace => Self::TRACE,
        }
    }
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
    /// Subsystem target (e.g. `"Rendering"`, `"Physics"`).
    ///
    /// Populated from (in priority order):
    /// 1. The `target:` directive.
    /// 2. The Rust module path (tracing's default target).
    pub target: String,
    /// UTC time at which the event was recorded.
    pub timestamp: OffsetDateTime,
    /// Fully formatted log message.
    pub message: String,
}

/// Handle returned after the global subscriber has been installed.
///
/// `Logger` follows a **builder pattern**: construct it with [`Logger::new`],
/// configure it with the chained setter methods, then call [`Logger::init`]
/// to install the global [`tracing`] subscriber.
///
/// ```rust
/// # use logging::{LogLevel, Logger};
/// # fn test() -> Result<(), Box<dyn std::error::Error>> {
/// let logger = Logger::new()
///     .max_level(LogLevel::Debug)
///     .write_fs(true)
///     .allowed_targets(["Rendering", "Physics"])
///     .init()?;
/// # Ok(()) };
/// ```
///
/// After `init` the returned `Logger` is the live handle. In **editor** builds
/// it exposes [`Logger::query`] for querying the in-memory log store.
/// In **game** builds it is a zero-sized type.
///
/// # Layer overview
///
/// | Layer | Always active | Condition |
/// |---|---|---|
/// | Console (ANSI, stderr/stdout) | ✓ | — |
/// | File (`latest.log` + timestamped) | | `write_fs(true)` |
/// | In-memory store | | `#[cfg(editor)]` |
///
/// # Errors
///
/// [`Logger::init`] fails if:
/// - The global subscriber has already been installed ([`InitError::AlreadyInitialised`]).
/// - A log file could not be created or opened ([`InitError::Io`]).
#[derive(Default)]
pub struct Logger {
    /// Shorthand for `max_level(LogLevel::Trace)`. Ignored when
    /// [`max_level`](Self::max_level) is also set.
    verbose: bool,
    /// Explicit ceiling on the log level. Overrides `verbose` when set.
    max_level: Option<LogLevel>,
    /// Whether to write logs to disk.
    write_fs: bool,
    /// Allowlist of target names. Empty means all targets pass.
    allowed_targets: Vec<String>,
    #[cfg(editor)]
    store: Arc<LogStore>,
}

impl Logger {
    /// Create a new logger builder with default settings:
    /// - `max_level`: `Info` (same as `verbose(false)`)
    /// - `write_fs`: `false`
    /// - all targets allowed
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable `TRACE`-level logging (equivalent to `max_level(LogLevel::Trace)`).
    ///
    /// Ignored if [`max_level`](Self::max_level) is also called; the explicit
    /// level always wins.
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Set the maximum [`LogLevel`] that will be processed.
    ///
    /// This takes precedence over [`verbose`](Self::verbose). Events less
    /// severe than `level` are dropped before reaching any layer.
    ///
    /// # Example
    /// ```rust
    /// # use logging::{Logger, LogLevel};
    /// # fn test() -> Result<(), Box<dyn std::error::Error>> {
    /// // Suppress debug and trace in a staging build:
    /// Logger::new().max_level(LogLevel::Info).init()?;
    /// # Ok(()) };
    /// ```
    pub fn max_level(mut self, level: LogLevel) -> Self {
        self.max_level = Some(level);
        self
    }

    /// Enable or disable writing log files to disk.
    ///
    /// When `true`, two files are written into the configured log directory:
    /// `latest.log` (truncated each run) and a timestamped archive.
    pub fn write_fs(mut self, write_fs: bool) -> Self {
        self.write_fs = write_fs;
        self
    }

    /// Restrict logging to entries whose target exactly matches one of the
    /// provided names. All other targets are silently dropped by every layer.
    ///
    /// Calling this with an empty iterator (or not calling it at all) allows
    /// all targets through.
    ///
    /// # Example
    /// ```rust
    /// # use logging::Logger;
    /// # fn test() -> Result<(), Box<dyn std::error::Error>> {
    /// Logger::new()
    ///     .allowed_targets(["Rendering", "Physics", "Audio"])
    ///     .init()?;
    /// # Ok(()) };
    /// ```
    pub fn allowed_targets(mut self, targets: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.allowed_targets = targets.into_iter().map(Into::into).collect();
        self
    }

    /// Install the global [`tracing`] subscriber and return the live `Logger`.
    ///
    /// Must be called exactly once per process. The builder is consumed and
    /// the configured `Logger` handle is returned on success.
    ///
    /// # Errors
    ///
    /// - [`InitError::AlreadyInitialised`] — called more than once.
    /// - [`InitError::Io`] — `write_fs` is `true` but a log file could not be
    ///   opened.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use logging::{LogLevel, Logger};
    /// # fn test() -> Result<(), Box<dyn std::error::Error>> {
    /// let logger = Logger::new()
    ///     .max_level(LogLevel::Warn)   // only Warn and Error
    ///     .write_fs(true)
    ///     .allowed_targets(["Rendering", "Physics"])
    ///     .init()?;
    /// # Ok(()) };
    ///
    /// tracing::info!(target: "Rendering", "this passes");
    /// tracing::info!(target: "Audio",     "this is dropped — wrong target");
    /// tracing::debug!(target: "Rendering","this is dropped — below Warn");
    /// ```
    pub fn init(self) -> Result<Self, InitError> {
        // max_level wins over the verbose shorthand.
        let max_level: LevelFilter = self.max_level.map(Into::into).unwrap_or_else(|| {
            if self.verbose {
                LevelFilter::TRACE
            } else {
                LevelFilter::INFO
            }
        });

        let targets_filter = if self.allowed_targets.is_empty() {
            None
        } else {
            let targets = self.allowed_targets.to_vec();
            Some(filter_fn(move |metadata| {
                targets.contains(&metadata.target().to_string())
            }))
        };

        let registry =
            Registry::default()
                .with(max_level)
                .with(ConsoleLayer)
                .with(if self.write_fs {
                    Some(FileLayer::new()?)
                } else {
                    None
                });

        #[cfg(editor)]
        let registry = registry.with(StorageLayer::new(Arc::clone(&self.store)));

        // always add the filter last
        let registry = registry.with(targets_filter);

        registry
            .try_init()
            .map_err(|_| InitError::AlreadyInitialised)?;

        Ok(self)
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
    /// # let logger = logging::Logger::new().init().unwrap();
    /// use logging::{Filter, LogLevel};
    ///
    /// // All errors across every target:
    /// let all_errors = logger
    ///     .query(Filter::new().of_level(LogLevel::Error))
    ///     .execute();
    ///
    /// // The 25 most recent Rendering warnings-or-worse from the last 60 s:
    /// let recent = logger
    ///     .query(
    ///         Filter::new()
    ///             .of_target("Rendering")
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
