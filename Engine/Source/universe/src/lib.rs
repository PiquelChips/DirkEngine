//! This crate holds the entire universe.
//!
//! The **Universe** is `DirkEngine`'s ECS system.

use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
};

use crate::{
    components::{AnyComponent, Component, Components},
    systems::{
        ComponentSystem, ComponentSystemStorage, EntitySystem, EntitySystemStorage, TickingSystem,
        TickingSystemStorage, UniverseSystem, UniverseSystemStorage,
    },
};

pub mod components;
pub mod query;
pub mod systems;

mod entity;
pub use entity::{Entity, EntityBuilder};

mod world;
pub use world::{World, WorldBuilder, WorldId};

/// This struct is the manager for all the worlds.
#[derive(Default)]
pub struct Universe {
    worlds: HashMap<WorldId, World>,
    next_world_id: WorldId,

    entities: HashMap<Entity, WorldId>,
    next_entity_id: Entity,

    // this field starts with `universe`
    #[allow(clippy::struct_field_names)]
    universe_systems: UniverseSystemStorage,
    ticking_systems: TickingSystemStorage,
    entity_systems: EntitySystemStorage,
    component_systems: ComponentSystemStorage,

    components: Components,
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
            self.ticking_systems.iter().for_each(|system| {
                // This allocates a new [`Vec`] per [`TickingSystem`] per tick.
                // TODO: optimise this. IDK how tho
                system.tick(
                    self,
                    delta_time,
                    system.query().query(&self.components, &world.alive),
                );
            });
        });
    }

    // UTILITIES

    /// Returns an optional reference to the requested [`World`].
    #[must_use]
    pub fn world(&self, world: WorldId) -> Option<&World> {
        self.worlds.get(&world)
    }

    /// Returns the [`WorldId`] of the [`Entity`]'s [`World`].
    #[must_use]
    pub fn get_world(&self, entity: Entity) -> Option<WorldId> {
        self.entities.get(&entity).copied()
    }

    /// Returns if the given [`Entity`] is in the given [`World`].
    #[must_use]
    pub fn is_in_world(&self, world: WorldId, entity: Entity) -> bool {
        self.entities.get(&entity) == Some(&world)
    }

    /// Returns the total number of alive entities.
    #[must_use]
    pub fn alive_count(&self) -> usize {
        self.entities.len()
    }

    /// Returns if the specified entity is alive
    #[must_use]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.entities.contains_key(&entity)
    }

    // WORLD MANAGEMENT

    /// Will create a new empty world & return its ID.
    pub fn create_world(&mut self, builder: WorldBuilder) -> WorldId {
        let id = self.next_world_id;
        self.next_world_id += 1;

        let world = World {
            id,
            name: builder.name,
            alive: HashSet::new(),
        };

        for builder in builder.entities {
            self.spawn(id, builder);
        }

        self.universe_systems
            .iter()
            .for_each(|system| system.world_created(self, &world));

        self.worlds.insert(id, world);
        id
    }

    /// Will destroy the world & call all its destruction systems.
    pub fn destroy_world(&mut self, world: WorldId) {
        let Some(world) = self.worlds.remove(&world) else {
            return;
        };
        self.universe_systems
            .iter()
            .for_each(|system| system.world_destroyed(self, &world));

        // `clone` is expensive but its the only way I found for the
        // borrow checker. As this is called very rarely (on world destruction),
        // it should not have too big of an effect on runtime performance.
        for entity in world.alive.clone() {
            self.despawn(entity);
        }
    }

    // ENTITY MANAGEMENT

    /// Will spawn a new [`Entity`] using the provided [`EntityBuilder`].
    /// Returns the handle of the new [`Entity`].
    ///
    /// If None, then the [`World`] does not exist.
    pub fn spawn(&mut self, world: WorldId, builder: EntityBuilder) -> Option<Entity> {
        let world = self.worlds.get_mut(&world)?;

        let entity = self.next_entity_id;
        self.next_entity_id += 1;
        self.entities.insert(entity, world.id);
        world.alive.insert(entity);

        for (_, mut component) in builder.components {
            self.component_systems
                .iter(component.type_id())
                .for_each(|system| system.added(entity, &mut component));

            self.components.insert_any(entity, component);
        }

        self.universe_systems
            .iter()
            .for_each(|system| system.entity_spawned(self, entity));

        self.entity_systems.iter().for_each(|system| {
            if let Some(query) = system.query()
                && !query.matches(&self.components, entity)
            {
                return;
            }

            system.spawned(self, entity);
        });
        Some(entity)
    }

    /// Will despawn the provided [`Entity`].
    ///
    /// Calls [`ComponentSystem::removed`] for every component still attached
    /// to the entity before the components are actually dropped.
    pub fn despawn(&mut self, entity: Entity) {
        let Some(world) = self.entities.remove(&entity) else {
            // if the entity was not present, systems shouldn't be called
            return;
        };
        let Some(world) = self.worlds.get_mut(&world) else {
            return;
        };

        if !world.alive.remove(&entity) {
            // if the entity was not present, systems shouldn't be called
            return;
        }

        self.universe_systems
            .iter()
            .for_each(|system| system.entity_despawned(self, entity));

        self.entity_systems.iter().for_each(|system| {
            if let Some(query) = system.query()
                && !query.matches(&self.components, entity)
            {
                return;
            }

            system.despawned(self, entity);
        });

        for (type_id, mut component) in self.components.remove_all(entity) {
            self.component_systems
                .iter(type_id)
                .for_each(|system| system.removed(entity, &mut component));
        }
    }

    /// Will send the [`Entity`] to the specified [`WorldId`].
    ///
    /// Returns if the operation was successful. Will fail if the [`Entity`]
    /// or the [`World`] don't exist.
    pub fn send(&mut self, entity: Entity, to: WorldId) -> bool {
        let Some(world) = self.entities.get(&entity).copied() else {
            return false;
        };

        if !self.worlds.contains_key(&to) {
            return false;
        }

        let Some(old) = self.worlds.get_mut(&world) else {
            return false;
        };
        old.alive.remove(&entity);

        let Some(new) = self.worlds.get_mut(&to) else {
            return false;
        };
        new.alive.insert(entity);

        self.entities.insert(entity, to);

        self.entity_systems.iter().for_each(|system| {
            if let Some(query) = system.query()
                && !query.matches(&self.components, entity)
            {
                return;
            }

            system.sent(self, entity, world, to);
        });

        true
    }

    // COMPONENT MANAGEMENT

    /// Attaches a [`Component`] to [`Entity`], replacing any existing component of
    /// the same type.
    ///
    /// [`ComponentSystem::added`] is called every time.
    ///
    /// When replacing, [`ComponentSystem::removed`] is called.
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

        if self.components.contains(entity, component.type_id()) {
            self.component_systems
                .iter(TypeId::of::<C>())
                .for_each(|system| system.removed(entity, &mut component));
        }

        self.components.insert_any(entity, component);
    }

    /// Returns a shared reference to a component, or `None` if the entity
    /// does not have one.
    #[must_use]
    pub fn component<C: Component>(&self, entity: Entity) -> Option<&C> {
        self.components.get(entity)
    }

    /// Returns a mutable reference to a component, or `None` if the entity
    /// does not have one.
    pub fn component_mut<C: Component>(&mut self, entity: Entity) -> Option<&mut C> {
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

/// Builder struct used to construct a [`Universe`].
#[derive(Default)]
pub struct UniverseBuilder {
    worlds: Vec<WorldBuilder>,
    universe_systems: UniverseSystemStorage,
    ticking_systems: TickingSystemStorage,
    entity_systems: EntitySystemStorage,
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
            entity_systems: self.entity_systems,
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

    /// Adds a [`EntitySystem`] that will be added to the [`Universe`].
    #[must_use]
    pub fn with_entity_system(mut self, system: impl EntitySystem) -> Self {
        self.entity_systems.insert(system);
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

    /// Will combine the `other` [`UniverseBuilder`] with this one.
    #[must_use]
    pub fn with_other(mut self, other: Self) -> Self {
        for world in other.worlds {
            self.worlds.push(world);
        }

        for system in other.universe_systems {
            self.universe_systems.insert_any(system);
        }

        for system in other.entity_systems {
            self.entity_systems.insert_any(system);
        }

        for system in other.ticking_systems {
            self.ticking_systems.insert_any(system);
        }

        for (type_id, systems) in other.component_systems {
            for system in systems {
                self.component_systems.insert_any(type_id, system);
            }
        }

        self
    }
}
