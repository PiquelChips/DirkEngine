use std::sync::Arc;

use ash::{Entry, vk};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use tracing::warn;

use super::{
    Garbage, Inner, VulkanBackend, VulkanImage, VulkanImageView, VulkanImageViewInner, map_error,
    mapping, unsupported,
};
use crate::{Format, ImageAspects, ImageSubresourceRange, Result, SwapchainCreateInfo};

/// Window-handle contract accepted by Vulkan surface creation.
pub trait VulkanSurfaceTarget: HasDisplayHandle + HasWindowHandle {}

impl<T> VulkanSurfaceTarget for T where T: HasDisplayHandle + HasWindowHandle + ?Sized {}

#[doc(hidden)]
pub struct VulkanSurface {
    inner: Arc<Inner>,
    raw: vk::SurfaceKHR,
}

impl Drop for VulkanSurface {
    fn drop(&mut self) {
        self.inner.defer(Garbage::Surface(self.raw));
    }
}

struct SwapchainImage {
    image: VulkanImage,
    view: VulkanImageView,
}

#[doc(hidden)]
pub struct VulkanSwapchain {
    inner: Arc<Inner>,
    raw: vk::SwapchainKHR,
    images: Vec<SwapchainImage>,
    extent: crate::Extent2D,
    format: Format,
}

impl Drop for VulkanSwapchain {
    fn drop(&mut self) {
        self.images.clear();
        self.inner.defer(Garbage::Swapchain(self.raw));
    }
}

#[doc(hidden)]
pub struct VulkanRenderImage {
    image: VulkanImage,
    view: VulkanImageView,
    index: u32,
    acquire_semaphore: vk::Semaphore,
}

pub(super) fn create_raw_surface(
    entry: &Entry,
    instance: &ash::Instance,
    target: &(impl VulkanSurfaceTarget + ?Sized),
) -> Result<vk::SurfaceKHR> {
    let display = target
        .display_handle()
        .map_err(|error| map_error("get Vulkan display handle", error))?;
    let window = target
        .window_handle()
        .map_err(|error| map_error("get Vulkan window handle", error))?;
    unsafe { ash_window::create_surface(entry, instance, display.as_raw(), window.as_raw(), None) }
        .map_err(|error| map_error("create Vulkan surface", error))
}

pub(super) fn create_surface(
    backend: &VulkanBackend,
    target: &dyn VulkanSurfaceTarget,
) -> Result<VulkanSurface> {
    let Some(present) = backend.inner.queues.present else {
        return Err(unsupported(
            "Vulkan device was created headlessly; use new_vulkan_for_window before creating a surface",
        ));
    };
    let raw = create_raw_surface(&backend.inner.entry, &backend.inner.instance, target)?;
    let supported = unsafe {
        backend
            .inner
            .surface_loader
            .get_physical_device_surface_support(
                backend.inner.physical_device,
                present.family_index,
                raw,
            )
    };
    let supported = match supported {
        Ok(supported) => supported,
        Err(error) => {
            unsafe { backend.inner.surface_loader.destroy_surface(raw, None) };
            return Err(map_error("query Vulkan surface support", error));
        }
    };
    if !supported {
        unsafe { backend.inner.surface_loader.destroy_surface(raw, None) };
        return Err(unsupported(
            "selected Vulkan present queue does not support this surface",
        ));
    }
    Ok(VulkanSurface {
        inner: Arc::clone(&backend.inner),
        raw,
    })
}

pub(super) fn create_swapchain(
    backend: &VulkanBackend,
    surface: &VulkanSurface,
    info: &SwapchainCreateInfo<'_>,
) -> Result<VulkanSwapchain> {
    let built = build_swapchain(backend, surface.raw, info, vk::SwapchainKHR::null())?;
    Ok(VulkanSwapchain {
        inner: Arc::clone(&backend.inner),
        raw: built.raw,
        images: built.images,
        extent: built.extent,
        format: built.format,
    })
}

