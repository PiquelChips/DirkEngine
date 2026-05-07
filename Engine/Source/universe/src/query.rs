//! This module handles querying entities from a [`World`] based on what
//! components they have (or don't have).

use std::any::TypeId;

use crate::{
    Entity,
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
/// # #[derive(Component)]
/// # struct Position;
/// # #[derive(Component)]
/// # struct Velocity;
/// # #[derive(Component)]
/// # struct Frozen;
/// let query = Query::new()
///     .with_component::<Position>()
///     .with_component::<Velocity>()
///     .without_component::<Frozen>();
/// ```
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
    pub(crate) fn query(&self, components: &Components, alive: &[Entity]) -> Vec<Entity> {
        alive
            .iter()
            .copied()
            .filter(|&e| self.matches(components, e))
            .collect()
    }
}
