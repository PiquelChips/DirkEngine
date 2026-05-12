//! This struct holds everything to do with [`Entity`]s.
//! Entities are just simple handles.
//! You can spawn them by creating an entity builder

use std::{
    any::TypeId,
    collections::HashMap,
    ops::{Add, AddAssign},
};

use crate::components::{AnyComponent, Component};

/// A unique, opaque identifier for a spawned entity.
#[derive(Clone, Copy, Debug, Default, Hash, Eq, PartialEq)]
pub struct Entity(pub(crate) u64);

impl Entity {
    /// Returns an empty [`EntityBuilder`].
    #[must_use]
    pub fn builder() -> EntityBuilder {
        EntityBuilder::new()
    }
}

impl Add<u64> for Entity {
    type Output = Self;
    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<u64> for Entity {
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

/// A builder struct to create a new entity. Allows adding of components.
#[derive(Default)]
pub struct EntityBuilder {
    pub(crate) components: HashMap<TypeId, Box<dyn AnyComponent>>,
}

impl EntityBuilder {
    #[must_use]
    fn new() -> Self {
        Self::default()
    }

    /// Adds a [`Component`] to this entity.
    ///
    /// If adding multiple components of the same type, only the last call
    /// to `with_component` will be kept.
    #[must_use]
    pub fn with_component<C: Component>(mut self, component: C) -> Self {
        self.components
            .insert(TypeId::of::<C>(), Box::new(component));
        self
    }
}

/// A concrete, lazily-evaluated iterator over [`Entity`] values.
///
/// # Lifetime
///
/// `'u` is the lifetime of the [`Universe`] borrow from which entities are
/// sourced. The iterator must not outlive that borrow.
///
/// [`Universe`]: crate::Universe
pub struct EntityIterator<'u> {
    inner: Box<dyn Iterator<Item = Entity> + 'u>,
}

impl<'u> EntityIterator<'u> {
    pub(crate) fn new(iter: impl Iterator<Item = Entity> + 'u) -> Self {
        Self {
            inner: Box::new(iter),
        }
    }
}

impl Iterator for EntityIterator<'_> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
