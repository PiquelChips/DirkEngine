//! This module holds proxies for various engine objects

use platform::WindowId;
use universe::{Entity, WorldId};
use world::player::PlayerId;

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

impl From<world::player::PlayerUpdateEvent> for PlayerProxy {
    fn from(event: world::player::PlayerUpdateEvent) -> Self {
        Self {
            id: event.id,
            world: event.world,
            entity: event.entity,
            window: event.window,
        }
    }
}
