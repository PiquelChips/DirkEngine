//! This module holds proxies for various engine objects

use dirk_platform::WindowId;
use dirk_player::{PlayerId, PlayerSpawned};
use dirk_universe::Entity;

pub mod scene;
pub mod systems;

pub struct PlayerProxy {
    #[allow(unused)]
    pub id: PlayerId,
    pub window: WindowId,
    pub entity: Option<Entity>,
}

impl From<PlayerSpawned> for PlayerProxy {
    fn from(event: PlayerSpawned) -> Self {
        Self {
            id: event.id,
            window: event.window,
            entity: None,
        }
    }
}
