use anyhow::Context;
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
    pub fn init(window: &platform::Window) -> Result<Self> {
        info!("Intializing Vulkan...");

        let entry = unsafe { Entry::load()? };

        let app_info = vk::ApplicationInfo::default()
            .application_name(c"DirkEngine")
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(c"DirkEngine")
            .engine_version(vk::make_api_version(0, 1, 0, 0))
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
