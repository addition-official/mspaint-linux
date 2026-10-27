//! The Windows 11 style toolbar ("ribbon"): Image, Tools, Brushes, Shapes, Size, Colors.

use crate::app::{Action, PaintApp};
use crate::icons::{self, Icon};
use crate::tools::{BrushKind, SelectMode, ShapeKind, ShapeStyle, Tool, size_presets};
use egui::{Color32, PopupCloseBehavior, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui, vec2};

pub const PALETTE: [[u8; 3]; 20] = [
    [0, 0, 0],
    [127, 127, 127],
    [136, 0, 21],
    [237, 28, 36],
    [255, 127, 39],
    [255, 242, 0],
    [34, 177, 76],
    [0, 162, 232],
    [63, 72, 204],
    [163, 73, 164],
    [255, 255, 255],
    [195, 195, 195],
    [185, 122, 87],
    [255, 174, 201],
    [255, 201, 14],
    [239, 228, 176],
    [181, 230, 29],
    [153, 217, 234],
    [112, 146, 190],
    [200, 191, 231],
];

/// Height of every group's content strip; labels sit below it.
const GROUP_H: f32 = 66.0;
const LABEL_H: f32 = 18.0;
const CHEVRON_H: f32 = 16.0;

fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(egui::pos2(x, y), vec2(w, h))
}

impl PaintApp {
    /// Fluent button background for `resp` (hover and pressed states) in `rect`.
    pub fn button_bg(&self, ui: &Ui, rect: Rect, resp: &Response, selected: bool) {
        let enabled = resp.sense.senses_click();
        self.pal.paint_button(
            ui.painter(),
            rect,
            enabled && resp.hovered(),
            enabled && resp.is_pointer_button_down_on(),
            selected,
        );
    }

    fn paint_icon(&self, ui: &Ui, center: egui::Pos2, size: f32, icon: Icon, enabled: bool) {
        self.paint_icon_fg(ui, center, size, icon, self.pal.icon_fg(enabled, false));
    }

    fn paint_icon_fg(&self, ui: &Ui, center: egui::Pos2, size: f32, icon: Icon, fg: Color32) {
        icons::paint(ui.painter(), Rect::from_center_size(center, vec2(size, size)), icon, fg, self.pal.accent);
    }

    /// Icon color for a button response (dims while pressed).
    fn resp_fg(&self, resp: &Response, enabled: bool) -> Color32 {
        self.pal.icon_fg(enabled, enabled && resp.is_pointer_button_down_on())
    }

    fn paint_chevron(&self, ui: &Ui, center: egui::Pos2) {
        icons::paint(
            ui.painter(),
            Rect::from_center_size(center, vec2(10.0, 10.0)),
            Icon::Chevron,
            self.pal.text_dim,
            self.pal.accent,
        );
    }

    /// Flat icon button in an exact rectangle.
    fn btn_at(&self, ui: &mut Ui, rect: Rect, key: &str, icon: Icon, selected: bool, tip: &str) -> Response {
        self.btn_at_enabled(ui, rect, key, icon, selected, tip, true)
    }

    #[allow(clippy::too_many_arguments)]
    fn btn_at_enabled(
        &self,
        ui: &mut Ui,
        rect: Rect,
        key: &str,
        icon: Icon,
        selected: bool,
        tip: &str,
        enabled: bool,
    ) -> Response {
        let sense = if enabled { Sense::click() } else { Sense::hover() };
        let resp = ui.interact(rect, ui.id().with(key), sense);
        self.button_bg(ui, rect, &resp, selected);
        let size = (rect.height() - 8.0).clamp(14.0, 20.0);
        self.paint_icon_fg(ui, rect.center(), size, icon, self.resp_fg(&resp, enabled));
        resp.on_hover_text(tip)
    }

