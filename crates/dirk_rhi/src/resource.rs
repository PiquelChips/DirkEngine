//! Devices and owned GPU resources.

use std::{fmt, sync::Arc};

use crate::{
    Backend, BindGroupLayoutCreateInfo, BufferCreateInfo, BufferUsage, Error, Extent3D, Format,
    ImageCreateInfo, ImageSubresourceRange, ImageViewCreateInfo, Result, SamplerCreateInfo,
    ShaderModuleCreateInfo,
};

/// A logical device implemented by a concrete backend.
pub struct Device<B: Backend> {
    pub(crate) backend: Arc<B>,
}

impl<B: Backend> Clone for Device<B> {
    fn clone(&self) -> Self {
        Self {
            backend: Arc::clone(&self.backend),
        }
    }
}

impl<B: Backend> fmt::Debug for Device<B> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("Device").finish_non_exhaustive()
    }
}

impl<B: Backend> Device<B> {
    /// Wraps an initialized backend device.
    #[must_use]
    pub fn new(backend: B) -> Self {
        Self {
            backend: Arc::new(backend),
        }
    }

    /// Waits until all device work has completed.
    pub fn wait_idle(&self) -> Result<()> {
        self.backend.wait_idle()
    }

    /// Flushes backend-managed deferred destruction at a caller-defined safe point.
    pub fn flush(&self) {
        self.backend.flush();
    }

    /// Creates an owned buffer.
    pub fn create_buffer(&self, info: &BufferCreateInfo<'_>) -> Result<Buffer<B>> {
        info.validate()?;
        let raw = Arc::new(self.backend.create_buffer(info)?);
        Ok(Buffer {
            backend: Arc::clone(&self.backend),
            raw,
            size: info.size,
            usage: info.usage,
        })
    }

    /// Writes bytes into host-writable buffer memory.
    pub fn write_buffer(&self, buffer: &Buffer<B>, offset: u64, data: &[u8]) -> Result<()> {
        self.ensure_resource(&buffer.backend, "write_buffer")?;
        let end = offset
            .checked_add(data.len() as u64)
            .ok_or_else(|| Error::invalid_descriptor("buffer write", "byte range overflows"))?;
        if end > buffer.size {
            return Err(Error::invalid_descriptor(
                "buffer write",
                "byte range exceeds the buffer size",
            ));
        }
        self.backend.write_buffer(buffer.raw.as_ref(), offset, data)
    }

    /// Creates an owned image.
    pub fn create_image(&self, info: &ImageCreateInfo<'_>) -> Result<Image<B>> {
        info.validate()?;
        let raw = Arc::new(self.backend.create_image(info)?);
        Ok(Image {
            backend: Arc::clone(&self.backend),
            raw,
            extent: info.extent,
            format: info.format,
            mip_levels: info.mip_levels,
            array_layers: info.array_layers,
        })
    }

    /// Creates an owned view into an image.
    pub fn create_image_view(
        &self,
        image: &Image<B>,
        info: &ImageViewCreateInfo<'_>,
    ) -> Result<ImageView<B>> {
        self.ensure_resource(&image.backend, "create_image_view")?;
        info.validate()?;
        let raw = self.backend.create_image_view(image.raw.as_ref(), info)?;
        Ok(ImageView {
            backend: Arc::clone(&self.backend),
            inner: Arc::new(ImageViewInner {
                raw,
                _image: Arc::clone(&image.raw),
            }),
            format: info.format.unwrap_or(image.format),
            range: info.range,
        })
    }

    /// Creates an owned sampler.
    pub fn create_sampler(&self, info: &SamplerCreateInfo<'_>) -> Result<Sampler<B>> {
        if info.lod_min > info.lod_max {
            return Err(Error::invalid_descriptor(
                "sampler",
                "minimum LOD must not exceed maximum LOD",
            ));
        }
        Ok(Sampler::new(
            Arc::clone(&self.backend),
            Arc::new(self.backend.create_sampler(info)?),
        ))
    }

