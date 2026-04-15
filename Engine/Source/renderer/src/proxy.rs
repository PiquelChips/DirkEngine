use platform::WindowId;
use world::{
    Entity, WorldId,
    player::{PlayerId, PlayerRegion},
};

pub struct CameraProxy {
    /// View matrix calculated from camera position.
    view: glam::Mat4,
    /// Projection matrix calculated from camera settings.
    proj: glam::Mat4,
}

pub struct PlayerProxy {
    id: PlayerId,
    world: WorldId,
    entity: Entity,
    window: WindowId,
    region: PlayerRegion,
}
