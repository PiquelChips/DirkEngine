//! Explicit command pools, command recording, and queue synchronization.

use std::{
    cell::{Cell, RefCell},
    fmt,
    rc::Rc,
    sync::Arc,
};

use crate::{
    AccessTypes, Backend, BindGroup, Buffer, Device, Error, ImageLayout, ImageRef,
    ImageSubresourceRange, ImageViewRef, Pipeline, PipelineLayout, PipelineStages, QueueType,
    ResourceState, Result, Semaphore, SemaphoreKind, flags::define_flags,
};

define_flags! {
    /// Command-pool behavior hints.
    pub struct CommandPoolFlags(u8) {
        /// Buffers allocated from the pool are short-lived.
        const TRANSIENT = 1 << 0;
        /// Individual command buffers may be reset.
        const RESET_COMMAND_BUFFER = 1 << 1;
    }
}

define_flags! {
    /// Command-buffer recording behavior hints.
    pub struct CommandBufferUsage(u8) {
        /// The command buffer will be submitted once before being reset.
        const ONE_TIME_SUBMIT = 1 << 0;
        /// Allows native backends to record for simultaneous use.
        ///
        /// The safe RHI still requires fence completion before resubmission.
        const SIMULTANEOUS_USE = 1 << 1;
    }
}

/// Command-buffer nesting level.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CommandBufferLevel {
    /// A directly submitted command buffer.
    #[default]
    Primary,
    /// A command buffer executed by another command buffer.
    Secondary,
}

/// Description of an explicit command pool.
#[derive(Clone, Copy, Debug)]
pub struct CommandPoolCreateInfo<'a> {
    /// Queue used by buffers allocated from this pool.
    pub queue: QueueType,
    /// Pool behavior flags.
    pub flags: CommandPoolFlags,
    /// Optional diagnostic label.
    pub label: Option<&'a str>,
}

/// Description used when beginning command recording.
#[derive(Clone, Copy, Debug, Default)]
pub struct CommandBufferBeginInfo {
    /// Recording and submission behavior flags.
    pub usage: CommandBufferUsage,
}

/// An explicit command pool.
pub struct CommandPool<B: Backend> {
    pub(crate) backend: Arc<B>,
    pub(crate) raw: Arc<B::CommandPool>,
    queue: QueueType,
    state: Rc<CommandPoolState>,
}

struct CommandPoolState {
    generation: Cell<u64>,
    pending_count: Cell<usize>,
    flags: CommandPoolFlags,
}

impl<B: Backend> fmt::Debug for CommandPool<B> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CommandPool")
            .field("queue", &self.queue)
            .finish_non_exhaustive()
    }
}

impl<B: Backend> CommandPool<B> {
    /// Returns the queue used by command buffers from this pool.
    #[must_use]
    pub const fn queue_type(&self) -> QueueType {
        self.queue
    }

    /// Allocates one command buffer from this pool.
    pub fn allocate(&self, level: CommandBufferLevel) -> Result<CommandBuffer<B>> {
        let raw = self
            .backend
            .allocate_command_buffer(self.raw.as_ref(), level)?;
        Ok(CommandBuffer {
            backend: Arc::clone(&self.backend),
            _pool: Arc::clone(&self.raw),
            raw,
            queue: self.queue,
            pool_state: Rc::clone(&self.state),
            observed_generation: Cell::new(self.state.generation.get()),
            state: Cell::new(CommandBufferState::Initial),
            usage: Cell::new(CommandBufferUsage::empty()),
            rendering: Cell::new(false),
            submitted_once: Cell::new(false),
            pending: RefCell::new(None),
        })
    }