    /// Creates an owned shader module.
    pub fn create_shader_module(
        &self,
        info: &ShaderModuleCreateInfo<'_>,
    ) -> Result<ShaderModule<B>> {
        info.validate()?;
        Ok(ShaderModule::new(
            Arc::clone(&self.backend),
            Arc::new(self.backend.create_shader_module(info)?),
        ))
    }

    /// Creates an owned bind-group layout.
    pub fn create_bind_group_layout(
        &self,
        info: &BindGroupLayoutCreateInfo<'_>,
    ) -> Result<BindGroupLayout<B>> {
        info.validate()?;
        Ok(BindGroupLayout::new(
            Arc::clone(&self.backend),
            Arc::new(self.backend.create_bind_group_layout(info)?),
        ))
    }

    /// Creates an owned bind group.
    pub fn create_bind_group(&self, info: &BindGroupCreateInfo<'_, B>) -> Result<BindGroup<B>> {
        self.ensure_resource(&info.layout.backend, "create_bind_group")?;
        for entry in info.entries {
            match entry.resource {
                BindingResource::Buffer { buffer, .. } => {
                    self.ensure_resource(&buffer.backend, "create_bind_group")?;
                }
                BindingResource::ImageView(view) => {
                    self.ensure_resource(&view.backend, "create_bind_group")?;
                }
                BindingResource::Sampler(sampler) => {
                    self.ensure_resource(&sampler.backend, "create_bind_group")?;
                }
                BindingResource::CombinedImageSampler { view, sampler } => {
                    self.ensure_resource(&view.backend, "create_bind_group")?;
                    self.ensure_resource(&sampler.backend, "create_bind_group")?;
                }
            }
        }
        let dependencies = info
            .entries
            .iter()
            .map(|entry| BindingDependency::from(entry.resource))
            .collect();
        Ok(BindGroup {
            backend: Arc::clone(&self.backend),
            raw: self.backend.create_bind_group(info)?,
            _layout: Arc::clone(&info.layout.raw),
            _resources: dependencies,
        })
    }

    /// Creates an owned pipeline layout.
    pub fn create_pipeline_layout(
        &self,
        info: &PipelineLayoutCreateInfo<'_, B>,
    ) -> Result<PipelineLayout<B>> {
        for layout in info.bind_group_layouts {
            self.ensure_resource(&layout.backend, "create_pipeline_layout")?;
        }
        Ok(PipelineLayout {
            backend: Arc::clone(&self.backend),
            raw: Arc::new(self.backend.create_pipeline_layout(info)?),
            bind_group_layouts: info
                .bind_group_layouts
                .iter()
                .map(|layout| Arc::clone(&layout.raw))
                .collect(),
        })
    }

    /// Creates an owned graphics pipeline.
    pub fn create_graphics_pipeline(
        &self,
        info: &GraphicsPipelineCreateInfo<'_, B>,
    ) -> Result<Pipeline<B>> {
        self.ensure_resource(&info.layout.backend, "create_graphics_pipeline")?;
        self.ensure_resource(&info.vertex.module.backend, "create_graphics_pipeline")?;
        if let Some(fragment) = &info.fragment {
            self.ensure_resource(&fragment.module.backend, "create_graphics_pipeline")?;
        }
        if info.vertex.entry_point.is_empty()
            || info
                .fragment
                .as_ref()
                .is_some_and(|fragment| fragment.entry_point.is_empty())
        {
            return Err(Error::invalid_descriptor(
                "graphics pipeline",
                "shader entry points must not be empty",
            ));
        }
        let mut shaders = vec![Arc::clone(&info.vertex.module.raw)];
        if let Some(fragment) = &info.fragment {
            shaders.push(Arc::clone(&fragment.module.raw));
        }
        Ok(Pipeline {
            backend: Arc::clone(&self.backend),
            raw: self.backend.create_graphics_pipeline(info)?,
            _layout: Arc::clone(&info.layout.raw),
            _bind_group_layouts: info.layout.bind_group_layouts.clone(),
            _shaders: shaders,
        })
    }

