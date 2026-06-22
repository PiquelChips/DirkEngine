use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use ash::vk;
use dirk_engine::editor::{
    EditorServices, EditorWindowDescriptor, EditorWindowId, VIEWPORT_CATEGORY,
};
use dirk_player::PlayerId;
use parking_lot::Mutex;

use crate::{
    MAX_FRAMES_IN_FLIGHT, Result,
    egui_integration::EguiState,
    resources::{
        descriptors::{DescriptorAllocator, DescriptorSet, DescriptorWriter, layouts::SetLayout},
        device::{Garbage, RenderDevice},
    },
    viewport::Viewport,
};

struct ViewportTextureBinding {
    texture_id: egui::TextureId,
    /// Just to keep the descriptor set alive as egui holds the raw set.
    _descriptor_set: DescriptorSet<ViewportTextureSet>,
}

struct RetiredViewportTextureBinding {
    binding: ViewportTextureBinding,
    frames_remaining: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ViewportEditorEntry {
    texture_id: egui::TextureId,
    extent: vk::Extent2D,
    ready: bool,
    requested_extent: Option<vk::Extent2D>,
}

struct ViewportTextureSet;

impl SetLayout for ViewportTextureSet {
    const BINDINGS: &'static [vk::DescriptorSetLayoutBinding<'static>] =
        &[vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            p_immutable_samplers: ::core::ptr::null(),
            _marker: PhantomData,
        }];
}

pub struct ViewportEditor {
    state: Arc<Mutex<ViewportEditorState>>,
    windows: HashMap<PlayerId, EditorWindowId>,
    textures: HashMap<PlayerId, ViewportTextureBinding>,
    retired_textures: Vec<RetiredViewportTextureBinding>,
    descriptor_allocator: DescriptorAllocator<ViewportTextureSet>,
    sampler: vk::Sampler,
    device: RenderDevice,
}

impl ViewportEditor {
    pub fn new(device: &RenderDevice) -> Result<Self> {
        Ok(Self {
            state: Arc::new(Mutex::new(ViewportEditorState::default())),
            windows: HashMap::new(),
            textures: HashMap::new(),
            retired_textures: Vec::new(),
            descriptor_allocator: DescriptorAllocator::new(device, 8)?,
            sampler: create_sampler(device)?,
            device: device.clone(),
        })
    }

    pub fn add_viewport(
        &mut self,
        player: PlayerId,
        viewport: &Viewport,
        editor: &EditorServices,
        egui: &mut EguiState,
    ) -> Result<()> {
        if self.windows.contains_key(&player) || self.textures.contains_key(&player) {
            self.remove_viewport(player, editor, egui);
        }

        let descriptor_set = self.descriptor_allocator.allocate()?;
        write_viewport_descriptor(
            &self.device,
            &descriptor_set,
            self.sampler,
            viewport.output_view(),
        );
        let texture_id = egui.add_user_texture(descriptor_set.raw());

        self.state.lock().insert(
            player,
            ViewportEditorEntry {
                texture_id,
                extent: viewport.settings().extent,
                ready: false,
                requested_extent: None,
            },
        );

        let state = Arc::clone(&self.state);
        let descriptor = EditorWindowDescriptor {
            title: format!("Viewport {player}"),
            category: VIEWPORT_CATEGORY.to_owned(),
            default_open: true,
            show_in_list: true,
        };
        let window_id = editor.add_window_fn(descriptor, move |ui, _context| {
            draw_viewport_window(ui, &state, player);
            Ok(())
        });

        self.windows.insert(player, window_id);
        self.textures.insert(
            player,
            ViewportTextureBinding {
                texture_id,
                _descriptor_set: descriptor_set,
            },
        );
        Ok(())
    }

    pub fn remove_viewport(
        &mut self,
        player: PlayerId,
        editor: &EditorServices,
        egui: &mut EguiState,
    ) {
        if let Some(window) = self.windows.remove(&player) {
            editor.remove_window(window);
        }
        if let Some(binding) = self.textures.remove(&player) {
            egui.remove_user_texture(binding.texture_id);
        }
        self.state.lock().remove(player);
    }

    pub fn release_retired_textures(&mut self, egui: &mut EguiState) {
        self.retired_textures.retain_mut(|retired| {
            retired.frames_remaining = retired.frames_remaining.saturating_sub(1);
            if retired.frames_remaining == 0 {
                egui.remove_user_texture(retired.binding.texture_id);
                false
            } else {
                true
            }
        });
    }

