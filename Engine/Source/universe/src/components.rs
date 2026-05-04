//! This module has all the traits & structs required for component storage
//! and creation.
//!
//! Implement the [`WorldComponent`] or [`EntityComponent`] traits to get
//! started (you can do this via a derive macro).
use crate::Entity;
use serde::{Serialize, de::DeserializeOwned};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::Debug,
    ops::{Index, IndexMut},
};

/// Base marker trait for component types.
pub trait Component: 'static + Sized + Debug + Serialize + DeserializeOwned {}
#[doc(hidden)]
pub use macros::Component;

/// Extends [`Component`] trait for world specific behavior
pub trait WorldComponent: Component {}
#[doc(hidden)]
pub use macros::WorldComponent;

/// Extends [`Component`] trait for entity specific behavior
pub trait EntityComponent: Component {}
#[doc(hidden)]
pub use macros::EntityComponent;

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

/// Type-erased storage for a single component type.
///
/// The `as_any` / `as_any_mut` pattern lets us downcast back to the concrete
/// `EntityComponentStorage<C>` without exposing `C` through the trait object.
trait AnyStorage {
    /// Remove the component for `entity` if present.
    fn remove(&mut self, entity: Entity);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

struct EntityComponentStorage<C: EntityComponent> {
    map: HashMap<Entity, C>,
}

impl<C: EntityComponent> EntityComponentStorage<C> {
    fn get(&self, entity: &Entity) -> Option<&C> {
        self.map.get(entity)
    }
    fn get_mut(&mut self, entity: &Entity) -> Option<&mut C> {
        self.map.get_mut(entity)
    }
    fn insert(&mut self, entity: Entity, component: C) -> Option<C> {
        self.map.insert(entity, component)
    }
}

impl<C: EntityComponent> AnyStorage for EntityComponentStorage<C> {
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

impl<C: EntityComponent> IndexMut<Entity> for EntityComponentStorage<C> {
    fn index_mut(&mut self, index: Entity) -> &mut Self::Output {
        self.map
            .get_mut(&index)
            .expect("entity should have component if indexing")
    }
}

impl<C: EntityComponent> Index<Entity> for EntityComponentStorage<C> {
    type Output = C;
    fn index(&self, index: Entity) -> &Self::Output {
        &self.map[&index]
    }
}

/// Dynamic, open-ended component storage.
///
/// Each component type gets its own `HashMap<Entity, C>`, looked up by
/// [`TypeId`].  No central registration is required — storage for a type is
/// created on the first `insert` and lives until the `World` is dropped.
#[derive(Default)]
pub(crate) struct EntityComponents {
    storages: HashMap<TypeId, Box<dyn AnyStorage>>,
}

impl std::fmt::Debug for EntityComponents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EntityComponents {{ {} type(s) }}", self.storages.len())
    }
}

impl EntityComponents {
    /// Returns a shared reference to the typed storage bucket for `C`,
    /// or `None` if no component of that type has ever been inserted.
    fn typed<C: EntityComponent>(&self) -> Option<&EntityComponentStorage<C>> {
        self.storages
            .get(&TypeId::of::<C>())
            .and_then(|b| b.as_any().downcast_ref::<EntityComponentStorage<C>>())
    }

    /// Returns a mutable reference to the typed storage bucket for `C`,
    /// creating an empty one if it does not yet exist.
    fn typed_mut<C: EntityComponent>(&mut self) -> Option<&mut EntityComponentStorage<C>> {
        self.storages
            .get_mut(&TypeId::of::<C>())
            .and_then(|b| b.as_any_mut().downcast_mut::<EntityComponentStorage<C>>())
    }

    pub fn insert<C: EntityComponent>(&mut self, entity: Entity, component: C) {
        self.storages
            .entry(TypeId::of::<C>())
            .or_insert_with(|| {
                Box::new(EntityComponentStorage::<C> {
                    map: HashMap::new(),
                })
            })
            .as_any_mut()
            .downcast_mut::<EntityComponentStorage<C>>()
            .expect("we just inserted exactly this type")
            .insert(entity, component);
    }

    pub fn get<C: EntityComponent>(&self, entity: Entity) -> Option<&C> {
        self.typed::<C>()?.get(&entity)
    }

    pub fn get_mut<C: EntityComponent>(&mut self, entity: Entity) -> Option<&mut C> {
        self.typed_mut::<C>()?.get_mut(&entity)
    }

    pub fn remove<C: EntityComponent>(&mut self, entity: Entity) {
        if let Some(storage) = self.storages.get_mut(&TypeId::of::<C>()) {
            storage.remove(entity);
        }
    }

    /// Removes **every** component attached to `entity` across all types.
    pub fn remove_all(&mut self, entity: Entity) {
        for storage in self.storages.values_mut() {
            storage.remove(entity);
        }
    }

    pub fn contains<C: EntityComponent>(&self, entity: Entity) -> bool {
        self.typed::<C>()
            .is_some_and(|s| s.map.contains_key(&entity))
    }
}
