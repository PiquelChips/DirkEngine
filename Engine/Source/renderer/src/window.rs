use ash::{
    Device,
    khr::{surface, swapchain},
    vk,
};
use platform::WindowId;

use crate::{Error, Renderer, Result};

#[derive(Clone)]
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

    // TODO: actually stop rendering the window when it is
    // occluded.
    occluded: bool,

    /// The semaphores associated with each swapchain image
    semaphores: Vec<(vk::Semaphore, vk::Semaphore)>,
    /// The current index of the semaphores
    semaphore_count: usize,
}

impl Window {
    pub fn build(
        id: WindowId,
        renderer: &Renderer,
        surface: vk::SurfaceKHR,
        size: vk::Extent2D,
    ) -> Result<Self> {
        let (swapchain, extent, images) =
            renderer.create_swap_chain(surface, size, vk::SwapchainKHR::null())?;

        let semaphore_info = vk::SemaphoreCreateInfo::default();
        let create_semaphore = || unsafe {
            Ok::<vk::Semaphore, Error>(renderer.device.create_semaphore(&semaphore_info, None)?)
        };

        let semaphores = (0..images.len())
            .map(|_| Ok((create_semaphore()?, create_semaphore()?)))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            id,
            surface,
            swapchain,
            extent,
            images,
            semaphores,
            semaphore_count: 0,
            occluded: false,
        })
    }
    /// Returns the window's ID
    pub fn id(&self) -> WindowId {
        self.id
    }
    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }
    pub fn swapchain(&self) -> vk::SwapchainKHR {
        self.swapchain
    }
    pub fn surface(&self) -> vk::SurfaceKHR {
        self.surface
    }
    /// (render_finished_semaphore, image_available_semaphore)
    pub fn current_semaphores(&self) -> (vk::Semaphore, vk::Semaphore) {
        self.semaphores[self.semaphore_count]
    }
    pub fn next_image(
        &mut self,
        swapchain_loader: &swapchain::Device,
    ) -> Result<(SwapchainImage, u32)> {
        self.semaphore_count = (self.semaphore_count + 1) % self.semaphores.len();
        let (_, image_available_semaphore) = self.current_semaphores();

        let (image_index, suboptimal) = unsafe {
            swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                image_available_semaphore,
                vk::Fence::null(),
            )?
        };

        if suboptimal {
            return Err(Error::SuboptimalSurface);
        }

        Ok((self.images[image_index as usize].clone(), image_index))
    }
    pub fn update_swapcahin(
        &mut self,
        device: &Device,
        swapchain: vk::SwapchainKHR,
        extent: vk::Extent2D,
        images: Vec<SwapchainImage>,
    ) {
        self.swapchain = swapchain;
        self.extent = extent;
        self.images.iter().for_each(|i| i.destroy(device));
        self.images = images;
    }
    pub fn set_occluded(&mut self, occluded: bool) {
        self.occluded = occluded
    }
    pub fn destroy(
        &self,
        device: &Device,
        surface: &surface::Instance,
        swapchain: &swapchain::Device,
    ) {
        self.images.iter().for_each(|i| i.destroy(device));
        unsafe {
            self.semaphores.iter().for_each(|&(s1, s2)| {
                device.device_wait_idle().unwrap();
                device.destroy_semaphore(s1, None);
                device.destroy_semaphore(s2, None);
            });
            swapchain.destroy_swapchain(self.swapchain, None);
            surface.destroy_surface(self.surface, None);
        }
    }
}
