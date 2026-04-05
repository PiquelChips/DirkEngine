use ash::vk;

use crate::{Renderer, Result, SwapchainImage};

pub type WindowId = usize;

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
    pub fn resize(&mut self, renderer: &Renderer, in_size: vk::Extent2D) -> Result<()> {
        let (swapchain, extent, images) = renderer.create_swap_chain(self.surface, in_size)?;
        self.swapchain = swapchain;
        self.extent = extent;
        self.images = images;
        Ok(())
    }
}
