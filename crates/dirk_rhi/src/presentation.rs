use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use crate::{Backend, Extent3d, Format, ImageUsages, Result, SurfaceStatus};

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

/// Reconfigurable presentation chain and its acquired frames.
pub trait Swapchain<B: Backend> {
    /// Acquires the next presentation frame.
    ///
    /// # Errors
    ///
    /// Returns an error when the surface is unavailable or must be recreated.
    fn acquire(&mut self) -> Result<B::SurfaceFrame>;
    /// Reconfigures this swapchain for a new extent.
    ///
    /// # Errors
    ///
    /// Returns an error when the extent is invalid or recreation fails.
    fn resize(&mut self, width: u32, height: u32) -> Result<()>;
    /// Presents a frame previously acquired from this swapchain.
    ///
    /// # Errors
    ///
    /// Returns an error when the frame is invalid or presentation fails.
    fn present(&mut self, frame: B::SurfaceFrame) -> Result<()>;
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
