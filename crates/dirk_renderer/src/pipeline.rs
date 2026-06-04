use std::marker::PhantomData;

use ash::vk;

use crate::{
    Result,
    resources::{
        command_pool::CommandBuffer,
        device::{Garbage, RenderDevice},
    },
    shaders::metadata::{FragmentShader, VertexShader},
};

pub struct GraphicsPipeline<V, F>
where
    V: VertexShader,
    F: FragmentShader,
{
    device: RenderDevice,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    _vert: PhantomData<V>,
    _frag: PhantomData<F>,
}

impl<V, F> GraphicsPipeline<V, F>
where
    V: VertexShader,
    F: FragmentShader,
{
    pub fn build(device: &RenderDevice) -> Result<Self> {
        let mut device = device.clone();

        let set_layouts = Self::create_pipeline_set_layouts(&mut device)?;

        let layout_info = vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts);
        let pipeline_layout = unsafe { device.device.create_pipeline_layout(&layout_info, None)? };

        let shader_stages = [
            V::shader_create_info(&mut device)?,
            F::shader_create_info(&mut device)?,
        ];

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(V::INPUT_BINDINGS)
            .vertex_attribute_descriptions(V::INPUT_ATTRIBUTES);

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let dynamic_state = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false) // enabling this disables output to frame buffer
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false);

        let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(device.properties.msaa_samples);

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA);

        let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .attachments(std::slice::from_ref(&color_blend_attachment));

        let depth_test_info = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS);

        let mut pipeline_rendering_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(std::slice::from_ref(
                &device.properties.surface_format.format,
            ))
            .depth_attachment_format(device.properties.depth_format);

        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisampling)
            .depth_stencil_state(&depth_test_info)
            .color_blend_state(&color_blending)
            .dynamic_state(&dynamic_state)
            .layout(pipeline_layout)
            .subpass(0)
            .base_pipeline_handle(vk::Pipeline::null())
            .base_pipeline_index(-1)
            .push_next(&mut pipeline_rendering_info);

        let pipeline = unsafe {
            device
                .device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    std::slice::from_ref(&pipeline_info),
                    None,
                )
                .map_err(|(_, err)| err)?[0]
        };

        Ok(Self {
            device,
            pipeline,
            pipeline_layout,
            _vert: PhantomData,
            _frag: PhantomData,
        })
    }
    pub fn bind(&self, cmd: &CommandBuffer) {
        cmd.bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline);
    }
    pub fn layout(&self) -> vk::PipelineLayout {
        self.pipeline_layout
    }
    fn create_pipeline_set_layouts(
        device: &mut RenderDevice,
    ) -> Result<Vec<vk::DescriptorSetLayout>> {
        let max_sets = V::SET_LAYOUTS.len().max(F::SET_LAYOUTS.len());
        (0..max_sets)
            .map(|set| {
                let mut bindings = Vec::new();
                bindings.extend(
                    V::SET_LAYOUTS
                        .get(set)
                        .copied()
                        .unwrap_or_default()
                        .iter()
                        .copied(),
                );
                bindings.extend(
                    F::SET_LAYOUTS
                        .get(set)
                        .copied()
                        .unwrap_or_default()
                        .iter()
                        .copied(),
                );
                bindings.sort_by_key(|binding| binding.binding);
                let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
                let layout = unsafe { device.device.create_descriptor_set_layout(&info, None)? };
                device.destroy(Garbage::DescriptorSetLayout(layout));
                Ok(layout)
            })
            .collect()
    }
}

impl<V: VertexShader, F: FragmentShader> Drop for GraphicsPipeline<V, F> {
    fn drop(&mut self) {
        self.device
            .destroy(Garbage::PipelineLayout(self.pipeline_layout));
        self.device.destroy(Garbage::Pipeline(self.pipeline));
    }
}
