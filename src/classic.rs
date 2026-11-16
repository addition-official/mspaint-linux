//! The classic looks: Paint's ribbon as it was in Windows 7 (glassy title bar, blue
//! application button, soft blue gradients, orange highlights) and in Windows 10 (flat
//! white title bar, blue File tab, light blue highlights). Same tools and actions as
//! the Windows 11 look; only the window chrome, ribbon, status bar and icons change.

use crate::app::{Action, Drag, PaintApp, ZOOM_MAX, ZOOM_MIN, app_icon, human_size};
use crate::classic_icons::{self, CIcon};
use crate::document::Overlay;
use crate::ribbon::{PALETTE, PALETTE_NAMES};
use crate::theme::{Palette, Style};
use crate::tools::{BrushKind, SelectMode, ShapeKind, ShapeStyle, Tool, size_presets};
use egui::{Align2, Color32, FontId, PopupCloseBehavior, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, ViewportCommand, pos2, vec2};

pub const TITLE_H: f32 = 30.0;
const TABS_H: f32 = 24.0;
/// Ribbon body: buttons (CONTENT_H) and the group names under them.
const CONTENT_H: f32 = 68.0;
const LABEL_H: f32 = 18.0;
pub const RIBBON_H: f32 = TABS_H + CONTENT_H + LABEL_H + 6.0;
pub const STATUS_H: f32 = 25.0;
/// Space between the workspace corner and the picture, as in classic Paint.
pub const CANVAS_PAD: f32 = 5.0;

fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(pos2(x, y), vec2(w, h))
}

/// Colors of one classic look.
#[derive(Clone, Copy)]
pub struct Skin {
    pub win7: bool,
    title: [Color32; 2],
    title_text: Color32,
    tabs_bg: [Color32; 2],
    ribbon: [Color32; 2],
    ribbon_border: Color32,
    label: Color32,
    sep: Color32,
    text: Color32,
    hover: [Color32; 2],
    hover_border: Color32,
    press: [Color32; 2],
    press_border: Color32,
    sel: [Color32; 2],
    sel_border: Color32,
    app_btn: [Color32; 2],
    app_btn_hover: [Color32; 2],
    tab_border: Color32,
    status: [Color32; 2],
    status_sep: Color32,
    workspace: Color32,
    box_border: Color32,
}

impl Skin {
    pub fn of(style: Style) -> Skin {
        if style == Style::Win7 {
            Skin {
                win7: true,
                title: [rgb(0xDB, 0xE7, 0xF4), rgb(0xBE, 0xD2, 0xE9)],
                title_text: rgb(0x1E, 0x1E, 0x1E),
                tabs_bg: [rgb(0xD9, 0xE5, 0xF3), rgb(0xDF, 0xE9, 0xF5)],
                ribbon: [rgb(0xFA, 0xFC, 0xFE), rgb(0xDC, 0xE7, 0xF5)],
                ribbon_border: rgb(0x9B, 0xB0, 0xCB),
                label: rgb(0x3E, 0x58, 0x7C),
                sep: rgb(0xB9, 0xC9, 0xDE),
                text: rgb(0x1E, 0x39, 0x5B),
                hover: [rgb(0xFF, 0xFC, 0xE8), rgb(0xFF, 0xE7, 0x9E)],
                hover_border: rgb(0xDB, 0xC2, 0x79),
                press: [rgb(0xFB, 0xDB, 0x9A), rgb(0xFE, 0xC2, 0x5A)],
                press_border: rgb(0xC2, 0x96, 0x4A),
                sel: [rgb(0xFF, 0xE9, 0xA9), rgb(0xFF, 0xD2, 0x6B)],
                sel_border: rgb(0xC2, 0x96, 0x4A),
                app_btn: [rgb(0x4C, 0x86, 0xD2), rgb(0x1D, 0x52, 0xA3)],
                app_btn_hover: [rgb(0x6A, 0xA0, 0xE6), rgb(0x2A, 0x66, 0xBC)],
                tab_border: rgb(0x9B, 0xB0, 0xCB),
                status: [rgb(0xEE, 0xF3, 0xFA), rgb(0xD5, 0xE0, 0xEF)],
                status_sep: rgb(0xA9, 0xBC, 0xD5),
                workspace: rgb(0xC8, 0xD3, 0xE4),
                box_border: rgb(0x9B, 0xB0, 0xCB),
            }
        } else {
            Skin {
                win7: false,
                title: [rgb(0xFF, 0xFF, 0xFF), rgb(0xFF, 0xFF, 0xFF)],
                title_text: rgb(0x00, 0x00, 0x00),
                tabs_bg: [rgb(0xFF, 0xFF, 0xFF), rgb(0xFF, 0xFF, 0xFF)],
                ribbon: [rgb(0xF5, 0xF6, 0xF7), rgb(0xF5, 0xF6, 0xF7)],
                ribbon_border: rgb(0xDA, 0xDB, 0xDC),
                label: rgb(0x60, 0x60, 0x60),
                sep: rgb(0xE2, 0xE3, 0xE4),
                text: rgb(0x26, 0x26, 0x26),
                // Light blue on hover, a clearly deeper blue when selected.
                hover: [rgb(0xE5, 0xF3, 0xFF), rgb(0xE5, 0xF3, 0xFF)],
                hover_border: rgb(0xCC, 0xE8, 0xFF),
                press: [rgb(0xC9, 0xE0, 0xF7), rgb(0xC9, 0xE0, 0xF7)],
                press_border: rgb(0x62, 0xA2, 0xE4),
                sel: [rgb(0xCC, 0xE8, 0xFF), rgb(0xCC, 0xE8, 0xFF)],
                sel_border: rgb(0x62, 0xA2, 0xE4),
                app_btn: [rgb(0x19, 0x79, 0xCA), rgb(0x19, 0x79, 0xCA)],
                app_btn_hover: [rgb(0x29, 0x8B, 0xDA), rgb(0x29, 0x8B, 0xDA)],
                tab_border: rgb(0xDA, 0xDB, 0xDC),
                status: [rgb(0xF0, 0xF0, 0xF0), rgb(0xF0, 0xF0, 0xF0)],
                status_sep: rgb(0xD0, 0xD0, 0xD0),
                workspace: rgb(0xC8, 0xD1, 0xE1),
                box_border: rgb(0xC6, 0xC6, 0xC6),
            }
        }
    }

    /// The skin for a `Palette::classic` code (7, else 10).
    pub fn from_code(code: u8) -> Skin {
        Skin::of(if code == 7 { Style::Win7 } else { Style::Win10 })
    }

    /// The parts of the shared palette the classic looks change (dialogs and menus
    /// keep the rest).
    pub fn adjust(&self, pal: &mut Palette) {
        pal.workspace = self.workspace;
        pal.ribbon = self.ribbon[1];
        pal.status = self.status[1];
        pal.canvas_shadow = Color32::TRANSPARENT;
        pal.classic = if self.win7 { 7 } else { 10 };
        // Dialogs, menus and the Settings page sit on the classic light gray.
        pal.chrome = rgb(0xF0, 0xF0, 0xF0);
        pal.text = rgb(0x1E, 0x1E, 0x1E);
        pal.text_dim = rgb(0x5A, 0x5A, 0x5A);
        pal.separator = rgb(0xD5, 0xD5, 0xD5);
        pal.accent = if self.win7 { rgb(0x33, 0x99, 0xFF) } else { rgb(0x00, 0x78, 0xD7) };
        // Menus and lists light up like the ribbon buttons.
        pal.hover = self.hover[1];
        pal.pressed = self.press[1];
        pal.selected = self.sel[1];
        pal.selected_border = self.sel_border;
    }

    /// Classic menus, tooltips, text boxes, combo boxes, push buttons and scroll bars.
    pub fn adjust_visuals(&self, ctx: &egui::Context) {
        let win7 = self.win7;
        let radius = egui::CornerRadius::same(if win7 { 2 } else { 0 });
        ctx.all_styles_mut(|s| {
            let v = &mut s.visuals;
            v.menu_corner_radius = radius;
            v.window_corner_radius = radius;
            v.window_fill = if win7 { rgb(0xF5, 0xF6, 0xF7) } else { rgb(0xF2, 0xF2, 0xF2) };
            v.panel_fill = rgb(0xF0, 0xF0, 0xF0);
            v.window_stroke = Stroke::new(1.0, if win7 { rgb(0x97, 0x97, 0x97) } else { rgb(0xA0, 0xA0, 0xA0) });
            let shadow = egui::Shadow { offset: [2, 2], blur: 4, spread: 0, color: Color32::from_black_alpha(45) };
            v.popup_shadow = shadow;
            v.window_shadow = egui::Shadow { offset: [3, 3], blur: 10, spread: 0, color: Color32::from_black_alpha(60) };
            v.extreme_bg_color = Color32::WHITE;
            v.selection.bg_fill = if win7 { rgb(0x33, 0x99, 0xFF) } else { rgb(0x00, 0x78, 0xD7) };
            v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
            v.override_text_color = Some(rgb(0x1E, 0x1E, 0x1E));
            // Push buttons, text boxes and combo boxes: gray with a darker border; blue
            // on hover and while pressed.
            let (rest, rest_border, hover, hover_border, press, press_border) = if win7 {
                (rgb(0xEC, 0xEC, 0xEC), rgb(0x70, 0x70, 0x70), rgb(0xDC, 0xEE, 0xFB), rgb(0x3C, 0x7F, 0xB1), rgb(0xC4, 0xE5, 0xF6), rgb(0x2C, 0x62, 0x8B))
            } else {
                (rgb(0xE1, 0xE1, 0xE1), rgb(0xAD, 0xAD, 0xAD), rgb(0xE5, 0xF1, 0xFB), rgb(0x00, 0x78, 0xD7), rgb(0xCC, 0xE4, 0xF7), rgb(0x00, 0x54, 0x99))
            };
            let w = &mut v.widgets;
            for (state, fill, border) in [
                (&mut w.inactive, rest, rest_border),
                (&mut w.hovered, hover, hover_border),
                (&mut w.active, press, press_border),
                (&mut w.open, press, press_border),
            ] {
                state.weak_bg_fill = fill;
                state.bg_fill = fill;
                state.bg_stroke = Stroke::new(1.0, border);
                state.corner_radius = radius;
                state.fg_stroke.color = rgb(0x1E, 0x1E, 0x1E);
                state.expansion = 0.0;
            }
            w.noninteractive.corner_radius = radius;
            w.noninteractive.bg_stroke = Stroke::new(1.0, rgb(0xD5, 0xD5, 0xD5));
            // Solid scroll bars, like Windows 7 and 10 (not the overlay ones).
            s.spacing.scroll = egui::style::ScrollStyle::solid();
            s.spacing.scroll.bar_width = 12.0;
            s.spacing.scroll.foreground_color = false;
            // Content doesn't fade out under the edges, as in Windows.
            s.spacing.scroll.fade.strength = 0.0;
        });
    }

