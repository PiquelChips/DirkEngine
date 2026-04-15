//! World and player event types.
//!
//! This module defines the two event families emitted by the world subsystem:
//!
//! * [`WorldEvent`] — coarse-grained lifecycle notifications about the world
//!   itself and its entities.
//! * [`PlayerUpdateEvent`] — fine-grained notifications about every observable
//!   state change of a [`Player`].
//!
//! # Design rationale
//!
//! Both types derive [`Event`] via the proc-macro so they can be dispatched
//! through the engine's [`EventManager`] / [`Dispatcher`] infrastructure.
//! Consumers register a typed listener once and receive a cloned snapshot for
//! every dispatch; no synchronisation is needed on the receiver side.
//!
//! [`WorldEvent`] is intentionally coarse: it covers entire-world and
//! entity-level transitions.  Any system that only cares about players should
//! prefer [`PlayerUpdateEvent`], which carries a richer snapshot (window,
//! region, update kind) and is emitted by [`Player`] itself rather than by the
//! world.

use events::Event;
use macros::Event;
use platform::WindowId;

use crate::{
    Entity, WorldId,
    player::{Player, PlayerId, PlayerRegion},
};

/// Coarse-grained lifecycle events emitted by a [`World`].
///
/// Each variant carries enough context for a listener to react without
/// needing to inspect the world directly.
///
/// # Variants
///
/// | Variant | When emitted |
/// |---------|-------------|
/// | [`Created`](WorldEvent::Created)     | A new world has been fully initialised. |
/// | [`Destroyed`](WorldEvent::Destroyed) | A world is about to be torn down. |
/// | [`EntitySpawn`](WorldEvent::EntitySpawn)   | An entity has been added to the world. |
/// | [`EntityUpdate`](WorldEvent::EntityUpdate) | A component was added, removed, or borrowed mutably. |
/// | [`EntityDespawn`](WorldEvent::EntityDespawn) | An entity has been removed from the world. |
///
/// # Examples
///
/// ```rust
/// # use world::events::WorldEvent;
/// # fn example(evt: WorldEvent) {
/// // Any variant exposes the originating world id through the helper.
/// let world_id = evt.world();
/// # }
/// ```
#[derive(Debug, Clone, Event)]
pub enum WorldEvent {
    /// A new world was created.
    ///
    /// Carries the newly-assigned [`WorldId`].
    #[event("World created with id {0}")]
    Created(WorldId),

    /// A world was destroyed.
    ///
    /// Carries the [`WorldId`] of the world that is being torn down.
    /// Listeners should treat any further state referencing this id as stale.
    #[event("World destroyed with id {0}")]
    Destroyed(WorldId),

    /// An entity was spawned in a world.
    ///
    /// Emitted after all default components have been attached.
    #[event("Entity spawned in world {world} with id {entity}")]
    EntitySpawn {
        /// The world the entity was spawned in.
        world: WorldId,
        /// The newly-created entity.
        entity: Entity,
    },

    /// An entity's component set changed, or a component was borrowed mutably.
    ///
    /// This event is intentionally broad: it fires on **any** structural or
    /// mutable access so that systems can invalidate caches conservatively.
    /// If granularity matters, compare the `entity` field against a watch-list.
    #[event("Entity updated in world {world} with id {entity}")]
    EntityUpdate {
        /// The world that contains the entity.
        world: WorldId,
        /// The entity that was updated.
        entity: Entity,
    },

    /// An entity was removed from a world.
    ///
    /// After this event is emitted the entity id must be considered invalid.
    #[event("Entity despawned in world {world} with id {entity}")]
    EntityDespawn {
        /// The world the entity was removed from.
        world: WorldId,
        /// The entity that was removed.
        entity: Entity,
    },
}

impl WorldEvent {
    /// Returns the [`WorldId`] associated with this event, regardless of
    /// variant.
    ///
    /// This is a convenience accessor so that a listener can filter by world
    /// with a single pattern-free check:
    ///
    /// ```rust
    /// # use world::events::WorldEvent;
    /// # fn example(events: Vec<WorldEvent>, my_world: world::WorldId) {
    /// let relevant: Vec<_> = events
    ///     .into_iter()
    ///     .filter(|e| e.world() == my_world)
    ///     .collect();
    /// # }
    /// ```
    pub fn world(&self) -> WorldId {
        match &self {
            Self::Created(id) => *id,
            Self::Destroyed(id) => *id,
            Self::EntitySpawn { world, .. } => *world,
            Self::EntityUpdate { world, .. } => *world,
            Self::EntityDespawn { world, .. } => *world,
        }
    }
}

/// A snapshot of a player's observable state at the moment a change occurred.
///
/// Emitted by [`Player`] on spawn, every call to
/// [`Player::set_region`](crate::player::Player::set_region), and on despawn.
/// Because the event is a *value snapshot* (not a reference), listeners can
/// safely store it or send it across threads without holding a lock on the
/// player.
///
/// # Fields
///
/// | Field | Description |
/// |-------|-------------|
/// | `id`          | The player's unique [`PlayerId`]. |
/// | `world`       | The [`WorldId`] the player lives in. |
/// | `entity`      | The ECS [`Entity`] for this player. |
/// | `window`      | The [`WindowId`] the player renders into. |
/// | `region`      | A clone of the player's [`PlayerRegion`] at the time of the event. |
/// | `update_type` | Why the event was fired — see [`PlayerUpdateType`]. |
///
/// # Examples
///
/// ```rust
/// # use world::events::{PlayerUpdateEvent, PlayerUpdateType};
/// # fn example(evt: PlayerUpdateEvent) {
/// match evt.update_type() {
///     PlayerUpdateType::Spawned   => { /* initialise per-player state */ }
///     PlayerUpdateType::Updated   => { /* refresh cached region / camera */ }
///     PlayerUpdateType::Despawned => { /* free per-player resources */ }
/// }
/// # }
/// ```
#[derive(Clone, Debug, Event)]
pub struct PlayerUpdateEvent {
    pub id: PlayerId,
    pub world: WorldId,
    pub entity: Entity,
    pub window: WindowId,
    pub region: PlayerRegion,
    pub update_type: PlayerUpdateType,
}

/// The reason a [`PlayerUpdateEvent`] was fired.
///
/// Variants are ordered chronologically: a player is first `Spawned`, may be
/// `Updated` zero or more times, and is finally `Despawned`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlayerUpdateType {
    /// The player was just created and its entity inserted into the world.
    Spawned,
    /// Some player state changed (currently: the viewport region).
    Updated,
    /// The player's entity was removed from the world.
    Despawned,
}

impl PlayerUpdateEvent {
    /// Constructs a [`PlayerUpdateEvent`] by snapshotting the relevant fields
    /// from `player`.
    ///
    /// This is the canonical constructor; it is called internally by [`Player`]
    /// and is exposed so that test harnesses or mock dispatchers can create
    /// events without going through the full `Player` machinery.
    ///
    /// # Arguments
    ///
    /// * `player`      — the player whose state should be snapshotted.
    /// * `update_type` — the reason for the event.
    pub fn from_player(player: &Player, update_type: PlayerUpdateType) -> Self {
        Self {
            id: player.id(),
            world: player.world(),
            entity: player.entity(),
            window: player.window(),
            region: player.region().clone(),
            update_type,
        }
    }
}