    pub(crate) fn ensure_resource(&self, backend: &Arc<B>, operation: &'static str) -> Result<()> {
        if Arc::ptr_eq(&self.backend, backend) {
            Ok(())
        } else {
            Err(Error::DeviceMismatch { operation })
        }
    }
}

/// An owned GPU buffer.
pub struct Buffer<B: Backend> {
    pub(crate) backend: Arc<B>,
    pub(crate) raw: Arc<B::Buffer>,
    size: u64,
    usage: BufferUsage,
}

impl<B: Backend> Buffer<B> {
    /// Returns the buffer size in bytes.
    #[must_use]
    pub const fn size(&self) -> u64 {
        self.size
    }

    /// Returns the declared buffer usage.
    #[must_use]
    pub const fn usage(&self) -> BufferUsage {
        self.usage
    }
}

impl<B: Backend> fmt::Debug for Buffer<B> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Buffer")
            .field("size", &self.size)
            .field("usage", &self.usage)
            .finish_non_exhaustive()
    }
}

/// An owned GPU image.
pub struct Image<B: Backend> {
    pub(crate) backend: Arc<B>,
    pub(crate) raw: Arc<B::Image>,
    extent: Extent3D,
    format: Format,
    mip_levels: u32,
    array_layers: u32,
}

impl<B: Backend> Image<B> {
    /// Returns a non-owning image reference for command recording.
    #[must_use]
    pub fn as_ref(&self) -> ImageRef<'_, B> {
        ImageRef {
            backend: &self.backend,
            raw: self.raw.as_ref(),
        }
    }

    /// Returns the image extent.
    #[must_use]
    pub const fn extent(&self) -> Extent3D {
        self.extent
    }

    /// Returns the image format.
    #[must_use]
    pub const fn format(&self) -> Format {
        self.format
    }

    /// Returns the mip-level count.
    #[must_use]
    pub const fn mip_levels(&self) -> u32 {
        self.mip_levels
    }

    /// Returns the array-layer count.
    #[must_use]
    pub const fn array_layers(&self) -> u32 {
        self.array_layers
    }
}

impl<B: Backend> fmt::Debug for Image<B> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Image")
            .field("extent", &self.extent)
            .field("format", &self.format)
            .finish_non_exhaustive()
    }
}

/// A borrowed image accepted by command recording methods.
pub struct ImageRef<'a, B: Backend> {
    pub(crate) backend: &'a Arc<B>,
    pub(crate) raw: &'a B::Image,
}

impl<B: Backend> Copy for ImageRef<'_, B> {}

impl<B: Backend> Clone for ImageRef<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> fmt::Debug for ImageRef<'_, B> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("ImageRef").finish_non_exhaustive()
    }
}

/// An owned view into an image.
pub struct ImageView<B: Backend> {
    pub(crate) backend: Arc<B>,
    inner: Arc<ImageViewInner<B>>,
    format: Format,
    range: ImageSubresourceRange,
}

struct ImageViewInner<B: Backend> {
    raw: B::ImageView,
    _image: Arc<B::Image>,
}

impl<B: Backend> ImageView<B> {
    /// Returns a non-owning image-view reference for command recording.
    #[must_use]
    pub fn as_ref(&self) -> ImageViewRef<'_, B> {
        ImageViewRef {
            backend: &self.backend,
            raw: &self.inner.raw,
        }
    }

    /// Returns the view format.
    #[must_use]
    pub const fn format(&self) -> Format {
        self.format
    }

    /// Returns the selected image subresources.
    #[must_use]
    pub const fn range(&self) -> ImageSubresourceRange {
        self.range
    }
}

impl<B: Backend> fmt::Debug for ImageView<B> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ImageView")
            .field("format", &self.format)
            .field("range", &self.range)
            .finish_non_exhaustive()
    }
}

/// A borrowed image view accepted by rendering descriptors.
pub struct ImageViewRef<'a, B: Backend> {
    pub(crate) backend: &'a Arc<B>,
    #[allow(dead_code)] // Read by concrete backend implementations.
    pub(crate) raw: &'a B::ImageView,
}

