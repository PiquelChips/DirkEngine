//! All player-related events.

use dirk_events::Event;
use dirk_platform::{InputEvent, WindowId};

use crate::PlayerId;

/// Fired when a new player is created via [`PlayerRegistry::new_player`].
///
/// [`PlayerRegistry::new_player`]: crate::PlayerRegistry::new_player
///
/// Game code should respond by spawning an ECS entity with a [`PlayerId`]
/// component to link the player to their in-world representation.
///
/// # Example
///
/// ```rust
/// # use dirk_player::PlayerSpawned;
/// # use dirk_events::Consumer;
/// # fn example(mut consumer: Consumer<PlayerSpawned>) {
/// for event in consumer.consume_all() {
///     // Spawn an entity and attach event.id as a component.
/// }
/// # }
/// ```
#[derive(Event, Debug, Clone)]
#[event("player {id} spawned")]
pub struct PlayerSpawned {
    /// The ID of the newly created player.
    pub id: PlayerId,
    /// The Window the player is rendered too.
    pub window: WindowId,
}

/// Fired when a player is removed via [`PlayerRegistry::remove_player`].
///
/// [`PlayerRegistry::remove_player`]: crate::PlayerRegistry::remove_player
///
/// Game code should respond by despawning the ECS entity that carries
/// this player's [`PlayerId`] component.
#[derive(Event, Debug, Clone)]
#[event("player {id} despawned")]
pub struct PlayerDespawned {
    /// The ID of the removed player.
    pub id: PlayerId,
}

/// Input routed to a specific player.
#[derive(Event, Debug, Clone)]
#[event("player {id} input")]
pub struct PlayerInput {
    /// The player that should receive this input event.
    pub id: PlayerId,
    /// The platform input event translated into the player's input space.
    pub event: InputEvent,
}
