//! The color editor used by Edit colors and the background color picker, laid out like
//! the Windows 11 color picker: a hue/saturation spectrum, a value slider, a preview of
//! the new and the current color, and hex plus RGB or HSV fields.

use crate::theme::Palette;
use egui::{Color32, Mesh, Pos2, Rect, RichText, Sense, Stroke, Ui, pos2, vec2};

/// Editing state for one dialog. HSV is kept separately from the color so the hue
/// survives dragging through grays, where RGB has no hue.
#[derive(Clone, Debug)]
pub struct ColorEdit {
    /// Hue in degrees (0..360), saturation and value 0..1, on sRGB values like Windows.
    pub hsv: [f32; 3],
    /// The color when the dialog opened, shown in the preview.
    pub original: Color32,
    hex: String,
}

impl ColorEdit {
    pub fn new(c: Color32) -> Self {
        Self { hsv: rgb_to_hsv(c), original: c, hex: hex(c) }
    }

    pub fn color(&self) -> Color32 {
        hsv_to_rgb(self.hsv)
    }

    pub fn set_rgb(&mut self, c: Color32) {
        let [h, s, v] = rgb_to_hsv(c);
        // Grays have no hue (and black no saturation): keep what the user picked.
        let h = if s == 0.0 || v == 0.0 { self.hsv[0] } else { h };
        let s = if v == 0.0 { self.hsv[1] } else { s };
        self.hsv = [h, s, v];
    }

    /// The editor itself, `width` wide. `hsv_fields` picks HSV over RGB fields.
    pub fn ui(&mut self, ui: &mut Ui, pal: &Palette, width: f32, hsv_fields: &mut bool) {
        let preview_w = 72.0;
        let spec = Rect::from_min_size(ui.cursor().min, vec2(width - preview_w - 12.0, 168.0));
        self.spectrum(ui, spec);
        let preview = Rect::from_min_size(pos2(spec.max.x + 12.0, spec.min.y), vec2(preview_w, spec.height()));
        // New color on top, the original below, as in Windows.
        let p = ui.painter();
        let (top, bottom) = preview.split_top_bottom_at_fraction(0.5);
        let r = 6;
        p.rect_filled(top, egui::CornerRadius { nw: r, ne: r, sw: 0, se: 0 }, self.color());
        p.rect_filled(bottom, egui::CornerRadius { nw: 0, ne: 0, sw: r, se: r }, self.original);
        p.rect_stroke(preview, r, Stroke::new(1.0, pal.separator), egui::StrokeKind::Outside);
        ui.allocate_rect(spec.union(preview), Sense::hover());
        ui.add_space(12.0);
        let slider = Rect::from_min_size(ui.cursor().min, vec2(spec.width(), 20.0));
        self.value_slider(ui, slider);
        ui.allocate_rect(slider, Sense::hover());
        ui.add_space(14.0);

        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("color_model")
                .width(90.0)
                .selected_text(if *hsv_fields { "HSV" } else { "RGB" })
                .show_ui(ui, |ui| {
                    ui.selectable_value(hsv_fields, false, "RGB");
                    ui.selectable_value(hsv_fields, true, "HSV");
                });
            ui.add_space(8.0);
            let resp = ui.add(egui::TextEdit::singleline(&mut self.hex).desired_width(100.0).char_limit(7));
            if resp.changed()
                && let Ok(c) = Color32::from_hex(&format!("#{}", self.hex.trim().trim_start_matches('#')))
            {
                self.set_rgb(c);
            }
            if !resp.has_focus() {
                self.hex = hex(self.color());
            }
        });
        ui.add_space(8.0);

        let field = |ui: &mut Ui, value: &mut f32, max: f32, label: &str| {
            ui.horizontal(|ui| {
                let changed = ui
                    .add_sized([100.0, 26.0], egui::DragValue::new(value).range(0.0..=max).speed(1.0).fixed_decimals(0))
                    .changed();
                ui.label(RichText::new(label).color(pal.text));
                changed
            })
            .inner
        };
        if *hsv_fields {
            let [mut h, mut s, mut v] = [self.hsv[0], self.hsv[1] * 100.0, self.hsv[2] * 100.0];
            let mut changed = field(ui, &mut h, 359.0, "Hue");
            changed |= field(ui, &mut s, 100.0, "Saturation");
            changed |= field(ui, &mut v, 100.0, "Value");
            if changed {
                self.hsv = [h, s / 100.0, v / 100.0];
            }
        } else {
            let [r, g, b, _] = self.color().to_array();
            let [mut r, mut g, mut b] = [r as f32, g as f32, b as f32];
            let mut changed = field(ui, &mut r, 255.0, "Red");
            changed |= field(ui, &mut g, 255.0, "Green");
            changed |= field(ui, &mut b, 255.0, "Blue");
            if changed {
                self.set_rgb(Color32::from_rgb(r.round() as u8, g.round() as u8, b.round() as u8));
            }
        }
    }

    /// Hue across, saturation down (full at the top), at the current value.
    fn spectrum(&mut self, ui: &mut Ui, rect: Rect) {
        let resp = ui.interact(rect, ui.id().with("spectrum"), Sense::click_and_drag());
        if let Some(p) = resp.interact_pointer_pos()
            && (resp.dragged() || resp.clicked() || resp.is_pointer_button_down_on())
        {
            let t = ((p - rect.min) / rect.size()).clamp(vec2(0.0, 0.0), vec2(1.0, 1.0));
            self.hsv[0] = (t.x * 360.0).min(359.9);
            self.hsv[1] = 1.0 - t.y;
        }
        let v = self.hsv[2];
        let mut mesh = Mesh::default();
        let cols = 48;
        for i in 0..=cols {
            let t = i as f32 / cols as f32;
            let x = rect.min.x + t * rect.width();
            mesh.colored_vertex(pos2(x, rect.min.y), hsv_to_rgb([t * 360.0, 1.0, v]));
            mesh.colored_vertex(pos2(x, rect.max.y), hsv_to_rgb([t * 360.0, 0.0, v]));
            if i > 0 {
                let k = 2 * i;
                mesh.add_triangle(k - 2, k - 1, k);
                mesh.add_triangle(k - 1, k, k + 1);
            }
        }
        let p = ui.painter_at(rect.expand(8.0));
        p.add(mesh);
        p.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::from_black_alpha(40)), egui::StrokeKind::Outside);
        let at = pos2(rect.min.x + self.hsv[0] / 360.0 * rect.width(), rect.min.y + (1.0 - self.hsv[1]) * rect.height());
        marker(&p, at, 7.0);
    }

    /// Value from black to the full-brightness color, with a round thumb.
    fn value_slider(&mut self, ui: &mut Ui, rect: Rect) {
        let resp = ui.interact(rect, ui.id().with("value"), Sense::click_and_drag());
        let track = rect.shrink2(vec2(0.0, 4.0));
        if let Some(p) = resp.interact_pointer_pos()
            && (resp.dragged() || resp.clicked() || resp.is_pointer_button_down_on())
        {
            self.hsv[2] = ((p.x - track.min.x) / track.width()).clamp(0.0, 1.0);
        }
        let mut mesh = Mesh::default();
        let full = hsv_to_rgb([self.hsv[0], self.hsv[1], 1.0]);
        let r = track.height() / 2.0;
        let inner = track.shrink2(vec2(r, 0.0));
        for (x, c) in [(inner.min.x, Color32::BLACK), (inner.max.x, full)] {
            mesh.colored_vertex(pos2(x, track.min.y), c);
            mesh.colored_vertex(pos2(x, track.max.y), c);
        }
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(1, 2, 3);
        let p = ui.painter_at(rect.expand(8.0));
        // Round ends in the end colors, then the gradient between them.
        p.circle_filled(pos2(inner.min.x, track.center().y), r, Color32::BLACK);
        p.circle_filled(pos2(inner.max.x, track.center().y), r, full);
        p.add(mesh);
        let at = pos2(inner.min.x + self.hsv[2] * inner.width(), track.center().y);
        marker(&p, at, 8.0);
    }
}

