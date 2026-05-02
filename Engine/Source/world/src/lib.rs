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
//! let mut event_manager = events::EventManager::new();
//!
//! let mut world = World::new(0, &mut event_manager);
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

use std::any::{Any, TypeId};
use std::collections::HashMap;

mod tests;

pub mod components;
pub mod events;
pub mod player;
use crate::events::WorldEvent;

/// A unique, opaque identifier for a spawned entity.
///
/// Entity IDs are never reused within a single [`World`] instance, so a stale
/// ID obtained before a [`World::despawn`] call will simply return `None` from
/// [`World::get`] after the entity is removed.
pub type Entity = u32;

/// An identifier that distinguishes multiple [`World`] instances from each other.
pub type WorldId = u32;

/// Marker trait for component types.
///
/// Implement this in **any** crate — no central registration or macro needed.
///
/// ```rust
/// use world::Component;
///
/// #[derive(Component)]
/// struct Health(f32);
/// ```
pub trait Component: 'static + Sized {}
#[doc(hidden)]
pub use macros::Component;

/// Type-erased storage for a single component type.
///
/// The `as_any` / `as_any_mut` pattern lets us downcast back to the concrete
/// `TypedStorage<C>` without exposing `C` through the trait object.
trait AnyStorage {
    /// Remove the component for `entity` if present.
    fn remove(&mut self, entity: Entity);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

struct TypedStorage<C: Component> {
    map: HashMap<Entity, C>,
}

impl<C: Component> AnyStorage for TypedStorage<C> {
    fn remove(&mut self, entity: Entity) {
        self.map.remove(&entity);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Dynamic, open-ended component storage.
///
/// Each component type gets its own `HashMap<Entity, C>`, looked up by
/// [`TypeId`].  No central registration is required — storage for a type is
/// created on the first `insert` and lives until the `World` is dropped.
#[derive(Default)]
struct Components {
    storages: HashMap<TypeId, Box<dyn AnyStorage>>,
}

impl std::fmt::Debug for Components {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Components {{ {} type(s) }}", self.storages.len())
    }
}

impl Components {
    /// Returns a shared reference to the typed storage bucket for `C`,
    /// or `None` if no component of that type has ever been inserted.
    fn typed<C: Component>(&self) -> Option<&TypedStorage<C>> {
        self.storages
            .get(&TypeId::of::<C>())
            .and_then(|b| b.as_any().downcast_ref::<TypedStorage<C>>())
    }

    /// Returns a mutable reference to the typed storage bucket for `C`,
    /// creating an empty one if it does not yet exist.
    fn typed_mut<C: Component>(&mut self) -> &mut TypedStorage<C> {
        self.storages
            .entry(TypeId::of::<C>())
            .or_insert_with(|| {
                Box::new(TypedStorage::<C> {
                    map: HashMap::new(),
                })
            })
            .as_any_mut()
            .downcast_mut::<TypedStorage<C>>()
            .expect("we just inserted exactly this type")
    }

    fn insert<C: Component>(&mut self, entity: Entity, component: C) {
        self.typed_mut::<C>().map.insert(entity, component);
    }

    fn get<C: Component>(&self, entity: Entity) -> Option<&C> {
        self.typed::<C>()?.map.get(&entity)
    }

    fn get_mut<C: Component>(&mut self, entity: Entity) -> Option<&mut C> {
        self.typed_mut::<C>().map.get_mut(&entity)
    }

    fn remove<C: Component>(&mut self, entity: Entity) {
        if let Some(storage) = self.storages.get_mut(&TypeId::of::<C>()) {
            storage.remove(entity);
        }
    }

    /// Removes **every** component attached to `entity` across all types.
    fn remove_all(&mut self, entity: Entity) {
        for storage in self.storages.values_mut() {
            storage.remove(entity);
        }
    }

    fn contains<C: Component>(&self, entity: Entity) -> bool {
        self.typed::<C>()
            .is_some_and(|s| s.map.contains_key(&entity))
    }
}

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
/// let mut event_manager = events::EventManager::new();
///
/// let mut w = World::new(1, &mut event_manager);
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
    dispatcher: ::events::Dispatcher<WorldEvent>,
}

impl World {
    /// Creates a new, empty world with the given ID.
    ///
    /// The `id` is an arbitrary tag used to distinguish worlds when more than
    /// one exists simultaneously (e.g. a game world and a UI world).
    #[must_use]
    pub fn new(id: WorldId, event_manager: &::events::EventManager) -> Self {
        let dispatcher = event_manager.register();
        dispatcher.dispatch(WorldEvent::Created(id));
        Self {
            id,
            next_id: 0,
            alive: Vec::new(),
            components: Components::default(),
            dispatcher,
        }
    }