    /// Big button with the icon on top and a drop-down chevron underneath, like Win11's
    /// Selection and Brushes buttons. Returns (main, chevron).
    fn split_btn_at(&self, ui: &mut Ui, rect: Rect, key: &str, icon: Icon, selected: bool, tip: &str) -> (Response, Response) {
        let top = Rect::from_min_max(rect.min, egui::pos2(rect.max.x, rect.max.y - CHEVRON_H));
        let bottom = Rect::from_min_max(egui::pos2(rect.min.x, rect.max.y - CHEVRON_H), rect.max);
        let main = ui.interact(top, ui.id().with((key, "main")), Sense::click());
        let chev = ui.interact(bottom, ui.id().with((key, "chev")), Sense::click());
        // Win11 split button: hovering either part lightly highlights the whole button and
        // the part under the pointer more strongly; the pressed part gets the pressed fill.
        let pal = self.pal;
        pal.paint_button(ui.painter(), rect, false, false, selected);
        if main.hovered() || chev.hovered() {
            ui.painter().rect_filled(rect, 4.0, pal.hover.gamma_multiply(0.6));
            let (hot, resp) = if main.hovered() { (top, &main) } else { (bottom, &chev) };
            let fill = if resp.is_pointer_button_down_on() { pal.pressed } else { pal.hover };
            ui.painter().rect_filled(hot, 4.0, fill);
        }
        self.paint_icon_fg(ui, top.center() + vec2(0.0, 2.0), 22.0, icon, self.resp_fg(&main, true));
        self.paint_chevron(ui, bottom.center());
        (main.on_hover_text(tip), chev.on_hover_text(format!("{tip} options")))
    }

    /// Big single drop-down: icon on top, chevron underneath, one hover area. Win11's
    /// Size button works like this (the whole button opens the list).
    fn stacked_dropdown_at(&self, ui: &mut Ui, rect: Rect, key: &str, icon: Icon, tip: &str) -> Response {
        let resp = ui.interact(rect, ui.id().with(key), Sense::click());
        self.button_bg(ui, rect, &resp, false);
        let top = Rect::from_min_max(rect.min, egui::pos2(rect.max.x, rect.max.y - CHEVRON_H));
        self.paint_icon_fg(ui, top.center() + vec2(0.0, 2.0), 22.0, icon, self.resp_fg(&resp, true));
        self.paint_chevron(ui, egui::pos2(rect.center().x, rect.max.y - CHEVRON_H / 2.0));
        resp.on_hover_text(tip)
    }

    /// Icon with a small chevron to its right; the whole button opens a drop-down.
    fn dropdown_at(&self, ui: &mut Ui, rect: Rect, key: &str, icon: Icon, tip: &str) -> Response {
        let resp = ui.interact(rect, ui.id().with(key), Sense::click());
        self.button_bg(ui, rect, &resp, false);
        let icon_size = (rect.height() - 8.0).clamp(14.0, 20.0);
        let fg = self.resp_fg(&resp, true);
        self.paint_icon_fg(ui, egui::pos2(rect.min.x + 4.0 + icon_size / 2.0, rect.center().y), icon_size, icon, fg);
        self.paint_chevron(ui, egui::pos2(rect.max.x - 8.0, rect.center().y));
        resp.on_hover_text(tip)
    }

    fn separator_v(&self, ui: &mut Ui) {
        let (rect, _) = ui.allocate_exact_size(vec2(17.0, GROUP_H + LABEL_H), Sense::hover());
        ui.painter().line_segment(
            [egui::pos2(rect.center().x, rect.min.y + 6.0), egui::pos2(rect.center().x, rect.max.y - 6.0)],
            Stroke::new(1.0, self.pal.separator),
        );
    }