    /// Resets all command buffers allocated from the pool.
    pub fn reset(&mut self) -> Result<()> {
        if self.state.pending_count.get() != 0 {
            return Err(Error::InvalidCommandBufferState {
                expected: "no pending command buffers",
                actual: "pending",
            });
        }
        self.backend.reset_command_pool(self.raw.as_ref())?;
        self.state
            .generation
            .set(self.state.generation.get().wrapping_add(1));
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandBufferState {
    Initial,
    Recording,
    Executable,
    Pending,
}

impl CommandBufferState {
    const fn name(self) -> &'static str {
        match self {
            Self::Initial => "initial",
            Self::Recording => "recording",
            Self::Executable => "executable",
            Self::Pending => "pending",
        }
    }
}

/// An owned command buffer allocated from an explicit pool.
pub struct CommandBuffer<B: Backend> {
    pub(crate) backend: Arc<B>,
    _pool: Arc<B::CommandPool>,
    pub(crate) raw: B::CommandBuffer,
    queue: QueueType,
    pool_state: Rc<CommandPoolState>,
    observed_generation: Cell<u64>,
    state: Cell<CommandBufferState>,
    usage: Cell<CommandBufferUsage>,
    rendering: Cell<bool>,
    submitted_once: Cell<bool>,
    pending: RefCell<Option<PendingSubmission>>,
}

struct PendingSubmission {
    fence: Rc<FenceTracking>,
    serial: u64,
}

impl<B: Backend> fmt::Debug for CommandBuffer<B> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CommandBuffer")
            .field("queue", &self.queue)
            .field("state", &self.current_state())
            .finish_non_exhaustive()
    }
}

impl<B: Backend> CommandBuffer<B> {
    /// Begins command recording.
    pub fn begin(&mut self, info: &CommandBufferBeginInfo) -> Result<()> {
        self.require_state(CommandBufferState::Initial, "initial")?;
        self.backend.begin_command_buffer(&mut self.raw, info)?;
        self.state.set(CommandBufferState::Recording);
        self.usage.set(info.usage);
        self.rendering.set(false);
        self.submitted_once.set(false);
        Ok(())
    }

    /// Finishes command recording and makes the buffer executable.
    pub fn end(&mut self) -> Result<()> {
        self.require_recording()?;
        self.require_outside_rendering("end command buffer")?;
        self.backend.end_command_buffer(&mut self.raw)?;
        self.state.set(CommandBufferState::Executable);
        Ok(())
    }

    /// Resets the command buffer to its initial state.
    pub fn reset(&mut self) -> Result<()> {
        self.synchronize_state();
        if !self
            .pool_state
            .flags
            .contains(CommandPoolFlags::RESET_COMMAND_BUFFER)
        {
            return Err(Error::invalid_descriptor(
                "command buffer reset",
                "the command pool must enable RESET_COMMAND_BUFFER",
            ));
        }
        if self.state.get() == CommandBufferState::Pending {
            return Err(Error::InvalidCommandBufferState {
                expected: "not pending",
                actual: "pending",
            });
        }
        self.backend.reset_command_buffer(&mut self.raw)?;
        self.reset_local_state();
        Ok(())
    }

