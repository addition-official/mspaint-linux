//! The Settings page (the gear button), which replaces the toolbar and canvas like
//! Windows 11 Paint's: app theme, title bar, and About.

use crate::app::{PaintApp, app_icon};
use crate::fluent;
use crate::icons::{self, Icon};
use crate::theme::ThemePref;
use egui::{Color32, RichText, Sense, Stroke, Ui, vec2};

impl PaintApp {
    pub fn settings_page(&mut self, ui: &mut Ui) {
        let pal = self.pal;
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_settings = false;
        }
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            ui.add_space(20.0);
            let width = (ui.available_width() - 80.0).min(1000.0);
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.vertical(|ui| {
                    ui.set_width(width);
                    ui.horizontal(|ui| {
                        let (r, back) = ui.allocate_exact_size(vec2(40.0, 36.0), Sense::click());
                        pal.paint_button(ui.painter(), r, back.hovered(), back.is_pointer_button_down_on(), false);
                        let c = r.center();
                        let s = Stroke::new(1.5, pal.text);
                        ui.painter().line_segment([c + vec2(-7.0, 0.0), c + vec2(7.0, 0.0)], s);
                        ui.painter().line_segment([c + vec2(-7.0, 0.0), c + vec2(-2.0, -5.0)], s);
                        ui.painter().line_segment([c + vec2(-7.0, 0.0), c + vec2(-2.0, 5.0)], s);
                        if back.on_hover_text("Back").clicked() {
                            self.show_settings = false;
                        }
                        ui.add_space(8.0);
                        ui.label(RichText::new("Settings").size(28.0).strong());
                    });
                    ui.add_space(24.0);

                    ui.label(RichText::new("Appearance").size(14.0).strong());
                    ui.add_space(6.0);
                    card(ui, &pal, |ui| {
                        card_header(ui, &pal, Icon::Palette, "App theme", "Select which app theme to display");
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.add_space(44.0);
                            ui.vertical(|ui| {
                                for (t, n) in [(ThemePref::Light, "Light"), (ThemePref::Dark, "Dark"), (ThemePref::System, "Use system setting")] {
                                    fluent::radio_value(ui, &pal, &mut self.theme_pref, t, n);
                                }
                            });
                        });
                    });
                    ui.add_space(4.0);
                    card(ui, &pal, |ui| {
                        ui.horizontal(|ui| {
                            card_header(ui, &pal, Icon::Desktop, "Use system title bar", "Use KDE Plasma's window title bar instead of the Windows 11 style one");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                fluent::toggle(ui, &pal, &mut self.system_titlebar);
                                ui.label(if self.system_titlebar { "On" } else { "Off" });
                            });
                        });
                    });

                    ui.add_space(24.0);
                    ui.label(RichText::new("About").size(14.0).strong());
                    ui.add_space(6.0);
                    card(ui, &pal, |ui| {
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                            app_icon(ui.painter(), r);
                            ui.add_space(8.0);
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Paint").size(14.0));
                                ui.label(RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).size(12.0).color(pal.text_dim));
                            });
                        });
                        ui.add_space(8.0);
                        ui.label("A Windows 11 Paint-style image editor for Linux and KDE Plasma, written in Rust.");
                        ui.label(RichText::new("Not affiliated with or endorsed by Microsoft.").size(12.0).color(pal.text_dim));
                    });
                    ui.add_space(24.0);
                });
            });
        });
    }
}

/// Settings card: a rounded, slightly raised panel spanning the column.
fn card(ui: &mut Ui, pal: &crate::theme::Palette, add: impl FnOnce(&mut Ui)) {
    let fill = if pal.dark { Color32::from_rgb(0x2B, 0x2B, 0x2B) } else { Color32::from_rgb(0xFB, 0xFB, 0xFB) };
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, pal.separator))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
}

/// Icon, title and description at the top of a card.
fn card_header(ui: &mut Ui, pal: &crate::theme::Palette, icon: Icon, title: &str, text: &str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(28.0, 32.0), Sense::hover());
        icons::paint(ui.painter(), egui::Rect::from_center_size(r.center(), vec2(20.0, 20.0)), icon, pal.text, pal.accent);
        ui.add_space(12.0);
        ui.vertical(|ui| {
            ui.label(RichText::new(title).size(14.0));
            ui.label(RichText::new(text).size(12.0).color(pal.text_dim));
        });
    });
}
