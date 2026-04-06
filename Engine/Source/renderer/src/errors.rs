use ash::vk;
use raw_window_handle::HandleError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Vulkan error: {0}")]
    VulkanError(#[from] ash::vk::Result),

    #[error("Error loading Vulkan functions: {0}")]
    Loading(#[from] ash::LoadingError),
    #[error("platform error: {0}")]
    Platform(#[from] platform::Error),
    #[error("resource manager error: {0}")]
    ResourceManager(#[from] resource_manager::Error),

    #[error("no suitable graphics device found")]
    NoDeviceFound,
    #[error("failed to find supported format")]
    NoSupportedFormat,
    #[error("failed to find a suitable memory type")]
    NoSuitableMemoryType,

    #[error("instance extension {0} not found")]
    ExtensionNotFound(String),
    #[error("validation layer {0} not found")]
    ValidationLayerNotFound(String),

    #[error("unsupported image layout transition {old:?} --> {new:?}")]
    UnsupportedImageLayoutTransition {
        old: vk::ImageLayout,
        new: vk::ImageLayout,
    },
}

impl From<HandleError> for Error {
    fn from(value: HandleError) -> Self {
        Error::Platform(value.into())
    }
}