    pub fn btn_bg(&self, p: &egui::Painter, rect: Rect, hovered: bool, pressed: bool, selected: bool) {
        let (fill, border) = if pressed {
            (self.press, self.press_border)
        } else if selected {
            (if hovered { self.press } else { self.sel }, self.sel_border)
        } else if hovered {
            (self.hover, self.hover_border)
        } else {
            return;
        };
        let radius = if self.win7 { 2.5 } else { 0.0 };
        gradient(p, rect, fill[0], fill[1], radius);
        p.rect_stroke(rect, radius, Stroke::new(1.0, border), StrokeKind::Inside);
    }
}

/// A vertical gradient rectangle (rounded corners are approximated by the outline).
fn gradient(p: &egui::Painter, rect: Rect, top: Color32, bottom: Color32, radius: f32) {
    if top == bottom {
        p.rect_filled(rect, radius, top);
        return;
    }
    let inset = rect.shrink2(vec2(radius.min(2.0) * 0.5, 0.0));
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(inset.left_top(), top);
    mesh.colored_vertex(inset.right_top(), top);
    mesh.colored_vertex(inset.right_bottom(), bottom);
    mesh.colored_vertex(inset.left_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(egui::Shape::mesh(mesh));
}

/// The small down arrow of drop-down buttons.
fn arrow(p: &egui::Painter, c: Pos2, color: Color32) {
    p.add(egui::Shape::convex_polygon(vec![c + vec2(-3.0, -1.5), c + vec2(3.0, -1.5), c + vec2(0.0, 1.5)], color, Stroke::NONE));
}

impl PaintApp {
    pub fn skin(&self) -> Skin {
        Skin::of(self.style)
    }

    fn font(&self) -> FontId {
        FontId::proportional(12.0)
    }

    // ---------------------------------------------------------------------
    // Buttons

    /// Large button: 32px icon on top, the label (one or two lines) underneath, and a
    /// drop-down arrow under the label when `dropdown`.
    #[allow(clippy::too_many_arguments)]
    fn big_button(&self, ui: &mut Ui, rect: Rect, key: &str, icon: CIcon, label: &str, dropdown: bool, selected: bool, enabled: bool) -> Response {
        let sense = if enabled { self.click_sense(ui) } else { Sense::hover() };
        let resp = ui.interact(rect, ui.id().with(key), sense);
        let skin = self.skin();
        skin.btn_bg(ui.painter(), rect, enabled && resp.hovered(), enabled && resp.is_pointer_button_down_on(), selected);
        self.big_button_face(ui, rect, icon, label, dropdown, enabled);
        resp
    }

    fn big_button_face(&self, ui: &Ui, rect: Rect, icon: CIcon, label: &str, dropdown: bool, enabled: bool) {
        self.big_button_face_custom(ui, rect, icon, None, label, dropdown, enabled);
    }

    /// `custom`: a custom icon file to prefer over `icon` (the current brush's).
    #[allow(clippy::too_many_arguments)]
    fn big_button_face_custom(&self, ui: &Ui, rect: Rect, icon: CIcon, custom: Option<&'static str>, label: &str, dropdown: bool, enabled: bool) {
        let skin = self.skin();
        let r32 = Rect::from_center_size(pos2(rect.center().x, rect.min.y + 20.0), vec2(32.0, 32.0));
        if !custom.is_some_and(|name| classic_icons::paint_custom(ui.painter(), r32, name, enabled)) {
            classic_icons::paint(ui.painter(), r32, icon, enabled);
        }
        let color = if enabled { skin.text } else { skin.text.gamma_multiply(0.45) };
        let lines: Vec<&str> = label.split('\n').collect();
        for (i, line) in lines.iter().enumerate() {
            ui.painter().text(pos2(rect.center().x, rect.min.y + 44.0 + i as f32 * 13.0), Align2::CENTER_CENTER, *line, self.font(), color);
        }
        if dropdown {
            arrow(ui.painter(), pos2(rect.center().x, rect.min.y + 44.0 + lines.len() as f32 * 13.0 - 1.0), color);
        }
    }

    /// Large split button: the icon part does the action, the label part (with the
    /// arrow) opens the menu. Returns (main, menu).
    fn big_split(&self, ui: &mut Ui, rect: Rect, key: &str, icon: CIcon, label: &str, selected: bool) -> (Response, Response) {
        self.big_split_custom(ui, rect, key, icon, None, label, selected)
    }

    #[allow(clippy::too_many_arguments)]
    fn big_split_custom(&self, ui: &mut Ui, rect: Rect, key: &str, icon: CIcon, custom: Option<&'static str>, label: &str, selected: bool) -> (Response, Response) {
        let top = Rect::from_min_max(rect.min, pos2(rect.max.x, rect.min.y + 37.0));
        let bottom = Rect::from_min_max(pos2(rect.min.x, top.max.y), rect.max);
        let main = ui.interact(top, ui.id().with((key, "main")), self.click_sense(ui));
        let menu = ui.interact(bottom, ui.id().with((key, "menu")), self.click_sense(ui));
        let skin = self.skin();
        let p = ui.painter();
        if main.hovered() || menu.hovered() || selected {
            // The whole button lights up; the part under the pointer more.
            skin.btn_bg(p, rect, true, false, selected);
            let (hot, resp) = if main.hovered() { (top, &main) } else { (bottom, &menu) };
            if resp.hovered() {
                skin.btn_bg(p, hot, true, resp.is_pointer_button_down_on(), selected);
            }
        }
        self.big_button_face_custom(ui, rect, icon, custom, label, true, true);
        (main, menu)
    }

    /// Small button: 16px icon and a label to its right (and an arrow for drop-downs).
    #[allow(clippy::too_many_arguments)]
    fn small_button(&self, ui: &mut Ui, rect: Rect, key: &str, icon: Option<CIcon>, label: &str, dropdown: bool, selected: bool, enabled: bool) -> Response {
        let sense = if enabled { self.click_sense(ui) } else { Sense::hover() };
        let resp = ui.interact(rect, ui.id().with(key), sense);
        let skin = self.skin();
        skin.btn_bg(ui.painter(), rect, enabled && resp.hovered(), enabled && resp.is_pointer_button_down_on(), selected);
        let mut x = rect.min.x + 3.0;
        if let Some(icon) = icon {
            classic_icons::paint(ui.painter(), Rect::from_min_size(pos2(x, rect.center().y - 8.0), vec2(16.0, 16.0)), icon, enabled);
            x += 20.0;
        }
        let color = if enabled { skin.text } else { skin.text.gamma_multiply(0.45) };
        let g = ui.painter().layout_no_wrap(label.to_string(), self.font(), color);
        let w = g.size().x;
        ui.painter().galley(pos2(x, rect.center().y - g.size().y / 2.0), g, color);
        if dropdown {
            arrow(ui.painter(), pos2(x + w + 7.0, rect.center().y + 0.5), color);
        }
        resp
    }

    /// Icon-only square button (the Tools grid, the quick access toolbar).
    fn icon_btn(&self, ui: &mut Ui, rect: Rect, key: &str, icon: CIcon, selected: bool, enabled: bool) -> Response {
        let sense = if enabled { self.click_sense(ui) } else { Sense::hover() };
        let resp = ui.interact(rect, ui.id().with(key), sense);
        self.skin().btn_bg(ui.painter(), rect, enabled && resp.hovered(), enabled && resp.is_pointer_button_down_on(), selected);
        classic_icons::paint(ui.painter(), Rect::from_center_size(rect.center(), vec2(16.0, 16.0)), icon, enabled);
        resp
    }

    /// Reserve a ribbon group: `add` gets the content rect; the name goes underneath
    /// and a separator after it.
    fn cgroup(&mut self, ui: &mut Ui, label: &str, width: f32, add: impl FnOnce(&mut Self, &mut Ui, Rect)) {
        let (rect, _) = ui.allocate_exact_size(vec2(width, CONTENT_H + LABEL_H), Sense::hover());
        add(self, ui, Rect::from_min_size(rect.min, vec2(width, CONTENT_H)));
        let skin = self.skin();
        ui.painter().text(pos2(rect.center().x, rect.max.y - LABEL_H / 2.0), Align2::CENTER_CENTER, label, self.font(), skin.label);
        let (sep, _) = ui.allocate_exact_size(vec2(9.0, CONTENT_H + LABEL_H), Sense::hover());
        let x = sep.center().x;
        ui.painter().line_segment([pos2(x, sep.min.y + 2.0), pos2(x, sep.max.y - 2.0)], Stroke::new(1.0, skin.sep));
        if skin.win7 {
            ui.painter().line_segment([pos2(x + 1.0, sep.min.y + 2.0), pos2(x + 1.0, sep.max.y - 2.0)], Stroke::new(1.0, Color32::from_white_alpha(160)));
        }
    }

    // ---------------------------------------------------------------------
    // Title bar

    pub fn classic_title_bar(&mut self, ui: &mut Ui) {
        let skin = self.skin();
        let ctx = ui.ctx().clone();
        let rect = ui.max_rect();
        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        gradient(ui.painter(), rect, skin.title[0], skin.title[1], 0.0);
        if !skin.win7 {
            ui.painter().line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.0, rgb(0xE6, 0xE6, 0xE6)));
        }
        let bar = ui.interact(rect, ui.id().with("titlebar"), Sense::click_and_drag());
        self.window_grab(&ctx, &bar, maximized);
        let icon_rect = Rect::from_center_size(pos2(rect.min.x + 16.0, rect.center().y), vec2(16.0, 16.0));
        if !classic_icons::paint_custom(ui.painter(), icon_rect, "app-icon", true) {
            app_icon(ui.painter(), icon_rect);
        }
        // Quick access toolbar: Save, Undo, Redo.
        let mut x = rect.min.x + 30.0;
        if !skin.win7 {
            ui.painter().line_segment([pos2(x - 2.0, rect.min.y + 8.0), pos2(x - 2.0, rect.max.y - 8.0)], Stroke::new(1.0, rgb(0xC8, 0xC8, 0xC8)));
            x += 2.0;
        }
        for (icon, tip, action, enabled) in [
            (CIcon::Save, "Save (Ctrl+S)", Action::Save, true),
            (CIcon::Undo, "Undo (Ctrl+Z)", Action::Undo, self.doc.can_undo()),
            (CIcon::Redo, "Redo (Ctrl+Y)", Action::Redo, self.doc.can_redo()),
        ] {
            let b = r(x, rect.center().y - 11.0, 22.0, 22.0);
            self.no_drag.push(b);
            if self.icon_btn(ui, b, tip, icon, false, enabled).on_hover_text(tip).clicked() {
                self.push(action);
            }
            x += 23.0;
        }
        if !skin.win7 {
            ui.painter().line_segment([pos2(x + 3.0, rect.min.y + 8.0), pos2(x + 3.0, rect.max.y - 8.0)], Stroke::new(1.0, rgb(0xC8, 0xC8, 0xC8)));
        }
        let title = self.title();
        if skin.win7 {
            // Aero: dark text with a soft light glow behind it.
            let g = ui.painter().layout_no_wrap(title.clone(), FontId::proportional(12.5), skin.title_text);
            // Several faint, growing layers fake a blur.
            for i in 0..6 {
                let grow = 4.0 + i as f32 * 3.0;
                let tr = Rect::from_center_size(rect.center(), g.size() + vec2(grow * 2.0, grow));
                ui.painter().rect_filled(tr, grow, Color32::from_white_alpha(14));
            }
        }
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, title, FontId::proportional(12.5), skin.title_text);
        if skin.win7 {
            self.win7_captions(ui, rect, maximized);
        } else {
            self.win10_captions(ui, rect, maximized);
        }
    }

    fn caption_action(&self, ctx: &egui::Context, i: usize, maximized: bool) {
        match i {
            0 => ctx.send_viewport_cmd(ViewportCommand::Close),
            1 => ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized)),
            _ => ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
        }
    }

    /// Windows 10: flat 46 px buttons with thin glyphs; Close turns red on hover.
    fn win10_captions(&mut self, ui: &mut Ui, rect: Rect, maximized: bool) {
        let ctx = ui.ctx().clone();
        let bw = 46.0;
        for i in 0..3 {
            let br = r(rect.max.x - bw * (i + 1) as f32, rect.min.y, bw, rect.height() - 1.0);
            self.no_drag.push(br);
            let resp = ui.interact(br, ui.id().with(("ccap", i)), Sense::click());
            let (hover, press) = (resp.hovered(), resp.is_pointer_button_down_on());
            if hover {
                let fill = match (i, press) {
                    (0, false) => rgb(0xE8, 0x11, 0x23),
                    (0, true) => rgb(0xF1, 0x70, 0x7A),
                    (_, false) => rgb(0xE5, 0xE5, 0xE5),
                    (_, true) => rgb(0xCC, 0xCC, 0xCC),
                };
                ui.painter().rect_filled(br, 0.0, fill);
            }
            let fg = if i == 0 && hover { Color32::WHITE } else { Color32::BLACK };
            let c = br.center();
            let s = Stroke::new(1.0, fg);
            let p = ui.painter();
            match i {
                0 => {
                    p.line_segment([c + vec2(-5.0, -5.0), c + vec2(5.0, 5.0)], s);
                    p.line_segment([c + vec2(5.0, -5.0), c + vec2(-5.0, 5.0)], s);
                }
                1 if maximized => {
                    p.rect_stroke(Rect::from_min_size(c + vec2(-5.0, -3.0), vec2(8.0, 8.0)), 0.0, s, StrokeKind::Inside);
                    p.line_segment([c + vec2(-3.0, -3.0), c + vec2(-3.0, -5.0)], s);
                    p.line_segment([c + vec2(-3.0, -5.0), c + vec2(5.0, -5.0)], s);
                    p.line_segment([c + vec2(5.0, -5.0), c + vec2(5.0, 3.0)], s);
                    p.line_segment([c + vec2(5.0, 3.0), c + vec2(3.0, 3.0)], s);
                }
                1 => {
                    p.rect_stroke(Rect::from_center_size(c, vec2(10.0, 10.0)), 0.0, s, StrokeKind::Inside);
                }
                _ => {
                    p.line_segment([c + vec2(-5.0, 0.0), c + vec2(5.0, 0.0)], s);
                }
            }
            let tip = ["Close", if maximized { "Restore Down" } else { "Maximize" }, "Minimize"][i];
            if resp.on_hover_text(tip).clicked() {
                self.caption_action(&ctx, i, maximized);
            }
        }
    }

    /// Windows 7: glassy rounded buttons hanging from the top edge; Close is red.
    fn win7_captions(&mut self, ui: &mut Ui, rect: Rect, maximized: bool) {
        let ctx = ui.ctx().clone();
        let h = 19.0;
        let widths = [44.0, 26.0, 27.0];
        let mut right = rect.max.x - if maximized { 2.0 } else { 6.0 };
        for i in 0..3 {
            let br = r(right - widths[i], rect.min.y, widths[i], h);
            self.no_drag.push(br);
            right -= widths[i];
            let resp = ui.interact(br, ui.id().with(("ccap7", i)), Sense::click());
            let (hover, press) = (resp.hovered(), resp.is_pointer_button_down_on());
            let (top, bottom, border) = match (i == 0, hover, press) {
                (true, _, true) => (rgb(0xC8, 0x6A, 0x58), rgb(0x8E, 0x22, 0x10), rgb(0x52, 0x18, 0x10)),
                (true, true, _) => (rgb(0xF2, 0x9A, 0x86), rgb(0xD4, 0x38, 0x1C), rgb(0x6A, 0x1C, 0x10)),
                (true, false, _) => (rgb(0xE4, 0x8C, 0x7A), rgb(0xC1, 0x3A, 0x22), rgb(0x6A, 0x2C, 0x22)),
                (false, _, true) => (rgb(0x9E, 0xC0, 0xE4), rgb(0x3E, 0x6E, 0xA8), rgb(0x2A, 0x3A, 0x55)),
                (false, true, _) => (rgb(0xC6, 0xE2, 0xFF), rgb(0x6A, 0x9C, 0xD6), rgb(0x3A, 0x4A, 0x66)),
                (false, false, _) => (rgb(0xE6, 0xEE, 0xF8), rgb(0xB4, 0xC8, 0xE0), rgb(0x6C, 0x7C, 0x96)),
            };
            let p = ui.painter();
            let shape = egui::CornerRadius { nw: 0, ne: 0, sw: if i == 2 { 4 } else { 0 }, se: if i == 0 { 4 } else { 0 } };
            p.rect_filled(br, shape, bottom);
            gradient(p, Rect::from_min_max(br.min, pos2(br.max.x, br.center().y)), top, top.lerp_to_gamma(bottom, 0.5), 0.0);
            p.rect_stroke(br, shape, Stroke::new(1.0, border), StrokeKind::Inside);
            // White glyphs with a dark edge, as on Aero buttons.
            let c = br.center();
            let glyph = |w: f32, color: Color32| {
                let s = Stroke::new(w, color);
                match i {
                    0 => {
                        p.line_segment([c + vec2(-4.0, -3.5), c + vec2(4.0, 3.5)], s);
                        p.line_segment([c + vec2(4.0, -3.5), c + vec2(-4.0, 3.5)], s);
                    }
                    1 if maximized => {
                        p.rect_stroke(Rect::from_min_size(c + vec2(-5.0, -2.0), vec2(7.0, 6.0)), 0.0, s, StrokeKind::Middle);
                        p.rect_stroke(Rect::from_min_size(c + vec2(-2.0, -5.0), vec2(7.0, 6.0)), 0.0, s, StrokeKind::Middle);
                    }
                    1 => {
                        p.rect_stroke(Rect::from_center_size(c, vec2(10.0, 7.0)), 0.0, s, StrokeKind::Middle);
                    }
                    _ => {
                        p.line_segment([c + vec2(-4.0, 3.0), c + vec2(4.0, 3.0)], s);
                    }
                }
            };
            glyph(3.2, rgb(0x2A, 0x2A, 0x3A).gamma_multiply(0.7));
            glyph(1.7, Color32::WHITE);
            let tip = ["Close", if maximized { "Restore Down" } else { "Maximize" }, "Minimize"][i];
            if resp.on_hover_text(tip).clicked() {
                self.caption_action(&ctx, i, maximized);
            }
        }
    }

    // ---------------------------------------------------------------------
    // Ribbon

    pub fn classic_ribbon(&mut self, ui: &mut Ui) {
        let skin = self.skin();
        let full = ui.max_rect();
        let text_active = matches!(self.doc.overlay, Overlay::Text(_));
        // The Text tab appears (and opens) while a text box is being edited, and goes
        // away with it, as in Paint.
        if text_active && self.classic_tab != 2 && !self.classic_text_seen {
            self.classic_tab = 2;
        }
        self.classic_text_seen = text_active;
        if !text_active && self.classic_tab == 2 {
            self.classic_tab = 0;
        }

        // Tab row. Its empty part looks like the title bar, so it moves the window
        // too (the tabs and buttons drawn on top of it take their own clicks).
        let tabs = Rect::from_min_size(full.min, vec2(full.width(), TABS_H));
        if !self.system_titlebar {
            let ctx = ui.ctx().clone();
            let grab = ui.interact(tabs, ui.id().with("tabrow_drag"), Sense::click_and_drag());
            let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
            self.window_grab(&ctx, &grab, maximized);
        }
        gradient(ui.painter(), tabs, skin.tabs_bg[0], skin.tabs_bg[1], 0.0);
        let body = Rect::from_min_max(pos2(full.min.x, tabs.max.y), full.max);
        let p = ui.painter();
        gradient(p, body, skin.ribbon[0], skin.ribbon[1], 0.0);
        p.line_segment([body.left_bottom() - vec2(0.0, 0.5), body.right_bottom() - vec2(0.0, 0.5)], Stroke::new(1.0, skin.ribbon_border));

        // Application menu button (Windows 7) / File tab (Windows 10).
        let app_rect = if skin.win7 { r(tabs.min.x + 2.0, tabs.min.y + 1.0, 54.0, TABS_H - 1.0) } else { r(tabs.min.x, tabs.min.y, 56.0, TABS_H) };
        self.no_drag.push(app_rect);
        let app_resp = ui.interact(app_rect, ui.id().with("app_menu_btn"), self.click_sense(ui));
        let fill = if app_resp.hovered() { skin.app_btn_hover } else { skin.app_btn };
        let radius = if skin.win7 { egui::CornerRadius { nw: 3, ne: 3, sw: 0, se: 0 } } else { egui::CornerRadius::ZERO };
        ui.painter().rect_filled(app_rect, radius, fill[1]);
        gradient(ui.painter(), Rect::from_min_max(app_rect.min + vec2(1.0, 1.0), pos2(app_rect.max.x - 1.0, app_rect.center().y)), fill[0], fill[0].lerp_to_gamma(fill[1], 0.6), 0.0);
        if skin.win7 {
            classic_icons::paint(ui.painter(), Rect::from_center_size(app_rect.center() - vec2(7.0, 0.0), vec2(16.0, 16.0)), CIcon::AppMenu, true);
            arrow(ui.painter(), app_rect.center() + vec2(11.0, 0.5), Color32::WHITE);
        } else {
            ui.painter().text(app_rect.center(), Align2::CENTER_CENTER, "File", self.font(), Color32::WHITE);
        }
        if app_resp.clicked() {
            self.app_menu_pane = 0;
        }
        // Alt+F opens the File menu; Alt+H and Alt+V switch tabs, as in Paint.
        let ctx = ui.ctx().clone();
        let alt = |key| ctx.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::ALT, key)));
        if alt(egui::Key::F) {
            self.app_menu_pane = 0;
            egui::Popup::open_id(&ctx, egui::Popup::default_response_id(&app_resp));
        }
        if alt(egui::Key::H) {
            self.classic_tab = 0;
        }
        if alt(egui::Key::V) {
            self.classic_tab = 1;
        }
        // Rows close it themselves; clicks on headers or empty space keep it open.
        egui::Popup::menu(&app_resp).width(480.0).close_behavior(PopupCloseBehavior::CloseOnClickOutside).show(|ui| self.classic_app_menu(ui));

        // Home, View (and Text while typing).
        let mut x = app_rect.max.x + 4.0;
        let names: &[&str] = if text_active { &["Home", "View", "Text"] } else { &["Home", "View"] };
        for (i, name) in names.iter().enumerate() {
            let g = ui.painter().layout_no_wrap(name.to_string(), self.font(), skin.text);
            let tab = r(x, tabs.min.y + 2.0, g.size().x + 26.0, TABS_H - 1.0);
            self.no_drag.push(tab);
            let resp = ui.interact(tab, ui.id().with(("ctab", i)), self.click_sense(ui));
            let selected = self.classic_tab as usize == i;
            let p = ui.painter();
            if selected {
                let top = if skin.win7 { skin.ribbon[0] } else { Color32::WHITE };
                p.rect_filled(tab.expand2(vec2(0.0, 1.0)), egui::CornerRadius { nw: 3, ne: 3, sw: 0, se: 0 }, top);
                let s = Stroke::new(1.0, skin.tab_border);
                p.line_segment([tab.left_bottom() + vec2(0.0, 1.0), tab.left_top() + vec2(0.0, 2.0)], s);
                p.line_segment([tab.left_top() + vec2(2.0, 0.0), tab.right_top() - vec2(2.0, 0.0)], s);
                p.line_segment([tab.right_top() + vec2(0.0, 2.0), tab.right_bottom() + vec2(0.0, 1.0)], s);
            } else if resp.hovered() {
                p.rect_filled(tab, 3.0, Color32::from_white_alpha(if skin.win7 { 120 } else { 0 }));
                p.rect_stroke(tab, 3.0, Stroke::new(1.0, skin.tab_border.gamma_multiply(0.6)), StrokeKind::Inside);
            }
            let color = if *name == "Text" && skin.win7 { rgb(0x8A, 0x5A, 0x00) } else { skin.text };
            p.galley(tab.center() - g.size() / 2.0 + vec2(0.0, 1.0), g, color);
            if resp.clicked() {
                self.classic_tab = i as u8;
            }
            x = tab.max.x + 2.0;
        }
        // With the system title bar, the quick access buttons move down here (Paint's
        // "Show below the ribbon" puts them in a row of their own; beside the tabs keeps
        // the window shorter).
        if self.system_titlebar {
            x += 8.0;
            for (icon, tip, action, enabled) in [
                (CIcon::Save, "Save (Ctrl+S)", Action::Save, true),
                (CIcon::Undo, "Undo (Ctrl+Z)", Action::Undo, self.doc.can_undo()),
                (CIcon::Redo, "Redo (Ctrl+Y)", Action::Redo, self.doc.can_redo()),
            ] {
                let b = r(x, tabs.min.y + 1.0, 22.0, 22.0);
                if self.icon_btn(ui, b, tip, icon, false, enabled).on_hover_text(tip).clicked() {
                    self.push(action);
                }
                x += 23.0;
            }
        }
        // Settings, where Windows had its help button.
        let help = r(tabs.max.x - 26.0, tabs.min.y + 3.0, 20.0, 18.0);
        self.no_drag.push(help);
        let hr = ui.interact(help, ui.id().with("chelp"), self.click_sense(ui));
        self.skin().btn_bg(ui.painter(), help, hr.hovered(), hr.is_pointer_button_down_on(), false);
        ui.painter().circle(help.center(), 7.0, rgb(0x2B, 0x6C, 0xC4), Stroke::new(1.0, rgb(0x1F, 0x4E, 0x96)));
        ui.painter().text(help.center() + vec2(0.0, 0.5), Align2::CENTER_CENTER, "?", FontId::proportional(11.0), Color32::WHITE);
        if hr.on_hover_text("Settings and About").clicked() {
            self.drag = None;
            self.doc.commit_overlay(&self.s);
            self.show_settings = true;
        }

        // The groups of the open tab.
        let content = Rect::from_min_max(body.min + vec2(4.0, 3.0), body.max - vec2(4.0, 3.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(content).layout(egui::Layout::left_to_right(egui::Align::Min)));
        egui::ScrollArea::horizontal()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(&mut child, |ui| {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    match self.classic_tab {
                        1 => self.view_tab(ui),
                        2 => self.text_tab(ui),
                        _ => self.home_tab(ui),
                    }
                });
            });
        ui.allocate_rect(full, Sense::hover());
    }

    /// The classic File menu: big commands on the left; on the right the recent
    /// pictures, or the choices of the command under the mouse (Save as, Set as
    /// desktop background).
    fn classic_app_menu(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(240.0);
                let items: [(CIcon, &str, u8); 10] = [
                    (CIcon::New, "New", 0),
                    (CIcon::Open, "Open", 0),
                    (CIcon::Save, "Save", 0),
                    (CIcon::SaveAs, "Save as", 1),
                    (CIcon::Print, "Print", 0),
                    (CIcon::Import, "Import to canvas", 0),
                    (CIcon::Wallpaper, "Set as desktop background", 2),
                    (CIcon::Properties, "Properties", 0),
                    (CIcon::Settings, "Settings and About", 0),
                    (CIcon::Exit, "Exit", 0),
                ];
                for (i, (icon, label, pane)) in items.into_iter().enumerate() {
                    if i == 7 || i == 9 {
                        let (sep, _) = ui.allocate_exact_size(vec2(240.0, 7.0), Sense::hover());
                        ui.painter().line_segment([pos2(sep.min.x + 44.0, sep.center().y), pos2(sep.max.x - 4.0, sep.center().y)], Stroke::new(1.0, rgb(0xD5, 0xD5, 0xD5)));
                    }
                    let (row, resp) = ui.allocate_exact_size(vec2(240.0, 40.0), Sense::click());
                    let open = pane != 0 && self.app_menu_pane == pane;
                    self.skin().btn_bg(ui.painter(), row, resp.hovered() || open, resp.is_pointer_button_down_on(), false);
                    classic_icons::paint(ui.painter(), Rect::from_center_size(pos2(row.min.x + 22.0, row.center().y), vec2(32.0, 32.0)), icon, true);
                    ui.painter().text(pos2(row.min.x + 46.0, row.center().y), Align2::LEFT_CENTER, label, FontId::proportional(12.5), rgb(0x1E, 0x1E, 0x1E));
                    if pane != 0 {
                        let c = pos2(row.max.x - 10.0, row.center().y);
                        ui.painter().add(egui::Shape::convex_polygon(vec![c + vec2(-2.0, -4.0), c + vec2(2.0, 0.0), c + vec2(-2.0, 4.0)], rgb(0x40, 0x40, 0x40), Stroke::NONE));
                    }
                    if resp.hovered() && pane != self.app_menu_pane {
                        let now = ui.input(|i| i.time);
                        match self.app_menu_hover {
                            Some((p, since)) if p == pane => {
                                if now - since > 0.3 {
                                    self.app_menu_pane = pane;
                                    self.app_menu_hover = None;
                                } else {
                                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(60));
                                }
                            }
                            _ => {
                                // Commands with choices open them at once; leaving them
                                // for a plain command waits a moment.
                                if pane != 0 {
                                    self.app_menu_pane = pane;
                                } else {
                                    self.app_menu_hover = Some((pane, now));
                                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(60));
                                }
                            }
                        }
                    }
                    if resp.clicked() {
                        let action = match i {
                            0 => Some(Action::New),
                            1 => Some(Action::Open),
                            2 => Some(Action::Save),
                            3 => Some(Action::SaveAs(None)),
                            4 => Some(Action::Print),
                            5 => Some(Action::Import),
                            7 => Some(Action::Properties),
                            9 => Some(Action::Exit),
                            _ => None,
                        };
                        if i == 8 {
                            self.drag = None;
                            self.doc.commit_overlay(&self.s);
                            self.show_settings = true;
                        }
                        if let Some(a) = action {
                            self.push(a);
                        }
                        if i != 6 {
                            ui.close();
                        }
                    }
                }
            });
            // The right side, on a lighter background with a line between.
            let (line, _) = ui.allocate_exact_size(vec2(1.0, 420.0), Sense::hover());
            ui.painter().line_segment([line.center_top(), line.center_bottom()], Stroke::new(1.0, rgb(0xD5, 0xD5, 0xD5)));
            ui.vertical(|ui| {
                ui.set_width(236.0);
                let header = |ui: &mut Ui, text: &str| {
                    let (h, _) = ui.allocate_exact_size(vec2(236.0, 30.0), Sense::hover());
                    ui.painter().text(pos2(h.min.x + 10.0, h.center().y), Align2::LEFT_CENTER, text, FontId::proportional(13.0), rgb(0x1E, 0x39, 0x5B));
                    ui.painter().line_segment([pos2(h.min.x + 8.0, h.max.y - 3.0), pos2(h.max.x - 8.0, h.max.y - 3.0)], Stroke::new(1.0, rgb(0xC5, 0xD2, 0xE3)));
                };
                match self.app_menu_pane {
                    1 => {
                        header(ui, "Save a copy as");
                        for (label, ext) in [("PNG picture", "png"), ("JPEG picture", "jpg"), ("BMP picture", "bmp"), ("GIF picture", "gif"), ("TIFF picture", "tiff"), ("WEBP picture", "webp"), ("ICO picture", "ico")] {
                            if self.pane_row(ui, Some(CIcon::Thumbnail), label) {
                                self.push(Action::SaveAs(Some(ext)));
                                ui.close();
                            }
                        }
                    }
                    2 => {
                        header(ui, "Set as desktop background");
                        for (label, mode) in [("Fill", "fill"), ("Tile", "tile"), ("Center", "center")] {
                            if self.pane_row(ui, Some(CIcon::Wallpaper), label) {
                                self.push(Action::Wallpaper(mode));
                                ui.close();
                            }
                        }
                    }
                    _ => {
                        header(ui, "Recent pictures");
                        let recent = self.recent.clone();
                        if recent.is_empty() {
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.add_space(10.0);
                                ui.label(egui::RichText::new("No recent pictures").size(12.0).color(rgb(0x80, 0x80, 0x80)));
                            });
                        }
                        for (i, path) in recent.iter().take(9).enumerate() {
                            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                            if self.pane_row(ui, None, &format!("{}   {name}", i + 1)) {
                                self.push(Action::OpenPath(path.clone()));
                                ui.close();
                            }
                        }
                    }
                }
            });
        });
    }

    /// A row on the right side of the classic File menu; true when clicked.
    fn pane_row(&self, ui: &mut Ui, icon: Option<CIcon>, label: &str) -> bool {
        let (row, resp) = ui.allocate_exact_size(vec2(236.0, 28.0), Sense::click());
        self.skin().btn_bg(ui.painter(), row.shrink2(vec2(4.0, 1.0)), resp.hovered(), resp.is_pointer_button_down_on(), false);
        let mut x = row.min.x + 10.0;
        if let Some(icon) = icon {
            classic_icons::paint(ui.painter(), Rect::from_center_size(pos2(x + 8.0, row.center().y), vec2(16.0, 16.0)), icon, true);
            x += 24.0;
        }
        let g = ui.painter().layout(label.to_string(), FontId::proportional(12.5), rgb(0x1E, 0x1E, 0x1E), 236.0 - (x - row.min.x) - 8.0);
        ui.painter().galley(pos2(x, row.center().y - g.size().y / 2.0), g, rgb(0x1E, 0x1E, 0x1E));
        resp.clicked()
    }

    fn home_tab(&mut self, ui: &mut Ui) {
        self.clipboard_cgroup(ui);
        self.cgroup(ui, "Image", 118.0, |app, ui, c| {
            let select = app.s.tool == Tool::Select;
            let (main, menu) = app.big_split(ui, r(c.min.x + 1.0, c.min.y, 46.0, CONTENT_H), "cselect", CIcon::Select, "Select", select);
            if main.on_hover_text("Select (S)").clicked() {
                app.set_tool(Tool::Select);
            }
            egui::Popup::menu(&menu).width(230.0).show(|ui| app.select_menu(ui));
            let has_sel = app.doc.floating().is_some();
            let x = c.min.x + 50.0;
            if app.small_button(ui, r(x, c.min.y + 1.0, 66.0, 22.0), "ccrop", Some(CIcon::Crop), "Crop", false, false, has_sel).on_hover_text("Crop (Ctrl+Shift+X)").clicked() {
                app.push(Action::Crop);
            }
            if app.small_button(ui, r(x, c.min.y + 23.0, 66.0, 22.0), "cresize", Some(CIcon::Resize), "Resize", false, false, true).on_hover_text("Resize and skew (Ctrl+W)").clicked() {
                app.push(Action::ResizeDialog);
            }
            let rot = app.small_button(ui, r(x, c.min.y + 45.0, 66.0, 22.0), "crotate", Some(CIcon::Rotate), "Rotate", true, false, true);
            egui::Popup::menu(&rot).width(210.0).show(|ui| {
                use crate::icons::Icon;
                for (icon, label, a) in [
                    (Icon::RotateRight, "Rotate right 90°", Action::RotateRight),
                    (Icon::RotateLeft, "Rotate left 90°", Action::RotateLeft),
                    (Icon::Rotate180, "Rotate 180°", Action::Rotate180),
                    (Icon::Rotate, "Custom rotate…", Action::CustomRotate),
                    (Icon::FlipV, "Flip vertical", Action::FlipVertical),
                    (Icon::FlipH, "Flip horizontal", Action::FlipHorizontal),
                ] {
                    if app.menu_item_icon(ui, icon, label, false) {
                        app.push(a);
                    }
                }
            });
        });
        self.cgroup(ui, "Tools", 72.0, |app, ui, c| {
            let tools = [
                (Tool::Pencil, CIcon::Pencil, "Pencil (P)"),
                (Tool::Fill, CIcon::Fill, "Fill with color (B)"),
                (Tool::Text, CIcon::Text, "Text (T)"),
                (Tool::Eraser, CIcon::Eraser, "Eraser (E)"),
                (Tool::Picker, CIcon::Picker, "Color picker (I)"),
                (Tool::Magnifier, CIcon::Magnifier, "Magnifier (Z)"),
            ];
            for (i, (t, icon, tip)) in tools.into_iter().enumerate() {
                let (col, row) = ((i % 3) as f32, (i / 3) as f32);
                let b = r(c.min.x + 2.0 + col * 23.0, c.min.y + 8.0 + row * 25.0, 22.0, 22.0);
                if app.icon_btn(ui, b, tip, icon, app.s.tool == t, true).on_hover_text(tip).clicked() {
                    app.set_tool(t);
                }
            }
        });
        self.cgroup(ui, "Brushes", 54.0, |app, ui, c| {
            // Shows the chosen brush's custom icon when there is one, as Paint did.
            let current = Some(app.s.brush.file_name());
            let (main, menu) = app.big_split_custom(ui, r(c.min.x + 2.0, c.min.y, 50.0, CONTENT_H), "cbrush", CIcon::Brushes, current, "Brushes", app.s.tool == Tool::Brush);
            if main.on_hover_text(app.s.brush.name()).clicked() {
                app.set_tool(Tool::Brush);
            }
            let previews = app.brush_previews(ui.ctx());
            egui::Popup::menu(&menu).width(300.0).show(|ui| {
                for (i, k) in BrushKind::ALL.into_iter().enumerate() {
                    let (row, resp) = ui.allocate_exact_size(vec2(290.0, 36.0), Sense::click());
                    let sel = app.s.tool == Tool::Brush && app.s.brush == k;
                    app.skin().btn_bg(ui.painter(), row, resp.hovered(), resp.is_pointer_button_down_on(), sel);
                    let icon = Rect::from_center_size(pos2(row.min.x + 20.0, row.center().y), vec2(24.0, 24.0));
                    if !classic_icons::paint_custom(ui.painter(), icon, k.file_name(), true) {
                        classic_icons::paint(ui.painter(), icon, CIcon::Brushes, true);
                    }
                    ui.painter().text(pos2(row.min.x + 40.0, row.center().y), Align2::LEFT_CENTER, k.name(), app.font(), app.skin().text);
                    if let Some(t) = previews.get(i) {
                        let pr = Rect::from_min_size(pos2(row.max.x - 140.0, row.center().y - 13.0), vec2(130.0, 26.0));
                        ui.painter().image(t.id(), pr, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                    }
                    if resp.clicked() {
                        app.s.brush = k;
                        app.set_tool(Tool::Brush);
                        ui.close();
                    }
                }
            });
        });
        self.cgroup(ui, "Shapes", 8.0 * 21.0 + 6.0 + 74.0, |app, ui, c| {
            // Gallery: 3 rows of shapes in a white box.
            let frame = r(c.min.x + 2.0, c.min.y + 2.0, 8.0 * 21.0 + 2.0, 3.0 * 21.0 + 2.0);
            ui.painter().rect_filled(frame, 0.0, Color32::WHITE);
            ui.painter().rect_stroke(frame, 0.0, Stroke::new(1.0, app.skin().box_border), StrokeKind::Inside);
            for (i, k) in ShapeKind::ALL.into_iter().enumerate() {
                let (col, row) = ((i % 8) as f32, (i / 8) as f32);
                let b = r(frame.min.x + 1.0 + col * 21.0, frame.min.y + 1.0 + row * 21.0, 21.0, 21.0);
                let resp = ui.interact(b, ui.id().with(("cshape", i)), app.click_sense(ui));
                let sel = app.s.tool == Tool::Shape && app.s.shape == k;
                app.skin().btn_bg(ui.painter(), b, resp.hovered(), resp.is_pointer_button_down_on(), sel);
                if !classic_icons::paint_custom(ui.painter(), b.shrink(2.5), k.file_name(), true) {
                    crate::icons::paint(ui.painter(), b.shrink(3.0), crate::icons::Icon::Shape(k), rgb(0x1E, 0x3A, 0x6A), rgb(0x2B, 0x6C, 0xC4));
                }
                if resp.on_hover_text(k.name()).clicked() {
                    app.doc.commit_overlay(&app.s);
                    app.s.shape = k;
                    app.set_tool(Tool::Shape);
                }
            }
            let x = frame.max.x + 4.0;
            for (i, outline) in [true, false].into_iter().enumerate() {
                let (icon, label, key) = if outline { (CIcon::Outline, "Outline", "coutline") } else { (CIcon::FillStyle, "Fill", "cfill") };
                let resp = app.small_button(ui, r(x, c.min.y + 3.0 + i as f32 * 24.0, 72.0, 22.0), key, Some(icon), label, true, false, true);
                let cur = if outline { app.s.outline } else { app.s.fill };
                egui::Popup::menu(&resp).width(190.0).show(|ui| {
                    for st in ShapeStyle::ALL {
                        let name = if outline { st.outline_name() } else { st.fill_name() };
                        if app.menu_item(ui, crate::icons::Icon::Blank, name, "", cur == st, true) {
                            if outline {
                                app.s.outline = st;
                            } else {
                                app.s.fill = st;
                            }
                        }
                    }
                });
            }
        });
        self.cgroup(ui, "", 46.0, |app, ui, c| {
            let resp = app.big_button(ui, r(c.min.x + 1.0, c.min.y, 44.0, CONTENT_H + LABEL_H - 2.0), "csize", CIcon::Size, "Size", true, false, true);
            egui::Popup::menu(&resp).width(170.0).close_behavior(PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
                let tool = match app.s.tool {
                    Tool::Select | Tool::Fill | Tool::Picker | Tool::Magnifier | Tool::Text => Tool::Brush,
                    t => t,
                };
                for &sz in size_presets(tool) {
                    let cur = app.size_for(tool);
                    let (row, resp) = ui.allocate_exact_size(vec2(160.0, 26.0), Sense::click());
                    app.skin().btn_bg(ui.painter(), row, resp.hovered(), resp.is_pointer_button_down_on(), (cur - sz).abs() < 0.01);
                    ui.painter().text(row.left_center() + vec2(8.0, 0.0), Align2::LEFT_CENTER, format!("{sz}px"), app.font(), app.skin().label);
                    let y = row.center().y;
                    ui.painter().line_segment([pos2(row.min.x + 48.0, y), pos2(row.max.x - 10.0, y)], Stroke::new(sz.min(18.0), Color32::BLACK));
                    if resp.clicked() {
                        *app.size_mut_for(tool) = sz;
                        ui.close();
                    }
                }
                // Opacity (pencil, brushes) or tolerance (fill), which the Windows 11
                // look shows as sliders beside the canvas.
                let pal = app.pal;
                let (label, value, range) = match app.s.tool {
                    Tool::Pencil | Tool::Brush => ("Opacity", &mut app.s.opacity, 1.0..=100.0),
                    Tool::Fill => ("Tolerance", &mut app.s.tolerance, 0.0..=100.0),
                    _ => return,
                };
                ui.add_space(4.0);
                ui.separator();
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(label).size(12.0));
                });
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    if crate::fluent::slider(ui, &pal, value, range, 110.0, false).changed() {
                        *value = value.round();
                    }
                    ui.label(egui::RichText::new(format!("{}%", *value)).size(12.0));
                });
            });
        });
        self.colors_cgroup(ui);
    }

    fn select_menu(&mut self, ui: &mut Ui) {
        use crate::icons::Icon;
        ui.label(egui::RichText::new("Selection shapes").strong().color(self.skin().label));
        for (mode, icon, label) in [(SelectMode::Rectangle, Icon::SelectRect, "Rectangular selection"), (SelectMode::FreeForm, Icon::SelectFree, "Free-form selection")] {
            if self.menu_item_icon(ui, icon, label, self.s.select_mode == mode) {
                self.s.select_mode = mode;
                self.set_tool(Tool::Select);
            }
        }
        ui.separator();
        ui.label(egui::RichText::new("Selection options").strong().color(self.skin().label));
        let has_sel = self.doc.floating().is_some();
        for (icon, label, sc, a, enabled) in [
            (Icon::SelectAll, "Select all", "Ctrl+A", Action::SelectAll, true),
            (Icon::Invert, "Invert selection", "", Action::InvertSelection, true),
            (Icon::Delete, "Delete", "Delete", Action::DeleteSelection, has_sel),
        ] {
            if self.menu_item(ui, icon, label, sc, false, enabled) {
                self.push(a);
            }
        }
        if self.menu_item_icon(ui, Icon::Transparent, "Transparent selection", self.s.transparent_selection) {
            self.s.transparent_selection = !self.s.transparent_selection;
        }
        ui.separator();
        if self.menu_item(ui, Icon::RemoveBg, "Remove background", "", false, true) {
            self.push(Action::RemoveBackground);
        }
    }

    fn clipboard_cgroup(&mut self, ui: &mut Ui) {
        self.cgroup(ui, "Clipboard", 96.0, |app, ui, c| {
            let (main, menu) = app.big_split(ui, r(c.min.x + 1.0, c.min.y, 44.0, CONTENT_H), "cpaste", CIcon::Paste, "Paste", false);
            if main.on_hover_text("Paste (Ctrl+V)").clicked() {
                app.push(Action::Paste);
            }
            egui::Popup::menu(&menu).width(180.0).show(|ui| {
                use crate::icons::Icon;
                if app.menu_item(ui, Icon::Paste, "Paste", "Ctrl+V", false, true) {
                    app.push(Action::Paste);
                }
                if app.menu_item(ui, Icon::Import, "Paste from", "", false, true) {
                    app.push(Action::Import);
                }
            });
            let has_sel = app.doc.floating().is_some();
            if app.small_button(ui, r(c.min.x + 48.0, c.min.y + 1.0, 46.0, 22.0), "ccut", Some(CIcon::Cut), "Cut", false, false, has_sel).on_hover_text("Cut (Ctrl+X)").clicked() {
                app.push(Action::Cut);
            }
            if app.small_button(ui, r(c.min.x + 48.0, c.min.y + 23.0, 46.0, 22.0), "ccopy", Some(CIcon::Copy), "Copy", false, false, has_sel).on_hover_text("Copy (Ctrl+C)").clicked() {
                app.push(Action::Copy);
            }
        });
    }

    fn colors_cgroup(&mut self, ui: &mut Ui) {
        let cell = 21.0;
        self.cgroup(ui, "Colors", 44.0 * 2.0 + 4.0 + cell * 10.0 + 6.0 + 48.0, |app, ui, c| {
            let skin = app.skin();
            for slot in [1u8, 2] {
                let b = r(c.min.x + 1.0 + (slot - 1) as f32 * 45.0, c.min.y, 44.0, CONTENT_H);
                let resp = ui.interact(b, ui.id().with(("cslot", slot)), app.click_sense(ui));
                skin.btn_bg(ui.painter(), b, resp.hovered(), resp.is_pointer_button_down_on(), app.active_slot == slot);
                let color = if slot == 1 { app.s.color1 } else { app.s.color2 };
                let sw = Rect::from_center_size(pos2(b.center().x, b.min.y + 19.0), vec2(26.0, 26.0));
                ui.painter().rect_filled(sw.expand(1.0), 0.0, Color32::WHITE);
                ui.painter().rect_stroke(sw.expand(2.0), 0.0, Stroke::new(1.0, skin.box_border), StrokeKind::Inside);
                ui.painter().rect_filled(sw.shrink(1.0), 0.0, color);
                for (i, line) in ["Color", if slot == 1 { "1" } else { "2" }].into_iter().enumerate() {
                    ui.painter().text(pos2(b.center().x, b.min.y + 43.0 + i as f32 * 13.0), Align2::CENTER_CENTER, line, app.font(), skin.text);
                }
                let tip = if slot == 1 { "Color 1 (foreground color)" } else { "Color 2 (background color)" };
                if resp.on_hover_text(tip).clicked() {
                    app.active_slot = slot;
                }
            }
            // Palette: two rows of standard colors, one row of custom colors.
            let x0 = c.min.x + 92.0;
            let y0 = c.min.y + (CONTENT_H - 3.0 * cell) / 2.0;
            for i in 0..30 {
                let (col, row) = ((i % 10) as f32, (i / 10) as f32);
                let b = r(x0 + col * cell, y0 + row * cell, cell, cell);
                let color = if let Some(rgb) = PALETTE.get(i) {
                    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
                } else {
                    app.s.custom_colors.get(i - 20).copied().unwrap_or(Color32::TRANSPARENT)
                };
                let resp = ui.interact(b, ui.id().with(("cpal", i)), app.click_sense(ui));
                let sw = b.shrink(2.0);
                if resp.hovered() {
                    ui.painter().rect_stroke(b.shrink(0.5), 0.0, Stroke::new(1.0, skin.hover_border), StrokeKind::Inside);
                }
                if color == Color32::TRANSPARENT {
                    ui.painter().rect_filled(sw, 0.0, Color32::WHITE.gamma_multiply(0.6));
                    ui.painter().rect_stroke(sw, 0.0, Stroke::new(1.0, skin.box_border.gamma_multiply(0.6)), StrokeKind::Inside);
                    if resp.clicked() {
                        app.push(Action::EditColors);
                    }
                    continue;
                }
                ui.painter().rect_filled(sw, 0.0, color);
                ui.painter().rect_stroke(sw, 0.0, Stroke::new(1.0, rgb(0x80, 0x80, 0x80)), StrokeKind::Inside);
                let resp = match PALETTE_NAMES.get(i) {
                    Some(name) => resp.on_hover_text(*name),
                    None => resp.on_hover_text("Custom color"),
                };
                if resp.clicked() && ui.input(|i| i.modifiers.shift) || resp.secondary_clicked() {
                    app.s.color2 = color;
                } else if resp.clicked() {
                    app.set_active_color(color);
                }
            }
            let b = r(x0 + cell * 10.0 + 4.0, c.min.y, 48.0, CONTENT_H);
            if app.big_button(ui, b, "cedit", CIcon::EditColors, "Edit\ncolors", false, false, true).on_hover_text("Edit colors").clicked() {
                app.push(Action::EditColors);
            }
        });
    }

    fn view_tab(&mut self, ui: &mut Ui) {
        self.cgroup(ui, "Zoom", 3.0 * 46.0 + 4.0, |app, ui, c| {
            for (i, (icon, label, a, enabled)) in [
                (CIcon::ZoomIn, "Zoom\nin", Action::ZoomIn, app.zoom < ZOOM_MAX),
                (CIcon::ZoomOut, "Zoom\nout", Action::ZoomOut, app.zoom > ZOOM_MIN),
                (CIcon::Zoom100, "100\n%", Action::Zoom100, true),
            ]
            .into_iter()
            .enumerate()
            {
                let b = r(c.min.x + 2.0 + i as f32 * 46.0, c.min.y, 45.0, CONTENT_H);
                if app.big_button(ui, b, label, icon, label, false, false, enabled).clicked() {
                    app.push(a);
                }
            }
        });
        self.cgroup(ui, "Show or hide", 190.0, |app, ui, c| {
            let mut items = vec![
                ("Rulers", app.show_rulers, Action::ToggleRulers),
                ("Gridlines", app.show_grid, Action::ToggleGrid),
                ("Status bar", app.show_status, Action::ToggleStatus),
            ];
            items.push(("Layers", app.show_layers, Action::ToggleStatus));
            for (i, (label, on, a)) in items.into_iter().enumerate() {
                let (col, row) = ((i / 3) as f32, (i % 3) as f32);
                let b = r(c.min.x + 4.0 + col * 96.0, c.min.y + 3.0 + row * 21.0, 92.0, 20.0);
                if app.check_row(ui, b, label, on) {
                    if label == "Layers" {
                        app.show_layers = !app.show_layers;
                    } else {
                        app.push(a);
                    }
                }
            }
        });
        self.cgroup(ui, "Display", 54.0 + 66.0 + 4.0, |app, ui, c| {
            if app.big_button(ui, r(c.min.x + 2.0, c.min.y, 52.0, CONTENT_H), "cfull", CIcon::FullScreen, "Full\nscreen", false, false, true).on_hover_text("Full screen (F11)").clicked() {
                app.push(Action::Fullscreen);
            }
            if app.big_button(ui, r(c.min.x + 56.0, c.min.y, 64.0, CONTENT_H), "cthumb", CIcon::Thumbnail, "Thumbnail", false, app.show_thumbnail, true).clicked() {
                app.push(Action::ToggleThumbnail);
            }
        });
    }

    /// A check box row in the ribbon; true when clicked.
    fn check_row(&self, ui: &mut Ui, rect: Rect, label: &str, on: bool) -> bool {
        let resp = ui.interact(rect, ui.id().with(("ccheck", label)), self.click_sense(ui));
        let skin = self.skin();
        let bx = Rect::from_center_size(pos2(rect.min.x + 8.0, rect.center().y), vec2(13.0, 13.0));
        let p = ui.painter();
        p.rect_filled(bx, 0.0, if resp.hovered() { rgb(0xE8, 0xF4, 0xFF) } else { Color32::WHITE });
        p.rect_stroke(bx, 0.0, Stroke::new(1.0, if resp.hovered() { rgb(0x33, 0x99, 0xFF) } else { rgb(0x8E, 0x8F, 0x8F) }), StrokeKind::Inside);
        if on {
            let c = bx.center();
            p.add(egui::Shape::line(vec![c + vec2(-3.5, 0.0), c + vec2(-1.0, 3.0), c + vec2(4.0, -3.5)], Stroke::new(1.6, rgb(0x20, 0x20, 0x20))));
        }
        p.text(pos2(rect.min.x + 20.0, rect.center().y), Align2::LEFT_CENTER, label, self.font(), skin.text);
        resp.clicked()
    }

    fn text_tab(&mut self, ui: &mut Ui) {
        self.clipboard_cgroup(ui);
        self.cgroup(ui, "Font", 250.0, |app, ui, c| {
            let mut row = ui.new_child(egui::UiBuilder::new().max_rect(r(c.min.x + 4.0, c.min.y + 4.0, 242.0, 28.0)).layout(egui::Layout::left_to_right(egui::Align::Center)));
            row.spacing_mut().item_spacing = vec2(4.0, 0.0);
            app.font_family_combo(&mut row, 150.0);
            row.spacing_mut().item_spacing.x = 0.0;
            app.font_size_box(&mut row);
            let t = app.s.text.clone();
            let mut new = t.clone();
            for (i, (label, on, tip)) in [("B", t.bold, "Bold (Ctrl+B)"), ("I", t.italic, "Italic (Ctrl+I)"), ("U", t.underline, "Underline (Ctrl+U)"), ("abc", t.strikethrough, "Strikethrough")].into_iter().enumerate() {
                let b = r(c.min.x + 4.0 + i as f32 * 25.0, c.min.y + 38.0, 24.0, 22.0);
                let resp = ui.interact(b, ui.id().with(("cfmt", i)), app.click_sense(ui));
                app.skin().btn_bg(ui.painter(), b, resp.hovered(), resp.is_pointer_button_down_on(), on);
                let rt = egui::RichText::new(label).size(13.0).color(rgb(0x1C, 0x2A, 0x48));
                let rt = match i {
                    0 => rt.strong(),
                    1 => rt.italics(),
                    2 => rt.underline(),
                    _ => rt.strikethrough().size(11.0),
                };
                let g = egui::WidgetText::from(rt).into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, egui::TextStyle::Body);
                ui.painter().galley(b.center() - g.size() / 2.0, g, rgb(0x1C, 0x2A, 0x48));
                if resp.on_hover_text(tip).clicked() {
                    match i {
                        0 => new.bold = !t.bold,
                        1 => new.italic = !t.italic,
                        2 => new.underline = !t.underline,
                        _ => new.strikethrough = !t.strikethrough,
                    }
                }
            }
            for (i, (align, icon, tip)) in [(0u8, crate::icons::Icon::AlignLeft, "Align left"), (1, crate::icons::Icon::AlignCenter, "Center"), (2, crate::icons::Icon::AlignRight, "Align right")].into_iter().enumerate() {
                let b = r(c.min.x + 112.0 + i as f32 * 25.0, c.min.y + 38.0, 24.0, 22.0);
                let resp = ui.interact(b, ui.id().with(("calign", i)), app.click_sense(ui));
                app.skin().btn_bg(ui.painter(), b, resp.hovered(), resp.is_pointer_button_down_on(), t.align == align);
                classic_icons::paint_extra(ui.painter(), Rect::from_center_size(b.center(), vec2(16.0, 16.0)), icon, true);
                if resp.on_hover_text(tip).clicked() {
                    new.align = align;
                }
            }
            app.s.text = new;
        });
        self.cgroup(ui, "Background", 104.0, |app, ui, c| {
            let opaque = app.s.text.opaque;
            if app.small_button(ui, r(c.min.x + 2.0, c.min.y + 6.0, 100.0, 22.0), "copaque", Some(CIcon::Opaque), "Opaque", false, opaque, true).clicked() {
                app.s.text.opaque = true;
            }
            if app.small_button(ui, r(c.min.x + 2.0, c.min.y + 30.0, 100.0, 22.0), "ctransp", Some(CIcon::Transparent), "Transparent", false, !opaque, true).clicked() {
                app.s.text.opaque = false;
            }
        });
        self.colors_cgroup(ui);
    }

    // ---------------------------------------------------------------------
    // Status bar

    pub fn classic_status_bar(&mut self, ui: &mut Ui) {
        let skin = self.skin();
        let rect = ui.max_rect();
        gradient(ui.painter(), rect, skin.status[0], skin.status[1], 0.0);
        ui.painter().line_segment([rect.left_top(), rect.right_top()], Stroke::new(1.0, skin.ribbon_border));
        let text = |ui: &Ui, x: f32, icon: crate::icons::Icon, s: String| {
            let ci = match icon {
                crate::icons::Icon::Cursor => CIcon::StatusCursor,
                crate::icons::Icon::SelectionSize => CIcon::StatusSelection,
                crate::icons::Icon::CanvasSize => CIcon::StatusCanvas,
                _ => CIcon::StatusDisk,
            };
            classic_icons::paint(ui.painter(), Rect::from_center_size(pos2(x + 8.0, rect.center().y), vec2(16.0, 16.0)), ci, true);
            ui.painter().text(pos2(x + 22.0, rect.center().y), Align2::LEFT_CENTER, s, FontId::proportional(12.0), rgb(0x20, 0x20, 0x20));
        };
        let sep = |ui: &Ui, x: f32| {
            ui.painter().line_segment([pos2(x, rect.min.y + 3.0), pos2(x, rect.max.y - 3.0)], Stroke::new(1.0, skin.status_sep));
        };
        let pos = self
            .cursor_img
            .filter(|p| p.x >= 0.0 && p.y >= 0.0 && p.x < self.doc.width() as f32 && p.y < self.doc.height() as f32)
            .map(|p| format!("{}, {}px", p.x.floor(), p.y.floor()))
            .unwrap_or_default();
        let sel = self.selection_size_text();
        let canvas = match &self.drag {
            Some(Drag::CanvasResize { size, .. }) => *size,
            _ => vec2(self.doc.width() as f32, self.doc.height() as f32),
        };
        // Fields from the left, each shown only while it fits before the zoom controls
        // (the last ones go first when the window gets narrow).
        let right = rect.max.x - 12.0;
        let zoom_left = right - 18.0 - 110.0 - 20.0 - 52.0;
        let mut fields = vec![
            (crate::icons::Icon::Cursor, pos, 150.0),
            (crate::icons::Icon::SelectionSize, sel, 150.0),
            (crate::icons::Icon::CanvasSize, format!("{} × {}px", canvas.x, canvas.y), 160.0),
        ];
        if let Some(size) = self.doc.file_size {
            fields.push((crate::icons::Icon::Disk, format!("Size: {}", human_size(size)), 150.0));
        }
        let mut x = rect.min.x + 6.0;
        for (icon, s, width) in fields {
            if x + width > zoom_left {
                break;
            }
            text(ui, x, icon, s);
            x += width;
            sep(ui, x - 6.0);
        }

        // Zoom: percentage, minus, slider, plus.
        let slider = r(right - 18.0 - 110.0, rect.center().y - 8.0, 110.0, 16.0);
        let plus = r(right - 16.0, rect.center().y - 8.0, 16.0, 16.0);
        let minus = r(slider.min.x - 20.0, rect.center().y - 8.0, 16.0, 16.0);
        ui.painter().text(pos2(minus.min.x - 8.0, rect.center().y), Align2::RIGHT_CENTER, format!("{}%", (self.zoom * 100.0).round()), FontId::proportional(12.0), rgb(0x20, 0x20, 0x20));
        for (b, sign, dir) in [(minus, "−", -1), (plus, "+", 1)] {
            let resp = ui.interact(b, ui.id().with(("czoom", dir)), Sense::click());
            skin.btn_bg(ui.painter(), b, resp.hovered(), resp.is_pointer_button_down_on(), false);
            ui.painter().text(b.center(), Align2::CENTER_CENTER, sign, FontId::proportional(14.0), rgb(0x30, 0x30, 0x30));
            if resp.clicked() {
                self.zoom_step(dir, None);
            }
        }
        let resp = ui.interact(slider, ui.id().with("czoomslider"), Sense::click_and_drag());
        let (lo, hi) = (ZOOM_MIN.ln(), ZOOM_MAX.ln());
        if let Some(p) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
            let t = ((p.x - slider.min.x) / slider.width()).clamp(0.0, 1.0);
            let z = (lo + t * (hi - lo)).exp();
            self.set_zoom(if (z - 1.0).abs() < 0.05 { 1.0 } else { z }, None);
        }
        let y = slider.center().y;
        ui.painter().line_segment([pos2(slider.min.x, y), pos2(slider.max.x, y)], Stroke::new(1.0, rgb(0x80, 0x80, 0x80)));
        ui.painter().line_segment([pos2(slider.min.x, y + 1.0), pos2(slider.max.x, y + 1.0)], Stroke::new(1.0, Color32::WHITE));
        // A tick at 100%.
        let t100 = (0.0 - lo) / (hi - lo);
        let x100 = slider.min.x + t100 * slider.width();
        ui.painter().line_segment([pos2(x100, y - 4.0), pos2(x100, y + 4.0)], Stroke::new(1.0, rgb(0x80, 0x80, 0x80)));
        let t = (self.zoom.ln() - lo) / (hi - lo);
        let thumb = Rect::from_center_size(pos2(slider.min.x + t * slider.width(), y), vec2(7.0, 14.0));
        let thumb_fill = if resp.hovered() || resp.dragged() { skin.hover[1] } else if skin.win7 { rgb(0xE6, 0xEE, 0xF8) } else { rgb(0x00, 0x7A, 0xD9) };
        ui.painter().rect_filled(thumb, 1.5, thumb_fill);
        ui.painter().rect_stroke(thumb, 1.5, Stroke::new(1.0, if skin.win7 { rgb(0x6C, 0x7C, 0x96) } else { rgb(0x00, 0x5A, 0xA8) }), StrokeKind::Inside);
        ui.allocate_rect(rect, Sense::hover());
    }
}

