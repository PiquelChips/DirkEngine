use std::{
    any::{Any, TypeId},
    fmt::Debug,
};

use crate::{Entity, Universe, World, components::Component, query::Query};

/// A dyn-compatible wrapper around Component, used wherever
/// type-erased component values must be passed around at runtime.
#[doc(hidden)]
pub trait AnyComponent: Any + Debug + 'static {
    /// Converts the box into `Box<dyn Any>` so it can be downcast.
    fn as_any_box(self: Box<Self>) -> Box<dyn Any>;
}

// Blanket impl: every concrete Component automatically becomes an AnyComponent.
impl<T: Component> AnyComponent for T {
    fn as_any_box(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

/// All systems must implement this trait.
pub trait System: Clone {
    /// Get a name for the system. For debug purposes only.
    fn name() -> String;
}

/// A system that is run for the entire universe on every tick.
pub trait UniverseSystem: System {
    /// This function will be called by the universe on every tick.
    fn tick(&self, delta_time: f32, universe: &Universe);
}

/// A system that is run for the entire world.
///
/// Each of the methods are optional.
pub trait WorldSystem: System {
    /// Called right after the world is created.
    fn world_created(&self, world: &World) {}
    /// Called as the world is being destroyed.
    /// In this state, the world is still valid
    /// and no entities have been removed.
    fn world_destroyed(&self, world: &World) {}

    /// Called on world tick
    fn tick(&self, world: &World, delta_time: f32) {}

    /// Called when an entity is spawned, with all
    /// the components it spawns with.
    fn entity_spawned(
        &self,
        world: &World,
        entity: Entity,
        components: Vec<Box<dyn AnyComponent>>,
    ) {
    }
}

/// Run on a specific World for components that match the query
pub trait TickingSystem: System {
    /// `world`: the current world we are ticking. This system would tick multiple
    /// time per frame, just on multiple different worlds.
    /// `entities`: the list of entities that were returned by the query returned
    /// by [`TickingSystem::query`].
    fn tick(&self, deta_time: f32, world: &World, entities: Vec<Entity>);
    /// Returns the query used to construct the `entities` of the tick function.
    fn query() -> Query;
}

/// System run for every component of the specified type.
/// Can be registered on both the [`World`] & [`Universe`]
///
/// Each of the methods are optional.
pub trait ComponentSystem: System {
    /// The concrete component type this system handles.
    type C: Component;

    /// When a component is added.
    /// `entity`: the entity with this component.
    fn added(&self, entity: Entity, component: &mut Self::C) {}
    /// When a component is updated.
    /// `entity`: the entity with this component.
    fn updated(&self, entity: Entity, component: &mut Self::C) {}
    /// When a component is removed.
    /// `entity`: the entity with this component.
    fn removed(&self, entity: Entity, component: &mut Self::C) {}
}

/// Private type-erasure trait for storage in [`World`] & [`Universe`]
trait AnyComponentSystem {
    fn type_id(&self) -> TypeId;
    // Uses Box<dyn AnyComponent> — dyn-compatible and downcasting-capable
    fn added(&self, entity: Entity, component: Box<dyn AnyComponent>);
    fn removed(&self, entity: Entity, component: Box<dyn AnyComponent>);
}

impl<T: ComponentSystem> AnyComponentSystem for T {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T::C>()
    }

    fn added(&self, entity: Entity, component: Box<dyn AnyComponent>) {
        // as_any_box() → Box<dyn Any> → downcast to the concrete type
        if let Ok(mut component) = component.as_any_box().downcast::<T::C>() {
            ComponentSystem::added(self, entity, &mut component);
        }
    }

    fn removed(&self, entity: Entity, component: Box<dyn AnyComponent>) {
        if let Ok(mut component) = component.as_any_box().downcast::<T::C>() {
            ComponentSystem::removed(self, entity, &mut component);
        }
    }
}
