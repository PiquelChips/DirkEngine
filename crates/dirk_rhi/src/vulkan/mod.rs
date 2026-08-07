//! Vulkan 1.3 implementation of the backend-neutral RHI.

use std::{
    error, fmt,
    sync::{Arc, Mutex, MutexGuard},
};

use ash::{Entry, Instance, vk};
use gpu_allocator::vulkan::{Allocation, Allocator};

use crate::{BackendInterop, Device, QueueType};

mod backend;
#[cfg(validation)]
mod debug;
mod init;
mod mapping;
#[cfg(feature = "presentation")]
mod presentation;

pub use ash::vk as raw;
pub use init::VulkanCreateInfo;
#[cfg(feature = "presentation")]
pub use presentation::VulkanSurfaceTarget;

/// A backend-neutral device backed by Vulkan 1.3.
pub type VulkanDevice = Device<VulkanBackend>;

/// Vulkan implementation of [`crate::Backend`].
pub struct VulkanBackend {
    inner: Arc<Inner>,
}

struct Inner {
    #[allow(dead_code)] // Kept alive with the instance for loader ownership.
    entry: Entry,
    instance: Instance,
    device: ash::Device,
    physical_device: vk::PhysicalDevice,
    queues: Queues,
    queue_lock: Mutex<()>,
    max_sampler_anisotropy: f32,
    allocator: Mutex<Option<Allocator>>,
    deletion_queue: Mutex<Vec<Garbage>>,
    #[cfg(validation)]
    debug_loader: ash::ext::debug_utils::Instance,
    #[cfg(validation)]
    debug_messenger: vk::DebugUtilsMessengerEXT,
    #[cfg(feature = "presentation")]
    surface_loader: ash::khr::surface::Instance,
    #[cfg(feature = "presentation")]
    swapchain_loader: ash::khr::swapchain::Device,
}

#[derive(Clone, Copy)]
struct Queue {
    raw: vk::Queue,
    family_index: u32,
}

struct Queues {
    graphics: Queue,
    compute: Queue,
    transfer: Queue,
    #[cfg(feature = "presentation")]
    present: Option<Queue>,
}

impl Queues {
    fn get(&self, queue: QueueType) -> Queue {
        match queue {
            QueueType::Graphics => self.graphics,
            QueueType::Compute => self.compute,
            QueueType::Transfer => self.transfer,
        }
    }
}

impl Inner {
    fn allocator(&self) -> MutexGuard<'_, Option<Allocator>> {
        lock(&self.allocator)
    }

    fn defer(&self, garbage: Garbage) {
        lock(&self.deletion_queue).push(garbage);
    }

    fn flush_deletions(&self) {
        let garbage = std::mem::take(&mut *lock(&self.deletion_queue));
        for item in garbage {
            self.destroy(item);
        }
    }

    fn destroy(&self, garbage: Garbage) {
        unsafe {
            match garbage {
                Garbage::Buffer { raw, allocation } => {
                    self.device.destroy_buffer(raw, None);
                    self.free(allocation);
                }
                Garbage::Image { raw, allocation } => {
                    self.device.destroy_image(raw, None);
                    self.free(allocation);
                }
                Garbage::ImageView(raw) => self.device.destroy_image_view(raw, None),
                Garbage::Sampler(raw) => self.device.destroy_sampler(raw, None),
                Garbage::ShaderModule(raw) => self.device.destroy_shader_module(raw, None),
                Garbage::DescriptorSetLayout(raw) => {
                    self.device.destroy_descriptor_set_layout(raw, None);
                }
                Garbage::DescriptorPool(raw) => self.device.destroy_descriptor_pool(raw, None),
                Garbage::PipelineLayout(raw) => self.device.destroy_pipeline_layout(raw, None),
                Garbage::Pipeline(raw) => self.device.destroy_pipeline(raw, None),
                Garbage::CommandPool(raw) => self.device.destroy_command_pool(raw, None),
                Garbage::Fence(raw) => self.device.destroy_fence(raw, None),
                Garbage::Semaphore(raw) => self.device.destroy_semaphore(raw, None),
                #[cfg(feature = "presentation")]
                Garbage::Surface(raw) => self.surface_loader.destroy_surface(raw, None),
                #[cfg(feature = "presentation")]
                Garbage::Swapchain(raw) => self.swapchain_loader.destroy_swapchain(raw, None),
            }
        }
    }

    fn free(&self, allocation: Allocation) {
        if let Some(allocator) = self.allocator().as_mut() {
            allocator
                .free(allocation)
                .expect("Vulkan allocation must belong to this allocator");
        }
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
        }
        self.flush_deletions();
        drop(lock(&self.allocator).take());
        unsafe {
            self.device.destroy_device(None);
            #[cfg(validation)]
            self.debug_loader
                .destroy_debug_utils_messenger(self.debug_messenger, None);
            self.instance.destroy_instance(None);
        }
    }
}

