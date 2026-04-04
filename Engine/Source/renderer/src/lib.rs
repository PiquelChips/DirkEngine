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
use log::{debug, error, info, trace, warn};

mod errors;
pub use errors::{Error, Result};

mod physical_device;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::window::{Window, WindowId};
mod legacy;
mod window;

fn make_version(version: utils::Version) -> u32 {
    vk::make_api_version(0, version.major(), version.minor(), version.patch())
}

const DEVICE_EXTENSIONS: &[&str] =
    &[unsafe { std::str::from_utf8_unchecked(swapchain::NAME.to_bytes()) }];
#[cfg(validation)]
const VALIDATION_LAYERS: &[*const i8] = &[c"VK_LAYER_KHRONOS_validation".as_ptr()];

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
    anisotropy: bool,
    surface_format: vk::SurfaceFormatKHR,
    min_image_count: u32,
    queue_family_indices: physical_device::QueueFamilyIndices,
    depth_format: vk::Format,
    present_mode: vk::PresentModeKHR,
}

struct SwapchainImage {
    image: vk::Image,
    view: vk::ImageView,
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

    properties: RendererProperties,
    surface: vk::SurfaceKHR, // The surface of the main window
    windows: HashMap<WindowId, Window>,

    // Extensions
    surface_loader: surface::Instance,
    swapchain_loader: swapchain::Device,
    #[cfg(validation)]
    debug_utils_loader: debug_utils::Instance,
    #[cfg(validation)]
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl Renderer {
    pub fn init(create_info: RendererCreateInfo, window: &platform::Window) -> Result<Self> {
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
                .require(|info| info.features.geometry_shader == vk::FALSE)
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
            let capabilities = unsafe {
                surface_loader
                    .get_physical_device_surface_capabilities(device_info.handle, surface)?
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
                min_image_count: capabilities.min_image_count,
                queue_family_indices: queues,
                depth_format,
                present_mode,
            };

            (device_info.handle, properties)
        };

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

        let windows = HashMap::new();

        let mut renderer = Self {
            entry,
            instance,
            device,
            queues,
            physical_device,
            properties,
            surface,
            windows,
            surface_loader,
            swapchain_loader,
            debug_utils_loader,
            debug_messenger,
        };

        let window_size = window.size();
        let size = vk::Extent2D {
            width: window_size.width,
            height: window_size.height,
        };
        let window = window::Window::build(window.id().into_raw(), &renderer, surface, size)?;
        renderer.windows.insert(window.id(), window);

        Ok(renderer)
    }

    pub fn render(&self) -> Result<()> {
        // TODO: render
        Ok(())
    }

    pub fn create_window(&mut self, plat_window: &platform::Window) -> Result<WindowId> {
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

        let window = window::Window::build(plat_window.id().into_raw(), self, surface, size)?;
        self.windows.insert(window.id(), window);
        Ok(plat_window.id().into_raw())
    }

    fn create_swap_chain(
        &self,
        surface: vk::SurfaceKHR,
        window_size: vk::Extent2D,
    ) -> Result<(vk::SwapchainKHR, vk::Extent2D, Vec<SwapchainImage>)> {
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
        let indices_array = [indices.graphics, indices.present, indices.transfer];

        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(self.properties.surface_format.format)
            .image_color_space(self.properties.surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::CONCURRENT)
            .queue_family_indices(&indices_array)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(self.properties.present_mode)
            .clipped(true);

        let swapchain = unsafe { self.swapchain_loader.create_swapchain(&create_info, None)? };
        let images = unsafe { self.swapchain_loader.get_swapchain_images(swapchain)? };

        let swap_images = images
            .into_iter()
            .map(|image| {
                // TODO: try to promote error
                let view = self
                    .create_image_view(
                        image,
                        self.properties.surface_format.format,
                        vk::ImageAspectFlags::COLOR,
                        1,
                    )
                    .unwrap();

                SwapchainImage { image, view }
            })
            .collect();

        Ok((swapchain, extent, swap_images))
    }

    // UTILITIES

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
