//! This crate holds the entire universe.
//!
//! The **Universe** is `DirkEngine`'s ECS system.

use serde::{Serialize, de::DeserializeOwned};
use std::{collections::HashMap, fmt::Debug};

pub mod query;
pub mod systems;

/// A unique, opaque identifier for a spawned entity.
pub type Entity = u32;
/// An identifier that distinguishes multiple [`World`] instances from each other.
pub type WorldId = u32;

/// Marker trait for component types.
pub trait Component: 'static + Sized + Debug + Serialize + DeserializeOwned {}
#[doc(hidden)]
pub use macros::Component;

/// This struct is the manager for all the worlds.
pub struct Universe {
    worlds: HashMap<WorldId, World>,
    next_id: WorldId,
}

/// This is a world. It has entities and components.
pub struct World {}
