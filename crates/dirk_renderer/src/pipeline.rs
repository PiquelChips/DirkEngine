use ash::vk;

use crate::{
    Renderer, Result,
    resources::{
        command_pool::CommandBuffer,
        descriptors::DescriptorLayouts,
        device::{Garbage, RenderDevice},
    },
    shaders::{
        FragmentShader, VertexShader,
        metadata::{VertexInput, VertexInputLayout},
    },
    utils::Vertex,
};

pub struct GraphicsPipelineInfo {
    vert: VertexShader,
    frag: FragmentShader,
}

pub struct GraphicsPipeline {
    device: RenderDevice,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
}

impl GraphicsPipeline {
    pub fn build(
        device: &RenderDevice,
        layouts: &DescriptorLayouts,
        pipeline_info: GraphicsPipelineInfo,
    ) -> Result<Self> {
        let set_layouts = layouts.pipeline_layouts();
        let layout_info = vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts);
        let pipeline_layout = unsafe { device.device.create_pipeline_layout(&layout_info, None)? };

        let vert = Renderer::create_shader_module(&device.device, &pipeline_info.vert)?;
        let vert_name = pipeline_info.vert.entrypoint();

        let frag = Renderer::create_shader_module(&device.device, &pipeline_info.frag)?;
        let frag_name = pipeline_info.frag.entrypoint();

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert)
                .name(vert_name),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag)
                .name(frag_name),
        ];

        #[allow(clippy::cast_possible_truncation)]
        let binding_description = Vertex::binding(0);
        let attribute_description = Vertex::ATTRIBUTES;
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(std::slice::from_ref(&binding_description))
            .vertex_attribute_descriptions(attribute_description);

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

        unsafe {
            device.device.destroy_shader_module(vert, None);
            device.device.destroy_shader_module(frag, None);
        }

        Ok(Self {
            device: device.clone(),
            pipeline,
            pipeline_layout,
        })
    }
    pub fn bind(&self, cmd: &CommandBuffer) {
        cmd.bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline);
    }
    pub fn layout(&self) -> vk::PipelineLayout {
        self.pipeline_layout
    }
}

impl Drop for GraphicsPipeline {
    fn drop(&mut self) {
        self.device
            .destroy(Garbage::PipelineLayout(self.pipeline_layout));
        self.device.destroy(Garbage::Pipeline(self.pipeline));
    }
}
