#![doc = include_str!("../README.md")]

#[cfg(validation)]
use std::os::raw::c_void;
use std::{
    collections::{HashMap, HashSet},
    ffi::{CStr, CString},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
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
use dirk_player::PlayerId;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use tracing::{debug, info};
#[cfg(validation)]
use tracing::{error, trace, warn};

use dirk_platform::{PlatformEvent, WindowEvent, WindowId};
use dirk_universe::{Universe, UniverseBuilder};

mod utils;
use dirk_utils::Version;
use resources::descriptors::DescriptorLayouts;
use utils::{Frame, RendererProperties, Vertex, make_version};

mod errors;
pub use errors::{Error, Result};

mod window;
use window::Window;

mod resources;
use resources::{command_pool::CommandPool, device::RenderDevice};

mod proxy;
use proxy::{
    scene::SceneManager,
    systems::{
        RendererMeshSystem, RendererPlayerSystem, RendererTransformSystem, RendererUniverseSystem,
    },
};

mod render_commands;
use render_commands::RenderCommandReceiver;

use crate::{proxy::PlayerProxy, resources::queues::QueueType};

mod models;
mod physical_device;
mod pipeline;
mod render_pass;

const MAX_FRAMES_IN_FLIGHT: usize = 2;
const DEVICE_EXTENSIONS: &[&str] =
    &[unsafe { std::str::from_utf8_unchecked(swapchain::NAME.to_bytes()) }];
#[cfg(validation)]
const VALIDATION_LAYERS: &[*const i8] = &[c"VK_LAYER_KHRONOS_validation".as_ptr()];

/// The information needed to create the renderer. This is primarily metadata
/// used for Vulkan initialisation.
pub struct RendererCreateInfo {
    /// The name of the engine. Used for vulkan initialisation.
    pub engine_name: CString,
    /// The version of the engine. Used for vulkan initialisation.
    pub engine_version: Version,
    /// The name of the application. Used for vulkan initialisation.
    pub app_name: CString,
    /// The version of the application. Used for vulkan initialisation.
    pub app_version: Version,
}

/// The Renderer struct that holds all render state and is called upon to handle
/// all rendering operations
pub struct Renderer {
    // Heavy renderer state:
    /// All of the [`window::Window`]s constructed from [`platform::Window`]s.
    windows: HashMap<WindowId, Window>,
    /// All of the internal [`world::World`] representations.
    scene_manager: SceneManager,
    /// The management for all the models.
    models: models::ModelRegistry,
    /// Maps each live [`PlayerId`] to its proxy.
    players: HashMap<PlayerId, PlayerProxy>,

    frames: [Frame; MAX_FRAMES_IN_FLIGHT],
    current_frame: Arc<AtomicUsize>,

    // Events
    window_consumer: dirk_events::Consumer<dirk_platform::WindowEvent>,
    platform_consumer: dirk_events::Consumer<dirk_platform::PlatformEvent>,
    player_spawn_consumer: dirk_events::Consumer<dirk_player::PlayerSpawned>,
    player_despawn_consumer: dirk_events::Consumer<dirk_player::PlayerDespawned>,

    /// These receive all the commands from the game thread.
    receivers: Vec<RenderCommandReceiver>,

    // last as should be dropped last
    render_device: RenderDevice,
}

impl Renderer {
    /// Renderer initialisation. Creates all Vulkan & other renderer objects.
    ///
    /// # Errors
    ///
    /// Plenty of Vulkan & platform errors can occur during renderer intializing
    // TODO: shorten this function by dividing into smaller functions. maybe
    // even create a separate init module with all the init functions in it
    #[allow(clippy::too_many_lines)]
    pub fn init(
        create_info: &RendererCreateInfo,
        window: &dirk_platform::Window,
        event_manager: &dirk_events::EventManager,
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
        let debug_messenger = {
            let loader = debug_utils::Instance::new(&entry, &instance);
            unsafe { loader.create_debug_utils_messenger(&debug_create_info, None)? }
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
            .copied()
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
                .map(|name| name.as_ptr().cast())
                .collect();
            let device_create_info = vk::DeviceCreateInfo::default()
                .queue_create_infos(&queue_create_infos)
                .enabled_features(&physical_device_features)
                .enabled_extension_names(&extensions)
                .push_next(&mut vulkan12_features)
                .push_next(&mut vulkan13_features);

            unsafe { instance.create_device(physical_device, &device_create_info, None)? }
        };

        let current_frame = Arc::new(AtomicUsize::new(0));

        // RENDER DEVICE
        let render_device = RenderDevice::new(
            entry.clone(),
            instance.clone(),
            device.clone(),
            physical_device,
            properties,
            current_frame.clone(),
            #[cfg(validation)]
            debug_messenger,
        )?;

        // IN FLIGHT FRAMES
        let build_frame = || -> Result<Frame> {
            let command_pool = CommandPool::build(
                &device,
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

        // TODO: should be removed once we get the frame graph to handle transient resources
        let extent = {
            let size = window.size();
            vk::Extent2D {
                width: size.width,
                height: size.height,
            }
        };

        let models = models::ModelRegistry::new(&render_device, event_manager)?;

        let scene_manager = SceneManager::init(&render_device, extent)?;

        // create the first window as we do not receive a create event for it
        let window = window::Window::build(&render_device, window)?;
        let mut windows = HashMap::new();
        windows.insert(window.id(), window);

        Ok(Self {
            render_device,

            windows,
            scene_manager,
            players: HashMap::new(),
            models,

            frames,
            current_frame,

            window_consumer: event_manager.subscribe(),
            platform_consumer: event_manager.subscribe(),
            player_spawn_consumer: event_manager.subscribe(),
            player_despawn_consumer: event_manager.subscribe(),
            receivers: Vec::new(),
        })
    }

    /// Returns a [`UniverseBuilder`] that is populated with [`Renderer`] systems.
    pub fn universe_builder(&mut self) -> UniverseBuilder {
        let (uni_sender, uni_receiver) = render_commands::channel();
        let (mesh_sender, mesh_receiver) = render_commands::channel();
        let (trans_sender, trans_receiver) = render_commands::channel();
        let (player_sender, player_receiver) = render_commands::channel();

        self.receivers.push(uni_receiver);
        self.receivers.push(mesh_receiver);
        self.receivers.push(trans_receiver);
        self.receivers.push(player_receiver);

        Universe::builder()
            .with_universe_system(RendererUniverseSystem::new(uni_sender))
            .with_component_system(RendererMeshSystem::new(mesh_sender))
            .with_component_system(RendererTransformSystem::new(trans_sender))
            .with_component_system(RendererPlayerSystem::new(player_sender))
    }

    /// Ticks the renderer. Used to improve the various internal representations
    /// of external engine objects.
    /// The renderer listens to events to properly sync windows, scenes, ...
    ///
    /// # Errors
    ///
    /// Errors can occur when updating the scene & world (if one is missing for example)
    /// Some platform errors can also occur when handling windows
    ///
    /// # Panics
    ///
    /// Will panic if the scene object does not exist for the specified
    /// world (unless in [`WorldEvent::Created`] or [`WorldEvent::Destroyed`].
    pub fn tick(
        &mut self,
        _delta_time: f64,
        windows: &HashMap<WindowId, dirk_platform::Window>,
    ) -> Result<()> {
        for event in self.player_spawn_consumer.consume_all() {
            self.players.insert(event.id, event.into());
        }

        for event in self.player_despawn_consumer.consume_all() {
            self.players.remove(&event.id);
        }

        // Temporarily move receivers out for the borrow checker
        let receivers = std::mem::take(&mut self.receivers);
        for receiver in &receivers {
            receiver.flush(self)?;
        }
        self.receivers = receivers;

        let platform_events: Vec<_> = self.platform_consumer.consume_all().collect();
        for event in platform_events {
            match event {
                PlatformEvent::WindowCreated { id } => {
                    let Some(plat_window) = windows.get(&id) else {
                        continue;
                    };

                    let window = window::Window::build(&self.render_device, plat_window)?;
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
                    let Some(window) = self.windows.get_mut(&id) else {
                        continue;
                    };
                    window.resize(vk::Extent2D { width, height })?;
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

        self.models.tick()?;

        Ok(())
    }

    /// The actual rendering. This records render commands & submits them to
    /// the GPU.
    ///
    /// # Errors
    ///
    /// Vulkan errors can occur during rendering
    pub fn render(&mut self) -> Result<()> {
        for player in self.players.values() {
            let Some(entity) = player.entity else {
                continue;
            };
            let Some(world) = self.scene_manager.entity_world(entity) else {
                continue;
            };

            let frame = &self.frames[self.current_frame()];
            let window_id = player.window;
            let Some(window) = self.windows.get_mut(&window_id) else {
                return Err(Error::WindowDoesNotExist(window_id));
            };

            let size = window.extent();
            let render_image = window.next_image()?;

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
            self.render_device.flush_deletions();

            let cmd = frame.command_pool.allocate_buffer()?;

            cmd.begin_command_buffer(&vk::CommandBufferBeginInfo::default())?;

            render_image.image.transition_image_layout(
                &cmd,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            )?;

            self.scene_manager.render(
                &self.models,
                &cmd,
                world,
                size,
                render_image.image.view(),
                entity,
            )?;

            render_image.image.transition_image_layout(
                &cmd,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                vk::ImageLayout::PRESENT_SRC_KHR,
            )?;

            cmd.end_command_buffer()?;

            let image_available_semaphore = render_image.image_available_semaphore;
            let render_finished_semaphore = render_image.render_finished_semaphore;
            let submit_info = vk::SubmitInfo::default()
                .wait_dst_stage_mask(std::slice::from_ref(
                    &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                ))
                .command_buffers(std::slice::from_ref(&cmd))
                .wait_semaphores(std::slice::from_ref(&image_available_semaphore))
                .signal_semaphores(std::slice::from_ref(&render_finished_semaphore));

            self.render_device.queues.submit(
                QueueType::Graphics,
                std::slice::from_ref(&submit_info),
                frame.fence,
            )?;

            render_image.present()?;

            self.current_frame.store(
                (self.current_frame() + 1) % MAX_FRAMES_IN_FLIGHT,
                Ordering::Relaxed,
            );
        }
        Ok(())
    }

    // EXTRA UTILS
    fn create_sampler(device: &RenderDevice, mip_levels: u32) -> Result<vk::Sampler> {
        let props = unsafe {
            device
                .instance
                .get_physical_device_properties(device.physical_device)
        };
        let max_aniso = props.limits.max_sampler_anisotropy;

        // the max_lod cast loses precision, as there are only a
        // small number of mip_levels, there should be no real
        // precision loss.
        #[allow(clippy::cast_precision_loss)]
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

    fn create_shader_module(
        device: &Device,
        shader: &'static dirk_shaders::Shader,
    ) -> Result<vk::ShaderModule> {
        let code = shader.code_as_u32();
        let info = vk::ShaderModuleCreateInfo::default().code(code.as_slice());
        Ok(unsafe { device.create_shader_module(&info, None)? })
    }
    #[inline]
    fn current_frame(&self) -> usize {
        self.current_frame
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.render_device.device.device_wait_idle().ok();
        }
        info!("cleaning up renderer");
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
            error!(target: "vulkan::validation", "{}", message);
        }

        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            warn! (target: "vulkan::validation", "{}", message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            info! (target: "vulkan::validation", "{}", message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            debug!(target: "vulkan::validation", "{}", message);
        }
        _ => trace!(target: "vulkan::validation", "{}", message),
    }

    vk::FALSE
}
