//! This module holds proxies for various engine objects

use platform::WindowId;
use universe::{Entity, WorldId};
use world::player::PlayerId;

pub struct CameraProxy {
    /// View matrix calculated from camera position.
    pub view: glam::Mat4,
    /// Projection matrix calculated from camera settings.
    pub proj: glam::Mat4,
}

pub struct PlayerProxy {
    #[allow(unused)]
    pub id: PlayerId,
    pub world: WorldId,
    pub entity: Entity,
    pub window: WindowId,
    // TODO: render to a specific region of the window
    // pub region: PlayerRegion,
}

// TODO: player update event
/*
impl From<world::events::PlayerUpdateEvent> for PlayerProxy {
    fn from(event: world::events::PlayerUpdateEvent) -> Self {
        Self {
            id: event.id,
            world: event.world,
            entity: event.entity,
            window: event.window,
        }
    }
}
*/