enum Garbage {
    Buffer {
        raw: vk::Buffer,
        allocation: Allocation,
    },
    Image {
        raw: vk::Image,
        allocation: Allocation,
    },
    ImageView(vk::ImageView),
    Sampler(vk::Sampler),
    ShaderModule(vk::ShaderModule),
    DescriptorSetLayout(vk::DescriptorSetLayout),
    DescriptorPool(vk::DescriptorPool),
    PipelineLayout(vk::PipelineLayout),
    Pipeline(vk::Pipeline),
    CommandPool(vk::CommandPool),
    Fence(vk::Fence),
    Semaphore(vk::Semaphore),
    #[cfg(feature = "presentation")]
    Surface(vk::SurfaceKHR),
    #[cfg(feature = "presentation")]
    Swapchain(vk::SwapchainKHR),
}

#[doc(hidden)]
pub struct VulkanBuffer {
    inner: Arc<Inner>,
    raw: vk::Buffer,
    allocation: Mutex<Option<Allocation>>,
    size: u64,
}

impl Drop for VulkanBuffer {
    fn drop(&mut self) {
        if let Some(allocation) = lock(&self.allocation).take() {
            self.inner.defer(Garbage::Buffer {
                raw: self.raw,
                allocation,
            });
        }
    }
}

#[derive(Clone)]
#[doc(hidden)]
pub struct VulkanImage(Arc<VulkanImageInner>);

struct VulkanImageInner {
    device: Arc<Inner>,
    raw: vk::Image,
    format: vk::Format,
    allocation: Option<Allocation>,
}

impl Drop for VulkanImageInner {
    fn drop(&mut self) {
        if let Some(allocation) = self.allocation.take() {
            self.device.defer(Garbage::Image {
                raw: self.raw,
                allocation,
            });
        }
    }
}

impl VulkanImage {
    fn raw(&self) -> vk::Image {
        self.0.raw
    }

    fn format(&self) -> vk::Format {
        self.0.format
    }

    #[cfg(feature = "presentation")]
    fn borrowed(device: Arc<Inner>, raw: vk::Image, format: vk::Format) -> Self {
        Self(Arc::new(VulkanImageInner {
            device,
            raw,
            format,
            allocation: None,
        }))
    }
}

#[derive(Clone)]
#[doc(hidden)]
pub struct VulkanImageView(Arc<VulkanImageViewInner>);

struct VulkanImageViewInner {
    device: Arc<Inner>,
    raw: vk::ImageView,
    format: vk::Format,
}

impl Drop for VulkanImageViewInner {
    fn drop(&mut self) {
        self.device.defer(Garbage::ImageView(self.raw));
    }
}

impl VulkanImageView {
    fn raw(&self) -> vk::ImageView {
        self.0.raw
    }
}

macro_rules! deferred_resource {
    ($name:ident, $raw:ty, $garbage:ident) => {
        #[doc(hidden)]
        pub struct $name {
            inner: Arc<Inner>,
            raw: $raw,
        }

        impl Drop for $name {
            fn drop(&mut self) {
                self.inner.defer(Garbage::$garbage(self.raw));
            }
        }
    };
}

deferred_resource!(VulkanSampler, vk::Sampler, Sampler);
deferred_resource!(VulkanShaderModule, vk::ShaderModule, ShaderModule);
deferred_resource!(VulkanPipelineLayout, vk::PipelineLayout, PipelineLayout);
deferred_resource!(VulkanPipeline, vk::Pipeline, Pipeline);
deferred_resource!(VulkanCommandPool, vk::CommandPool, CommandPool);
deferred_resource!(VulkanFence, vk::Fence, Fence);
deferred_resource!(VulkanSemaphore, vk::Semaphore, Semaphore);

