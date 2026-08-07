//! Optional display-surface and swapchain abstractions.

use std::{
    cell::{Cell, RefCell},
    fmt,
    sync::Arc,
};

use crate::{
    Backend, Device, Error, Extent2D, Format, ImageRef, ImageUsage, ImageViewRef, Result,
    Semaphore, SemaphoreKind,
};

/// Preferred display scheduling mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PresentMode {
    /// Wait for the display refresh and avoid tearing.
    #[default]
    Fifo,
    /// Replace pending frames while waiting for refresh when supported.
    Mailbox,
    /// Present immediately when supported.
    Immediate,
}

/// Swapchain configuration shared by concrete backends.
#[derive(Clone, Copy, Debug)]
pub struct SwapchainCreateInfo<'a> {
    /// Requested drawable extent.
    pub extent: Extent2D,
    /// Required image usages.
    pub image_usage: ImageUsage,
    /// Preferred image formats in priority order.
    pub preferred_formats: &'a [Format],
    /// Preferred display scheduling mode.
    pub present_mode: PresentMode,
    /// Optional diagnostic label.
    pub label: Option<&'a str>,
}

impl SwapchainCreateInfo<'_> {
    fn validate(&self) -> Result<()> {
        if self.extent.is_empty() {
            return Err(Error::invalid_descriptor(
                "swapchain",
                "requested extent must be non-zero",
            ));
        }
        if self.image_usage.is_empty() {
            return Err(Error::invalid_descriptor(
                "swapchain",
                "at least one image usage must be selected",
            ));
        }
        Ok(())
    }
}

/// An owned platform display surface.
pub struct Surface<B: Backend> {
    pub(crate) backend: Arc<B>,
    pub(crate) raw: Arc<B::Surface>,
}

impl<B: Backend> Clone for Surface<B> {
    fn clone(&self) -> Self {
        Self {
            backend: Arc::clone(&self.backend),
            raw: Arc::clone(&self.raw),
        }
    }
}

impl<B: Backend> fmt::Debug for Surface<B> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("Surface").finish_non_exhaustive()
    }
}

struct SwapchainState<B: Backend> {
    raw: RefCell<B::Swapchain>,
    surface: Surface<B>,
    acquired_count: Cell<usize>,
}

/// An owned presentation swapchain.
pub struct Swapchain<B: Backend> {
    backend: Arc<B>,
    state: Arc<SwapchainState<B>>,
}

impl<B: Backend> fmt::Debug for Swapchain<B> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Swapchain")
            .field("extent", &self.extent())
            .field("format", &self.format())
            .finish_non_exhaustive()
    }
}

impl<B: Backend> Device<B> {
    /// Creates a surface from the concrete backend's platform target.
    pub fn create_surface(&self, target: &B::SurfaceTarget) -> Result<Surface<B>> {
        Ok(Surface {
            backend: Arc::clone(&self.backend),
            raw: Arc::new(self.backend.create_surface(target)?),
        })
    }

    /// Creates a swapchain for a surface.
    pub fn create_swapchain(
        &self,
        surface: &Surface<B>,
        info: &SwapchainCreateInfo<'_>,
    ) -> Result<Swapchain<B>> {
        self.ensure_resource(&surface.backend, "create_swapchain")?;
        info.validate()?;
        let raw = self.backend.create_swapchain(surface.raw.as_ref(), info)?;
        Ok(Swapchain {
            backend: Arc::clone(&self.backend),
            state: Arc::new(SwapchainState {
                raw: RefCell::new(raw),
                surface: surface.clone(),
                acquired_count: Cell::new(0),
            }),
        })
    }
}

impl<B: Backend> Swapchain<B> {
    /// Returns the selected drawable extent.
    #[must_use]
    pub fn extent(&self) -> Extent2D {
        B::swapchain_extent(&self.state.raw.borrow())
    }

    /// Returns the selected image format.
    #[must_use]
    pub fn format(&self) -> Format {
        B::swapchain_format(&self.state.raw.borrow())
    }

