use ash::vk;
use dirk_platform::WindowId;
use dirk_player::PlayerId;
use dirk_universe::{Entity, WorldId};
use gpu_allocator::MemoryLocation;

use crate::{
    Result,
    frame_graph::{ImportedTexture, TextureStateDesc},
    resources::{
        device::RenderDevice,
        image::{Image, ImageCreateInfo},
        sync::TimelineSemaphore,
    },
};

#[derive(Clone, Copy)]
pub(crate) struct TextureState {
    layout: vk::ImageLayout,
    stage: vk::PipelineStageFlags2,
    access: vk::AccessFlags2,
}

impl From<TextureState> for TextureStateDesc {
    fn from(state: TextureState) -> Self {
        Self {
            layout: state.layout,
            stage: state.stage,
            access: state.access,
        }
    }
}

pub(crate) struct Viewport {
    player: PlayerId,
    window: WindowId,
    camera: Option<Entity>,
    world: Option<WorldId>,
    settings: ViewportSettings,
    output: Image,
    output_state: TextureState,
    render_semaphore: TimelineSemaphore,
    last_render_value: u64,
}

impl Viewport {
    pub(crate) fn new(
        device: &RenderDevice,
        player: PlayerId,
        window: WindowId,
        settings: ViewportSettings,
    ) -> Result<Self> {
        let settings = settings.clamped();

        Ok(Self {
            player,
            window,
            camera: None,
            world: None,
            settings,
            output: Self::create_output(device, &settings)?,
            output_state: Viewport::undefined_state(),
            render_semaphore: TimelineSemaphore::create(&device.device, 0)?,
            last_render_value: 0,
        })
    }

    pub(crate) fn player(&self) -> PlayerId {
        self.player
    }

    pub(crate) fn camera(&self) -> Option<Entity> {
        self.camera
    }

    pub(crate) fn world(&self) -> Option<WorldId> {
        self.world
    }

    pub(crate) fn settings(&self) -> &ViewportSettings {
        &self.settings
    }

    pub(crate) fn sync_from_player(
        &mut self,
        window: WindowId,
        camera: Option<Entity>,
        world: Option<WorldId>,
    ) {
        self.window = window;
        self.camera = camera;
        self.world = world;
    }

    pub(crate) fn resize(&mut self, device: &RenderDevice, extent: vk::Extent2D) -> Result<()> {
        self.reconfigure(
            device,
            ViewportSettings {
                extent,
                ..self.settings
            },
        )
    }

    pub(crate) fn reconfigure(
        &mut self,
        device: &RenderDevice,
        settings: ViewportSettings,
    ) -> Result<()> {
        let settings = settings.clamped();
        if self.settings == settings {
            return Ok(());
        }

        self.settings = settings;
        self.output = Self::create_output(device, &self.settings)?;
        Ok(())
    }

    pub(crate) fn import(&self) -> ImportedTexture {
        ImportedTexture {
            image: self.output.image(),
            view: self.output.view(),
            aspect_flags: self.output.aspect_flags(),
            initial_state: self.output_state.into(),
            final_state: Self::shader_read_state().into(),
        }
    }

    pub(crate) fn next_render_value(&self) -> u64 {
        self.last_render_value + 1
    }

    pub(crate) fn render_semaphore(&self) -> vk::Semaphore {
        self.render_semaphore.raw()
    }

    pub(crate) fn mark_render_submitted(&mut self, value: u64) {
        self.last_render_value = value;
        self.output_state = Self::shader_read_state();
    }

    fn undefined_state() -> TextureState {
        TextureState {
            layout: vk::ImageLayout::UNDEFINED,
            stage: vk::PipelineStageFlags2::TOP_OF_PIPE,
            access: vk::AccessFlags2::empty(),
        }
    }

    fn shader_read_state() -> TextureState {
        TextureState {
            layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            stage: vk::PipelineStageFlags2::FRAGMENT_SHADER,
            access: vk::AccessFlags2::SHADER_READ,
        }
    }

    fn create_output(device: &RenderDevice, settings: &ViewportSettings) -> Result<Image> {
        Image::create_image(
            device,
            &ImageCreateInfo {
                size: settings.extent,
                format: settings.format,
                tiling: vk::ImageTiling::OPTIMAL,
                usage: vk::ImageUsageFlags::COLOR_ATTACHMENT
                    | vk::ImageUsageFlags::SAMPLED
                    | vk::ImageUsageFlags::TRANSFER_SRC,
                location: MemoryLocation::GpuOnly,
                mip_levels: 1,
                num_samples: vk::SampleCountFlags::TYPE_1,
                aspect_flags: vk::ImageAspectFlags::COLOR,
            },
        )
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct ViewportSettings {
    pub extent: vk::Extent2D,
    pub format: vk::Format,
    pub clear_color: [f32; 4],
    pub fov_y_radians: f32,
    pub near: f32,
    pub far: f32,
}

impl ViewportSettings {
    pub(crate) fn new(extent: vk::Extent2D, format: vk::Format) -> Self {
        Self {
            extent,
            format,
            clear_color: [0.0, 0.0, 0.0, 1.0],
            fov_y_radians: 45_f32.to_radians(),
            near: 0.1,
            far: 100_000.0,
        }
    }

    fn clamped(self) -> Self {
        Self {
            extent: vk::Extent2D {
                width: self.extent.width.max(1),
                height: self.extent.height.max(1),
            },
            ..self
        }
    }
}
