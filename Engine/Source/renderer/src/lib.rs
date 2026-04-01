//! This crate contains everything to do with the main rendering logic.
//! As the Ash Vulkan bindings are unsafe, all Vulkan calls should be
//! make here, to centralize the unsafe Vulkan code.

use std::{
    collections::{BTreeMap, HashSet},
    ffi::{CStr, c_void},
    u64,
};

use anyhow::Context;
#[cfg(validation)]
use ash::ext::debug_utils;
#[cfg(target_os = "linux")]
use ash::khr::wayland_surface;

use ash::{
    Device, Entry, Instance,
    khr::{surface, swapchain},
    vk,
};
use log::{debug, error, info, trace, warn};

mod errors;

use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

pub use errors::{RendererError, Result};

const MAX_DESCRIPTOR_SET_COUNT: u32 = 1024;

const DEVICE_EXTENSIONS: &[&CStr] = &[ash::khr::swapchain::NAME];

#[cfg(validation)]
const VALIDATION_LAYERS: &[*const i8] = &[c"VK_LAYER_KHRONOS_validation".as_ptr()];

struct Queues {
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
}

#[derive(Clone, Copy, Default)]
struct QueueFamilyIndices {
    graphics_family: Option<u32>,
    present_family: Option<u32>,
}

impl QueueFamilyIndices {
    fn is_complete(&self) -> bool {
        self.graphics_family.is_some() && self.present_family.is_some()
    }
}

#[derive(Clone, Copy)]
struct DeviceFeatures {
    anisotropy: bool,
    msaa_samples: vk::SampleCountFlags,
}

struct RendererProperties {
    msaa_samples: vk::SampleCountFlags,
    anisotropy: bool,
    surface_format: vk::SurfaceFormatKHR,
    min_image_count: u32,
    queue_family_indices: QueueFamilyIndices,
    depth_format: vk::Format,
}

impl DeviceFeatures {
    fn is_complete(&self) -> bool {
        self.anisotropy && self.msaa_samples.as_raw() > 1
    }
    fn get_score(&self) -> u32 {
        if self.is_complete() {
            return 1000;
        }

        let mut score = 0;
        if self.anisotropy {
            score += 10;
        }
        score += self.msaa_samples.as_raw();
        score
    }
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
    command_pool: vk::CommandPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,

    // Renderer State
    surface: vk::SurfaceKHR, // The surface of the main window
    in_flight_fence: vk::Fence,

    properties: RendererProperties,

