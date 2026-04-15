use ash::{Device, vk};

use crate::{
    physical_device,
    resources::command_pool::{CommandPool, Graphics},
};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct Vertex {
    pub(crate) position: [f32; 3],
    pub(crate) normal: [f32; 3],
    pub(crate) texcoord: [f32; 2],
}

impl Vertex {
    pub(crate) const fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }
    }
    pub(crate) const fn attribute_description() -> [vk::VertexInputAttributeDescription; 3] {
        [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: std::mem::offset_of!(Self, position) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: std::mem::offset_of!(Self, normal) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: std::mem::offset_of!(Self, texcoord) as u32,
            },
        ]
    }
}

pub(crate) fn make_version(version: utils::Version) -> u32 {
    vk::make_api_version(0, version.major(), version.minor(), version.patch())
}

pub(crate) struct Frame {
    pub(crate) device: Device,
    /// Command pool to allocate command buffers on every frame
    pub(crate) command_pool: CommandPool<Graphics>,
    /// Main synchronization fence
    pub(crate) fence: vk::Fence,
    // TODO: have one primary command buffer that is allocated once and
    // secondary command for each scene. Should be allocated every time
    // there is a change in scene count. If not reallocated, reset.
}

impl Frame {
    pub(crate) fn destroy(&self) {
        self.command_pool.destroy();
        unsafe {
            self.device.destroy_fence(self.fence, None);
        }
    }
}

/// This struct is owned by [Renderer] and stores
/// all the different descriptor set layouts used by
/// the renderer.
/// Every field should be a descriptor set layout with a
/// propper comment explain what the layout is and where
/// it is used.
pub(crate) struct DescriptorLayouts {
    // TODO: much better comments for descriptor set layouts
    /// Per scene layout. Holds view & proj matrices for rendering.
    pub(crate) scene: vk::DescriptorSetLayout,
    /// Per object layout. For model matrix.
    pub(crate) object: vk::DescriptorSetLayout,
    /// Per material layout. For texture descriptor.
    pub(crate) material: vk::DescriptorSetLayout,
}

impl DescriptorLayouts {
    pub(crate) fn destroy(&self, device: &Device) {
        unsafe {
            device.destroy_descriptor_set_layout(self.scene, None);
            device.destroy_descriptor_set_layout(self.object, None);
            device.destroy_descriptor_set_layout(self.material, None);
        }
    }
}

pub(crate) struct Queues {
    pub(crate) graphics: vk::Queue,
    pub(crate) compute: vk::Queue,
    pub(crate) transfer: vk::Queue,
    pub(crate) present: vk::Queue,
}

pub(crate) struct RendererProperties {
    pub(crate) msaa_samples: vk::SampleCountFlags,
    #[allow(unused)]
    pub(crate) anisotropy: bool,
    pub(crate) surface_format: vk::SurfaceFormatKHR,
    pub(crate) queue_family_indices: physical_device::QueueFamilyIndices,
    pub(crate) depth_format: vk::Format,
    pub(crate) present_mode: vk::PresentModeKHR,
}
