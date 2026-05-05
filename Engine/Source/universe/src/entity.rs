//! This struct holds everything to do with [`Entity`]s.
//! Entities are just simple handles.
//! You can spawn them by creating an entity builder

/// A unique, opaque identifier for a spawned entity.
#[derive(Clone, Copy, Debug, Default, Hash, Eq, PartialEq)]
pub struct Entity(pub(crate) u32);

impl Entity {
    /// Returns an empty [`EntityBuilder`].
    pub fn builder() -> EntityBuilder {
        EntityBuilder::new()
    }
}

/// A builder struct to create a new entity. Allows adding of components.
#[derive(Default)]
pub struct EntityBuilder;

impl EntityBuilder {
    fn new() -> Self {
        Self::default()
    }
}
