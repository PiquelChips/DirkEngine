use ash::vk;

use crate::{Error, Result, command_pool::CommandBuffer, image::Image};

impl Image {
    pub fn transition_image_layout(
        &self,
        cmd: &CommandBuffer,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
        mip_levels: u32,
        base_mip: u32,
    ) -> Result<()> {
        let (src_access, src_stage) = Self::src_layout_info(old_layout)?;
        let (dst_access, dst_stage) = Self::dst_layout_info(new_layout)?;

        let barrier = vk::ImageMemoryBarrier::default()
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .src_access_mask(src_access)
            .dst_access_mask(dst_access)
            .image(self.image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: base_mip,
                level_count: mip_levels,
                base_array_layer: 0,
                layer_count: 1,
            });

        unsafe {
            self.device.cmd_pipeline_barrier(
                cmd.raw(),
                src_stage,
                dst_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            )
        };
        Ok(())
    }

    fn src_layout_info(
        layout: vk::ImageLayout,
    ) -> Result<(vk::AccessFlags, vk::PipelineStageFlags)> {
        match layout {
            vk::ImageLayout::UNDEFINED => Ok((
                vk::AccessFlags::empty(),
                vk::PipelineStageFlags::TOP_OF_PIPE,
            )),
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL => Ok((
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            )),
            vk::ImageLayout::TRANSFER_DST_OPTIMAL => Ok((
                vk::AccessFlags::TRANSFER_WRITE,
                vk::PipelineStageFlags::TRANSFER,
            )),
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL => Ok((
                vk::AccessFlags::TRANSFER_READ,
                vk::PipelineStageFlags::TRANSFER,
            )),
            vk::ImageLayout::PRESENT_SRC_KHR => Ok((
                vk::AccessFlags::empty(),
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            )),
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL => Ok((
                vk::AccessFlags::SHADER_READ,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
            )),
            _ => Err(Error::UnsupportedSourceLayout(layout)),
        }
    }
    fn dst_layout_info(
        layout: vk::ImageLayout,
    ) -> Result<(vk::AccessFlags, vk::PipelineStageFlags)> {
        match layout {
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL => Ok((
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            )),
            vk::ImageLayout::TRANSFER_DST_OPTIMAL => Ok((
                vk::AccessFlags::TRANSFER_WRITE,
                vk::PipelineStageFlags::TRANSFER,
            )),
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL => Ok((
                vk::AccessFlags::TRANSFER_READ,
                vk::PipelineStageFlags::TRANSFER,
            )),
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL => Ok((
                vk::AccessFlags::SHADER_READ,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
            )),
            vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL => Ok((
                vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                    | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            )),
            vk::ImageLayout::PRESENT_SRC_KHR => Ok((
                vk::AccessFlags::empty(),
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            )),
            _ => Err(Error::UnsupportedDesinationLayout(layout)),
        }
    }
}
