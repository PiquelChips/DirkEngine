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

impl Drop for Frame {
    fn drop(&mut self) {
        self.command_pool.destroy();
        unsafe {
            self.device.destroy_fence(self.fence, None);
        }
    }
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
