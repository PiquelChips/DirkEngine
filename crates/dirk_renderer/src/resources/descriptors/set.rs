//! Owned descriptor set handle.

use std::marker::PhantomData;

use ash::vk;

use crate::resources::device::{Garbage, RenderDevice};

use super::layout_types::SetLayout;

/// An owned Vulkan descriptor set with deferred RAII cleanup.
///
/// The phantom type `L` ensures that descriptor sets allocated for different
/// layouts cannot be mixed up at the call site.
pub struct DescriptorSet<L: SetLayout> {
    device: RenderDevice,
    pool: vk::DescriptorPool,
    raw: vk::DescriptorSet,
    _layout: PhantomData<L>,
}

impl<L: SetLayout> DescriptorSet<L> {
    /// Returns the raw Vulkan handle.
    #[inline]
    pub fn raw(&self) -> vk::DescriptorSet {
        self.raw
    }

    pub(super) fn new(
        device: RenderDevice,
        pool: vk::DescriptorPool,
        raw: vk::DescriptorSet,
    ) -> Self {
        Self {
            device,
            pool,
            raw,
            _layout: PhantomData,
        }
    }
}

impl<L: SetLayout> Drop for DescriptorSet<L> {
    fn drop(&mut self) {
        self.device.destroy(Garbage::DescriptorSet {
            pool: self.pool,
            set: self.raw,
        });
    }
}
