use std::sync::Arc;

use time::OffsetDateTime;

use crate::{
    entry::{LogEntry, LogLevel},
    store::LogStore,
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// QueryBuilder
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

type Filter = Box<dyn Fn(&LogEntry) -> bool + Send + Sync>;

/// Fluent query builder for the in-memory log store.
///
/// Filters are composed with AND semantics and evaluated lazily when
/// [`execute`](Self::execute) or [`last`](Self::last) is called.
///
/// # Example
/// ```rust,ignore
/// let errors = logger
///     .query()
///     .of_category("Rendering")
///     .min_level(LogLevel::Warn)
///     .since(start_of_frame)
///     .execute();
/// ```
pub struct QueryBuilder {
    store: Arc<LogStore>,
    filters: Vec<Filter>,
}

impl QueryBuilder {
    pub(crate) fn new(store: Arc<LogStore>) -> Self {
        Self {
            store,
            filters: Vec::new(),
        }
    }

    // ── Category ─────────────────────────────────────────────────────────────

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

    // ── Level ─────────────────────────────────────────────────────────────────

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

    // ── Time ──────────────────────────────────────────────────────────────────

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

    // ── Message ───────────────────────────────────────────────────────────────

    /// Only include entries whose message contains `pattern` (case-sensitive).
    pub fn matching(mut self, pattern: impl Into<String>) -> Self {
        let pat = pattern.into();
        self.filters
            .push(Box::new(move |e| e.message.contains(pat.as_str())));
        self
    }

    // ── Terminal operations ───────────────────────────────────────────────────

    /// Run all accumulated filters and return matching entries in
    /// chronological order (oldest first).
    pub fn execute(self) -> Vec<LogEntry> {
        self.store.with_entries(|entries| {
            entries
                .iter()
                .filter(|e| self.filters.iter().all(|f| f(e)))
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
        self.store.with_entries(|entries| {
            entries
                .iter()
                .filter(|e| self.filters.iter().all(|f| f(e)))
                .count()
        })
    }
}