    /// Records image and buffer synchronization barriers.
    pub fn barriers(
        &mut self,
        image_barriers: &[ImageBarrier<'_, B>],
        buffer_barriers: &[BufferBarrier<'_, B>],
    ) -> Result<()> {
        self.require_recording()?;
        self.require_outside_rendering("barriers")?;
        for barrier in image_barriers {
            self.ensure_backend(barrier.image.backend, "barriers")?;
        }
        for barrier in buffer_barriers {
            self.ensure_backend(&barrier.buffer.backend, "barriers")?;
        }
        self.backend
            .command_barriers(&mut self.raw, image_barriers, buffer_barriers)
    }

    /// Begins a dynamic rendering scope.
    pub fn begin_rendering(&mut self, info: &RenderingInfo<'_, B>) -> Result<()> {
        self.require_recording()?;
        self.require_outside_rendering("begin rendering")?;
        for attachment in info.color_attachments {
            self.ensure_backend(attachment.view.backend, "begin_rendering")?;
            if let Some(resolve) = attachment.resolve_target {
                self.ensure_backend(resolve.backend, "begin_rendering")?;
            }
        }
        if let Some(attachment) = &info.depth_stencil_attachment {
            self.ensure_backend(attachment.view.backend, "begin_rendering")?;
        }
        self.backend.command_begin_rendering(&mut self.raw, info)?;
        self.rendering.set(true);
        Ok(())
    }

    /// Ends the current dynamic rendering scope.
    pub fn end_rendering(&mut self) -> Result<()> {
        self.require_recording()?;
        self.require_inside_rendering("end rendering")?;
        self.backend.command_end_rendering(&mut self.raw)?;
        self.rendering.set(false);
        Ok(())
    }

    /// Copies bytes between buffers.
    pub fn copy_buffer(
        &mut self,
        source: &Buffer<B>,
        destination: &Buffer<B>,
        regions: &[BufferCopy],
    ) -> Result<()> {
        self.require_recording()?;
        self.require_outside_rendering("copy buffer")?;
        self.ensure_backend(&source.backend, "copy_buffer")?;
        self.ensure_backend(&destination.backend, "copy_buffer")?;
        self.backend.command_copy_buffer(
            &mut self.raw,
            source.raw.as_ref(),
            destination.raw.as_ref(),
            regions,
        )
    }

    /// Copies bytes from a buffer into an image.
    pub fn copy_buffer_to_image(
        &mut self,
        source: &Buffer<B>,
        destination: ImageRef<'_, B>,
        layout: ImageLayout,
        regions: &[BufferImageCopy],
    ) -> Result<()> {
        self.require_recording()?;
        self.require_outside_rendering("copy buffer to image")?;
        self.ensure_backend(&source.backend, "copy_buffer_to_image")?;
        self.ensure_backend(destination.backend, "copy_buffer_to_image")?;
        self.backend.command_copy_buffer_to_image(
            &mut self.raw,
            source.raw.as_ref(),
            destination.raw,
            layout,
            regions,
        )
    }

    /// Copies texels between images.
    pub fn copy_image(
        &mut self,
        source: ImageRef<'_, B>,
        source_layout: ImageLayout,
        destination: ImageRef<'_, B>,
        destination_layout: ImageLayout,
        regions: &[ImageCopy],
    ) -> Result<()> {
        self.require_recording()?;
        self.require_outside_rendering("copy image")?;
        self.ensure_backend(source.backend, "copy_image")?;
        self.ensure_backend(destination.backend, "copy_image")?;
        self.backend.command_copy_image(
            &mut self.raw,
            source.raw,
            source_layout,
            destination.raw,
            destination_layout,
            regions,
        )
    }

    /// Blits and optionally filters texels between images.
    pub fn blit_image(
        &mut self,
        source: ImageRef<'_, B>,
        source_layout: ImageLayout,
        destination: ImageRef<'_, B>,
        destination_layout: ImageLayout,
        regions: &[ImageBlit],
        filter: crate::Filter,
    ) -> Result<()> {
        self.require_recording()?;
        self.require_outside_rendering("blit image")?;
        self.ensure_backend(source.backend, "blit_image")?;
        self.ensure_backend(destination.backend, "blit_image")?;
        self.backend.command_blit_image(
            &mut self.raw,
            source.raw,
            source_layout,
            destination.raw,
            destination_layout,
            regions,
            filter,
        )
    }

    /// Sets the dynamic viewport.
    pub fn set_viewport(&mut self, viewport: Viewport) -> Result<()> {
        self.require_recording()?;
        self.backend.command_set_viewport(&mut self.raw, viewport)
    }

    /// Sets the dynamic scissor rectangle.
    pub fn set_scissor(&mut self, scissor: Rect2D) -> Result<()> {
        self.require_recording()?;
        self.backend.command_set_scissor(&mut self.raw, scissor)
    }

    /// Binds a graphics pipeline.
    pub fn bind_graphics_pipeline(&mut self, pipeline: &Pipeline<B>) -> Result<()> {
        self.require_recording()?;
        self.ensure_backend(&pipeline.backend, "bind_graphics_pipeline")?;
        self.backend
            .command_bind_graphics_pipeline(&mut self.raw, &pipeline.raw)
    }

    /// Binds resource groups to a graphics pipeline layout.
    pub fn bind_graphics_groups(
        &mut self,
        layout: &PipelineLayout<B>,
        first_group: u32,
        groups: &[&BindGroup<B>],
        dynamic_offsets: &[u32],
    ) -> Result<()> {
        self.require_recording()?;
        self.ensure_backend(&layout.backend, "bind_graphics_groups")?;
        for group in groups {
            self.ensure_backend(&group.backend, "bind_graphics_groups")?;
        }
        self.backend.command_bind_graphics_groups(
            &mut self.raw,
            layout.raw.as_ref(),
            first_group,
            groups,
            dynamic_offsets,
        )
    }

    /// Binds vertex buffers beginning at `first_binding`.
    pub fn bind_vertex_buffers(
        &mut self,
        first_binding: u32,
        buffers: &[VertexBufferBinding<'_, B>],
    ) -> Result<()> {
        self.require_recording()?;
        for binding in buffers {
            self.ensure_backend(&binding.buffer.backend, "bind_vertex_buffers")?;
        }
        self.backend
            .command_bind_vertex_buffers(&mut self.raw, first_binding, buffers)
    }

    /// Binds an index buffer.
    pub fn bind_index_buffer(
        &mut self,
        buffer: &Buffer<B>,
        offset: u64,
        format: IndexFormat,
    ) -> Result<()> {
        self.require_recording()?;
        self.ensure_backend(&buffer.backend, "bind_index_buffer")?;
        self.backend
            .command_bind_index_buffer(&mut self.raw, buffer.raw.as_ref(), offset, format)
    }

    /// Records an indexed draw.
    pub fn draw_indexed(&mut self, draw: DrawIndexed) -> Result<()> {
        self.require_recording()?;
        self.require_inside_rendering("draw indexed")?;
        self.backend.command_draw_indexed(&mut self.raw, draw)
    }

    /// Records a non-indexed draw.
    pub fn draw(&mut self, draw: Draw) -> Result<()> {
        self.require_recording()?;
        self.require_inside_rendering("draw")?;
        self.backend.command_draw(&mut self.raw, draw)
    }

    fn require_recording(&self) -> Result<()> {
        self.require_state(CommandBufferState::Recording, "recording")
    }

    fn require_state(&self, state: CommandBufferState, expected: &'static str) -> Result<()> {
        let actual = self.current_state();
        if actual == state {
            Ok(())
        } else {
            Err(Error::InvalidCommandBufferState {
                expected,
                actual: actual.name(),
            })
        }
    }

    fn require_inside_rendering(&self, operation: &'static str) -> Result<()> {
        if self.rendering.get() {
            Ok(())
        } else {
            Err(Error::InvalidCommandBufferState {
                expected: operation,
                actual: "recording outside rendering",
            })
        }
    }

    fn require_outside_rendering(&self, operation: &'static str) -> Result<()> {
        if self.rendering.get() {
            Err(Error::InvalidCommandBufferState {
                expected: operation,
                actual: "recording inside rendering",
            })
        } else {
            Ok(())
        }
    }

    fn current_state(&self) -> CommandBufferState {
        self.synchronize_state();
        self.state.get()
    }

    fn synchronize_state(&self) {
        let generation = self.pool_state.generation.get();
        if self.observed_generation.get() != generation {
            self.observed_generation.set(generation);
            self.reset_local_state();
            return;
        }

        let completed = self
            .pending
            .borrow()
            .as_ref()
            .is_some_and(|pending| pending.fence.completed_serial.get() >= pending.serial);
        if completed {
            self.pending.borrow_mut().take();
            self.state.set(CommandBufferState::Executable);
        }
    }

    fn reset_local_state(&self) {
        self.state.set(CommandBufferState::Initial);
        self.usage.set(CommandBufferUsage::empty());
        self.rendering.set(false);
        self.submitted_once.set(false);
        self.pending.borrow_mut().take();
    }

    fn ensure_backend(&self, backend: &Arc<B>, operation: &'static str) -> Result<()> {
        if Arc::ptr_eq(&self.backend, backend) {
            Ok(())
        } else {
            Err(Error::DeviceMismatch { operation })
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FenceState {
    Unsignaled,
    Pending,
    Signaled,
}

struct FenceTracking {
    state: Cell<FenceState>,
    submitted_serial: Cell<u64>,
    completed_serial: Cell<u64>,
    pending_pools: RefCell<Vec<Rc<CommandPoolState>>>,
}

impl FenceTracking {
    fn begin_submission(&self) -> u64 {
        let serial = self.submitted_serial.get().wrapping_add(1);
        self.submitted_serial.set(serial);
        self.state.set(FenceState::Pending);
        serial
    }

    fn complete(&self) {
        self.completed_serial.set(self.submitted_serial.get());
        self.state.set(FenceState::Signaled);
        for pool in self.pending_pools.borrow_mut().drain(..) {
            pool.pending_count
                .set(pool.pending_count.get().saturating_sub(1));
        }
    }
}

/// An owned host-visible completion fence.
pub struct Fence<B: Backend> {
    pub(crate) backend: Arc<B>,
    pub(crate) raw: B::Fence,
    tracking: Rc<FenceTracking>,
}

impl<B: Backend> Fence<B> {
    fn new(backend: Arc<B>, raw: B::Fence, signaled: bool) -> Self {
        Self {
            backend,
            raw,
            tracking: Rc::new(FenceTracking {
                state: Cell::new(if signaled {
                    FenceState::Signaled
                } else {
                    FenceState::Unsignaled
                }),
                submitted_serial: Cell::new(0),
                completed_serial: Cell::new(0),
                pending_pools: RefCell::new(Vec::new()),
            }),
        }
    }
}

impl<B: Backend> fmt::Debug for Fence<B> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Fence")
            .field("state", &self.tracking.state.get())
            .finish_non_exhaustive()
    }
}

impl<B: Backend> Device<B> {
    /// Creates an explicit command pool.
    pub fn create_command_pool(&self, info: &CommandPoolCreateInfo<'_>) -> Result<CommandPool<B>> {
        Ok(CommandPool {
            backend: Arc::clone(&self.backend),
            raw: Arc::new(self.backend.create_command_pool(info)?),
            queue: info.queue,
            state: Rc::new(CommandPoolState {
                generation: Cell::new(0),
                pending_count: Cell::new(0),
                flags: info.flags,
            }),
        })
    }

    /// Creates a fence in the requested initial state.
    pub fn create_fence(&self, signaled: bool) -> Result<Fence<B>> {
        Ok(Fence::new(
            Arc::clone(&self.backend),
            self.backend.create_fence(signaled)?,
            signaled,
        ))
    }

    /// Waits for a fence and releases command buffers tracked by its submission.
    pub fn wait_for_fence(&self, fence: &Fence<B>, timeout_ns: u64) -> Result<()> {
        self.ensure_resource(&fence.backend, "wait_for_fence")?;
        self.backend.wait_for_fence(&fence.raw, timeout_ns)?;
        fence.tracking.complete();
        Ok(())
    }

    /// Resets a fence to the unsignaled state.
    pub fn reset_fence(&self, fence: &mut Fence<B>) -> Result<()> {
        self.ensure_resource(&fence.backend, "reset_fence")?;
        if fence.tracking.state.get() == FenceState::Pending {
            return Err(Error::invalid_descriptor(
                "fence reset",
                "a pending fence must be waited before reset",
            ));
        }
        self.backend.reset_fence(&mut fence.raw)?;
        fence.tracking.state.set(FenceState::Unsignaled);
        Ok(())
    }

    /// Creates a binary or timeline semaphore.
    pub fn create_semaphore(&self, kind: SemaphoreKind) -> Result<Semaphore<B>> {
        Ok(Semaphore::new(
            Arc::clone(&self.backend),
            self.backend.create_semaphore(kind)?,
            kind,
        ))
    }

    /// Waits until a timeline semaphore reaches `value`.
    pub fn wait_for_semaphore(
        &self,
        semaphore: &Semaphore<B>,
        value: u64,
        timeout_ns: u64,
    ) -> Result<()> {
        self.ensure_resource(&semaphore.backend, "wait_for_semaphore")?;
        if semaphore.kind() == SemaphoreKind::Binary {
            return Err(Error::invalid_descriptor(
                "semaphore wait",
                "host waits require a timeline semaphore",
            ));
        }
        self.backend
            .wait_for_semaphore(&semaphore.raw, value, timeout_ns)
    }

    /// Returns a timeline semaphore's current value.
    pub fn semaphore_value(&self, semaphore: &Semaphore<B>) -> Result<u64> {
        self.ensure_resource(&semaphore.backend, "semaphore_value")?;
        if semaphore.kind() == SemaphoreKind::Binary {
            return Err(Error::invalid_descriptor(
                "semaphore value",
                "binary semaphores do not have counter values",
            ));
        }
        self.backend.semaphore_value(&semaphore.raw)
    }

    /// Submits executable command buffers and tracks them until `fence` is waited.
    pub fn submit(
        &self,
        queue: QueueType,
        info: &SubmitInfo<'_, B>,
        fence: &Fence<B>,
    ) -> Result<()> {
        for wait in info.waits {
            self.ensure_resource(&wait.semaphore.backend, "submit")?;
            wait.validate()?;
        }
        for (index, command_buffer) in info.command_buffers.iter().enumerate() {
            if info.command_buffers[..index]
                .iter()
                .any(|previous| std::ptr::eq(*previous, *command_buffer))
            {
                return Err(Error::invalid_descriptor(
                    "queue submission",
                    "a command buffer may appear only once in a submission",
                ));
            }
            self.ensure_resource(&command_buffer.backend, "submit")?;
            command_buffer.require_state(CommandBufferState::Executable, "executable")?;
            if command_buffer
                .usage
                .get()
                .contains(CommandBufferUsage::ONE_TIME_SUBMIT)
                && command_buffer.submitted_once.get()
            {
                return Err(Error::invalid_descriptor(
                    "queue submission",
                    "a ONE_TIME_SUBMIT command buffer must be reset before reuse",
                ));
            }
            if command_buffer.queue != queue {
                return Err(Error::invalid_descriptor(
                    "queue submission",
                    "command buffer was allocated for a different queue type",
                ));
            }
        }
        for signal in info.signals {
            self.ensure_resource(&signal.semaphore.backend, "submit")?;
            signal.validate()?;
        }
        self.ensure_resource(&fence.backend, "submit")?;
        if fence.tracking.state.get() != FenceState::Unsignaled {
            return Err(Error::invalid_descriptor(
                "queue submission",
                "the submission fence must be unsignaled and not pending",
            ));
        }
        self.backend.submit(queue, info, fence)?;

        let serial = fence.tracking.begin_submission();
        for command_buffer in info.command_buffers {
            let pending_count = command_buffer.pool_state.pending_count.get();
            command_buffer
                .pool_state
                .pending_count
                .set(pending_count + 1);
            fence
                .tracking
                .pending_pools
                .borrow_mut()
                .push(Rc::clone(&command_buffer.pool_state));
            command_buffer.state.set(CommandBufferState::Pending);
            command_buffer.submitted_once.set(true);
            command_buffer
                .pending
                .borrow_mut()
                .replace(PendingSubmission {
                    fence: Rc::clone(&fence.tracking),
                    serial,
                });
        }
        Ok(())
    }
}

/// Offset in three-dimensional image coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Offset3D {
    /// X coordinate.
    pub x: i32,
    /// Y coordinate.
    pub y: i32,
    /// Z coordinate.
    pub z: i32,
}

/// Two-dimensional render rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rect2D {
    /// X offset in pixels.
    pub x: i32,
    /// Y offset in pixels.
    pub y: i32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

/// Dynamic viewport state.
#[derive(Clone, Copy, Debug)]
pub struct Viewport {
    /// Left coordinate.
    pub x: f32,
    /// Top coordinate.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
    /// Minimum depth value.
    pub min_depth: f32,
    /// Maximum depth value.
    pub max_depth: f32,
}

/// One buffer-to-buffer copy region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BufferCopy {
    /// Source byte offset.
    pub source_offset: u64,
    /// Destination byte offset.
    pub destination_offset: u64,
    /// Number of bytes copied.
    pub size: u64,
}

/// Image layers addressed by a copy operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ImageSubresourceLayers {
    /// Selected aspect.
    pub aspects: crate::ImageAspects,
    /// Selected mip level.
    pub mip_level: u32,
    /// First array layer.
    pub base_array_layer: u32,
    /// Number of array layers.
    pub array_layer_count: u32,
}

/// One buffer-to-image copy region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BufferImageCopy {
    /// Source byte offset.
    pub buffer_offset: u64,
    /// Image subresources copied.
    pub image_subresource: ImageSubresourceLayers,
    /// Destination image offset.
    pub image_offset: Offset3D,
    /// Copied image extent.
    pub image_extent: crate::Extent3D,
}

