//! This crate handles the world and the ECS that it runs.
//! All world state is managed by the World struct.
//!
//! To add a component, add a struct to the [components] module
//! and add it inside the [define_components] macro. You can
//! then use it like any other component.

use std::collections::HashMap;

pub mod components;
use components::*;

/// An Entity is nothing more than a unique numeric identifier.
pub type Entity = u32;
/// An identifier used to find the world.
pub type WorldId = u32;

/// Component trait. Should not be implemented manually.
/// Should be implemented by the [define_components] macro.
/// NOT MANUALLY
pub trait Component: 'static + Sized {
    fn storage(components: &Components) -> &HashMap<Entity, Self>;
    fn storage_mut(components: &mut Components) -> &mut HashMap<Entity, Self>;
}

/// Collects all the structs that will be used as components. Will
/// create the [Components] struct that is used to store the components
/// for entities.
macro_rules! define_components {
    ( $( $C:ident ),* $(,)? ) => {

        #[allow(non_snake_case)]
        #[derive(Default)]
        pub struct Components {
            $( $C: HashMap<Entity, $C>, )*
        }

        impl Components {
            fn remove_all(&mut self, entity: Entity) {
                $( self.$C.remove(&entity); )*
            }
        }

        $(
            impl Component for $C {
                fn storage(components: &Components) -> &HashMap<Entity, Self> {
                    &components.$C
                }
                fn storage_mut(components: &mut Components) -> &mut HashMap<Entity, Self> {
                    &mut components.$C
                }
            }
        )*
    };
}

define_components!(Transform, Renderable, Camera);

/// Stores all the entities and their components. Handles state
/// of all the entities in the world.
pub struct World {
    id: WorldId,
    next_id: Entity,
    alive: Vec<Entity>,
    components: Components,
}

impl World {
    /// Creates a new empty world.
    pub fn new(id: WorldId) -> Self {
        Self {
            id,
            next_id: 0,
            alive: Vec::new(),
            components: Components::default(),
        }
    }
    pub fn id(&self) -> WorldId {
        self.id
    }

    /// Spawn a new entity and return its ID.
    pub fn spawn(&mut self) -> Entity {
        let id = self.next_id;
        self.next_id += 1;
        self.alive.push(id);
        id
    }
    /// Despawn an entity - removes it and all its components.
    pub fn despawn(&mut self, entity: Entity) {
        self.alive.retain(|&e| e != entity);
        self.components.remove_all(entity);
    }
    /// Returns all the spawned actors
    pub fn alive(&self) -> &[Entity] {
        &self.alive
    }

    /// Attach (or overwrite) a component on an entity.
    pub fn insert<C: Component>(&mut self, entity: Entity, component: C) {
        C::storage_mut(&mut self.components).insert(entity, component);
    }
    /// Shared reference to a component; `None` if the entity lacks it.
    pub fn get<C: Component>(&self, entity: Entity) -> Option<&C> {
        C::storage(&self.components).get(&entity)
    }
    /// Mutable reference to a component; `None` if the entity lacks it.
    pub fn get_mut<C: Component>(&mut self, entity: Entity) -> Option<&mut C> {
        C::storage_mut(&mut self.components).get_mut(&entity)
    }
    /// Detach a single component from an entity (entity itself survives).
    pub fn remove<C: Component>(&mut self, entity: Entity) {
        C::storage_mut(&mut self.components).remove(&entity);
    }

    /// Queries all the components that have the specified component.
    pub fn query_single<A: Component>(&self) -> Vec<Entity> {
        self.alive
            .iter()
            .filter(|&e| A::storage(&self.components).contains_key(e))
            .cloned()
            .collect()
    }
    /// Queries all the components that have the two specified components.
    pub fn query_double<A: Component, B: Component>(&self) -> Vec<Entity> {
        self.alive
            .iter()
            .filter(|&e| {
                A::storage(&self.components).contains_key(e)
                    && B::storage(&self.components).contains_key(e)
            })
            .cloned()
            .collect()
    }
    /// Queries all the components that have the three specified components.
    pub fn query_triple<A: Component, B: Component, C: Component>(&self) -> Vec<Entity> {
        self.alive
            .iter()
            .filter(|&e| {
                A::storage(&self.components).contains_key(e)
                    && B::storage(&self.components).contains_key(e)
                    && C::storage(&self.components).contains_key(e)
            })
            .cloned()
            .collect()
    }
    /// Queries all the components that have the four specified components.
    pub fn query_quadruple<A: Component, B: Component, C: Component, D: Component>(
        &self,
    ) -> Vec<Entity> {
        self.alive
            .iter()
            .filter(|&e| {
                A::storage(&self.components).contains_key(e)
                    && B::storage(&self.components).contains_key(e)
                    && C::storage(&self.components).contains_key(e)
                    && D::storage(&self.components).contains_key(e)
            })
            .cloned()
            .collect()
    }
}
