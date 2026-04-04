use ash::vk;
use raw_window_handle::HandleError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, RendererError>;

#[derive(Debug, Error)]
pub enum RendererError {
    // TODO: make it so that errors convert to RendererError before context is added
    #[error(transparent)]
    Anyhow(anyhow::Error),

    #[error("Vulkan error: {0}")]
    VulkanError(#[from] ash::vk::Result),

    #[error("Error loading Vulkan functions: {0}")]
    Loading(#[from] ash::LoadingError),
    #[error("platform error: {0}")]
    Platform(anyhow::Error),

    #[error("no suitable graphics device found")]
    NoDeviceFound,
    #[error("failed to find supported format")]
    NoSupportedFormat,

    #[error("instance extension {0} not found")]
    ExtensionNotFound(String),
    #[error("validation layer {0} not found")]
    ValidationLayerNotFound(String),

    #[error("unsupported image layout transition {old:?} --> {new:?}")]
    UnsupportedImageLayoutTransition {
        old: vk::ImageLayout,
        new: vk::ImageLayout,
    },

    #[error("texture image format does not support linear blitting")]
    FormatNoBlittingSupport,
}

impl From<HandleError> for RendererError {
    fn from(value: HandleError) -> Self {
        RendererError::Platform(value.into())
    }
}
