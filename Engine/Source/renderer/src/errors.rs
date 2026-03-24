use thiserror::Error;
use winit::raw_window_handle::HandleError;

pub type RendererResult<T> = std::result::Result<T, RendererError>;

#[derive(Debug, Error)]
pub enum RendererError {
    #[error("Vulkan error: {0}")]
    VulkanError(#[from] ash::vk::Result),

    #[error("Error loading Vulkan functions: {0}")]
    Loading(#[from] ash::LoadingError),

    #[error("platform error: {0}")]
    Platform(#[from] platform::PlatformError),
}

impl From<HandleError> for RendererError {
    fn from(value: HandleError) -> Self {
        RendererError::Platform(value.into())
    }
}