    // Extensions
    surface_loader: surface::Instance,
    swapchain_loader: swapchain::Device,
    #[cfg(validation)]
    debug_utils_loader: debug_utils::Instance,
    #[cfg(validation)]
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl Renderer {
    pub fn init(window: &platform::Window) -> Result<Self> {
        info!("Intializing Vulkan...");

        let entry = unsafe { Entry::load()? };

        // TODO: don't hard code app name & engine name
        let app_info = vk::ApplicationInfo::default()
            .application_name(c"DirkEngine")
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(c"DirkEngine")
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            // TODO: 1.4
            .api_version(vk::API_VERSION_1_3);

        let mut extensions: Vec<*const i8> = vec![surface::NAME.as_ptr()];

        // TODO: use proper platform-specific stuff
        #[cfg(target_os = "linux")]
        extensions.push(wayland_surface::NAME.as_ptr());

        let mut instance_create_info =
            vk::InstanceCreateInfo::default().application_info(&app_info);

        #[cfg(validation)]
        let mut debug_create_info: vk::DebugUtilsMessengerCreateInfoEXT;
        #[cfg(validation)]
        {
            info!("using validation layers");
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

            instance_create_info = instance_create_info
                .enabled_layer_names(VALIDATION_LAYERS)
                .push_next(&mut debug_create_info);
        }

        // TODO: check required instance extensions

        instance_create_info = instance_create_info.enabled_extension_names(&extensions);

        let instance = unsafe {
            entry
                .create_instance(&instance_create_info, None)
                .context("creating vulkan instance")?
        };

        #[cfg(validation)]
        let (debug_utils_loader, debug_messenger) = {
            let loader = debug_utils::Instance::new(&entry, &instance);
            let messenger = unsafe {
                loader
                    .create_debug_utils_messenger(&debug_create_info, None)
                    .context("creatint vulkan debug messenger")?
            };
            (loader, messenger)
        };

        let (surface_loader, surface) = {
            let surface = unsafe {
                ash_window::create_surface(
                    &entry,
                    &instance,
                    window.display_handle()?.as_raw(),
                    window.window_handle()?.as_raw(),
                    None,
                )
                .context("creating vulkan surface")?
            };
            let loader = surface::Instance::new(&entry, &instance);

            (loader, surface)
        };

        // PHYSICAL DEVICE
        let (physical_device, queue_family_indices, properties) = {
            let physical_devices = unsafe { instance.enumerate_physical_devices()? };

            let mut candidates: BTreeMap<u32, vk::PhysicalDevice> = BTreeMap::new();
            for &device in &physical_devices {
                let score =
                    Self::get_device_suitability(&instance, device, surface, &surface_loader)?;
                candidates.insert(score, device);
            }

            let device = *candidates
                .iter()
                .rev()
                .find(|&(&score, _)| score > 0)
                .map(|(_, device)| device)
                .ok_or(RendererError::NoDeviceFound)?;

            let properties = unsafe { instance.get_physical_device_properties(device) };
            info!(
                "Physical device selected: {:#?} (vendor: {}, id: {}, api: {}, driver: {})",
                properties.device_name_as_c_str().unwrap_or_default(),
                properties.vendor_id,
                properties.device_id,
                properties.api_version,
                properties.driver_version
            );

            let features = Self::get_device_features(&instance, device);
            let formats =
                unsafe { surface_loader.get_physical_device_surface_formats(device, surface)? };
            let capabilities = unsafe {
                surface_loader.get_physical_device_surface_capabilities(device, surface)?
            };

            let surface_format = Self::choose_swap_surface_format(&formats);
            let queue_family_indices =
                Self::find_queue_families(&instance, device, surface, &surface_loader)?;
            let depth_format = Self::find_supported_format(
                &instance,
                device,
                &[
                    vk::Format::D32_SFLOAT,
                    vk::Format::D32_SFLOAT_S8_UINT,
                    vk::Format::D24_UNORM_S8_UINT,
                ],
                vk::ImageTiling::OPTIMAL,
                vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
            )?;

            let properties = RendererProperties {
                msaa_samples: Self::get_max_usable_sample_count(&instance, device),
                anisotropy: features.anisotropy,
                surface_format,
                min_image_count: capabilities.min_image_count,
                queue_family_indices,
                depth_format,
            };

            (device, queue_family_indices, properties)
        };

        // LOGICAL DEVICE
        let device = {
            let unique_families: HashSet<u32> = [
                queue_family_indices
                    .graphics_family
                    .expect("should have a graphics queue family"),
                queue_family_indices
                    .present_family
                    .expect("should have a present queue family"),
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

            let extensions: Vec<*const i8> =
                DEVICE_EXTENSIONS.iter().map(|name| name.as_ptr()).collect();
            let device_create_info = vk::DeviceCreateInfo::default()
                .queue_create_infos(&queue_create_infos)
                .enabled_features(&physical_device_features)
                .enabled_extension_names(&extensions)
                .push_next(&mut vulkan13_features);

            unsafe {
                instance
                    .create_device(physical_device, &device_create_info, None)
                    .context("create logical device")?
            }
        };

        // QUEUES
        let queues = Queues {
            graphics_queue: unsafe {
                device.get_device_queue(
                    queue_family_indices
                        .graphics_family
                        .context("creating graphics queue")?,
                    0,
                )
            },
            present_queue: unsafe {
                device.get_device_queue(
                    queue_family_indices
                        .present_family
                        .context("creating present queue")?,
                    0,
                )
            },
        };

        // SWAP CHAIN
        let swapchain_loader = swapchain::Device::new(&instance, &device);

        // SYNCHRONIZATION
        let in_flight_fence = unsafe {
            device
                .create_fence(
                    &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                    None,
                )
                .context("creating in flight fence")?
        };

        // COMMAND POOL
        let command_pool = unsafe {
            device
                .create_command_pool(
                    &vk::CommandPoolCreateInfo::default()
                        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                        .queue_family_index(queue_family_indices.graphics_family.unwrap()),
                    None,
                )
                .context("creating command pool")?
        };

        // DESCRIPTOR SETS
        let (descriptor_pool, descriptor_set_layout) = {
            let bindings = [
                vk::DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::VERTEX),
                vk::DescriptorSetLayoutBinding::default()
                    .binding(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            ];

            let layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);

            let descriptor_set_layout = unsafe {
                device
                    .create_descriptor_set_layout(&layout_info, None)
                    .context("creating descriptor set layout")?
            };

            let pool_sizes = [
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(MAX_DESCRIPTOR_SET_COUNT),
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(MAX_DESCRIPTOR_SET_COUNT),
            ];

            let descriptor_pool = unsafe {
                device
                    .create_descriptor_pool(
                        &vk::DescriptorPoolCreateInfo::default()
                            .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
                            .max_sets(MAX_DESCRIPTOR_SET_COUNT)
                            .pool_sizes(&pool_sizes),
                        None,
                    )
                    .context("creating descriptor pool")?
            };

            (descriptor_pool, descriptor_set_layout)
        };

        info!("Vulkan initalized successfully");

        Ok(Self {
            entry,
            instance,
            device,
            queues,
            physical_device,
            command_pool,
            descriptor_pool,
            descriptor_set_layout,

            surface,
            in_flight_fence,
            properties,

            surface_loader,
            swapchain_loader,

            #[cfg(validation)]
            debug_utils_loader,
            #[cfg(validation)]
            debug_messenger,
        })
    }
    pub fn render(&self) -> Result<()> {
        unsafe {
            self.device
                .wait_for_fences(&[self.in_flight_fence], true, u64::MAX)?;
            self.device.reset_fences(&[self.in_flight_fence])?;
        }

        // TODO: actually render stuff
        Ok(())
    }
    fn get_device_suitability(
        instance: &Instance,
        device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        surface_loader: &surface::Instance,
    ) -> Result<u32> {
        let properties = unsafe { instance.get_physical_device_properties(device) };
        let features = unsafe { instance.get_physical_device_features(device) };

        // TODO: update with vulkan tutorial checks

        // prereturn if required stuff doesn't exist
        if features.geometry_shader == vk::FALSE {
            return Ok(0);
        }

        let indices = Self::find_queue_families(instance, device, surface, surface_loader)?;
        if !indices.is_complete() {
            return Ok(0);
        }

        if !Self::check_device_extension_support(instance, device)? {
            return Ok(0);
        }

        let formats =
            unsafe { surface_loader.get_physical_device_surface_formats(device, surface)? };
        let present_modes =
            unsafe { surface_loader.get_physical_device_surface_present_modes(device, surface)? };
        if formats.is_empty() || present_modes.is_empty() {
            return Ok(0);
        }

        let mut score = 0_u32;

        if properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
            score += 1000;
        }
        score += properties.limits.max_image_dimension2_d;

        if indices.graphics_family == indices.present_family {
            score += 10;
        }

        score += formats.len() as u32;
        score += present_modes.len() as u32;
        score += Self::get_device_features(instance, device).get_score();

        let name = properties.device_name_as_c_str().unwrap_or_default();
        debug!("Found device: {:#?} (score {})", name, score);

        Ok(score)
    }
    fn find_queue_families(
        instance: &Instance,
        device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        surface_loader: &surface::Instance,
    ) -> Result<QueueFamilyIndices> {
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(device) };

