//! Paint for Linux: a Windows 11 Paint style image editor for Linux desktops.

mod app;
mod brush;
mod canvas;
mod clipkeep;
mod color_dialog;
mod cursor;
mod desktop;
mod document;
mod fluent;
mod icons;
mod layers_panel;
mod pixels;
mod ribbon;
mod settings;
mod shapes;
mod text;
mod theme;
mod tiles;
mod tool_sliders;
mod tools;

use std::path::PathBuf;

/// Rasterize the application icon (palette + brush) for the window.
fn window_icon(size: u32) -> egui::IconData {
    use tiny_skia::{FillRule, LineCap, Paint, PathBuilder, Pixmap, Stroke, Transform};
    let mut pm = Pixmap::new(size, size).unwrap();
    let k = size as f32 / 24.0;
    let paint = |r, g, b| {
        let mut p = Paint::default();
        p.set_color_rgba8(r, g, b, 255);
        p.anti_alias = true;
        p
    };
    let t = Transform::from_scale(k, k);
    let mut pb = PathBuilder::new();
    for i in 0..=40 {
        let a = i as f32 / 40.0 * std::f32::consts::TAU;
        let dent = if (a - 0.6).abs() < 0.45 { 0.72 } else { 1.0 };
        let (x, y) = (12.0 + a.cos() * 10.5 * dent, 12.0 + a.sin() * 9.0 * dent);
        if i == 0 { pb.move_to(x, y) } else { pb.line_to(x, y) }
    }
    pb.close();
    let path = pb.finish().unwrap();
    pm.fill_path(&path, &paint(0xF5, 0xE6, 0xC8), FillRule::Winding, t, None);
    pm.stroke_path(&path, &paint(0xB0, 0x8A, 0x55), &Stroke { width: 1.0, ..Default::default() }, t, None);
    for (dx, dy, c) in [
        (-5.5, -3.5, (0xE8, 0x3B, 0x3B)),
        (-0.5, -5.5, (0xF5, 0xB7, 0x00)),
        (4.5, -3.5, (0x2E, 0xA0, 0x43)),
        (-6.0, 2.0, (0x1E, 0x6F, 0xD9)),
    ] {
        let circle = PathBuilder::from_circle(12.0 + dx, 12.0 + dy, 2.1).unwrap();
        pm.fill_path(&circle, &paint(c.0, c.1, c.2), FillRule::Winding, t, None);
    }
    let mut pb = PathBuilder::new();
    pb.move_to(21.0, 2.5);
    pb.line_to(13.0, 16.5);
    let handle = pb.finish().unwrap();
    let s = Stroke { width: 2.2, line_cap: LineCap::Round, ..Default::default() };
    pm.stroke_path(&handle, &paint(0x8B, 0x5A, 0x2B), &s, t, None);
    let tip = PathBuilder::from_circle(12.5, 17.5, 1.8).unwrap();
    pm.fill_path(&tip, &paint(0x1E, 0x6F, 0xD9), FillRule::Winding, t, None);
    let rgba = pixels::to_rgba_unmultiplied(&pm).into_raw();
    egui::IconData { rgba, width: size, height: size }
}

/// The app's id: the name of its .desktop file and icon, and its Wayland app id.
const APP_ID: &str = "io.github.addition_official.Paintbox";

/// Paintbox was called mspaint-linux before 0.4.0: keep the settings and recent files
/// saved under the old name.
fn move_old_settings() {
    // eframe keeps them in a folder named after the app id.
    let Some(new) = eframe::storage_dir(APP_ID) else { return };
    let Some(old) = eframe::storage_dir("mspaint-linux") else { return };
    if !new.join("app.ron").exists() && old.join("app.ron").is_file() && std::fs::create_dir_all(&new).is_ok() {
        let _ = std::fs::copy(old.join("app.ron"), new.join("app.ron"));
    }
}

fn main() -> eframe::Result {
    if clipkeep::run_if_helper() {
        return Ok(());
    }
    let mut open = None;
    for arg in std::env::args_os().skip(1) {
        match arg.to_str() {
            Some("-h" | "--help") => {
                println!("Usage: paintbox [IMAGE]\n\nPaintbox: a simple, familiar paint program for Linux.");
                return Ok(());
            }
            Some("-V" | "--version") => {
                println!("paintbox {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            Some(s) if s.starts_with("file://") => open = desktop::path_from_uri(s),
            _ => open = Some(PathBuf::from(arg)),
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Untitled - Paintbox")
            .with_app_id(APP_ID)
            // Tall enough to show the whole default 1152 x 648 canvas at 100%.
            .with_inner_size([1280.0, 880.0])
            .with_min_inner_size([680.0, 460.0])
            // Paint draws a Windows 11 style title bar; the setting can switch back to the desktop's.
            .with_decorations(false)
            // Needed for the rounded Windows 11 window corners.
            .with_transparent(true)
            .with_icon(window_icon(64)),
        ..Default::default()
    };
    move_old_settings();
    eframe::run_native(
        "paintbox",
        options,
        Box::new(move |cc| Ok(Box::new(app::PaintApp::new(cc, open)))),
    )
}
