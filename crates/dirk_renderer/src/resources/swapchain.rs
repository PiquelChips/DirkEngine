//! Safe swapchain abstraction for renderer windows.

use ash::vk;

use crate::{
    Error, Result,
    resources::{
        device::{Garbage, RenderDevice},
        image::SwapchainImage,
    },
};

/// An acquired image from a [`Swapchain`].
///
/// The renderer records work against [`Self::image`] and then consumes this
/// value with [`Self::present`] after the render-finished semaphore has been
/// signalled.
pub struct RenderImage<'a> {
    pub image: &'a SwapchainImage,
    pub image_index: u32,

    pub image_available_semaphore: vk::Semaphore,
    pub render_finished_semaphore: vk::Semaphore,

    device: RenderDevice,
    swapchain: vk::SwapchainKHR,
}

impl RenderImage<'_> {
    /// Presents the acquired swapchain image.
    pub fn present(self) -> Result<()> {
        let wait_semaphores = [self.render_finished_semaphore];
        let swapchains = [self.swapchain];
        let image_indices = [self.image_index];
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        if self.device.queues.present(&present_info)? {
            return Err(Error::SuboptimalSurface);
        }

        Ok(())
    }
}

/// Swapchain and image resources for a single window surface.
pub struct Swapchain {
    device: RenderDevice,
    surface: vk::SurfaceKHR,
    raw: vk::SwapchainKHR,
    images: Vec<SwapchainImage>,
    extent: vk::Extent2D,
    semaphores: Vec<(vk::Semaphore, vk::Semaphore)>,
    semaphore_index: usize,
}

impl Swapchain {
    /// Creates a swapchain for `surface` sized to `window_size`.
    pub fn build(
        device: &RenderDevice,
        surface: vk::SurfaceKHR,
        window_size: vk::Extent2D,
    ) -> Result<Self> {
        let (raw, extent, images) =
            Self::create(device, surface, window_size, vk::SwapchainKHR::null())?;
        let semaphores = match Self::create_semaphores(device, images.len()) {
            Ok(semaphores) => semaphores,
            Err(error) => {
                drop(images);
                Self::destroy_raw(device, raw);
                return Err(error);
            }
        };

        Ok(Self {
            device: device.clone(),
            surface,
            raw,
            images,
            extent,
            semaphores,
            semaphore_index: 0,
        })
    }

    /// Returns the swapchain extent selected by the surface.
    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    /// Acquires the next image and returns the semaphores required to render it.
    pub fn acquire_next_image(&mut self) -> Result<RenderImage<'_>> {
        self.semaphore_index = (self.semaphore_index + 1) % self.semaphores.len();
        let (render_finished_semaphore, image_available_semaphore) =
            self.semaphores[self.semaphore_index];

