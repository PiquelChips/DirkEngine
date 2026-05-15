#[cfg(editor)]
use {crate::store::LogStore, std::sync::Arc};

use time::OffsetDateTime;

use crate::{LogEntry, LogLevel};

type Filter = Box<dyn Fn(&LogEntry) -> bool + Send + Sync>;

/// Fluent, composable filter builder for [`LogEntry`] values.
///
/// Predicates are accumulated with **AND** semantics: every predicate that has
/// been added must hold for an entry to pass. Filters are evaluated lazily
/// when [`filter`](Self::filter) (or a [`StoreFilter`] terminal) is called, so
/// building a `LogFilter` is always allocation-free beyond the closure storage.
///
/// # Quick start
///
/// ```rust
/// use dirk_logging::{Filter, LogLevel, LogEntry};
/// use time::OffsetDateTime;
///
/// let entry = LogEntry {
///     level:     LogLevel::Error,
///     target:  "Rendering".to_string(),
///     timestamp: OffsetDateTime::now_utc(),
///     message:   "GPU fence timeout".to_string(),
/// };
///
/// // A filter that passes only Rendering errors:
/// let passes = Filter::new()
///     .of_target("Rendering")
///     .min_level(LogLevel::Warn)   // Error < Warn, so Error passes
///     .filter(&entry);
///
/// assert!(passes);
/// ```
///
/// # Combining with a [`StoreFilter`]
///
/// In editor builds the filter can be applied to the live store via
/// [`Logger::query`](crate::Logger::query), which returns a [`StoreFilter`]
/// with terminal methods ([`execute`](StoreFilter::execute),
/// [`last`](StoreFilter::last), [`count`](StoreFilter::count)).
///
/// ```rust
/// # use dirk_logging::{Filter, LogLevel};
/// # let logger = dirk_logging::Logger::new().init().unwrap();
/// # #[cfg(editor)]
/// let recent_render_errors = logger
///     .query(
///         Filter::new()
///             .of_target("Rendering")
///             .min_level(LogLevel::Error)
///             .within_last_seconds(60),
///     )
///     .last(25);
/// ```
#[derive(Default)]
pub struct LogFilter {
    filters: Vec<Filter>,
}

impl LogFilter {
    /// Create a new, empty filter that matches every [`LogEntry`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Only include entries whose target **exactly** matches `target`.
    ///
    /// # Example
    /// ```rust
    /// # use dirk_logging::{Filter, LogLevel, LogEntry};
    /// # use time::OffsetDateTime;
    /// # fn make(cat: &str) -> LogEntry {
    /// #     LogEntry { level: LogLevel::Info, target: cat.to_string(),
    /// #                timestamp: OffsetDateTime::now_utc(), message: String::new() }
    /// # }
    /// let f = Filter::new().of_target("Audio");
    /// assert!( f.filter(&make("Audio")));
    /// assert!(!f.filter(&make("AudioManager"))); // substring — does not match
    /// assert!(!f.filter(&make("Rendering")));
    /// ```
    #[must_use]
    pub fn of_target(mut self, target: impl Into<String>) -> Self {
        let cat = target.into();
        self.filters.push(Box::new(move |e| e.target == cat));
        self
    }

    /// Only include entries whose target **contains** `substring`.
    ///
    /// Useful when targets follow a hierarchical naming scheme
    /// (e.g. `"Rendering/Shadows"`, `"Rendering/PostFX"`).
    ///
    /// # Example
    /// ```rust
    /// # use dirk_logging::{Filter, LogLevel, LogEntry};
    /// # use time::OffsetDateTime;
    /// # fn make(cat: &str) -> LogEntry {
    /// #     LogEntry { level: LogLevel::Info, target: cat.to_string(),
    /// #                timestamp: OffsetDateTime::now_utc(), message: String::new() }
    /// # }
    /// let f = Filter::new().target_contains("Render");
    /// assert!(f.filter(&make("Rendering")));
    /// assert!(f.filter(&make("Rendering/Shadows")));
    /// assert!(!f.filter(&make("Physics")));
    /// ```
    #[must_use]
    pub fn target_contains(mut self, substring: impl Into<String>) -> Self {
        let sub = substring.into();
        self.filters
            .push(Box::new(move |e| e.target.contains(sub.as_str())));
        self
    }

