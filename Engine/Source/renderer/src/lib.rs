#[cfg(validation)]
use ash::ext::debug_utils;
use ash::{
    Device, Entry, Instance,
    khr::{surface, swapchain},
    vk,
};

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
