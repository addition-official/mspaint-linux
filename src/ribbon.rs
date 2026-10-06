//! The Windows 11 style toolbar ("ribbon"): Image, Tools, Brushes, Shapes, Size, Colours.

use crate::app::{Action, PaintApp};
use crate::icons::{self, Icon};
use crate::tools::{BrushKind, SelectMode, ShapeKind, ShapeStyle, Tool, size_presets};
use egui::{Align, Color32, Layout, PopupCloseBehavior, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui, vec2};

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

const GROUP_H: f32 = 64.0;

impl PaintApp {
    /// A flat icon button with Fluent-style hover/selected backgrounds.
    fn icon_btn(&self, ui: &mut Ui, icon: Icon, size: egui::Vec2, selected: bool, tip: &str) -> Response {
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
        let pal = self.pal;
        let p = ui.painter();
        if selected {
            p.rect_filled(rect, 4.0, pal.selected);
            p.rect_stroke(rect.shrink(0.5), 4.0, Stroke::new(1.0, pal.accent.gamma_multiply(0.45)), StrokeKind::Inside);
        } else if resp.hovered() {
            p.rect_filled(rect, 4.0, pal.hover);
        }
        let s = (size.y - 10.0).clamp(14.0, 22.0);
        icons::paint(p, Rect::from_center_size(rect.center(), vec2(s, s)), icon, pal.text, pal.accent);
        resp.on_hover_text(tip)
    }

