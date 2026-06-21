//! Default editor theme and palette.

use dirk_engine::editor::EditorStyle;

/// Color palette for the default Dirk editor theme.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EditorPalette {
    /// Main editor background.
    pub background: egui::Color32,
    /// Primary panel fill.
    pub panel: egui::Color32,
    /// Alternate panel fill.
    pub panel_alt: egui::Color32,
    /// Floating surface fill.
    pub surface: egui::Color32,
    /// Emphasized floating surface fill.
    pub surface_high: egui::Color32,
    /// Default interactive control fill.
    pub control: egui::Color32,
    /// Hovered interactive control fill.
    pub control_hovered: egui::Color32,
    /// Active control fill for specialized controls.
    pub control_active: egui::Color32,
    /// Low-contrast border color.
    pub stroke_subtle: egui::Color32,
    /// High-contrast border color.
    pub stroke_strong: egui::Color32,
    /// Default text color.
    pub text: egui::Color32,
    /// Muted text color.
    pub text_muted: egui::Color32,
    /// Bright text color.
    pub text_bright: egui::Color32,
    /// Primary accent color.
    pub accent: egui::Color32,
    /// Hovered accent color.
    pub accent_hovered: egui::Color32,
    /// Selection fill color.
    pub selection: egui::Color32,
    /// Warning text color.
    pub warn: egui::Color32,
    /// Error text color.
    pub error: egui::Color32,
}

impl Default for EditorPalette {
    fn default() -> Self {
        Self {
            background: color(0x15, 0x17, 0x1a),
            panel: color(0x1b, 0x1e, 0x22),
            panel_alt: color(0x20, 0x24, 0x2a),
            surface: color(0x24, 0x28, 0x2e),
            surface_high: color(0x2b, 0x30, 0x37),
            control: color(0x30, 0x36, 0x40),
            control_hovered: color(0x3a, 0x42, 0x4d),
            control_active: color(0x24, 0x6f, 0xa8),
            stroke_subtle: color(0x34, 0x3a, 0x43),
            stroke_strong: color(0x58, 0x61, 0x6d),
            text: color(0xd8, 0xdd, 0xe4),
            text_muted: color(0x99, 0xa2, 0xad),
            text_bright: color(0xf1, 0xf4, 0xf8),
            accent: color(0x2f, 0x8c, 0xcf),
            accent_hovered: color(0x49, 0xa7, 0xec),
            selection: color(0x1f, 0x5f, 0x8f),
            warn: color(0xe6, 0xa2, 0x3c),
            error: color(0xf5, 0x6c, 0x6c),
        }
    }
}

/// Returns the default compact dark editor style.
#[must_use]
pub fn default_editor_style() -> EditorStyle {
    EditorPalette::default().into()
}

impl From<EditorPalette> for EditorStyle {
    fn from(palette: EditorPalette) -> Self {
        Self::new(move |ctx| {
            ctx.set_theme(egui::Theme::Dark);

            let mut style = egui::Style::default();

            style.spacing.item_spacing = egui::vec2(6.0, 4.0);
            style.spacing.window_margin = egui::Margin::symmetric(10, 8);
            style.spacing.menu_margin = egui::Margin::symmetric(8, 6);
            style.spacing.button_padding = egui::vec2(8.0, 3.0);
            style.spacing.interact_size = egui::vec2(28.0, 22.0);
            style.spacing.slider_width = 120.0;
            style.spacing.combo_width = 140.0;
            style.spacing.text_edit_width = 260.0;

            style.text_styles.insert(
                egui::TextStyle::Small,
                egui::FontId::new(10.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(13.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Button,
                egui::FontId::new(13.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Monospace,
                egui::FontId::new(13.0, egui::FontFamily::Monospace),
            );
            style.text_styles.insert(
                egui::TextStyle::Heading,
                egui::FontId::new(18.0, egui::FontFamily::Proportional),
            );

            let mut visuals = egui::Visuals::dark();
            visuals.window_fill = palette.surface;
            visuals.panel_fill = palette.background;
            visuals.window_stroke = egui::Stroke::new(1.0, palette.stroke_subtle);
            visuals.window_corner_radius = egui::CornerRadius::same(4);
            visuals.menu_corner_radius = egui::CornerRadius::same(4);
            visuals.window_shadow = egui::Shadow {
                offset: [0, 8],
                blur: 18,
                spread: 0,
                color: egui::Color32::from_black_alpha(120),
            };
            visuals.popup_shadow = egui::Shadow {
                offset: [0, 6],
                blur: 14,
                spread: 0,
                color: egui::Color32::from_black_alpha(110),
            };
            visuals.selection.bg_fill = palette.selection;
            visuals.selection.stroke = egui::Stroke::new(1.0, palette.text_bright);
            visuals.hyperlink_color = palette.accent_hovered;
            visuals.warn_fg_color = palette.warn;
            visuals.error_fg_color = palette.error;
            visuals.weak_text_color = Some(palette.text_muted);
            visuals.striped = true;
            visuals.slider_trailing_fill = true;
            visuals.button_frame = true;
            visuals.indent_has_left_vline = true;
            visuals.handle_shape = egui::style::HandleShape::Rect { aspect_ratio: 0.6 };

            visuals.widgets.noninteractive = widget_visuals(
                palette.panel,
                palette.surface,
                palette.stroke_subtle,
                palette.text_muted,
            );
            visuals.widgets.inactive = widget_visuals(
                palette.control,
                palette.control,
                palette.stroke_subtle,
                palette.text,
            );
            visuals.widgets.hovered = widget_visuals(
                palette.control_hovered,
                palette.control_hovered,
                palette.stroke_strong,
                palette.text_bright,
            );
            visuals.widgets.active = widget_visuals(
                palette.accent,
                palette.accent,
                palette.accent_hovered,
                palette.text_bright,
            );
            visuals.widgets.open = widget_visuals(
                palette.surface_high,
                palette.surface_high,
                palette.accent,
                palette.text_bright,
            );

            style.visuals = visuals;

            ctx.set_style_of(egui::Theme::Dark, style.clone());
            ctx.set_style_of(egui::Theme::Light, style);
        })
    }
}

impl From<&EditorPalette> for EditorStyle {
    fn from(palette: &EditorPalette) -> Self {
        Self::from(*palette)
    }
}

fn widget_visuals(
    bg_fill: egui::Color32,
    weak_bg_fill: egui::Color32,
    bg_stroke: egui::Color32,
    fg_stroke: egui::Color32,
) -> egui::style::WidgetVisuals {
    egui::style::WidgetVisuals {
        bg_fill,
        weak_bg_fill,
        bg_stroke: egui::Stroke::new(1.0, bg_stroke),
        corner_radius: egui::CornerRadius::same(3),
        fg_stroke: egui::Stroke::new(1.0, fg_stroke),
        expansion: 0.0,
    }
}

fn color(r: u8, g: u8, b: u8) -> egui::Color32 {
    egui::Color32::from_rgb(r, g, b)
}