        let (image_index, suboptimal) = unsafe {
            self.device.swapchain_loader.acquire_next_image(
                self.raw,
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
            image_available_semaphore,
            render_finished_semaphore,
            device: self.device.clone(),
            swapchain: self.raw,
        })
    }

    /// Recreates the swapchain for a new requested window extent.
    pub fn recreate(&mut self, window_size: vk::Extent2D) -> Result<()> {
        let old_raw = self.raw;
        let (raw, extent, images) = Self::create(&self.device, self.surface, window_size, old_raw)?;
        let semaphores = match Self::create_semaphores(&self.device, images.len()) {
            Ok(semaphores) => semaphores,
            Err(error) => {
                drop(images);
                Self::destroy_raw(&self.device, raw);
                return Err(error);
            }
        };

        let old_semaphores = std::mem::replace(&mut self.semaphores, semaphores);
        for (render_finished, image_available) in old_semaphores {
            self.device.destroy(Garbage::Semaphore(render_finished));
            self.device.destroy(Garbage::Semaphore(image_available));
        }

        self.images = images;
        self.raw = raw;
        self.extent = extent;
        self.semaphore_index = 0;

        self.device.destroy(Garbage::Swapchain(old_raw));

        Ok(())
    }

    fn create(
        device: &RenderDevice,
        surface: vk::SurfaceKHR,
        window_size: vk::Extent2D,
        old_swapchain: vk::SwapchainKHR,
    ) -> Result<(vk::SwapchainKHR, vk::Extent2D, Vec<SwapchainImage>)> {
        let capabilities = unsafe {
            device
                .surface_loader
                .get_physical_device_surface_capabilities(device.physical_device, surface)?
        };

        let extent = if capabilities.current_extent.width == u32::MAX {
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
        } else {
            capabilities.current_extent
        };

        let mut image_count = capabilities.min_image_count + 1;
        if capabilities.max_image_count > 0 && image_count > capabilities.max_image_count {
            image_count = capabilities.max_image_count;
        }

        let indices = &device.properties.queue_family_indices;
        let mut unique_indices = vec![indices.graphics, indices.present, indices.transfer];
        unique_indices.sort_unstable();
        unique_indices.dedup();

        let (sharing_mode, indices_slice): (vk::SharingMode, &[u32]) = if unique_indices.len() > 1 {
            (vk::SharingMode::CONCURRENT, &unique_indices)
        } else {
            (vk::SharingMode::EXCLUSIVE, &[])
        };

        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(device.properties.surface_format.format)
            .image_color_space(device.properties.surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(sharing_mode)
            .queue_family_indices(indices_slice)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(device.properties.present_mode)
            .clipped(true)
            .old_swapchain(old_swapchain);

        let swapchain = unsafe {
            device
                .swapchain_loader
                .create_swapchain(&create_info, None)?
        };
        let images = unsafe { device.swapchain_loader.get_swapchain_images(swapchain)? };

        let swapchain_images = match images
            .into_iter()
            .map(|image| {
                SwapchainImage::new(device, image, device.properties.surface_format.format)
            })
            .collect::<Result<Vec<_>>>()
        {
            Ok(images) => images,
            Err(error) => {
                Self::destroy_raw(device, swapchain);
                return Err(error);
            }
        };

        Ok((swapchain, extent, swapchain_images))
    }

    fn create_semaphores(
        device: &RenderDevice,
        count: usize,
    ) -> Result<Vec<(vk::Semaphore, vk::Semaphore)>> {
        let semaphore_info = vk::SemaphoreCreateInfo::default();
        let mut semaphores = Vec::with_capacity(count);

        for _ in 0..count {
            let render_finished =
                match unsafe { device.device.create_semaphore(&semaphore_info, None) } {
                    Ok(semaphore) => semaphore,
                    Err(error) => {
                        Self::destroy_semaphores(device, semaphores);
                        return Err(error.into());
                    }
                };

            let image_available =
                match unsafe { device.device.create_semaphore(&semaphore_info, None) } {
                    Ok(semaphore) => semaphore,
                    Err(error) => {
                        let mut device = device.clone();
                        device.destroy(Garbage::Semaphore(render_finished));
                        Self::destroy_semaphores(&device, semaphores);
                        return Err(error.into());
                    }
                };

            semaphores.push((render_finished, image_available));
        }

        Ok(semaphores)
    }

    fn destroy_raw(device: &RenderDevice, raw: vk::SwapchainKHR) {
        let mut device = device.clone();
        device.destroy(Garbage::Swapchain(raw));
    }

    fn destroy_semaphores(device: &RenderDevice, semaphores: Vec<(vk::Semaphore, vk::Semaphore)>) {
        let mut device = device.clone();
        for (render_finished, image_available) in semaphores {
            device.destroy(Garbage::Semaphore(render_finished));
            device.destroy(Garbage::Semaphore(image_available));
        }
    }

    /// Enqueues all swapchain-owned resources for destruction.
    pub fn destroy(&mut self) {
        self.semaphores
            .drain(..)
            .for_each(|(render_finished, image_available)| {
                self.device.destroy(Garbage::Semaphore(render_finished));
                self.device.destroy(Garbage::Semaphore(image_available));
            });
        self.images.clear();

        if self.raw != vk::SwapchainKHR::null() {
            self.device.destroy(Garbage::Swapchain(self.raw));
            self.raw = vk::SwapchainKHR::null();
        }
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        self.destroy();
    }
}
