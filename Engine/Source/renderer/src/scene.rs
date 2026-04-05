use std::ffi::c_void;

use ash::vk::{self, MemoryMapFlags};
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
            proxies: Self::make_scene_proxies(renderer, world),
            view: camera_trans.view(),
            proj: camera.projection(),
        })
    }
    /// This function will reconstruct the internal world data with the new input world.
    /// This includes: [SceneProxy]s, view matrix & projection matrix.
    pub fn rebuild(&mut self, renderer: &Renderer, world: &World) {
        let (camera, camera_trans) = Self::get_camera(world);
        self.proxies = Self::make_scene_proxies(renderer, world);
        self.view = camera_trans.view();
        self.proj = camera.projection();
    }
    fn make_scene_proxies(renderer: &Renderer, world: &World) -> Vec<SceneProxy> {
        world
            .query_double::<components::Renderable, components::Transform>()
            .iter()
            .map(|&entity| {
                // already made sure the entity has the component
                let renderable = world.get::<components::Renderable>(entity).unwrap();
                let transform = world.get::<components::Transform>(entity).unwrap();
                SceneProxy::build(renderer, &renderable.model, transform.matrix())
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

    ubo_buffer: vk::Buffer,
    ubo_memory: vk::DeviceMemory,
    ubo_mapped: *mut c_void,
}

impl SceneProxy {
    pub fn build(renderer: &Renderer, model: &str, model_matrix: glam::Mat4) -> Result<Self> {
        let size = size_of::<crate::ModelViewProjection>() as u64;

        let (buffer, memory) = renderer.create_buffer(
            size,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let mapped = unsafe {
            renderer
                .device
                .map_memory(memory, 0, size, MemoryMapFlags::empty())?
        };

        Ok(Self {
            model: model.to_string(),
            model_matrix,
            ubo_buffer: buffer,
            ubo_memory: memory,
            ubo_mapped: mapped,
        })
    }
    pub fn record_cmd(&self, renderer: &Renderer, cmd: vk::CommandBuffer) -> Result<()> {
        let model = renderer
            .get_model(&self.model)
            .expect("should have the model");
        let device = &renderer.device;

        // Assuming you've built a descriptor set layout with combined image samplers:
        for prim in &model.primitives {
            if let Some(mat_idx) = prim.material {
                // from GpuPrimitive
                let mat = &model.materials[mat_idx];

                if let Some(tex_idx) = mat.base_color_texture() {
                    let tex = &model.textures[*tex_idx];

                    let image_info = vk::DescriptorImageInfo::default()
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .image_view(tex.view)
                        .sampler(tex.sampler);

                    let write = vk::WriteDescriptorSet::default()
                        // TODO: setup descriptor sets for the textures
                        //.dst_set(descriptor_set)
                        .dst_binding(0)
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(std::slice::from_ref(&image_info));

                    unsafe { device.update_descriptor_sets(&[write], &[]) };
                }
            }

            unsafe {
                device.cmd_bind_vertex_buffers(cmd, 0, &[prim.vertex_buffer], &[0]);
                device.cmd_bind_index_buffer(cmd, prim.index_buffer, 0, vk::IndexType::UINT32);
                device.cmd_draw_indexed(cmd, prim.index_count, 1, 0, 0, 0);
            }
        }
        Ok(())
    }
}
