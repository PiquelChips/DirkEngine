use ash::vk;
use raw_window_handle::HandleError;
use thiserror::Error;

/// An engine Result type to wrap the engine [Error].
pub type Result<T> = std::result::Result<T, Error>;

/// All engine related errors
#[derive(Debug, Error)]
pub enum Error {
    /// An error returned by Vulkan
    #[error("Vulkan error: {0}")]
    VulkanError(#[from] ash::vk::Result),
    /// An error occuring during the allocation of a GPU object
    #[error("Allocation error: {0}")]
    Allocation(#[from] gpu_allocator::AllocationError),

    /// An error loading Vulkan function
    #[error("Error loading Vulkan functions: {0}")]
    Loading(#[from] ash::LoadingError),
    /// A wrapper for platform errors
    #[error("platform error: {0}")]
    Platform(#[from] platform::Error),
    #[error("gltf error: {0}")]
    GltfError(#[from] gltf::Error),

    /// If no physical device is found
    #[error("no suitable graphics device found")]
    NoDeviceFound,
    /// If no supported vulkan image format is found
    #[error("failed to find supported format")]
    NoSupportedFormat,

    /// The required Vulkan instance extension was not found
    #[error("instance extension {0} not found")]
    ExtensionNotFound(String),
    /// The required Vulkan validation layer was not found
    #[error("validation layer {0} not found")]
    ValidationLayerNotFound(String),

    /// Error during layout transition: the specified source layout
    /// is not currently supported by the engine. If it should be supported,
    /// add support in the transition function.
    #[error("the layout {0:?} is not supported as a source. implement it")]
    UnsupportedSourceLayout(vk::ImageLayout),
    /// Error during layout transition: the specified destination layout
    /// is not currently supported by the engine. If it should be supported,
    /// add support in the transition function.
    #[error("the layout {0:?} is not supported as a destination. implement it")]
    UnsupportedDesinationLayout(vk::ImageLayout),

    /// The Vulkan surface is suboptimal
    #[error("suboptimal surface")]
    SuboptimalSurface,

    /// If there is no camera in the scene
    #[error("camera {1} does not exist in world {1}")]
    CameraDoesNotExist(world::WorldId, world::Entity),
    /// If the requested world does not exist
    #[error("world {0} is not registered on renderer")]
    WorldDoesNotExist(world::WorldId),
    /// The requested window does not exist
    #[error("window {0:?} is not registered on renderer")]
    WindowDoesNotExist(platform::WindowId),
}

impl From<HandleError> for Error {
    fn from(value: HandleError) -> Self {
        Error::Platform(value.into())
    }
}
