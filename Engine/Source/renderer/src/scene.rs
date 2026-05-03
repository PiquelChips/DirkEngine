use std::collections::HashMap;

use ash::vk;
use gpu_allocator::MemoryLocation;
use world::WorldId;

use crate::{
    Error, MAX_FRAMES_IN_FLIGHT, MAX_RENDERABLES, Result,
    models::ModelRegistry,
    pipeline::GraphicsPipeline,
    render_pass::RenderPass,
    resources::{
        buffer::UniformBuffer,
        command_pool::CommandBuffer,
        device::{Garbage, RenderDevice},
        image::{Image, ImageCreateInfo},
    },
};

/// This scene is created from a [`world::World`].
/// It should then be updated whenever the world is updated.
///
/// Handles rendering all the [`world::components::Renderable`] objects
/// of the world.
pub struct Scene {
    world: WorldId,
    device: RenderDevice,
    /// The entities to render.
    proxies: HashMap<world::Entity, SceneProxy>,

    descriptor_pool: vk::DescriptorPool,
    ubo: [UniformBuffer; MAX_FRAMES_IN_FLIGHT],
    descriptor_sets: [vk::DescriptorSet; MAX_FRAMES_IN_FLIGHT],

    // TODO: these need to be removed
    color: Image,
    depth: Image,
    // render graph should fix this
    graphics_pipeline: GraphicsPipeline,
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
    pub fn build(device: &RenderDevice, size: vk::Extent2D, world: WorldId) -> Result<Self> {
        // MAX_FRAMES_IN_FLIGHT never gets anywhere near u32::MAX
        #[allow(clippy::cast_possible_truncation)]
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

        // MAX_FRAMES_IN_FLIGHT never gets anywhere near u32::MAX
        #[allow(clippy::cast_possible_truncation)]
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_sizes)
            .max_sets((1 + MAX_RENDERABLES * 2) * MAX_FRAMES_IN_FLIGHT as u32);

        let descriptor_pool = unsafe { device.device.create_descriptor_pool(&pool_info, None)? };

        // Allocate scene-level sets (one per frame)
        let layouts = [device.layouts.scene; MAX_FRAMES_IN_FLIGHT];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);

        let scene_desc_sets: [vk::DescriptorSet; MAX_FRAMES_IN_FLIGHT] = unsafe {
            device
                .device
                .allocate_descriptor_sets(&alloc_info)?
                .try_into()
                .expect("should be able to convert desc_sets to array")
        };

        let ubo_size = size_of::<SceneUbo>() as u64;
        let build_ubo = || UniformBuffer::create(device, ubo_size, MemoryLocation::CpuToGpu);
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
            device
                .device
                .update_descriptor_sets(&descriptor_writes, &[]);
        };

        // TEMP
        let color_info = ImageCreateInfo {
            size,
            format: device.properties.surface_format.format,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::TRANSIENT_ATTACHMENT
                | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            location: MemoryLocation::GpuOnly,
            mip_levels: 1,
            num_samples: device.properties.msaa_samples,
            aspect_flags: vk::ImageAspectFlags::COLOR,
        };
        let color = Image::create_image(device, &color_info)?;

        let depth_info = ImageCreateInfo {
            size,
            format: device.properties.depth_format,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            location: MemoryLocation::GpuOnly,
            mip_levels: 1,
            num_samples: device.properties.msaa_samples,
            aspect_flags: vk::ImageAspectFlags::DEPTH,
        };
        let depth = Image::create_image(device, &depth_info)?;
        let graphics_pipeline =
            GraphicsPipeline::build(device, &device.layouts, &device.properties)?;

        Ok(Self {
            world,
            device: device.clone(),
            proxies: HashMap::new(),
            descriptor_pool,
            ubo,
            descriptor_sets: scene_desc_sets,

            color,
            depth,
            graphics_pipeline,
        })
    }
    pub fn render(
        &self,
        models: &ModelRegistry,
        cmd: &CommandBuffer,
        size: vk::Extent2D,
        view: vk::ImageView,
        camera: world::Entity,
    ) -> Result<()> {
        let frame = self.device.current_frame();
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

        RenderPass::begin(
            &self.device.device,
            cmd,
            size,
            view,
            &self.color,
            &self.depth,
        );
        self.graphics_pipeline.bind(cmd);

        // the window size never gets anywhere near 2^23
        #[allow(clippy::cast_precision_loss)]
        let viewport = vk::Viewport::default()
            .width(size.width as f32)
            .height(size.height as f32)
            .min_depth(0.)
            .max_depth(1.);
        unsafe {
            self.device
                .device
                .cmd_set_viewport(cmd.raw(), 0, &[viewport]);
        };

        let scissor = vk::Rect2D::default()
            .offset(vk::Offset2D::default())
            .extent(size);
        unsafe { self.device.device.cmd_set_scissor(cmd.raw(), 0, &[scissor]) };

        for proxy in self.proxies.values() {
            let Some(ref model) = proxy.model else {
                continue;
            };

            models.render_model(
                model,
                cmd,
                self.descriptor_sets[frame],
                proxy.sets[frame],
                self.graphics_pipeline.layout(),
            )?;
        }

        RenderPass::end(&self.device.device, cmd);
        Ok(())
    }

    pub fn add_proxy(&mut self, entity: world::Entity, proxy: SceneProxy) {
        self.proxies.insert(entity, proxy);
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
        self.device
            .destroy(Garbage::DescriptorPool(self.descriptor_pool));
    }
}

/// A renderable entity's representation for the renderer.
/// Owned by [Scene], constructed from [`world::components::Renderable`] and
/// [`world::components::Transform`].
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

#[derive(Clone, Copy)]
// fields are read by Vulkan, not us
#[allow(unused)]
struct ProxyUbo {
    model: glam::Mat4,
}

impl SceneProxy {
    pub fn build(device: &RenderDevice, scene: &Scene) -> Result<Self> {
        let size = size_of::<ProxyUbo>() as u64;
        let build_ubo = || UniformBuffer::create(device, size, MemoryLocation::CpuToGpu);
        let ubo = [build_ubo()?, build_ubo()?];

        // Allocate scene-level sets (one per frame)
        let layouts = [device.layouts.object; MAX_FRAMES_IN_FLIGHT];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(scene.descriptor_pool)
            .set_layouts(&layouts);

        let sets: [vk::DescriptorSet; MAX_FRAMES_IN_FLIGHT] = unsafe {
            device
                .device
                .allocate_descriptor_sets(&alloc_info)?
                .try_into()
                .expect("vec should be MAX_FRAMES_IN_FLIGHT large so Into shouldn't fail")
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
            device
                .device
                .update_descriptor_sets(&descriptor_writes, &[]);
        };

        Ok(Self {
            model: None,
            model_matrix: None,
            camera: None,
            ubo,
            sets,
        })
    }
    pub fn set_model(&mut self, model: assets::AssetHandle) {
        self.model = Some(model);
    }
    pub fn set_model_matrix(&mut self, mat: glam::Mat4) {
        self.model_matrix = Some(mat);

        let proxy_ubo = ProxyUbo { model: mat };
        for ubo in &self.ubo {
            unsafe { ubo.write(&proxy_ubo) };
        }
    }
    pub fn set_camera(&mut self, view: glam::Mat4, proj: glam::Mat4) {
        self.camera = Some(CameraProxy { view, proj });
    }
    fn write_ubo(&self, frame: usize) {
        let Some(model) = self.model_matrix else {
            return;
        };

        let data = ProxyUbo { model };
        unsafe { self.ubo[frame].write(&data) };
    }
}