pub(super) fn recreate_swapchain(
    backend: &VulkanBackend,
    swapchain: &mut VulkanSwapchain,
    surface: &VulkanSurface,
    info: &SwapchainCreateInfo<'_>,
) -> Result<()> {
    let built = build_swapchain(backend, surface.raw, info, swapchain.raw)?;
    let old_raw = std::mem::replace(&mut swapchain.raw, built.raw);
    let old_images = std::mem::replace(&mut swapchain.images, built.images);
    swapchain.extent = built.extent;
    swapchain.format = built.format;
    drop(old_images);
    backend.inner.defer(Garbage::Swapchain(old_raw));
    Ok(())
}

struct BuiltSwapchain {
    raw: vk::SwapchainKHR,
    images: Vec<SwapchainImage>,
    extent: crate::Extent2D,
    format: Format,
}

#[allow(clippy::too_many_lines)]
fn build_swapchain(
    backend: &VulkanBackend,
    surface: vk::SurfaceKHR,
    info: &SwapchainCreateInfo<'_>,
    old_swapchain: vk::SwapchainKHR,
) -> Result<BuiltSwapchain> {
    let present = backend
        .inner
        .queues
        .present
        .ok_or_else(|| unsupported("Vulkan device has no presentation queue"))?;
    let capabilities = unsafe {
        backend
            .inner
            .surface_loader
            .get_physical_device_surface_capabilities(backend.inner.physical_device, surface)
    }
    .map_err(|error| map_error("query Vulkan surface capabilities", error))?;
    let formats = unsafe {
        backend
            .inner
            .surface_loader
            .get_physical_device_surface_formats(backend.inner.physical_device, surface)
    }
    .map_err(|error| map_error("query Vulkan surface formats", error))?;
    let modes = unsafe {
        backend
            .inner
            .surface_loader
            .get_physical_device_surface_present_modes(backend.inner.physical_device, surface)
    }
    .map_err(|error| map_error("query Vulkan present modes", error))?;

    let (surface_format, format) = choose_surface_format(&formats, info.preferred_formats)?;
    let requested_mode = mapping::present_mode(info.present_mode);
    let present_mode = if modes.contains(&requested_mode) {
        requested_mode
    } else {
        warn!(
            ?requested_mode,
            "requested Vulkan present mode unavailable; using FIFO"
        );
        vk::PresentModeKHR::FIFO
    };
    let extent = if capabilities.current_extent.width == u32::MAX {
        vk::Extent2D {
            width: info.extent.width.clamp(
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ),
            height: info.extent.height.clamp(
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ),
        }
    } else {
        capabilities.current_extent
    };
    let usage = mapping::image_usage(info.image_usage);
    if !capabilities.supported_usage_flags.contains(usage) {
        return Err(unsupported(format!(
            "Vulkan surface supports {:?}, but swapchain requires {:?}",
            capabilities.supported_usage_flags, usage
        )));
    }
    let mut image_count = capabilities.min_image_count.saturating_add(1);
    if capabilities.max_image_count != 0 {
        image_count = image_count.min(capabilities.max_image_count);
    }
    let mut families = vec![
        backend.inner.queues.graphics.family_index,
        backend.inner.queues.transfer.family_index,
        present.family_index,
    ];
    families.sort_unstable();
    families.dedup();
    let (sharing_mode, family_indices) = if families.len() > 1 {
        (vk::SharingMode::CONCURRENT, families.as_slice())
    } else {
        (vk::SharingMode::EXCLUSIVE, &[][..])
    };
    let composite_alpha = choose_composite_alpha(capabilities.supported_composite_alpha)?;
    let create_info = vk::SwapchainCreateInfoKHR::default()
        .surface(surface)
        .min_image_count(image_count)
        .image_format(surface_format.format)
        .image_color_space(surface_format.color_space)
        .image_extent(extent)
        .image_array_layers(1)
        .image_usage(usage)
        .image_sharing_mode(sharing_mode)
        .queue_family_indices(family_indices)
        .pre_transform(capabilities.current_transform)
        .composite_alpha(composite_alpha)
        .present_mode(present_mode)
        .clipped(true)
        .old_swapchain(old_swapchain);
    let raw = unsafe {
        backend
            .inner
            .swapchain_loader
            .create_swapchain(&create_info, None)
    }
    .map_err(|error| map_error("create Vulkan swapchain", error))?;
    let raw_images = match unsafe { backend.inner.swapchain_loader.get_swapchain_images(raw) } {
        Ok(images) => images,
        Err(error) => {
            unsafe { backend.inner.swapchain_loader.destroy_swapchain(raw, None) };
            return Err(map_error("get Vulkan swapchain images", error));
        }
    };
    let mut raw_views = Vec::with_capacity(raw_images.len());
    for &image in &raw_images {
        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(surface_format.format)
            .subresource_range(mapping::subresource_range(ImageSubresourceRange::all(
                ImageAspects::COLOR,
                1,
                1,
            )));
        match unsafe { backend.inner.device.create_image_view(&view_info, None) } {
            Ok(view) => raw_views.push(view),
            Err(error) => {
                unsafe {
                    for view in raw_views {
                        backend.inner.device.destroy_image_view(view, None);
                    }
                    backend.inner.swapchain_loader.destroy_swapchain(raw, None);
                }
                return Err(map_error("create Vulkan swapchain image view", error));
            }
        }
    }
    let images = raw_images
        .into_iter()
        .zip(raw_views)
        .map(|(image, view)| SwapchainImage {
            image: VulkanImage::borrowed(Arc::clone(&backend.inner), image, surface_format.format),
            view: VulkanImageView(Arc::new(VulkanImageViewInner {
                device: Arc::clone(&backend.inner),
                raw: view,
                format: surface_format.format,
            })),
        })
        .collect();
    Ok(BuiltSwapchain {
        raw,
        images,
        extent: crate::Extent2D::new(extent.width, extent.height),
        format,
    })
}