/// One image-to-image copy region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ImageCopy {
    /// Source image subresources.
    pub source_subresource: ImageSubresourceLayers,
    /// Source image offset.
    pub source_offset: Offset3D,
    /// Destination image subresources.
    pub destination_subresource: ImageSubresourceLayers,
    /// Destination image offset.
    pub destination_offset: Offset3D,
    /// Copied extent.
    pub extent: crate::Extent3D,
}

/// One filtered image blit region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ImageBlit {
    /// Source image subresources.
    pub source_subresource: ImageSubresourceLayers,
    /// Opposite source-region corners.
    pub source_offsets: [Offset3D; 2],
    /// Destination image subresources.
    pub destination_subresource: ImageSubresourceLayers,
    /// Opposite destination-region corners.
    pub destination_offsets: [Offset3D; 2],
}

/// An image transition and memory dependency.
#[derive(Clone, Copy, Debug)]
pub struct ImageBarrier<'a, B: Backend> {
    /// Image being synchronized.
    pub image: ImageRef<'a, B>,
    /// Selected image subresources.
    pub range: ImageSubresourceRange,
    /// State before the barrier.
    pub before: ResourceState,
    /// State after the barrier.
    pub after: ResourceState,
}

/// A buffer memory dependency.
#[derive(Clone, Copy, Debug)]
pub struct BufferBarrier<'a, B: Backend> {
    /// Buffer being synchronized.
    pub buffer: &'a Buffer<B>,
    /// First synchronized byte.
    pub offset: u64,
    /// Number of synchronized bytes.
    pub size: u64,
    /// Source pipeline stages.
    pub source_stages: PipelineStages,
    /// Source accesses.
    pub source_access: AccessTypes,
    /// Destination pipeline stages.
    pub destination_stages: PipelineStages,
    /// Destination accesses.
    pub destination_access: AccessTypes,
}

