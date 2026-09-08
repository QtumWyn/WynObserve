use std::sync::{OnceLock, RwLock};

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Visuals};

use crate::config::{ThemePreset, UiPreferences};

#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    pub bg: Color32,
    pub panel: Color32,
    pub panel_alt: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,

    /// Selection / darker primary surface.
    pub selection: Color32,

    /// Main highlight, historically the Observatory pink.
    pub primary: Color32,

    /// Secondary highlight, historically violet.
    pub secondary: Color32,

    /// Informational / memory / link highlight.
    pub info: Color32,

    /// Positive / observed / network highlight.
    pub success: Color32,

    /// Warning / thermal / inferred highlight.
    pub warning: Color32,

    /// High-contrast foreground.
    pub bright: Color32,
}

pub const UI_ZOOM: f32 = 1.10;

static CURRENT: OnceLock<RwLock<ThemePalette>> = OnceLock::new();

fn store() -> &'static RwLock<ThemePalette> {
    CURRENT.get_or_init(|| RwLock::new(gothic_palette()))
}

fn set_current(palette: ThemePalette) {
    let mut guard = store()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    *guard = palette;
}

pub fn current() -> ThemePalette {
    *store()
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub const fn gothic_palette() -> ThemePalette {
    ThemePalette {
        bg: Color32::from_rgb(9, 7, 13),
        panel: Color32::from_rgb(18, 14, 25),
        panel_alt: Color32::from_rgb(24, 17, 31),
        border: Color32::from_rgb(75, 53, 88),
        text: Color32::from_rgb(239, 234, 244),
        muted: Color32::from_rgb(158, 143, 169),

        selection: Color32::from_rgb(116, 38, 72),
        primary: Color32::from_rgb(244, 174, 204),
        secondary: Color32::from_rgb(132, 91, 180),
        info: Color32::from_rgb(162, 211, 255),
        success: Color32::from_rgb(147, 223, 181),
        warning: Color32::from_rgb(226, 193, 125),
        bright: Color32::from_rgb(248, 244, 250),
    }
}

pub const fn pastel_palette() -> ThemePalette {
    ThemePalette {
        bg: Color32::from_rgb(25, 22, 32),
        panel: Color32::from_rgb(38, 33, 47),
        panel_alt: Color32::from_rgb(48, 42, 58),
        border: Color32::from_rgb(108, 96, 125),
        text: Color32::from_rgb(247, 242, 250),
        muted: Color32::from_rgb(191, 179, 199),

        selection: Color32::from_rgb(153, 104, 145),
        primary: Color32::from_rgb(255, 185, 219),
        secondary: Color32::from_rgb(204, 181, 255),
        info: Color32::from_rgb(176, 222, 255),
        success: Color32::from_rgb(179, 235, 203),
        warning: Color32::from_rgb(248, 220, 166),
        bright: Color32::from_rgb(255, 250, 255),
    }
}

pub const fn cyberpunk_palette() -> ThemePalette {
    ThemePalette {
        bg: Color32::from_rgb(4, 8, 13),
        panel: Color32::from_rgb(8, 15, 22),
        panel_alt: Color32::from_rgb(13, 23, 32),
        border: Color32::from_rgb(31, 105, 125),
        text: Color32::from_rgb(226, 250, 252),
        muted: Color32::from_rgb(105, 157, 168),

        selection: Color32::from_rgb(113, 16, 76),
        primary: Color32::from_rgb(255, 48, 151),
        secondary: Color32::from_rgb(177, 68, 255),
        info: Color32::from_rgb(49, 237, 255),
        success: Color32::from_rgb(106, 255, 157),
        warning: Color32::from_rgb(255, 226, 72),
        bright: Color32::from_rgb(245, 255, 255),
    }
}

pub fn palette_for_preset(preset: ThemePreset) -> ThemePalette {
    match preset {
        ThemePreset::Gothic => gothic_palette(),
        ThemePreset::Pastel => pastel_palette(),
        ThemePreset::Cyberpunk => cyberpunk_palette(),
        ThemePreset::Custom => current(),
    }
}

pub fn palette_for(preferences: &UiPreferences) -> ThemePalette {
    match preferences.theme {
        ThemePreset::Gothic => gothic_palette(),
        ThemePreset::Pastel => pastel_palette(),
        ThemePreset::Cyberpunk => cyberpunk_palette(),
        ThemePreset::Custom => preferences.custom_theme,
    }
}

pub fn install(ctx: &egui::Context, preferences: &UiPreferences) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.set_zoom_factor(UI_ZOOM);

    apply(ctx, preferences);

    ctx.global_style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(9.0, 9.0);
        style.spacing.button_padding = egui::vec2(13.0, 8.0);

        style.text_styles.insert(
            TextStyle::Small,
            FontId::new(11.5, FontFamily::Proportional),
        );
        style
            .text_styles
            .insert(TextStyle::Body, FontId::new(15.0, FontFamily::Proportional));
        style.text_styles.insert(
            TextStyle::Monospace,
            FontId::new(14.0, FontFamily::Monospace),
        );
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(14.5, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(22.0, FontFamily::Proportional),
        );
    });
}

