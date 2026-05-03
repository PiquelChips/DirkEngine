//! This crate holds the entire universe.
//!
//! The **Universe** is `DirkEngine`'s ECS system.

use std::collections::HashMap;

use crate::components::Components;

pub mod components;
pub mod query;
pub mod systems;

/// A unique, opaque identifier for a spawned entity.
pub type Entity = u32;
/// An identifier that distinguishes multiple [`World`] instances from each other.
pub type WorldId = u32;

/// This struct is the manager for all the worlds.
pub struct Universe {
    worlds: HashMap<WorldId, World>,
    next_id: WorldId,
}

/// This is a world. It has entities and components.
pub struct World {
    id: WorldId,
    next_id: Entity,
    alive: Vec<Entity>,
    components: Components,
}
