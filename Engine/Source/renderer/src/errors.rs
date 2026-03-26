use thiserror::Error;
use winit::raw_window_handle::HandleError;

pub type Result<T> = std::result::Result<T, RendererError>;

#[derive(Debug, Error)]
pub enum RendererError {
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),

    #[error("Vulkan error: {0}")]
    VulkanError(#[from] ash::vk::Result),

    #[error("Error loading Vulkan functions: {0}")]
    Loading(#[from] ash::LoadingError),
    #[error("platform error: {0}")]
    Platform(#[from] platform::PlatformError),

    #[error("no suitable graphics device found")]
    NoDeviceFound,
    #[error("failed to find supported format")]
    NoSupportedFormat,
}

impl From<HandleError> for RendererError {
    fn from(value: HandleError) -> Self {
        RendererError::Platform(value.into())
    }
}