pub fn apply(ctx: &egui::Context, preferences: &UiPreferences) {
    let palette = palette_for(preferences);
    set_current(palette);

    let mut visuals = Visuals::dark();

    visuals.panel_fill = palette.bg;
    visuals.window_fill = palette.panel;
    visuals.extreme_bg_color = deep_bg();
    visuals.faint_bg_color = palette.panel_alt;
    visuals.override_text_color = Some(palette.text);

    visuals.widgets.noninteractive.bg_fill = palette.panel;
    visuals.widgets.noninteractive.weak_bg_fill = palette.panel;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.border);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);

    // Buttons and other inactive controls use `weak_bg_fill` for their normal
    // resting surface in egui. If we only theme `bg_fill`, many buttons retain
    // the library's default gray appearance.
    let button_rest = blend(palette.panel_alt, palette.primary, 0.10);

    visuals.widgets.inactive.bg_fill = button_rest;
    visuals.widgets.inactive.weak_bg_fill = button_rest;
    visuals.widgets.inactive.bg_stroke =
        Stroke::new(1.0, blend(palette.border, palette.primary, 0.38));
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette.text);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);

    let button_hover = blend(palette.panel_alt, palette.primary, 0.24);

    visuals.widgets.hovered.bg_fill = button_hover;
    visuals.widgets.hovered.weak_bg_fill = button_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.25, palette.secondary);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, palette.bright);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
    visuals.widgets.hovered.expansion = 1.0;

    let button_active = blend(palette.panel_alt, palette.primary, 0.38);

    visuals.widgets.active.bg_fill = button_active;
    visuals.widgets.active.weak_bg_fill = button_active;
    visuals.widgets.active.bg_stroke = Stroke::new(1.5, palette.primary);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, palette.bright);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);
    visuals.widgets.active.expansion = 0.5;

    // Selected buttons use the selection palette, so navigation tabs, playback
    // speed controls, and other toggles visibly belong to the active theme.
    visuals.selection.bg_fill = palette.selection;
    visuals.selection.stroke = Stroke::new(1.5, palette.primary);

    visuals.window_corner_radius = CornerRadius::same(8);

    ctx.set_visuals(visuals);
}

pub fn blend(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);

    let mix = |x: u8, y: u8| {
        (x as f32 + (y as f32 - x as f32) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };

    Color32::from_rgba_unmultiplied(
        mix(a.r(), b.r()),
        mix(a.g(), b.g()),
        mix(a.b(), b.b()),
        mix(a.a(), b.a()),
    )
}

pub fn bg() -> Color32 {
    current().bg
}

pub fn panel() -> Color32 {
    current().panel
}

pub fn panel_alt() -> Color32 {
    current().panel_alt
}

pub fn border() -> Color32 {
    current().border
}

pub fn text() -> Color32 {
    current().text
}

pub fn muted() -> Color32 {
    current().muted
}

pub fn wine() -> Color32 {
    current().selection
}

pub fn pink() -> Color32 {
    current().primary
}

pub fn violet() -> Color32 {
    current().secondary
}

pub fn blue() -> Color32 {
    current().info
}

pub fn green() -> Color32 {
    current().success
}

pub fn gold() -> Color32 {
    current().warning
}

pub fn white() -> Color32 {
    current().bright
}

pub fn deep_bg() -> Color32 {
    blend(bg(), Color32::BLACK, 0.44)
}

pub fn terminal_bg() -> Color32 {
    blend(bg(), Color32::BLACK, 0.58)
}

pub fn selected_panel() -> Color32 {
    blend(panel_alt(), pink(), 0.12)
}

pub fn cell_bg() -> Color32 {
    blend(panel(), bg(), 0.38)
}

pub fn track_bg() -> Color32 {
    blend(panel_alt(), bg(), 0.26)
}

pub fn grid_inactive() -> Color32 {
    blend(panel_alt(), bg(), 0.18)
}

pub fn truth_color(level: crate::model::TruthLevel) -> Color32 {
    use crate::model::TruthLevel;

    match level {
        TruthLevel::Observed => green(),
        TruthLevel::Sampled => blue(),
        TruthLevel::Inferred => gold(),
        TruthLevel::Schematic => muted(),
    }
}