    /// Returns this world's [`WorldId`].
    #[must_use]
    pub fn id(&self) -> WorldId {
        self.id
    }

    /// Spawns a new entity and returns its unique [`Entity`] ID.
    ///
    /// The returned ID is stable for the lifetime of the world and is never
    /// reused, even after the entity is despawned.
    pub fn spawn(&mut self) -> Entity {
        let id = self.next_id;
        self.next_id += 1;
        self.alive.push(id);
        self.dispatcher.dispatch(WorldEvent::EntitySpawn {
            world: self.id,
            entity: id,
        });
        id
    }

    /// Despawns an entity, removing it and **all** of its components.
    ///
    /// If `entity` is not alive this is a no-op.
    pub fn despawn(&mut self, entity: Entity) {
        self.alive.retain(|&e| e != entity);
        self.components.remove_all(entity);

        self.dispatcher.dispatch(WorldEvent::EntityDespawn {
            world: self.id,
            entity,
        });
    }

    /// Returns a slice of all currently alive entity IDs in spawn order.
    #[must_use]
    pub fn alive(&self) -> &[Entity] {
        &self.alive
    }

    /// Returns the total number of alive entities.
    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.alive.len()
    }

    /// Returns if the specified entity is alive
    #[must_use]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.alive.contains(&entity)
    }

    /// Attaches `component` to `entity`, replacing any existing component of
    /// the same type.
    ///
    /// # Panics
    ///
    /// Does **not** panic if the entity is not alive — the component is stored
    /// regardless, but it will never be returned by queries. Callers should
    /// ensure the entity was obtained from [`World::spawn`] on this world.
    pub fn insert<C: Component>(&mut self, entity: Entity, component: C) {
        self.components.insert(entity, component);
        self.dispatcher.dispatch(WorldEvent::EntityUpdate {
            world: self.id,
            entity,
        });
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
        self.dispatcher.dispatch(WorldEvent::EntityUpdate {
            world: self.id,
            entity,
        });
        self.components.get_mut(entity)
    }

    /// Removes a single component from an entity.
    ///
    /// The entity itself is **not** despawned. If the component is not
    /// present this is a no-op.
    pub fn remove<C: Component>(&mut self, entity: Entity) {
        self.components.remove::<C>(entity);
        self.dispatcher.dispatch(WorldEvent::EntityUpdate {
            world: self.id,
            entity,
        });
    }

    /// Returns all alive entities that have component `A`.
    ///
    /// # Examples
    /// ```
    /// # use world::{World, components::{Transform, Renderable}};
    /// # use glam::Vec3;
    /// # let mut event_manager = events::EventManager::new();
    /// # let mut w = World::new(0, &mut event_manager);
    /// # let e = w.spawn();
    /// # w.insert(e, Transform::default());
    /// let results = w.query_single::<Transform>();
    /// assert!(results.contains(&e));
    /// ```
    #[must_use]
    pub fn query_single<A: Component>(&self) -> Vec<Entity> {
        self.alive
            .iter()
            .filter(|&&e| self.components.contains::<A>(e))
            .copied()
            .collect()
    }

    /// Returns all alive entities that have **both** components `A` and `B`.
    #[must_use]
    pub fn query_double<A: Component, B: Component>(&self) -> Vec<Entity> {
        self.alive
            .iter()
            .filter(|&&e| self.components.contains::<A>(e) && self.components.contains::<B>(e))
            .copied()
            .collect()
    }

    /// Returns all alive entities that have **all three** components `A`, `B`,
    /// and `C`.
    #[must_use]
    pub fn query_triple<A: Component, B: Component, C: Component>(&self) -> Vec<Entity> {
        self.alive
            .iter()
            .filter(|&&e| {
                self.components.contains::<A>(e)
                    && self.components.contains::<B>(e)
                    && self.components.contains::<C>(e)
            })
            .copied()
            .collect()
    }

    /// Returns all alive entities that have **all four** components `A`, `B`,
    /// `C`, and `D`.
    #[must_use]
    pub fn query_quadruple<A: Component, B: Component, C: Component, D: Component>(
        &self,
    ) -> Vec<Entity> {
        self.alive
            .iter()
            .filter(|&&e| {
                self.components.contains::<A>(e)
                    && self.components.contains::<B>(e)
                    && self.components.contains::<C>(e)
                    && self.components.contains::<D>(e)
            })
            .copied()
            .collect()
    }
}

impl Drop for World {
    fn drop(&mut self) {
        self.dispatcher.dispatch(WorldEvent::Destroyed(self.id));
    }
}
