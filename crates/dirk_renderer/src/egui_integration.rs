use std::time::Instant;

use ash::vk;
use dirk_input::{
    ButtonState, InputEvent, LogicalKey, NamedKey, NormalizedDelta, NormalizedPosition,
    PointerButton,
};
use dirk_platform::{Theme, WindowId, WindowInputEvent};
use egui::{
    ClippedPrimitive, Context, MouseWheelUnit, Pos2, TextureId, TexturesDelta, Vec2, ViewportId,
    ViewportInfo,
};
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

pub struct EguiFrameInput {
    pub window_id: WindowId,
    pub extent: vk::Extent2D,
    pub native_pixels_per_point: f32,
    pub focused: bool,
    pub theme: Option<Theme>,
    pub events: Vec<WindowInputEvent>,
}

impl EguiState {
    pub fn new(device: &RenderDevice) -> Result<Self> {
        let surface_format = device.properties.surface_format.format;
        let renderer = egui_ash_renderer::Renderer::with_default_allocator(
            &device.instance,
            device.physical_device,
            device.device.clone(),
            DynamicRendering {
                color_attachment_format: surface_format,
                depth_attachment_format: None,
            },
            Options {
                in_flight_frames: MAX_FRAMES_IN_FLIGHT,
                srgb_framebuffer: is_srgb_format(surface_format),
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
    pub fn begin_frame(&mut self, input: &EguiFrameInput) -> Context {
        let native_pixels_per_point = input.native_pixels_per_point.max(f32::EPSILON);
        let screen_rect = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(
                input.extent.width as f32 / native_pixels_per_point,
                input.extent.height as f32 / native_pixels_per_point,
            ),
        );
        let events = translate_events(
            input.window_id,
            input.extent,
            native_pixels_per_point,
            input.events.as_slice(),
        );
        let system_theme = input.theme.map(|theme| match theme {
            Theme::Dark => egui::Theme::Dark,
            Theme::Light => egui::Theme::Light,
        });
        let mut raw_input = egui::RawInput {
            screen_rect: Some(screen_rect),
            time: Some(self.start_time.elapsed().as_secs_f64()),
            focused: input.focused,
            system_theme,
            events,
            ..egui::RawInput::default()
        };
        raw_input.viewports.insert(
            ViewportId::ROOT,
            ViewportInfo {
                native_pixels_per_point: Some(native_pixels_per_point),
                inner_rect: Some(screen_rect),
                focused: Some(input.focused),
                ..ViewportInfo::default()
            },
        );

        self.ctx.begin_pass(raw_input);
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

    pub fn add_user_texture(&mut self, set: vk::DescriptorSet) -> TextureId {
        self.renderer.add_user_texture(set)
    }

    pub fn remove_user_texture(&mut self, id: TextureId) {
        self.renderer.remove_user_texture(id);
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

fn is_srgb_format(format: vk::Format) -> bool {
    matches!(
        format,
        vk::Format::R8G8B8A8_SRGB | vk::Format::B8G8R8A8_SRGB | vk::Format::A8B8G8R8_SRGB_PACK32
    )
}

fn translate_events(
    window_id: WindowId,
    extent: vk::Extent2D,
    native_pixels_per_point: f32,
    events: &[WindowInputEvent],
) -> Vec<egui::Event> {
    let mut translated = Vec::new();
    for event in events {
        if event.window != window_id {
            continue;
        }
        append_translated_event(
            &mut translated,
            extent,
            native_pixels_per_point,
            &event.event,
        );
    }
    translated
}

fn append_translated_event(
    out: &mut Vec<egui::Event>,
    extent: vk::Extent2D,
    native_pixels_per_point: f32,
    event: &InputEvent,
) {
    match event {
        InputEvent::Key {
            key,
            state,
            repeat,
            modifiers,
        } => {
            if let Some(key) = key_to_egui(key) {
                out.push(egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: *state == ButtonState::Pressed,
                    repeat: *repeat,
                    modifiers: (*modifiers).into(),
                });
            }
        }
        InputEvent::PointerMoved { position, .. } => {
            out.push(egui::Event::PointerMoved(position_to_egui(
                *position,
                extent,
                native_pixels_per_point,
            )));
        }
        InputEvent::PointerEntered => {}
        InputEvent::PointerLeft => {
            out.push(egui::Event::PointerGone);
        }
        InputEvent::PointerButton {
            button,
            state,
            position,
            modifiers,
        } => {
            out.push(egui::Event::PointerButton {
                pos: position_to_egui(*position, extent, native_pixels_per_point),
                button: button_to_egui(*button),
                pressed: *state == ButtonState::Pressed,
                modifiers: egui::Modifiers::from(*modifiers),
            });
        }
        InputEvent::Scroll {
            delta,
            modifiers,
        } => {
            out.push(egui::Event::MouseWheel {
                unit: MouseWheelUnit::Point,
                delta: delta_to_egui(*delta, extent, native_pixels_per_point),
                modifiers: egui::Modifiers::from(*modifiers),
            });
        }
    }
}

fn key_to_egui(key: &LogicalKey) -> Option<egui::Key> {
    match key {
        LogicalKey::Character(text) => text.chars().next().and_then(char_to_egui_key),
        LogicalKey::Named(named) => named_key_to_egui(*named),
    }
}

fn named_key_to_egui(key: NamedKey) -> Option<egui::Key> {
    Some(match key {
        NamedKey::ArrowDown => egui::Key::ArrowDown,
        NamedKey::ArrowLeft => egui::Key::ArrowLeft,
        NamedKey::ArrowRight => egui::Key::ArrowRight,
        NamedKey::ArrowUp => egui::Key::ArrowUp,
        NamedKey::Escape => egui::Key::Escape,
        NamedKey::Tab => egui::Key::Tab,
        NamedKey::Backspace => egui::Key::Backspace,
        NamedKey::Enter => egui::Key::Enter,
        NamedKey::Space => egui::Key::Space,
        NamedKey::Insert => egui::Key::Insert,
        NamedKey::Delete => egui::Key::Delete,
        NamedKey::Home => egui::Key::Home,
        NamedKey::End => egui::Key::End,
        NamedKey::PageUp => egui::Key::PageUp,
        NamedKey::PageDown => egui::Key::PageDown,
        NamedKey::Function(1) => egui::Key::F1,
        NamedKey::Function(2) => egui::Key::F2,
        NamedKey::Function(3) => egui::Key::F3,
        NamedKey::Function(4) => egui::Key::F4,
        NamedKey::Function(5) => egui::Key::F5,
        NamedKey::Function(6) => egui::Key::F6,
        NamedKey::Function(7) => egui::Key::F7,
        NamedKey::Function(8) => egui::Key::F8,
        NamedKey::Function(9) => egui::Key::F9,
        NamedKey::Function(10) => egui::Key::F10,
        NamedKey::Function(11) => egui::Key::F11,
        NamedKey::Function(12) => egui::Key::F12,
        NamedKey::Function(13) => egui::Key::F13,
        NamedKey::Function(14) => egui::Key::F14,
        NamedKey::Function(15) => egui::Key::F15,
        NamedKey::Function(16) => egui::Key::F16,
        NamedKey::Function(17) => egui::Key::F17,
        NamedKey::Function(18) => egui::Key::F18,
        NamedKey::Function(19) => egui::Key::F19,
        NamedKey::Function(_) => return None,
    })
}

fn char_to_egui_key(character: char) -> Option<egui::Key> {
    Some(match character.to_ascii_lowercase() {
        'a' => egui::Key::A,
        'b' => egui::Key::B,
        'c' => egui::Key::C,
        'd' => egui::Key::D,
        'e' => egui::Key::E,
        'f' => egui::Key::F,
        'g' => egui::Key::G,
        'h' => egui::Key::H,
        'i' => egui::Key::I,
        'j' => egui::Key::J,
        'k' => egui::Key::K,
        'l' => egui::Key::L,
        'm' => egui::Key::M,
        'n' => egui::Key::N,
        'o' => egui::Key::O,
        'p' => egui::Key::P,
        'q' => egui::Key::Q,
        'r' => egui::Key::R,
        's' => egui::Key::S,
        't' => egui::Key::T,
        'u' => egui::Key::U,
        'v' => egui::Key::V,
        'w' => egui::Key::W,
        'x' => egui::Key::X,
        'y' => egui::Key::Y,
        'z' => egui::Key::Z,
        '0' => egui::Key::Num0,
        '1' => egui::Key::Num1,
        '2' => egui::Key::Num2,
        '3' => egui::Key::Num3,
        '4' => egui::Key::Num4,
        '5' => egui::Key::Num5,
        '6' => egui::Key::Num6,
        '7' => egui::Key::Num7,
        '8' => egui::Key::Num8,
        '9' => egui::Key::Num9,
        _ => return None,
    })
}

fn button_to_egui(button: PointerButton) -> egui::PointerButton {
    match button {
        PointerButton::Primary => egui::PointerButton::Primary,
        PointerButton::Secondary => egui::PointerButton::Secondary,
        PointerButton::Middle => egui::PointerButton::Middle,
        PointerButton::Back | PointerButton::Other(_) => egui::PointerButton::Extra1,
        PointerButton::Forward => egui::PointerButton::Extra2,
    }
}

#[allow(clippy::cast_precision_loss)]
fn position_to_egui(
    position: NormalizedPosition,
    extent: vk::Extent2D,
    native_pixels_per_point: f32,
) -> Pos2 {
    let scale = native_pixels_per_point.max(f32::EPSILON);
    Pos2::new(
        position.0.x * extent.width as f32 / scale,
        position.0.y * extent.height as f32 / scale,
    )
}

#[allow(clippy::cast_precision_loss)]
fn delta_to_egui(
    delta: NormalizedDelta,
    extent: vk::Extent2D,
    native_pixels_per_point: f32,
) -> Vec2 {
    let scale = native_pixels_per_point.max(f32::EPSILON);
    Vec2::new(
        delta.0.x * extent.width as f32 / scale,
        delta.0.y * extent.height as f32 / scale,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window_id(raw: usize) -> WindowId {
        WindowId::from_raw(raw)
    }

    #[test]
    fn detects_common_srgb_formats() {
        assert!(is_srgb_format(vk::Format::R8G8B8A8_SRGB));
        assert!(is_srgb_format(vk::Format::B8G8R8A8_SRGB));
        assert!(is_srgb_format(vk::Format::A8B8G8R8_SRGB_PACK32));
    }

    #[test]
    fn pointer_positions_are_scaled_from_normalized_to_points() {
        let events = translate_events(
            window_id(1),
            vk::Extent2D {
                width: 200,
                height: 100,
            },
            2.0,
            &[WindowInputEvent {
                window: window_id(1),
                event: InputEvent::PointerMoved {
                    position: NormalizedPosition::new(glam::vec2(0.5, 0.5)),
                    delta: NormalizedDelta(glam::Vec2::ZERO),
                },
            }],
        );

        assert_eq!(
            events,
            vec![egui::Event::PointerMoved(Pos2::new(50.0, 25.0))]
        );
    }

    #[test]
    fn pointer_buttons_preserve_modifiers() {
        let modifiers = Modifiers {
            alt: true,
            ctrl: true,
            shift: true,
            super_key: false,
        };
        let events = translate_events(
            window_id(1),
            vk::Extent2D {
                width: 100,
                height: 100,
            },
            1.0,
            &[WindowInputEvent {
                window: window_id(1),
                event: InputEvent::PointerButton {
                    button: PointerButton::Primary,
                    state: ButtonState::Pressed,
                    position: NormalizedPosition::new(glam::vec2(0.25, 0.75)),
                    modifiers,
                },
            }],
        );

        assert_eq!(
            events,
            vec![egui::Event::PointerButton {
                pos: Pos2::new(25.0, 75.0),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: modifiers.into(),
            }]
        );
    }

}