    pub fn apply_resize_requests(
        &mut self,
        viewports: &mut HashMap<PlayerId, Viewport>,
        egui: &mut EguiState,
    ) -> Result<()> {
        let requests = self.state.lock().take_resize_requests();
        for (player, requested_extent) in requests {
            let Some(viewport) = viewports.get_mut(&player) else {
                continue;
            };
            if viewport.settings().extent == requested_extent {
                continue;
            }

            let descriptor_set = self.descriptor_allocator.allocate()?;
            viewport.resize(&self.device, requested_extent)?;
            write_viewport_descriptor(
                &self.device,
                &descriptor_set,
                self.sampler,
                viewport.output_view(),
            );
            let texture_id = egui.add_user_texture(descriptor_set.raw());
            let new_binding = ViewportTextureBinding {
                texture_id,
                _descriptor_set: descriptor_set,
            };
            if let Some(old_binding) = self.textures.insert(player, new_binding) {
                self.retired_textures.push(RetiredViewportTextureBinding {
                    binding: old_binding,
                    frames_remaining: MAX_FRAMES_IN_FLIGHT + 1,
                });
            }
            self.state
                .lock()
                .replace_texture(player, texture_id, viewport.settings().extent);
        }
        Ok(())
    }

    pub fn sync_ready_state(&self, viewports: &HashMap<PlayerId, Viewport>) {
        let mut state = self.state.lock();
        for (player, entry) in state.entries_mut() {
            if let Some(viewport) = viewports.get(player) {
                entry.ready = viewport.is_renderable() && viewport.has_rendered();
                entry.extent = viewport.settings().extent;
            } else {
                entry.ready = false;
            }
        }
    }
}

impl Drop for ViewportEditor {
    fn drop(&mut self) {
        self.textures.clear();
        self.retired_textures.clear();
        self.device.destroy(Garbage::Sampler(self.sampler));
    }
}

#[derive(Default)]
pub struct ViewportEditorState {
    entries: HashMap<PlayerId, ViewportEditorEntry>,
}

impl ViewportEditorState {
    fn insert(&mut self, player: PlayerId, entry: ViewportEditorEntry) {
        self.entries.insert(player, entry);
    }

    fn remove(&mut self, player: PlayerId) {
        self.entries.remove(&player);
    }

    fn entry(&self, player: PlayerId) -> Option<ViewportEditorEntry> {
        self.entries.get(&player).copied()
    }

    fn entries_mut(&mut self) -> impl Iterator<Item = (&PlayerId, &mut ViewportEditorEntry)> {
        self.entries.iter_mut()
    }

    fn request_extent(&mut self, player: PlayerId, extent: vk::Extent2D) {
        if let Some(entry) = self.entries.get_mut(&player) {
            entry.requested_extent = Some(clamp_extent(extent));
        }
    }

    #[must_use]
    fn take_resize_requests(&mut self) -> Vec<(PlayerId, vk::Extent2D)> {
        self.entries
            .iter_mut()
            .filter_map(|(player, entry)| {
                entry
                    .requested_extent
                    .take()
                    .map(|extent| (*player, clamp_extent(extent)))
            })
            .collect()
    }

    fn replace_texture(
        &mut self,
        player: PlayerId,
        texture_id: egui::TextureId,
        extent: vk::Extent2D,
    ) {
        if let Some(entry) = self.entries.get_mut(&player) {
            entry.texture_id = texture_id;
            entry.extent = extent;
            entry.ready = false;
        }
    }
}

fn draw_viewport_window(
    ui: &mut egui::Ui,
    state: &Arc<Mutex<ViewportEditorState>>,
    player: PlayerId,
) {
    let available = ui.available_size_before_wrap();
    let requested_extent = extent_from_points(available, ui.ctx().pixels_per_point());

    let entry = {
        let mut state = state.lock();
        state.request_extent(player, requested_extent);
        state.entry(player)
    };

    let Some(entry) = entry else {
        ui.centered_and_justified(|ui| {
            ui.label("Waiting for camera");
        });
        return;
    };

    if entry.ready {
        ui.add(egui::Image::new((entry.texture_id, available)));
    } else {
        ui.centered_and_justified(|ui| {
            ui.label("Waiting for camera");
        });
    }
}

fn extent_from_points(size: egui::Vec2, pixels_per_point: f32) -> vk::Extent2D {
    let pixels_per_point = pixels_per_point.max(f32::EPSILON);
    vk::Extent2D {
        width: point_size_to_pixels(size.x, pixels_per_point),
        height: point_size_to_pixels(size.y, pixels_per_point),
    }
}

fn point_size_to_pixels(points: f32, pixels_per_point: f32) -> u32 {
    if points.is_finite() {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            (points.max(1.0) * pixels_per_point).round().max(1.0) as u32
        }
    } else {
        1
    }
}

fn clamp_extent(extent: vk::Extent2D) -> vk::Extent2D {
    vk::Extent2D {
        width: extent.width.max(1),
        height: extent.height.max(1),
    }
}

fn write_viewport_descriptor(
    device: &RenderDevice,
    descriptor_set: &DescriptorSet<ViewportTextureSet>,
    sampler: vk::Sampler,
    view: vk::ImageView,
) {
    DescriptorWriter::new(&device.device)
        .combined_image_sampler(descriptor_set, 0, view, sampler)
        .flush();
}

fn create_sampler(device: &RenderDevice) -> Result<vk::Sampler> {
    let sampler_info = vk::SamplerCreateInfo::default()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .compare_enable(false)
        .min_lod(0.0)
        .max_lod(1.0)
        .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
        .unnormalized_coordinates(false);

    Ok(unsafe { device.device.create_sampler(&sampler_info, None)? })
}