/// Attachment load behavior.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LoadOperation<T> {
    /// Preserve existing contents.
    Load,
    /// Clear to the supplied value.
    Clear(T),
    /// Existing contents may be discarded.
    DontCare,
}

/// Attachment store behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StoreOperation {
    /// Preserve rendered contents.
    Store,
    /// Rendered contents may be discarded.
    DontCare,
}

/// Depth and stencil clear value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepthStencilValue {
    /// Depth clear value.
    pub depth: f32,
    /// Stencil clear value.
    pub stencil: u32,
}

/// One color rendering attachment.
#[derive(Clone, Copy, Debug)]
pub struct ColorAttachment<'a, B: Backend> {
    /// Attachment image view.
    pub view: ImageViewRef<'a, B>,
    /// Optional multisample resolve target.
    pub resolve_target: Option<ImageViewRef<'a, B>>,
    /// Load behavior and clear color.
    pub load: LoadOperation<[f32; 4]>,
    /// Store behavior.
    pub store: StoreOperation,
}

/// One depth/stencil rendering attachment.
#[derive(Clone, Copy, Debug)]
pub struct DepthStencilAttachment<'a, B: Backend> {
    /// Attachment image view.
    pub view: ImageViewRef<'a, B>,
    /// Load behavior and clear value.
    pub load: LoadOperation<DepthStencilValue>,
    /// Store behavior.
    pub store: StoreOperation,
}

