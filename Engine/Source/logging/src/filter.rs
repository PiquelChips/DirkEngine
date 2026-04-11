#[cfg(editor)]
use {crate::store::LogStore, std::sync::Arc};

use time::OffsetDateTime;

use crate::{LogEntry, LogLevel};

type Filter = Box<dyn Fn(&LogEntry) -> bool + Send + Sync>;

/// Fluent filter builder.
///
/// Filters are composed with AND semantics and evaluated lazily
/// when [`filter`](Self::filter) is called.
///
/// # Example
/// ```rust
/// # use logging::{LogLevel, Filter, LogEntry};
/// # let entry = LogEntry {
/// #     level: LogLevel::Warn,
/// #     category: "Test".to_string(),
/// #     timestamp: time::OffsetDateTime::now_utc(),
/// #     message: "Test".to_string(),
/// # };
/// let errors_filter = Filter::new()
///     .of_category("Rendering")
///     .min_level(LogLevel::Warn)
///     .filter(&entry);
/// ```
#[derive(Default)]
pub struct LogFilter {
    filters: Vec<Filter>,
}

impl LogFilter {
    pub fn new() -> Self {
        Self::default()
    }

    // Filters

    /// Only include entries whose category exactly matches `category`.
    pub fn of_category(mut self, category: impl Into<String>) -> Self {
        let cat = category.into();
        self.filters.push(Box::new(move |e| e.category == cat));
        self
    }

    /// Only include entries whose category contains `substring`.
    pub fn category_contains(mut self, substring: impl Into<String>) -> Self {
        let sub = substring.into();
        self.filters
            .push(Box::new(move |e| e.category.contains(sub.as_str())));
        self
    }

    /// Only include entries with exactly this level.
    pub fn of_level(mut self, level: LogLevel) -> Self {
        self.filters.push(Box::new(move |e| e.level == level));
        self
    }

    /// Only include entries at or above this severity
    /// (`Error` is most severe; `Trace` is least).
    pub fn min_level(mut self, level: LogLevel) -> Self {
        self.filters.push(Box::new(move |e| e.level <= level));
        self
    }

    /// Only include entries recorded at or after `time`.
    pub fn since(mut self, time: OffsetDateTime) -> Self {
        self.filters.push(Box::new(move |e| e.timestamp >= time));
        self
    }

    /// Only include entries recorded at or before `time`.
    pub fn until(mut self, time: OffsetDateTime) -> Self {
        self.filters.push(Box::new(move |e| e.timestamp <= time));
        self
    }

    /// Only include entries recorded within the last `seconds` seconds.
    pub fn within_last_seconds(self, seconds: i64) -> Self {
        let cutoff = OffsetDateTime::now_utc() - time::Duration::seconds(seconds);
        self.since(cutoff)
    }

    /// Only include entries whose message contains `pattern` (case-sensitive).
    pub fn matching(mut self, pattern: impl Into<String>) -> Self {
        let pat = pattern.into();
        self.filters
            .push(Box::new(move |e| e.message.contains(pat.as_str())));
        self
    }

    // Using the filter

    pub fn filter(&self, entry: &LogEntry) -> bool {
        self.filters.iter().all(|f| f(entry))
    }

    #[cfg(editor)]
    pub fn with_store(self, store: Arc<LogStore>) -> StoreFilter {
        StoreFilter {
            filter: self,
            store,
        }
    }
}

#[cfg(editor)]
pub struct StoreFilter {
    filter: LogFilter,
    store: Arc<LogStore>,
}

#[cfg(editor)]
impl StoreFilter {
    /// Run all accumulated filters and return matching entries in
    /// chronological order (oldest first).
    pub fn execute(self) -> Vec<LogEntry> {
        self.store.with_entries(|entries| {
            entries
                .iter()
                .filter(|e| self.filter.filter(e))
                .cloned()
                .collect()
        })
    }

    /// Return the most recent `n` matching entries (still in chronological
    /// order). Equivalent to `execute()` then taking the tail.
    pub fn last(self, n: usize) -> Vec<LogEntry> {
        let mut results = self.execute();
        let len = results.len();
        if n < len {
            results.drain(..len - n);
        }
        results
    }

    /// Return the count of matching entries without cloning them.
    pub fn count(self) -> usize {
        self.store
            .with_entries(|entries| entries.iter().filter(|e| self.filter.filter(e)).count())
    }
}
