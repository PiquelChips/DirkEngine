use crate::Entity;
use serde::{Serialize, de::DeserializeOwned};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::Debug,
};

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
pub(crate) struct Components {
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