/// A dialog in the classic style: a title strip, the light gray body, and push buttons
/// on the right with the first one as the default (blue border). Returns the index of
/// the clicked button.
pub fn dialog(
    ctx: &egui::Context,
    pal: Palette,
    id: &str,
    width: f32,
    title: &str,
    buttons: &[&str],
    f: &mut dyn FnMut(&mut Ui),
) -> Option<usize> {
    let skin = Skin::from_code(pal.classic);
    let mut clicked = None;
    let visuals = ctx.global_style().visuals.clone();
    let frame = egui::Frame::new()
        .fill(rgb(0xF0, 0xF0, 0xF0))
        .corner_radius(if skin.win7 { 4 } else { 0 })
        .stroke(Stroke::new(1.0, if skin.win7 { rgb(0x6C, 0x7C, 0x96) } else { rgb(0x00, 0x78, 0xD7) }))
        .shadow(visuals.window_shadow);
    egui::Modal::new(egui::Id::new(id)).frame(frame).show(ctx, |ui| {
        let spacing = ui.spacing().item_spacing;
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.style_mut().override_font_id = Some(FontId::proportional(12.5));
        let inner = width - 32.0;
        // Title strip.
        let (strip, _) = ui.allocate_exact_size(vec2(width, 28.0), Sense::hover());
        let radius = if skin.win7 { egui::CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 } } else { egui::CornerRadius::ZERO };
        ui.painter().rect_filled(strip, radius, skin.title[1]);
        gradient(ui.painter(), Rect::from_min_max(strip.min + vec2(1.0, 1.0), pos2(strip.max.x - 1.0, strip.center().y)), skin.title[0], skin.title[0].lerp_to_gamma(skin.title[1], 0.5), 0.0);
        ui.painter().text(strip.left_center() + vec2(12.0, 0.0), Align2::LEFT_CENTER, title, FontId::proportional(12.5), skin.title_text);
        if !skin.win7 {
            ui.painter().line_segment([strip.left_bottom(), strip.right_bottom()], Stroke::new(1.0, rgb(0xE6, 0xE6, 0xE6)));
        }
        // Close button, which answers like the last button (Cancel, or OK when alone).
        let x = if skin.win7 { r(strip.max.x - 46.0, strip.min.y + 1.0, 42.0, 18.0) } else { r(strip.max.x - 46.0, strip.min.y, 46.0, strip.height()) };
        let xr = ui.interact(x, ui.id().with("dialog_close"), Sense::click());
        let red = xr.hovered() || xr.is_pointer_button_down_on();
        if red {
            let fill = if xr.is_pointer_button_down_on() { rgb(0xF1, 0x70, 0x7A) } else { rgb(0xE8, 0x11, 0x23) };
            ui.painter().rect_filled(x, if skin.win7 { 3.0 } else { 0.0 }, fill);
        }
        let (c, k) = (x.center(), 4.5);
        let s = Stroke::new(1.0, if red { Color32::WHITE } else { skin.title_text });
        ui.painter().line_segment([c + vec2(-k, -k), c + vec2(k, k)], s);
        ui.painter().line_segment([c + vec2(-k, k), c + vec2(k, -k)], s);
        if xr.on_hover_text("Close").clicked() {
            clicked = Some(buttons.len().saturating_sub(1));
        }
        egui::Frame::new().inner_margin(egui::Margin { left: 16, right: 16, top: 14, bottom: 12 }).show(ui, |ui| {
            ui.spacing_mut().item_spacing = spacing;
            ui.set_width(inner);
            f(ui);
        });
        // Buttons, right-aligned on a slightly darker band.
        let band = egui::Frame::new().fill(rgb(0xF0, 0xF0, 0xF0)).inner_margin(egui::Margin { left: 16, right: 16, top: 6, bottom: 12 }).show(ui, |ui| {
            ui.set_width(inner);
            let alt = ui.input(|i| i.modifiers.alt);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                for (i, label) in buttons.iter().enumerate().rev() {
                    let (job, key) = crate::app::access_label(label, rgb(0x1E, 0x1E, 0x1E), alt, 12.5);
                    let galley = ui.painter().layout_job(job);
                    let w = (galley.size().x + 24.0).max(75.0);
                    let (rect, resp) = ui.allocate_exact_size(vec2(w, 24.0), Sense::click());
                    push_button(ui, skin, rect, &resp, i == 0);
                    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, rgb(0x1E, 0x1E, 0x1E));
                    let pressed = key.is_some_and(|k| ui.input(|inp| inp.key_pressed(k) && inp.modifiers.is_none()));
                    if resp.clicked() || pressed {
                        clicked = Some(i);
                    }
                }
            });
        });
        let _ = band;
    });
    clicked
}

