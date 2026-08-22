use raw_window_handle::{DisplayHandle, WindowHandle};

use crate::{Backend, Extent3d, ImageUsages, PresentMode, Result, SurfaceStatus, TextureFormat};

/// Borrowed platform handles used to create a graphics presentation surface.
///
/// The display and window owners must remain alive and valid until the backend
/// surface is destroyed. Backends must destroy their surface before those
/// owners are dropped.
#[derive(Clone, Copy, Debug)]
pub struct SurfaceCreateInfo<'a> {
    /// Platform display handle.
    pub display: DisplayHandle<'a>,
    /// Platform window handle.
    pub window: WindowHandle<'a>,
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
    /// Preferred image formats, most preferred first.
    ///
    /// The backend selects the first supported entry, or its own default when
    /// no entry is supported.
    pub preferred_formats: &'a [TextureFormat],
    /// Preferred presentation mode. The backend may fall back when it is not
    /// supported by the surface.
    pub present_mode: PresentMode,
}

/// Reconfigurable presentation chain and its acquired frames.
///
/// Frames follow a strict lifecycle: [`Self::acquire`] yields a frame, a
/// submission lists that frame in [`Submission::surface_frames`](crate::Submission::surface_frames) to order
/// rendering against presentation, and [`Self::present`](Swapchain::present)
/// returns it to the display. A frame must not be acquired twice, submitted
/// twice, or presented without an intervening submission.
pub trait Swapchain<B: Backend> {
    /// Format selected for images in the current swapchain generation.
    fn format(&self) -> TextureFormat;
    /// Dimensions selected for images in the current swapchain generation.
    fn extent(&self) -> Extent3d;
    /// Acquires the next presentation frame.
    ///
    /// This call may block the calling thread until an image is available,
    /// including when the presentation queue is full.
    ///
    /// The caller owns frame pacing: multiple frames may be held between
    /// acquisition and presentation up to the backend's limit (at least one
    /// more than the swapchain's reported minimum image count), but holding
    /// every frame deadlocks; present acquired frames promptly.
    ///
    /// # Errors
    ///
    /// Returns an error when the surface is unavailable or must be recreated.
    fn acquire(&mut self) -> Result<B::SurfaceFrame>;
    /// Reconfigures this swapchain for a new extent.
    ///
    /// All frames previously acquired from this swapchain must have been
    /// presented or discarded before resizing.
    ///
    /// # Errors
    ///
    /// Returns an error when the extent is invalid or recreation fails.
    fn resize(&mut self, width: u32, height: u32) -> Result<()>;
    /// Presents a frame previously acquired from this swapchain.
    ///
    /// The frame's rendering submission must have listed the frame in
    /// [`Submission::surface_frames`](crate::Submission::surface_frames); presenting otherwise has undefined
    /// synchronization behavior.
    ///
    /// Returns [`SurfaceStatus::Suboptimal`] when the frame was presented but
    /// the swapchain should be recreated.
    ///
    /// # Errors
    ///
    /// Returns an error when the frame is invalid or presentation fails.
    fn present(&mut self, frame: B::SurfaceFrame) -> Result<SurfaceStatus>;
}

/// A presentation image acquired from a backend swapchain.
pub trait SurfaceFrame<B: Backend> {
    /// Image backing this frame.
    fn image(&self) -> &B::Image;
    /// Default view of [`Self::image`].
    fn view(&self) -> &B::ImageView;
    /// Image format.
    fn format(&self) -> TextureFormat;
    /// Image dimensions.
    fn extent(&self) -> Extent3d;
    /// Whether the frame is usable but should trigger recreation.
    fn status(&self) -> SurfaceStatus;
}
