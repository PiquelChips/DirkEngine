use std::time::Instant;

use ash::vk;
use dirk_platform::{
    ButtonSource, InputCapture, Key, KeyCode, ModifiersState, MouseButton, NamedKey, PhysicalKey,
    ScrollDelta, Theme, UiImeEvent, UiInputEvent, WindowId,
};
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
    pub events: Vec<UiInputEvent>,
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

    pub fn end_frame(&mut self) -> InputCapture {
        let output = self.ctx.end_pass();
        let primitives = self.ctx.tessellate(output.shapes, output.pixels_per_point);
        self.pending = Some(EguiPaintData {
            textures_delta: output.textures_delta,
            primitives,
            pixels_per_point: output.pixels_per_point,
        });
        InputCapture {
            pointer: self.ctx.wants_pointer_input(),
            keyboard: self.ctx.wants_keyboard_input(),
        }
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
    native_pixels_per_point: f32,
    events: &[UiInputEvent],
) -> Vec<egui::Event> {
    let mut translated = Vec::new();
    for event in events {
        if event.id() != window_id {
            continue;
        }
        append_translated_event(&mut translated, native_pixels_per_point, event);
    }
    translated
}

#[allow(clippy::too_many_lines)]
fn append_translated_event(
    out: &mut Vec<egui::Event>,
    native_pixels_per_point: f32,
    event: &UiInputEvent,
) {
    match event {
        UiInputEvent::WindowFocused { focused, .. } => {
            out.push(egui::Event::WindowFocused(*focused));
        }
        UiInputEvent::ModifiersChanged { .. } => {}
        UiInputEvent::Key {
            key,
            physical_key,
            pressed,
            repeat,
            modifiers,
            text,
            ..
        } => {
            let modifiers = modifiers_to_egui(*modifiers);
            if let Some(key) = key_to_egui(key) {
                out.push(egui::Event::Key {
                    key,
                    physical_key: physical_key_to_egui(*physical_key),
                    pressed: *pressed,
                    repeat: *repeat,
                    modifiers,
                });
            }
            if *pressed
                && !modifiers.command
                && !modifiers.ctrl
                && let Some(text) = text
                && should_emit_text_event(text)
            {
                out.push(egui::Event::Text(text.clone()));
            }
        }
        UiInputEvent::PointerMoved { position, .. } => {
            out.push(egui::Event::PointerMoved(position_to_egui(
                *position,
                native_pixels_per_point,
            )));
        }
        UiInputEvent::PointerGone { .. } => {
            out.push(egui::Event::PointerGone);
        }
        UiInputEvent::PointerButton {
            button,
            position,
            pressed,
            modifiers,
            ..
        } => {
            if let Some(button) = button_to_egui(button.clone()) {
                out.push(egui::Event::PointerButton {
                    pos: position_to_egui(*position, native_pixels_per_point),
                    button,
                    pressed: *pressed,
                    modifiers: modifiers_to_egui(*modifiers),
                });
            }
        }
        UiInputEvent::MouseWheel {
            delta, modifiers, ..
        } => {
            let (unit, delta) = scroll_to_egui(delta);
            out.push(egui::Event::MouseWheel {
                unit,
                delta,
                modifiers: modifiers_to_egui(*modifiers),
            });
        }
        UiInputEvent::Ime { event, .. } => {
            out.push(egui::Event::Ime(match event {
                UiImeEvent::Enabled => egui::ImeEvent::Enabled,
                UiImeEvent::Preedit(text) => egui::ImeEvent::Preedit(text.clone()),
                UiImeEvent::Commit(text) => egui::ImeEvent::Commit(text.clone()),
                UiImeEvent::Disabled => egui::ImeEvent::Disabled,
            }));
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn position_to_egui(position: glam::DVec2, native_pixels_per_point: f32) -> Pos2 {
    let scale = f64::from(native_pixels_per_point.max(f32::EPSILON));
    Pos2::new((position.x / scale) as f32, (position.y / scale) as f32)
}

fn modifiers_to_egui(modifiers: ModifiersState) -> egui::Modifiers {
    let ctrl = modifiers.control_key();
    let mac_cmd = cfg!(target_os = "macos") && modifiers.meta_key();
    egui::Modifiers {
        alt: modifiers.alt_key(),
        ctrl,
        shift: modifiers.shift_key(),
        mac_cmd,
        command: if cfg!(target_os = "macos") {
            modifiers.meta_key()
        } else {
            ctrl
        },
    }
}

fn key_to_egui(key: &Key) -> Option<egui::Key> {
    match key {
        Key::Character(text) => text.chars().next().and_then(char_to_egui_key),
        Key::Named(named) => named_key_to_egui(*named),
        Key::Unidentified(_) | Key::Dead(_) => None,
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
        NamedKey::Insert => egui::Key::Insert,
        NamedKey::Delete => egui::Key::Delete,
        NamedKey::Home => egui::Key::Home,
        NamedKey::End => egui::Key::End,
        NamedKey::PageUp => egui::Key::PageUp,
        NamedKey::PageDown => egui::Key::PageDown,
        NamedKey::Copy => egui::Key::Copy,
        NamedKey::Cut => egui::Key::Cut,
        NamedKey::Paste => egui::Key::Paste,
        NamedKey::F1 => egui::Key::F1,
        NamedKey::F2 => egui::Key::F2,
        NamedKey::F3 => egui::Key::F3,
        NamedKey::F4 => egui::Key::F4,
        NamedKey::F5 => egui::Key::F5,
        NamedKey::F6 => egui::Key::F6,
        NamedKey::F7 => egui::Key::F7,
        NamedKey::F8 => egui::Key::F8,
        NamedKey::F9 => egui::Key::F9,
        NamedKey::F10 => egui::Key::F10,
        NamedKey::F11 => egui::Key::F11,
        NamedKey::F12 => egui::Key::F12,
        NamedKey::F13 => egui::Key::F13,
        NamedKey::F14 => egui::Key::F14,
        NamedKey::F15 => egui::Key::F15,
        NamedKey::F16 => egui::Key::F16,
        NamedKey::F17 => egui::Key::F17,
        NamedKey::F18 => egui::Key::F18,
        NamedKey::F19 => egui::Key::F19,
        NamedKey::BrowserBack => egui::Key::BrowserBack,
        _ => return None,
    })
}

fn physical_key_to_egui(key: PhysicalKey) -> Option<egui::Key> {
    match key {
        PhysicalKey::Code(code) => key_code_to_egui(code),
        PhysicalKey::Unidentified(_) => None,
    }
}

fn key_code_to_egui(code: KeyCode) -> Option<egui::Key> {
    Some(match code {
        KeyCode::ArrowDown => egui::Key::ArrowDown,
        KeyCode::ArrowLeft => egui::Key::ArrowLeft,
        KeyCode::ArrowRight => egui::Key::ArrowRight,
        KeyCode::ArrowUp => egui::Key::ArrowUp,
        KeyCode::Escape => egui::Key::Escape,
        KeyCode::Tab => egui::Key::Tab,
        KeyCode::Backspace => egui::Key::Backspace,
        KeyCode::Enter => egui::Key::Enter,
        KeyCode::Space => egui::Key::Space,
        KeyCode::Insert => egui::Key::Insert,
        KeyCode::Delete => egui::Key::Delete,
        KeyCode::Home => egui::Key::Home,
        KeyCode::End => egui::Key::End,
        KeyCode::PageUp => egui::Key::PageUp,
        KeyCode::PageDown => egui::Key::PageDown,
        KeyCode::KeyA => egui::Key::A,
        KeyCode::KeyB => egui::Key::B,
        KeyCode::KeyC => egui::Key::C,
        KeyCode::KeyD => egui::Key::D,
        KeyCode::KeyE => egui::Key::E,
        KeyCode::KeyF => egui::Key::F,
        KeyCode::KeyG => egui::Key::G,
        KeyCode::KeyH => egui::Key::H,
        KeyCode::KeyI => egui::Key::I,
        KeyCode::KeyJ => egui::Key::J,
        KeyCode::KeyK => egui::Key::K,
        KeyCode::KeyL => egui::Key::L,
        KeyCode::KeyM => egui::Key::M,
        KeyCode::KeyN => egui::Key::N,
        KeyCode::KeyO => egui::Key::O,
        KeyCode::KeyP => egui::Key::P,
        KeyCode::KeyQ => egui::Key::Q,
        KeyCode::KeyR => egui::Key::R,
        KeyCode::KeyS => egui::Key::S,
        KeyCode::KeyT => egui::Key::T,
        KeyCode::KeyU => egui::Key::U,
        KeyCode::KeyV => egui::Key::V,
        KeyCode::KeyW => egui::Key::W,
        KeyCode::KeyX => egui::Key::X,
        KeyCode::KeyY => egui::Key::Y,
        KeyCode::KeyZ => egui::Key::Z,
        KeyCode::Digit0 | KeyCode::Numpad0 => egui::Key::Num0,
        KeyCode::Digit1 | KeyCode::Numpad1 => egui::Key::Num1,
        KeyCode::Digit2 | KeyCode::Numpad2 => egui::Key::Num2,
        KeyCode::Digit3 | KeyCode::Numpad3 => egui::Key::Num3,
        KeyCode::Digit4 | KeyCode::Numpad4 => egui::Key::Num4,
        KeyCode::Digit5 | KeyCode::Numpad5 => egui::Key::Num5,
        KeyCode::Digit6 | KeyCode::Numpad6 => egui::Key::Num6,
        KeyCode::Digit7 | KeyCode::Numpad7 => egui::Key::Num7,
        KeyCode::Digit8 | KeyCode::Numpad8 => egui::Key::Num8,
        KeyCode::Digit9 | KeyCode::Numpad9 => egui::Key::Num9,
        KeyCode::F1 => egui::Key::F1,
        KeyCode::F2 => egui::Key::F2,
        KeyCode::F3 => egui::Key::F3,
        KeyCode::F4 => egui::Key::F4,
        KeyCode::F5 => egui::Key::F5,
        KeyCode::F6 => egui::Key::F6,
        KeyCode::F7 => egui::Key::F7,
        KeyCode::F8 => egui::Key::F8,
        KeyCode::F9 => egui::Key::F9,
        KeyCode::F10 => egui::Key::F10,
        KeyCode::F11 => egui::Key::F11,
        KeyCode::F12 => egui::Key::F12,
        KeyCode::F13 => egui::Key::F13,
        KeyCode::F14 => egui::Key::F14,
        KeyCode::F15 => egui::Key::F15,
        KeyCode::F16 => egui::Key::F16,
        KeyCode::F17 => egui::Key::F17,
        KeyCode::F18 => egui::Key::F18,
        KeyCode::F19 => egui::Key::F19,
        KeyCode::BrowserBack => egui::Key::BrowserBack,
        _ => return None,
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
        ' ' => egui::Key::Space,
        ':' => egui::Key::Colon,
        ',' => egui::Key::Comma,
        '\\' => egui::Key::Backslash,
        '/' => egui::Key::Slash,
        '|' => egui::Key::Pipe,
        '?' => egui::Key::Questionmark,
        '!' => egui::Key::Exclamationmark,
        '[' => egui::Key::OpenBracket,
        ']' => egui::Key::CloseBracket,
        '{' => egui::Key::OpenCurlyBracket,
        '}' => egui::Key::CloseCurlyBracket,
        '`' => egui::Key::Backtick,
        '-' => egui::Key::Minus,
        '.' => egui::Key::Period,
        '+' => egui::Key::Plus,
        '=' => egui::Key::Equals,
        ';' => egui::Key::Semicolon,
        '\'' => egui::Key::Quote,
        _ => return None,
    })
}

fn button_to_egui(button: ButtonSource) -> Option<egui::PointerButton> {
    Some(match button.mouse_button()? {
        MouseButton::Left => egui::PointerButton::Primary,
        MouseButton::Right => egui::PointerButton::Secondary,
        MouseButton::Middle => egui::PointerButton::Middle,
        MouseButton::Back => egui::PointerButton::Extra1,
        MouseButton::Forward => egui::PointerButton::Extra2,
        _ => return None,
    })
}

#[allow(clippy::cast_possible_truncation)]
fn scroll_to_egui(delta: &ScrollDelta) -> (MouseWheelUnit, Vec2) {
    match *delta {
        ScrollDelta::Lines { x, y } => (MouseWheelUnit::Line, Vec2::new(x, y)),
        ScrollDelta::Pixels { x, y } => (MouseWheelUnit::Point, Vec2::new(x as f32, y as f32)),
    }
}

fn should_emit_text_event(text: &str) -> bool {
    !text.is_empty() && !text.chars().any(char::is_control)
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
    fn does_not_treat_unorm_formats_as_srgb() {
        assert!(!is_srgb_format(vk::Format::R8G8B8A8_UNORM));
        assert!(!is_srgb_format(vk::Format::B8G8R8A8_UNORM));
        assert!(!is_srgb_format(vk::Format::A8B8G8R8_UNORM_PACK32));
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn modifiers_map_ctrl_to_command_on_non_macos() {
        let modifiers = modifiers_to_egui(ModifiersState::CONTROL);

        assert!(modifiers.ctrl);
        assert!(modifiers.command);
        assert!(!modifiers.mac_cmd);
    }

    #[test]
    fn key_characters_map_to_egui_letters_and_digits() {
        assert_eq!(key_to_egui(&Key::Character("a".into())), Some(egui::Key::A));
        assert_eq!(
            key_to_egui(&Key::Character("7".into())),
            Some(egui::Key::Num7)
        );
    }

    #[test]
    fn named_keys_map_to_navigation_keys() {
        assert_eq!(
            key_to_egui(&Key::Named(NamedKey::ArrowLeft)),
            Some(egui::Key::ArrowLeft)
        );
        assert_eq!(
            key_to_egui(&Key::Named(NamedKey::PageDown)),
            Some(egui::Key::PageDown)
        );
    }

    #[test]
    fn pointer_positions_are_scaled_from_physical_pixels_to_points() {
        let events = translate_events(
            window_id(1),
            2.0,
            &[UiInputEvent::PointerMoved {
                id: window_id(1),
                position: glam::dvec2(20.0, 10.0),
            }],
        );

        assert_eq!(
            events,
            vec![egui::Event::PointerMoved(Pos2::new(10.0, 5.0))]
        );
    }

    #[test]
    fn wheel_line_delta_maps_to_line_unit() {
        let (unit, delta) = scroll_to_egui(&ScrollDelta::Lines { x: 1.0, y: -2.0 });

        assert_eq!(unit, MouseWheelUnit::Line);
        assert_eq!(delta, Vec2::new(1.0, -2.0));
    }

    #[test]
    fn wheel_pixel_delta_maps_to_point_unit() {
        let (unit, delta) = scroll_to_egui(&ScrollDelta::Pixels { x: 3.0, y: -4.0 });

        assert_eq!(unit, MouseWheelUnit::Point);
        assert_eq!(delta, Vec2::new(3.0, -4.0));
    }

    #[test]
    fn text_events_are_not_emitted_for_command_shortcuts() {
        let events = translate_events(
            window_id(1),
            1.0,
            &[UiInputEvent::Key {
                id: window_id(1),
                key: Key::Character("c".into()),
                physical_key: PhysicalKey::Code(KeyCode::KeyC),
                pressed: true,
                repeat: false,
                modifiers: ModifiersState::CONTROL,
                text: Some("c".to_owned()),
            }],
        );

        assert!(
            events
                .iter()
                .all(|event| !matches!(event, egui::Event::Text(_)))
        );
    }
}
