#[cfg(validation)]
use std::os::raw::c_void;
use std::{
    collections::{HashMap, HashSet},
    ffi::{CStr, CString},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[cfg(validation)]
use ash::ext::debug_utils;
#[cfg(platform_linux)]
use ash::khr::wayland_surface;
use ash::{
    Device, Entry,
    khr::{surface, swapchain},
    vk,
};
use events::EventManager;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use tracing::{debug, info};
#[cfg(validation)]
use tracing::{error, trace, warn};

use platform::{PlatformEvent, WindowEvent, WindowId};
use world::events::WorldEvent;

mod utils;
use ::utils::*;
use utils::*;

mod errors;
pub use errors::{Error, Result};

mod scene;
use scene::Scene;

mod window;
use window::Window;

mod resources;
use resources::{
    buffer::{IndexBuffer, VertexBuffer},
    command_pool::{CommandPool, Graphics},
    device::RenderDevice,
    image::{Image, SwapchainImage},
    model::*,
};

use crate::resources::device::Garbage;

mod physical_device;
mod pipeline;
mod render_pass;

/// The maximum numer of renderables in a scene.
/// Used to construct Ubo samples.
/// TODO: find a way to set this limit dynamically or have a error when the limit is reached.
const MAX_RENDERABLES: u32 = 100;
/// TODO: also find a way to do this dynamically
const MAX_MATERIAL_DESCRIPTOR_SET: u32 = 256;

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

/// This struct is owned by [Renderer] and stores
/// all the different descriptor set layouts used by
/// the renderer.
/// Every field should be a descriptor set layout with a
/// propper comment explain what the layout is and where
/// it is used.
struct DescriptorLayouts {
    // TODO: much better comments for descriptor set layouts
    /// Per scene layout. Holds view & proj matrices for rendering.
    scene: vk::DescriptorSetLayout,
    /// Per object layout. For model matrix.
    object: vk::DescriptorSetLayout,
    /// Per material layout. For texture descriptor.
    material: vk::DescriptorSetLayout,
}

impl DescriptorLayouts {
    fn destroy(&self, device: &mut RenderDevice) {
        device.destroy(Garbage::DescriptorSetLayout(self.scene));
        device.destroy(Garbage::DescriptorSetLayout(self.object));
        device.destroy(Garbage::DescriptorSetLayout(self.material));
    }
}

pub struct RendererCreateInfo {
    pub engine_name: CString,
    pub engine_version: Version,
    pub app_name: CString,
    pub app_version: Version,
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
    render_device: RenderDevice,

    // Heavy renderer state:
    /// All of the [window::Window]s constructed from [platform::Window]s.
    windows: HashMap<WindowId, Window>,
    /// All the uploaded [resource_manager::Model]s.
    models: HashMap<String, Model>,
    /// All of the internal [world::World] representations.
    scenes: HashMap<world::WorldId, Scene>,
    material_descriptor_pool: vk::DescriptorPool,

    frames: [Frame; MAX_FRAMES_IN_FLIGHT],
    current_frame: Arc<AtomicU64>,

    #[cfg(validation)]
    debug_utils_loader: debug_utils::Instance,
    #[cfg(validation)]
    debug_messenger: vk::DebugUtilsMessengerEXT,

    // Events
    /// TODO: will be used to create listeners for scenes
    #[allow(unused)]
    event_manager: EventManager,
    window_consumer: events::Consumer<platform::WindowEvent>,
    platform_consumer: events::Consumer<platform::PlatformEvent>,
    world_consumer: events::Consumer<world::events::WorldEvent>,

    /// The size of the output
    /// TODO: should be removed once we get the frame graph to
    /// handle transient resources
    extent: vk::Extent2D,
}

impl Renderer {
    pub fn init(
        create_info: RendererCreateInfo,
        window: &platform::Window,
        event_manager: events::EventManager,
    ) -> Result<Self> {
        info!("Intializing Vulkan...");

        let entry = unsafe { Entry::load()? };

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
                    let found = available
                        .iter()
                        .any(|ext| unsafe { CStr::from_ptr(ext.layer_name.as_ptr()) } == required);

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
                let found = available
                    .iter()
                    .any(|ext| unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) } == required);

                if !found {
                    return Err(Error::ExtensionNotFound(
                        required.to_string_lossy().into_owned(),
                    ));
                }
            }
        }

        instance_create_info = instance_create_info.enabled_extension_names(&extensions);

        let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

        #[cfg(validation)]
        let (debug_utils_loader, debug_messenger) = {
            let loader = debug_utils::Instance::new(&entry, &instance);
            let messenger =
                unsafe { loader.create_debug_utils_messenger(&debug_create_info, None)? };
            (loader, messenger)
        };

        // this is a temporary surface, it is destroyed very soon
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
            let mut vulkan12_features =
                vk::PhysicalDeviceVulkan12Features::default().buffer_device_address(true);
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
                .push_next(&mut vulkan12_features)
                .push_next(&mut vulkan13_features);

            unsafe { instance.create_device(physical_device, &device_create_info, None)? }
        };

        let current_frame = Arc::new(AtomicU64::new(0));

        // RENDER DEVICE
        let render_device = RenderDevice::new(
            instance.clone(),
            device.clone(),
            surface_loader.clone(),
            physical_device,
            properties,
            current_frame.clone(),
            event_manager.clone(),
        )?;

        // IN FLIGHT FRAMES
        let build_frame = || -> Result<Frame> {
            let command_pool = CommandPool::build(
                &device,
                &render_device.queues,
                &render_device.properties.queue_family_indices,
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
        };
        let frames = [build_frame()?, build_frame()?];
        // nightly currently allows:
        // let frames: [Frame; MAX_FRAMES_IN_FLIGHT] = std::array::try_from_fn(|_| build_frame())?;
        // could be nice in the future

        // MATERIAL DESCRIPTOR SETS
        let material_descriptor_pool = {
            let pool_size = vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: MAX_MATERIAL_DESCRIPTOR_SET,
            };
            let pool_info = vk::DescriptorPoolCreateInfo::default()
                .pool_sizes(std::slice::from_ref(&pool_size))
                .max_sets(MAX_MATERIAL_DESCRIPTOR_SET);

            unsafe { device.create_descriptor_pool(&pool_info, None)? }
        };

        let extent = {
            let size = window.size();
            vk::Extent2D {
                width: size.width,
                height: size.height,
            }
        };
        Ok(Self {
            entry,
            render_device,
            windows: HashMap::new(),
            models: HashMap::new(),
            scenes: HashMap::new(),
            material_descriptor_pool,
            frames,
            current_frame,
            #[cfg(validation)]
            debug_utils_loader,
            #[cfg(validation)]
            debug_messenger,

            extent,

            window_consumer: event_manager.subscribe(),
            platform_consumer: event_manager.subscribe(),
            world_consumer: event_manager.subscribe(),
            event_manager,
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
                    let scene = Scene::build(self.render_device.clone(), self.extent, id)?;
                    self.scenes.insert(id, scene);
                }
                WorldEvent::Destroyed(id) => {
                    self.scenes.remove(&id);
                }
                WorldEvent::EntitySpawn { .. }
                | WorldEvent::EntityUpdate { .. }
                | WorldEvent::EntityDespawn { .. } => {}
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
                            &self.render_device.instance,
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

        self.scenes.values_mut().try_for_each(|scene| {
            let Some(world) = worlds.get(&scene.world()) else {
                return Ok(());
            };
            scene.tick(world)
        })?;

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

        let (render_finished_semaphore, image_available_semaphore) = window.current_semaphores();
        let size = window.extent();
        let swapchain = window.swapchain();
        let (swapchain_img, idx) = window.next_image(&self.render_device.swapchain_loader)?;

        let frame = &self.frames[self.current_frame.load(Ordering::Relaxed) as usize];

        unsafe {
            self.render_device.device.wait_for_fences(
                std::slice::from_ref(&frame.fence),
                true,
                u64::MAX,
            )?;
            self.render_device
                .device
                .reset_fences(std::slice::from_ref(&frame.fence))?;
        }

        let cmd = frame.command_pool.allocate_buffer()?;

        unsafe {
            self.render_device
                .device
                .begin_command_buffer(cmd.raw(), &vk::CommandBufferBeginInfo::default())?
        }

        swapchain_img.transition_image_layout(
            &cmd,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        )?;

        scene.render(&self.models, &cmd, size, swapchain_img.view(), camera)?;

        swapchain_img.transition_image_layout(
            &cmd,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::ImageLayout::PRESENT_SRC_KHR,
        )?;

        unsafe { self.render_device.device.end_command_buffer(cmd.raw())? }

        let submit_info = vk::SubmitInfo::default()
            .wait_dst_stage_mask(std::slice::from_ref(
                &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ))
            .command_buffers(std::slice::from_ref(&cmd))
            .wait_semaphores(std::slice::from_ref(&image_available_semaphore))
            .signal_semaphores(std::slice::from_ref(&render_finished_semaphore));

        unsafe {
            self.render_device.device.queue_submit(
                self.render_device.queues.graphics,
                std::slice::from_ref(&submit_info),
                frame.fence,
            )?
        }

        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(std::slice::from_ref(&render_finished_semaphore))
            .swapchains(std::slice::from_ref(&swapchain))
            .image_indices(std::slice::from_ref(&idx));

        unsafe {
            self.render_device
                .swapchain_loader
                .queue_present(self.render_device.queues.present, &present_info)?
        };

        self.current_frame.store(
            ((self.current_frame() + 1) % MAX_FRAMES_IN_FLIGHT)
                .try_into()
                .unwrap(),
            Ordering::Relaxed,
        );
        Ok(())
    }

    // WINDOW MANAGEMENT

    fn create_swap_chain(
        &self,
        surface: vk::SurfaceKHR,
        window_size: vk::Extent2D,
        old_swapchain: vk::SwapchainKHR,
    ) -> Result<(vk::SwapchainKHR, vk::Extent2D, Vec<SwapchainImage>)> {
        let capabilities = unsafe {
            self.render_device
                .surface_loader
                .get_physical_device_surface_capabilities(
                    self.render_device.physical_device,
                    surface,
                )?
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

        let indices = &self.render_device.properties.queue_family_indices;
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
            .image_format(self.render_device.properties.surface_format.format)
            .image_color_space(self.render_device.properties.surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(sharing_mode)
            .queue_family_indices(indices_slice)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(self.render_device.properties.present_mode)
            .clipped(true)
            .old_swapchain(old_swapchain);

        let swapchain = unsafe {
            self.render_device
                .swapchain_loader
                .create_swapchain(&create_info, None)?
        };
        let images = unsafe {
            self.render_device
                .swapchain_loader
                .get_swapchain_images(swapchain)?
        };

        let swap_images = images
            .into_iter()
            .map(|image| {
                SwapchainImage::new(
                    &self.render_device,
                    image,
                    self.render_device.properties.surface_format.format,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        unsafe {
            self.render_device
                .swapchain_loader
                .destroy_swapchain(old_swapchain, None)
        };

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
            .map(|t| Image::upload_texture(self.render_device.clone(), t))
            .collect::<Result<_>>()?;

        let material_count = model.materials().len();
        // Allocate one set per material
        let layouts: Vec<vk::DescriptorSetLayout> =
            vec![self.render_device.layouts.material; material_count];

        let material_sets: Vec<vk::DescriptorSet> = if material_count > 0 {
            let alloc_info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(self.material_descriptor_pool)
                .set_layouts(&layouts);

            unsafe {
                self.render_device
                    .device
                    .allocate_descriptor_sets(&alloc_info)?
            }
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
                .image_view(tex.image.view())
                .sampler(tex.sampler);

            let write = vk::WriteDescriptorSet::default()
                .dst_set(material_sets[i])
                .dst_binding(2) // matches layouts.material
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&image_info));

            unsafe {
                self.render_device
                    .device
                    .update_descriptor_sets(&[write], &[])
            };
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

    fn upload_primitive(&mut self, prim: &resource_manager::Primitive) -> Result<Primitive> {
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

        let vertex_buffer = VertexBuffer::upload_slice(self.render_device.clone(), &vertices)?;
        let index_buffer = IndexBuffer::upload_slice(self.render_device.clone(), prim.indices())?;

        Ok(Primitive {
            vertex_buffer,
            index_buffer,
            index_count: prim.indices().len() as u32,
            material: *prim.material(),
        })
    }

    // IMAGE UTILITIES

    fn create_sampler(device: &RenderDevice, mip_levels: u32) -> Result<vk::Sampler> {
        let props = unsafe {
            device
                .instance
                .get_physical_device_properties(device.physical_device)
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
            .anisotropy_enable(true) // TODO: use the detected prpoerty
            .max_anisotropy(max_aniso) // use hardware maximum
            .compare_enable(false)
            .min_lod(0.0)
            .max_lod(mip_levels as f32)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false);

        Ok(unsafe { device.device.create_sampler(&sampler_info, None)? })
    }

    // EXTRA UTILS

    fn create_shader_module(
        device: &Device,
        shader: &'static shaders::Shader,
    ) -> Result<vk::ShaderModule> {
        let code = shader.code_as_u32();
        let info = vk::ShaderModuleCreateInfo::default().code(code.as_slice());
        Ok(unsafe { device.create_shader_module(&info, None)? })
    }
    fn current_frame(&self) -> usize {
        self.current_frame
            .load(std::sync::atomic::Ordering::Relaxed) as usize
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.render_device.device.device_wait_idle().ok();
        }
        info!("cleaning up renderer");

        self.scenes.clear();
        self.models.clear();
        self.windows.clear();
        self.frames.iter().for_each(|f| f.destroy());
        self.render_device.shutdown();

        self.render_device
            .layouts
            .destroy(&self.render_device.device);
        self.render_device.graphics_pool.destroy();
        self.render_device.transfer_pool.destroy();
        unsafe {
            self.render_device
                .device
                .destroy_descriptor_pool(self.material_descriptor_pool, None);

            self.render_device.device.destroy_device(None);
            #[cfg(validation)]
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_messenger, None);

            self.render_device.instance.destroy_instance(None);
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
