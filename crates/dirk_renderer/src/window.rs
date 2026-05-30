use ash::vk;
use dirk_platform::WindowId;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::{
    Error, Renderer, Result,
    resources::{
        device::{Garbage, RenderDevice},
        image::SwapchainImage,
    },
};

/// This holds all the data required to render to the owning window's
/// swap chain. These are created on `window::next_image`
pub struct RenderImage<'a> {
    pub image: &'a SwapchainImage,
    pub image_index: u32,
    pub swapchain: vk::SwapchainKHR,

    pub image_available_semaphore: vk::Semaphore,
    pub render_finished_semaphore: vk::Semaphore,
}

/// The renderer's representation of a platform window.
/// Holds the swapchain, surface & other related state.
/// Doesn't actually do any of the rendering of the game.
pub struct Window {
    device: RenderDevice,

    id: WindowId,
    surface: vk::SurfaceKHR,
    swapchain: vk::SwapchainKHR,
    images: Vec<SwapchainImage>,
    extent: vk::Extent2D,

    // TODO: stop rendering when the window is occluded
    occluded: bool,

    /// The semaphores associated with each swapchain image
    semaphores: Vec<(vk::Semaphore, vk::Semaphore)>,
    /// The current index of the semaphores
    semaphore_count: usize,
}

impl Window {
    pub fn build(device: &RenderDevice, plat_window: &dirk_platform::Window) -> Result<Self> {
        let surface = unsafe {
            ash_window::create_surface(
                &device.entry,
                &device.instance,
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

        let (swapchain, extent, images) =
            Renderer::create_swap_chain(device, surface, size, vk::SwapchainKHR::null())?;

        let semaphore_info = vk::SemaphoreCreateInfo::default();
        let create_semaphore = || unsafe {
            Ok::<vk::Semaphore, Error>(device.device.create_semaphore(&semaphore_info, None)?)
        };

        let semaphores = (0..images.len())
            .map(|_| Ok((create_semaphore()?, create_semaphore()?)))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            id: plat_window.id(),
            device: device.clone(),
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
    pub fn next_image(&mut self) -> Result<RenderImage<'_>> {
        self.semaphore_count = (self.semaphore_count + 1) % self.semaphores.len();
        let (render_finished_semaphore, image_available_semaphore) =
            self.semaphores[self.semaphore_count];

        let (image_index, suboptimal) = unsafe {
            self.device.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                image_available_semaphore,
                vk::Fence::null(),
            )?
        };

        if suboptimal {
            return Err(Error::SuboptimalSurface);
        }

        Ok(RenderImage {
            image: &self.images[image_index as usize],
            image_index,
            swapchain: self.swapchain,
            image_available_semaphore,
            render_finished_semaphore,
        })
    }
    pub fn resize(&mut self, extent: vk::Extent2D) -> Result<()> {
        let (swapchain, extent, images) =
            Renderer::create_swap_chain(&self.device, self.surface, extent, self.swapchain)?;
        self.swapchain = swapchain;
        self.extent = extent;
        self.images = images;
        Ok(())
    }
    pub fn set_occluded(&mut self, occluded: bool) {
        self.occluded = occluded;
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        self.semaphores.iter().for_each(|&(s1, s2)| {
            self.device.destroy(Garbage::Semaphore(s1));
            self.device.destroy(Garbage::Semaphore(s2));
        });
        self.images.clear();
        self.device.destroy(Garbage::Swapchain(self.swapchain));
        self.device.destroy(Garbage::Surface(self.surface));
    }
}
