use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use ash::vk;
use dirk_engine::editor::{
    EditorServices, EditorWindowDescriptor, EditorWindowId, VIEWPORT_CATEGORY,
};
use dirk_platform::{InputEvent, WindowId};
use dirk_player::{PlayerId, PlayerInput};
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
            self.remove_viewport(player, editor);
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

    pub fn remove_viewport(&mut self, player: PlayerId, editor: &EditorServices) {
        if let Some(window) = self.windows.remove(&player) {
            editor.remove_window(window);
        }
        if let Some(binding) = self.textures.remove(&player) {
            self.retired_textures.push(RetiredViewportTextureBinding {
                binding,
                frames_remaining: MAX_FRAMES_IN_FLIGHT + 1,
            });
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

    pub fn begin_frame(&self) {
        self.state.lock().clear_input_regions();
    }

    pub fn route_input_events(
        &self,
        window: WindowId,
        pixels_per_point: f32,
        events: impl IntoIterator<Item = InputEvent>,
    ) -> Vec<PlayerInput> {
        self.state
            .lock()
            .route_input_events(window, pixels_per_point, events)
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
    input_regions: HashMap<PlayerId, egui::Rect>,
    focused: Option<PlayerId>,
    hovered: Option<PlayerId>,
    pointer_capture: Option<PlayerId>,
    last_pointer_position: Option<glam::DVec2>,
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

    fn clear_input_regions(&mut self) {
        self.input_regions.clear();
    }

    fn set_input_region(&mut self, player: PlayerId, rect: egui::Rect) {
        self.input_regions.insert(player, rect);
    }

    fn route_input_events(
        &mut self,
        window: WindowId,
        pixels_per_point: f32,
        events: impl IntoIterator<Item = InputEvent>,
    ) -> Vec<PlayerInput> {
        events
            .into_iter()
            .flat_map(|event| self.route_input_event(window, pixels_per_point, event))
            .collect()
    }

    fn route_input_event(
        &mut self,
        window: WindowId,
        pixels_per_point: f32,
        event: InputEvent,
    ) -> Vec<PlayerInput> {
        match event {
            InputEvent::PointerMoved { position, .. } => {
                self.last_pointer_position = Some(position);
                self.route_pointer_moved(window, pixels_per_point, position)
            }
            InputEvent::MouseButtonPressed {
                button, position, ..
            } => self.route_pointer_button(window, pixels_per_point, button, position, true),
            InputEvent::MouseButtonReleased {
                button, position, ..
            } => self.route_pointer_button(window, pixels_per_point, button, position, false),
            InputEvent::MouseWheelScrolled { delta, .. } => self
                .target_for_wheel(pixels_per_point)
                .map_or_else(Vec::new, |player| {
                    vec![PlayerInput {
                        id: player,
                        event: InputEvent::MouseWheelScrolled { id: window, delta },
                    }]
                }),
            InputEvent::KeyPressed {
                key,
                physical_key,
                modifiers,
                repeat,
                ..
            } => self.focused.map_or_else(Vec::new, |player| {
                vec![PlayerInput {
                    id: player,
                    event: InputEvent::KeyPressed {
                        id: window,
                        key,
                        physical_key,
                        modifiers,
                        repeat,
                    },
                }]
            }),
            InputEvent::KeyReleased {
                key,
                physical_key,
                modifiers,
                ..
            } => self.focused.map_or_else(Vec::new, |player| {
                vec![PlayerInput {
                    id: player,
                    event: InputEvent::KeyReleased {
                        id: window,
                        key,
                        physical_key,
                        modifiers,
                    },
                }]
            }),
            InputEvent::ModifiersChanged { modifiers, .. } => {
                self.focused.map_or_else(Vec::new, |player| {
                    vec![PlayerInput {
                        id: player,
                        event: InputEvent::ModifiersChanged {
                            id: window,
                            modifiers,
                        },
                    }]
                })
            }
            InputEvent::PointerLeft { .. } => self.route_pointer_left(window),
            InputEvent::PointerEntered { .. } => Vec::new(),
        }
    }

    fn route_pointer_moved(
        &mut self,
        window: WindowId,
        pixels_per_point: f32,
        position: glam::DVec2,
    ) -> Vec<PlayerInput> {
        let target = self
            .pointer_capture
            .or_else(|| self.player_at_position(position, pixels_per_point));
        let mut routed = self.update_hover(window, target);
        if let Some(player) = target {
            routed.push(PlayerInput {
                id: player,
                event: InputEvent::PointerMoved {
                    id: window,
                    position: self.local_position(player, position, pixels_per_point),
                },
            });
        }
        routed
    }

    fn route_pointer_button(
        &mut self,
        window: WindowId,
        pixels_per_point: f32,
        button: dirk_platform::ButtonSource,
        position: glam::DVec2,
        pressed: bool,
    ) -> Vec<PlayerInput> {
        self.last_pointer_position = Some(position);
        let target = if pressed {
            self.player_at_position(position, pixels_per_point)
        } else {
            self.pointer_capture
                .or_else(|| self.player_at_position(position, pixels_per_point))
        };

        let mut routed = self.update_hover(window, target);
        if let Some(player) = target {
            if pressed {
                self.focused = Some(player);
                self.pointer_capture = Some(player);
            } else if self.pointer_capture == Some(player) {
                self.pointer_capture = None;
            }

            let position = self.local_position(player, position, pixels_per_point);
            let event = if pressed {
                InputEvent::MouseButtonPressed {
                    id: window,
                    button,
                    position,
                }
            } else {
                InputEvent::MouseButtonReleased {
                    id: window,
                    button,
                    position,
                }
            };
            routed.push(PlayerInput { id: player, event });
        } else if !pressed {
            self.pointer_capture = None;
        }

        routed
    }

    fn route_pointer_left(&mut self, window: WindowId) -> Vec<PlayerInput> {
        let mut routed = Vec::new();
        let previous_hover = self.hovered.take();
        if let Some(player) = previous_hover {
            routed.push(PlayerInput {
                id: player,
                event: InputEvent::PointerLeft { id: window },
            });
        }
        if let Some(player) = self.pointer_capture.take()
            && Some(player) != previous_hover
        {
            routed.push(PlayerInput {
                id: player,
                event: InputEvent::PointerLeft { id: window },
            });
        }
        self.last_pointer_position = None;
        routed
    }

    fn update_hover(&mut self, window: WindowId, target: Option<PlayerId>) -> Vec<PlayerInput> {
        if self.hovered == target {
            return Vec::new();
        }

        let mut routed = Vec::new();
        if let Some(previous) = self.hovered.replace_or_clear(target) {
            routed.push(PlayerInput {
                id: previous,
                event: InputEvent::PointerLeft { id: window },
            });
        }
        if let Some(player) = target {
            routed.push(PlayerInput {
                id: player,
                event: InputEvent::PointerEntered { id: window },
            });
        }
        routed
    }

    fn target_for_wheel(&self, pixels_per_point: f32) -> Option<PlayerId> {
        self.pointer_capture.or_else(|| {
            self.last_pointer_position
                .and_then(|position| self.player_at_position(position, pixels_per_point))
        })
    }

    fn player_at_position(&self, position: glam::DVec2, pixels_per_point: f32) -> Option<PlayerId> {
        let position = position_to_points(position, pixels_per_point);
        self.input_regions
            .iter()
            .find_map(|(player, rect)| rect.contains(position).then_some(*player))
    }

    fn local_position(
        &self,
        player: PlayerId,
        position: glam::DVec2,
        pixels_per_point: f32,
    ) -> glam::DVec2 {
        let Some(rect) = self.input_regions.get(&player) else {
            return position;
        };
        let scale = f64::from(pixels_per_point.max(f32::EPSILON));
        glam::dvec2(
            position.x - (f64::from(rect.min.x) * scale),
            position.y - (f64::from(rect.min.y) * scale),
        )
    }
}

trait ReplaceOrClear<T> {
    fn replace_or_clear(&mut self, value: Option<T>) -> Option<T>;
}

impl<T> ReplaceOrClear<T> for Option<T> {
    fn replace_or_clear(&mut self, value: Option<T>) -> Option<T> {
        match value {
            Some(value) => self.replace(value),
            None => self.take(),
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
        let response =
            ui.add(egui::Image::new((entry.texture_id, available)).sense(egui::Sense::drag()));
        state.lock().set_input_region(player, response.rect);
    } else {
        ui.centered_and_justified(|ui| {
            ui.label("Waiting for camera");
        });
    }
}

#[allow(clippy::cast_possible_truncation)]
fn position_to_points(position: glam::DVec2, pixels_per_point: f32) -> egui::Pos2 {
    let scale = f64::from(pixels_per_point.max(f32::EPSILON));
    egui::Pos2::new((position.x / scale) as f32, (position.y / scale) as f32)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn player(index: u32) -> PlayerId {
        PlayerId::default() + index
    }

    #[test]
    fn viewport_window_state_records_requested_extents_clamped_to_at_least_1x1() {
        let mut state = ViewportEditorState::default();
        state.insert(
            player(0),
            ViewportEditorEntry {
                texture_id: egui::TextureId::User(7),
                extent: vk::Extent2D {
                    width: 640,
                    height: 480,
                },
                ready: false,
                requested_extent: None,
            },
        );

        state.request_extent(
            player(0),
            vk::Extent2D {
                width: 0,
                height: 0,
            },
        );

        assert_eq!(
            state.take_resize_requests(),
            vec![(
                player(0),
                vk::Extent2D {
                    width: 1,
                    height: 1
                }
            )]
        );
    }

    #[test]
    fn removing_a_viewport_clears_shared_state() {
        let mut state = ViewportEditorState::default();
        state.insert(
            player(0),
            ViewportEditorEntry {
                texture_id: egui::TextureId::User(1),
                extent: vk::Extent2D {
                    width: 1,
                    height: 1,
                },
                ready: false,
                requested_extent: None,
            },
        );

        state.remove(player(0));

        assert_eq!(state.entry(player(0)), None);
    }

    #[test]
    fn ready_state_mirrors_renderable_and_rendered_flags() {
        let mut entry = ViewportEditorEntry {
            texture_id: egui::TextureId::User(1),
            extent: vk::Extent2D {
                width: 1,
                height: 1,
            },
            ready: true,
            requested_extent: None,
        };

        entry.ready = ready_from_flags(true, false);
        assert!(!entry.ready);

        entry.ready = ready_from_flags(false, true);
        assert!(!entry.ready);

        entry.ready = ready_from_flags(true, true);
        assert!(entry.ready);
    }

    #[test]
    fn descriptor_window_bookkeeping_maps_one_player_to_one_editor_window_entry() {
        let editor = EditorServices::new();
        let first = editor.add_window_fn(
            EditorWindowDescriptor {
                title: "Viewport 0".to_owned(),
                category: VIEWPORT_CATEGORY.to_owned(),
                default_open: true,
                show_in_list: true,
            },
            |_ui, _context| Ok(()),
        );
        let second = editor.add_window_fn(
            EditorWindowDescriptor {
                title: "Viewport 1".to_owned(),
                category: VIEWPORT_CATEGORY.to_owned(),
                default_open: true,
                show_in_list: true,
            },
            |_ui, _context| Ok(()),
        );
        let mut windows = HashMap::new();

        assert_eq!(windows.insert(player(0), first), None);
        assert_eq!(windows.insert(player(1), second), None);

        assert_eq!(windows.get(&player(0)), Some(&first));
        assert_eq!(windows.get(&player(1)), Some(&second));
        assert_eq!(windows.len(), 2);
    }

    #[test]
    fn pointer_button_inside_viewport_routes_to_player_with_local_position() {
        let mut state = ViewportEditorState::default();
        let window = window_id(1);
        state.set_input_region(
            player(0),
            egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(100.0, 80.0)),
        );

        let routed = state.route_input_events(
            window,
            2.0,
            [InputEvent::MouseButtonPressed {
                id: window,
                button: dirk_platform::ButtonSource::Mouse(dirk_platform::MouseButton::Right),
                position: glam::dvec2(30.0, 50.0),
            }],
        );

        assert_eq!(routed.len(), 2);
        assert_eq!(routed[0].id, player(0));
        assert!(matches!(
            routed[0].event,
            InputEvent::PointerEntered { id } if id == window
        ));
        assert!(matches!(
            &routed[1].event,
            InputEvent::MouseButtonPressed { position, .. }
                if *position == glam::dvec2(10.0, 10.0)
        ));
    }

    #[test]
    fn keyboard_input_routes_to_focused_viewport() {
        let mut state = ViewportEditorState::default();
        let window = window_id(1);
        state.set_input_region(
            player(0),
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100.0, 80.0)),
        );
        state.route_input_events(
            window,
            1.0,
            [InputEvent::MouseButtonPressed {
                id: window,
                button: dirk_platform::ButtonSource::Mouse(dirk_platform::MouseButton::Right),
                position: glam::DVec2::ZERO,
            }],
        );

        let routed = state.route_input_events(
            window,
            1.0,
            [InputEvent::KeyPressed {
                id: window,
                key: dirk_platform::Key::Character("w".into()),
                physical_key: dirk_platform::PhysicalKey::Code(dirk_platform::KeyCode::KeyW),
                modifiers: dirk_platform::ModifiersState::empty(),
                repeat: false,
            }],
        );

        assert_eq!(routed.len(), 1);
        assert_eq!(routed[0].id, player(0));
        assert!(matches!(routed[0].event, InputEvent::KeyPressed { .. }));
    }

    #[test]
    fn captured_pointer_keeps_routing_after_moving_outside_viewport() {
        let mut state = ViewportEditorState::default();
        let window = window_id(1);
        state.set_input_region(
            player(0),
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100.0, 100.0)),
        );
        state.route_input_events(
            window,
            1.0,
            [InputEvent::MouseButtonPressed {
                id: window,
                button: dirk_platform::ButtonSource::Mouse(dirk_platform::MouseButton::Right),
                position: glam::dvec2(10.0, 10.0),
            }],
        );

        let routed = state.route_input_events(
            window,
            1.0,
            [InputEvent::PointerMoved {
                id: window,
                position: glam::dvec2(200.0, 150.0),
            }],
        );

        assert_eq!(routed.len(), 1);
        assert_eq!(routed[0].id, player(0));
        assert!(matches!(
            &routed[0].event,
            InputEvent::PointerMoved { position, .. }
                if *position == glam::dvec2(200.0, 150.0)
        ));
    }

    fn window_id(raw: usize) -> WindowId {
        WindowId::from_raw(raw)
    }

    fn ready_from_flags(renderable: bool, rendered: bool) -> bool {
        renderable && rendered
    }
}
