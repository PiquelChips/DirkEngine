use ash::vk;

pub trait VertexInput: Sized {
    const ATTRIBUTES: &'static [vk::VertexInputAttributeDescription];

    fn binding(binding: u32) -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding,
            stride: size_of::<Self>() as u32,
            // we default to vertex input for now
            input_rate: vk::VertexInputRate::VERTEX,
        }
    }

    fn layout() -> VertexInputLayout {
        let binding = Self::binding(0);
        VertexInputLayout {
            stride: binding.stride,
            input_rate: binding.input_rate,
            attributes: Self::ATTRIBUTES,
        }
    }
}

pub struct VertexInputLayout {
    stride: u32,
    input_rate: vk::VertexInputRate,
    attributes: &'static [vk::VertexInputAttributeDescription],
}

impl VertexInputLayout {
    pub fn binding(&self, binding: u32) -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding,
            stride: self.stride,
            input_rate: self.input_rate,
        }
    }
}