    /// Small chevron button that toggles a popup.
    fn chevron_btn(&self, ui: &mut Ui, size: egui::Vec2, tip: &str) -> Response {
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(rect, 4.0, self.pal.hover);
        }
        icons::paint(
            ui.painter(),
            Rect::from_center_size(rect.center(), vec2(10.0, 10.0)),
            Icon::Chevron,
            self.pal.text_dim,
            self.pal.accent,
        );
        resp.on_hover_text(tip)
    }

    fn separator_v(&self, ui: &mut Ui) {
        let (r, _) = ui.allocate_exact_size(vec2(13.0, GROUP_H + 12.0), Sense::hover());
        ui.painter().line_segment(
            [egui::pos2(r.center().x, r.min.y + 6.0), egui::pos2(r.center().x, r.max.y - 4.0)],
            Stroke::new(1.0, self.pal.separator),
        );
    }

    pub fn ribbon(&mut self, ui: &mut Ui) {
        egui::ScrollArea::horizontal().scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
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
            });
        });
    }

    fn with_group(&mut self, ui: &mut Ui, label: &str, add: impl FnOnce(&mut Self, &mut Ui)) {
        ui.vertical(|ui| {
            let r = ui.allocate_ui_with_layout(vec2(0.0, GROUP_H), Layout::left_to_right(Align::Center), |ui| {
                ui.set_min_height(GROUP_H);
                add(self, ui)
            });
            let w = r.response.rect.width();
            ui.allocate_ui_with_layout(vec2(w, 14.0), Layout::top_down(Align::Center), |ui| {
                ui.set_width(w);
                ui.label(RichText::new(label).size(11.0).color(self.pal.text_dim));
            });
        });
    }

    fn image_group(&mut self, ui: &mut Ui) {
        self.with_group(ui, "Image", |app, ui| {
            // Selection: big button + drop-down.
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                let icon = if app.s.select_mode == SelectMode::FreeForm { Icon::SelectFree } else { Icon::SelectRect };
                if app.icon_btn(ui, icon, vec2(44.0, 40.0), app.s.tool == Tool::Select, "Selection").clicked() {
                    app.set_tool(Tool::Select);
                }
                let chev = app.chevron_btn(ui, vec2(44.0, 18.0), "Selection options");
                egui::Popup::menu(&chev).width(220.0).show(|ui| {
                    ui.label(RichText::new("Selection shapes").small().color(app.pal.text_dim));
                    if ui.add(egui::Button::new("Rectangle").selected(app.s.select_mode == SelectMode::Rectangle)).clicked() {
                        app.s.select_mode = SelectMode::Rectangle;
                        app.set_tool(Tool::Select);
                    }
                    if ui.add(egui::Button::new("Free-form").selected(app.s.select_mode == SelectMode::FreeForm)).clicked() {
                        app.s.select_mode = SelectMode::FreeForm;
                        app.set_tool(Tool::Select);
                    }
                    ui.separator();
                    ui.label(RichText::new("Selection options").small().color(app.pal.text_dim));
                    for (icon, label, a) in [
                        (Icon::SelectAll, "Select all", Action::SelectAll),
                        (Icon::Invert, "Invert selection", Action::InvertSelection),
                        (Icon::Delete, "Delete", Action::DeleteSelection),
                    ] {
                        if app.menu_item_icon(ui, icon, label) {
                            app.push(a);
                        }
                    }
                    let label = if app.s.transparent_selection { "Transparent selection  ✔" } else { "Transparent selection" };
                    if app.menu_item_icon(ui, Icon::Transparent, label) {
                        app.s.transparent_selection = !app.s.transparent_selection;
                    }
                });
            });
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 1.0);
                let has_sel = app.doc.floating().is_some();
                ui.add_enabled_ui(has_sel, |ui| {
                    if app.icon_btn(ui, Icon::Crop, vec2(32.0, 20.0), false, "Crop (Ctrl+Shift+X)").clicked() {
                        app.push(Action::Crop);
                    }
                });
                if app.icon_btn(ui, Icon::Resize, vec2(32.0, 20.0), false, "Resize and skew (Ctrl+W)").clicked() {
                    app.push(Action::ResizeDialog);
                }
                let rot = app.icon_btn(ui, Icon::Rotate, vec2(32.0, 20.0), false, "Rotate");
                egui::Popup::menu(&rot).width(200.0).show(|ui| {
                    for (icon, label, a) in [
                        (Icon::RotateRight, "Rotate right 90°", Action::RotateRight),
                        (Icon::RotateLeft, "Rotate left 90°", Action::RotateLeft),
                        (Icon::Rotate180, "Rotate 180°", Action::Rotate180),
                        (Icon::FlipV, "Flip vertical", Action::FlipVertical),
                        (Icon::FlipH, "Flip horizontal", Action::FlipHorizontal),
                    ] {
                        if app.menu_item_icon(ui, icon, label) {
                            app.push(a);
                        }
                    }
                });
            });
        });
    }

    fn menu_item_icon(&self, ui: &mut Ui, icon: Icon, label: &str) -> bool {
        let resp = ui.add(egui::Button::new(format!("        {label}")));
        let r = Rect::from_min_size(resp.rect.left_center() + vec2(6.0, -8.0), vec2(16.0, 16.0));
        icons::paint(ui.painter(), r, icon, self.pal.text, self.pal.accent);
        resp.clicked()
    }

    fn tools_group(&mut self, ui: &mut Ui) {
        self.with_group(ui, "Tools", |app, ui| {
            egui::Grid::new("tools_grid").spacing(vec2(2.0, 2.0)).show(ui, |ui| {
                let items = [
                    (Tool::Pencil, Icon::Pencil, "Pencil"),
                    (Tool::Fill, Icon::Fill, "Fill with colour"),
                    (Tool::Text, Icon::Text, "Text"),
                    (Tool::Eraser, Icon::Eraser, "Eraser"),
                    (Tool::Picker, Icon::Picker, "Colour picker"),
                    (Tool::Magnifier, Icon::Magnifier, "Magnifier"),
                ];
                for (i, (t, icon, tip)) in items.into_iter().enumerate() {
                    if app.icon_btn(ui, icon, vec2(30.0, 28.0), app.s.tool == t, tip).clicked() {
                        app.set_tool(t);
                    }
                    if i == 2 {
                        ui.end_row();
                    }
                }
            });
        });
    }

    fn brushes_group(&mut self, ui: &mut Ui) {
        self.with_group(ui, "Brushes", |app, ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                let b = app.s.brush;
                if app.icon_btn(ui, Icon::Brush(b), vec2(44.0, 40.0), app.s.tool == Tool::Brush, b.name()).clicked() {
                    app.set_tool(Tool::Brush);
                }
                let chev = app.chevron_btn(ui, vec2(44.0, 18.0), "Brushes");
                egui::Popup::menu(&chev).show(|ui| {
                    egui::Grid::new("brush_grid").spacing(vec2(4.0, 4.0)).show(ui, |ui| {
                        for (i, k) in BrushKind::ALL.into_iter().enumerate() {
                            let sel = app.s.tool == Tool::Brush && app.s.brush == k;
                            if app.icon_btn(ui, Icon::Brush(k), vec2(40.0, 40.0), sel, k.name()).clicked() {
                                app.s.brush = k;
                                app.set_tool(Tool::Brush);
                                ui.close();
                            }
                            if i % 3 == 2 {
                                ui.end_row();
                            }
                        }
                    });
                });
            });
        });
    }

    fn shapes_group(&mut self, ui: &mut Ui) {
        self.with_group(ui, "Shapes", |app, ui| {
            egui::Frame::new()
                .stroke(Stroke::new(1.0, app.pal.separator))
                .corner_radius(4.0)
                .inner_margin(2.0)
                .show(ui, |ui| {
                    egui::Grid::new("shape_grid").spacing(vec2(0.0, 0.0)).show(ui, |ui| {
                        for (i, k) in ShapeKind::ALL.into_iter().enumerate() {
                            let sel = app.s.tool == Tool::Shape && app.s.shape == k;
                            if app.icon_btn(ui, Icon::Shape(k), vec2(22.0, 19.0), sel, k.name()).clicked() {
                                app.doc.commit_overlay(&app.s);
                                app.s.shape = k;
                                app.set_tool(Tool::Shape);
                            }
                            if i % 8 == 7 {
                                ui.end_row();
                            }
                        }
                    });
                });
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
                for (outline, icon) in [(true, Icon::Outline), (false, Icon::FillStyle)] {
                    let cur = if outline { app.s.outline } else { app.s.fill };
                    let tip = if outline { "Shape outline" } else { "Shape fill" };
                    let r = app.icon_btn(ui, icon, vec2(36.0, 28.0), cur != ShapeStyle::None, tip);
                    egui::Popup::menu(&r).width(170.0).show(|ui| {
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
        });
    }

    fn size_group(&mut self, ui: &mut Ui) {
        self.with_group(ui, "Size", |app, ui| {
            let r = app.icon_btn(ui, Icon::Size, vec2(44.0, 56.0), false, "Size");
            egui::Popup::menu(&r)
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
                        if (cur - sz).abs() < 0.01 {
                            ui.painter().rect_filled(rect, 4.0, app.pal.selected);
                        } else if resp.hovered() {
                            ui.painter().rect_filled(rect, 4.0, app.pal.hover);
                        }
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

    fn color_circle(&self, ui: &mut Ui, c: Color32, d: f32, selected: bool, tip: &str) -> Response {
        let (rect, resp) = ui.allocate_exact_size(vec2(d + 4.0, d + 4.0), Sense::click());
        let p = ui.painter();
        let center = rect.center();
        if selected {
            p.circle_stroke(center, d / 2.0 + 1.5, Stroke::new(2.0, self.pal.accent));
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
        self.with_group(ui, "Colours", |app, ui| {
            // Colour 1 / Colour 2.
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
                let c1 = app.color_circle(ui, app.s.color1, 26.0, app.active_slot == 1, "Colour 1 (foreground)");
                if c1.clicked() {
                    app.active_slot = 1;
                }
                let c2 = app.color_circle(ui, app.s.color2, 26.0, app.active_slot == 2, "Colour 2 (background)");
                if c2.clicked() {
                    app.active_slot = 2;
                }
            });
            ui.add_space(6.0);
            egui::Grid::new("palette").spacing(vec2(0.0, 0.0)).show(ui, |ui| {
                for (i, rgb) in PALETTE.iter().enumerate() {
                    let c = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                    let r = app.color_circle(ui, c, 16.0, false, "");
                    if r.clicked() {
                        app.set_active_color(c);
                    }
                    if r.secondary_clicked() {
                        app.s.color2 = c;
                    }
                    if i % 10 == 9 {
                        ui.end_row();
                    }
                }
                for i in 0..10 {
                    let c = app.s.custom_colors.get(i).copied().unwrap_or(Color32::TRANSPARENT);
                    let r = app.color_circle(ui, c, 16.0, false, "");
                    if c != Color32::TRANSPARENT {
                        if r.clicked() {
                            app.set_active_color(c);
                        }
                        if r.secondary_clicked() {
                            app.s.color2 = c;
                        }
                    } else if r.clicked() {
                        app.push(Action::EditColors);
                    }
                }
                ui.end_row();
            });
            ui.add_space(4.0);
            // Edit colours (rainbow ring).
            let (rect, resp) = ui.allocate_exact_size(vec2(40.0, 56.0), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(rect, 4.0, app.pal.hover);
            }
            let c = rect.center();
            for i in 0..36 {
                let a0 = i as f32 / 36.0 * std::f32::consts::TAU;
                let a1 = (i + 1) as f32 / 36.0 * std::f32::consts::TAU;
                let col = egui::ecolor::Hsva::new(i as f32 / 36.0, 0.85, 1.0, 1.0);
                let pts = vec![
                    c + vec2(a0.cos(), a0.sin()) * 7.0,
                    c + vec2(a0.cos(), a0.sin()) * 14.0,
                    c + vec2(a1.cos(), a1.sin()) * 14.0,
                    c + vec2(a1.cos(), a1.sin()) * 7.0,
                ];
                ui.painter().add(egui::Shape::convex_polygon(pts, Color32::from(col), Stroke::NONE));
            }
            ui.painter().text(
                c + vec2(0.0, 0.0),
                egui::Align2::CENTER_CENTER,
                "+",
                egui::FontId::proportional(13.0),
                app.pal.text,
            );
            if resp.on_hover_text("Edit colours").clicked() {
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

    /// Contextual text formatting bar shown while a text box is active.
    pub fn text_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Text").strong());
            ui.add_space(8.0);
            let fams = self.fm.families.clone();
            egui::ComboBox::from_id_salt("font_family")
                .selected_text(self.s.text.family.clone())
                .width(200.0)
                .height(400.0)
                .show_ui(ui, |ui| {
                    for f in &fams {
                        ui.selectable_value(&mut self.s.text.family, f.clone(), f);
                    }
                });
            egui::ComboBox::from_id_salt("font_size")
                .selected_text(format!("{}", self.s.text.size_pt))
                .width(60.0)
                .show_ui(ui, |ui| {
                    for sz in [8.0, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0, 36.0, 48.0, 72.0] {
                        ui.selectable_value(&mut self.s.text.size_pt, sz, format!("{sz}"));
                    }
                });
            ui.add(egui::DragValue::new(&mut self.s.text.size_pt).range(4.0..=400.0).speed(0.5));
            ui.separator();
            let t = &mut self.s.text;
            ui.toggle_value(&mut t.bold, RichText::new("B").strong()).on_hover_text("Bold");
            ui.toggle_value(&mut t.italic, RichText::new("I").italics()).on_hover_text("Italic");
            ui.toggle_value(&mut t.underline, RichText::new("U").underline()).on_hover_text("Underline");
            ui.toggle_value(&mut t.strikethrough, RichText::new("S").strikethrough()).on_hover_text("Strikethrough");
            ui.separator();
            ui.selectable_value(&mut t.opaque, false, "Transparent background");
            ui.selectable_value(&mut t.opaque, true, "Fill background");
        });
    }
}
