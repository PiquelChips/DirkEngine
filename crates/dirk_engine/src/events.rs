//! This module contains all the main events that are used by the engine.

use dirk_events::Event;

/// An event emitted after the engine has accepted an exit request.
///
/// Systems request exit through [`EngineHandle::exit`] or
/// [`EngineHandle::exit_with_error`]. The engine dispatches this event once it
/// transitions into an exiting state.
///
/// [`EngineHandle::exit`]: crate::EngineHandle::exit
/// [`EngineHandle::exit_with_error`]: crate::EngineHandle::exit_with_error
#[derive(Debug, Clone, Event)]
#[event("Exiting")]
pub struct Exiting;

/// An event run at the beginning of every tick.
///
/// This event contains the frame number.
/// Used for thread synchronization.
#[derive(Debug, Clone, Event)]
#[event("Begin frame number {0}")]
pub struct BeginFrame(pub u64);
