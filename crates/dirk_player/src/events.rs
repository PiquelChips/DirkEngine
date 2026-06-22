//! All player-related events.

use dirk_events::Event;
use dirk_input::InputEvent;

use crate::PlayerId;

/// Fired when a new player is created via [`PlayerRegistry::new_player`].
///
/// [`PlayerRegistry::new_player`]: crate::PlayerRegistry::new_player
///
/// Game code should respond by spawning an ECS entity with a [`PlayerId`]
/// component to link the player to their in-world representation.
#[derive(Event, Debug, Clone)]
#[event("player {id} spawned")]
pub struct PlayerSpawned {
    /// The ID of the newly created player.
    pub id: PlayerId,
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
/// TODO: see about removing this event
#[derive(Event, Debug, Clone)]
#[event("player {id} input")]
pub(crate) struct PlayerInput {
    /// The player that should receive this input event.
    pub id: PlayerId,
    /// The input event translated into the player's input space.
    pub event: InputEvent,
}