fn choose_surface_format(
    available: &[vk::SurfaceFormatKHR],
    preferred: &[Format],
) -> Result<(vk::SurfaceFormatKHR, Format)> {
    if let [undefined] = available
        && undefined.format == vk::Format::UNDEFINED
    {
        let format = preferred.first().copied().unwrap_or(Format::Bgra8Srgb);
        return Ok((
            vk::SurfaceFormatKHR {
                format: mapping::format(format),
                color_space: undefined.color_space,
            },
            format,
        ));
    }
    let find = |format: Format| {
        available.iter().copied().find(|candidate| {
            candidate.format == mapping::format(format)
                && candidate.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
    };
    for &format in preferred {
        if let Some(surface_format) = find(format) {
            return Ok((surface_format, format));
        }
    }
    if !preferred.is_empty() {
        return Err(unsupported(format!(
            "none of the preferred Vulkan swapchain formats {preferred:?} are supported"
        )));
    }
    for &surface_format in available {
        if surface_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            && let Ok(format) = mapping::format_from_vk(surface_format.format)
        {
            return Ok((surface_format, format));
        }
    }
    Err(unsupported(
        "Vulkan surface exposes no swapchain format represented by the RHI",
    ))
}

fn choose_composite_alpha(
    supported: vk::CompositeAlphaFlagsKHR,
) -> Result<vk::CompositeAlphaFlagsKHR> {
    [
        vk::CompositeAlphaFlagsKHR::OPAQUE,
        vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED,
        vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED,
        vk::CompositeAlphaFlagsKHR::INHERIT,
    ]
    .into_iter()
    .find(|flag| supported.contains(*flag))
    .ok_or_else(|| unsupported("Vulkan surface exposes no supported composite-alpha mode"))
}

pub(super) fn acquire_render_image(
    backend: &VulkanBackend,
    swapchain: &mut VulkanSwapchain,
    timeout_ns: u64,
    signal: &super::VulkanSemaphore,
) -> Result<VulkanRenderImage> {
    let (index, suboptimal) = unsafe {
        backend.inner.swapchain_loader.acquire_next_image(
            swapchain.raw,
            timeout_ns,
            signal.raw,
            vk::Fence::null(),
        )
    }
    .map_err(|error| map_error("acquire Vulkan swapchain image", error))?;
    let image = swapchain
        .images
        .get(index as usize)
        .ok_or_else(|| unsupported("Vulkan returned an out-of-range swapchain image index"))?;
    if suboptimal {
        present_raw(backend, swapchain, index, std::slice::from_ref(&signal.raw))?;
        return Err(unsupported(
            "Vulkan swapchain is suboptimal and must be recreated",
        ));
    }
    Ok(VulkanRenderImage {
        image: image.image.clone(),
        view: image.view.clone(),
        index,
        acquire_semaphore: signal.raw,
    })
}

pub(super) fn swapchain_extent(swapchain: &VulkanSwapchain) -> crate::Extent2D {
    swapchain.extent
}

pub(super) fn swapchain_format(swapchain: &VulkanSwapchain) -> Format {
    swapchain.format
}

pub(super) fn render_image_parts(
    image: &VulkanRenderImage,
) -> (&VulkanImage, &VulkanImageView, u32) {
    (&image.image, &image.view, image.index)
}

pub(super) fn present(
    backend: &VulkanBackend,
    swapchain: &mut VulkanSwapchain,
    image: &VulkanRenderImage,
    waits: &[&super::VulkanSemaphore],
) -> Result<()> {
    let wait_semaphores = waits.iter().map(|wait| wait.raw).collect::<Vec<_>>();
    present_raw(backend, swapchain, image.index, &wait_semaphores)
}

pub(super) fn abandon_render_image(
    backend: &VulkanBackend,
    swapchain: &mut VulkanSwapchain,
    image: &VulkanRenderImage,
) -> Result<()> {
    // This path is valid only before queue work consumes the acquire semaphore.
    present_raw(
        backend,
        swapchain,
        image.index,
        std::slice::from_ref(&image.acquire_semaphore),
    )
}

fn present_raw(
    backend: &VulkanBackend,
    swapchain: &VulkanSwapchain,
    index: u32,
    waits: &[vk::Semaphore],
) -> Result<()> {
    let _queue_guard = super::lock(&backend.inner.queue_lock);
    let present = backend
        .inner
        .queues
        .present
        .ok_or_else(|| unsupported("Vulkan device has no presentation queue"))?;
    let swapchains = [swapchain.raw];
    let indices = [index];
    let present_info = vk::PresentInfoKHR::default()
        .wait_semaphores(waits)
        .swapchains(&swapchains)
        .image_indices(&indices);
    let suboptimal = unsafe {
        backend
            .inner
            .swapchain_loader
            .queue_present(present.raw, &present_info)
    }
    .map_err(|error| map_error("present Vulkan swapchain image", error))?;
    if suboptimal {
        Err(unsupported(
            "Vulkan swapchain is suboptimal and must be recreated",
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undefined_surface_format_accepts_the_first_preference() {
        let available = [vk::SurfaceFormatKHR {
            format: vk::Format::UNDEFINED,
            color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
        }];
        let preferred = [Format::Rgba8Srgb, Format::Bgra8Srgb];

        let (selected, format) =
            choose_surface_format(&available, &preferred).expect("surface format should map");

        assert_eq!(selected.format, vk::Format::R8G8B8A8_SRGB);
        assert_eq!(format, Format::Rgba8Srgb);
    }

    #[test]
    fn unsupported_explicit_surface_preferences_are_rejected() {
        let available = [vk::SurfaceFormatKHR {
            format: vk::Format::B8G8R8A8_SRGB,
            color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
        }];

        assert!(
            choose_surface_format(&available, &[Format::Rgba8Srgb]).is_err(),
            "an explicit unsupported preference must not be substituted",
        );
    }
}
