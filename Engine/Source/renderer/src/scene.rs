use std::ffi::c_void;

use ash::vk::{self};
use world::{World, components};

use crate::{MAX_FRAMES_IN_FLIGHT, Renderer, Result, model, render_pass::RenderPass};

/// This scene is created from a [world::World].
/// It should then be updated whenever the world is updated.
///
/// Handles rendering all the [world::components::Renderable] objects
/// of the world.
pub struct Scene {
    /// The entities to render.
    proxies: Vec<SceneProxy>,
    /// View matrix calculated from camera position.
    view: glam::Mat4,
    /// Projection matrix calculated from screen settings.
    proj: glam::Mat4,

    descriptor_pool: vk::DescriptorPool,
    ubo: [UboData; MAX_FRAMES_IN_FLIGHT],
    descriptor_sets: [vk::DescriptorSet; MAX_FRAMES_IN_FLIGHT],
}

struct SceneUbo {
    view: glam::Mat4,
    proj: glam::Mat4,
}

#[derive(Debug)]
struct UboData {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    mapped: *mut c_void,
}

impl Scene {
    /// Builds a [Scene].
    /// Constructs the renderer stuff like command pools, descriptor sets, ... from
    /// the [Renderer] and all world proxy stuff from [World].
    pub fn build(renderer: &Renderer, world: &World) -> Result<Self> {
        // TODO: load all the models that are used by the scene proxies
        let (camera, camera_trans) = Self::get_camera(world);

        let proxy_count = world
            .query_double::<components::Renderable, components::Transform>()
            .len() as u32;

        let pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                // scene UBOs + object UBOs, all × frames in flight
                descriptor_count: (1 + proxy_count) * MAX_FRAMES_IN_FLIGHT as u32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                // rough upper bound on material textures
                descriptor_count: proxy_count * MAX_FRAMES_IN_FLIGHT as u32,
            },
        ];

        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_sizes)
            .max_sets((1 + proxy_count * 2) * MAX_FRAMES_IN_FLIGHT as u32);

        let descriptor_pool = unsafe { renderer.device.create_descriptor_pool(&pool_info, None)? };

        // Allocate scene-level sets (one per frame)
        let layouts = [renderer.layouts.scene; MAX_FRAMES_IN_FLIGHT];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);

        let scene_desc_sets: [vk::DescriptorSet; MAX_FRAMES_IN_FLIGHT] = unsafe {
            renderer
                .device
                .allocate_descriptor_sets(&alloc_info)?
                .try_into()
                .unwrap()
        };

        let ubo: Vec<UboData> = (0..MAX_FRAMES_IN_FLIGHT)
            .map(|_| {
                let size = size_of::<SceneUbo>() as u64;
                let (buffer, memory) = renderer.create_buffer(
                    size,
                    vk::BufferUsageFlags::UNIFORM_BUFFER,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                )?;

                let mapped = unsafe {
                    renderer
                        .device
                        .map_memory(memory, 0, size, vk::MemoryMapFlags::empty())?
                };

                Ok(UboData {
                    buffer,
                    memory,
                    mapped,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let ubo: [UboData; MAX_FRAMES_IN_FLIGHT] = ubo.try_into().unwrap();

        let buffer_infos: [vk::DescriptorBufferInfo; MAX_FRAMES_IN_FLIGHT] =
            std::array::from_fn(|i| {
                vk::DescriptorBufferInfo::default()
                    .buffer(ubo[i].buffer)
                    .range(size_of::<SceneUbo>() as u64)
                    .offset(0)
            });

        let descriptor_writes: [vk::WriteDescriptorSet; MAX_FRAMES_IN_FLIGHT] =
            std::array::from_fn(|i| {
                vk::WriteDescriptorSet::default()
                    .dst_set(scene_desc_sets[i])
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(std::slice::from_ref(&buffer_infos[i]))
            });

        unsafe {
            renderer
                .device
                .update_descriptor_sets(&descriptor_writes, &[])
        };

        // TODO: build proxies

        Ok(Self {
            proxies: Self::make_scene_proxies(renderer, world)?,
            view: camera_trans.view(),
            proj: camera.projection(),
            descriptor_pool,
            ubo,
            descriptor_sets: scene_desc_sets,
        })
    }
    // TODO: on tick, worlds should be sent to update scenes
    #[allow(unused)]
    /// This function will reconstruct the internal world data with the new input world.
    /// This includes: [SceneProxy]s, view matrix & projection matrix.
    pub fn rebuild(&mut self, renderer: &Renderer, world: &World) -> Result<()> {
        let (camera, camera_trans) = Self::get_camera(world);
        self.proxies = Self::make_scene_proxies(renderer, world)?;
        self.view = camera_trans.view();
        self.proj = camera.projection();
        Ok(())
    }
    fn make_scene_proxies(renderer: &Renderer, world: &World) -> Result<Vec<SceneProxy>> {
        Ok(world
            .query_double::<components::Renderable, components::Transform>()
            .iter()
            .map(|&entity| {
                // already made sure the entity has the component
                let renderable = world.get::<components::Renderable>(entity).unwrap();
                let transform = world.get::<components::Transform>(entity).unwrap();
                SceneProxy::build(renderer, &renderable.model, transform.matrix())
            })
            .collect::<Result<Vec<_>>>()?)
    }
    fn get_camera(world: &World) -> (&components::Camera, &components::Transform) {
        // TODO: don't just get the first camera + error handling if no camera
        let camera_entity = world.query_double::<components::Transform, components::Camera>()[0];
        (
            world.get::<components::Camera>(camera_entity).unwrap(),
            world.get::<components::Transform>(camera_entity).unwrap(),
        )
    }
    pub fn render(&self, renderer: &Renderer, cmd: vk::CommandBuffer) {
        let frame = renderer.frames[renderer.current_frame];
        let device = &renderer.device;

        RenderPass::begin(renderer, cmd);
        renderer.graphics_pipeline.bind(renderer, cmd);

        // TODO: bind scene sets
        // TODO: for each proxy: record cmds
        // TODO: end render pass

        for proxy in &self.proxies {
            // Assuming you've built a descriptor set layout with combined image samplers:
            for prim in &proxy.model.primitives {
                if let Some(mat_idx) = prim.material {
                    // from GpuPrimitive
                    let mat = &proxy.model.materials[mat_idx];

                    if let Some(tex_idx) = mat.base_color_texture() {
                        let tex = &proxy.model.textures[*tex_idx];

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
        }
    }
}

/// A renderable entity's representation for the renderer.
/// Owned by [Scene], constructed from [world::components::Renderable] and
/// [world::components::Transform].
pub struct SceneProxy {
    /// The name of the model. Used to request a [crate::model::Model] from the
    /// renderer at render time.
    model: model::Model,
    /// The model matrix used for rendering. Constructed from the
    /// [world::components::Transform] of the entity.
    model_matrix: glam::Mat4,
    ubo: [UboData; MAX_FRAMES_IN_FLIGHT],
}

impl SceneProxy {
    pub fn build(renderer: &Renderer, model: &str, model_matrix: glam::Mat4) -> Result<Self> {
        let model = renderer
            .get_model(model)
            .expect("should have the model")
            .clone();

        let ubo: Vec<UboData> = (0..MAX_FRAMES_IN_FLIGHT)
            .map(|_| {
                let size = size_of::<glam::Mat4>() as u64;
                let (buffer, memory) = renderer.create_buffer(
                    size,
                    vk::BufferUsageFlags::UNIFORM_BUFFER,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                )?;

                let mapped = unsafe {
                    renderer
                        .device
                        .map_memory(memory, 0, size, vk::MemoryMapFlags::empty())?
                };

                Ok(UboData {
                    buffer,
                    memory,
                    mapped,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let ubo: [UboData; MAX_FRAMES_IN_FLIGHT] = ubo.try_into().unwrap();

        Ok(Self {
            model,
            model_matrix,
            ubo,
        })
    }
}
