use ash::vk;
use world::{World, components};

use crate::Result;

/// This scene is created from a [world::World].
/// It should then be updated whenever the world is updated.
///
/// Handles rendering all the [world::components::Renderable] objects
/// of the world.
pub struct Scene {
    /// Each scene has its own command pool.
    command_pool: vk::CommandPool,
    /// The entities to render.
    proxies: Vec<SceneProxy>,
    /// View matrix calculated from camera position.
    view: glam::Vec4,
    /// Projection matrix calculated from screen settings.
    proj: glam::Vec4,
}

impl Scene {
    pub fn build(renderer: &crate::Renderer, world: &World) -> Result<Self> {
        let command_pool = {
            let pool_info = vk::CommandPoolCreateInfo::default()
                .queue_family_index(renderer.properties.queue_family_indices.graphics)
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

            unsafe { renderer.device.create_command_pool(&pool_info, None)? }
        };

        Ok(Self {
            command_pool,
            proxies: Self::make_scene_proxies(world),
        })
    }
    fn make_scene_proxies(world: &World) -> Vec<SceneProxy> {
        world
            .query_double::<components::Renderable, components::Transform>()
            .iter()
            .map(|&entity| {
                // already made sure the entity has the component
                let renderable = world.get::<components::Renderable>(entity).unwrap();
                let transform = world.get::<components::Transform>(entity).unwrap();
                SceneProxy {
                    model: renderable.model.clone(),
                    model_matrix: transform.to_owned().into(),
                }
            })
            .collect()
    }
}

/// A renderable entity's representation for the renderer.
/// Owned by [Scene], constructed from [world::components::Renderable] and
/// [world::components::Transform].
pub struct SceneProxy {
    /// The name of the model. Used to request a [crate::model::Model] from the
    /// renderer at render time.
    model: String,
    /// The model matrix used for rendering. Constructed from the
    /// [world::components::Transform] of the entity.
    model_matrix: glam::Mat4,
}
