//! This module contains everything to do with subsystems.
//!
//! Subsystems are engine systems that you can activate when building the
//! engine through [`EngineBuilder`].
//!
//! [`EngineBuilder`]: crate::build::EngineBuilder

use dirk_universe::{Universe, UniverseBuilder};

use crate::EngineHandle;

/// A subsystem is a type that will be run by the engine.
pub trait Subsystem {
    /// The name of the subsystem.
    /// Used for debugging.
    const NAME: &str;

    /// Initialises the subsystem. Called only once.
    fn init(engine: EngineHandle) -> Self;
    /// Initialises the subsystem. Called only once.
    fn shutdown(self) -> anyhow::Result<()>;

    /// Returns a [`UniverseBuilder`].
    ///
    /// The universe is a core part of the engine.
    /// Use this to register any systems you would need on the ECS system.
    fn universe_builder(&self) -> UniverseBuilder {
        Universe::builder()
    }
}
