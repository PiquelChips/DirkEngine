//! All player-related events.

use dirk_events::Event;

/// Event fired whenenver a player is spawned.
#[derive(Event, Debug, Clone)]
pub struct PlayerSpawned {}
