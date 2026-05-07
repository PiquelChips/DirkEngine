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

/// A dyn-compatible wrapper around Component, used wherever
/// type-erased component values must be passed around at runtime.
#[doc(hidden)]
pub(crate) trait AnyComponent: Any + Debug + 'static {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// Consume this boxed value and return it as a plain `Box<dyn Any>`,
    /// enabling `Box::downcast::<C>()` at call sites.
    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    fn new_storage(&self) -> Box<dyn AnyStorage>;
}

// Blanket impl: every concrete Component automatically becomes an AnyComponent.
impl<T: Component> AnyComponent for T {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn new_storage(&self) -> Box<dyn AnyStorage> {
        Box::new(ComponentStorage::<T>::default())
    }
}

/// Type-erased storage for a single component type.
///
/// The `as_any` / `as_any_mut` pattern lets us downcast back to the concrete
/// `EntityComponentStorage<C>` without exposing `C` through the trait object.
pub(crate) trait AnyStorage {
    /// Remove the component for `entity` and return it as a type-erased box,
    /// or `None` if the entity had no component in this storage.
    fn remove(&mut self, entity: Entity) -> Option<Box<dyn AnyComponent>>;
    /// Insert a type-erased component for `entity`.
    ///
    /// The concrete type inside `component` must match the type this storage
    /// was created for; mismatches are silently ignored.
    fn insert(&mut self, entity: Entity, component: Box<dyn AnyComponent>);
    /// Returns `true` if this storage holds a component for `entity`.
    fn contains(&self, entity: Entity) -> bool;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

struct ComponentStorage<C: Component> {
    map: HashMap<Entity, C>,
}

impl<C: Component> Default for ComponentStorage<C> {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl<C: Component> ComponentStorage<C> {
    fn get(&self, entity: Entity) -> Option<&C> {
        self.map.get(&entity)
    }
    fn get_mut(&mut self, entity: Entity) -> Option<&mut C> {
        self.map.get_mut(&entity)
    }
    fn insert(&mut self, entity: Entity, component: C) -> Option<C> {
        self.map.insert(entity, component)
    }
}

impl<C: Component> AnyStorage for ComponentStorage<C> {
    fn remove(&mut self, entity: Entity) -> Option<Box<dyn AnyComponent>> {
        self.map
            .remove(&entity)
            .map(|c| Box::new(c) as Box<dyn AnyComponent>)
    }
    fn insert(&mut self, entity: Entity, component: Box<dyn AnyComponent>) {
        // Downcast through Box<dyn Any> so we can move the value out of the box.
        match component.into_any().downcast::<C>() {
            Ok(c) => {
                self.map.insert(entity, *c);
            }
            Err(_) => {
                // Type mismatch — should never happen in practice because
                // insert_box always routes to the storage that matches C.
                debug_assert!(false, "insert_any called with wrong component type");
            }
        }
    }

    fn contains(&self, entity: Entity) -> bool {
        self.map.contains_key(&entity)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl<C: Component> IndexMut<Entity> for ComponentStorage<C> {
    fn index_mut(&mut self, index: Entity) -> &mut Self::Output {
        self.map
            .get_mut(&index)
            .expect("entity should have component if indexing")
    }
}

impl<C: Component> Index<Entity> for ComponentStorage<C> {
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
pub(crate) struct Components {
    storages: HashMap<TypeId, Box<dyn AnyStorage>>,
}

impl std::fmt::Debug for Components {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EntityComponents {{ {} type(s) }}", self.storages.len())
    }
}

impl Components {
    /// Returns a shared reference to the typed storage bucket for `C`,
    /// or `None` if no component of that type has ever been inserted.
    fn typed<C: Component>(&self) -> Option<&ComponentStorage<C>> {
        self.storages
            .get(&TypeId::of::<C>())
            .and_then(|b| b.as_any().downcast_ref::<ComponentStorage<C>>())
    }

    /// Returns a mutable reference to the typed storage bucket for `C`,
    /// creating an empty one if it does not yet exist.
    fn typed_mut<C: Component>(&mut self) -> Option<&mut ComponentStorage<C>> {
        self.storages
            .get_mut(&TypeId::of::<C>())
            .and_then(|b| b.as_any_mut().downcast_mut::<ComponentStorage<C>>())
    }

    #[allow(unused)]
    pub fn insert<C: Component>(&mut self, entity: Entity, component: C) {
        self.storages
            .entry(TypeId::of::<C>())
            .or_insert_with(|| Box::new(ComponentStorage::<C>::default()))
            .as_any_mut()
            .downcast_mut::<ComponentStorage<C>>()
            .expect("we just inserted exactly this type")
            .insert(entity, component);
    }

    /// Insert a type-erased component.
    ///
    /// Storage for the component's concrete type is created on demand using
    /// [`AnyComponent::new_storage`].  The `TypeId` used as the key comes from
    /// `Any::type_id`, which dispatches to the concrete type at runtime.
    pub fn insert_any(&mut self, entity: Entity, component: Box<dyn AnyComponent>) {
        // Obtain the concrete TypeId via dynamic dispatch before the box is
        // consumed, then ensure a matching storage bucket exists.
        let type_id = (*component).type_id();

        self.storages
            .entry(type_id)
            .or_insert_with(|| component.new_storage())
            .insert(entity, component);
    }

    pub fn get<C: Component>(&self, entity: Entity) -> Option<&C> {
        self.typed::<C>()?.get(entity)
    }

    pub fn get_mut<C: Component>(&mut self, entity: Entity) -> Option<&mut C> {
        self.typed_mut::<C>()?.get_mut(entity)
    }

    /// Remove a single typed component from `entity`, returning it if present.
    pub fn remove<C: Component>(&mut self, entity: Entity) -> Option<C> {
        self.storages
            .get_mut(&TypeId::of::<C>())
            .and_then(|b| b.as_any_mut().downcast_mut::<ComponentStorage<C>>())
            .and_then(|s| s.map.remove(&entity))
    }

    /// Remove **every** component attached to `entity` across all types,
    /// returning them so callers can invoke lifecycle hooks before dropping.
    pub fn remove_all(&mut self, entity: Entity) -> Vec<(TypeId, Box<dyn AnyComponent>)> {
        self.storages
            .iter_mut()
            .filter_map(|(type_id, storage)| storage.remove(entity).map(|c| (*type_id, c)))
            .collect()
    }

    /// Check whether `entity` has a component of the given `TypeId`.
    ///
    /// Used by [`Query`] so it can work with a plain `Vec<TypeId>` rather
    /// than monomorphised generics.
    ///
    /// [`Query`]: crate::query::Query
    pub(crate) fn contains(&self, entity: Entity, type_id: TypeId) -> bool {
        self.storages
            .get(&type_id)
            .is_some_and(|s| s.contains(entity))
    }
}
