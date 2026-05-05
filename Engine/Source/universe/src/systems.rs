//! This crate has all the traits for the ECS [`System`]s.
use std::{any::TypeId, collections::HashMap};

use crate::{
    Entity, Universe, World,
    components::{AnyComponent, Component},
    query::Query,
};
use macros::system;

/// All systems must implement this trait.
pub trait System: Clone + 'static {
    /// Get a name for the system. For debug purposes only.
    fn name() -> String;
}

/// A system that is run by the [`Universe`].
#[system]
pub trait UniverseSystem: System {
    /// Called right after the world is created.
    fn world_created(&self, world: &World);
    /// Called as the world is being destroyed.
    /// In this state, the world is still valid
    /// and no entities have been removed.
    fn world_destroyed(&self, world: &World);
    /// This function will be called by the universe on every tick.
    fn tick(&self, delta_time: f32, universe: &Universe);
}

/// A system that is run for the entire [`World`].
#[system]
pub trait WorldSystem: System {
    /// Called on world tick
    fn tick(&self, world: &World, delta_time: f32);

    /// Called when an entity is spawned. At this point, components have
    /// already been added. They can thus be queried for.
    fn entity_spawned(&self, world: &World, entity: Entity);
    /// Called when an entity is despawned. At this point, components have
    /// not yet been removed. They can thus be queried for.
    fn entity_despawned(&self, world: &World, entity: Entity);
}

/// Run on a specific World for components that match the query
#[system]
pub trait TickingSystem: System {
    /// `world`: the current world we are ticking. This system would tick multiple
    /// time per frame, just on multiple different worlds.
    /// `entities`: the list of entities that were returned by the query returned
    /// by [`TickingSystem::query`].
    fn tick(&self, delta_time: f32, world: &World, entities: Vec<Entity>);
    /// Returns the query used to construct the `entities` of the tick function.
    fn query(&self) -> Query;

    /// This is an outer tick function that should actually be called.
    /// [`TickingSystem::tick`] should only be implemented, never called.
    ///
    /// This function should not be implemented by users.
    fn outer_tick(&self, delta_time: f32, world: &World) {
        self.tick(delta_time, world, world.query(&self.query()));
    }
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
    fn type_id() -> TypeId;
    fn added(&self, entity: Entity, component: Box<dyn AnyComponent>);
    fn removed(&self, entity: Entity, component: Box<dyn AnyComponent>);
}

impl<T: ComponentSystem> AnyComponentSystem for T {
    fn type_id() -> TypeId {
        TypeId::of::<T::Component>()
    }

    fn added(&self, entity: Entity, component: Box<dyn AnyComponent>) {
        if let Ok(mut component) = component.as_any_box().downcast::<T::Component>() {
            T::added(self, entity, &mut component);
        }
    }

    fn removed(&self, entity: Entity, component: Box<dyn AnyComponent>) {
        if let Ok(mut component) = component.as_any_box().downcast::<T::Component>() {
            T::removed(self, entity, &mut component);
        }
    }
}

// TODO: EntityComponentSystemStorage
