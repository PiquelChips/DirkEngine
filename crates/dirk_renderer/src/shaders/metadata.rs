#![allow(unused)]

use ash::vk;
use std::ffi::CStr;

use crate::{
    Result,
    resources::device::{Garbage, RenderDevice},
    shaders::ShaderCode,
};

pub trait VertexInput: Sized + Copy {
    const ATTRIBUTES: &'static [vk::VertexInputAttributeDescription];

    fn binding(binding: u32) -> vk::VertexInputBindingDescription {
        // size_of::<Self> will never reach u32::MAX
        #[allow(clippy::cast_possible_truncation)]
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

pub trait VertexInputs {
    fn layouts() -> Vec<VertexInputLayout>;
}

impl<A> VertexInputs for A
where
    A: VertexInput,
{
    fn layouts() -> Vec<VertexInputLayout> {
        vec![A::layout()]
    }
}

macro_rules! impl_vertex_inputs_for_tuple {
    ($($name:ident),+ $(,)?) => {
        impl<$($name),+> VertexInputs for ($($name,)+)
        where
            $($name: VertexInput),+
        {
            fn layouts() -> Vec<VertexInputLayout> {
                vec![$($name::layout()),+]
            }
        }
    };
}

impl_vertex_inputs_for_tuple!(A, B);
impl_vertex_inputs_for_tuple!(A, B, C);
impl_vertex_inputs_for_tuple!(A, B, C, D);
impl_vertex_inputs_for_tuple!(A, B, C, D, E);
impl_vertex_inputs_for_tuple!(A, B, C, D, E, F);
impl_vertex_inputs_for_tuple!(A, B, C, D, E, F, G);
impl_vertex_inputs_for_tuple!(A, B, C, D, E, F, G, H);

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

    #[must_use]
    pub fn attribute_count(&self) -> usize {
        self.attributes.len()
    }

    pub fn attrs(
        &self,
        binding: u32,
        location_offset: u32,
    ) -> Vec<vk::VertexInputAttributeDescription> {
        self.attributes
            .iter()
            .copied()
            .map(|mut attr| {
                attr.binding = binding;
                attr.location += location_offset;
                attr
            })
            .collect()
    }
}

pub trait Shader {
    const CODE: ShaderCode;
    const ENTRYPOINT: &'static CStr;
    const SET_LAYOUTS: &'static [&'static [vk::DescriptorSetLayoutBinding<'static>]];

    fn shader_create_info_for_stage<'a>(
        device: &mut RenderDevice,
        stage: vk::ShaderStageFlags,
    ) -> Result<vk::PipelineShaderStageCreateInfo<'a>> {
        let module = Self::CODE.create_module(&device.device)?;
        device.destroy(Garbage::Shader(module));
        Ok(vk::PipelineShaderStageCreateInfo::default()
            .stage(stage)
            .module(module)
            .name(Self::ENTRYPOINT))
    }

    fn set_layouts(device: &mut RenderDevice) -> Result<Vec<vk::DescriptorSetLayout>> {
        Self::SET_LAYOUTS
            .iter()
            .map(|&set| {
                let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(set);
                let layout = unsafe { device.device.create_descriptor_set_layout(&info, None)? };
                device.destroy(Garbage::DescriptorSetLayout(layout));
                Ok(layout)
            })
            .collect::<Result<Vec<_>>>()
    }
}

pub trait VertexShader: Shader {
    const STAGE: vk::ShaderStageFlags = vk::ShaderStageFlags::VERTEX;
    type Input: VertexInputs;

    fn shader_create_info<'a>(
        device: &mut RenderDevice,
    ) -> Result<vk::PipelineShaderStageCreateInfo<'a>> {
        Self::shader_create_info_for_stage(device, Self::STAGE)
    }

    // TODO: when creating the pipeline, compare these against
    // the layouts in GraphicsPipelineInfo. Return error if needed &
    // use to determine bindings & locations
    fn input_layouts() -> Vec<VertexInputLayout> {
        Self::Input::layouts()
    }
}

pub trait FragmentShader: Shader {
    const STAGE: vk::ShaderStageFlags = vk::ShaderStageFlags::FRAGMENT;
    fn shader_create_info<'a>(
        device: &mut RenderDevice,
    ) -> Result<vk::PipelineShaderStageCreateInfo<'a>> {
        Self::shader_create_info_for_stage(device, Self::STAGE)
    }
}
