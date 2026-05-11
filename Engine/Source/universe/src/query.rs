//! This module handles querying entities from a [`World`] based on what
//! components they have (or don't have).
//!
//! [`World`]: crate::World

use std::{any::TypeId, collections::HashSet};

use crate::{
    Entity, WorldId,
    components::{Component, Components},
};

/// A struct to query entities from a [`World`].
///
/// Conditions are evaluated as:
///   - ALL `with_component` types must be present, **and**
///   - NONE of the `without_component` types may be present.
///
/// An empty query matches every entity.
///
/// # Example
/// ```rust
/// # use universe::components::Component;
/// # use universe::query::Query;
/// # #[derive(Component, Debug, serde::Deserialize, serde::Serialize)]
/// # struct Position;
/// # #[derive(Component, Debug, serde::Deserialize, serde::Serialize)]
/// # struct Velocity;
/// # #[derive(Component, Debug, serde::Deserialize, serde::Serialize)]
/// # struct Frozen;
/// let query = Query::new()
///     .with_component::<Position>()
///     .with_component::<Velocity>()
///     .without_component::<Frozen>();
/// ```
///
/// [`World`]: crate::World
#[derive(Default)]
pub struct Query {
    /// Component types that must ALL be present on a matching entity.
    required: Vec<TypeId>,
    /// Component types that must ALL be absent on a matching entity.
    excluded: Vec<TypeId>,
}

impl Query {
    /// Creates a new, empty [`Query`] that matches every entity.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Require that matching entities have component `C`.
    ///
    /// Calling this multiple times with different types adds AND conditions;
    /// calling it with the same type twice is a no-op (duplicate [`TypeId`]s are
    /// ignored at match time).
    #[must_use]
    pub fn with_component<C: Component>(mut self) -> Self {
        self.required.push(TypeId::of::<C>());
        self
    }

    /// Require that matching entities do **not** have component `C`.
    ///
    /// Like [`with_component`], multiple calls add independent AND NOT
    /// conditions.
    ///
    /// [`with_component`]: Query::with_component
    #[must_use]
    pub fn without_component<C: Component>(mut self) -> Self {
        self.excluded.push(TypeId::of::<C>());
        self
    }

    /// Will match only entities of specified worlds
    #[must_use]
    pub fn with_world(self, _world: WorldId) -> Self {
        // TODO: this will require changing the query function to not take
        // in a HashSet, or combine all the HashSets (tbd)
        todo!("Query::with_world")
    }

    /// Returns `true` if `entity` satisfies every condition in this [`Query`].
    ///
    /// This is the single source of truth; [`query`] is implemented in terms
    /// of it.
    ///
    /// [`query`]: Query::query
    #[must_use]
    pub(crate) fn matches(&self, components: &Components, entity: Entity) -> bool {
        self.required
            .iter()
            .all(|&t| components.contains(entity, t))
            && self
                .excluded
                .iter()
                .all(|&t| !components.contains(entity, t))
    }

    /// Filter `alive` down to only the entities that satisfy this [`Query`].
    ///
    /// The `alive` slice is provided by [`World`] so that the query never
    /// returns despawned entity IDs, even if their component data has not yet
    /// been cleaned up.
    ///
    /// [`World`]: crate::World
    #[must_use]
    pub(crate) fn query(&self, components: &Components, alive: &HashSet<Entity>) -> Vec<Entity> {
        alive
            .iter()
            .copied()
            .filter(|&e| self.matches(components, e))
            .collect()
    }
}
