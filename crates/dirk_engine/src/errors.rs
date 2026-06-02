//! This module contains all engine related errors.

use thiserror::Error;

/// A wrapper for the result type that has an [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// All errors that could be emitted by the engine.
#[derive(Error, Debug)]
pub enum Error {
    /// An error occured when initializing the logger
    #[error("failed to initialize logging")]
    LoggerFailure(#[from] piquel_log::InitError),
    /// When an engine system failed to tick
    #[error("system {name} failed tick: {source}")]
    SubsystemFailedTick {
        /// The name of the system
        name: &'static str,
        /// The error that caused the failure
        #[source]
        source: anyhow::Error,
    },
    /// When an engine system failed to initialize
    #[error("system failed to initialize: {0}")]
    SubsystemFailedInit(#[source] anyhow::Error),
    /// An error occured while starting
    #[error("engine failed to start: {0}")]
    StartFailed(#[source] anyhow::Error),
    /// An error occured while ticking
    #[error("engine failed while ticking: {0}")]
    TickFailed(#[source] anyhow::Error),
    /// An error occured while shutting down
    #[error("engine failed while shutting down: {0}")]
    ShutdownFailed(#[source] anyhow::Error),
}
