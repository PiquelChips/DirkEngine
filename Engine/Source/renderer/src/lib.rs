#[cfg(validation)]
use std::os::raw::c_void;
use std::{
    collections::{HashMap, HashSet},
    ffi::{CStr, CString},
};

#[cfg(validation)]
use ash::ext::debug_utils;
#[cfg(platform_linux)]
use ash::khr::wayland_surface;
use ash::{
    Device, Entry, Instance,
    khr::{surface, swapchain},
    vk,
};
use platform::{PlatformEvent, WindowEvent, WindowId};
use tracing::{debug, error, info, trace, warn};

mod errors;
pub use errors::{Error, Result};

mod physical_device;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

mod model;
use model::*;

mod scene;
use scene::{Scene, SceneProxy};

mod window;
use window::Window;

mod pipeline;
use pipeline::GraphicsPipeline;

mod command_pool;
use command_pool::{CommandBuffer, CommandPool, Graphics, Transfer};
use world::{components, events::WorldEvent};

mod layouts;

mod render_pass;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    texcoord: [f32; 2],
}

impl Vertex {
    const fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }
    }
    const fn attribute_description() -> [vk::VertexInputAttributeDescription; 3] {
        [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: std::mem::offset_of!(Self, position) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: std::mem::offset_of!(Self, normal) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: std::mem::offset_of!(Self, texcoord) as u32,
            },
        ]
    }
}

fn make_version(version: utils::Version) -> u32 {
    vk::make_api_version(0, version.major(), version.minor(), version.patch())
}

/// The maximum numer of renderables in a scene.
/// Used to construct Ubo samples.
/// TODO: find a way to set this limit dynamically or have a error when the limit is reached.
const MAX_RENDERABLES: u32 = 100;

const MAX_FRAMES_IN_FLIGHT: usize = 2;
const DEVICE_EXTENSIONS: &[&str] =
    &[unsafe { std::str::from_utf8_unchecked(swapchain::NAME.to_bytes()) }];
#[cfg(validation)]
const VALIDATION_LAYERS: &[*const i8] = &[c"VK_LAYER_KHRONOS_validation".as_ptr()];

struct Frame {
    device: Device,
    /// Command pool to allocate command buffers on every frame
    command_pool: CommandPool<Graphics>,
    /// Main synchronization fence
    fence: vk::Fence,
    // TODO: have one primary command buffer that is allocated once and
    // secondary command for each scene. Should be allocated every time
    // there is a change in scene count. If not reallocated, reset.
}

impl Frame {
    fn destroy(&self) {
        self.command_pool.destroy();
        unsafe {
            self.device.destroy_fence(self.fence, None);
        }
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        self.destroy();
    }
}

/// This struct is owned by [Renderer] and stores
/// all the different descriptor set layouts used by
/// the renderer.
/// Every field should be a descriptor set layout with a
/// propper comment explain what the layout is and where
/// it is used.
struct DescriptorLayouts {
    device: Device,
    // TODO: much better comments for descriptor set layouts
    /// Per scene layout. Holds view & proj matrices for rendering.
    scene: vk::DescriptorSetLayout,
    /// Per object layout. For model matrix.
    object: vk::DescriptorSetLayout,
    /// Per material layout. For texture descriptor.
    material: vk::DescriptorSetLayout,
}

impl Drop for DescriptorLayouts {
    fn drop(&mut self) {
        self.destroy();
    }
}

impl DescriptorLayouts {
    fn destroy(&self) {
        unsafe {
            self.device.destroy_descriptor_set_layout(self.scene, None);
            self.device.destroy_descriptor_set_layout(self.object, None);
            self.device
                .destroy_descriptor_set_layout(self.material, None);
        }
    }
}

pub struct RendererCreateInfo {
    pub engine_name: CString,
    pub engine_version: utils::Version,
    pub app_name: CString,
    pub app_version: utils::Version,
}

struct Queues {
    graphics: vk::Queue,
    compute: vk::Queue,
    transfer: vk::Queue,
    present: vk::Queue,
}

pub struct RendererProperties {
    msaa_samples: vk::SampleCountFlags,
    #[allow(unused)]
    anisotropy: bool,
    surface_format: vk::SurfaceFormatKHR,
    queue_family_indices: physical_device::QueueFamilyIndices,
    depth_format: vk::Format,
    present_mode: vk::PresentModeKHR,
}

/// The Renderer struct that holds all render state and is called upon to handle
/// all rendering operations
pub struct Renderer {
    entry: Entry,

    // Renderer Resources
    instance: Instance,
    device: Device,
    queues: Queues,
    physical_device: vk::PhysicalDevice,
    /// For single use buffers.
    /// Used for texture uploads and layout transitions.
    transfer_pool: CommandPool<Transfer>,
    /// For single use buffers.
    /// Used for mip generation.
    graphics_pool: CommandPool<Graphics>,