/// Ring marker visible on any color: white with thin dark edges.
fn marker(p: &egui::Painter, at: Pos2, r: f32) {
    p.circle_stroke(at, r, Stroke::new(3.5, Color32::from_black_alpha(90)));
    p.circle_stroke(at, r, Stroke::new(2.0, Color32::WHITE));
}

fn hex(c: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

/// HSV on sRGB values (what Windows shows), not egui's linear-light `Hsva`.
pub fn rgb_to_hsv(c: Color32) -> [f32; 3] {
    let [r, g, b] = [c.r(), c.g(), c.b()].map(|x| x as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let s = if max == 0.0 { 0.0 } else { d / max };
    [h, s, max]
}

pub fn hsv_to_rgb([h, s, v]: [f32; 3]) -> Color32 {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    let to = |x: f32| ((x + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgb(to(r), to(g), to(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsv_round_trips() {
        for c in [Color32::RED, Color32::from_rgb(12, 200, 99), Color32::from_rgb(255, 128, 0), Color32::from_gray(77)] {
            assert_eq!(hsv_to_rgb(rgb_to_hsv(c)), c);
        }
        assert_eq!(rgb_to_hsv(Color32::from_rgb(0, 0, 255)), [240.0, 1.0, 1.0]);
    }

    #[test]
    fn grays_keep_their_hue() {
        let mut e = ColorEdit::new(Color32::from_rgb(0, 0, 255));
        e.set_rgb(Color32::from_gray(128));
        assert_eq!(e.hsv[0], 240.0);
        e.set_rgb(Color32::BLACK);
        assert_eq!((e.hsv[0], e.hsv[1]), (240.0, 0.0));
    }
}
