//! This crate holds the entire universe.
//!
//! The **Universe** is `DirkEngine`'s ECS system.

use std::collections::HashMap;

use crate::systems::{TickingSystemStorage, UniverseSystemStorage, WorldSystemStorage};

pub mod components;
pub mod query;
pub mod systems;

mod world;
pub use world::{World, WorldBuilder, WorldId};

mod entity;
pub use entity::{Entity, EntityBuilder};

/// This struct is the manager for all the worlds.
pub struct Universe {
    worlds: HashMap<WorldId, World>,
    next_id: WorldId,

    // this field starts with `universe`
    #[allow(clippy::struct_field_names)]
    universe_systems: UniverseSystemStorage,
    ticking_systems: TickingSystemStorage,
    world_systems: WorldSystemStorage,
    // TODO: component system storage
    // component_systems: ComponentSystemStorage,
}

impl Universe {
    #[must_use]
    pub fn builder() -> UniverseBuilder {
        UniverseBuilder::new()
    }
    /// Returns an optional reference to the requested [`World`].
    #[must_use]
    pub fn get_world(&self, world: WorldId) -> Option<&World> {
        self.worlds.get(&world)
    }
    /// Returns an optional mutable reference to the requested [`World`].
    #[must_use]
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

#[derive(Default)]
pub struct UniverseBuilder {}

impl UniverseBuilder {
    #[must_use]
    fn new() -> Self {
        Self::default()
    }

    pub fn with_world(self, builder: WorldBuilder) -> Self {
        todo!("UniverseBuilder::with_world")
    }
}