        let mut indices = QueueFamilyIndices::default();

        for (i, family) in queue_families.iter().enumerate() {
            let i = i as u32;

            // graphics queue
            if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                indices.graphics_family = Some(i);
            }

            // present queue
            let present_support =
                unsafe { surface_loader.get_physical_device_surface_support(device, i, surface)? };
            if present_support {
                indices.present_family = Some(i);
            }

            if indices.is_complete() {
                break;
            }
        }

        Ok(indices)
    }
    fn check_device_extension_support(
        instance: &Instance,
        device: vk::PhysicalDevice,
    ) -> Result<bool> {
        let available = unsafe { instance.enumerate_device_extension_properties(device)? };
        let available_extensions: HashSet<&CStr> = available
            .iter()
            .map(|e| e.extension_name_as_c_str().unwrap_or_default())
            .collect();

        Ok(DEVICE_EXTENSIONS
            .iter()
            .all(|req| available_extensions.contains(req)))
    }
    fn get_device_features(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
    ) -> DeviceFeatures {
        let features = unsafe { instance.get_physical_device_features(physical_device) };
        DeviceFeatures {
            anisotropy: features.sampler_anisotropy == vk::TRUE,
            msaa_samples: Self::get_max_usable_sample_count(instance, physical_device),
        }
    }
    fn get_max_usable_sample_count(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
    ) -> vk::SampleCountFlags {
        let properties = unsafe { instance.get_physical_device_properties(physical_device) };
        let counts = properties.limits.framebuffer_color_sample_counts
            & properties.limits.framebuffer_depth_sample_counts;

        for flag in [
            vk::SampleCountFlags::TYPE_64,
            vk::SampleCountFlags::TYPE_32,
            vk::SampleCountFlags::TYPE_16,
            vk::SampleCountFlags::TYPE_8,
            vk::SampleCountFlags::TYPE_4,
            vk::SampleCountFlags::TYPE_2,
        ] {
            if counts.contains(flag) {
                return flag;
            }
        }
        vk::SampleCountFlags::TYPE_1
    }
    fn choose_swap_surface_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
        formats
            .iter()
            .find(|format| {
                format.format == vk::Format::B8G8R8A8_SRGB
                    && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .copied()
            .unwrap_or(formats[0])
    }
    fn find_supported_format(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        candidates: &[vk::Format],
        tiling: vk::ImageTiling,
        features: vk::FormatFeatureFlags,
    ) -> Result<vk::Format> {
        for &format in candidates {
            let properties =
                unsafe { instance.get_physical_device_format_properties(physical_device, format) };
            let supported = match tiling {
                vk::ImageTiling::LINEAR => properties.linear_tiling_features.contains(features),
                vk::ImageTiling::OPTIMAL => properties.optimal_tiling_features.contains(features),
                _ => false,
            };
            if supported {
                return Ok(format);
            }
        }
        Err(RendererError::NoSupportedFormat)
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

    // TODO: logging should be better (see tracing crate)
    match severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => error!("[Vulkan] {}", message),
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => warn!("[Vulkan] {}", message),
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => info!("[Vulkan] {}", message),
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => debug!("[Vulkan] {}", message),
        _ => trace!("[Vulkan] {}", message),
    }

    vk::FALSE
}
