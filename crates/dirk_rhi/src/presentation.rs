use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use crate::{Backend, Extent3d, Format, ImageUsages, SurfaceStatus};

/// Raw platform handles used to create a graphics presentation surface.
#[derive(Clone, Copy, Debug)]
pub struct SurfaceCreateInfo {
    /// Platform display handle.
    pub display: RawDisplayHandle,
    /// Platform window handle.
    pub window: RawWindowHandle,
}

/// Presentation swapchain description.
pub struct SwapchainDesc<'a, B: Backend> {
    /// Debug label.
    pub label: &'a str,
    /// Presentation surface.
    pub surface: &'a B::Surface,
    /// Requested image dimensions.
    pub width: u32,
    /// Requested image dimensions.
    pub height: u32,
    /// Required swapchain image uses.
    pub usage: ImageUsages,
}

/// A presentation image acquired from a backend swapchain.
pub trait SurfaceFrame<B: Backend> {
    /// Image backing this frame.
    fn image(&self) -> &B::Image;
    /// Default view of [`Self::image`].
    fn view(&self) -> &B::ImageView;
    /// Image format.
    fn format(&self) -> Format;
    /// Image dimensions.
    fn extent(&self) -> Extent3d;
    /// Whether the frame is usable but should trigger recreation.
    fn status(&self) -> SurfaceStatus;
}
