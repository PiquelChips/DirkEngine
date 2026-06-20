//! Synchronization resource wrappers.

use ash::{Device, prelude::VkResult, vk};

/// RAII wrapper for a Vulkan fence.
pub struct Fence {
    device: Device,
    raw: vk::Fence,
}

impl Fence {
    /// Creates a fence with the supplied creation flags.
    pub fn create(device: &Device, flags: vk::FenceCreateFlags) -> VkResult<Self> {
        let info = vk::FenceCreateInfo::default().flags(flags);
        let raw = unsafe { device.create_fence(&info, None)? };

        Ok(Self {
            device: device.clone(),
            raw,
        })
    }

    /// Creates an unsignaled fence.
    pub fn unsignaled(device: &Device) -> VkResult<Self> {
        Self::create(device, vk::FenceCreateFlags::empty())
    }

    /// Creates a signaled fence.
    pub fn signaled(device: &Device) -> VkResult<Self> {
        Self::create(device, vk::FenceCreateFlags::SIGNALED)
    }

    /// Returns the raw Vulkan fence handle.
    pub fn raw(&self) -> vk::Fence {
        self.raw
    }

    /// Returns the raw null fence handle for Vulkan calls that take an optional fence.
    pub(crate) fn null_handle() -> vk::Fence {
        vk::Fence::null()
    }

    /// Waits for this fence to become signaled.
    pub fn wait(&self, timeout: u64) -> VkResult<()> {
        unsafe {
            self.device
                .wait_for_fences(std::slice::from_ref(&self.raw), true, timeout)
        }
    }

    /// Resets this fence to the unsignaled state.
    pub fn reset(&self) -> VkResult<()> {
        unsafe { self.device.reset_fences(std::slice::from_ref(&self.raw)) }
    }
}

impl Drop for Fence {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_fence(self.raw, None);
        }
    }
}
