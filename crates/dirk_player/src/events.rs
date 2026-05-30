//! All player-related events.

use dirk_events::Event;

use crate::PlayerId;

/// Fired when a new player is created via [`PlayerManager::new_player`].
///
/// Game code should respond by spawning an ECS entity with a [`PlayerId`]
/// component to link the player to their in-world representation.
///
/// # Example
///
/// ```rust
/// # use dirk_player::events::PlayerSpawned;
/// # use dirk_events::Consumer;
/// # fn example(consumer: Consumer<PlayerSpawned>) {
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
}

/// Fired when a player is removed via [`PlayerManager::remove_player`].
///
/// Game code should respond by despawning the ECS entity that carries
/// this player's [`PlayerId`] component.
#[derive(Event, Debug, Clone)]
#[event("player {id} despawned")]
pub struct PlayerDespawned {
    /// The ID of the removed player.
    pub id: PlayerId,
}

/// Fired by [`PlayerManager::tick`] when the window a player renders to
/// is resized.
///
/// Systems that manage cameras should subscribe to this event and update
/// the camera component's width and height accordingly.
#[derive(Event, Debug, Clone)]
#[event("player {id} window resized to {width}x{height}")]
pub struct PlayerWindowResized {
    /// The affected player.
    pub id: PlayerId,
    /// New window width in pixels.
    pub width: u32,
    /// New window height in pixels.
    pub height: u32,
}
