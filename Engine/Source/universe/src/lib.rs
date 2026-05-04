//! This crate holds the entire universe.
//!
//! The **Universe** is `DirkEngine`'s ECS system.

use std::{any::TypeId, collections::HashMap};

use crate::components::{AnyComponent, EntityComponents};

pub mod components;
pub mod query;
pub mod systems;

/// A unique, opaque identifier for a spawned entity.
pub type Entity = u32;
/// An identifier that distinguishes multiple [`World`] instances from each other.
pub type WorldId = u32;

/// This struct is the manager for all the worlds.
#[derive(Default)]
pub struct Universe {
    worlds: HashMap<WorldId, World>,
    next_id: WorldId,
}

impl Universe {
    /// Creates a new empty [`Universe`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns an optional reference to the requested [`World`].
    pub fn get_world(&self, world: WorldId) -> Option<&World> {
        self.worlds.get(&world)
    }
    /// Returns an optional mutable reference to the requested [`World`].
    pub fn get_world_mut(&mut self, world: WorldId) -> Option<&mut World> {
        self.worlds.get_mut(&world)
    }
    /// Will create a new empty world & return its ID.
    pub fn create_world(&mut self) -> WorldId {
        // maybe some kind of world builder?
        todo!("create a new world")
    }
    /// Will destroy the world & call all its destruction systems.
    pub fn destroy_world(&mut self, world: WorldId) {
        let _world = self.worlds.remove(&world);
        todo!("call all the world destruction systems")
    }
}

/// This is a world. It has entities and components.
#[derive(Default)]
pub struct World {
    id: WorldId,
    next_id: Entity,
    alive: Vec<Entity>,
    entity_components: EntityComponents,
    /// These should only be [`components::WorldComponent`]. This is
    /// guaranteed by the [`World`] API. Please make sure to add these
    /// properly internally.
    components: HashMap<TypeId, Box<dyn AnyComponent>>,
}

impl World {
    /// Creates a new empty world with the specified ID.
    fn new(id: WorldId) -> Self {
        Self {
            id,
            ..Self::default()
        }
    }
    /// Calls all the destruction [`System`]s on the world
    fn destroy(&mut self) {
        todo!("call all the world systems for destruction")
    }
}
