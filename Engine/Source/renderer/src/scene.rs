use ash::vk;
use world::{World, components};

use crate::{Renderer, Result};

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
    view: glam::Mat4,
    /// Projection matrix calculated from screen settings.
    proj: glam::Mat4,
}

impl Scene {
    /// Builds a [Scene].
    /// Constructs the renderer stuff like command pools, descriptor sets, ... from
    /// the [Renderer] and all world proxy stuff from [World].
    pub fn build(renderer: &Renderer, world: &World) -> Result<Self> {
        let command_pool = {
            let pool_info = vk::CommandPoolCreateInfo::default()
                .queue_family_index(renderer.properties.queue_family_indices.graphics)
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

            unsafe { renderer.device.create_command_pool(&pool_info, None)? }
        };

        let (camera, camera_trans) = Self::get_camera(world);

        Ok(Self {
            command_pool,
            proxies: Self::make_scene_proxies(world),
            view: camera_trans.view(),
            proj: camera.projection(),
        })
    }
    /// This function will reconstruct the internal world data with the new input world.
    /// This includes: [SceneProxy]s, view matrix & projection matrix.
    pub fn rebuild(&mut self, world: &World) {
        let (camera, camera_trans) = Self::get_camera(world);
        self.proxies = Self::make_scene_proxies(world);
        self.view = camera_trans.view();
        self.proj = camera.projection();
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
    fn get_camera(world: &World) -> (&components::Camera, &components::Transform) {
        // TODO: don't just get the first camera + error handling if no camera
        let camera_entity = world.query_double::<components::Transform, components::Camera>()[0];
        (
            world.get::<components::Camera>(camera_entity).unwrap(),
            world.get::<components::Transform>(camera_entity).unwrap(),
        )
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