/// A classic push button background; `default` draws the default button's blue rim.
fn push_button(ui: &Ui, skin: Skin, rect: Rect, resp: &Response, default: bool) {
    let p = ui.painter();
    let (hover, press) = (resp.hovered(), resp.is_pointer_button_down_on());
    if skin.win7 {
        let (top, bottom, border) = if press {
            (rgb(0xE5, 0xF4, 0xFC), rgb(0x98, 0xD1, 0xEF), rgb(0x2C, 0x62, 0x8B))
        } else if hover {
            (rgb(0xEA, 0xF6, 0xFD), rgb(0xBE, 0xE6, 0xFD), rgb(0x3C, 0x7F, 0xB1))
        } else {
            (rgb(0xF2, 0xF2, 0xF2), rgb(0xD0, 0xD0, 0xD0), if default { rgb(0x3C, 0x7F, 0xB1) } else { rgb(0x70, 0x70, 0x70) })
        };
        p.rect_filled(rect, 3.0, bottom);
        gradient(p, Rect::from_min_max(rect.min + vec2(1.0, 1.0), pos2(rect.max.x - 1.0, rect.center().y)), top, top.lerp_to_gamma(bottom, 0.4), 0.0);
        p.rect_stroke(rect, 3.0, Stroke::new(1.0, border), StrokeKind::Inside);
        if default && !hover && !press {
            p.rect_stroke(rect.shrink(1.0), 2.0, Stroke::new(1.0, rgb(0x9C, 0xCB, 0xEE)), StrokeKind::Inside);
        }
    } else {
        let (fill, border) = if press {
            (rgb(0xCC, 0xE4, 0xF7), rgb(0x00, 0x54, 0x99))
        } else if hover {
            (rgb(0xE5, 0xF1, 0xFB), rgb(0x00, 0x78, 0xD7))
        } else {
            (rgb(0xE1, 0xE1, 0xE1), if default { rgb(0x00, 0x78, 0xD7) } else { rgb(0xAD, 0xAD, 0xAD) })
        };
        p.rect_filled(rect, 0.0, fill);
        p.rect_stroke(rect, 0.0, Stroke::new(if default { 2.0 } else { 1.0 }, border), StrokeKind::Inside);
    }
}