    /// Only include entries with **exactly** this level.
    ///
    /// Prefer [`min_level`](Self::min_level) when you want all entries at or
    /// above a given severity.
    ///
    /// # Example
    /// ```rust
    /// # use dirk_logging::{Filter, LogLevel, LogEntry};
    /// # use time::OffsetDateTime;
    /// # fn make(lvl: LogLevel) -> LogEntry {
    /// #     LogEntry { level: lvl, target: String::new(),
    /// #                timestamp: OffsetDateTime::now_utc(), message: String::new() }
    /// # }
    /// let f = Filter::new().of_level(LogLevel::Warn);
    /// assert!( f.filter(&make(LogLevel::Warn)));
    /// assert!(!f.filter(&make(LogLevel::Error))); // more severe — excluded
    /// assert!(!f.filter(&make(LogLevel::Info)));  // less severe — excluded
    /// ```
    #[must_use]
    pub fn of_level(mut self, level: LogLevel) -> Self {
        self.filters.push(Box::new(move |e| e.level == level));
        self
    }

    /// Only include entries at or **above** this severity.
    ///
    /// Severity ordering (most → least severe):
    /// `Error` > `Warn` > `Info` > `Debug` > `Trace`
    ///
    /// # Example
    /// ```rust
    /// # use dirk_logging::{Filter, LogLevel, LogEntry};
    /// # use time::OffsetDateTime;
    /// # fn make(lvl: LogLevel) -> LogEntry {
    /// #     LogEntry { level: lvl, target: String::new(),
    /// #                timestamp: OffsetDateTime::now_utc(), message: String::new() }
    /// # }
    /// let f = Filter::new().min_level(LogLevel::Warn);
    ///
    /// assert!( f.filter(&make(LogLevel::Error))); // more severe  ✓
    /// assert!( f.filter(&make(LogLevel::Warn)));  // exact match  ✓
    /// assert!(!f.filter(&make(LogLevel::Info)));  // less severe  ✗
    /// assert!(!f.filter(&make(LogLevel::Debug))); //              ✗
    /// assert!(!f.filter(&make(LogLevel::Trace))); //              ✗
    /// ```
    #[must_use]
    pub fn min_level(mut self, level: LogLevel) -> Self {
        self.filters.push(Box::new(move |e| e.level <= level));
        self
    }

    /// Only include entries recorded **at or after** `time`.
    ///
    /// # Example
    /// ```rust
    /// # use dirk_logging::{Filter, LogLevel, LogEntry};
    /// # use time::OffsetDateTime;
    /// let cutoff = OffsetDateTime::now_utc();
    /// let f = Filter::new().since(cutoff);
    ///
    /// // An entry stamped before the cutoff is excluded.
    /// let old = LogEntry {
    ///     level:     LogLevel::Info,
    ///     target:  String::new(),
    ///     timestamp: cutoff - time::Duration::seconds(1),
    ///     message:   String::new(),
    /// };
    /// assert!(!f.filter(&old));
    /// ```
    #[must_use]
    pub fn since(mut self, time: OffsetDateTime) -> Self {
        self.filters.push(Box::new(move |e| e.timestamp >= time));
        self
    }

    /// Only include entries recorded **at or before** `time`.
    ///
    /// Combine with [`since`](Self::since) to create a time window.
    ///
    /// # Example
    /// ```rust
    /// # use dirk_logging::{Filter, LogLevel, LogEntry};
    /// # use time::OffsetDateTime;
    /// let now = OffsetDateTime::now_utc();
    /// // Entries from the last 10 seconds only:
    /// let f = Filter::new()
    ///     .since(now - time::Duration::seconds(10))
    ///     .until(now);
    /// ```
    #[must_use]
    pub fn until(mut self, time: OffsetDateTime) -> Self {
        self.filters.push(Box::new(move |e| e.timestamp <= time));
        self
    }

    /// Only include entries recorded within the **last `seconds` seconds**.
    ///
    /// The cutoff is fixed at the moment this method is called, so the window
    /// does not slide as time passes.
    ///
    /// # Example
    /// ```rust
    /// # use dirk_logging::{Filter, LogLevel, LogEntry};
    /// # use time::OffsetDateTime;
    /// // Keep only entries from the last minute:
    /// let f = Filter::new()
    ///     .of_target("Physics")
    ///     .within_last_seconds(60);
    /// ```
    #[must_use]
    pub fn within_last_seconds(self, seconds: i64) -> Self {
        let cutoff = OffsetDateTime::now_utc() - time::Duration::seconds(seconds);
        self.since(cutoff)
    }

