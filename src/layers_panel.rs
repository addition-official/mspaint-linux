//! The Layers side panel (Windows 11 Paint shows it on the right of the canvas).

use crate::app::{Action, PaintApp, content_dialog};
use crate::icons::{self, Icon};
use crate::pixels;
use egui::{Color32, Key, Rect, Sense, Stroke, StrokeKind, TextureOptions, Ui, vec2};

const THUMB: egui::Vec2 = vec2(72.0, 48.0);

impl PaintApp {
    /// Regenerate layer thumbnails when the layers changed (throttled while drawing).
    fn update_thumbnails(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        let stale = self.thumbs_rev != self.doc.layers_rev || self.layer_thumbs.len() != self.doc.layers.len();
        if !stale {
            return;
        }
        if now - self.thumbs_time < 0.25 && self.layer_thumbs.len() == self.doc.layers.len() {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
            return;
        }
        self.thumbs_time = now;
        self.thumbs_rev = self.doc.layers_rev;
        let (w, h) = (self.doc.width() as f32, self.doc.height() as f32);
        let scale = (THUMB.x / w).min(THUMB.y / h).min(1.0);
        let (tw, th) = ((w * scale).round().max(1.0) as u32, (h * scale).round().max(1.0) as u32);
        self.layer_thumbs = (0..self.doc.layers.len())
            .map(|i| {
                let small = pixels::resize(self.doc.layer_pix(i), tw, th);
                ctx.load_texture(format!("layer-thumb-{i}"), pixels::to_color_image(&small), TextureOptions::LINEAR)
            })
            .collect();
    }

