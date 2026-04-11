use crate::Logger;
use crate::layers::{console::ConsoleLayer, file::FileLayer};
use crate::{layers::storage::StorageLayer, store::LogStore};

use thiserror::Error;
use tracing_subscriber::{
    Registry, filter::LevelFilter, layer::SubscriberExt, util::SubscriberInitExt,
};

#[cfg(editor)]
use std::sync::Arc;

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
