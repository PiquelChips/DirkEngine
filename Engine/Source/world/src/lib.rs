//! Entity-Component-System world for the game engine.
//!
//! All mutable world state lives in [`World`]. Entities are lightweight
//! numeric handles ([`Entity`]) and components are plain Rust structs stored in
//! per-type [`HashMap`]s inside [`Components`].
//!
//! # Quick-start
//!
//! ```
//! use world::{World, components::{Transform, Renderable}};
//! use glam::Vec3;
//!
//! let mut world = World::new(0);
//!
//! // Spawn an entity and attach components.
//! let player = world.spawn();
//! world.insert(player, Transform {
//!     location: Vec3::new(0.0, 1.0, 0.0),
//!     rotation: Vec3::ZERO,
//!     scale:    Vec3::ONE,
//! });
//! world.insert(player, Renderable { model: "meshes/player.glb".into() });
//!
//! // Query all entities that have both a Transform and a Renderable.
//! let renderables = world.query_double::<Transform, Renderable>();
//! assert_eq!(renderables, vec![player]);
//! ```
//!
//! # Adding a new component
//!
//! 1. Define a struct in the [`components`] module.
//! 2. Add its name to the [`define_components!`] invocation at the bottom of
//!    `lib.rs`.
//!
//! The macro generates all the boilerplate storage and [`Component`] trait
//! implementations automatically.

use std::collections::HashMap;

mod tests;

pub mod components;
use components::*;

/// A unique, opaque identifier for a spawned entity.
///
/// Entity IDs are never reused within a single [`World`] instance, so a stale
/// ID obtained before a [`World::despawn`] call will simply return `None` from
/// [`World::get`] after the entity is removed.
pub type Entity = u32;

/// An identifier that distinguishes multiple [`World`] instances from each other.
pub type WorldId = u32;

/// Marker trait implemented automatically by the [`define_components!`] macro.
///
/// Do **not** implement this trait manually; use the macro instead.
pub trait Component: 'static + Sized {
    /// Returns a shared reference to the per-type storage map.
    #[doc(hidden)]
    fn storage(components: &Components) -> &HashMap<Entity, Self>;
    /// Returns a mutable reference to the per-type storage map.
    #[doc(hidden)]
    fn storage_mut(components: &mut Components) -> &mut HashMap<Entity, Self>;
}

/// Declares the set of component types used by the engine.
///
/// Generates the [`Components`] aggregate struct, the `remove_all` helper, and
/// the [`Component`] trait impl for every listed type.
macro_rules! define_components {
    ( $( $C:ident ),* $(,)? ) => {

        /// Aggregate storage for all registered component types.
        ///
        /// Each field is a [`HashMap`] keyed by [`Entity`]. Fields are named
        /// after their component type. Access is mediated through [`World`];
        /// prefer the [`World::get`] / [`World::insert`] API over touching
        /// this struct directly.
        #[allow(non_snake_case)]
        #[derive(Debug, Default)]
        pub struct Components {
            $( $C: HashMap<Entity, $C>, )*
        }

        impl Components {
            /// Removes every component attached to `entity`.
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

/// Stores all entities and their components for a single game world.
///
/// `World` is the central data structure of the ECS. It owns every entity and
/// every component and is the sole entry-point for spawning, querying, and
/// mutating game state.
///
/// # Entity lifecycle
///
/// ```
/// use world::World;
///
/// let mut w = World::new(1);
/// let e = w.spawn();
/// assert!(w.alive().contains(&e));
///
/// w.despawn(e);
/// assert!(!w.alive().contains(&e));
/// ```
#[derive(Debug)]
pub struct World {
    id: WorldId,
    next_id: Entity,
    alive: Vec<Entity>,
    components: Components,
}

impl World {
    /// Creates a new, empty world with the given ID.
    ///
    /// The `id` is an arbitrary tag used to distinguish worlds when more than
    /// one exists simultaneously (e.g. a game world and a UI world).
    pub fn new(id: WorldId) -> Self {
        Self {
            id,
            next_id: 0,
            alive: Vec::new(),
            components: Components::default(),
        }
    }

    /// Returns this world's [`WorldId`].
    pub fn id(&self) -> WorldId {
        self.id
    }

    // -----------------------------------------------------------------------
    // Entity management
    // -----------------------------------------------------------------------

    /// Spawns a new entity and returns its unique [`Entity`] ID.
    ///
    /// The returned ID is stable for the lifetime of the world and is never
    /// reused, even after the entity is despawned.
    pub fn spawn(&mut self) -> Entity {
        let id = self.next_id;
        self.next_id += 1;
        self.alive.push(id);
        id
    }

    /// Despawns an entity, removing it and **all** of its components.
    ///
    /// If `entity` is not alive this is a no-op.
    pub fn despawn(&mut self, entity: Entity) {
        self.alive.retain(|&e| e != entity);
        self.components.remove_all(entity);
    }

    /// Returns a slice of all currently alive entity IDs in spawn order.
    pub fn alive(&self) -> &[Entity] {
        &self.alive
    }

    /// Returns the total number of alive entities.
    pub fn entity_count(&self) -> usize {
        self.alive.len()
    }

    // -----------------------------------------------------------------------
    // Component access
    // -----------------------------------------------------------------------

    /// Attaches `component` to `entity`, replacing any existing component of
    /// the same type.
    ///
    /// # Panics
    ///
    /// Does **not** panic if the entity is not alive — the component is stored
    /// regardless, but it will never be returned by queries. Callers should
    /// ensure the entity was obtained from [`World::spawn`] on this world.
    pub fn insert<C: Component>(&mut self, entity: Entity, component: C) {
        C::storage_mut(&mut self.components).insert(entity, component);
    }

    /// Returns a shared reference to a component, or `None` if the entity
    /// does not have one.
    pub fn get<C: Component>(&self, entity: Entity) -> Option<&C> {
        C::storage(&self.components).get(&entity)
    }

    /// Returns a mutable reference to a component, or `None` if the entity
    /// does not have one.
    pub fn get_mut<C: Component>(&mut self, entity: Entity) -> Option<&mut C> {
        C::storage_mut(&mut self.components).get_mut(&entity)
    }

    /// Removes a single component from an entity.
    ///
    /// The entity itself is **not** despawned. If the component is not
    /// present this is a no-op.
    pub fn remove<C: Component>(&mut self, entity: Entity) {
        C::storage_mut(&mut self.components).remove(&entity);
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// Returns all alive entities that have component `A`.
    ///
    /// # Examples
    /// ```
    /// # use world::{World, components::{Transform, Renderable}};
    /// # use glam::Vec3;
    /// # let mut w = World::new(0);
    /// # let e = w.spawn();
    /// # w.insert(e, Transform::default());
    /// let results = w.query_single::<Transform>();
    /// assert!(results.contains(&e));
    /// ```
    pub fn query_single<A: Component>(&self) -> Vec<Entity> {
        self.alive
            .iter()
            .filter(|&e| A::storage(&self.components).contains_key(e))
            .cloned()
            .collect()
    }

    /// Returns all alive entities that have **both** components `A` and `B`.
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

    /// Returns all alive entities that have **all three** components `A`, `B`,
    /// and `C`.
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

    /// Returns all alive entities that have **all four** components `A`, `B`,
    /// `C`, and `D`.
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