impl<B: Backend> Copy for ImageViewRef<'_, B> {}

impl<B: Backend> Clone for ImageViewRef<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> fmt::Debug for ImageViewRef<'_, B> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ImageViewRef")
            .finish_non_exhaustive()
    }
}

macro_rules! shared_opaque_resource {
    ($name:ident, $associated:ident, $docs:literal) => {
        #[doc = $docs]
        pub struct $name<B: Backend> {
            pub(crate) backend: Arc<B>,
            #[allow(dead_code)] // Read by concrete backend implementations.
            pub(crate) raw: Arc<B::$associated>,
        }

        impl<B: Backend> $name<B> {
            pub(crate) fn new(backend: Arc<B>, raw: Arc<B::$associated>) -> Self {
                Self { backend, raw }
            }
        }

        impl<B: Backend> fmt::Debug for $name<B> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_struct(stringify!($name))
                    .finish_non_exhaustive()
            }
        }
    };
}

shared_opaque_resource!(Sampler, Sampler, "An owned texture sampler.");
shared_opaque_resource!(ShaderModule, ShaderModule, "An owned shader module.");
shared_opaque_resource!(
    BindGroupLayout,
    BindGroupLayout,
    "An owned bind-group layout."
);
/// An owned group of bound resources.
pub struct BindGroup<B: Backend> {
    pub(crate) backend: Arc<B>,
    #[allow(dead_code)] // Read by concrete backend implementations.
    pub(crate) raw: B::BindGroup,
    _layout: Arc<B::BindGroupLayout>,
    _resources: Vec<BindingDependency<B>>,
}

/// An owned pipeline resource layout.
pub struct PipelineLayout<B: Backend> {
    pub(crate) backend: Arc<B>,
    pub(crate) raw: Arc<B::PipelineLayout>,
    bind_group_layouts: Vec<Arc<B::BindGroupLayout>>,
}

/// An owned graphics pipeline.
pub struct Pipeline<B: Backend> {
    pub(crate) backend: Arc<B>,
    pub(crate) raw: B::Pipeline,
    _layout: Arc<B::PipelineLayout>,
    _bind_group_layouts: Vec<Arc<B::BindGroupLayout>>,
    _shaders: Vec<Arc<B::ShaderModule>>,
}

macro_rules! debug_opaque_resource {
    ($($name:ident),+ $(,)?) => {$ (
        impl<B: Backend> fmt::Debug for $name<B> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_struct(stringify!($name))
                    .finish_non_exhaustive()
            }
        }
    )+ };
}

debug_opaque_resource!(BindGroup, PipelineLayout, Pipeline);

/// Kind and initial value of a semaphore.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SemaphoreKind {
    /// A binary semaphore.
    Binary,
    /// A monotonically increasing timeline semaphore.
    Timeline {
        /// Initial counter value.
        initial_value: u64,
    },
}

/// An owned GPU semaphore.
pub struct Semaphore<B: Backend> {
    pub(crate) backend: Arc<B>,
    pub(crate) raw: B::Semaphore,
    kind: SemaphoreKind,
}

impl<B: Backend> Semaphore<B> {
    pub(crate) fn new(backend: Arc<B>, raw: B::Semaphore, kind: SemaphoreKind) -> Self {
        Self { backend, raw, kind }
    }

    /// Returns the semaphore kind.
    #[must_use]
    pub const fn kind(&self) -> SemaphoreKind {
        self.kind
    }
}

impl<B: Backend> fmt::Debug for Semaphore<B> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Semaphore")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

