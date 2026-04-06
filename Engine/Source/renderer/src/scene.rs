use std::ffi::c_void;

use ash::{Device, vk};
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
    render_pass: RenderPass,
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

impl UboData {
    fn destroy(&self, device: &Device) {
        unsafe {
            device.destroy_buffer(self.buffer, None);
            device.free_memory(self.memory, None);
        }
    }
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

        let window = renderer.windows.get(&renderer.main_window).unwrap();
        let render_pass = RenderPass::build(renderer, window.extent())?;

        let mut scene = Self {
            proxies: Vec::new(),
            view: camera_trans.view(),
            proj: camera.projection(),
            descriptor_pool,
            ubo,
            descriptor_sets: scene_desc_sets,
            render_pass,
        };
        scene.proxies = scene.make_scene_proxies(renderer, world)?;
        Ok(scene)
    }
    pub fn destroy(&self, device: &Device) {
        self.proxies.iter().for_each(|proxy| proxy.destroy(device));
        self.render_pass.destroy(device);
        self.ubo.iter().for_each(|ubo| ubo.destroy(device));
        unsafe { device.destroy_descriptor_pool(self.descriptor_pool, None) };
    }
    // TODO: on tick, worlds should be sent to update scenes
    #[allow(unused)]
    /// This function will reconstruct the internal world data with the new input world.
    /// This includes: [SceneProxy]s, view matrix & projection matrix.
    pub fn rebuild(&mut self, renderer: &Renderer, world: &World) -> Result<()> {
        let (camera, camera_trans) = Self::get_camera(world);
        self.proxies = self.make_scene_proxies(renderer, world)?;
        self.view = camera_trans.view();
        self.proj = camera.projection();
        Ok(())
    }
    fn make_scene_proxies(&self, renderer: &Renderer, world: &World) -> Result<Vec<SceneProxy>> {
        world
            .query_double::<components::Renderable, components::Transform>()
            .iter()
            .map(|&entity| {
                // already made sure the entity has the component
                let renderable = world.get::<components::Renderable>(entity).unwrap();
                let transform = world.get::<components::Transform>(entity).unwrap();
                SceneProxy::build(renderer, self, &renderable.model, transform.matrix())
            })
            .collect::<Result<Vec<_>>>()
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
        let device = &renderer.device;

        let window = renderer.windows.get(&renderer.main_window).unwrap();

        self.render_pass
            .begin(renderer, cmd, window.extent(), window.next_image().view);
        renderer.graphics_pipeline.bind(renderer, cmd);

        let viewport = vk::Viewport::default()
            .width(window.extent().width as f32)
            .height(window.extent().height as f32)
            .min_depth(0.)
            .max_depth(1.);
        unsafe { renderer.device.cmd_set_viewport(cmd, 1, &[viewport]) };

        let scissor = vk::Rect2D::default()
            .offset(vk::Offset2D::default())
            .extent(window.extent());
        unsafe { renderer.device.cmd_set_scissor(cmd, 1, &[scissor]) };

        let mut descriptor_sets = [
            self.descriptor_sets[renderer.current_frame],
            vk::DescriptorSet::null(),
            vk::DescriptorSet::null(),
        ];

        for proxy in &self.proxies {
            descriptor_sets[1] = proxy.sets[renderer.current_frame];
            // TODO: material descriptpor set
            // descriptor_sets[2] = material_desc_set;

            unsafe {
                device.cmd_bind_descriptor_sets(
                    cmd,
                    vk::PipelineBindPoint::GRAPHICS,
                    renderer.graphics_pipeline.layout(),
                    0,
                    &descriptor_sets,
                    &[],
                )
            };

            for prim in &proxy.model.primitives {
                unsafe {
                    device.cmd_bind_vertex_buffers(cmd, 0, &[prim.vertex_buffer], &[0]);
                    device.cmd_bind_index_buffer(cmd, prim.index_buffer, 0, vk::IndexType::UINT32);
                    device.cmd_draw_indexed(cmd, prim.index_count, 1, 0, 0, 0);
                }
            }
        }

        self.render_pass.end(renderer, cmd);
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
    sets: [vk::DescriptorSet; MAX_FRAMES_IN_FLIGHT],
}

struct ProxyUbo {
    model: glam::Mat4,
}

impl SceneProxy {
    pub fn build(
        renderer: &Renderer,
        scene: &Scene,
        model: &str,
        model_matrix: glam::Mat4,
    ) -> Result<Self> {
        let model = renderer
            .get_model(model)
            .expect("should have the model")
            .clone();

        let size = size_of::<ProxyUbo>() as u64;
        let ubo: Vec<UboData> = (0..MAX_FRAMES_IN_FLIGHT)
            .map(|_| {
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

        // Allocate scene-level sets (one per frame)
        let layouts = [renderer.layouts.object; MAX_FRAMES_IN_FLIGHT];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(scene.descriptor_pool)
            .set_layouts(&layouts);

        let sets: [vk::DescriptorSet; MAX_FRAMES_IN_FLIGHT] = unsafe {
            renderer
                .device
                .allocate_descriptor_sets(&alloc_info)?
                .try_into()
                .unwrap()
        };

        let buffer_infos: [vk::DescriptorBufferInfo; MAX_FRAMES_IN_FLIGHT] =
            std::array::from_fn(|i| {
                vk::DescriptorBufferInfo::default()
                    .buffer(ubo[i].buffer)
                    .range(size)
                    .offset(0)
            });

        let descriptor_writes: [vk::WriteDescriptorSet; MAX_FRAMES_IN_FLIGHT] =
            std::array::from_fn(|i| {
                vk::WriteDescriptorSet::default()
                    .dst_set(sets[i])
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(std::slice::from_ref(&buffer_infos[i]))
            });

        unsafe {
            renderer
                .device
                .update_descriptor_sets(&descriptor_writes, &[])
        };

        Ok(Self {
            model,
            model_matrix,
            ubo,
            sets,
        })
    }
    fn destroy(&self, device: &Device) {
        self.ubo.iter().for_each(|ubo| ubo.destroy(device));
    }
}
