//! This crate holds the entire universe.
//!
//! The **Universe** is `DirkEngine`'s ECS system.

use std::collections::HashMap;

use crate::systems::{
    ComponentSystemStorage, TickingSystemStorage, UniverseSystemStorage, WorldSystemStorage,
};

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
    component_systems: ComponentSystemStorage,
}

impl Universe {
    /// Returns a [`UniverseBuilder`] to easily construct a [`Universe`].
    #[must_use]
    pub fn builder() -> UniverseBuilder {
        UniverseBuilder::new()
    }

    /// Ticks every the entire [`Universe`].
    pub fn tick(&mut self, delta_time: f32) {
        self.universe_systems
            .iter()
            .for_each(|system| system.tick(self, delta_time));

        self.worlds.values().for_each(|world| {
            self.world_systems
                .iter()
                .for_each(|system| system.tick(world, delta_time));

            self.ticking_systems
                .iter()
                .for_each(|system| system.outer_tick(world, delta_time));

            world.tick(delta_time);
        });
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
        let Some(mut world) = self.worlds.remove(&world) else {
            return;
        };
        self.universe_systems
            .iter()
            .for_each(|system| system.world_destroyed(&world));
        world.destroy();
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
