use std::any::TypeId;

use crate::{Entity, Universe, World, components::AnyComponent, query::Query};

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
    fn created(&self, world: &World);
    /// Called as the world is being destroyed.
    /// In this state, the world is still valid
    /// and no entities have been removed.
    fn destroyed(&self, world: &World);

    /// Called on world tick
    fn tick(&self, world: &World, delta_time: f32);

    /// Called when an entity is spawned, with all
    /// the components it spawns with.
    fn entity_spawned(&self, world: &World, entity: Entity, components: Vec<Box<dyn AnyComponent>>);
    /// Called when an entity is despawned, with all
    /// the components it had with it.
    fn entity_despawned(
        &self,
        world: &World,
        entity: Entity,
        components: Vec<Box<dyn AnyComponent>>,
    );
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

macro_rules! component_system {
    ($kind:ident) => {
        pastey::paste! {
            use crate::components::[<$kind Component>];

            /// System run for every component of the specified type.
            /// Can be registered on both the [`World`] & [`Universe`]
            ///
            /// Each of the methods are optional.
            pub trait [<$kind ComponentSystem>]: System {
                /// The concrete component type this system handles.
                type Component: [<$kind Component>];

                /// When a component is added.
                /// `entity`: the entity with this component.
                fn added(&self, [<$kind:snake>]: $kind, component: &mut Self::Component);

                /// When a component is removed.
                /// `entity`: the entity with this component.
                fn removed(&self, [<$kind:snake>]: $kind, component: &mut Self::Component);
            }

            /// Private type-erasure trait for storage in [`World`] & [`Universe`]
            pub(crate) trait [<Any $kind ComponentSystem>] {
                fn type_id(&self) -> TypeId;
                fn added(&self, [<$kind:snake>]: $kind, component: Box<dyn AnyComponent>);
                fn removed(&self, [<$kind:snake>]: $kind, component: Box<dyn AnyComponent>);
            }

            impl<T: [<$kind ComponentSystem>]> [<Any $kind ComponentSystem>] for T {
                fn type_id(&self) -> TypeId {
                    TypeId::of::<T::Component>()
                }

                fn added(&self, obj: $kind, component: Box<dyn AnyComponent>) {
                    if let Ok(mut component) = component.as_any_box().downcast::<T::Component>() {
                        [<$kind ComponentSystem>]::added(self, obj, &mut component);
                    }
                }

                fn removed(&self, obj: $kind, component: Box<dyn AnyComponent>) {
                    if let Ok(mut component) = component.as_any_box().downcast::<T::Component>() {
                        [<$kind ComponentSystem>]::removed(self, obj, &mut component);
                    }
                }
            }
        }
    };
}

component_system!(Entity);
component_system!(World);
