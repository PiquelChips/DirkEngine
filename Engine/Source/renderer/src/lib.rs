use std::ffi::{CStr, CString};
#[cfg(validation)]
use std::os::raw::c_void;

#[cfg(validation)]
use ash::ext::debug_utils;
#[cfg(platform_linux)]
use ash::khr::wayland_surface;
use ash::{Entry, Instance, khr::surface, vk};
use log::{debug, error, info, trace, warn};

mod errors;
pub use errors::{RendererError, Result};
mod legacy;
mod structs;

fn make_version(version: utils::Version) -> u32 {
    vk::make_api_version(0, version.major(), version.minor(), version.patch())
}

#[cfg(validation)]
const VALIDATION_LAYERS: &[*const i8] = &[c"VK_LAYER_KHRONOS_validation".as_ptr()];

pub struct RendererCreateInfo<'a> {
    pub engine_name: CString,
    pub engine_version: utils::Version,
    pub app_name: CString,
    pub app_version: utils::Version,
    pub window: &'a dyn utils::Window,
}

/// The Renderer struct that holds all render state and is called upon to handle
/// all rendering operations
pub struct Renderer {
    entry: Entry,

    // Renderer Resources
    instance: Instance,
    /*
    device: Device,
    queues: Queues,
    physical_device: vk::PhysicalDevice,

    properties: RendererProperties,
    */
    surface: vk::SurfaceKHR, // The surface of the main window

    // Extensions
    surface_loader: surface::Instance,
    //swapchain_loader: swapchain::Device,
    #[cfg(validation)]
    debug_utils_loader: debug_utils::Instance,
    #[cfg(validation)]
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl Renderer {
    pub fn init(create_info: RendererCreateInfo) -> Result<Self> {
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
                            return Err(RendererError::ValidationLayerNotFound(
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
                        return Err(RendererError::ExtensionNotFound(
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
                    create_info.window.display_handle()?.as_raw(),
                    create_info.window.window_handle()?.as_raw(),
                    None,
                )?
            };
            let loader = surface::Instance::new(&entry, &instance);

            (loader, surface)
        };

        Ok(Self {
            entry,
            instance,
            /*
            device,
            queues,
            physical_device,
            properties,
            */
            surface,
            surface_loader,
            //swapchain_loader,
            debug_utils_loader,
            debug_messenger,
        })
    }

    pub fn render(&self) -> Result<()> {
        // TODO: render
        Ok(())
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
