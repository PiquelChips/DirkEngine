use std::collections::{HashMap, HashSet};

use ash::vk;
use dirk_universe::{Entity, WorldId};
use gpu_allocator::MemoryLocation;

use crate::{
    Error, MAX_FRAMES_IN_FLIGHT, Result,
    models::ModelRegistry,
    pipeline::GraphicsPipeline,
    resources::{
        buffer::UniformBuffer,
        command_pool::CommandBuffer,
        descriptors::{
            DescriptorAllocator, DescriptorSet, DescriptorWriter, ObjectLayout, SceneLayout,
        },
        device::RenderDevice,
    },
};

/// This is the renderer proxy for the [`Universe`]. It also has
/// most of the rendering state needed to render each scene.
pub struct SceneManager {
    device: RenderDevice,

    scenes: HashMap<WorldId, Scene>,
    entities: HashMap<Entity, WorldId>,
    proxies: HashMap<Entity, SceneProxy>,

    // TODO: see about centralising the different pipelines (link with
    // descriptor layouts, ...)
    graphics_pipeline: GraphicsPipeline,

    scene_alloc: DescriptorAllocator<SceneLayout>,
    proxy_alloc: DescriptorAllocator<ObjectLayout>,
}

impl SceneManager {
    pub fn init(device: &RenderDevice) -> Result<Self> {
        let scene_alloc = DescriptorAllocator::<SceneLayout>::new(device, 16)?;
        let proxy_alloc = DescriptorAllocator::<ObjectLayout>::new(device, 256)?;

        let graphics_pipeline =
            GraphicsPipeline::build(device, &device.layouts, &device.properties)?;

        Ok(Self {
            device: device.clone(),
            entities: HashMap::new(),
            scenes: HashMap::new(),
            proxies: HashMap::new(),
            graphics_pipeline,
            scene_alloc,
            proxy_alloc,
        })
    }
    pub fn render(
        &self,
        models: &ModelRegistry,
        cmd: &CommandBuffer,
        world: WorldId,
        size: vk::Extent2D,
        camera: Entity,
    ) -> Result<()> {
        let frame = self.device.current_frame();
        let scene = self
            .scenes
            .get(&world)
            .ok_or(Error::WorldDoesNotExist(world))?;

        let proxies = scene
            .entities
            .iter()
            .filter_map(|e| self.proxies.get(e))
            .collect::<Vec<_>>();

        // CAMERA
        {
            let proxy = &self
                .proxies
                .get(&camera)
                .ok_or(Error::CameraDoesNotExist(camera))?;

            let view = proxy.view.ok_or(Error::CameraDoesNotExist(camera))?;

            // TODO: proper viewport & camera system
            let proj = {
                // `width` & `height` aren't large enough for this to matter
                #[allow(clippy::cast_precision_loss)]
                let aspect = size.width as f32 / size.height.max(1) as f32;
                let mut proj = glam::Mat4::perspective_rh(
                    45_f32.to_radians(), // FOV
                    aspect,              // Aspect Ratio
                    0.1,                 // near clip
                    100_000.0,           // far clip
                );
                // Vulkan NDC has Y pointing down; flip the projection accordingly.
                proj.y_axis.y *= -1.0;
                proj
            };

            let scene_ubo = SceneUbo { view, proj };
            unsafe { scene.ubo[frame].write(&scene_ubo) };
        };

        for proxy in &proxies {
            proxy.write_ubo(frame);
        }

        self.graphics_pipeline.bind(cmd);

        // the window size never gets anywhere near 2^23
        #[allow(clippy::cast_precision_loss)]
        let viewport = vk::Viewport::default()
            .width(size.width as f32)
            .height(size.height as f32)
            .min_depth(0.)
            .max_depth(1.);
        cmd.set_viewport(0, &[viewport]);

        let scissor = vk::Rect2D::default()
            .offset(vk::Offset2D::default())
            .extent(size);
        cmd.set_scissor(0, &[scissor]);

        for proxy in &proxies {
            let Some(ref model) = proxy.model else {
                continue;
            };

            match models.render_model(
                model,
                cmd,
                &scene.sets[frame],
                &proxy.sets[frame],
                self.graphics_pipeline.layout(),
            ) {
                Ok(()) | Err(dirk_assets::Error::NotFound(_)) => (),
                Err(err) => return Err(err.into()),
            }
        }

        Ok(())
    }
    pub fn create_scene(&mut self, world: WorldId) -> Result<()> {
        let scene = Scene::build(self)?;
        self.scenes.insert(world, scene);
        Ok(())
    }
    pub fn destroy_scene(&mut self, world: WorldId) {
        self.scenes.remove(&world);
    }
    pub fn create_proxy(&mut self, entity: Entity, world: WorldId) -> Result<()> {
        let proxy = SceneProxy::build(self)?;
        self.proxies.insert(entity, proxy);

        self.entities.insert(entity, world);
        self.scenes
            .get_mut(&world)
            .ok_or(Error::WorldDoesNotExist(world))?
            .entities
            .insert(entity);
        Ok(())
    }
    pub fn get_proxy_mut(&mut self, entity: Entity) -> Option<&mut SceneProxy> {
        self.proxies.get_mut(&entity)
    }
    #[must_use]
    pub fn entity_world(&self, entity: Entity) -> Option<WorldId> {
        self.entities.get(&entity).copied()
    }
    pub fn send_proxy(&mut self, entity: Entity, to: WorldId) -> Result<()> {
        let world = self
            .entities
            .get(&entity)
            .copied()
            .ok_or(Error::EntityDoesNotExist(entity))?;

        if !self.scenes.contains_key(&to) {
            return Err(Error::WorldDoesNotExist(to));
        }

        let old = self
            .scenes
            .get_mut(&world)
            .ok_or(Error::WorldDoesNotExist(world))?;

        old.entities.remove(&entity);

        let new = self
            .scenes
            .get_mut(&to)
            .ok_or(Error::WorldDoesNotExist(to))?;
        new.entities.insert(entity);

        self.entities.insert(entity, to);
        Ok(())
    }
    pub fn destroy_proxy(&mut self, entity: Entity) -> Result<()> {
        let world = self
            .entities
            .get(&entity)
            .ok_or(Error::EntityDoesNotExist(entity))?;

        self.scenes
            .get_mut(world)
            .ok_or(Error::WorldDoesNotExist(*world))?
            .entities
            .remove(&entity);
        self.proxies.remove(&entity);
        Ok(())
    }
}

