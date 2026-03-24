//! This crate contains everything to do with the main rendering logic

use std::ffi::{CStr, c_void};

#[cfg(validation)]
use ash::ext::debug_utils;
#[cfg(target_os = "linux")]
use ash::khr::wayland_surface;

use ash::{Entry, Instance, khr::surface, vk};
use log::{debug, error, info, trace, warn};

mod errors;

use errors::RendererResult;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

const MAX_DESCRIPTOR_SET_COUNT: u32 = 1024;

const DEVICE_EXTENSIONS: &[*const i8] = &[ash::khr::swapchain::NAME.as_ptr()];

#[cfg(validation)]
const VALIDATION_LAYERS: &[*const i8] = &[b"VK_LAYER_KHRONOS_validation\0".as_ptr() as *const i8];

struct Queues {
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
}

/// The Renderer struct that holds all render state and is called upon to handle
/// all rendering operations
pub struct Renderer {
    entry: Entry,

    // Renderer Resources
    instance: Instance,
    // device: Device,
    // physical_device: vk::PhysicalDevice,
    // queue: Queues,

    // Renderer State
    surface: vk::SurfaceKHR, // The surface of the main window

    // Extensions
    surface_loader: surface::Instance,
    #[cfg(validation)]
    debug_utils_loader: debug_utils::Instance,
    #[cfg(validation)]
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl Renderer {
    pub fn init(window: platform::Window) -> RendererResult<Self> {
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

        let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

        #[cfg(validation)]
        let (debug_utils_loader, debug_messenger) = {
            let loader = debug_utils::Instance::new(&entry, &instance);
            let messenger =
                unsafe { loader.create_debug_utils_messenger(&debug_create_info, None)? };
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
                )?
            };
            let loader = surface::Instance::new(&entry, &instance);

            (loader, surface)
        };

        Ok(Self {
            entry,
            instance,

            surface,
            surface_loader,

            #[cfg(validation)]
            debug_utils_loader,
            #[cfg(validation)]
            debug_messenger,
        })
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
