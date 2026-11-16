//! The Settings page (the gear button), which replaces the toolbar and canvas like
//! Windows 11 Paint's: app theme, title bar, and About.

use crate::app::{PaintApp, app_icon};
use crate::fluent;
use crate::icons::{self, Icon};
use crate::theme::{Style, ThemePref};
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
                        ui.label(RichText::new("Settings").size(if pal.classic != 0 { 20.0 } else { 28.0 }).strong());
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
                        card_header(ui, &pal, Icon::Picture, "Style", "Look like Paint in Windows 11, Windows 10 or Windows 7 (the classic styles are always light)");
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.add_space(44.0);
                            ui.vertical(|ui| {
                                for (st, n) in [(Style::Win11, "Windows 11"), (Style::Win10, "Windows 10"), (Style::Win7, "Windows 7")] {
                                    fluent::radio_value(ui, &pal, &mut self.style, st, n);
                                }
                            });
                        });
                    });
                    ui.add_space(4.0);
                    card(ui, &pal, |ui| {
                        ui.horizontal(|ui| {
                            card_header(ui, &pal, Icon::Desktop, "Use system title bar", "Use your desktop's window title bar instead of Paintbox's own");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                fluent::toggle(ui, &pal, &mut self.system_titlebar);
                                ui.label(if self.system_titlebar { "On" } else { "Off" });
                            });
                        });
                    });

                    ui.add_space(4.0);
                    card(ui, &pal, |ui| self.custom_icons_card(ui));

                    ui.add_space(24.0);
                    ui.label(RichText::new("About").size(14.0).strong());
                    ui.add_space(6.0);
                    card(ui, &pal, |ui| {
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                            app_icon(ui.painter(), r);
                            ui.add_space(8.0);
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Paintbox").size(14.0));
                                ui.label(RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).size(12.0).color(pal.text_dim));
                            });
                        });
                        ui.add_space(8.0);
                        ui.label("A simple, familiar paint program for Linux, inspired by Windows 11 Paint. Written in Rust.");
                        ui.label(RichText::new("Not affiliated with or endorsed by Microsoft.").size(12.0).color(pal.text_dim));
                    });
                    ui.add_space(24.0);
                });
            });
        });
    }
}

impl PaintApp {
    /// Custom icons for the Windows 10 and 7 styles: where they go, and their names.
    fn custom_icons_card(&mut self, ui: &mut Ui) {
        use crate::classic_icons::{self as ci, CIcon};
        let pal = self.pal;
        card_header(
            ui,
            &pal,
            Icon::Picture,
            "Custom icons",
            "Replace the icons of the Windows 10 and Windows 7 styles with your own PNG, ICO or BMP files",
        );
        ui.add_space(8.0);
        let Some(dir) = ci::custom_dir() else { return };
        ui.horizontal(|ui| {
            ui.add_space(44.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(dir.display().to_string()).monospace().size(12.0));
                let n = ci::custom_count();
                ui.label(RichText::new(match n {
                    0 => "No custom icons yet.".to_string(),
                    1 => "1 button uses a custom icon.".to_string(),
                    n => format!("{n} buttons use a custom icon."),
                }).size(12.0).color(pal.text_dim));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Open folder").clicked() {
                        let _ = std::fs::create_dir_all(&dir);
                        if let Err(e) = crate::desktop::open_folder(&dir) {
                            self.dialog = Some(crate::app::Dialog::Message(format!("Could not open the folder.\n\n{e}")));
                        }
                    }
                    if ui.button("Reload icons").clicked() {
                        ci::reload_custom();
                    }
                });
                ui.add_space(6.0);
                egui::CollapsingHeader::new("File names").id_salt("custom_icon_names").show(ui, |ui| {
                    ui.label(
                        RichText::new(
                            "Name each file after its button. Add -16 or -32 to the name (paste-32.png) for an icon made \
                             for the small or big buttons; otherwise one file is used for both.",
                        )
                        .size(12.0)
                        .color(pal.text_dim),
                    );
                    ui.add_space(4.0);
                    egui::Grid::new("custom_icon_grid").num_columns(4).spacing([18.0, 4.0]).show(ui, |ui| {
                        for (i, icon) in CIcon::ALL.iter().enumerate() {
                            let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                            ci::paint(ui.painter(), r, *icon, true);
                            ui.label(RichText::new(format!("{}.png", icon.file_name())).monospace().size(12.0));
                            if i % 2 == 1 {
                                ui.end_row();
                            }
                        }
                        ui.end_row();
                        for (i, k) in crate::tools::BrushKind::ALL.iter().enumerate() {
                            let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                            if !ci::paint_custom(ui.painter(), r, k.file_name(), true) {
                                ci::paint(ui.painter(), r, CIcon::Brushes, true);
                            }
                            ui.label(RichText::new(format!("{}.png", k.file_name())).monospace().size(12.0));
                            if i % 2 == 1 {
                                ui.end_row();
                            }
                        }
                        ui.end_row();
                        let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                        if !ci::paint_custom(ui.painter(), r, "app-icon", true) {
                            app_icon(ui.painter(), r);
                        }
                        ui.label(RichText::new("app-icon.png  (title bar)").monospace().size(12.0));
                        ui.end_row();
                        for (i, k) in crate::tools::ShapeKind::ALL.iter().enumerate() {
                            let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                            if !ci::paint_custom(ui.painter(), r, k.file_name(), true) {
                                icons::paint(ui.painter(), r, Icon::Shape(*k), pal.text, pal.accent);
                            }
                            ui.label(RichText::new(format!("{}.png", k.file_name())).monospace().size(12.0));
                            if i % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });
                });
            });
        });
    }
}

/// Settings card: a rounded, slightly raised panel spanning the column.
fn card(ui: &mut Ui, pal: &crate::theme::Palette, add: impl FnOnce(&mut Ui)) {
    let fill = if pal.dark { Color32::from_rgb(0x2B, 0x2B, 0x2B) } else if pal.classic != 0 { Color32::WHITE } else { Color32::from_rgb(0xFB, 0xFB, 0xFB) };
    // The classic styles get square group boxes.
    let radius = if pal.classic != 0 { 0 } else { 6 };
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, pal.separator))
        .corner_radius(radius)
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
        if pal.classic == 0 {
            icons::paint(ui.painter(), egui::Rect::from_center_size(r.center(), vec2(20.0, 20.0)), icon, pal.text, pal.accent);
        } else if let Some(ci) = crate::classic::menu_icon(icon) {
            crate::classic_icons::paint(ui.painter(), egui::Rect::from_center_size(r.center(), vec2(24.0, 24.0)), ci, true);
        }
        ui.add_space(12.0);
        ui.vertical(|ui| {
            ui.label(RichText::new(title).size(14.0));
            ui.label(RichText::new(text).size(12.0).color(pal.text_dim));
        });
    });
}
