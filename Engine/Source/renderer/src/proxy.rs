//! This module holds proxies for various engine objects

use platform::WindowId;
use world::{
    Entity, WorldId,
    player::{PlayerId, PlayerRegion},
};

pub struct CameraProxy {
    /// View matrix calculated from camera position.
    pub view: glam::Mat4,
    /// Projection matrix calculated from camera settings.
    pub proj: glam::Mat4,
}

pub struct PlayerProxy {
    id: PlayerId,
    world: WorldId,
    entity: Entity,
    window: WindowId,
    region: PlayerRegion,
}

impl From<world::events::PlayerUpdateEvent> for PlayerProxy {
    fn from(event: world::events::PlayerUpdateEvent) -> Self {
        Self {
            id: event.id,
            world: event.world,
            entity: event.entity,
            window: event.window,
            region: event.region,
        }
    }
}
