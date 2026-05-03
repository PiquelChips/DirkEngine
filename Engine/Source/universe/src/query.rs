//! This module handles querying entities from a [`World`] based on what
//! components they have (or don't have).

use crate::{Component, Entity, World};

/// A struct to query entities from a [`World`].
///
/// Uses a builder pattern to add queries.
pub struct Query {
    // Some kind of type erased storage for easy querying in the [`World`]
}

impl Query {
    /// Add an AND condition for the component type
    pub fn with_component<C: Component>(self) -> Self {
        todo!("add component to query")
    }
    /// Add an AND NOT condition for the specified component type
    pub fn without_component<C: Component>(self) -> Self {
        todo!("remove component from query")
    }
    /// Will actually run the [`Query`] against a world.
    pub(crate) fn query(&self, world: &World) -> Vec<Entity> {
        todo!("actually query the entities")
    }
    /// Returns if the [`Entity`] from the [`World`] matches the [`Query`].
    pub(crate) fn matches(&self, world: &World, entity: Entity) -> bool {
        todo!("see if the entity matches the query")
    }
}