    /// Only include entries whose message contains `pattern` (case-sensitive).
    ///
    /// # Example
    /// ```rust
    /// # use dirk_logging::{Filter, LogLevel, LogEntry};
    /// # use time::OffsetDateTime;
    /// # fn make(msg: &str) -> LogEntry {
    /// #     LogEntry { level: LogLevel::Error, target: String::new(),
    /// #                timestamp: OffsetDateTime::now_utc(), message: msg.to_string() }
    /// # }
    /// let f = Filter::new().matching("overflow");
    ///
    /// assert!( f.filter(&make("Broad-phase overflow detected")));
    /// assert!(!f.filter(&make("Broad-phase Overflow detected"))); // case-sensitive
    /// assert!(!f.filter(&make("All contacts resolved")));
    /// ```
    #[must_use]
    pub fn matching(mut self, pattern: impl Into<String>) -> Self {
        let pat = pattern.into();
        self.filters
            .push(Box::new(move |e| e.message.contains(pat.as_str())));
        self
    }

    /// Test `entry` against all accumulated predicates.
    ///
    /// Returns `true` only when **all** predicates pass (AND semantics).
    /// An empty filter (no predicates added) always returns `true`.
    #[must_use]
    pub fn filter(&self, entry: &LogEntry) -> bool {
        self.filters.iter().all(|f| f(entry))
    }

    /// Attach this filter to `store`, returning a [`StoreFilter`] that can
    /// execute the query against the live log store.
    ///
    /// Typically called indirectly via [`Logger::query`](crate::Logger::query).
    #[cfg(editor)]
    pub fn with_store(self, store: Arc<LogStore>) -> StoreFilter {
        StoreFilter {
            filter: self,
            store,
        }
    }
}

/// A [`LogFilter`] bound to a `LogStore`, ready to execute a query.
///
/// Obtain one via [`Logger::query`](crate::Logger::query) or
/// [`LogFilter::with_store`].
///
/// # Example
/// ```rust
/// # let logger = dirk_logging::Logger::new().init().unwrap();
/// # use dirk_logging::{Filter, LogLevel};
/// let errors = logger
///     .query(Filter::new().min_level(LogLevel::Error))
///     .last(100);
/// ```
#[cfg(editor)]
pub struct StoreFilter {
    filter: LogFilter,
    store: Arc<LogStore>,
}

#[cfg(editor)]
impl StoreFilter {
    /// Run the filter and return **all** matching entries in chronological
    /// order (oldest first).
    ///
    /// # Example
    /// ```rust
    /// # let logger = dirk_logging::Logger::new().init().unwrap();
    /// # use dirk_logging::{LogEntry, LogLevel, Filter};
    /// let all_warnings: Vec<LogEntry> = logger
    ///     .query(Filter::new().min_level(LogLevel::Warn))
    ///     .execute();
    /// ```
    #[must_use]
    pub fn execute(self) -> Vec<LogEntry> {
        self.store.with_entries(|entries| {
            entries
                .iter()
                .filter(|e| self.filter.filter(e))
                .cloned()
                .collect()
        })
    }

    /// Return the **most recent `n`** matching entries, still in chronological
    /// order (oldest of the `n` first).
    ///
    /// If fewer than `n` entries match, all matching entries are returned.
    ///
    /// # Example
    /// ```rust
    /// // Show the 50 most recent Rendering entries in the log panel:
    /// # let logger = dirk_logging::Logger::new().init().unwrap();
    /// # use dirk_logging::{LogEntry, LogLevel, Filter};
    /// let recent: Vec<LogEntry> = logger
    ///     .query(Filter::new().of_target("Rendering"))
    ///     .last(50);
    /// ```
    #[must_use]
    pub fn last(self, n: usize) -> Vec<LogEntry> {
        let mut results = self.execute();
        let len = results.len();
        if n < len {
            results.drain(..len - n);
        }
        results
    }

    /// Return the **count** of matching entries without cloning them.
    ///
    /// More efficient than `execute().len()` as it avoids the overhead
    /// of cloning log entries.
    ///
    /// # Example
    /// ```rust
    /// # let logger = dirk_logging::Logger::new().init().unwrap();
    /// # use dirk_logging::{LogEntry, LogLevel, Filter};
    /// let error_count: usize = logger
    ///     .query(Filter::new().of_level(LogLevel::Error))
    ///     .count();
    /// ```
    #[must_use]
    pub fn count(self) -> usize {
        self.store
            .with_entries(|entries| entries.iter().filter(|e| self.filter.filter(e)).count())
    }
}
