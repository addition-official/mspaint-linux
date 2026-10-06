//! Windows 11 Paint look: Fluent-ish colours, rounded controls, desktop font.

use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Stroke, Visuals};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ThemePref {
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    /// Title bar and menu row.
    pub chrome: Color32,
    /// The toolbar ("ribbon") strip.
    pub ribbon: Color32,
    /// Window border when we draw our own title bar.
    pub border: Color32,
    /// Area around the canvas.
    pub workspace: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub separator: Color32,
    pub accent: Color32,
    pub hover: Color32,
    pub selected: Color32,
    pub status: Color32,
    pub canvas_shadow: Color32,
}

impl Palette {
    pub fn new(dark: bool) -> Self {
        if dark {
            Self {
                dark,
                chrome: Color32::from_rgb(0x20, 0x20, 0x20),
                ribbon: Color32::from_rgb(0x2B, 0x2B, 0x2B),
                border: Color32::from_rgb(0x45, 0x45, 0x45),
                workspace: Color32::from_rgb(0x16, 0x16, 0x16),
                text: Color32::from_rgb(0xF2, 0xF2, 0xF2),
                text_dim: Color32::from_rgb(0xA8, 0xA8, 0xA8),
                separator: Color32::from_rgb(0x3A, 0x3A, 0x3A),
                accent: Color32::from_rgb(0x60, 0xCD, 0xFF),
                hover: Color32::from_rgb(0x2D, 0x2D, 0x2D),
                selected: Color32::from_rgb(0x38, 0x38, 0x38),
                status: Color32::from_rgb(0x1C, 0x1C, 0x1C),
                canvas_shadow: Color32::from_black_alpha(140),
            }
        } else {
            Self {
                dark,
                chrome: Color32::from_rgb(0xEE, 0xF2, 0xF8),
                ribbon: Color32::from_rgb(0xF8, 0xF9, 0xFB),
                border: Color32::from_rgb(0xC8, 0xCC, 0xD2),
                workspace: Color32::from_rgb(0xE6, 0xEB, 0xF1),
                text: Color32::from_rgb(0x1A, 0x1A, 0x1A),
                text_dim: Color32::from_rgb(0x5F, 0x5F, 0x5F),
                separator: Color32::from_rgb(0xE0, 0xE0, 0xE0),
                accent: Color32::from_rgb(0x00, 0x5F, 0xB8),
                hover: Color32::from_rgb(0xEC, 0xEC, 0xEC),
                selected: Color32::from_rgb(0xE3, 0xE9, 0xF2),
                status: Color32::from_rgb(0xF3, 0xF3, 0xF3),
                canvas_shadow: Color32::from_black_alpha(40),
            }
        }
    }
}

pub fn apply(ctx: &egui::Context, pal: &Palette) {
    let mut v = if pal.dark { Visuals::dark() } else { Visuals::light() };
    let r4 = CornerRadius::same(4);
    v.window_corner_radius = CornerRadius::same(8);
    v.menu_corner_radius = CornerRadius::same(8);
    v.panel_fill = pal.chrome;
    v.window_fill = if pal.dark { Color32::from_rgb(0x2B, 0x2B, 0x2B) } else { Color32::from_rgb(0xFB, 0xFB, 0xFB) };
    v.extreme_bg_color = if pal.dark { Color32::from_rgb(0x1E, 0x1E, 0x1E) } else { Color32::WHITE };
    v.selection.bg_fill = pal.accent.gamma_multiply(if pal.dark { 0.45 } else { 0.25 });
    v.selection.stroke = Stroke::new(1.0, pal.accent);
    v.hyperlink_color = pal.accent;
    v.override_text_color = Some(pal.text);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = r4;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, pal.separator);
    v.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    v.widgets.inactive.bg_fill = if pal.dark { Color32::from_rgb(0x37, 0x37, 0x37) } else { Color32::from_rgb(0xE8, 0xE8, 0xE8) };
    v.widgets.hovered.weak_bg_fill = pal.hover;
    v.widgets.hovered.bg_fill = pal.hover;
    v.widgets.hovered.bg_stroke = Stroke::NONE;
    v.widgets.active.weak_bg_fill = pal.selected;
    v.widgets.active.bg_stroke = Stroke::NONE;
    v.widgets.open.weak_bg_fill = pal.selected;
    v.window_stroke = Stroke::new(1.0, pal.separator);
    ctx.set_visuals(v);
    ctx.all_styles_mut(|s| {
        s.spacing.scroll.fade.strength = 0.0;
        s.spacing.button_padding = egui::vec2(8.0, 4.0);
        s.spacing.item_spacing = egui::vec2(6.0, 4.0);
        s.spacing.menu_margin = egui::Margin::same(6);
        s.interaction.tooltip_delay = 0.4;
    });
}

/// Use the desktop's UI font (Noto Sans on Kubuntu) so the app blends in with Plasma.
pub fn install_fonts(ctx: &egui::Context, fm: &crate::text::FontManager) {
    let mut defs = FontDefinitions::default();
    for (fam, key) in [("Noto Sans", "ui"), ("Segoe UI", "ui"), ("Ubuntu", "ui"), ("DejaVu Sans", "ui")] {
        if !fm.families.iter().any(|f| f == fam) {
            continue;
        }
        if let Some((bytes, index)) = fm.font_bytes(fam, false) {
            let mut fd = FontData::from_owned(bytes);
            fd.index = index;
            defs.font_data.insert(key.into(), Arc::new(fd));
            defs.families.entry(FontFamily::Proportional).or_default().insert(0, key.into());
            break;
        }
    }
    ctx.set_fonts(defs);
}

/// Whether the KDE Plasma colour scheme is dark, read from `kdeglobals`.
/// (winit cannot report the system theme on X11, and only overrides on Wayland.)
pub fn kde_prefers_dark() -> Option<bool> {
    let cfg = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))?;
    let text = std::fs::read_to_string(cfg.join("kdeglobals")).ok()?;
    parse_kdeglobals(&text)
}

fn parse_kdeglobals(text: &str) -> Option<bool> {
    let mut section = "";
    let mut scheme_name = None;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            section = line;
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        match (section, k.trim()) {
            ("[Colors:Window]", "BackgroundNormal") => {
                let rgb: Vec<f32> = v.split(',').filter_map(|c| c.trim().parse().ok()).collect();
                if rgb.len() >= 3 {
                    let luma = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2];
                    return Some(luma < 128.0);
                }
            }
            ("[General]", "ColorScheme") => scheme_name = Some(v.to_ascii_lowercase()),
            _ => {}
        }
    }
    scheme_name.map(|n| n.contains("dark"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_breeze_dark() {
        let dark = "[General]\nColorScheme=BreezeDark\n\n[Colors:Window]\nBackgroundNormal=32,35,38\n";
        let light = "[Colors:Window]\nBackgroundNormal=239,240,241\n";
        assert_eq!(super::parse_kdeglobals(dark), Some(true));
        assert_eq!(super::parse_kdeglobals(light), Some(false));
        assert_eq!(super::parse_kdeglobals("[General]\nColorScheme=BreezeDark\n"), Some(true));
        assert_eq!(super::parse_kdeglobals(""), None);
    }
}
