use std::{collections::HashMap, ffi::c_void};

use ash::{Device, vk};
use world::WorldId;

use crate::{
    Error, MAX_FRAMES_IN_FLIGHT, MAX_RENDERABLES, Renderer, Result, command_pool::CommandBuffer,
    model, render_pass::RenderPass,
};

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

    /// Copies `data` into the persistently-mapped host-visible memory.
    ///
    /// # Safety
    /// The mapped pointer must be valid and the allocation must cover at least
    /// `size_of::<T>()` bytes — both invariants are guaranteed by every
    /// `UboData` constructed in this module.
    fn write<T: Copy>(&self, data: &T) {
        unsafe {
            std::ptr::copy_nonoverlapping(
                data as *const T as *const u8,
                self.mapped as *mut u8,
                size_of::<T>(),
            )
        };
    }
}

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
    ubo: [UboData; MAX_FRAMES_IN_FLIGHT],
    descriptor_sets: [vk::DescriptorSet; MAX_FRAMES_IN_FLIGHT],

    color: vk::ImageView,
    color_image: vk::Image,
    color_memory: vk::DeviceMemory,
    depth: vk::ImageView,
    depth_image: vk::Image,
    depth_memory: vk::DeviceMemory,
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
    pub fn build(renderer: &Renderer, size: vk::Extent2D, world: WorldId) -> Result<Self> {
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

        // IMAGES
        let (color_image, color_memory) = renderer.create_image(
            size,
            renderer.properties.surface_format.format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::TRANSIENT_ATTACHMENT | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            (1, renderer.properties.msaa_samples),
        )?;
        let color = renderer.create_image_view(
            color_image,
            renderer.properties.surface_format.format,
            vk::ImageAspectFlags::COLOR,
            1,
        )?;

        let (depth_image, depth_memory) = renderer.create_image(
            size,
            renderer.properties.depth_format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            (1, renderer.properties.msaa_samples),
        )?;
        let depth = renderer.create_image_view(
            depth_image,
            renderer.properties.depth_format,
            vk::ImageAspectFlags::DEPTH,
            1,
        )?;

        Ok(Self {
            world,
            device: renderer.device.clone(),
            proxies: HashMap::new(),
            descriptor_pool,
            ubo,
            descriptor_sets: scene_desc_sets,

            color,
            color_image,
            color_memory,
            depth,
            depth_image,
            depth_memory,
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
                .ok_or(Error::CameraDoesNotExit(self.world, camera))?;

            let camera = proxy
                .camera
                .as_ref()
                .ok_or(Error::CameraDoesNotExit(self.world, camera))?;

            let scene_ubo = SceneUbo {
                view: camera.view,
                proj: camera.proj,
            };
            self.ubo[frame].write(&scene_ubo);
        };

        for proxy in self.proxies.values() {
            proxy.write_ubo(frame);
        }

        RenderPass::begin(renderer, cmd, size, view, self.color, self.depth);
        renderer.graphics_pipeline.bind(renderer, cmd);

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
                    device.cmd_bind_vertex_buffers(cmd.raw(), 0, &[prim.vertex_buffer], &[0]);
                    device.cmd_bind_index_buffer(
                        cmd.raw(),
                        prim.index_buffer,
                        0,
                        vk::IndexType::UINT32,
                    );
                    device.cmd_draw_indexed(cmd.raw(), prim.index_count, 1, 0, 0, 0);
                }
            }
        }

        RenderPass::end(renderer, cmd);
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
        self.proxies.clear();
        self.ubo.iter().for_each(|ubo| ubo.destroy(&self.device));
        unsafe {
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.destroy_image_view(self.color, None);
            self.device.destroy_image(self.color_image, None);
            self.device.free_memory(self.color_memory, None);
            self.device.destroy_image_view(self.depth, None);
            self.device.destroy_image(self.depth_image, None);
            self.device.free_memory(self.depth_memory, None);
        };
    }
}

/// A renderable entity's representation for the renderer.
/// Owned by [Scene], constructed from [world::components::Renderable] and
/// [world::components::Transform].
pub struct SceneProxy {
    device: Device,
    /// The model matrix used for rendering. Constructed from the
    /// [world::components::Transform] of the entity.
    model_matrix: Option<glam::Mat4>,
    /// The name of the model. Used to request a [crate::model::Model] from the
    /// renderer at render time.
    model: Option<model::Model>,
    /// An optional camera that could be attached to the mesh.
    camera: Option<CameraProxy>,

    // Per frame render stuff
    ubo: [UboData; MAX_FRAMES_IN_FLIGHT],
    sets: [vk::DescriptorSet; MAX_FRAMES_IN_FLIGHT],
}

#[derive(Clone, Copy)]
// fields are read by Vulkan, not us
#[allow(unused)]
struct ProxyUbo {
    model: glam::Mat4,
}

impl SceneProxy {
    pub fn build(renderer: &Renderer, scene: &Scene) -> Result<Self> {
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
            device: renderer.device.clone(),
            model: None,
            model_matrix: None,
            camera: None,
            ubo,
            sets,
        })
    }
    pub fn set_model(&mut self, model: &model::Model) {
        self.model = Some(model.clone());
    }
    pub fn set_model_matrix(&mut self, mat: glam::Mat4) {
        self.model_matrix = Some(mat);

        let proxy_ubo = ProxyUbo { model: mat };
        for ubo in &self.ubo {
            ubo.write(&proxy_ubo);
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
        self.ubo[frame].write(&data);
    }
}

impl Drop for SceneProxy {
    fn drop(&mut self) {
        self.ubo.iter().for_each(|ubo| ubo.destroy(&self.device));
    }
}