    pub fn layers_panel(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        self.update_thumbnails(&ctx);
        let pal = self.pal;
        let s = self.s.clone();

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Layers").size(15.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (r, resp) = ui.allocate_exact_size(vec2(30.0, 28.0), Sense::click());
                pal.paint_button(ui.painter(), r, resp.hovered(), resp.is_pointer_button_down_on(), false);
                icons::paint(ui.painter(), Rect::from_center_size(r.center(), vec2(18.0, 18.0)), Icon::Plus, pal.text, pal.accent);
                if resp.on_hover_text("Add layer").clicked() {
                    self.doc.add_layer(&s);
                }
            });
        });
        ui.add_space(6.0);

        let mut rows: Vec<(usize, Rect)> = Vec::new();
        egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
            // Top layer first, as in Paint.
            for i in (0..self.doc.layers.len()).rev() {
                let selected = i == self.doc.active;
                let (row, resp) = ui.allocate_exact_size(vec2(ui.available_width(), THUMB.y + 16.0), Sense::click_and_drag());
                rows.push((i, row));
                if resp.drag_started() {
                    self.layer_drag = Some(i);
                }
                let p = ui.painter();
                pal.paint_button(p, row, resp.hovered(), resp.is_pointer_button_down_on(), selected);
                // Thumbnail on a checkerboard so transparency is visible.
                let thumb_area = Rect::from_min_size(row.min + vec2(8.0, 8.0), THUMB);
                if let Some(t) = self.layer_thumbs.get(i) {
                    let size = t.size_vec2();
                    let tr = Rect::from_center_size(thumb_area.center(), size);
                    self.paint_checker(p, tr);
                    p.image(t.id(), tr, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                    p.rect_stroke(tr, 0.0, Stroke::new(1.0, pal.separator), StrokeKind::Outside);
                }
                let layer = &self.doc.layers[i];
                let name_color = if layer.visible { pal.text } else { pal.text_dim };
                p.text(
                    egui::pos2(thumb_area.max.x + 10.0, row.center().y),
                    egui::Align2::LEFT_CENTER,
                    &layer.name,
                    egui::FontId::proportional(13.0),
                    name_color,
                );
                // Visibility toggle.
                let eye = Rect::from_center_size(egui::pos2(row.max.x - 20.0, row.center().y), vec2(28.0, 28.0));
                let eye_resp = ui.interact(eye, ui.id().with(("eye", i)), Sense::click());
                pal.paint_button(ui.painter(), eye, eye_resp.hovered(), eye_resp.is_pointer_button_down_on(), false);
                let icon = if layer.visible { Icon::Eye } else { Icon::EyeOff };
                icons::paint(ui.painter(), Rect::from_center_size(eye.center(), vec2(18.0, 18.0)), icon, pal.text_dim, pal.accent);
                let tip = if layer.visible { "Hide layer" } else { "Show layer" };
                if eye_resp.on_hover_text(tip).clicked() {
                    self.doc.toggle_layer_visible(&s, i);
                } else if resp.clicked() {
                    self.doc.set_active_layer(&s, i);
                }
                // Apply menu actions after the menu closes: they can remove this row's layer.
                let mut op: Option<&str> = None;
                resp.context_menu(|ui| {
                    let n = self.doc.layers.len();
                    for (label, key, enabled) in [
                        ("Duplicate layer", "dup", true),
                        ("Merge down", "merge", i > 0),
                        ("Move up", "up", i + 1 < n),
                        ("Move down", "down", i > 0),
                        (if self.doc.layers[i].visible { "Hide layer" } else { "Show layer" }, "vis", true),
                    ] {
                        if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                            op = Some(key);
                        }
                    }
                    ui.separator();
                    if ui.add_enabled(n > 1, egui::Button::new("Delete layer")).clicked() {
                        op = Some("delete");
                    }
                });
                if let Some(op) = op {
                    self.doc.set_active_layer(&s, i);
                    match op {
                        "dup" => self.doc.duplicate_layer(&s),
                        "merge" => self.doc.merge_down(&s),
                        "up" => self.doc.move_layer(&s, 1),
                        "down" => self.doc.move_layer(&s, -1),
                        "vis" => self.doc.toggle_layer_visible(&s, i),
                        _ => self.doc.delete_layer(&s),
                    }
                    ui.close();
                    break;
                }
                ui.add_space(4.0);
            }
            self.background_tile(ui);
        });

        // Drag and drop to reorder: show the drop position and move on release.
        if let Some(from) = self.layer_drag {
            let ptr = ctx.input(|i| i.pointer.interact_pos());
            let target = ptr.and_then(|p| {
                rows.iter().min_by(|a, b| (a.1.center().y - p.y).abs().total_cmp(&(b.1.center().y - p.y).abs())).copied()
            });
            if let Some((to, r)) = target
                && to != from
            {
                let y = if to > from { r.min.y - 2.0 } else { r.max.y + 2.0 };
                ui.painter().line_segment([egui::pos2(r.min.x, y), egui::pos2(r.max.x, y)], Stroke::new(2.0, pal.accent));
            }
            if !ctx.input(|i| i.pointer.any_down()) {
                self.layer_drag = None;
                if let Some((to, _)) = target {
                    self.doc.move_layer_to(&s, from, to);
                }
            }
        }
    }

    /// The Background tile at the bottom of the layer list: its color and visibility.
    fn background_tile(&mut self, ui: &mut Ui) {
        let pal = self.pal;
        ui.add_space(2.0);
        ui.separator();
        let (row, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::click());
        pal.paint_button(ui.painter(), row, resp.hovered(), resp.is_pointer_button_down_on(), false);
        let sw = Rect::from_min_size(row.min + vec2(8.0, 8.0), vec2(THUMB.x, 28.0));
        if self.doc.background_visible {
            ui.painter().rect_filled(sw, 3.0, self.doc.background);
        } else {
            self.paint_checker(ui.painter(), sw);
        }
        ui.painter().rect_stroke(sw, 3.0, Stroke::new(1.0, pal.separator), StrokeKind::Outside);
        ui.painter().text(
            egui::pos2(sw.max.x + 10.0, row.center().y),
            egui::Align2::LEFT_CENTER,
            "Background",
            egui::FontId::proportional(13.0),
            if self.doc.background_visible { pal.text } else { pal.text_dim },
        );
        let eye = Rect::from_center_size(egui::pos2(row.max.x - 20.0, row.center().y), vec2(28.0, 28.0));
        let eye_resp = ui.interact(eye, ui.id().with("bg_eye"), Sense::click());
        pal.paint_button(ui.painter(), eye, eye_resp.hovered(), eye_resp.is_pointer_button_down_on(), false);
        let icon = if self.doc.background_visible { Icon::Eye } else { Icon::EyeOff };
        icons::paint(ui.painter(), Rect::from_center_size(eye.center(), vec2(18.0, 18.0)), icon, pal.text_dim, pal.accent);
        if eye_resp.on_hover_text("Show or hide the background (Ctrl+Shift+H)").clicked() {
            let (c, v) = (self.doc.background, self.doc.background_visible);
            self.doc.set_background(c, !v, true);
        } else if resp.on_hover_text("Background color (Ctrl+Shift+B)").clicked() {
            self.push(Action::BackgroundColor);
        }
    }

    /// Small window for choosing the Background tile color.
    /// Background color dialog. The canvas previews the color live; OK keeps it as one
    /// undo step and Cancel puts the old color back.
    pub fn background_picker(&mut self, ctx: &egui::Context) {
        let Some(mut edit) = self.bg_edit.take() else { return };
        let pal = self.pal;
        let mut hsv_fields = self.edit_hsv;
        let swatches = [("White", Color32::WHITE), ("Black", Color32::BLACK), ("Color 1", self.s.color1), ("Color 2", self.s.color2)];
        let clicked = content_dialog(ctx, pal, "bg_color", 420.0, "Background color", &["OK", "Cancel"], &mut |ui| {
            edit.ui(ui, &pal, 372.0, &mut hsv_fields);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                for (name, c) in swatches {
                    if ui.button(name).clicked() {
                        edit.set_rgb(c);
                    }
                }
            });
        });
        self.edit_hsv = hsv_fields;
        let (enter, esc) = ctx.input(|i| (i.key_pressed(Key::Enter), i.key_pressed(Key::Escape)));
        let visible = self.doc.background_visible;
        match clicked.map(|i| i == 0).or(enter.then_some(true)).or(esc.then_some(false)) {
            Some(ok) => {
                // Back to the original without a history entry, then record the change once.
                self.doc.set_background(edit.original, visible, false);
                if ok && edit.color() != edit.original {
                    self.doc.set_background(edit.color(), true, true);
                }
            }
            None => {
                if edit.color() != self.doc.background {
                    self.doc.set_background(edit.color(), visible, false);
                }
                self.bg_edit = Some(edit);
            }
        }
    }

    /// Light/dark checkerboard used behind transparent pixels.
    pub fn paint_checker(&self, p: &egui::Painter, r: Rect) {
        if let Some(t) = &self.checker_tex {
            // Each texel is one 8px square on screen.
            let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(r.width() / 16.0, r.height() / 16.0));
            p.image(t.id(), r, uv, Color32::WHITE);
        } else {
            p.rect_filled(r, 0.0, Color32::WHITE);
        }
    }

    pub fn ensure_checker(&mut self, ctx: &egui::Context) {
        if self.checker_tex.is_some() {
            return;
        }
        let (a, b) = (Color32::from_gray(255), Color32::from_gray(222));
        let img = egui::ColorImage::new([2, 2], vec![a, b, b, a]);
        let opts = TextureOptions {
            magnification: egui::TextureFilter::Nearest,
            minification: egui::TextureFilter::Nearest,
            wrap_mode: egui::TextureWrapMode::Repeat,
            ..Default::default()
        };
        self.checker_tex = Some(ctx.load_texture("checker", img, opts));
    }
}
