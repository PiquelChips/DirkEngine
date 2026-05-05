use std::{any::TypeId, collections::HashMap};

use crate::{
    Entity, EntityBuilder,
    components::{Component, Components},
    query::Query,
};

/// An identifier that distinguishes multiple [`World`] instances from each other.
pub type WorldId = u32;

/// This is a world. It has entities and components.
#[derive(Default)]
pub struct World {
    id: WorldId,
    next_id: Entity,
    alive: Vec<Entity>,
    components: Components,
}

impl World {
    /// Returns a [`WorldBuilder`] to easily construct a [`World`].
    #[must_use]
    pub fn builder() -> WorldBuilder {
        WorldBuilder::new()
    }
    /// Calls all the destruction [`System`]s on the world
    pub(crate) fn destroy(&mut self) {
        todo!("call all the world systems for destruction")
    }

    // ENTITY MANAGEMENT

    /// Will spawn a new [`Entity`] using the provided [`EntityBuilder`].
    /// Returns the handle of the new [`Entity`].
    pub fn spawn(&mut self, builder: &EntityBuilder) -> Entity {
        let id = self.next_id;
        self.next_id += 1;
        self.alive.push(id);

        // TODO: find way to add the components to [`Components`].
        // TODO: run all corresponding ComponentSystem::added

        // TODO: Run WorldSystem::entity spawned

        todo!("World::spawn")
    }
    /// Will despawn the provided [`Entity`].
    pub fn despawn(&mut self, entity: Entity) {
        self.alive.retain(|&e| e != entity);
        // TODO: call WorldComponent::entity_despawned
        // TODO: for each component call ComponentSystem::removed
        self.components.remove_all(entity);
        todo!("World::spawn")
    }

    #[must_use]
    pub(crate) fn query(&self, query: &Query) -> Vec<Entity> {
        query.query(self)
    }

    /// Returns a slice of all currently alive entity IDs in spawn order.
    #[must_use]
    fn alive(&self) -> &[Entity] {
        &self.alive
    }

    /// Returns the total number of alive entities.
    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.alive.len()
    }

    /// Returns if the specified entity is alive
    #[must_use]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.alive.contains(&entity)
    }

    // COMPONENTS

    /// Attaches an [`EntityComponent`]` to [`Entity`], replacing any existing component of
    /// the same type.
    pub fn insert<C: Component>(&mut self, entity: Entity, component: C) {
        // TODO: check if entity is alive, if not ignore
        self.components.insert(entity, component);
        todo!("call all related systems")
    }

    /// Returns a shared reference to a component, or `None` if the entity
    /// does not have one.
    #[must_use]
    pub fn get<C: Component>(&self, entity: Entity) -> Option<&C> {
        self.components.get(entity)
    }

    /// Returns a mutable reference to a component, or `None` if the entity
    /// does not have one.
    pub fn get_mut<C: Component>(&mut self, entity: Entity) -> Option<&mut C> {
        self.components.get_mut(entity)
    }

    /// Removes a single component from an entity.
    ///
    /// The entity itself is **not** despawned. If the component is not
    /// present this is a no-op.
    pub fn remove<C: Component>(&mut self, entity: Entity) {
        self.components.remove::<C>(entity);
        todo!("call all related systems");
    }
}

/// Builder struct for [`World`].
#[derive(Default)]
pub struct WorldBuilder {
    entities: Vec<EntityBuilder>,
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

    // TODO: handle systems
}
