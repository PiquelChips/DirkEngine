//! This module has all the proxy types. These are the render representations
//! of the components needed for rendering

use ash::vk;
use platform::WindowId;
use universe::{Entity, WorldId};
use world::player::PlayerId;

use crate::{MAX_FRAMES_IN_FLIGHT, resources::buffer::UniformBuffer};

pub struct SceneProxy {
    /// The model matrix used for rendering. Constructed from the
    /// [`world::components::Transform`] of the entity.
    model_matrix: Option<glam::Mat4>,
    /// The name of the model. Used to request a [`crate::model::Model`] from the
    /// renderer at render time.
    model: Option<assets::AssetHandle>,
    /// An optional camera that could be attached to the mesh.
    camera: Option<CameraProxy>,

    // Per frame render stuff
    ubo: [UniformBuffer; MAX_FRAMES_IN_FLIGHT],
    sets: [vk::DescriptorSet; MAX_FRAMES_IN_FLIGHT],
}

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

#[derive(Clone, Copy)]
// fields are read by Vulkan, not us
#[allow(unused)]
pub struct SceneUbo {
    view: glam::Mat4,
    proj: glam::Mat4,
}

#[derive(Clone, Copy)]
// fields are read by Vulkan, not us
#[allow(unused)]
pub struct ProxyUbo {
    model: glam::Mat4,
}
