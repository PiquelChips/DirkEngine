use std::any::TypeId;

use crate::{
    Entity, EntityBuilder,
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
    id: WorldId,
    next_id: Entity,
    alive: Vec<Entity>,
    components: Components,

    #[allow(clippy::struct_field_names)]
    world_systems: WorldSystemStorage,
    ticking_systems: TickingSystemStorage,
    component_systems: ComponentSystemStorage,
}

impl World {
    /// Returns a [`WorldBuilder`] to easily construct a [`World`].
    #[must_use]
    pub fn builder() -> WorldBuilder {
        WorldBuilder::new()
    }
    /// Calls all the destruction [`System`]s on the world
    pub(crate) fn destroy(&mut self) {
        for entity in self.alive().to_vec() {
            self.despawn(entity);
        }
    }

    pub(crate) fn tick(&self, delta_time: f32) {
        self.world_systems
            .iter()
            .for_each(|system| system.tick(self, delta_time));

        self.ticking_systems
            .iter()
            .for_each(|system| system.outer_tick(self, delta_time));
    }

    /// Returns the [`World`]'s [`WorldId`].
    #[must_use]
    pub fn id(&self) -> WorldId {
        self.id
    }

    // ENTITY MANAGEMENT

    /// Will spawn a new [`Entity`] using the provided [`EntityBuilder`].
    /// Returns the handle of the new [`Entity`].
    pub fn spawn(&mut self, builder: EntityBuilder) -> Entity {
        let id = self.next_id;
        self.next_id += 1;
        self.alive.push(id);

        for (_, mut component) in builder.components {
            self.component_systems
                .iter(component.type_id())
                .for_each(|system| system.added(id, &mut component));

            self.components.insert_any(id, component);
        }

        self.world_systems
            .iter()
            .for_each(|system| system.entity_spawned(self, id));
        id
    }

    /// Will despawn the provided [`Entity`].
    ///
    /// Calls [`ComponentSystem::removed`] for every component still attached
    /// to the entity before the components are actually dropped.
    pub fn despawn(&mut self, entity: Entity) {
        self.alive.retain(|&e| e != entity);

        self.world_systems
            .iter()
            .for_each(|system| system.entity_despawned(self, entity));

        for (type_id, mut component) in self.components.remove_all(entity) {
            self.component_systems
                .iter(type_id)
                .for_each(|system| system.removed(entity, &mut component));
        }
    }

    /// Run `query` against all currently alive entities and return the
    /// matching subset.
    ///
    /// Passing the `alive` slice ensures that entities despawned mid-frame
    /// (whose component data may linger briefly) are never returned.
    #[must_use]
    pub(crate) fn query(&self, query: &Query) -> Vec<Entity> {
        query.query(&self.components, &self.alive)
    }

    /// Returns a slice of all currently alive entity IDs in spawn order.
    #[must_use]
    fn alive(&self) -> &[Entity] {
        &self.alive
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

    // COMPONENTS

    /// Attaches a [`Component`] to [`Entity`], replacing any existing component of
    /// the same type.
    ///
    /// [`Entity`]: crate::Entity
    pub fn insert<C: Component>(&mut self, entity: Entity, component: C) {
        if !self.is_alive(entity) {
            return;
        }

        let mut component: Box<dyn AnyComponent> = Box::new(component);

        self.component_systems
            .iter(TypeId::of::<C>())
            .for_each(|system| system.added(entity, &mut component));

        self.components.insert_any(entity, component);
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

    /// Removes a single component from an entity, calling [`ComponentSystem::removed`]
    /// if the component was present.
    ///
    /// The entity itself is **not** despawned. If the component is not
    /// present this is a no-op.
    pub fn remove<C: Component>(&mut self, entity: Entity) {
        if let Some(component) = self.components.remove::<C>(entity) {
            let mut component: Box<dyn AnyComponent> = Box::new(component);
            self.component_systems
                .iter(TypeId::of::<C>())
                .for_each(|system| system.removed(entity, &mut component));
        }
    }
}

/// Builder struct for [`World`].
#[derive(Default)]
pub struct WorldBuilder {
    entities: Vec<EntityBuilder>,
    world_systems: WorldSystemStorage,
    ticking_systems: TickingSystemStorage,
    component_systems: ComponentSystemStorage,
}

impl WorldBuilder {
    #[must_use]
    fn new() -> Self {
        Self::default()
    }

    /// Will actually build a world struct with the provided `id`.
    #[must_use]
    pub fn build(self, id: WorldId) -> World {
        let mut world = World {
            id,
            world_systems: self.world_systems,
            ticking_systems: self.ticking_systems,
            component_systems: self.component_systems,
            ..World::default()
        };

        for builder in self.entities {
            world.spawn(builder);
        }

        world
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
