//! This is the legacy C++ engine ported to Rust
//!
//! This crate contains everything to do with the main rendering logic.
//! As the Ash Vulkan bindings are unsafe, all Vulkan calls should be
//! make here, to centralize the unsafe Vulkan code.

#![allow(dead_code, unused, clippy::all)]

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

use crate::errors::{RendererError, Result};

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

const MAX_DESCRIPTOR_SET_COUNT: u32 = 1024;

const DEVICE_EXTENSIONS: &[&CStr] = &[ash::khr::swapchain::NAME];

#[cfg(validation)]
const VALIDATION_LAYERS: &[*const i8] = &[c"VK_LAYER_KHRONOS_validation".as_ptr()];

struct Queues {
    graphics: vk::Queue,
    present: vk::Queue,
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

#[derive(Clone)]
struct SwapchainImage {
    image: vk::Image,
    view: vk::ImageView,
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

struct ImageMemoryView {
    image: vk::Image,
    memory: vk::DeviceMemory,
    view: vk::ImageView,
}

struct CreateImageMemoryViewInfo {
    width: u32,
    height: u32,
    format: vk::Format,
    tiling: vk::ImageTiling,
    usage: vk::ImageUsageFlags,
    properties: vk::MemoryPropertyFlags,
    num_samples: vk::SampleCountFlags,
    mip_levels: u32,
    image_aspect: vk::ImageAspectFlags,
}

// Mirrors the ModelViewProjection UBO used in descriptor set writes.
#[repr(C)]
struct ModelViewProjection {
    model: glam::Mat4,
    view: glam::Mat4,
    proj: glam::Mat4,
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

/// TODO: REMOVE
impl From<anyhow::Error> for RendererError {
    fn from(value: anyhow::Error) -> Self {
        Self::Anyhow(value)
    }
}

impl Renderer {
    // MAIN RENDERER RUNTIME FUNCTIONS

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
                    .context("creating vulkan debug messenger")?
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
            graphics: unsafe {
                device.get_device_queue(
                    queue_family_indices
                        .graphics_family
                        .context("creating graphics queue")?,
                    0,
                )
            },
            present: unsafe {
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

    // EXTERNAL HELPER FUNCTIONS

    fn create_swap_chain(
        &self,
        surface: vk::SurfaceKHR,
        window_size: vk::Extent2D,
        surface_format: vk::SurfaceFormatKHR,
        present_mode: vk::PresentModeKHR,
        old_swapchain: vk::SwapchainKHR,
        extent: &mut vk::Extent2D,
        out_swapchain: &mut vk::SwapchainKHR,
    ) -> Result<Vec<SwapchainImage>> {
        let capabilities = unsafe {
            self.surface_loader
                .get_physical_device_surface_capabilities(self.physical_device, surface)?
        };

        *extent = Self::choose_swap_extent(window_size, &capabilities);

        let mut image_count = capabilities.min_image_count + 1;
        if capabilities.max_image_count > 0 && image_count > capabilities.max_image_count {
            image_count = capabilities.max_image_count;
        }

        let indices = self.properties.queue_family_indices;
        let indices_array = [
            indices.graphics_family.expect("graphics_family"),
            indices.present_family.expect("present_family"),
        ];

        let (sharing_mode, indices_slice): (vk::SharingMode, &[u32]) =
            if indices.graphics_family != indices.present_family {
                (vk::SharingMode::CONCURRENT, &indices_array)
            } else {
                (vk::SharingMode::EXCLUSIVE, &[])
            };

        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(*extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(sharing_mode)
            .queue_family_indices(indices_slice)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(old_swapchain);

        *out_swapchain = unsafe {
            self.swapchain_loader
                .create_swapchain(&create_info, None)
                .context("swap chain creation")?
        };

        let images = unsafe {
            self.swapchain_loader
                .get_swapchain_images(*out_swapchain)
                .context("get swap chain images")?
        };

        Ok(images
            .into_iter()
            .map(|image| {
                // TODO: try to promote error
                let cmd = self.begin_single_time_commands().unwrap();
                // TODO: try to promote error
                self.transition_image_layout(
                    cmd,
                    image,
                    surface_format.format,
                    vk::ImageLayout::UNDEFINED,
                    vk::ImageLayout::PRESENT_SRC_KHR,
                    1,
                )
                .unwrap();
                // TODO: try to promote error
                self.end_single_time_commands(cmd, self.queues.graphics)
                    .unwrap();

                // TODO: try to promote error
                let view = self
                    .create_image_view(image, surface_format.format, vk::ImageAspectFlags::COLOR, 1)
                    .unwrap();

                SwapchainImage { image, view }
            })
            .collect())
    }

    // RENDERER UTILITY FUNCTIONS

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
    fn has_stencil_component(format: vk::Format) -> bool {
        matches!(
            format,
            vk::Format::D32_SFLOAT_S8_UINT | vk::Format::D24_UNORM_S8_UINT
        )
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
    fn choose_swap_present_mode(available: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
        if available.contains(&vk::PresentModeKHR::MAILBOX) {
            vk::PresentModeKHR::MAILBOX
        } else {
            vk::PresentModeKHR::FIFO
        }
    }
    fn choose_swap_extent(
        window_size: vk::Extent2D,
        capabilities: &vk::SurfaceCapabilitiesKHR,
    ) -> vk::Extent2D {
        if capabilities.current_extent.width != u32::MAX {
            return capabilities.current_extent;
        }

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
    }
    fn begin_single_time_commands(&self) -> Result<vk::CommandBuffer> {
        let pool_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::TRANSIENT)
            .queue_family_index(
                self.properties
                    .queue_family_indices
                    .graphics_family
                    .expect("graphics_family"),
            );

        let pool = unsafe {
            self.device
                .create_command_pool(&pool_info, None)
                .context("creating command pool")?
        };

        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let cmd = unsafe {
            self.device
                .allocate_command_buffers(&alloc_info)
                .context("allocating command buffers")?[0]
        };

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.device
                .begin_command_buffer(cmd, &begin_info)
                .context("begin command buffer")?
        };
        Ok(cmd)
    }
    fn end_single_time_commands(&self, cmd: vk::CommandBuffer, queue: vk::Queue) -> Result<()> {
        unsafe {
            self.device
                .end_command_buffer(cmd)
                .context("end command buffer")?
        };

        let submit_info = vk::SubmitInfo::default().command_buffers(std::slice::from_ref(&cmd));

        unsafe {
            self.device
                .queue_submit(queue, &[submit_info], vk::Fence::null())
                .context("submit command buffer")?;
            self.device
                .queue_wait_idle(queue)
                .context("waiting for device idle")?;
        };

        Ok(())
    }
    fn transition_image_layout(
        &self,
        cmd: vk::CommandBuffer,
        image: vk::Image,
        format: vk::Format,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
        mip_levels: u32,
    ) -> Result<()> {
        let mut aspect_mask = vk::ImageAspectFlags::COLOR;

        if new_layout == vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL {
            aspect_mask = vk::ImageAspectFlags::DEPTH;
            if Self::has_stencil_component(format) {
                aspect_mask |= vk::ImageAspectFlags::STENCIL;
            }
        }

        let mut barrier = vk::ImageMemoryBarrier::default()
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask,
                base_mip_level: 0,
                level_count: mip_levels,
                base_array_layer: 0,
                layer_count: 1,
            });

