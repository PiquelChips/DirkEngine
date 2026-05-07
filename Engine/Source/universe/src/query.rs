//! This module handles querying entities from a [`World`] based on what
//! components they have (or don't have).

use crate::{
    Entity,
    components::{Component, Components},
};

/// A struct to query entities from a [`World`].
///
/// Uses a builder pattern to add queries.
pub struct Query {
    // Some kind of type erased storage for easy querying in the [`World`]
}

impl Query {
    /// Add an AND condition for the component type
    #[must_use]
    pub fn with_component<C: Component>(self) -> Self {
        todo!("add component to query")
    }
    /// Add an AND NOT condition for the specified component type
    #[must_use]
    pub fn without_component<C: Component>(self) -> Self {
        todo!("remove component from query")
    }
    /// Will actually run the [`Query`] against [`Components`].
    #[must_use]
    pub(crate) fn query(&self, components: &Components) -> Vec<Entity> {
        todo!("actually query the entities")
    }
    /// Returns if the [`Entity`] from the [`Components`] matches the [`Query`].
    #[must_use]
    pub(crate) fn matches(&self, components: &Components, entity: Entity) -> bool {
        todo!("see if the entity matches the query")
    }
}