/// Resource supplied to one bind-group entry.
#[derive(Debug)]
pub enum BindingResource<'a, B: Backend> {
    /// A byte range within a buffer.
    Buffer {
        /// Bound buffer.
        buffer: &'a Buffer<B>,
        /// First bound byte.
        offset: u64,
        /// Number of bound bytes.
        size: u64,
    },
    /// An image view.
    ImageView(&'a ImageView<B>),
    /// A sampler.
    Sampler(&'a Sampler<B>),
    /// An image view and sampler occupying one combined binding.
    CombinedImageSampler {
        /// Sampled image view.
        view: &'a ImageView<B>,
        /// Sampler used to read the image.
        sampler: &'a Sampler<B>,
    },
}

impl<B: Backend> Copy for BindingResource<'_, B> {}

impl<B: Backend> Clone for BindingResource<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

#[allow(dead_code)] // Ownership is the purpose of each stored handle.
enum BindingDependency<B: Backend> {
    Buffer(Arc<B::Buffer>),
    ImageView(Arc<ImageViewInner<B>>),
    Sampler(Arc<B::Sampler>),
    CombinedImageSampler(Arc<ImageViewInner<B>>, Arc<B::Sampler>),
}

impl<B: Backend> From<BindingResource<'_, B>> for BindingDependency<B> {
    fn from(resource: BindingResource<'_, B>) -> Self {
        match resource {
            BindingResource::Buffer { buffer, .. } => Self::Buffer(Arc::clone(&buffer.raw)),
            BindingResource::ImageView(view) => Self::ImageView(Arc::clone(&view.inner)),
            BindingResource::Sampler(sampler) => Self::Sampler(Arc::clone(&sampler.raw)),
            BindingResource::CombinedImageSampler { view, sampler } => {
                Self::CombinedImageSampler(Arc::clone(&view.inner), Arc::clone(&sampler.raw))
            }
        }
    }
}

/// One resource written into a bind group.
#[derive(Clone, Copy, Debug)]
pub struct BindGroupEntry<'a, B: Backend> {
    /// Binding number in the layout.
    pub binding: u32,
    /// First array element written by this entry.
    pub array_element: u32,
    /// Bound resource.
    pub resource: BindingResource<'a, B>,
}

/// Description of an owned bind group.
#[derive(Clone, Copy, Debug)]
pub struct BindGroupCreateInfo<'a, B: Backend> {
    /// Layout implemented by the bind group.
    pub layout: &'a BindGroupLayout<B>,
    /// Resources written into the group.
    pub entries: &'a [BindGroupEntry<'a, B>],
    /// Optional diagnostic label.
    pub label: Option<&'a str>,
}

/// Description of an owned pipeline layout.
#[derive(Clone, Copy, Debug)]
pub struct PipelineLayoutCreateInfo<'a, B: Backend> {
    /// Bind-group layouts in shader group order.
    pub bind_group_layouts: &'a [&'a BindGroupLayout<B>],
    /// Optional diagnostic label.
    pub label: Option<&'a str>,
}

/// Vertex shader state for a graphics pipeline.
#[derive(Clone, Copy, Debug)]
pub struct VertexState<'a, B: Backend> {
    /// Vertex shader module.
    pub module: &'a ShaderModule<B>,
    /// Shader entry point.
    pub entry_point: &'a str,
    /// Vertex-buffer layouts by binding number.
    pub buffers: &'a [crate::VertexBufferLayout<'a>],
}

/// Fragment shader state for a graphics pipeline.
#[derive(Clone, Copy, Debug)]
pub struct FragmentState<'a, B: Backend> {
    /// Fragment shader module.
    pub module: &'a ShaderModule<B>,
    /// Shader entry point.
    pub entry_point: &'a str,
    /// Color attachment output states.
    pub targets: &'a [crate::ColorTargetState],
}

/// Description of an owned graphics pipeline.
#[derive(Clone, Copy, Debug)]
pub struct GraphicsPipelineCreateInfo<'a, B: Backend> {
    /// Pipeline resource layout.
    pub layout: &'a PipelineLayout<B>,
    /// Vertex shader and input state.
    pub vertex: VertexState<'a, B>,
    /// Optional fragment shader and output state.
    pub fragment: Option<FragmentState<'a, B>>,
    /// Primitive assembly and rasterization state.
    pub primitive: crate::PrimitiveState,
    /// Optional depth state.
    pub depth: Option<crate::DepthState>,
    /// Rasterization sample count.
    pub samples: crate::SampleCount,
    /// Optional diagnostic label.
    pub label: Option<&'a str>,
}
