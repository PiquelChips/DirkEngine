use std::collections::HashSet;

use crate::{Entity, EntityBuilder};

/// An identifier that distinguishes multiple [`World`] instances from each other.
pub type WorldId = u32;

/// This is a world. It has entities and components.
pub struct World {
    id: WorldId,
    name: String,
    pub(crate) alive: HashSet<Entity>,
}

impl World {
    /// Returns a [`WorldBuilder`].
    #[must_use]
    pub fn builder(name: String) -> WorldBuilder {
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
    fn new(name: String) -> Self {
        Self {
            name,
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
