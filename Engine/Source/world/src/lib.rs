//! This crate has a bunch of frequently used types in the [`World`].
//!
//! [`World`]: universe::world

use events::EventManager;
use universe::{Universe, UniverseBuilder};

pub mod components;
use components::ModelUploadSystem;

pub mod player;

/// Creates a [`UniverseBuilder`] with all the systems used by the various
/// utilities & types in this crate.
#[must_use] 
pub fn universe_builder(events: &EventManager) -> UniverseBuilder {
    Universe::builder().with_component_system(ModelUploadSystem::new(events))
}
