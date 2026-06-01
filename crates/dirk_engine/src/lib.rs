//! This module holds all the traits & struct for the engine.

pub mod events;

pub mod errors;

pub mod build;
use build::EngineBuilder;

use crate::errors::{EngineBuildError, EngineError};

/// The main engine object.
///
/// Create an [`Engine`] with the [`EngineBuilder`] to specifiy modules &
/// subsystems that you want in the engine.
pub struct Engine {}

impl Engine {
    /// Returns a [`EngineBuilder`]. This struct allows you to specify
    /// the different modules & subsystems.
    pub fn builder() -> EngineBuilder {
        EngineBuilder {}
    }

    /// Actually runs the engine. This is a blocking loop that will not
    /// exit until exit is requested or an error occurs.
    pub fn run(self) -> Result<(), EngineError> {
        Ok(())
    }

    fn build(builder: EngineBuilder) -> Result<Self, EngineBuildError> {
        Ok(Self {})
    }
}
