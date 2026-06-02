//! Runtime subsystem lifecycle API.
//!
//! Subsystems are objects owned by the engine after [`EngineBuilder`] has
//! finished building the core runtime. They are distinct from plugins: plugins
//! configure the builder, while subsystems are started, ticked, and shut down by
//! the engine loop.
//!
//! [`EngineBuilder`]: crate::EngineBuilder

use dirk_universe::Universe;

use crate::{EngineBuilder, EngineHandle};

/// Builder-time extension point for engine features.
///
/// Plugins configure an [`EngineBuilder`]. They should register subsystem
/// factories, ECS systems, resources, event registrations, editor panels, or
/// other future build-time integrations. They are not ticked by the engine.
pub trait EnginePlugin {
    /// Returns the plugin name for diagnostics.
    fn name(&self) -> &'static str;

    /// Registers this plugin with the engine builder.
    ///
    /// # Errors
    ///
    /// Returns an error if plugin registration fails.
    fn build(&self, builder: &mut EngineBuilder) -> anyhow::Result<()>;
}

/// A runtime system owned and driven by the engine.
pub trait Subsystem {
    /// The name of the subsystem.
    ///
    /// Used for diagnostics and error reporting.
    fn name(&self) -> &'static str;

    /// Starts the subsystem after all subsystems and the ECS have been built.
    ///
    /// The default implementation is a no-op.
    ///
    /// # Errors
    ///
    /// Returns an error if startup fails.
    fn start(&mut self, _handle: &EngineHandle, _universe: &mut Universe) -> anyhow::Result<()> {
        Ok(())
    }

    /// Advances the subsystem by one engine tick.
    ///
    /// The default implementation is a no-op.
    ///
    /// # Errors
    ///
    /// Returns an error if the subsystem cannot complete its tick.
    fn tick(
        &mut self,
        _delta_time: f64,
        _handle: &EngineHandle,
        _universe: &mut Universe,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// Shuts the subsystem down before the engine releases core services.
    ///
    /// The default implementation is a no-op.
    ///
    /// # Errors
    ///
    /// Returns an error if shutdown fails.
    fn shutdown(&mut self, _handle: &EngineHandle, _universe: &mut Universe) -> anyhow::Result<()> {
        Ok(())
    }
}
