//! This struct holds everything to do with [`Entity`]s.
//! Entities are just simple handles.
//! You can spawn them by creating an entity builder

use std::ops::{Add, AddAssign};

use crate::components::Component;

/// A unique, opaque identifier for a spawned entity.
#[derive(Clone, Copy, Debug, Default, Hash, Eq, PartialEq)]
pub struct Entity(pub(crate) u32);

impl Entity {
    /// Returns an empty [`EntityBuilder`].
    #[must_use]
    pub fn builder() -> EntityBuilder {
        EntityBuilder::new()
    }
}

impl Add<u32> for Entity {
    type Output = Self;
    fn add(self, rhs: u32) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<u32> for Entity {
    fn add_assign(&mut self, rhs: u32) {
        self.0 += rhs;
    }
}

/// A builder struct to create a new entity. Allows adding of components.
#[derive(Default)]
pub struct EntityBuilder {}

impl EntityBuilder {
    #[must_use]
    fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_component<C: Component>(self, component: C) -> Self {
        todo!("EntityBuilder::with_component")
    }
}
