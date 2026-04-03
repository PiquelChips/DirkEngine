use std::ffi::CString;

#[cfg(validation)]
use ash::ext::debug_utils;
use ash::{
    Device, Entry, Instance,
    khr::{surface, swapchain},
    vk,
};
use log::info;

mod errors;
pub use errors::{RendererError, Result};
mod legacy;
mod structs;
use structs::*;

fn make_version(version: utils::Version) -> u32 {
    vk::make_api_version(0, version.major(), version.minor(), version.patch())
}

pub struct RendererCreateInfo {
    engine_name: CString,
    engine_version: utils::Version,
    app_name: CString,
    app_version: utils::Version,
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

    surface: vk::SurfaceKHR, // The surface of the main window
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
    pub fn init(create_info: RendererCreateInfo, window: &platform::Window) -> Result<Self> {
        info!("Intializing Vulkan...");

        let entry = unsafe { Entry::load()? };

        let app_info = vk::ApplicationInfo::default()
            .application_name(create_info.app_name.as_c_str())
            .application_version(make_version(create_info.app_version))
            .engine_name(create_info.engine_name.as_c_str())
            .engine_version(make_version(create_info.engine_version))
            .api_version(vk::API_VERSION_1_3);

        Ok(Self {
            entry,
            instance: (),
            device: (),
            queues: (),
            physical_device: (),
            surface: (),
            properties: (),
            surface_loader: (),
            swapchain_loader: (),
            debug_utils_loader: (),
            debug_messenger: (),
        })
    }
}
