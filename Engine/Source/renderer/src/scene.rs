use std::collections::HashMap;

use ash::{Device, vk};
use gpu_allocator::MemoryLocation;
use world::WorldId;

use crate::{
    Error, MAX_FRAMES_IN_FLIGHT, MAX_RENDERABLES, Renderer, Result,
    buffer::UniformBuffer,
    command_pool::CommandBuffer,
    image::{Image, ImageCreateInfo},
    render_pass::RenderPass,
};

/// This scene is created from a [world::World].
/// It should then be updated whenever the world is updated.
///
/// Handles rendering all the [world::components::Renderable] objects
/// of the world.
pub struct Scene {
    world: WorldId,
    device: Device,
    /// The entities to render.
    proxies: HashMap<world::Entity, SceneProxy>,

    descriptor_pool: vk::DescriptorPool,
    ubo: [UniformBuffer; MAX_FRAMES_IN_FLIGHT],
    descriptor_sets: [vk::DescriptorSet; MAX_FRAMES_IN_FLIGHT],

    color: Image,
    depth: Image,
}

#[derive(Clone, Copy)]
// fields are read by Vulkan, not us
#[allow(unused)]
struct SceneUbo {
    view: glam::Mat4,
    proj: glam::Mat4,
}

struct CameraProxy {
    /// View matrix calculated from camera position.
    view: glam::Mat4,
    /// Projection matrix calculated from camera settings.
    proj: glam::Mat4,
}

impl Scene {
    /// Builds a [Scene].
    /// Constructs the renderer stuff like command pools, descriptor sets, ... from
    /// the [Renderer].
    pub fn build(renderer: &mut Renderer, size: vk::Extent2D, world: WorldId) -> Result<Self> {
        let pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                // scene UBOs + object UBOs, all × frames in flight
                descriptor_count: (1 + MAX_RENDERABLES) * MAX_FRAMES_IN_FLIGHT as u32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                // rough upper bound on material textures
                descriptor_count: MAX_RENDERABLES * MAX_FRAMES_IN_FLIGHT as u32,
            },
        ];

        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_sizes)
            .max_sets((1 + MAX_RENDERABLES * 2) * MAX_FRAMES_IN_FLIGHT as u32);

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

        let ubo_size = size_of::<SceneUbo>() as u64;
        let mut build_ubo = || UniformBuffer::create(renderer, ubo_size, MemoryLocation::CpuToGpu);
        let ubo = [build_ubo()?, build_ubo()?];

        let buffer_infos: [vk::DescriptorBufferInfo; MAX_FRAMES_IN_FLIGHT] =
            std::array::from_fn(|i| {
                vk::DescriptorBufferInfo::default()
                    .buffer(ubo[i].buffer())
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

        // IMAGES
        let color_info = ImageCreateInfo {
            size,
            format: renderer.properties.surface_format.format,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::TRANSIENT_ATTACHMENT
                | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            location: MemoryLocation::GpuOnly,
            mip_levels: 1,
            num_samples: renderer.properties.msaa_samples,
            aspect_flags: vk::ImageAspectFlags::COLOR,
        };
        let color = Image::create_image(renderer, color_info)?;

        let depth_info = ImageCreateInfo {
            size,
            format: renderer.properties.depth_format,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            location: MemoryLocation::GpuOnly,
            mip_levels: 1,
            num_samples: renderer.properties.msaa_samples,
            aspect_flags: vk::ImageAspectFlags::DEPTH,
        };
        let depth = Image::create_image(renderer, depth_info)?;

        Ok(Self {
            world,
            device: renderer.device.clone(),
            proxies: HashMap::new(),
            descriptor_pool,
            ubo,
            descriptor_sets: scene_desc_sets,

            color,
            depth,
        })
    }
    pub fn render(
        &self,
        renderer: &Renderer,
        cmd: &CommandBuffer,
        size: vk::Extent2D,
        view: vk::ImageView,
        camera: world::Entity,
    ) -> Result<()> {
        let device = &renderer.device;

        let frame = renderer.current_frame;

        // CAMERA
        {
            let proxy = &self
                .proxies
                .get(&camera)
                .ok_or(Error::CameraDoesNotExist(self.world, camera))?;

            let camera = proxy
                .camera
                .as_ref()
                .ok_or(Error::CameraDoesNotExist(self.world, camera))?;

            let scene_ubo = SceneUbo {
                view: camera.view,
                proj: camera.proj,
            };
            unsafe { self.ubo[frame].write(&scene_ubo) };
        };

        for proxy in self.proxies.values() {
            proxy.write_ubo(frame);
        }

        RenderPass::begin(&renderer.device, cmd, size, view, &self.color, &self.depth);
        renderer.graphics_pipeline.bind(cmd);

        let viewport = vk::Viewport::default()
            .width(size.width as f32)
            .height(size.height as f32)
            .min_depth(0.)
            .max_depth(1.);
        unsafe { renderer.device.cmd_set_viewport(cmd.raw(), 0, &[viewport]) };

        let scissor = vk::Rect2D::default()
            .offset(vk::Offset2D::default())
            .extent(size);
        unsafe { renderer.device.cmd_set_scissor(cmd.raw(), 0, &[scissor]) };

        let mut descriptor_sets = [
            self.descriptor_sets[renderer.current_frame],
            vk::DescriptorSet::null(),
            vk::DescriptorSet::null(),
        ];

        for proxy in self.proxies.values() {
            let Some(ref model) = proxy.model else {
                continue;
            };
            let Some(model) = renderer.models.get(model) else {
                continue;
            };

            for prim in &model.primitives {
                let mat_set = prim
                    .material
                    .and_then(|idx| model.material_sets.get(idx).copied())
                    .unwrap_or(vk::DescriptorSet::null());

                descriptor_sets[1] = proxy.sets[renderer.current_frame];
                descriptor_sets[2] = mat_set;

                unsafe {
                    device.cmd_bind_descriptor_sets(
                        cmd.raw(),
                        vk::PipelineBindPoint::GRAPHICS,
                        renderer.graphics_pipeline.layout(),
                        0,
                        &descriptor_sets,
                        &[],
                    );
                    device.cmd_bind_vertex_buffers(
                        cmd.raw(),
                        0,
                        &[prim.vertex_buffer.buffer()],
                        &[0],
                    );
                    device.cmd_bind_index_buffer(
                        cmd.raw(),
                        prim.index_buffer.buffer(),
                        0,
                        vk::IndexType::UINT32,
                    );
                    device.cmd_draw_indexed(cmd.raw(), prim.index_count, 1, 0, 0, 0);
                }
            }
        }

        RenderPass::end(&renderer.device, cmd);
        Ok(())
    }
    pub fn add_proxy(&mut self, entity: world::Entity, proxy: SceneProxy) -> Result<()> {
        self.proxies.insert(entity, proxy);
        Ok(())
    }
    pub fn get_proxy(&mut self, entity: world::Entity) -> Option<&mut SceneProxy> {
        self.proxies.get_mut(&entity)
    }
    pub fn remove_proxy(&mut self, entity: world::Entity) {
        self.proxies.remove(&entity);
    }
}

