use ash::{
    Device,
    khr::{surface, swapchain},
    vk,
};

use crate::{Error, Renderer, Result};

pub type WindowId = usize;

pub struct SwapchainImage {
    pub image: vk::Image,
    pub view: vk::ImageView,
}
impl SwapchainImage {
    pub fn destroy(&self, device: &Device) {
        // don't destroy image as it is owned
        // by swap chain
        unsafe {
            device.destroy_image_view(self.view, None);
        }
    }
}

/// The renderer's representation of a platform window.
/// Holds the swapchain, surface & other related state.
/// Doesn't actually do any of the rendering of the game.
pub struct Window {
    id: WindowId,
    surface: vk::SurfaceKHR,
    swapchain: vk::SwapchainKHR,
    images: Vec<SwapchainImage>,
    extent: vk::Extent2D,
}

impl Window {
    pub fn build(
        id: WindowId,
        renderer: &Renderer,
        surface: vk::SurfaceKHR,
        size: vk::Extent2D,
    ) -> Result<Self> {
        let (swapchain, extent, images) = renderer.create_swap_chain(surface, size)?;
        Ok(Self {
            id,
            surface,
            swapchain,
            extent,
            images,
        })
    }
    /// Returns the window's ID
    pub fn id(&self) -> WindowId {
        self.id
    }
    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }
    pub fn next_image(&self, renderer: &Renderer) -> Result<&SwapchainImage> {
        let frame = renderer.get_current_frame();

        let (image_index, suboptimal) = unsafe {
            renderer.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                frame.image_available_semaphore,
                vk::Fence::null(),
            )?
        };

        if suboptimal {
            return Err(Error::SuboptimalSurface);
        }

        Ok(&self.images[image_index as usize])
    }
    pub fn resize(&mut self, renderer: &Renderer, in_size: vk::Extent2D) -> Result<()> {
        let (swapchain, extent, images) = renderer.create_swap_chain(self.surface, in_size)?;
        self.swapchain = swapchain;
        self.extent = extent;
        self.images = images;
        Ok(())
    }
    pub fn destroy(
        &self,
        device: &Device,
        surface: &surface::Instance,
        swapchain: &swapchain::Device,
    ) {
        self.images.iter().for_each(|i| i.destroy(device));
        unsafe {
            swapchain.destroy_swapchain(self.swapchain, None);
            surface.destroy_surface(self.surface, None);
        }
    }
}
