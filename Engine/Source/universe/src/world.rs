use std::{any::TypeId, collections::HashMap};

use crate::{
    Entity,
    components::{AnyComponent, EntityComponents},
    query::Query,
};

/// An identifier that distinguishes multiple [`World`] instances from each other.
pub type WorldId = u32;

/// This is a world. It has entities and components.
#[derive(Default)]
pub struct World {
    id: WorldId,
    next_id: Entity,
    alive: Vec<Entity>,
    entity_components: EntityComponents,
    /// These should only be [`components::WorldComponent`]. This is
    /// guaranteed by the [`World`] API. Please make sure to add these
    /// properly internally.
    components: HashMap<TypeId, Box<dyn AnyComponent>>,
}

impl World {
    /// Creates a new empty world with the specified ID.
    #[must_use]
    pub(crate) fn new(id: WorldId) -> Self {
        Self {
            id,
            ..Self::default()
        }
    }
    /// Calls all the destruction [`System`]s on the world
    pub(crate) fn destroy(&mut self) {
        todo!("call all the world systems for destruction")
    }

    #[must_use]
    pub(crate) fn query(&self, query: &Query) -> Vec<Entity> {
        todo!("Query for entities")
    }
}
