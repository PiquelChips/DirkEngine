//! This module holds proxies for various engine objects

use dirk_platform::WindowId;
use dirk_universe::{Entity, WorldId};
use dirk_world::player::{PlayerId, PlayerUpdateEvent};

pub mod scene;
pub mod systems;

pub struct PlayerProxy {
    #[allow(unused)]
    pub id: PlayerId,
    pub world: WorldId,
    pub entity: Entity,
    pub window: WindowId,
    // TODO: render to a specific region of the window
    // pub region: PlayerRegion,
}

impl From<PlayerUpdateEvent> for PlayerProxy {
    fn from(event: PlayerUpdateEvent) -> Self {
        Self {
            id: event.id,
            world: event.world,
            entity: event.entity,
            window: event.window,
        }
    }
}
