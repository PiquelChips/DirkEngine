use ash::{Device, vk};

use crate::{
    physical_device,
    resources::command_pool::{CommandPool, Graphics},
};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub texcoord: [f32; 2],
}

impl Vertex {
    pub const fn binding_description() -> vk::VertexInputBindingDescription {
        // the size_of::<Self> is far from u32::MAX
        #[allow(clippy::cast_possible_truncation)]
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }
    }
    // the offset is far from u32::MAX
    #[allow(clippy::cast_possible_truncation)]
    pub const fn attribute_description() -> [vk::VertexInputAttributeDescription; 3] {
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

pub fn make_version(version: dirk_utils::Version) -> u32 {
    vk::make_api_version(0, version.major(), version.minor(), version.patch())
}

pub struct Frame {
    pub device: Device,
    /// Command pool to allocate command buffers on every frame
    pub command_pool: CommandPool<Graphics>,
    /// Main synchronization fence
    pub fence: vk::Fence,
    // TODO: have one primary command buffer that is allocated once and
    // secondary command for each scene. Should be allocated every time
    // there is a change in scene count. If not reallocated, reset.
}

impl Frame {
    pub fn destroy(&self) {
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
pub struct DescriptorLayouts {
    // TODO: much better comments for descriptor set layouts
    /// Per scene layout. Holds view & proj matrices for rendering.
    pub scene: vk::DescriptorSetLayout,
    /// Per object layout. For model matrix.
    pub object: vk::DescriptorSetLayout,
    /// Per material layout. For texture descriptor.
    pub material: vk::DescriptorSetLayout,
}

impl DescriptorLayouts {
    pub fn destroy(&self, device: &Device) {
        unsafe {
            device.destroy_descriptor_set_layout(self.scene, None);
            device.destroy_descriptor_set_layout(self.object, None);
            device.destroy_descriptor_set_layout(self.material, None);
        }
    }
}

pub struct Queues {
    pub graphics: vk::Queue,
    pub compute: vk::Queue,
    pub transfer: vk::Queue,
    pub present: vk::Queue,
}

pub struct RendererProperties {
    pub msaa_samples: vk::SampleCountFlags,
    #[allow(unused)]
    pub anisotropy: bool,
    pub surface_format: vk::SurfaceFormatKHR,
    pub queue_family_indices: physical_device::QueueFamilyIndices,
    pub depth_format: vk::Format,
    pub present_mode: vk::PresentModeKHR,
}
