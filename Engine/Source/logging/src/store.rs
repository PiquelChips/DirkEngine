use parking_lot::RwLock;

use crate::LogEntry;

/// Thread-safe append-only store for captured [`LogEntry`] values.
///
/// Shared via `Arc` between the [`StorageLayer`](crate::layers::storage::StorageLayer)
/// (which writes) and the public [`Logger`](crate::Logger) API (which reads through
/// [`QueryBuilder`](crate::QueryBuilder)).
#[derive(Default)]
pub struct LogStore {
    entries: RwLock<Vec<LogEntry>>,
}

impl LogStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a new entry. Called from the tracing subscriber on every event.
    pub fn push(&self, entry: LogEntry) {
        self.entries.write().push(entry);
    }

    /// Borrow the full entry slice inside a closure, avoiding an unnecessary
    /// clone of the entire vector for short-lived reads.
    pub fn with_entries<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&[LogEntry]) -> R,
    {
        f(&self.entries.read())
    }
}
