//! This module contains common event types used throughout the engine.

use crate::Event;

/// An event to request to the engine to exit.
///
/// This event is used by various engine systems.
/// It can be used by the platform to signal to the engine that the windows
/// have all been closed. It is also used when users manually exit the engine.
#[derive(Debug, Clone, Event)]
#[event("Exiting")]
pub struct Exiting;

/// An event to request to the engine to exit.
///
/// This event is used by various engine systems.
/// It can be used by the platform to signal to the engine that the windows
/// have all been closed. It is also used when users manually exit the engine.
///
/// TODO: pass an engine handle around & allow calling engine exit
#[derive(Debug, Clone, Event)]
#[event("App exit requested: {0}")]
pub struct AppExit(pub String);

/// An event run at the beginning of every tick.
///
/// This event contains the frame number.
/// Used for thread synchronization.
#[derive(Debug, Clone, Event)]
#[event("Begin frame number {0}")]
pub struct BeginFrame(pub u64);
