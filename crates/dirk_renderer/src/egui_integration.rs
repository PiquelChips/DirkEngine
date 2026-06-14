use std::time::Instant;

use ash::vk;
use egui::{ClippedPrimitive, Context, TextureId, TexturesDelta};
use egui_ash_renderer::{DynamicRendering, Options};

use crate::{
    MAX_FRAMES_IN_FLIGHT, Result,
    resources::{command_pool::CommandBuffer, device::RenderDevice, queues::QueueType},
};

pub struct EguiState {
    ctx: Context,
    renderer: egui_ash_renderer::Renderer,
    start_time: Instant,
    pending: Option<EguiPaintData>,
    textures_to_free: [Vec<TextureId>; MAX_FRAMES_IN_FLIGHT],
}

impl EguiState {
    pub fn new(device: &RenderDevice) -> Result<Self> {
        let renderer = egui_ash_renderer::Renderer::with_default_allocator(
            &device.instance,
            device.physical_device,
            device.device.clone(),
            DynamicRendering {
                color_attachment_format: device.properties.surface_format.format,
                depth_attachment_format: None,
            },
            Options {
                in_flight_frames: MAX_FRAMES_IN_FLIGHT,
                ..Options::default()
            },
        )?;

        Ok(Self {
            ctx: Context::default(),
            renderer,
            start_time: Instant::now(),
            pending: None,
            textures_to_free: std::array::from_fn(|_| Vec::new()),
        })
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn begin_frame(&self, extent: vk::Extent2D) -> Context {
        let screen_rect = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(extent.width as f32, extent.height as f32),
        );

        self.ctx.begin_pass(egui::RawInput {
            screen_rect: Some(screen_rect),
            time: Some(self.start_time.elapsed().as_secs_f64()),
            ..egui::RawInput::default()
        });
        self.ctx.clone()
    }

    pub fn end_frame(&mut self) {
        let output = self.ctx.end_pass();
        let primitives = self.ctx.tessellate(output.shapes, output.pixels_per_point);
        self.pending = Some(EguiPaintData {
            textures_delta: output.textures_delta,
            primitives,
            pixels_per_point: output.pixels_per_point,
        });
    }

    pub fn free_textures_for_frame(&mut self, frame: usize) -> Result<()> {
        let textures = std::mem::take(&mut self.textures_to_free[frame]);
        self.renderer.free_textures(&textures)?;
        Ok(())
    }

    pub fn render(
        &mut self,
        device: &RenderDevice,
        cmd: &CommandBuffer,
        extent: vk::Extent2D,
        frame: usize,
    ) -> Result<()> {
        let Some(pending) = self.pending.take() else {
            return Ok(());
        };

        self.renderer.set_textures(
            device.queues.raw(QueueType::Graphics),
            device.graphics_pool.raw(),
            pending.textures_delta.set.as_slice(),
        )?;

        self.renderer.cmd_draw(
            **cmd,
            extent,
            pending.pixels_per_point,
            pending.primitives.as_slice(),
        )?;

        self.textures_to_free[frame].extend(pending.textures_delta.free);
        Ok(())
    }
}

struct EguiPaintData {
    textures_delta: TexturesDelta,
    primitives: Vec<ClippedPrimitive>,
    pixels_per_point: f32,
}