        let (src_stage, dst_stage) = match (old_layout, new_layout) {
            (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => {
                barrier.src_access_mask = vk::AccessFlags::empty();
                barrier.dst_access_mask = vk::AccessFlags::TRANSFER_WRITE;
                (
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::TRANSFER,
                )
            }
            (vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => {
                barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
                barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;
                (
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                )
            }
            (vk::ImageLayout::UNDEFINED, vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL) => {
                barrier.src_access_mask = vk::AccessFlags::empty();
                barrier.dst_access_mask = vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                    | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE;
                (
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
                )
            }
            (vk::ImageLayout::PRESENT_SRC_KHR, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL) => {
                barrier.src_access_mask = vk::AccessFlags::empty();
                barrier.dst_access_mask = vk::AccessFlags::COLOR_ATTACHMENT_WRITE;
                (
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                )
            }
            (vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::ImageLayout::PRESENT_SRC_KHR) => {
                barrier.src_access_mask = vk::AccessFlags::COLOR_ATTACHMENT_WRITE;
                barrier.dst_access_mask = vk::AccessFlags::empty();
                (
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                )
            }
            (vk::ImageLayout::UNDEFINED, vk::ImageLayout::PRESENT_SRC_KHR) => {
                barrier.src_access_mask = vk::AccessFlags::empty();
                barrier.dst_access_mask = vk::AccessFlags::empty();
                (
                    vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                )
            }
            (vk::ImageLayout::UNDEFINED, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => {
                barrier.src_access_mask = vk::AccessFlags::empty();
                barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;
                (
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                )
            }
            _ => {
                return Err(RendererError::UnsupportedImageLayoutTransition {
                    old: old_layout,
                    new: new_layout,
                });
            }
        };

        Ok(unsafe {
            self.device.cmd_pipeline_barrier(
                cmd,
                src_stage,
                dst_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        })
    }
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

        Ok(unsafe {
            self.device
                .create_image_view(&create_info, None)
                .context("create image view")?
        })
    }
    fn create_image(
        &self,
        width: u32,
        height: u32,
        format: vk::Format,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        properties: vk::MemoryPropertyFlags,
        num_samples: vk::SampleCountFlags,
        mip_levels: u32,
    ) -> Result<(vk::Image, vk::DeviceMemory)> {
        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .mip_levels(mip_levels)
            .array_layers(1)
            .samples(num_samples)
            .tiling(tiling)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let image = unsafe {
            self.device
                .create_image(&image_info, None)
                .context("create image")?
        };

        let mem_req = unsafe { self.device.get_image_memory_requirements(image) };

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_req.size)
            .memory_type_index(Self::find_memory_type(
                &self.instance,
                self.physical_device,
                mem_req.memory_type_bits,
                properties,
            )?);

        let memory = unsafe {
            self.device
                .allocate_memory(&alloc_info, None)
                .context("allocate image memory")?
        };

        unsafe { self.device.bind_image_memory(image, memory, 0).unwrap() };

        Ok((image, memory))
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

        let buffer = unsafe {
            self.device
                .create_buffer(&buffer_info, None)
                .expect("Failed to create buffer")
        };

        let mem_req = unsafe { self.device.get_buffer_memory_requirements(buffer) };

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_req.size)
            .memory_type_index(Self::find_memory_type(
                &self.instance,
                self.physical_device,
                mem_req.memory_type_bits,
                properties,
            )?);

        let memory = unsafe {
            self.device
                .allocate_memory(&alloc_info, None)
                .expect("Failed to allocate buffer memory")
        };

        unsafe { self.device.bind_buffer_memory(buffer, memory, 0).unwrap() };

        Ok((buffer, memory))
    }
    fn create_image_memory_view(
        &self,
        info: &CreateImageMemoryViewInfo,
    ) -> Result<ImageMemoryView> {
        assert_ne!(info.format, vk::Format::UNDEFINED);

        let (image, memory) = self.create_image(
            info.width,
            info.height,
            info.format,
            info.tiling,
            info.usage,
            info.properties,
            info.num_samples,
            info.mip_levels,
        )?;

        let view =
            self.create_image_view(image, info.format, info.image_aspect, info.mip_levels)?;

        Ok(ImageMemoryView {
            image,
            memory,
            view,
        })
    }

