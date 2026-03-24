use thiserror::Error;

pub type RendererResult<T> = std::result::Result<T, RendererError>;

#[derive(Debug, Error)]
pub enum RendererError {
    #[error("Vulkan error: {0}")]
    VulkanError(#[from] ash::vk::Result),

    #[error("Error loading Vulkan functions: {0}")]
    Loading(#[from] ash::LoadingError),
}