    properties: RendererProperties,
    /// All of the [window::Window]s constructed from [platform::Window]s.
    windows: HashMap<WindowId, Window>,
    /// All the uploaded [resource_manager::Model]s.
    models: HashMap<String, Model>,
    /// All of the internal [world::World] representations.
    scenes: HashMap<world::WorldId, Scene>,
    /// All the descriptor layouts used in the renderer.
    layouts: DescriptorLayouts,
    material_descriptor_pool: vk::DescriptorPool,

    frames: [Frame; MAX_FRAMES_IN_FLIGHT],
    current_frame: usize,

    // Extensions
    surface_loader: surface::Instance,
    swapchain_loader: swapchain::Device,
    #[cfg(validation)]
    debug_utils_loader: debug_utils::Instance,
    #[cfg(validation)]
    debug_messenger: vk::DebugUtilsMessengerEXT,

    // Events
    window_consumer: events::Consumer<platform::WindowEvent>,
    platform_consumer: events::Consumer<platform::PlatformEvent>,
    world_consumer: events::Consumer<world::events::WorldEvent>,

    // TODO: these are temporary while we get rendering to work
    // render graph should fix this
    graphics_pipeline: GraphicsPipeline,
    /// The size of the output
    extent: vk::Extent2D,
}

impl Renderer {
    pub fn init(
        create_info: RendererCreateInfo,
        window: &platform::Window,
        event_manager: &mut events::EventManager,
    ) -> Result<Self> {
        info!("Intializing Vulkan...");

        let entry = unsafe { Entry::load()? };

        let (instance, debug_utils_loader, debug_messenger) = {
            let app_info = vk::ApplicationInfo::default()
                .application_name(create_info.app_name.as_c_str())
                .application_version(make_version(create_info.app_version))
                .engine_name(create_info.engine_name.as_c_str())
                .engine_version(make_version(create_info.engine_version))
                .api_version(vk::API_VERSION_1_3);

            // Collect extensions
            let mut extensions: Vec<*const i8> = vec![surface::NAME.as_ptr()];

            #[cfg(platform_linux)]
            extensions.push(wayland_surface::NAME.as_ptr());

            let mut instance_create_info =
                vk::InstanceCreateInfo::default().application_info(&app_info);

            #[cfg(validation)]
            let mut debug_create_info: vk::DebugUtilsMessengerCreateInfoEXT;
            #[cfg(validation)]
            {
                info!(target: "vulkan::validation", "using validation layers");
                extensions.push(debug_utils::NAME.as_ptr());

                let severity_flags = vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR;

                let message_type_flags = vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION;

                debug_create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
                    .message_severity(severity_flags)
                    .message_type(message_type_flags)
                    .pfn_user_callback(Some(debug_callback));

                let validation_layers = VALIDATION_LAYERS;

                // check validation layer support
                {
                    let available = unsafe {
                        entry
                            .enumerate_instance_layer_properties()
                            .unwrap_or_default()
                    };
                    for &required in validation_layers {
                        let required = unsafe { CStr::from_ptr(required) };
                        let found = available.iter().any(
                            |ext| unsafe { CStr::from_ptr(ext.layer_name.as_ptr()) } == required,
                        );

                        if !found {
                            return Err(Error::ValidationLayerNotFound(
                                required.to_string_lossy().into_owned(),
                            ));
                        }
                    }
                }

                instance_create_info = instance_create_info
                    .enabled_layer_names(VALIDATION_LAYERS)
                    .push_next(&mut debug_create_info);
            }

            // check required instance extensions
            {
                let available = unsafe {
                    entry
                        .enumerate_instance_extension_properties(None)
                        .unwrap_or_default()
                };
                for &required in &extensions {
                    let required = unsafe { CStr::from_ptr(required) };
                    let found = available.iter().any(
                        |ext| unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) } == required,
                    );

                    if !found {
                        return Err(Error::ExtensionNotFound(
                            required.to_string_lossy().into_owned(),
                        ));
                    }
                }
            }

            instance_create_info = instance_create_info.enabled_extension_names(&extensions);

            let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

            let (debug_utils_loader, debug_messenger) = {
                let loader = debug_utils::Instance::new(&entry, &instance);
                let messenger =
                    unsafe { loader.create_debug_utils_messenger(&debug_create_info, None)? };
                (loader, messenger)
            };