    fn create_semaphore(&self) -> Result<vk::Semaphore> {
        let info = vk::SemaphoreCreateInfo::default();
        Ok(unsafe {
            self.device
                .create_semaphore(&info, None)
                .context("create semaphore")?
        })
    }
    fn create_command_buffer(&self) -> Result<vk::CommandBuffer> {
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        Ok(unsafe {
            self.device
                .allocate_command_buffers(&alloc_info)
                .context("allocate command buffers")?[0]
        })
    }
    fn create_descriptor_sets(
        &self,
        uniform_buffer: vk::Buffer,
        sampler: vk::Sampler,
        image_view: vk::ImageView,
        layout: vk::ImageLayout,
    ) -> Result<vk::DescriptorSet> {
        let layouts = [self.descriptor_set_layout];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_set = unsafe {
            self.device
                .allocate_descriptor_sets(&alloc_info)
                .context("allocate descriptor set")?[0]
        };

        let buffer_info = vk::DescriptorBufferInfo {
            buffer: uniform_buffer,
            offset: 0,
            range: std::mem::size_of::<ModelViewProjection>() as u64,
        };

        let image_info = vk::DescriptorImageInfo {
            image_view,
            sampler,
            image_layout: layout,
        };

        let descriptor_writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(std::slice::from_ref(&buffer_info)),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(1)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&image_info)),
        ];

