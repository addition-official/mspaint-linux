//! Windows 11 Paint shows vertical sliders at the left of the canvas: size for the
//! pencil, brushes and eraser, opacity for the pencil and brushes, and tolerance for
//! the fill tool.

use crate::app::PaintApp;
use crate::tools::Tool;
use egui::{Color32, RichText, Stroke};

impl PaintApp {
    pub fn tool_sliders(&mut self, ctx: &egui::Context) {
        let tool = self.s.tool;
        let sliders: &[&str] = match tool {
            Tool::Pencil | Tool::Brush => &["size", "opacity"],
            Tool::Eraser => &["size"],
            Tool::Fill => &["tolerance"],
            _ => return,
        };
        // Hidden behind dialogs, which would otherwise be drawn under them.
        if !self.viewport.is_positive() || self.modal_open() {
            return;
        }
        let pal = self.pal;
        let height = 200.0;
        let total = sliders.len() as f32 * (height + 46.0);
        let pos = egui::pos2(self.viewport.min.x + 12.0, self.viewport.center().y - total / 2.0);
        let mut preview = false;
        egui::Area::new(egui::Id::new("tool_sliders"))
            .fixed_pos(pos)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .corner_radius(12)
                    .inner_margin(egui::Margin::symmetric(6, 10))
                    .show(ui, |ui| {
                        ui.spacing_mut().slider_width = height;
                        ui.set_width(44.0);
                        ui.vertical_centered(|ui| {
                            for (i, which) in sliders.iter().enumerate() {
                                if i > 0 {
                                    ui.add_space(6.0);
                                    ui.separator();
                                    ui.add_space(4.0);
                                }
                                let (value, range, suffix, tip): (&mut f32, std::ops::RangeInclusive<f32>, &str, &str) = match *which {
                                    "size" => {
                                        let max = if tool == Tool::Eraser { 100.0 } else { 50.0 };
                                        (self.s.size_mut(), 1.0..=max, "px", "Size")
                                    }
                                    "opacity" => (&mut self.s.opacity, 1.0..=100.0, "%", "Opacity"),
                                    _ => (&mut self.s.tolerance, 0.0..=100.0, "%", "Tolerance"),
                                };
                                ui.label(RichText::new(format!("{}{suffix}", value.round())).size(12.0).color(pal.text));
                                let resp = ui
                                    .add(egui::Slider::new(value, range).vertical().show_value(false).step_by(1.0))
                                    .on_hover_text(tip);
                                if *which == "size" && (resp.dragged() || resp.hovered()) {
                                    preview = true;
                                }
                            }
                        });
                    });
            });

        // Live size preview in the middle of the canvas while adjusting the size.
        if preview {
            let r = self.s.size() * self.zoom / 2.0;
            let c = self.viewport.center();
            let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("size_preview")));
            p.circle_filled(c, r.max(1.0), self.s.color1.gamma_multiply(0.85));
            p.circle_stroke(c, r.max(1.0) + 1.0, Stroke::new(1.0, Color32::from_white_alpha(200)));
        }
    }
}