            (instance, debug_utils_loader, debug_messenger)
        };

        let (surface_loader, surface) = {
            let surface = unsafe {
                ash_window::create_surface(
                    &entry,
                    &instance,
                    window.display_handle()?.as_raw(),
                    window.window_handle()?.as_raw(),
                    None,
                )?
            };
            let loader = surface::Instance::new(&entry, &instance);

            (loader, surface)
        };

        // PHYSICAL DEVICE
        let (physical_device, properties) = {
            let (device_info, queues) = physical_device::PhysicalDeviceSelector::new()
                .require_extensions(DEVICE_EXTENSIONS)
                .require(|info| info.features.geometry_shader == vk::TRUE)
                .select(&instance, &surface_loader, surface)
                .ok_or(Error::NoDeviceFound)?;

            info!(
                "Physical device selected: {:#?} (vendor: {}, id: {}, api: {}, driver: {})",
                device_info
                    .properties
                    .device_name_as_c_str()
                    .unwrap_or_default(),
                device_info.properties.vendor_id,
                device_info.properties.device_id,
                device_info.properties.api_version,
                device_info.properties.driver_version
            );

            let formats = unsafe {
                surface_loader.get_physical_device_surface_formats(device_info.handle, surface)?
            };

            let surface_format = formats
                .iter()
                .find(|format| {
                    format.format == vk::Format::B8G8R8A8_SRGB
                        && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
                })
                .copied()
                .unwrap_or(formats[0]);

            let depth_format = *{
                let candidates = &[
                    vk::Format::D32_SFLOAT,
                    vk::Format::D32_SFLOAT_S8_UINT,
                    vk::Format::D24_UNORM_S8_UINT,
                ];
                let features = vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT;

                candidates
                    .iter()
                    .find(|&f| {
                        let properties = unsafe {
                            instance.get_physical_device_format_properties(device_info.handle, *f)
                        };
                        properties.optimal_tiling_features.contains(features)
                    })
                    .ok_or(Error::NoSupportedFormat)
            }?;

            let msaa_samples = *{
                let counts = device_info
                    .properties
                    .limits
                    .framebuffer_color_sample_counts
                    & device_info
                        .properties
                        .limits
                        .framebuffer_depth_sample_counts;
                [
                    vk::SampleCountFlags::TYPE_64,
                    vk::SampleCountFlags::TYPE_32,
                    vk::SampleCountFlags::TYPE_16,
                    vk::SampleCountFlags::TYPE_8,
                    vk::SampleCountFlags::TYPE_4,
                    vk::SampleCountFlags::TYPE_2,
                ]
                .iter()
                .find(|&flag| counts.contains(*flag))
                .unwrap_or(&vk::SampleCountFlags::TYPE_1)
            };

            let present_mode = {
                let modes = unsafe {
                    surface_loader
                        .get_physical_device_surface_present_modes(device_info.handle, surface)?
                };

                *modes
                    .iter()
                    .find(|&mode| *mode == vk::PresentModeKHR::MAILBOX)
                    .unwrap_or(&vk::PresentModeKHR::FIFO)
            };

            let properties = RendererProperties {
                msaa_samples,
                anisotropy: device_info.features.sampler_anisotropy == vk::TRUE,
                surface_format,
                queue_family_indices: queues,
                depth_format,
                present_mode,
            };

            (device_info.handle, properties)
        };

        // destroy the surface as it is no longer needed.
        unsafe { surface_loader.destroy_surface(surface, None) };

        // DEVICE
        let device = {
            let unique_families: HashSet<u32> = [
                properties.queue_family_indices.graphics,
                properties.queue_family_indices.present,
                properties.queue_family_indices.compute,
                properties.queue_family_indices.transfer,
            ]
            .iter()
            .cloned()
            .collect();

            // only one queue per family, so all 1.0 priority
            let queue_priorities = vec![1.0_f32];
            let queue_create_infos: Vec<vk::DeviceQueueCreateInfo> = unique_families
                .iter()
                .map(|&family| {
                    vk::DeviceQueueCreateInfo::default()
                        .queue_family_index(family)
                        .queue_priorities(&queue_priorities)
                })
                .collect();

            let physical_device_features =
                vk::PhysicalDeviceFeatures::default().sampler_anisotropy(true);
            let mut vulkan13_features =
                vk::PhysicalDeviceVulkan13Features::default().dynamic_rendering(true);

            let extensions: Vec<*const i8> = DEVICE_EXTENSIONS
                .iter()
                .map(|name| unsafe { std::mem::transmute(name.as_ptr()) })
                .collect();
            let device_create_info = vk::DeviceCreateInfo::default()
                .queue_create_infos(&queue_create_infos)
                .enabled_features(&physical_device_features)
                .enabled_extension_names(&extensions)
                .push_next(&mut vulkan13_features);

            unsafe { instance.create_device(physical_device, &device_create_info, None)? }
        };

        // QUEUES
        let queues = {
            let indices = &properties.queue_family_indices;
            Queues {
                graphics: unsafe { device.get_device_queue(indices.graphics, 0) },
                present: unsafe { device.get_device_queue(indices.present, 0) },
                compute: unsafe { device.get_device_queue(indices.compute, 0) },
                transfer: unsafe { device.get_device_queue(indices.transfer, 0) },
            }
        };

        // SWAP CHAIN
        let swapchain_loader = swapchain::Device::new(&instance, &device);

        // IN FLIGHT FRAMES
        let frames: Result<Vec<Frame>> = (0..MAX_FRAMES_IN_FLIGHT)
            .map(|_| {
                let command_pool = CommandPool::build(
                    &device,
                    &queues,
                    &properties.queue_family_indices,
                    vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                )?;
                let fence = unsafe {
                    device.create_fence(
                        &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                        None,
                    )?
                };

                Ok(Frame {
                    device: device.clone(),
                    command_pool,
                    fence,
                })
            })
            .collect();
        let frames: [Frame; MAX_FRAMES_IN_FLIGHT] = frames?
            .try_into()
            .unwrap_or_else(|_| panic!("cannot convert frames to array"));

        // LAYOUTS
        let layouts = DescriptorLayouts {
            device: device.clone(),
            scene: {
                let binding = vk::DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::VERTEX);

                let info = vk::DescriptorSetLayoutCreateInfo::default()
                    .bindings(std::slice::from_ref(&binding));

                unsafe { device.create_descriptor_set_layout(&info, None)? }
            },
            object: {
                let binding = vk::DescriptorSetLayoutBinding::default()
                    .binding(1)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::VERTEX);

                let info = vk::DescriptorSetLayoutCreateInfo::default()
                    .bindings(std::slice::from_ref(&binding));

                unsafe { device.create_descriptor_set_layout(&info, None)? }
            },
            material: {
                let binding = vk::DescriptorSetLayoutBinding::default()
                    .binding(2)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT);

                let info = vk::DescriptorSetLayoutCreateInfo::default()
                    .bindings(std::slice::from_ref(&binding));

                unsafe { device.create_descriptor_set_layout(&info, None)? }
            },
        };

        let graphics_pipeline = GraphicsPipeline::build(&device, &layouts, &properties)?;

        // MATERIAL DESCRIPTOR SETS
        let material_descriptor_pool = {
            const MAX_MATERIAL_DESCRIPTOR_SET: u32 = 256;
            let pool_size = vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: MAX_MATERIAL_DESCRIPTOR_SET,
            };
            let pool_info = vk::DescriptorPoolCreateInfo::default()
                .pool_sizes(std::slice::from_ref(&pool_size))
                .max_sets(MAX_MATERIAL_DESCRIPTOR_SET);

            unsafe { device.create_descriptor_pool(&pool_info, None)? }
        };

        let transfer_pool = CommandPool::build(
            &device,
            &queues,
            &properties.queue_family_indices,
            vk::CommandPoolCreateFlags::TRANSIENT,
        )?;
        let graphics_pool = CommandPool::build(
            &device,
            &queues,
            &properties.queue_family_indices,
            vk::CommandPoolCreateFlags::TRANSIENT,
        )?;

        let extent = {
            let size = window.size();
            vk::Extent2D {
                width: size.width,
                height: size.height,
            }
        };

        Ok(Self {
            entry,
            instance,
            device,
            queues,
            physical_device,
            properties,
            transfer_pool,
            graphics_pool,
            windows: HashMap::new(),
            models: HashMap::new(),
            scenes: HashMap::new(),
            layouts,
            material_descriptor_pool,
            frames,
            current_frame: 0,
            surface_loader,
            swapchain_loader,
            debug_utils_loader,
            debug_messenger,

            graphics_pipeline,
            extent,

            window_consumer: event_manager.subscribe(),
            platform_consumer: event_manager.subscribe(),
            world_consumer: event_manager.subscribe(),
        })
    }

    pub fn tick(
        &mut self,
        _delta_time: f32,
        worlds: &HashMap<world::WorldId, world::World>,
        windows: &HashMap<WindowId, platform::Window>,
    ) -> Result<()> {
        let world_events: Vec<_> = self.world_consumer.consume_all().collect();
        for event in world_events {
            match event {
                WorldEvent::Created(id) => {
                    self.scenes.insert(id, Scene::build(self, self.extent, id)?);
                }
                WorldEvent::Destroyed(id) => {
                    self.scenes.remove(&id);
                }
                WorldEvent::EntitySpawn { world, entity } => {
                    let Some(scene) = self.scenes.get(&world) else {
                        continue;
                    };
                    let proxy = SceneProxy::build(self, scene)?;
                    let Some(scene) = self.scenes.get_mut(&world) else {
                        continue;
                    };
                    scene.add_proxy(entity, proxy)?;
                }
                WorldEvent::EntityUpdate { world, entity } => {
                    let Some(world) = worlds.get(&world) else {
                        continue;
                    };
                    if let Some(renderable) = world.get::<components::Renderable>(entity) {
                        self.get_or_upload_model(&renderable.model)?;
                    }
                    let Some(scene) = self.scenes.get_mut(&world.id()) else {
                        continue;
                    };
                    let Some(proxy) = scene.get_proxy(entity) else {
                        continue;
                    };

                    let Some(transform) = world.get::<components::Transform>(entity) else {
                        continue;
                    };
                    proxy.set_model_matrix(transform.matrix());

                    if let Some(renderable) = world.get::<components::Renderable>(entity) {
                        proxy.set_model(
                            self.models
                                .get(&renderable.model)
                                .expect("model should have been loaded"),
                        );
                    };
                    if let Some(camera) = world.get::<components::Camera>(entity) {
                        proxy.set_camera(transform.view(), camera.projection());
                    }
                }
                WorldEvent::EntityDespawn { world, entity } => {
                    let Some(scene) = self.scenes.get_mut(&world) else {
                        continue;
                    };
                    scene.remove_proxy(entity);
                }
            }
        }

        let platform_events: Vec<_> = self.platform_consumer.consume_all().collect();
        for event in platform_events {
            match event {
                PlatformEvent::WindowCreated { id } => {
                    let Some(plat_window) = windows.get(&id) else {
                        continue;
                    };

                    let surface = unsafe {
                        ash_window::create_surface(
                            &self.entry,
                            &self.instance,
                            plat_window.display_handle()?.as_raw(),
                            plat_window.window_handle()?.as_raw(),
                            None,
                        )?
                    };

                    let window_size = plat_window.size();
                    let size = vk::Extent2D {
                        width: window_size.width,
                        height: window_size.height,
                    };

                    let window = window::Window::build(plat_window.id(), self, surface, size)?;
                    self.windows.insert(window.id(), window);

                    debug!("created renderer window with id {}", id.into_raw());
                }
                PlatformEvent::WindowDestroyed { id } => {
                    // in case the window was not destroyed by WindowCloseRequested
                    self.windows.remove(&id);
                }
                PlatformEvent::WindowCloseRequested { id } => {
                    self.windows.remove(&id);
                }
            }
        }

        let window_events: Vec<_> = self.window_consumer.consume_all().collect();
        for event in window_events {
            match event {
                WindowEvent::Resized { id, width, height } => {
                    let Some(window) = self.windows.get(&id) else {
                        continue;
                    };
                    let surface = window.surface();
                    let (swapchain, extent, images) = self.create_swap_chain(
                        surface,
                        vk::Extent2D { width, height },
                        window.swapchain(),
                    )?;
                    let Some(window) = self.windows.get_mut(&id) else {
                        continue;
                    };
                    window.update_swapcahin(swapchain, extent, images);
                }
                WindowEvent::Occluded { id, occluded } => {
                    let Some(window) = self.windows.get_mut(&id) else {
                        continue;
                    };
                    window.set_occluded(occluded);
                }
                // don't care about these
                WindowEvent::FocusChanged { .. } | WindowEvent::ThemeChanged { .. } => {}
            }
        }

        Ok(())
    }

    pub fn render(
        &mut self,
        window: WindowId,
        world: world::WorldId,
        camera: world::Entity,
    ) -> Result<()> {
        let Some(window) = self.windows.get_mut(&window) else {
            return Err(Error::WindowDoesNotExist(window));
        };
        let Some(scene) = self.scenes.get(&world) else {
            return Err(Error::WorldDoesNotExist(world));
        };

        let (swapchain_img, idx) = window.next_image(&self.swapchain_loader)?;
        let (render_finished_semaphore, image_available_semaphore) = window.current_semaphores();
        let size = window.extent();
        let swapchain = window.swapchain();

        let frame = self.get_current_frame();

        unsafe {
            self.device
                .wait_for_fences(std::slice::from_ref(&frame.fence), true, u64::MAX)?;
            self.device
                .reset_fences(std::slice::from_ref(&frame.fence))?;
        }

        let cmd = frame.command_pool.allocate_buffer(&self.device)?;

        unsafe {
            self.device
                .begin_command_buffer(cmd.raw(), &vk::CommandBufferBeginInfo::default())?
        }

        self.transition_image_layout(
            &cmd,
            swapchain_img.image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            1,
            0,
        )?;

        scene.render(self, &cmd, size, swapchain_img.view, camera)?;

        self.transition_image_layout(
            &cmd,
            swapchain_img.image,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::ImageLayout::PRESENT_SRC_KHR,
            1,
            0,
        )?;

        unsafe { self.device.end_command_buffer(cmd.raw())? }

        let submit_info = vk::SubmitInfo::default()
            .wait_dst_stage_mask(std::slice::from_ref(
                &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ))
            .command_buffers(std::slice::from_ref(&cmd))
            .wait_semaphores(std::slice::from_ref(&image_available_semaphore))
            .signal_semaphores(std::slice::from_ref(&render_finished_semaphore));

        unsafe {
            self.device.queue_submit(
                self.queues.graphics,
                std::slice::from_ref(&submit_info),
                frame.fence,
            )?
        }

        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(std::slice::from_ref(&render_finished_semaphore))
            .swapchains(std::slice::from_ref(&swapchain))
            .image_indices(std::slice::from_ref(&idx));

        unsafe {
            self.swapchain_loader
                .queue_present(self.queues.present, &present_info)?
        };

        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
        Ok(())
    }
    fn get_current_frame(&self) -> &Frame {
        &self.frames[self.current_frame]
    }

    // WINDOW MANAGEMENT

    fn create_swap_chain(
        &self,
        surface: vk::SurfaceKHR,
        window_size: vk::Extent2D,
        old_swapchain: vk::SwapchainKHR,
    ) -> Result<(vk::SwapchainKHR, vk::Extent2D, Vec<window::SwapchainImage>)> {
        let capabilities = unsafe {
            self.surface_loader
                .get_physical_device_surface_capabilities(self.physical_device, surface)?
        };

        let extent = if capabilities.current_extent.width != u32::MAX {
            capabilities.current_extent
        } else {
            vk::Extent2D {
                width: window_size.width.clamp(
                    capabilities.min_image_extent.width,
                    capabilities.max_image_extent.width,
                ),
                height: window_size.height.clamp(
                    capabilities.min_image_extent.height,
                    capabilities.max_image_extent.height,
                ),
            }
        };

        let mut image_count = capabilities.min_image_count + 1;
        if capabilities.max_image_count > 0 && image_count > capabilities.max_image_count {
            image_count = capabilities.max_image_count;
        }

        let indices = &self.properties.queue_family_indices;
        // Deduplicate — concurrent mode requires unique family indices
        let mut unique_indices: Vec<u32> =
            vec![indices.graphics, indices.present, indices.transfer];
        unique_indices.sort_unstable();
        unique_indices.dedup();

        let (sharing_mode, indices_slice): (vk::SharingMode, &[u32]) = if unique_indices.len() > 1 {
            (vk::SharingMode::CONCURRENT, &unique_indices)
        } else {
            (vk::SharingMode::EXCLUSIVE, &[])
        };

        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(self.properties.surface_format.format)
            .image_color_space(self.properties.surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(sharing_mode)
            .queue_family_indices(indices_slice)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(self.properties.present_mode)
            .clipped(true)
            .old_swapchain(old_swapchain);

        let swapchain = unsafe { self.swapchain_loader.create_swapchain(&create_info, None)? };
        let images = unsafe { self.swapchain_loader.get_swapchain_images(swapchain)? };

        unsafe { self.swapchain_loader.destroy_swapchain(old_swapchain, None) };

        let swap_images = images
            .into_iter()
            .map(|image| {
                let view = self.create_image_view(
                    image,
                    self.properties.surface_format.format,
                    vk::ImageAspectFlags::COLOR,
                    1,
                )?;
                Ok(window::SwapchainImage { image, view })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok((swapchain, extent, swap_images))
    }

    // UPLOADING TO THE RENDERER

    pub fn upload_model(&mut self, model: resource_manager::Model) -> Result<&Model> {
        let primitives = model
            .meshes()
            .iter()
            .flat_map(|m| m.primitives().iter())
            .map(|p| self.upload_primitive(p))
            .collect::<Result<_>>()?;

        let textures: Vec<_> = model
            .textures()
            .iter()
            .map(|t| self.upload_texture(t))
            .collect::<Result<_>>()?;

        let material_count = model.materials().len();
        // Allocate one set per material
        let layouts: Vec<vk::DescriptorSetLayout> = vec![self.layouts.material; material_count];

        let material_sets: Vec<vk::DescriptorSet> = if material_count > 0 {
            let alloc_info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(self.material_descriptor_pool)
                .set_layouts(&layouts);

            unsafe { self.device.allocate_descriptor_sets(&alloc_info)? }
        } else {
            Vec::new()
        };

        // Write the base-colour sampler into each set that has one
        for (i, mat) in model.materials().iter().enumerate() {
            let Some(&tex_idx) = mat.base_color_texture().as_ref() else {
                continue; // leave this set in its default (null) state
            };

            let tex = &textures[tex_idx];

            let image_info = vk::DescriptorImageInfo::default()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(tex.view)
                .sampler(tex.sampler);

            let write = vk::WriteDescriptorSet::default()
                .dst_set(material_sets[i])
                .dst_binding(2) // matches layouts.material
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&image_info));

            unsafe { self.device.update_descriptor_sets(&[write], &[]) };
        }

        self.models.insert(
            model.name().to_string(),
            Model {
                name: model.name().to_owned(),
                primitives,
                textures,
                materials: model.materials().to_vec(),
                material_sets,
            },
        );
        Ok(self.models.get(model.name()).unwrap())
    }

    fn get_or_upload_model(&mut self, name: &str) -> Result<&Model> {
        if self.models.contains_key(name) {
            return Ok(self.models.get(name).unwrap());
        }

        self.upload_model(resource_manager::ResourceManager::load_model(name)?)
    }

    fn upload_primitive(&self, prim: &resource_manager::Primitive) -> Result<Primitive> {
        let vertices: Vec<Vertex> = prim
            .positions()
            .iter()
            .enumerate()
            .map(|(i, &position)| Vertex {
                position,
                normal: prim.normals().get(i).copied().unwrap_or([0.0, 1.0, 0.0]),
                texcoord: prim.texcoords().get(i).copied().unwrap_or([0.0, 0.0]),
            })
            .collect();

        let (vertex_buffer, vertex_buffer_memory) =
            self.upload_slice(&vertices, vk::BufferUsageFlags::VERTEX_BUFFER)?;

        let (index_buffer, index_buffer_memory) =
            self.upload_slice(prim.indices(), vk::BufferUsageFlags::INDEX_BUFFER)?;

        Ok(Primitive {
            device: self.device.clone(),
            vertex_buffer,
            vertex_buffer_memory,
            index_buffer,
            index_buffer_memory,
            index_count: prim.indices().len() as u32,
            material: *prim.material(),
        })
    }
    fn upload_texture(&self, tex: &resource_manager::Texture) -> Result<Texture> {
        let mip_levels = Self::mip_levels(*tex.width(), *tex.height());
        let size = (tex.pixels().len()) as vk::DeviceSize;
        let format = vk::Format::R8G8B8A8_SRGB;

        let (staging_buf, staging_mem) = self.create_buffer(
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        unsafe {
            let ptr = self
                .device
                .map_memory(staging_mem, 0, size, vk::MemoryMapFlags::empty())?
                as *mut u8;
            ptr.copy_from_nonoverlapping(tex.pixels().as_ptr(), tex.pixels().len());
            self.device.unmap_memory(staging_mem);
        }

        let (image, memory) = self.create_image(
            vk::Extent2D {
                width: *tex.width(),
                height: *tex.height(),
            },
            format,
            vk::ImageTiling::OPTIMAL,
            // TRANSFER_SRC needed for mip creation
            vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::SAMPLED,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            (mip_levels, vk::SampleCountFlags::TYPE_1),
        )?;

        let cmd = self.graphics_pool.begin_single_time(&self.device)?;

        self.transition_image_layout(
            &cmd,
            image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            mip_levels, // all mip levels start undefined
            0,
        )?;

        let region = vk::BufferImageCopy::default()
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .image_extent(vk::Extent3D {
                width: *tex.width(),
                height: *tex.height(),
                depth: 1,
            });

        unsafe {
            self.device.cmd_copy_buffer_to_image(
                cmd.raw(),
                staging_buf,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region],
            );
        }

        self.generate_mipmaps(&cmd, image, *tex.width(), *tex.height(), mip_levels)?;
        cmd.end_and_submit(&self.device)?;

        // TODO: destroy buffer when it is no longer needed (maybe with VMA)
        // currently it is destroyed too early, it is still in use
        // unsafe {
        //     self.device.destroy_buffer(staging_buf, None);
        //     self.device.free_memory(staging_mem, None);
        // }

        let view =
            self.create_image_view(image, format, vk::ImageAspectFlags::COLOR, mip_levels)?;
        let sampler = self.create_sampler(mip_levels)?;

        Ok(Texture {
            device: self.device.clone(),
            image,
            memory,
            view,
            sampler,
            mip_levels,
        })
    }

    // IMAGE UTILITIES

    fn create_image_view(
        &self,
        image: vk::Image,
        format: vk::Format,
        aspect_flags: vk::ImageAspectFlags,
        mip_levels: u32,
    ) -> Result<vk::ImageView> {
        let create_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: aspect_flags,
                base_mip_level: 0,
                level_count: mip_levels,
                base_array_layer: 0,
                layer_count: 1,
            });

        Ok(unsafe { self.device.create_image_view(&create_info, None)? })
    }
    fn create_image(
        &self,
        size: vk::Extent2D,
        format: vk::Format,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        properties: vk::MemoryPropertyFlags,
        (mip_levels, num_samples): (u32, vk::SampleCountFlags),
    ) -> Result<(vk::Image, vk::DeviceMemory)> {
        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D {
                width: size.width,
                height: size.height,
                depth: 1,
            })
            .mip_levels(mip_levels)
            .array_layers(1)
            .samples(num_samples)
            .tiling(tiling)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let image = unsafe { self.device.create_image(&image_info, None)? };

        let mem_req = unsafe { self.device.get_image_memory_requirements(image) };

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_req.size)
            .memory_type_index(self.find_memory_type(mem_req.memory_type_bits, properties)?);

        let memory = unsafe { self.device.allocate_memory(&alloc_info, None)? };

        unsafe { self.device.bind_image_memory(image, memory, 0)? };

        Ok((image, memory))
    }
    fn generate_mipmaps(
        &self,
        cmd: &CommandBuffer,
        image: vk::Image,
        width: u32,
        height: u32,
        mip_levels: u32,
    ) -> Result<()> {
        let mut mip_width = width;
        let mut mip_height = height;

        for level in 1..mip_levels {
            let base_mip = level - 1;
            // Transition previous level: TRANSFER_DST → TRANSFER_SRC
            self.transition_image_layout(
                cmd,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                1,
                base_mip,
            )?;

            let next_w = (mip_width / 2).max(1);
            let next_h = (mip_height / 2).max(1);

            let blit = vk::ImageBlit::default()
                .src_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: level - 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .src_offsets([
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D {
                        x: mip_width as i32,
                        y: mip_height as i32,
                        z: 1,
                    },
                ])
                .dst_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: level,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .dst_offsets([
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D {
                        x: next_w as i32,
                        y: next_h as i32,
                        z: 1,
                    },
                ]);

            unsafe {
                self.device.cmd_blit_image(
                    cmd.raw(),
                    image,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[blit],
                    vk::Filter::LINEAR,
                );
            }

            // Previous level is fully consumed — transition to shader-readable
            self.transition_image_layout(
                cmd,
                image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                1,
                base_mip,
            )?;

            mip_width = next_w;
            mip_height = next_h;
        }

        // Transition the final mip level (never used as a blit source)
        self.transition_image_layout(
            cmd,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            1,
            mip_levels - 1,
        )
    }
    fn create_sampler(&self, mip_levels: u32) -> Result<vk::Sampler> {
        let props = unsafe {
            self.instance
                .get_physical_device_properties(self.physical_device)
        };
        let max_aniso = props.limits.max_sampler_anisotropy;

        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .mip_lod_bias(0.0)
            .anisotropy_enable(true)
            .max_anisotropy(max_aniso) // use hardware maximum
            .compare_enable(false)
            .min_lod(0.0)
            .max_lod(mip_levels as f32)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false);

        Ok(unsafe { self.device.create_sampler(&sampler_info, None)? })
    }

    // BUFFER UTILITIES

    fn upload_slice<T: Copy>(
        &self,
        data: &[T],
        usage: vk::BufferUsageFlags,
    ) -> Result<(vk::Buffer, vk::DeviceMemory)> {
        let size = std::mem::size_of_val(data) as vk::DeviceSize;

        let (staging_buf, staging_mem) = self.create_buffer(
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        unsafe {
            let ptr = self
                .device
                .map_memory(staging_mem, 0, size, vk::MemoryMapFlags::empty())?
                as *mut T;
            ptr.copy_from_nonoverlapping(data.as_ptr(), data.len());
            self.device.unmap_memory(staging_mem);
        }

        let (device_buf, device_mem) = self.create_buffer(
            size,
            vk::BufferUsageFlags::TRANSFER_DST | usage,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        self.copy_buffer(staging_buf, device_buf, size)?;
        // TODO: destroy buffer when it is no longer needed (maybe with VMA)
        // currently it is destroyed too early, it is still in use
        // unsafe {
        //     self.device.destroy_buffer(staging_buf, None);
        //     self.device.free_memory(staging_mem, None);
        // }

        Ok((device_buf, device_mem))
    }
    fn copy_buffer(&self, src: vk::Buffer, dst: vk::Buffer, size: vk::DeviceSize) -> Result<()> {
        let cmd = self.transfer_pool.begin_single_time(&self.device)?;

        let region = vk::BufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size,
        };
        unsafe { self.device.cmd_copy_buffer(cmd.raw(), src, dst, &[region]) };

        cmd.end_and_submit(&self.device)?;
        Ok(())
    }
    fn create_buffer(
        &self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Buffer, vk::DeviceMemory)> {
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { self.device.create_buffer(&buffer_info, None)? };
        let requirements = unsafe { self.device.get_buffer_memory_requirements(buffer) };
        let memory_type = self.find_memory_type(requirements.memory_type_bits, properties)?;

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(requirements.size)
            .memory_type_index(memory_type);

        let memory = unsafe { self.device.allocate_memory(&alloc_info, None)? };

        unsafe { self.device.bind_buffer_memory(buffer, memory, 0)? };

        Ok((buffer, memory))
    }

    // EXTRA UTILS

    fn mip_levels(width: u32, height: u32) -> u32 {
        // How many times can we halve the larger dimension before hitting 1px?
        (width.max(height) as f32).log2().floor() as u32 + 1
    }
    fn find_memory_type(
        &self,
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<u32> {
        let mem_props = unsafe {
            self.instance
                .get_physical_device_memory_properties(self.physical_device)
        };

        (0..mem_props.memory_type_count)
            .find(|&i| {
                let type_match = type_filter & (1 << i) != 0;
                let prop_match = mem_props.memory_types[i as usize]
                    .property_flags
                    .contains(properties);
                type_match && prop_match
            })
            .ok_or(Error::NoSuitableMemoryType)
    }
    fn create_shader_module(
        device: &Device,
        shader: &'static shaders::Shader,
    ) -> Result<vk::ShaderModule> {
        let code = shader.code_as_u32();
        let info = vk::ShaderModuleCreateInfo::default().code(code.as_slice());
        Ok(unsafe { device.create_shader_module(&info, None)? })
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().ok();
        }
        info!("cleaning up renderer");

        self.scenes.clear();
        self.models.clear();
        self.windows.clear();
        self.frames.iter().for_each(|f| f.destroy());
        self.graphics_pipeline.destroy();
        self.layouts.destroy();

        self.graphics_pool.destroy();
        self.transfer_pool.destroy();
        unsafe {
            self.device
                .destroy_descriptor_pool(self.material_descriptor_pool, None);

            self.device.destroy_device(None);
            #[cfg(validation)]
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_messenger, None);

            self.instance.destroy_instance(None);
        }
    }
}

#[cfg(validation)]
extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut c_void,
) -> vk::Bool32 {
    let message = unsafe { CStr::from_ptr((*callback_data).p_message).to_string_lossy() };

    match severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            error!(target: "vulkan::validation", "{}", message)
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            warn! (target: "vulkan::validation", "{}", message)
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            info! (target: "vulkan::validation", "{}", message)
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            debug!(target: "vulkan::validation", "{}", message)
        }
        _ => trace!(target: "vulkan::validation", "{}", message),
    }

    vk::FALSE
}
