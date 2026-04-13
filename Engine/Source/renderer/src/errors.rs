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

    #[error("the layout {0:?} is not supported as a source. implement it")]
    UnsupportedSourceLayout(vk::ImageLayout),
    #[error("the layout {0:?} is not supported as a destination. implement it")]
    UnsupportedDesinationLayout(vk::ImageLayout),

    #[error("suboptimal surface")]
    SuboptimalSurface,

    #[error("camera {1} does not exist in world {1}")]
    CameraDoesNotExist(world::WorldId, world::Entity),
}

impl From<HandleError> for Error {
    fn from(value: HandleError) -> Self {
        Error::Platform(value.into())
    }
}
