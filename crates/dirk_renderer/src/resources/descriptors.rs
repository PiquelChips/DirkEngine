//! Safe descriptor layout and descriptor pool helpers.

mod allocator;
mod layout_types;
mod set;
pub mod slots;
mod writer;

use ash::{Device, vk};

pub use allocator::DescriptorAllocator;
pub use layout_types::{MaterialLayout, ObjectLayout, SceneLayout, SetLayout};
pub use set::DescriptorSet;
pub use writer::DescriptorWriter;

use crate::Result;

/// All descriptor set layouts used by the renderer.
pub struct DescriptorLayouts {
    /// Per scene layout. Holds view and projection matrices for rendering.
    pub scene: vk::DescriptorSetLayout,
    /// Per object layout. Holds the model matrix.
    pub object: vk::DescriptorSetLayout,
    /// Per material layout. Holds a base color texture descriptor.
    pub material: vk::DescriptorSetLayout,
}

impl DescriptorLayouts {
    /// Creates the descriptor set layouts used by the renderer.
    pub fn create(device: &Device) -> Result<Self> {
        Ok(Self {
            scene: create_layout::<SceneLayout>(device)?,
            object: create_layout::<ObjectLayout>(device)?,
            material: create_layout::<MaterialLayout>(device)?,
        })
    }

    /// Returns the layouts in pipeline set order.
    pub fn pipeline_layouts(&self) -> [vk::DescriptorSetLayout; 3] {
        let mut layouts = [vk::DescriptorSetLayout::null(); 3];
        layouts[SceneLayout::SET_INDEX] = self.scene;
        layouts[ObjectLayout::SET_INDEX] = self.object;
        layouts[MaterialLayout::SET_INDEX] = self.material;
        layouts
    }

    /// Destroys every descriptor set layout.
    pub fn destroy(&self, device: &Device) {
        unsafe {
            device.destroy_descriptor_set_layout(self.scene, None);
            device.destroy_descriptor_set_layout(self.object, None);
            device.destroy_descriptor_set_layout(self.material, None);
        }
    }
}

fn create_layout<L: SetLayout>(device: &Device) -> Result<vk::DescriptorSetLayout> {
    let binding = vk::DescriptorSetLayoutBinding::default()
        .binding(L::BINDING)
        .descriptor_type(L::DESCRIPTOR_TYPE)
        .descriptor_count(L::DESCRIPTORS_PER_SET)
        .stage_flags(L::STAGE);
    let info =
        vk::DescriptorSetLayoutCreateInfo::default().bindings(std::slice::from_ref(&binding));

    Ok(unsafe { device.create_descriptor_set_layout(&info, None)? })
}