/// Dynamic rendering scope description.
#[derive(Clone, Copy, Debug)]
pub struct RenderingInfo<'a, B: Backend> {
    /// Render area.
    pub render_area: Rect2D,
    /// Number of rendered array layers.
    pub layer_count: u32,
    /// Color attachments in shader output order.
    pub color_attachments: &'a [ColorAttachment<'a, B>],
    /// Optional depth/stencil attachment.
    pub depth_stencil_attachment: Option<DepthStencilAttachment<'a, B>>,
}

/// One bound vertex buffer and byte offset.
#[derive(Clone, Copy, Debug)]
pub struct VertexBufferBinding<'a, B: Backend> {
    /// Bound buffer.
    pub buffer: &'a Buffer<B>,
    /// First vertex-data byte.
    pub offset: u64,
}

/// Index element format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IndexFormat {
    /// Unsigned 16-bit index.
    Uint16,
    /// Unsigned 32-bit index.
    Uint32,
}

/// Parameters for one indexed draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DrawIndexed {
    /// First index element.
    pub first_index: u32,
    /// Number of index elements.
    pub index_count: u32,
    /// Value added to each index before vertex lookup.
    pub vertex_offset: i32,
    /// First instance.
    pub first_instance: u32,
    /// Number of instances.
    pub instance_count: u32,
}

