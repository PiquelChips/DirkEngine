//! This crate has all the traits for the ECS [`System`]s.
use std::{any::TypeId, collections::HashMap};

use crate::{
    Entity, Universe, WorldId,
    components::{AnyComponent, Component},
    query::Query,
};
use macros::system_trait;

/// All systems must implement this trait.
pub trait System: 'static {
    /// Get a name for the system. For debug purposes only.
    fn name() -> &'static str;
}
#[doc(hidden)]
pub use macros::System;

/// A system that is run by the [`Universe`].
#[system_trait]
pub trait UniverseSystem: System {
    /// Called right after the world is created.
    fn world_created(&self, universe: &Universe, world: WorldId);
    /// Called as the world is being destroyed.
    /// In this state, the world is still valid
    /// and no entities have been removed.
    fn world_destroyed(&self, universe: &Universe, world: WorldId);

    /// Called when an entity is spawned. At this point, components have
    /// already been added. They can thus be queried for.
    fn entity_spawned(&self, universe: &Universe, entity: Entity);
    /// Called when an entity is despawned. At this point, components have
    /// not yet been removed. They can thus be queried for.
    /// However, the entity has been removed from the [`World`], so
    /// querying for it will not work.
    fn entity_despawned(&self, universe: &Universe, entity: Entity);

    /// This function will be called by the [`Universe`] on every tick.
    fn tick(&self, universe: &Universe, delta_time: f32);
}

/// A [`System`] that is run on every entity that matches the query.
#[system_trait]
pub trait EntitySystem: System {
    /// Called when an entity is spawned. At this point, components have
    /// already been added. They can thus be queried for.
    fn spawned(&self, universe: &Universe, entity: Entity);
    /// Called when an entity is despawned. At this point, components have
    /// not yet been removed. They can thus be queried for.
    /// However, the entity has been removed from the [`World`], so
    /// querying for it will not work.
    fn despawned(&self, universe: &Universe, entity: Entity);

    /// Called when the entity is moved to another [`World`].
    fn sent(&self, universe: &Universe, entity: Entity, old: WorldId, new: WorldId);

    /// This query will decide if `entity_spawned` & `entity_despawned` should
    /// be run for given entities. If there is not query, the system will run
    /// on every entity.
    fn query(&self) -> Option<Query>;
}

/// Run for [`Entity`]s that match the query
#[system_trait]
pub trait TickingSystem: System {
    /// `world`: the current world we are ticking. This system would tick multiple
    /// time per frame, just on multiple different worlds.
    /// `entities`: the list of entities that were returned by the query returned
    /// by [`TickingSystem::query`].
    fn tick(&self, universe: &Universe, delta_time: f32, entities: Vec<Entity>);
    /// Returns the query used to construct the `entities` of the tick function.
    fn query(&self) -> Query;
}

/// System run for every component of the specified type.
/// Can be registered on both the [`Entity`] & [`Universe`]
///
/// Each of the methods are optional.
pub trait ComponentSystem: System {
    /// The concrete component type this system handles.
    type Component: Component;

    /// When a component is added.
    /// `entity`: the entity with this component.
    fn added(&self, entity: Entity, component: &mut Self::Component);

    /// When a component is removed.
    /// `entity`: the entity with this component.
    fn removed(&self, entity: Entity, component: &mut Self::Component);
}

/// Private type-erasure trait for storage in [`Entity`] & [`Universe`]
pub(crate) trait AnyComponentSystem {
    fn type_id(&self) -> TypeId;
    fn added(&self, entity: Entity, component: &mut Box<dyn AnyComponent>);
    fn removed(&self, entity: Entity, component: &mut Box<dyn AnyComponent>);
}

impl<T: ComponentSystem> AnyComponentSystem for T {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T::Component>()
    }

    fn added(&self, entity: Entity, component: &mut Box<dyn AnyComponent>) {
        if let Some(component) = component.as_any_mut().downcast_mut::<T::Component>() {
            T::added(self, entity, component);
        }
    }

    fn removed(&self, entity: Entity, component: &mut Box<dyn AnyComponent>) {
        if let Some(component) = component.as_any_mut().downcast_mut::<T::Component>() {
            T::removed(self, entity, component);
        }
    }
}

#[derive(Default)]
pub(crate) struct ComponentSystemStorage {
    systems: HashMap<TypeId, Vec<Box<dyn AnyComponentSystem>>>,
}

impl ComponentSystemStorage {
    pub fn insert<S: ComponentSystem>(&mut self, system: S) {
        let systems = self
            .systems
            .entry(AnyComponentSystem::type_id(&system))
            .or_default();

        systems.push(Box::new(system));
    }

    pub fn iter(&mut self, type_id: TypeId) -> std::slice::Iter<'_, Box<dyn AnyComponentSystem>> {
        self.systems.entry(type_id).or_default().iter()
    }
}
