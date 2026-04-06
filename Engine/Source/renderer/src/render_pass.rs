use ash::vk::{self};

use crate::{Renderer, Result};

/// This struct holds the graphics pipeline & stuff.
/// It can be called on to begin the pass (begin rendering,
/// bind graphics pipeline, ...)
pub struct RenderPass {
    color: vk::ImageView,
    color_image: vk::Image,
    color_memory: vk::DeviceMemory,
    depth: vk::ImageView,
    depth_image: vk::Image,
    depth_memory: vk::DeviceMemory,
}

impl RenderPass {
    pub fn build(renderer: &Renderer, size: vk::Extent2D) -> Result<Self> {
        let (color_image, color_memory) = renderer.create_image(
            size,
            renderer.properties.surface_format.format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::TRANSIENT_ATTACHMENT | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            (1, renderer.properties.msaa_samples),
        )?;
        let color = renderer.create_image_view(
            color_image,
            renderer.properties.surface_format.format,
            vk::ImageAspectFlags::COLOR,
            1,
        )?;

        let (depth_image, depth_memory) = renderer.create_image(
            size,
            renderer.properties.depth_format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            (1, vk::SampleCountFlags::TYPE_1),
        )?;
        let depth = renderer.create_image_view(
            depth_image,
            renderer.properties.depth_format,
            vk::ImageAspectFlags::DEPTH,
            1,
        )?;

        Ok(Self {
            color,
            color_image,
            color_memory,
            depth,
            depth_image,
            depth_memory,
        })
    }
    pub fn begin(
        &self,
        renderer: &Renderer,
        cmd: vk::CommandBuffer,
        size: vk::Extent2D,
        out: vk::ImageView,
    ) {
        let color_attachement = vk::RenderingAttachmentInfo::default()
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .image_view(self.color)
            .resolve_image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .resolve_mode(vk::ResolveModeFlags::AVERAGE)
            .resolve_image_view(out)
            .clear_value(vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0., 0., 0., 1.],
                },
            });

        let depth_info = vk::RenderingAttachmentInfo::default()
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .image_view(self.depth)
            .clear_value(vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.,
                    stencil: 0,
                },
            });

        let rendering_info = vk::RenderingInfo::default()
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: size,
            })
            .color_attachments(std::slice::from_ref(&color_attachement))
            .depth_attachment(&depth_info)
            .layer_count(1);

        unsafe { renderer.device.cmd_begin_rendering(cmd, &rendering_info) };
    }
    pub fn end(renderer: &Renderer, cmd: vk::CommandBuffer) {
        unsafe { renderer.device.cmd_end_rendering(cmd) }
    }
}