#[doc(hidden)]
pub struct VulkanBindGroupLayout {
    inner: Arc<Inner>,
    raw: vk::DescriptorSetLayout,
    entries: Vec<crate::BindGroupLayoutEntry>,
}

impl Drop for VulkanBindGroupLayout {
    fn drop(&mut self) {
        self.inner.defer(Garbage::DescriptorSetLayout(self.raw));
    }
}

#[doc(hidden)]
pub struct VulkanBindGroup {
    inner: Arc<Inner>,
    pool: vk::DescriptorPool,
    set: vk::DescriptorSet,
}

impl Drop for VulkanBindGroup {
    fn drop(&mut self) {
        self.inner.defer(Garbage::DescriptorPool(self.pool));
    }
}

#[doc(hidden)]
pub struct VulkanCommandBuffer {
    raw: vk::CommandBuffer,
}

/// Narrow Vulkan handle adapter for integrations that cannot use generic RHI APIs.
pub struct VulkanInterop<'a> {
    backend: &'a Arc<VulkanBackend>,
}

impl VulkanInterop<'_> {
    /// Returns the Ash instance.
    #[must_use]
    pub fn instance(&self) -> &ash::Instance {
        &self.backend.inner.instance
    }

    /// Returns the selected physical-device handle.
    #[must_use]
    pub fn physical_device(&self) -> vk::PhysicalDevice {
        self.backend.inner.physical_device
    }

    /// Returns the Ash logical device.
    #[must_use]
    pub fn device(&self) -> &ash::Device {
        &self.backend.inner.device
    }

    /// Runs an operation while holding the Vulkan queue serialization lock.
    /// The queue handle is valid only for the duration of `operation`.
    pub fn with_queue(
        &self,
        queue: QueueType,
        operation: impl FnOnce(vk::Queue) -> crate::Result<()>,
    ) -> crate::Result<()> {
        let _guard = lock(&self.backend.inner.queue_lock);
        operation(self.backend.inner.queues.get(queue).raw)
    }

    /// Returns the native family index for a queue type.
    #[must_use]
    pub fn queue_family_index(&self, queue: QueueType) -> u32 {
        self.backend.inner.queues.get(queue).family_index
    }

    /// Returns the native handle for an explicit RHI command pool.
    pub fn command_pool(
        &self,
        pool: &crate::CommandPool<VulkanBackend>,
    ) -> crate::Result<vk::CommandPool> {
        ensure_same_device(self.backend, &pool.backend, "Vulkan command-pool interop")?;
        Ok(pool.raw.raw)
    }

    /// Returns the native handle for an RHI command buffer.
    pub fn command_buffer(
        &self,
        command_buffer: &crate::CommandBuffer<VulkanBackend>,
    ) -> crate::Result<vk::CommandBuffer> {
        ensure_same_device(
            self.backend,
            &command_buffer.backend,
            "Vulkan command-buffer interop",
        )?;
        Ok(command_buffer.raw.raw)
    }
}

fn ensure_same_device<T>(
    expected: &Arc<T>,
    actual: &Arc<T>,
    operation: &'static str,
) -> crate::Result<()> {
    if Arc::ptr_eq(expected, actual) {
        Ok(())
    } else {
        Err(crate::Error::DeviceMismatch { operation })
    }
}

impl crate::backend::sealed::InteropSealed for Device<VulkanBackend> {}

impl BackendInterop for Device<VulkanBackend> {
    type Adapter<'a> = VulkanInterop<'a>;

    fn interop(&self) -> Self::Adapter<'_> {
        VulkanInterop {
            backend: &self.backend,
        }
    }
}

#[derive(Debug)]
struct VulkanError(String);

impl fmt::Display for VulkanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl error::Error for VulkanError {}

pub(super) fn unsupported(message: impl Into<String>) -> crate::Error {
    crate::Error::backend(VulkanError(message.into()))
}

pub(super) fn map_error(context: &'static str, error: impl fmt::Display) -> crate::Error {
    unsupported(format!("{context}: {error}"))
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interop_identity_rejects_another_device() {
        let first = Arc::new(());
        let second = Arc::new(());

        assert!(ensure_same_device(&first, &first, "test").is_ok());
        assert!(matches!(
            ensure_same_device(&first, &second, "test"),
            Err(crate::Error::DeviceMismatch { .. })
        ));
    }

    #[test]
    fn vulkan_device_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<VulkanDevice>();
    }
}
