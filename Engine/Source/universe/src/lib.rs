//! This crate holds the entire universe.
//!
//! The **Universe** is `DirkEngine`'s ECS system.

use std::{any::TypeId, collections::HashMap};

use crate::{
    components::{AnyComponent, Component},
    systems::{
        ComponentSystem, ComponentSystemStorage, TickingSystem, TickingSystemStorage,
        UniverseSystem, UniverseSystemStorage, WorldSystem, WorldSystemStorage,
    },
};

pub mod components;
pub mod query;
pub mod systems;

pub mod world;
use world::{World, WorldBuilder, WorldId};

pub mod entity;
use entity::{Entity, EntityBuilder};

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
    // TODO: run when adding any component
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

            self.ticking_systems.iter().for_each(|system| {
                // This allocates a new [`Vec`] per [`TickingSystem`] per tick.
                // TODO: optimise this. IDK how tho
                system.tick(world, delta_time, world.query(&system.query()));
            });

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

        let world = World {
            id,
            world_systems: builder.world_systems,
            ticking_systems: builder.ticking_systems,
            component_systems: builder.component_systems,
            ..World::default()
        };

        for builder in builder.entities {
            self.spawn(id, builder);
        }

        self.universe_systems
            .iter()
            .for_each(|system| system.world_created(&world));

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
            .for_each(|system| system.world_destroyed(&world));

        // `clone` is expensive but its the only way I found for the
        // borrow checker. As this is called very rarely (on world destruction),
        // it should not have too big of an effect on runtime performance.
        for entity in world.alive.clone() {
            self.despawn(world.id(), entity);
        }
    }

    /// Returns all the [`World`]s of the [`Universe`].
    #[must_use]
    pub fn worlds(&self) -> &HashMap<WorldId, World> {
        &self.worlds
    }

    /// Returns all the [`World`]s of the [`Universe`].
    #[must_use]
    pub fn worlds_mut(&mut self) -> &mut HashMap<WorldId, World> {
        &mut self.worlds
    }

    // ENTITY MANAGEMENT

    /// Will spawn a new [`Entity`] using the provided [`EntityBuilder`].
    /// Returns the handle of the new [`Entity`].
    ///
    /// If None, then the [`World`] does not exist.
    pub fn spawn(&mut self, world: WorldId, builder: EntityBuilder) -> Option<Entity> {
        let world = self.worlds.get_mut(&world)?;

        let id = world.next_id;
        world.next_id += 1;
        world.alive.insert(id);

        for (_, mut component) in builder.components {
            self.component_systems
                .iter(component.type_id())
                .for_each(|system| system.added(id, &mut component));
            world
                .component_systems
                .iter(component.type_id())
                .for_each(|system| system.added(id, &mut component));

            world.components.insert_any(id, component);
        }

        world.world_systems.iter().for_each(|system| {
            if let Some(query) = system.query()
                && !query.matches(&world.components, id)
            {
                return;
            }

            system.entity_spawned(world, id);
        });
        Some(id)
    }

    /// Will despawn the provided [`Entity`].
    ///
    /// Calls [`ComponentSystem::removed`] for every component still attached
    /// to the entity before the components are actually dropped.
    pub fn despawn(&mut self, world: WorldId, entity: Entity) {
        let Some(world) = self.worlds.get_mut(&world) else {
            return;
        };

        if !world.alive.remove(&entity) {
            // if the entity was not present, systems shouldn't be called
            return;
        }

        world.world_systems.iter().for_each(|system| {
            if let Some(query) = system.query()
                && !query.matches(&world.components, entity)
            {
                return;
            }

            system.entity_despawned(world, entity);
        });

        for (type_id, mut component) in world.components.remove_all(entity) {
            self.component_systems
                .iter(type_id)
                .for_each(|system| system.removed(entity, &mut component));
            world
                .component_systems
                .iter(type_id)
                .for_each(|system| system.removed(entity, &mut component));
        }
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
    pub fn insert<C: Component>(&mut self, world: WorldId, entity: Entity, component: C) {
        let Some(world) = self.worlds.get_mut(&world) else {
            return;
        };

        if !world.is_alive(entity) {
            return;
        }

        let mut component: Box<dyn AnyComponent> = Box::new(component);

        self.component_systems
            .iter(TypeId::of::<C>())
            .for_each(|system| system.added(entity, &mut component));
        world
            .component_systems
            .iter(TypeId::of::<C>())
            .for_each(|system| system.added(entity, &mut component));

        if world.components.contains(entity, component.type_id()) {
            self.component_systems
                .iter(TypeId::of::<C>())
                .for_each(|system| system.removed(entity, &mut component));
            world
                .component_systems
                .iter(TypeId::of::<C>())
                .for_each(|system| system.removed(entity, &mut component));
        }

        world.components.insert_any(entity, component);
    }

    /// Returns a shared reference to a component, or `None` if the entity
    /// does not have one.
    #[must_use]
    pub fn get<C: Component>(&self, world: WorldId, entity: Entity) -> Option<&C> {
        let world = self.worlds.get(&world)?;

        world.components.get(entity)
    }

    /// Returns a mutable reference to a component, or `None` if the entity
    /// does not have one.
    pub fn get_mut<C: Component>(&mut self, world: WorldId, entity: Entity) -> Option<&mut C> {
        let world = self.worlds.get_mut(&world)?;

        world.components.get_mut(entity)
    }

    /// Removes a single component from an entity, calling [`ComponentSystem::removed`]
    /// if the component was present.
    ///
    /// The entity itself is **not** despawned. If the component is not
    /// present this is a no-op.
    pub fn remove<C: Component>(&mut self, world: WorldId, entity: Entity) {
        let Some(world) = self.worlds.get_mut(&world) else {
            return;
        };

        if let Some(component) = world.components.remove::<C>(entity) {
            let mut component: Box<dyn AnyComponent> = Box::new(component);
            self.component_systems
                .iter(TypeId::of::<C>())
                .for_each(|system| system.removed(entity, &mut component));
            world
                .component_systems
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
