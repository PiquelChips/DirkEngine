use std::{
    collections::HashSet,
    fmt::Display,
    ops::{Add, AddAssign},
};

use crate::{Entity, EntityBuilder};

/// An identifier that distinguishes multiple [`World`] instances from each other.
#[derive(Clone, Copy, Debug, Default, Hash, Eq, PartialEq)]
pub struct WorldId(u32);

impl WorldId {
    /// Returns the raw entity ID
    #[must_use]
<<<<<<< HEAD
    pub fn raw(&self) -> u32 {
=======
    pub fn raw(self) -> u32 {
>>>>>>> main
        self.0
    }
}

impl Display for WorldId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Add<u32> for WorldId {
    type Output = Self;
    fn add(self, rhs: u32) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<u32> for WorldId {
    fn add_assign(&mut self, rhs: u32) {
        self.0 += rhs;
    }
}

/// This is a world. It has entities and components.
pub struct World {
    id: WorldId,
    name: String,
    pub(crate) alive: HashSet<Entity>,
}

impl World {
    /// Returns a [`WorldBuilder`].
    #[must_use]
    pub fn builder(name: impl Into<String>) -> WorldBuilder {
        WorldBuilder::new(name)
    }
    /// Creates an empty world with a name & id.
    #[must_use]
    pub(crate) fn new(id: WorldId, name: String) -> Self {
        Self {
            id,
            name,
            alive: HashSet::new(),
        }
    }
    /// Returns the [`WorldId`] of the [`World`].
    #[must_use]
    pub fn id(&self) -> WorldId {
        self.id
    }
    /// Returns the name of the [`World`].
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Builder struct for [`World`].
#[derive(Default)]
pub struct WorldBuilder {
    pub(crate) name: String,
    pub(crate) entities: Vec<EntityBuilder>,
}

impl WorldBuilder {
    /// Creates a new empty [`WorldBuilder`].
    #[must_use]
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    /// Adds an [`Entity`] that will be spawned on [`World`] creation.
    #[must_use]
    pub fn with_entity(mut self, entity: EntityBuilder) -> Self {
        self.entities.push(entity);
        self
    }
}
