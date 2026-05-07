//! This crate holds the entire universe.
//!
//! The **Universe** is `DirkEngine`'s ECS system.

use std::collections::HashMap;

use crate::systems::{
    ComponentSystem, ComponentSystemStorage, TickingSystem, TickingSystemStorage, UniverseSystem,
    UniverseSystemStorage, WorldSystem, WorldSystemStorage,
};

pub mod components;
pub mod query;
pub mod systems;

mod world;
pub use world::{World, WorldBuilder, WorldId};

mod entity;
pub use entity::{Entity, EntityBuilder};

/// This struct is the manager for all the worlds.
#[derive(Default)]
pub struct Universe {
    worlds: HashMap<WorldId, World>,
    next_id: WorldId,

    // this field starts with `universe`
    #[allow(clippy::struct_field_names)]
    universe_systems: UniverseSystemStorage,
    ticking_systems: TickingSystemStorage,
    world_systems: WorldSystemStorage,
    // TODO: figure out how to apply these to every [`World`].
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
    pub fn create_world(&mut self, builder: WorldBuilder) -> WorldId {
        let id = self.next_id;
        self.next_id += 1;

        let world = builder.build(id);

        self.universe_systems
            .iter()
            .for_each(|system| system.world_created(&world));

        self.worlds.insert(id, world);
        id
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

/// Builder struct used to construct a [`Universe`].
#[derive(Default)]
pub struct UniverseBuilder {
    worlds: Vec<WorldBuilder>,
    universe_systems: UniverseSystemStorage,
    ticking_systems: TickingSystemStorage,
    world_systems: WorldSystemStorage,
    component_systems: ComponentSystemStorage,
}

impl UniverseBuilder {
    #[must_use]
    fn new() -> Self {
        Self::default()
    }

    /// Actually builds the [`Universe`].
    #[must_use]
    pub fn build(self) -> Universe {
        let mut universe = Universe {
            universe_systems: self.universe_systems,
            ticking_systems: self.ticking_systems,
            world_systems: self.world_systems,
            component_systems: self.component_systems,
            ..Universe::default()
        };

        for builder in self.worlds {
            universe.create_world(builder);
        }

        universe
    }

    /// Adds a [`World`] that will be created at the same time as the [`Universe`].
    #[must_use]
    pub fn with_world(mut self, builder: WorldBuilder) -> Self {
        self.worlds.push(builder);
        self
    }

    /// Adds a [`UniverseSystem`] that will be added to the [`Universe`].
    #[must_use]
    pub fn with_universe_system(mut self, system: impl UniverseSystem) -> Self {
        self.universe_systems.insert(system);
        self
    }

    /// Adds a [`WorldSystem`] that will be added to the [`Universe`].
    #[must_use]
    pub fn with_world_system(mut self, system: impl WorldSystem) -> Self {
        self.world_systems.insert(system);
        self
    }

    /// Adds a [`TickingSystem`] that will be added to the [`Universe`].
    #[must_use]
    pub fn with_ticking_system(mut self, system: impl TickingSystem) -> Self {
        self.ticking_systems.insert(system);
        self
    }

    /// Adds a [`ComponentSystem`] that will be added to the [`Universe`].
    #[must_use]
    pub fn with_component_system(mut self, system: impl ComponentSystem) -> Self {
        self.component_systems.insert(system);
        self
    }
}