impl Drop for Scene {
    fn drop(&mut self) {
        unsafe {
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
        };
    }
}

/// A renderable entity's representation for the renderer.
/// Owned by [Scene], constructed from [world::components::Renderable] and
/// [world::components::Transform].
pub struct SceneProxy {
    /// The model matrix used for rendering. Constructed from the
    /// [world::components::Transform] of the entity.
    model_matrix: Option<glam::Mat4>,
    /// The name of the model. Used to request a [crate::model::Model] from the
    /// renderer at render time.
    model: Option<String>,
    /// An optional camera that could be attached to the mesh.
    camera: Option<CameraProxy>,

    // Per frame render stuff
    ubo: [UniformBuffer; MAX_FRAMES_IN_FLIGHT],
    sets: [vk::DescriptorSet; MAX_FRAMES_IN_FLIGHT],
}

#[derive(Clone, Copy)]
// fields are read by Vulkan, not us
#[allow(unused)]
struct ProxyUbo {
    model: glam::Mat4,
}

impl SceneProxy {
    pub fn build(renderer: &mut Renderer, world: WorldId) -> Result<Self> {
        let size = size_of::<ProxyUbo>() as u64;
        let mut build_ubo = || UniformBuffer::create(renderer, size, MemoryLocation::CpuToGpu);
        let ubo = [build_ubo()?, build_ubo()?];

        let Some(scene) = renderer.scenes.get(&world) else {
            return Err(Error::WorldDoesNotExist(world));
        };

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
                    .buffer(ubo[i].buffer())
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
            model: None,
            model_matrix: None,
            camera: None,
            ubo,
            sets,
        })
    }
    pub fn set_model(&mut self, model: &str) {
        self.model = Some(model.to_string());
    }
    pub fn set_model_matrix(&mut self, mat: glam::Mat4) {
        self.model_matrix = Some(mat);

        let proxy_ubo = ProxyUbo { model: mat };
        for ubo in &self.ubo {
            unsafe { ubo.write(&proxy_ubo) };
        }
    }
    pub fn set_camera(&mut self, view: glam::Mat4, proj: glam::Mat4) {
        self.camera = Some(CameraProxy { view, proj })
    }
    fn write_ubo(&self, frame: usize) {
        let Some(model) = self.model_matrix else {
            return;
        };

        let data = ProxyUbo { model };
        unsafe { self.ubo[frame].write(&data) };
    }
}