/// Parameters for one non-indexed draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Draw {
    /// First vertex.
    pub first_vertex: u32,
    /// Number of vertices.
    pub vertex_count: u32,
    /// First instance.
    pub first_instance: u32,
    /// Number of instances.
    pub instance_count: u32,
}

/// A semaphore dependency before queue work begins.
#[derive(Clone, Copy, Debug)]
pub struct SemaphoreWait<'a, B: Backend> {
    /// Semaphore to wait on.
    pub semaphore: &'a Semaphore<B>,
    /// Timeline value, or `None` for a binary semaphore.
    pub value: Option<u64>,
    /// Pipeline stages blocked by the wait.
    pub stages: PipelineStages,
}

impl<B: Backend> SemaphoreWait<'_, B> {
    fn validate(&self) -> Result<()> {
        if self.stages.is_empty() {
            return Err(Error::invalid_descriptor(
                "semaphore wait",
                "at least one destination stage must be selected",
            ));
        }
        validate_semaphore_value(self.semaphore.kind(), self.value)
    }
}

/// A semaphore signaled after queue work completes.
#[derive(Clone, Copy, Debug)]
pub struct SemaphoreSignal<'a, B: Backend> {
    /// Semaphore to signal.
    pub semaphore: &'a Semaphore<B>,
    /// Timeline value, or `None` for a binary semaphore.
    pub value: Option<u64>,
}

impl<B: Backend> SemaphoreSignal<'_, B> {
    fn validate(&self) -> Result<()> {
        validate_semaphore_value(self.semaphore.kind(), self.value)
    }
}

fn validate_semaphore_value(kind: SemaphoreKind, value: Option<u64>) -> Result<()> {
    let valid = matches!(
        (kind, value),
        (SemaphoreKind::Binary, None) | (SemaphoreKind::Timeline { .. }, Some(_))
    );
    if valid {
        Ok(())
    } else {
        Err(Error::invalid_descriptor(
            "semaphore operation",
            "binary semaphores omit values and timeline semaphores require them",
        ))
    }
}

/// One queue submission batch.
#[derive(Clone, Copy, Debug)]
pub struct SubmitInfo<'a, B: Backend> {
    /// Semaphores waited before execution.
    pub waits: &'a [SemaphoreWait<'a, B>],
    /// Executable command buffers in submission order.
    pub command_buffers: &'a [&'a CommandBuffer<B>],
    /// Semaphores signaled after execution.
    pub signals: &'a [SemaphoreSignal<'a, B>],
}