impl Drop for SceneManager {
    fn drop(&mut self) {
        // Clear collections before allocators are dropped. Scene and
        // SceneProxy hold DescriptorSet values whose Drop impls enqueue descriptor
        // set frees; the allocators enqueue descriptor pool destroys.
        self.scenes.clear();
        self.entities.clear();
        self.proxies.clear();
    }
}

#[derive(Clone, Copy)]
// fields are read by Vulkan, not us
#[allow(unused)]
struct SceneUbo {
    view: glam::Mat4,
    proj: glam::Mat4,
}

#[derive(Clone, Copy)]
// fields are read by Vulkan, not us
#[allow(unused)]
struct ProxyUbo {
    model: glam::Mat4,
}

/// Renderer representation of a [`World`].
struct Scene {
    entities: HashSet<Entity>,

    ubo: [UniformBuffer; MAX_FRAMES_IN_FLIGHT],
    sets: [DescriptorSet<SceneLayout>; MAX_FRAMES_IN_FLIGHT],
}

impl Scene {
    /// Builds a [Scene].
    /// Constructs the renderer stuff like command pools, descriptor sets, ... from
    /// the [Renderer].
    pub fn build(manager: &mut SceneManager) -> Result<Self> {
        // Allocate scene-level sets (one per frame)
        let sets = manager
            .scene_alloc
            .allocate_array::<MAX_FRAMES_IN_FLIGHT>()?;

        let ubo_size = size_of::<SceneUbo>() as u64;
        let build_ubo =
            || UniformBuffer::create(&manager.device, ubo_size, MemoryLocation::CpuToGpu);
        let ubo = [build_ubo()?, build_ubo()?];

        let mut writer = DescriptorWriter::new(&manager.device.device);
        for (set, ubo) in sets.iter().zip(&ubo) {
            writer = writer.uniform_buffer(set, ubo.buffer(), ubo_size);
        }
        writer.flush();

        Ok(Self {
            entities: HashSet::new(),
            ubo,
            sets,
        })
    }
}

pub struct SceneProxy {
    /// The model matrix used for rendering. Constructed from the
    /// [`world::components::Transform`] of the entity.
    model_matrix: Option<glam::Mat4>,
    /// The view matrix used for rendering as camera
    view: Option<glam::Mat4>,
    /// The name of the model. Used to request a [`crate::model::Model`] from the
    /// renderer at render time.
    model: Option<dirk_assets::AssetHandle>,

    // Per frame render stuff
    ubo: [UniformBuffer; MAX_FRAMES_IN_FLIGHT],
    sets: [DescriptorSet<ObjectLayout>; MAX_FRAMES_IN_FLIGHT],
}

impl SceneProxy {
    pub fn build(manager: &mut SceneManager) -> Result<Self> {
        let size = size_of::<ProxyUbo>() as u64;
        let build_ubo = || UniformBuffer::create(&manager.device, size, MemoryLocation::CpuToGpu);
        let ubo = [build_ubo()?, build_ubo()?];

        // Allocate scene-level sets (one per frame)
        let sets = manager
            .proxy_alloc
            .allocate_array::<MAX_FRAMES_IN_FLIGHT>()?;

        let mut writer = DescriptorWriter::new(&manager.device.device);
        for (set, ubo) in sets.iter().zip(&ubo) {
            writer = writer.uniform_buffer(set, ubo.buffer(), size);
        }
        writer.flush();

        Ok(Self {
            model: None,
            model_matrix: None,
            view: None,
            ubo,
            sets,
        })
    }
    pub fn set_model(&mut self, model: Option<dirk_assets::AssetHandle>) {
        self.model = model;
    }
    pub fn set_model_matrix(&mut self, mat: Option<glam::Mat4>) {
        self.model_matrix = mat;

        if let Some(mat) = mat {
            let proxy_ubo = ProxyUbo { model: mat };
            for ubo in &self.ubo {
                unsafe { ubo.write(&proxy_ubo) };
            }
        }
    }
    pub fn set_view(&mut self, view: Option<glam::Mat4>) {
        self.view = view;
    }
    pub fn write_ubo(&self, frame: usize) {
        let Some(model) = self.model_matrix else {
            return;
        };

        let data = ProxyUbo { model };
        unsafe { self.ubo[frame].write(&data) };
    }
}
