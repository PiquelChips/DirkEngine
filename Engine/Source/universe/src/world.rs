use std::{any::TypeId, collections::HashSet};

use crate::{
    Entity, EntityBuilder, Universe,
    components::{AnyComponent, Component, Components},
    query::Query,
    systems::{
        ComponentSystem, ComponentSystemStorage, TickingSystem, TickingSystemStorage, WorldSystem,
        WorldSystemStorage,
    },
};

/// An identifier that distinguishes multiple [`World`] instances from each other.
pub type WorldId = u32;

/// This is a world. It has entities and components.
#[derive(Default)]
pub struct World {
    pub(crate) id: WorldId,
    pub(crate) next_id: Entity,
    pub(crate) alive: HashSet<Entity>,
    pub(crate) components: Components,

    #[allow(clippy::struct_field_names)]
    pub(crate) world_systems: WorldSystemStorage,
    pub(crate) ticking_systems: TickingSystemStorage,
    pub(crate) component_systems: ComponentSystemStorage,
}

impl World {
    /// Returns a [`WorldBuilder`] to easily construct a [`World`].
    #[must_use]
    pub fn builder() -> WorldBuilder {
        WorldBuilder::new()
    }

    pub(crate) fn tick(&self, delta_time: f32) {
        self.world_systems
            .iter()
            .for_each(|system| system.tick(self, delta_time));

        self.ticking_systems.iter().for_each(|system| {
            // This allocates a new [`Vec`] per [`TickingSystem`] per tick.
            // TODO: optimise this. IDK how tho
            system.tick(self, delta_time, self.query(&system.query()));
        });
    }

    /// Returns the [`World`]'s [`WorldId`].
    #[must_use]
    pub fn id(&self) -> WorldId {
        self.id
    }

    // ENTITY MANAGEMENT

    /// Run `query` against all currently alive entities and return the
    /// matching subset.
    ///
    /// Passing the `alive` slice ensures that entities despawned mid-frame
    /// (whose component data may linger briefly) are never returned.
    #[must_use]
    pub(crate) fn query(&self, query: &Query) -> Vec<Entity> {
        query.query(&self.components, &self.alive)
    }

    /// Returns the total number of alive entities.
    #[must_use]
    pub fn alive_count(&self) -> usize {
        self.alive.len()
    }

    /// Returns if the specified entity is alive
    #[must_use]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.alive.contains(&entity)
    }
}

/// Builder struct for [`World`].
#[derive(Default)]
pub struct WorldBuilder {
    pub(crate) entities: Vec<EntityBuilder>,
    pub(crate) world_systems: WorldSystemStorage,
    pub(crate) ticking_systems: TickingSystemStorage,
    pub(crate) component_systems: ComponentSystemStorage,
}

impl WorldBuilder {
    #[must_use]
    fn new() -> Self {
        Self::default()
    }

    /// Adds an [`Entity`] that will be spawned on [`World`] creation.
    #[must_use]
    pub fn with_entity(mut self, entity: EntityBuilder) -> Self {
        self.entities.push(entity);
        self
    }

    /// Adds a [`WorldSystem`] that will be added to the [`World`].
    #[must_use]
    pub fn with_world_system(mut self, system: impl WorldSystem) -> Self {
        self.world_systems.insert(system);
        self
    }

    /// Adds a [`TickingSystem`] that will be added to the [`World`].
    #[must_use]
    pub fn with_ticking_system(mut self, system: impl TickingSystem) -> Self {
        self.ticking_systems.insert(system);
        self
    }

    /// Adds a [`ComponentSystem`] that will be added to the [`World`].
    #[must_use]
    pub fn with_component_system(mut self, system: impl ComponentSystem) -> Self {
        self.component_systems.insert(system);
        self
    }
}
