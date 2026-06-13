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
    /// When a plugin failed to register its build-time pieces.
    #[error("plugin `{name}` failed to build")]
    PluginBuildFailed {
        /// The plugin name.
        name: &'static str,
        /// The error that caused the build failure.
        #[source]
        source: anyhow::Error,
    },
    /// When plugin dependencies contain a cycle.
    #[error("plugin dependency cycle detected while building `{name}` ({type_name})")]
    PluginDependencyCycle {
        /// The plugin name.
        name: &'static str,
        /// The concrete plugin type name.
        type_name: &'static str,
    },
    /// When a requested typed engine resource has not been registered.
    #[error("resource `{type_name}` was not registered")]
    ResourceMissing {
        /// The requested resource type name.
        type_name: &'static str,
    },
    /// When a typed engine resource has already been registered.
    #[error("resource `{type_name}` was already registered")]
    ResourceAlreadyRegistered {
        /// The duplicate resource type name.
        type_name: &'static str,
    },
    /// When a stored resource does not match the requested concrete type.
    #[error("resource `{type_name}` had an unexpected concrete type")]
    ResourceTypeMismatch {
        /// The requested resource type name.
        type_name: &'static str,
    },
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