    /// Recreates this swapchain for changed surface configuration.
    pub fn recreate(&mut self, info: &SwapchainCreateInfo<'_>) -> Result<()> {
        info.validate()?;
        if self.state.acquired_count.get() != 0 {
            return Err(Error::invalid_descriptor(
                "swapchain recreation",
                "all acquired images must be presented or abandoned first",
            ));
        }
        let mut raw = self.state.raw.borrow_mut();
        self.backend
            .recreate_swapchain(&mut raw, self.state.surface.raw.as_ref(), info)
    }

    /// Acquires the next renderable swapchain image.
    pub fn acquire_next_image(
        &self,
        timeout_ns: u64,
        signal: &Semaphore<B>,
    ) -> Result<RenderImage<B>> {
        if !Arc::ptr_eq(&self.backend, &signal.backend) {
            return Err(Error::DeviceMismatch {
                operation: "acquire_next_image",
            });
        }
        if signal.kind() != SemaphoreKind::Binary {
            return Err(Error::invalid_descriptor(
                "swapchain acquisition",
                "image acquisition requires a binary semaphore",
            ));
        }
        let raw = self.backend.acquire_render_image(
            &mut self.state.raw.borrow_mut(),
            timeout_ns,
            &signal.raw,
        )?;
        self.state
            .acquired_count
            .set(self.state.acquired_count.get() + 1);
        Ok(RenderImage {
            backend: Arc::clone(&self.backend),
            swapchain: Arc::clone(&self.state),
            raw: Some(raw),
        })
    }
}

/// An acquired swapchain image that must be presented or abandoned.
#[must_use = "an acquired image must be presented or abandoned"]
pub struct RenderImage<B: Backend> {
    backend: Arc<B>,
    swapchain: Arc<SwapchainState<B>>,
    raw: Option<B::RenderImage>,
}

impl<B: Backend> fmt::Debug for RenderImage<B> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RenderImage")
            .field("index", &self.index())
            .finish_non_exhaustive()
    }
}

impl<B: Backend> RenderImage<B> {
    /// Returns the acquired image index.
    #[must_use]
    pub fn index(&self) -> u32 {
        B::render_image_parts(self.raw()).2
    }

    /// Returns the acquired image for barriers and copy operations.
    #[must_use]
    pub fn image(&self) -> ImageRef<'_, B> {
        ImageRef {
            backend: &self.backend,
            raw: B::render_image_parts(self.raw()).0,
        }
    }

    /// Returns the acquired image view for rendering attachments.
    #[must_use]
    pub fn view(&self) -> ImageViewRef<'_, B> {
        ImageViewRef {
            backend: &self.backend,
            raw: B::render_image_parts(self.raw()).1,
        }
    }

    /// Presents the acquired image after waiting on binary semaphores.
    pub fn present(mut self, waits: &[&Semaphore<B>]) -> Result<()> {
        for wait in waits {
            if !Arc::ptr_eq(&self.backend, &wait.backend) {
                return Err(Error::DeviceMismatch {
                    operation: "present",
                });
            }
            if wait.kind() != SemaphoreKind::Binary {
                return Err(Error::invalid_descriptor(
                    "presentation",
                    "presentation waits require binary semaphores",
                ));
            }
        }
        let backend_waits = waits.iter().map(|wait| &wait.raw).collect::<Vec<_>>();
        let raw = self.take_raw();
        self.backend
            .present(&mut self.swapchain.raw.borrow_mut(), raw, &backend_waits)
    }

    /// Releases an acquired image that will not be presented.
    pub fn abandon(mut self) -> Result<()> {
        let raw = self.take_raw();
        self.backend
            .abandon_render_image(&mut self.swapchain.raw.borrow_mut(), raw)
    }

    fn raw(&self) -> &B::RenderImage {
        self.raw
            .as_ref()
            .expect("render image is inaccessible after consumption")
    }

    fn take_raw(&mut self) -> B::RenderImage {
        self.swapchain
            .acquired_count
            .set(self.swapchain.acquired_count.get().saturating_sub(1));
        self.raw
            .take()
            .expect("render image can only be consumed once")
    }
}

impl<B: Backend> Drop for RenderImage<B> {
    fn drop(&mut self) {
        let Some(raw) = self.raw.take() else {
            return;
        };
        self.swapchain
            .acquired_count
            .set(self.swapchain.acquired_count.get().saturating_sub(1));
        let _ = self
            .backend
            .abandon_render_image(&mut self.swapchain.raw.borrow_mut(), raw);
    }
}
