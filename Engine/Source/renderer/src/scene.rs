use std::collections::HashMap;

use ash::vk;
use gpu_allocator::MemoryLocation;
use universe::{Entity, WorldId};

use crate::{
    Error, MAX_FRAMES_IN_FLIGHT, Result,
    models::ModelRegistry,
    pipeline::GraphicsPipeline,
    proxy::CameraProxy,
    render_pass::RenderPass,
    resources::{
        buffer::UniformBuffer,
        command_pool::CommandBuffer,
        device::{Garbage, RenderDevice},
        image::Image,
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
    proxies: HashMap<Entity, SceneProxy>,

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

impl Scene {
    // TODO: use a system to sync world with renderer
    /*
    pub fn process_event(&mut self, world: &World, event: &WorldEvent) -> Result<()> {
        match *event {
            WorldEvent::Created(..) | WorldEvent::Destroyed(..) => {}
            WorldEvent::EntitySpawn { world: _, entity } => {
                self.proxies
                    .insert(entity, SceneProxy::build(&self.device, self)?);
            }
            WorldEvent::EntityUpdate { world: _, entity } => {
                let Some(proxy) = self.proxies.get_mut(&entity) else {
                    return Ok(());
                };
                let Some(transform) = world.get::<components::Transform>(entity) else {
                    return Ok(());
                };
                proxy.set_model_matrix(transform.matrix());

                if let Some(renderable) = world.get::<components::Renderable>(entity) {
                    proxy.set_model(renderable.model.clone());
                }
                if let Some(camera) = world.get::<components::Camera>(entity) {
                    proxy.set_camera(transform.view(), camera.projection());
                }
            }
            WorldEvent::EntityDespawn { world: _, entity } => {
                self.proxies.remove(&entity);
            }
        }
        Ok(())
    }
    */
    pub fn render(
        &self,
        models: &ModelRegistry,
        cmd: &CommandBuffer,
        size: vk::Extent2D,
        view: vk::ImageView,
        camera: Entity,
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
