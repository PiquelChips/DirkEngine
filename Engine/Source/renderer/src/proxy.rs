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
