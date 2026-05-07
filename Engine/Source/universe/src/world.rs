use crate::{
    Entity, EntityBuilder,
    components::{Component, Components},
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

    // ENTITY MANAGEMENT

    /// Will spawn a new [`Entity`] using the provided [`EntityBuilder`].
    /// Returns the handle of the new [`Entity`].
    pub fn spawn(&mut self, mut builder: EntityBuilder) -> Entity {
        let id = self.next_id;
        self.next_id += 1;
        self.alive.push(id);

        // TODO: find way to add the components to [`Components`].

        builder.components.values_mut().for_each(|component| {
            self.component_systems
                .iter(component.type_id())
                .for_each(|system| system.added(id, component));
        });

        self.world_systems
            .iter()
            .for_each(|system| system.entity_spawned(self, id));

        todo!("World::spawn add components on entity spawn")
    }
    /// Will despawn the provided [`Entity`].
    pub fn despawn(&mut self, entity: Entity) {
        self.alive.retain(|&e| e != entity);
        self.world_systems
            .iter()
            .for_each(|system| system.entity_despawned(self, entity));
        // TODO: for each component call ComponentSystem::removed
        self.components.remove_all(entity);
        todo!("World::spawn call ComponentSystem::removed")
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
    pub fn alive_count(&self) -> usize {
        self.alive.len()
    }

    /// Returns if the specified entity is alive
    #[must_use]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.alive.contains(&entity)
    }

    // COMPONENTS

    /// Attaches a [`Component`]` to [`Entity`], replacing any existing component of
    /// the same type.
    ///
    /// [`Entity`]: crate::Entity
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