        unsafe { self.device.update_descriptor_sets(&descriptor_writes, &[]) };

        Ok(descriptor_set)
    }
    fn copy_buffer_to_image(
        &self,
        cmd: vk::CommandBuffer,
        buffer: vk::Buffer,
        image: vk::Image,
        width: u32,
        height: u32,
        offset_x: u32,
        offset_y: u32,
    ) {
        let region = vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: vk::Offset3D {
                x: offset_x as i32,
                y: offset_y as i32,
                z: 0,
            },
            image_extent: vk::Extent3D {
                width,
                height,
                depth: 1,
            },
        };

        unsafe {
            self.device.cmd_copy_buffer_to_image(
                cmd,
                buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region],
            );
        }
    }
    fn generate_mipmaps(
        &self,
        cmd: vk::CommandBuffer,
        image: vk::Image,
        image_format: vk::Format,
        tex_width: u32,
        tex_height: u32,
        mip_levels: u32,
    ) -> Result<()> {
        let format_properties = unsafe {
            self.instance
                .get_physical_device_format_properties(self.physical_device, image_format)
        };

        if !format_properties
            .optimal_tiling_features
            .contains(vk::FormatFeatureFlags::SAMPLED_IMAGE_FILTER_LINEAR)
        {
            return Err(RendererError::FormatNoBlittingSupport);
        }

        let mut barrier = vk::ImageMemoryBarrier::default()
            .image(image)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_array_layer: 0,
                layer_count: 1,
                level_count: 1,
                base_mip_level: 0,
            });

        let mut mip_width = tex_width;
        let mut mip_height = tex_height;

        for i in 1..mip_levels {
            barrier.subresource_range.base_mip_level = i - 1;
            barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
            barrier.new_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            barrier.dst_access_mask = vk::AccessFlags::TRANSFER_READ;

            unsafe {
                self.device.cmd_pipeline_barrier(
                    cmd,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[barrier],
                );
            }

            let blit = vk::ImageBlit {
                src_subresource: vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: i - 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_offsets: [
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D {
                        x: mip_width as i32,
                        y: mip_height as i32,
                        z: 1,
                    },
                ],
                dst_subresource: vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: i,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                dst_offsets: [
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D {
                        x: if mip_width > 1 {
                            mip_width as i32 / 2
                        } else {
                            1
                        },
                        y: if mip_height > 1 {
                            mip_height as i32 / 2
                        } else {
                            1
                        },
                        z: 1,
                    },
                ],
            };

            unsafe {
                self.device.cmd_blit_image(
                    cmd,
                    image,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[blit],
                    vk::Filter::LINEAR,
                );
            }

            barrier.old_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
            barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_READ;
            barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;

            unsafe {
                self.device.cmd_pipeline_barrier(
                    cmd,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[barrier],
                );
            }

            if mip_width > 1 {
                mip_width /= 2;
            }
            if mip_height > 1 {
                mip_height /= 2;
            }
        }

        // Transition the last mip level (never blitted from)
        barrier.subresource_range.base_mip_level = mip_levels - 1;
        barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
        barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
        barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
        barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;

        Ok(unsafe {
            self.device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        })
    }
    fn find_memory_type(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<u32> {
        let mem_props = unsafe { instance.get_physical_device_memory_properties(physical_device) };

        Ok((0..mem_props.memory_type_count)
            .find(|&i| {
                (type_filter & (1 << i)) != 0
                    && mem_props.memory_types[i as usize]
                        .property_flags
                        .contains(properties)
            })
            .context("Failed to find suitable memory type")?)
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().ok();
            log::info!("cleaning up renderer");

            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_fence(self.in_flight_fence, None);
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