    pub fn ribbon(&mut self, ui: &mut Ui) {
        egui::ScrollArea::horizontal()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    self.clipboard_group(ui);
                    self.separator_v(ui);
                    self.image_group(ui);
                    self.separator_v(ui);
                    self.tools_group(ui);
                    self.separator_v(ui);
                    self.brushes_group(ui);
                    self.separator_v(ui);
                    self.shapes_group(ui);
                    self.separator_v(ui);
                    self.size_group(ui);
                    self.separator_v(ui);
                    self.colors_group(ui);
                    self.separator_v(ui);
                    self.layers_button(ui);
                });
            });
    }

    /// Reserve a group of fixed width; `add` receives the content rect (GROUP_H tall).
    fn with_group(&mut self, ui: &mut Ui, label: &str, width: f32, add: impl FnOnce(&mut Self, &mut Ui, Rect)) {
        let (rect, _) = ui.allocate_exact_size(vec2(width, GROUP_H + LABEL_H), Sense::hover());
        let content = Rect::from_min_size(rect.min, vec2(width, GROUP_H));
        add(self, ui, content);
        ui.painter().text(
            egui::pos2(rect.center().x, rect.max.y - LABEL_H / 2.0),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(11.5),
            self.pal.text_dim,
        );
    }

    fn clipboard_group(&mut self, ui: &mut Ui) {
        self.with_group(ui, "Clipboard", 94.0, |app, ui, c| {
            let (main, chev) = app.split_btn_at(ui, r(c.min.x + 2.0, c.min.y + 2.0, 44.0, 62.0), "paste", Icon::Paste, false, "Paste (Ctrl+V)");
            if main.clicked() {
                app.push(Action::Paste);
            }
            egui::Popup::menu(&chev).width(180.0).show(|ui| {
                if app.menu_item_icon(ui, Icon::Paste, "Paste", false) {
                    app.push(Action::Paste);
                }
                if app.menu_item_icon(ui, Icon::Paste, "Paste from file", false) {
                    app.push(Action::Import);
                }
            });
            let has_sel = app.doc.floating().is_some();
            let x = c.min.x + 50.0;
            if app.btn_at_enabled(ui, r(x, c.min.y + 2.0, 40.0, 30.0), "cut", Icon::Cut, false, "Cut (Ctrl+X)", has_sel).clicked() {
                app.push(Action::Cut);
            }
            if app.btn_at_enabled(ui, r(x, c.min.y + 34.0, 40.0, 30.0), "copy", Icon::Copy, false, "Copy (Ctrl+C)", has_sel).clicked() {
                app.push(Action::Copy);
            }
        });
    }

    fn layers_button(&mut self, ui: &mut Ui) {
        self.with_group(ui, "Layers", 52.0, |app, ui, c| {
            let rect = r(c.min.x + 2.0, c.min.y + 2.0, 48.0, 62.0);
            if app.btn_at(ui, rect, "layers", Icon::Layers, app.show_layers, "Layers").clicked() {
                app.show_layers = !app.show_layers;
            }
        });
    }

    fn image_group(&mut self, ui: &mut Ui) {
        self.with_group(ui, "Image", 180.0, |app, ui, c| {
            // Selection: big split button.
            let icon = if app.s.select_mode == SelectMode::FreeForm { Icon::SelectFree } else { Icon::SelectRect };
            let (main, chev) = app.split_btn_at(ui, r(c.min.x + 2.0, c.min.y + 2.0, 44.0, 62.0), "select", icon, app.s.tool == Tool::Select, "Selection");
            if main.clicked() {
                app.set_tool(Tool::Select);
            }
            egui::Popup::menu(&chev).width(230.0).show(|ui| {
                ui.label(RichText::new("Selection shapes").small().color(app.pal.text_dim));
                for (mode, icon, label) in [
                    (SelectMode::Rectangle, Icon::SelectRect, "Rectangle"),
                    (SelectMode::FreeForm, Icon::SelectFree, "Free-form"),
                ] {
                    if app.menu_item_icon(ui, icon, label, app.s.select_mode == mode) {
                        app.s.select_mode = mode;
                        app.set_tool(Tool::Select);
                    }
                }
                ui.separator();
                ui.label(RichText::new("Selection options").small().color(app.pal.text_dim));
                for (icon, label, a) in [
                    (Icon::SelectAll, "Select all", Action::SelectAll),
                    (Icon::Invert, "Invert selection", Action::InvertSelection),
                    (Icon::Delete, "Delete", Action::DeleteSelection),
                ] {
                    if app.menu_item_icon(ui, icon, label, false) {
                        app.push(a);
                    }
                }
                if app.menu_item_icon(ui, Icon::Transparent, "Transparent selection", app.s.transparent_selection) {
                    app.s.transparent_selection = !app.s.transparent_selection;
                }
            });
            // Crop / Rotate on top, Resize / Flip below (as in Win11), then Remove background.
            let x = c.min.x + 50.0;
            let has_sel = app.doc.floating().is_some();
            if app
                .btn_at_enabled(ui, r(x, c.min.y + 4.0, 36.0, 28.0), "crop", Icon::Crop, false, "Crop (Ctrl+Shift+X)", has_sel)
                .clicked()
            {
                app.push(Action::Crop);
            }
            if app.btn_at(ui, r(x, c.min.y + 34.0, 36.0, 28.0), "resize", Icon::Resize, false, "Resize and skew (Ctrl+W)").clicked() {
                app.push(Action::ResizeDialog);
            }
            let rot = app.dropdown_at(ui, r(x + 38.0, c.min.y + 4.0, 46.0, 28.0), "rotate", Icon::Rotate, "Rotate");
            egui::Popup::menu(&rot).width(200.0).show(|ui| {
                for (icon, label, a) in [
                    (Icon::RotateRight, "Rotate right 90°", Action::RotateRight),
                    (Icon::RotateLeft, "Rotate left 90°", Action::RotateLeft),
                    (Icon::Rotate180, "Rotate 180°", Action::Rotate180),
                    (Icon::Rotate, "Custom rotate…", Action::CustomRotate),
                ] {
                    if app.menu_item_icon(ui, icon, label, false) {
                        app.push(a);
                    }
                }
            });
            let flip = app.dropdown_at(ui, r(x + 38.0, c.min.y + 34.0, 46.0, 28.0), "flip", Icon::Flip, "Flip");
            egui::Popup::menu(&flip).width(190.0).show(|ui| {
                for (icon, label, a) in [
                    (Icon::FlipV, "Flip vertical", Action::FlipVertical),
                    (Icon::FlipH, "Flip horizontal", Action::FlipHorizontal),
                ] {
                    if app.menu_item_icon(ui, icon, label, false) {
                        app.push(a);
                    }
                }
            });
            if app
                .btn_at(ui, r(x + 88.0, c.min.y + 4.0, 40.0, 58.0), "removebg", Icon::RemoveBg, false, "Remove background")
                .clicked()
            {
                app.push(Action::RemoveBackground);
            }
        });
    }

    /// Menu entry with an icon in a fixed gutter; `checked` shows a tick on the right.
    fn menu_item_icon(&self, ui: &mut Ui, icon: Icon, label: &str, checked: bool) -> bool {
        let width = ui.available_width().max(180.0);
        let (rect, resp) = ui.allocate_exact_size(vec2(width, 28.0), Sense::click());
        self.button_bg(ui, rect, &resp, false);
        self.paint_icon(ui, egui::pos2(rect.min.x + 16.0, rect.center().y), 16.0, icon, true);
        ui.painter().text(
            egui::pos2(rect.min.x + 34.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(13.5),
            self.pal.text,
        );
        if checked {
            ui.painter().text(
                egui::pos2(rect.max.x - 10.0, rect.center().y),
                egui::Align2::RIGHT_CENTER,
                "✔",
                egui::FontId::proportional(13.0),
                self.pal.accent,
            );
        }
        if resp.clicked() {
            ui.close();
        }
        resp.clicked()
    }

    fn tools_group(&mut self, ui: &mut Ui) {
        self.with_group(ui, "Tools", 104.0, |app, ui, c| {
            let items = [
                (Tool::Pencil, Icon::Pencil, "Pencil"),
                (Tool::Fill, Icon::Fill, "Fill with color"),
                (Tool::Text, Icon::Text, "Text"),
                (Tool::Eraser, Icon::Eraser, "Eraser"),
                (Tool::Picker, Icon::Picker, "Color picker"),
                (Tool::Magnifier, Icon::Magnifier, "Magnifier"),
            ];
            let (bw, bh, gap) = (32.0, 30.0, 2.0);
            let x0 = c.center().x - (3.0 * bw + 2.0 * gap) / 2.0;
            let y0 = c.center().y - (2.0 * bh + gap) / 2.0;
            for (i, (t, icon, tip)) in items.into_iter().enumerate() {
                let (col, row) = ((i % 3) as f32, (i / 3) as f32);
                let rect = r(x0 + col * (bw + gap), y0 + row * (bh + gap), bw, bh);
                if app.btn_at(ui, rect, tip, icon, app.s.tool == t, tip).clicked() {
                    app.set_tool(t);
                }
            }
        });
    }

    fn brushes_group(&mut self, ui: &mut Ui) {
        self.with_group(ui, "Brushes", 52.0, |app, ui, c| {
            let b = app.s.brush;
            let (main, chev) = app.split_btn_at(ui, r(c.min.x + 2.0, c.min.y + 2.0, 48.0, 62.0), "brush", Icon::Brush(b), app.s.tool == Tool::Brush, b.name());
            if main.clicked() {
                app.set_tool(Tool::Brush);
            }
            // Win11 lists the brushes by name with a preview of their stroke.
            let previews = app.brush_previews(ui.ctx());
            egui::Popup::menu(&chev).width(300.0).show(|ui| {
                for (i, k) in BrushKind::ALL.into_iter().enumerate() {
                    let (row, resp) = ui.allocate_exact_size(vec2(290.0, 40.0), Sense::click());
                    let sel = app.s.tool == Tool::Brush && app.s.brush == k;
                    app.button_bg(ui, row, &resp, sel);
                    app.paint_icon(ui, egui::pos2(row.min.x + 20.0, row.center().y), 20.0, Icon::Brush(k), true);
                    ui.painter().text(
                        egui::pos2(row.min.x + 40.0, row.center().y),
                        egui::Align2::LEFT_CENTER,
                        k.name(),
                        egui::FontId::proportional(13.5),
                        app.pal.text,
                    );
                    if let Some(t) = previews.get(i) {
                        let pr = Rect::from_min_size(egui::pos2(row.max.x - 128.0, row.center().y - 13.0), vec2(120.0, 26.0));
                        ui.painter().image(t.id(), pr, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                    }
                    if resp.clicked() {
                        app.s.brush = k;
                        app.set_tool(Tool::Brush);
                        ui.close();
                    }
                }
            });
        });
    }

    /// Stroke previews for the brush list, rendered with the real brush code.
    fn brush_previews(&mut self, ctx: &egui::Context) -> Vec<egui::TextureHandle> {
        let dark = self.pal.dark;
        if self.brush_preview_dark != Some(dark) || self.brush_preview_tex.is_empty() {
            let ink = if dark { Color32::from_gray(235) } else { Color32::from_gray(30) };
            self.brush_preview_tex = BrushKind::ALL
                .iter()
                .map(|&k| {
                    let (w, h) = (240u32, 52u32);
                    let mut layer = crate::pixels::new_pixmap(w, h, Color32::TRANSPARENT);
                    let mut sc = crate::brush::StrokeCtx::new(10.0);
                    sc.distance = -1.0e6; // previews never run out of paint
                    let pts: Vec<egui::Pos2> = (0..=60)
                        .map(|i| {
                            let t = i as f32 / 60.0;
                            egui::pos2(14.0 + t * (w as f32 - 28.0), h as f32 / 2.0 - (t * std::f32::consts::TAU).sin() * 12.0)
                        })
                        .collect();
                    for seg in pts.windows(2) {
                        crate::brush::brush_segment(&mut layer, k, seg[0], seg[1], 10.0, ink, &mut sc);
                    }
                    // Apply the brush's layer opacity (marker, watercolor...).
                    let op = k.layer_opacity();
                    if op < 1.0 {
                        for px in layer.data_mut().as_chunks_mut::<4>().0.iter_mut() {
                            for c in px.iter_mut() {
                                *c = (*c as f32 * op) as u8;
                            }
                        }
                    }
                    ctx.load_texture(format!("brush-preview-{k:?}"), crate::pixels::to_color_image(&layer), egui::TextureOptions::LINEAR)
                })
                .collect();
            self.brush_preview_dark = Some(dark);
        }
        self.brush_preview_tex.clone()
    }

    fn shapes_group(&mut self, ui: &mut Ui) {
        let (cw, ch, cols) = (24.0, 20.0, 8usize);
        let grid_w = cw * cols as f32 + 6.0;
        self.with_group(ui, "Shapes", grid_w + 6.0 + 52.0, |app, ui, c| {
            // Shape gallery in a bordered box.
            let frame = r(c.min.x, c.center().y - (3.0 * ch + 6.0) / 2.0, grid_w, 3.0 * ch + 6.0);
            ui.painter().rect_stroke(frame, 4.0, Stroke::new(1.0, app.pal.separator), StrokeKind::Inside);
            for (i, k) in ShapeKind::ALL.into_iter().enumerate() {
                let (col, row) = ((i % cols) as f32, (i / cols) as f32);
                let rect = r(frame.min.x + 3.0 + col * cw, frame.min.y + 3.0 + row * ch, cw, ch);
                let sel = app.s.tool == Tool::Shape && app.s.shape == k;
                if app.btn_at(ui, rect, k.name(), Icon::Shape(k), sel, k.name()).clicked() {
                    app.doc.commit_overlay(&app.s);
                    app.s.shape = k;
                    app.set_tool(Tool::Shape);
                }
            }
            // Outline and Fill drop-downs.
            let x = frame.max.x + 6.0;
            for (i, outline) in [true, false].into_iter().enumerate() {
                let (icon, tip, key) = if outline {
                    (Icon::Outline, "Shape outline", "outline")
                } else {
                    (Icon::FillStyle, "Shape fill", "fill")
                };
                let rect = r(x, c.min.y + 3.0 + i as f32 * 31.0, 52.0, 29.0);
                let resp = app.dropdown_at(ui, rect, key, icon, tip);
                let cur = if outline { app.s.outline } else { app.s.fill };
                egui::Popup::menu(&resp).width(170.0).show(|ui| {
                    for st in ShapeStyle::ALL {
                        let name = if outline { st.outline_name() } else { st.fill_name() };
                        if ui.add(egui::Button::new(name).selected(cur == st)).clicked() {
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
    }

    fn size_group(&mut self, ui: &mut Ui) {
        self.with_group(ui, "Size", 52.0, |app, ui, c| {
            let resp = app.stacked_dropdown_at(ui, r(c.min.x + 2.0, c.min.y + 2.0, 48.0, 62.0), "size", Icon::Size, "Size");
            egui::Popup::menu(&resp)
                .width(180.0)
                .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    let tool = match app.s.tool {
                        Tool::Select | Tool::Fill | Tool::Picker | Tool::Magnifier | Tool::Text => Tool::Brush,
                        t => t,
                    };
                    for &sz in size_presets(tool) {
                        let cur = app.size_for(tool);
                        let (rect, resp) = ui.allocate_exact_size(vec2(170.0, 26.0), Sense::click());
                        app.button_bg(ui, rect, &resp, (cur - sz).abs() < 0.01);
                        ui.painter().text(
                            rect.left_center() + vec2(8.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            format!("{sz}px"),
                            egui::FontId::proportional(12.0),
                            app.pal.text_dim,
                        );
                        let y = rect.center().y;
                        ui.painter().line_segment(
                            [egui::pos2(rect.min.x + 50.0, y), egui::pos2(rect.max.x - 10.0, y)],
                            Stroke::new(sz.min(20.0), app.pal.text),
                        );
                        if resp.clicked() {
                            *app.size_mut_for(tool) = sz;
                        }
                    }
                    ui.separator();
                    let max = if tool == Tool::Eraser { 100.0 } else { 50.0 };
                    let size = app.size_mut_for(tool);
                    ui.add(egui::Slider::new(size, 1.0..=max).step_by(1.0).suffix("px"));
                });
        });
    }

    fn size_for(&self, tool: Tool) -> f32 {
        let mut s = self.s.clone();
        s.tool = tool;
        s.size()
    }

    fn size_mut_for(&mut self, tool: Tool) -> &mut f32 {
        match tool {
            Tool::Pencil => &mut self.s.pencil_size,
            Tool::Eraser => &mut self.s.eraser_size,
            Tool::Shape => &mut self.s.shape_size,
            _ => &mut self.s.brush_size,
        }
    }

    fn color_circle_at(&self, ui: &mut Ui, rect: Rect, key: &str, c: Color32, selected: bool, tip: &str) -> Response {
        let resp = ui.interact(rect, ui.id().with(key), Sense::click());
        let p = ui.painter();
        let center = rect.center();
        let d = rect.width().min(rect.height()) - 5.0;
        if selected {
            p.circle_stroke(center, d / 2.0 + 2.0, Stroke::new(2.0, self.pal.accent));
        } else if resp.hovered() {
            p.circle_stroke(center, d / 2.0 + 1.5, Stroke::new(1.5, self.pal.text_dim));
        }
        if c == Color32::TRANSPARENT {
            p.circle_stroke(center, d / 2.0 - 0.5, Stroke::new(1.0, self.pal.separator));
        } else {
            p.circle_filled(center, d / 2.0, c);
            p.circle_stroke(center, d / 2.0, Stroke::new(1.0, Color32::from_black_alpha(50)));
        }
        if tip.is_empty() { resp } else { resp.on_hover_text(tip) }
    }

    fn colors_group(&mut self, ui: &mut Ui) {
        let cell = 21.0;
        let pal_w = cell * 10.0;
        self.with_group(ui, "Colors", 36.0 + 8.0 + pal_w + 8.0 + 46.0, |app, ui, c| {
            // Color 1 / Color 2, stacked.
            let x = c.min.x + 2.0;
            let c1 = app.color_circle_at(ui, r(x, c.min.y + 1.0, 32.0, 32.0), "c1", app.s.color1, app.active_slot == 1, "Color 1 (foreground)");
            if c1.clicked() {
                app.active_slot = 1;
            }
            let c2 = app.color_circle_at(ui, r(x, c.min.y + 33.0, 32.0, 32.0), "c2", app.s.color2, app.active_slot == 2, "Color 2 (background)");
            if c2.clicked() {
                app.active_slot = 2;
            }
            // Palette: 2 standard rows + 1 custom row.
            let px0 = c.min.x + 44.0;
            let py0 = c.center().y - 1.5 * cell;
            for i in 0..30 {
                let (col, row) = ((i % 10) as f32, (i / 10) as f32);
                let rect = r(px0 + col * cell, py0 + row * cell, cell, cell);
                let color = if let Some(rgb) = PALETTE.get(i) {
                    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
                } else {
                    app.s.custom_colors.get(i - 20).copied().unwrap_or(Color32::TRANSPARENT)
                };
                let resp = app.color_circle_at(ui, rect, &format!("pal{i}"), color, false, "");
                if color == Color32::TRANSPARENT {
                    if resp.clicked() {
                        app.push(Action::EditColors);
                    }
                    continue;
                }
                // Shift+click (or right-click) a swatch sets color 2.
                if resp.clicked() && ui.input(|i| i.modifiers.shift) {
                    app.s.color2 = color;
                } else if resp.clicked() {
                    app.set_active_color(color);
                }
                if resp.secondary_clicked() {
                    app.s.color2 = color;
                }
            }
            // Edit colors (rainbow ring).
            let rect = r(px0 + pal_w + 8.0, c.min.y + 2.0, 44.0, 62.0);
            let resp = ui.interact(rect, ui.id().with("edit_colors"), Sense::click());
            app.button_bg(ui, rect, &resp, false);
            let ctr = rect.center();
            for i in 0..36 {
                let a0 = i as f32 / 36.0 * std::f32::consts::TAU;
                let a1 = (i + 1) as f32 / 36.0 * std::f32::consts::TAU;
                let col = egui::ecolor::Hsva::new(i as f32 / 36.0, 0.85, 1.0, 1.0);
                let pts = vec![
                    ctr + vec2(a0.cos(), a0.sin()) * 8.0,
                    ctr + vec2(a0.cos(), a0.sin()) * 14.0,
                    ctr + vec2(a1.cos(), a1.sin()) * 14.0,
                    ctr + vec2(a1.cos(), a1.sin()) * 8.0,
                ];
                ui.painter().add(egui::Shape::convex_polygon(pts, Color32::from(col), Stroke::NONE));
            }
            ui.painter().text(ctr, egui::Align2::CENTER_CENTER, "+", egui::FontId::proportional(13.0), app.pal.text);
            if resp.on_hover_text("Edit colors").clicked() {
                app.push(Action::EditColors);
            }
        });
    }

    pub fn set_active_color(&mut self, c: Color32) {
        if self.active_slot == 2 {
            self.s.color2 = c;
        } else {
            self.s.color1 = c;
        }
    }

    /// Square toggle used by the text toolbar; selected state gets an accent border.
    fn toggle_box(&self, ui: &mut Ui, selected: bool, tip: &str, paint: impl FnOnce(&Ui, Rect, Color32)) -> bool {
        let (rect, resp) = ui.allocate_exact_size(vec2(36.0, 32.0), Sense::click());
        self.button_bg(ui, rect, &resp, selected);
        paint(ui, rect, self.resp_fg(&resp, true));
        resp.on_hover_text(tip).clicked()
    }

    /// Contextual text formatting bar shown while a text box is active.
    pub fn text_bar(&mut self, ui: &mut Ui) {
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 0.0);
            ui.spacing_mut().interact_size.y = 32.0;
            let fams = self.fm.families.clone();
            egui::ComboBox::from_id_salt("font_family")
                .selected_text(self.s.text.family.clone())
                .width(190.0)
                .height(420.0)
                .show_ui(ui, |ui| {
                    for f in &fams {
                        ui.selectable_value(&mut self.s.text.family, f.clone(), f);
                    }
                })
                .response
                .on_hover_text("Font");
            egui::ComboBox::from_id_salt("font_size")
                .selected_text(format!("{}", self.s.text.size_pt))
                .width(64.0)
                .height(420.0)
                .show_ui(ui, |ui| {
                    for sz in [8.0, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0, 36.0, 48.0, 72.0] {
                        ui.selectable_value(&mut self.s.text.size_pt, sz, format!("{sz}"));
                    }
                })
                .response
                .on_hover_text("Font size");
            ui.separator();
            let letter = |text: RichText| {
                move |ui: &Ui, rect: Rect, fg: Color32| {
                    let galley = egui::WidgetText::from(text.size(17.0).color(fg)).into_galley(
                        ui,
                        Some(egui::TextWrapMode::Extend),
                        f32::INFINITY,
                        egui::TextStyle::Body,
                    );
                    let pos = rect.center() - galley.size() / 2.0;
                    ui.painter().galley(pos, galley, fg);
                }
            };
            let t = self.s.text.clone();
            let mut new = t.clone();
            if self.toggle_box(ui, t.bold, "Bold", letter(RichText::new("B").strong())) {
                new.bold = !t.bold;
            }
            if self.toggle_box(ui, t.italic, "Italic", letter(RichText::new("I").italics())) {
                new.italic = !t.italic;
            }
            if self.toggle_box(ui, t.underline, "Underline", letter(RichText::new("U").underline())) {
                new.underline = !t.underline;
            }
            if self.toggle_box(ui, t.strikethrough, "Strikethrough", letter(RichText::new("S").strikethrough())) {
                new.strikethrough = !t.strikethrough;
            }
            ui.separator();
            let accent = self.pal.accent;
            for (align, icon, tip) in [(0u8, Icon::AlignLeft, "Align left"), (1, Icon::AlignCenter, "Center"), (2, Icon::AlignRight, "Align right")] {
                let paint = move |ui: &Ui, rect: Rect, fg: Color32| {
                    icons::paint(ui.painter(), Rect::from_center_size(rect.center(), vec2(20.0, 20.0)), icon, fg, accent);
                };
                if self.toggle_box(ui, t.align == align, tip, paint) {
                    new.align = align;
                }
            }
            ui.separator();
            ui.checkbox(&mut new.opaque, "Background fill");
            self.s.text = new;
        });
    }
}
