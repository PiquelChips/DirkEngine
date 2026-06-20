//! Engine-owned editor subsystem and capability API.

use std::{
    any::type_name,
    collections::{BTreeMap, HashMap},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use anyhow::Context as _;
use parking_lot::Mutex;

use crate::{EngineBuildContext, EngineHandle, errors::Error};

/// Editor lifecycle subsystem owned by the engine.
pub trait EditorSubsystem: Send + 'static {
    /// The subsystem name used for diagnostics.
    fn name(&self) -> &'static str;

    /// Starts the editor subsystem after regular subsystems have started.
    ///
    /// # Errors
    ///
    /// Returns an error if startup fails.
    fn start(&mut self, _context: &mut EditorStartContext<'_>) -> anyhow::Result<()> {
        Ok(())
    }

    /// Advances the editor subsystem by one engine tick.
    ///
    /// # Errors
    ///
    /// Returns an error if ticking fails.
    fn tick(&mut self, _context: &mut EditorTickContext<'_>) -> anyhow::Result<()> {
        Ok(())
    }

    /// Shuts the editor subsystem down.
    ///
    /// # Errors
    ///
    /// Returns an error if shutdown fails.
    fn shutdown(&mut self, _context: &mut EditorShutdownContext<'_>) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Build-time context for editor subsystem factories.
pub struct EditorBuildContext<'a> {
    /// Mutable engine build context for core services and typed resources.
    pub engine: &'a mut EngineBuildContext,
    /// Shared editor services available to editor subsystems.
    pub editor: &'a EditorServices,
}

/// Context passed to editor subsystem startup.
pub struct EditorStartContext<'a> {
    /// Shared engine handle.
    pub engine: &'a EngineHandle,
    /// Read-only ECS universe.
    pub universe: &'a dirk_universe::Universe,
    /// Shared editor services.
    pub editor: &'a EditorServices,
}

/// Context passed to editor subsystem ticks.
pub struct EditorTickContext<'a> {
    /// Seconds elapsed since the previous engine tick.
    pub delta_time: f64,
    /// Shared engine handle.
    pub engine: &'a EngineHandle,
    /// Read-only ECS universe.
    pub universe: &'a dirk_universe::Universe,
    /// Shared editor services.
    pub editor: &'a EditorServices,
}

/// Context passed to editor subsystem shutdown.
pub struct EditorShutdownContext<'a> {
    /// Shared engine handle.
    pub engine: &'a EngineHandle,
    /// Read-only ECS universe.
    pub universe: &'a dirk_universe::Universe,
    /// Shared editor services.
    pub editor: &'a EditorServices,
}

