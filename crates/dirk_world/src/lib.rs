#![doc = include_str!("../README.md")]

use dirk_events::EventManager;
use dirk_universe::{Universe, UniverseBuilder};

pub mod components;
use components::ModelUploadSystem;

/// Creates a [`UniverseBuilder`] with all the systems used by the various
/// utilities & types in this crate.
#[must_use]
pub fn universe_builder(events: &EventManager) -> UniverseBuilder {
    Universe::builder().with_component_system(ModelUploadSystem::new(events))
}
