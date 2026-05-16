#![cfg(feature = "editor")]

use std::sync::Arc;

use tracing::{Event, Subscriber};
use tracing_subscriber::{Layer, layer::Context};

use super::format::extract_event_data;
use crate::{
    store::LogStore,
    {LogEntry, LogLevel},
};

/// A [`tracing_subscriber::Layer`] that captures every event into the shared
/// [`LogStore`], making them available to the Editor's log panel via
/// [`Logger::query`](crate::Logger::query).
///
/// Only compiled when the `editor` feature is enabled.
pub struct StorageLayer {
    store: Arc<LogStore>,
}

impl StorageLayer {
    pub fn new(store: Arc<LogStore>) -> Self {
        Self { store }
    }
}

impl<S: Subscriber> Layer<S> for StorageLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let (message, target, timestamp) = extract_event_data(event);
        let level = LogLevel::from(event.metadata().level());

        self.store.push(LogEntry {
            level,
            target,
            timestamp,
            message,
        });
    }
}
