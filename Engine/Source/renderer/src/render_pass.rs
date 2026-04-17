use ash::{Device, vk};

use crate::resources::{command_pool::CommandBuffer, image::Image};

/// This struct holds the graphics pipeline & stuff.
/// It can be called on to begin the pass (begin rendering,
/// bind graphics pipeline, ...)
pub struct RenderPass {}

impl RenderPass {
    pub fn begin(
        device: &Device,
        cmd: &CommandBuffer,
        size: vk::Extent2D,
        out: vk::ImageView,
        color: &Image,
        depth: &Image,
    ) {
        let color_attachement = vk::RenderingAttachmentInfo::default()
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .image_view(color.view())
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
            .image_view(depth.view())
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

        unsafe { device.cmd_begin_rendering(cmd.raw(), &rendering_info) };
    }
    pub fn end(device: &Device, cmd: &CommandBuffer) {
        unsafe { device.cmd_end_rendering(cmd.raw()) }
    }
}