/// The classic icon to show for a Windows 11 icon in menus (None: no icon, as many
/// classic menu items had).
pub fn menu_icon(icon: crate::icons::Icon) -> Option<CIcon> {
    use crate::icons::Icon as I;
    Some(match icon {
        I::Paste => CIcon::Paste,
        I::Cut => CIcon::Cut,
        I::Copy => CIcon::Copy,
        I::Save => CIcon::Save,
        I::Undo => CIcon::Undo,
        I::Redo => CIcon::Redo,
        I::Crop => CIcon::Crop,
        I::Resize => CIcon::Resize,
        I::Rotate | I::RotateRight | I::RotateLeft | I::Rotate180 => CIcon::Rotate,
        I::SelectRect | I::SelectAll => CIcon::Select,
        I::Pencil => CIcon::Pencil,
        I::Fill => CIcon::Fill,
        I::Text => CIcon::Text,
        I::Eraser => CIcon::Eraser,
        I::Picker => CIcon::Picker,
        I::Magnifier => CIcon::Magnifier,
        I::ZoomIn => CIcon::ZoomIn,
        I::ZoomOut => CIcon::ZoomOut,
        I::FullScreen => CIcon::FullScreen,
        I::Brush(_) => CIcon::Brushes,
        I::Transparent => CIcon::Transparent,
        I::Palette => CIcon::EditColors,
        I::Picture => CIcon::Thumbnail,
        I::Desktop => CIcon::FullScreen,
        _ => return None,
    })
}
