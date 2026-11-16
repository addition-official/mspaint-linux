//! Colorful icons for the classic (Windows 7 / Windows 10) styles, in the spirit of
//! that era's ribbon icons: soft gradients, outlines and a bit of color. All drawn in
//! code on a 16x16 design grid (scaled for the 32x32 large buttons); nothing is copied
//! from Windows.

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, pos2, vec2};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CIcon {
    Paste,
    Cut,
    Copy,
    Select,
    Crop,
    Resize,
    Rotate,
    Pencil,
    Fill,
    Text,
    Eraser,
    Picker,
    Magnifier,
    Brushes,
    Outline,
    FillStyle,
    Size,
    EditColors,
    ZoomIn,
    ZoomOut,
    Zoom100,
    FullScreen,
    Thumbnail,
    Save,
    Undo,
    Redo,
    AppMenu,
    Opaque,
    Transparent,
    StatusCursor,
    StatusSelection,
    StatusCanvas,
    StatusDisk,
}

fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// Draws on a 16-unit grid inside `rect`.
struct Pen<'a> {
    p: &'a Painter,
    o: Pos2,
    k: f32,
}

impl Pen<'_> {
    fn pt(&self, x: f32, y: f32) -> Pos2 {
        self.o + vec2(x, y) * self.k
    }
    fn r(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(self.pt(x, y), vec2(w, h) * self.k)
    }
    /// Filled rectangle with an outline, both on the pixel grid.
    #[allow(clippy::too_many_arguments)]
    fn boxed(&self, x: f32, y: f32, w: f32, h: f32, fill: Color32, line: Color32, radius: f32) {
        let r = self.r(x, y, w, h);
        self.p.rect_filled(r, radius * self.k, fill);
        self.p.rect_stroke(r, radius * self.k, Stroke::new(self.k.max(1.0) * 0.8, line), StrokeKind::Inside);
    }
    /// Vertical gradient rectangle (top color to bottom color).
    fn grad(&self, x: f32, y: f32, w: f32, h: f32, top: Color32, bottom: Color32) {
        let r = self.r(x, y, w, h);
        let mut mesh = egui::Mesh::default();
        mesh.colored_vertex(r.left_top(), top);
        mesh.colored_vertex(r.right_top(), top);
        mesh.colored_vertex(r.right_bottom(), bottom);
        mesh.colored_vertex(r.left_bottom(), bottom);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        self.p.add(Shape::mesh(mesh));
    }
    fn line(&self, pts: &[(f32, f32)], w: f32, c: Color32) {
        let pts: Vec<Pos2> = pts.iter().map(|&(x, y)| self.pt(x, y)).collect();
        self.p.add(Shape::line(pts, Stroke::new(w * self.k, c)));
    }
    fn poly(&self, pts: &[(f32, f32)], fill: Color32, line: Color32) {
        let pts: Vec<Pos2> = pts.iter().map(|&(x, y)| self.pt(x, y)).collect();
        self.p.add(Shape::convex_polygon(pts, fill, Stroke::new(0.8 * self.k, line)));
    }
    fn circle(&self, x: f32, y: f32, r: f32, fill: Color32, line: Color32) {
        self.p.circle(self.pt(x, y), r * self.k, fill, Stroke::new(0.8 * self.k, line));
    }
    fn text(&self, x: f32, y: f32, size: f32, s: &str, c: Color32) {
        self.p.text(self.pt(x, y), egui::Align2::CENTER_CENTER, s, egui::FontId::proportional(size * self.k), c);
    }
}

impl CIcon {
    pub const ALL: [CIcon; 33] = [
        CIcon::Paste,
        CIcon::Cut,
        CIcon::Copy,
        CIcon::Select,
        CIcon::Crop,
        CIcon::Resize,
        CIcon::Rotate,
        CIcon::Pencil,
        CIcon::Fill,
        CIcon::Text,
        CIcon::Eraser,
        CIcon::Picker,
        CIcon::Magnifier,
        CIcon::Brushes,
        CIcon::Outline,
        CIcon::FillStyle,
        CIcon::Size,
        CIcon::EditColors,
        CIcon::ZoomIn,
        CIcon::ZoomOut,
        CIcon::Zoom100,
        CIcon::FullScreen,
        CIcon::Thumbnail,
        CIcon::Save,
        CIcon::Undo,
        CIcon::Redo,
        CIcon::AppMenu,
        CIcon::Opaque,
        CIcon::Transparent,
        CIcon::StatusCursor,
        CIcon::StatusSelection,
        CIcon::StatusCanvas,
        CIcon::StatusDisk,
    ];

    /// The file name (without extension) a custom icon for this button uses.
    pub fn file_name(self) -> &'static str {
        match self {
            CIcon::Paste => "paste",
            CIcon::Cut => "cut",
            CIcon::Copy => "copy",
            CIcon::Select => "select",
            CIcon::Crop => "crop",
            CIcon::Resize => "resize",
            CIcon::Rotate => "rotate",
            CIcon::Pencil => "pencil",
            CIcon::Fill => "fill",
            CIcon::Text => "text",
            CIcon::Eraser => "eraser",
            CIcon::Picker => "color-picker",
            CIcon::Magnifier => "magnifier",
            CIcon::Brushes => "brushes",
            CIcon::Outline => "shape-outline",
            CIcon::FillStyle => "shape-fill",
            CIcon::Size => "size",
            CIcon::EditColors => "edit-colors",
            CIcon::ZoomIn => "zoom-in",
            CIcon::ZoomOut => "zoom-out",
            CIcon::Zoom100 => "zoom-100",
            CIcon::FullScreen => "full-screen",
            CIcon::Thumbnail => "thumbnail",
            CIcon::Save => "save",
            CIcon::Undo => "undo",
            CIcon::Redo => "redo",
            CIcon::AppMenu => "app-menu",
            CIcon::Opaque => "text-opaque",
            CIcon::Transparent => "text-transparent",
            CIcon::StatusCursor => "status-position",
            CIcon::StatusSelection => "status-selection",
            CIcon::StatusCanvas => "status-canvas",
            CIcon::StatusDisk => "status-file-size",
        }
    }
}

/// Icons the user put in the custom icons folder (see `custom_dir`), loaded when
/// first drawn. Cleared by `reload_custom`.
mod custom {
    use egui::TextureHandle;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// (file name, size) -> the loaded texture, or None when there is no file.
    pub type Cache = HashMap<(&'static str, u32), Option<TextureHandle>>;
    pub static CACHE: Mutex<Option<Cache>> = Mutex::new(None);
}

/// The folder custom icons are read from: `~/.local/share/paintbox/icons` (inside
/// Flatpak, the app's own data folder).
pub fn custom_dir() -> Option<std::path::PathBuf> {
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share")))?;
    Some(data.join("paintbox").join("icons"))
}

/// Draw the custom icon file `name` into `rect` if the user has one; false if not.
pub fn paint_custom(p: &Painter, rect: Rect, name: &'static str, enabled: bool) -> bool {
    let side = rect.width().min(rect.height());
    let size = if side > 24.0 { 32 } else { 16 };
    let Some(tex) = custom_texture(p.ctx(), name, size) else { return false };
    // Keep the picture's shape inside the square.
    let [w, h] = tex.size();
    let scale = side / w.max(h) as f32;
    let r = Rect::from_center_size(rect.center(), vec2(w as f32, h as f32) * scale);
    let tint = if enabled { Color32::WHITE } else { Color32::from_white_alpha(110) };
    p.image(tex.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), tint);
    true
}

/// Forget the loaded custom icons, so changed files are read again.
pub fn reload_custom() {
    if let Ok(mut c) = custom::CACHE.lock() {
        *c = None;
    }
}

/// How many buttons currently have a custom icon file.
pub fn custom_count() -> usize {
    let Some(dir) = custom_dir() else { return 0 };
    let names = CIcon::ALL.iter().map(|i| i.file_name()).chain(crate::tools::ShapeKind::ALL.iter().map(|k| k.file_name()));
    names.filter(|n| find_file(&dir, n, 16).is_some() || find_file(&dir, n, 32).is_some()).count()
}

/// `name-SIZE.ext` first (made for that size), then `name.ext`.
fn find_file(dir: &std::path::Path, name: &str, size: u32) -> Option<std::path::PathBuf> {
    for stem in [format!("{name}-{size}"), name.to_string()] {
        for ext in ["png", "ico", "bmp"] {
            let p = dir.join(format!("{stem}.{ext}"));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// The custom icon file `name` at `size` points, loading it the first time.
fn custom_texture(ctx: &egui::Context, name: &'static str, size: u32) -> Option<egui::TextureHandle> {
    let mut guard = custom::CACHE.lock().ok()?;
    let cache = guard.get_or_insert_with(Default::default);
    let key = (name, size);
    if let Some(t) = cache.get(&key) {
        return t.clone();
    }
    let tex = custom_dir().and_then(|dir| find_file(&dir, name, size)).and_then(|path| {
        // Icons are small: refuse anything big rather than decode it.
        if std::fs::metadata(&path).ok()?.len() > 4 << 20 {
            return None;
        }
        let img = image::open(&path).ok()?;
        if img.width() > 512 || img.height() > 512 {
            return None;
        }
        let rgba = img.to_rgba8();
        let ci = egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw());
        Some(ctx.load_texture(format!("custom-icon-{name}-{size}"), ci, egui::TextureOptions::LINEAR))
    });
    cache.insert(key, tex.clone());
    tex
}

/// Paint `icon` into `rect` (square). `enabled` false draws it faded. A custom icon
/// from the icons folder is used when there is one.
pub fn paint(p: &Painter, rect: Rect, icon: CIcon, enabled: bool) {
    if paint_custom(p, rect, icon.file_name(), enabled) {
        return;
    }
    let side = rect.width().min(rect.height());
    let pen = Pen { p, o: rect.center() - vec2(side, side) / 2.0, k: side / 16.0 };
    if !enabled {
        // Draw into a faded layer: paint normally, then veil it.
        draw(&pen, icon);
        p.rect_filled(rect, 0.0, Color32::from_white_alpha(150));
        return;
    }
    draw(&pen, icon);
}

fn draw(d: &Pen, icon: CIcon) {
    let outline = rgb(0x4A, 0x55, 0x66);
    let blue = rgb(0x2B, 0x6C, 0xC4);
    let paper = rgb(0xFF, 0xFF, 0xFF);
    let paper_line = rgb(0x8A, 0x94, 0xA3);
    match icon {
        CIcon::Paste => {
            // Brown clipboard with a metal clip and a sheet of paper.
            d.boxed(1.5, 2.5, 11.0, 13.0, rgb(0xC8, 0x8C, 0x48), rgb(0x8A, 0x5A, 0x24), 1.0);
            d.boxed(4.5, 1.0, 5.0, 3.0, rgb(0xC9, 0xCF, 0xD6), rgb(0x6E, 0x78, 0x84), 0.8);
            d.boxed(6.5, 6.0, 8.5, 9.5, paper, paper_line, 0.3);
            for y in [8.5, 10.5, 12.5] {
                d.line(&[(8.0, y), (13.5, y)], 0.7, rgb(0xA8, 0xB4, 0xC4));
            }
        }
        CIcon::Cut => {
            // Scissors: two blades crossing, blue handles.
            d.line(&[(4.0, 1.5), (10.0, 10.0)], 1.4, rgb(0x7C, 0x86, 0x92));
            d.line(&[(12.0, 1.5), (6.0, 10.0)], 1.4, rgb(0x9A, 0xA3, 0xAE));
            d.circle(5.0, 12.5, 2.4, Color32::TRANSPARENT, blue);
            d.circle(11.0, 12.5, 2.4, Color32::TRANSPARENT, blue);
            d.p.circle_stroke(d.pt(5.0, 12.5), 2.0 * d.k, Stroke::new(1.3 * d.k, blue));
            d.p.circle_stroke(d.pt(11.0, 12.5), 2.0 * d.k, Stroke::new(1.3 * d.k, blue));
        }
        CIcon::Copy => {
            for (x, y) in [(1.5, 1.5), (5.5, 5.0)] {
                d.boxed(x, y, 8.5, 10.0, paper, paper_line, 0.3);
                for dy in [3.0, 5.0, 7.0] {
                    d.line(&[(x + 1.8, y + dy), (x + 6.8, y + dy)], 0.7, rgb(0xA8, 0xB4, 0xC4));
                }
            }
        }
        CIcon::Select => {
            // Dashed selection rectangle.
            let r = d.r(1.5, 2.5, 13.0, 11.0);
            d.p.rect_filled(r, 0.0, Color32::from_white_alpha(120));
            let dash = |a: Pos2, b: Pos2| {
                let n = ((b - a).length() / (2.0 * d.k)).max(1.0) as usize;
                for i in (0..n).step_by(2) {
                    let t0 = i as f32 / n as f32;
                    let t1 = ((i + 1) as f32 / n as f32).min(1.0);
                    d.p.line_segment([a + (b - a) * t0, a + (b - a) * t1], Stroke::new(0.9 * d.k, rgb(0x20, 0x20, 0x20)));
                }
            };
            dash(r.left_top(), r.right_top());
            dash(r.right_top(), r.right_bottom());
            dash(r.right_bottom(), r.left_bottom());
            dash(r.left_bottom(), r.left_top());
        }
        CIcon::Crop => {
            d.line(&[(4.5, 1.0), (4.5, 11.5), (15.0, 11.5)], 1.4, blue);
            d.line(&[(1.0, 4.5), (11.5, 4.5), (11.5, 15.0)], 1.4, rgb(0x6B, 0x75, 0x82));
        }
        CIcon::Resize => {
            d.boxed(1.5, 6.5, 8.0, 8.0, rgb(0xE6, 0xF0, 0xFB), blue, 0.3);
            d.p.rect_stroke(d.r(1.5, 1.5, 13.0, 13.0), 0.0, Stroke::new(0.8 * d.k, rgb(0x8A, 0x94, 0xA3)), StrokeKind::Inside);
            d.line(&[(7.5, 8.5), (13.0, 3.0)], 1.1, rgb(0x2E, 0x8B, 0x3A));
            d.line(&[(9.5, 3.0), (13.0, 3.0), (13.0, 6.5)], 1.1, rgb(0x2E, 0x8B, 0x3A));
        }
        CIcon::Rotate => {
            d.boxed(5.5, 5.5, 9.0, 9.0, rgb(0xE6, 0xF0, 0xFB), blue, 0.3);
            let pts: Vec<(f32, f32)> = (0..=10)
                .map(|i| {
                    let a = std::f32::consts::PI * (1.0 + 0.5 * i as f32 / 10.0);
                    (7.5 + 5.5 * a.cos(), 8.0 + 5.5 * a.sin())
                })
                .collect();
            d.line(&pts, 1.2, rgb(0x2E, 0x8B, 0x3A));
            d.poly(&[(7.0, 0.8), (10.0, 2.5), (7.0, 4.2)], rgb(0x2E, 0x8B, 0x3A), rgb(0x2E, 0x8B, 0x3A));
        }
        CIcon::Pencil => {
            // Yellow pencil, diagonal, with a pink eraser and a dark tip.
            d.poly(&[(3.0, 13.0), (1.5, 14.5), (2.2, 11.8)], rgb(0x30, 0x30, 0x30), rgb(0x30, 0x30, 0x30));
            d.poly(&[(2.2, 11.8), (11.5, 2.5), (13.5, 4.5), (4.2, 13.8), (3.0, 13.0)], rgb(0xF5, 0xC8, 0x2A), rgb(0x9A, 0x74, 0x10));
            d.poly(&[(11.5, 2.5), (12.8, 1.2), (14.8, 3.2), (13.5, 4.5)], rgb(0xF2, 0x8F, 0xA8), rgb(0xA0, 0x50, 0x60));
            d.line(&[(1.8, 12.4), (3.6, 14.2)], 0.9, rgb(0xE8, 0xC9, 0x96));
        }
        CIcon::Fill => {
            // Tipped paint bucket pouring blue paint.
            d.poly(&[(3.0, 6.0), (8.5, 0.8), (14.0, 6.3), (8.5, 11.8)], rgb(0xC9, 0xCF, 0xD6), rgb(0x5E, 0x68, 0x74));
            d.line(&[(5.0, 3.5), (10.0, 8.5)], 0.7, rgb(0x8A, 0x94, 0xA3));
            d.poly(&[(3.0, 6.0), (1.5, 9.0), (2.0, 13.5), (3.6, 13.5), (4.2, 8.0)], rgb(0x2B, 0x7B, 0xE0), rgb(0x1A, 0x5B, 0xB0));
        }
        CIcon::Text => {
            d.text(8.0, 8.5, 15.0, "A", rgb(0x1C, 0x2A, 0x48));
        }
        CIcon::Eraser => {
            d.poly(&[(1.5, 10.5), (8.0, 4.0), (12.5, 8.5), (6.0, 15.0)], rgb(0xF5, 0xF5, 0xF5), rgb(0x8A, 0x94, 0xA3));
            d.poly(&[(8.0, 4.0), (10.5, 1.5), (15.0, 6.0), (12.5, 8.5)], rgb(0xEE, 0x7B, 0x95), rgb(0xA0, 0x40, 0x58));
        }
        CIcon::Picker => {
            d.line(&[(2.0, 14.0), (8.0, 8.0)], 1.6, rgb(0x9A, 0xA3, 0xAE));
            d.line(&[(2.0, 14.0), (3.5, 12.5)], 1.6, blue);
            d.poly(&[(7.0, 7.0), (10.5, 3.5), (12.5, 5.5), (9.0, 9.0)], rgb(0x6E, 0x78, 0x84), rgb(0x40, 0x48, 0x52));
            d.circle(12.5, 3.5, 2.3, rgb(0x40, 0x48, 0x52), rgb(0x30, 0x36, 0x3E));
        }
        CIcon::Magnifier | CIcon::ZoomIn | CIcon::ZoomOut | CIcon::Zoom100 => {
            d.line(&[(10.0, 10.0), (14.5, 14.5)], 2.4, rgb(0x5A, 0x40, 0x28));
            d.circle(6.5, 6.5, 5.0, rgb(0xD6, 0xEA, 0xFB), rgb(0x5E, 0x68, 0x74));
            d.p.circle_stroke(d.pt(6.5, 6.5), 5.0 * d.k, Stroke::new(1.1 * d.k, rgb(0x5E, 0x68, 0x74)));
            match icon {
                CIcon::ZoomIn => {
                    d.line(&[(4.0, 6.5), (9.0, 6.5)], 1.3, rgb(0x2E, 0x8B, 0x3A));
                    d.line(&[(6.5, 4.0), (6.5, 9.0)], 1.3, rgb(0x2E, 0x8B, 0x3A));
                }
                CIcon::ZoomOut => d.line(&[(4.0, 6.5), (9.0, 6.5)], 1.3, rgb(0xC0, 0x30, 0x30)),
                CIcon::Zoom100 => d.text(6.5, 6.6, 4.3, "100", rgb(0x1C, 0x2A, 0x48)),
                _ => {}
            }
        }
        CIcon::Brushes => {
            // Brush with a wooden handle, a metal band and an orange-painted tip.
            d.line(&[(14.5, 1.5), (8.0, 8.0)], 2.2, rgb(0xA8, 0x6A, 0x30));
            d.poly(&[(6.5, 7.5), (8.5, 9.5), (7.2, 10.8), (5.2, 8.8)], rgb(0xC9, 0xCF, 0xD6), rgb(0x6E, 0x78, 0x84));
            d.poly(&[(5.2, 8.8), (7.2, 10.8), (4.0, 14.5), (1.0, 15.0), (1.5, 12.0)], rgb(0xF2, 0x7A, 0x1E), rgb(0xA8, 0x48, 0x08));
        }
        CIcon::Outline => {
            d.p.rect_stroke(d.r(1.5, 4.5, 10.0, 9.0), 0.0, Stroke::new(1.0 * d.k, rgb(0x20, 0x20, 0x20)), StrokeKind::Inside);
            d.poly(&[(9.0, 9.0), (14.0, 4.0), (15.5, 5.5), (10.5, 10.5)], rgb(0xF5, 0xC8, 0x2A), rgb(0x9A, 0x74, 0x10));
        }
        CIcon::FillStyle => {
            d.boxed(1.5, 4.5, 10.0, 9.0, rgb(0x6F, 0xA8, 0xE8), rgb(0x2B, 0x5C, 0xA0), 0.0);
            d.poly(&[(9.5, 6.0), (12.5, 3.0), (15.5, 6.0), (12.5, 9.0)], rgb(0xC9, 0xCF, 0xD6), rgb(0x5E, 0x68, 0x74));
        }
        CIcon::Size => {
            d.boxed(1.0, 1.5, 14.0, 13.0, paper, rgb(0xA8, 0xB4, 0xC4), 1.0);
            for (y, w) in [(4.0, 0.6), (6.5, 1.0), (9.3, 1.6), (12.4, 2.3)] {
                d.line(&[(3.0, y), (13.0, y)], w, rgb(0x20, 0x20, 0x20));
            }
        }
        CIcon::EditColors => {
            // Square color spectrum: hue across, white to the top.
            let r = d.r(1.5, 1.5, 13.0, 13.0);
            let mut mesh = egui::Mesh::default();
            let steps = 12;
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                let hue = Color32::from(egui::ecolor::Hsva::new(t, 1.0, 1.0, 1.0));
                let x = r.min.x + t * r.width();
                mesh.colored_vertex(pos2(x, r.min.y), Color32::WHITE);
                mesh.colored_vertex(pos2(x, r.max.y), hue);
            }
            for i in 0..steps as u32 {
                let a = i * 2;
                mesh.add_triangle(a, a + 1, a + 3);
                mesh.add_triangle(a, a + 3, a + 2);
            }
            d.p.add(Shape::mesh(mesh));
            d.p.rect_stroke(r, 0.0, Stroke::new(0.8 * d.k, rgb(0x6E, 0x78, 0x84)), StrokeKind::Inside);
        }
        CIcon::FullScreen => {
            d.boxed(1.0, 2.0, 14.0, 10.0, rgb(0x3A, 0x8E, 0xE6), rgb(0x2A, 0x3A, 0x50), 0.8);
            d.grad(2.0, 3.0, 12.0, 8.0, rgb(0x7E, 0xC2, 0xF7), rgb(0x2B, 0x7B, 0xE0));
            d.boxed(5.5, 12.0, 5.0, 1.5, rgb(0x8A, 0x94, 0xA3), rgb(0x5E, 0x68, 0x74), 0.0);
            d.boxed(3.5, 13.5, 9.0, 1.5, rgb(0xB0, 0xB8, 0xC2), rgb(0x5E, 0x68, 0x74), 0.5);
        }
        CIcon::Thumbnail => {
            d.boxed(1.0, 1.5, 14.0, 13.0, rgb(0xEE, 0xF3, 0xFA), rgb(0x6E, 0x78, 0x84), 0.8);
            d.grad(1.5, 2.0, 13.0, 2.5, rgb(0x9C, 0xC4, 0xEE), rgb(0x5A, 0x92, 0xD8));
            d.boxed(6.0, 7.0, 7.5, 6.0, rgb(0xFF, 0xF2, 0xB0), rgb(0x8A, 0x94, 0xA3), 0.0);
            d.poly(&[(6.5, 12.5), (9.0, 9.5), (13.0, 12.5)], rgb(0x4C, 0xA8, 0x4C), rgb(0x2E, 0x7A, 0x2E));
        }
        CIcon::Save => {
            // Blue floppy disk.
            d.boxed(1.5, 1.5, 13.0, 13.0, rgb(0x3E, 0x7B, 0xD0), rgb(0x1F, 0x4E, 0x96), 1.0);
            d.boxed(4.0, 1.5, 8.0, 5.0, rgb(0xF4, 0xF6, 0xF9), rgb(0x1F, 0x4E, 0x96), 0.0);
            d.boxed(9.5, 2.3, 1.6, 3.2, rgb(0x3E, 0x7B, 0xD0), rgb(0x3E, 0x7B, 0xD0), 0.0);
            d.boxed(3.5, 9.0, 9.0, 5.5, rgb(0xFF, 0xFF, 0xFF), rgb(0x1F, 0x4E, 0x96), 0.0);
        }
        CIcon::Undo | CIcon::Redo => {
            let flip = if icon == CIcon::Redo { -1.0 } else { 1.0 };
            let x = |v: f32| if flip < 0.0 { 16.0 - v } else { v };
            let pts: Vec<(f32, f32)> = (0..=12)
                .map(|i| {
                    let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * i as f32 / 12.0;
                    (x(8.5 + 4.5 * a.cos()), 8.5 + 4.0 * a.sin())
                })
                .collect();
            d.line(&pts, 2.0, blue);
            d.line(&[(x(8.5), 12.5), (x(5.0), 12.5)], 2.0, blue);
            d.poly(&[(x(1.5), 4.5), (x(6.0), 1.0), (x(6.0), 8.0)], blue, rgb(0x1F, 0x4E, 0x96));
        }
        CIcon::AppMenu => {
            // A sheet with lines (the application menu button's glyph).
            d.boxed(3.0, 1.5, 10.0, 13.0, rgb(0xFF, 0xFF, 0xFF), rgb(0xD8, 0xE4, 0xF4), 0.5);
            for y in [5.0, 7.5, 10.0] {
                d.line(&[(5.0, y), (11.0, y)], 0.9, rgb(0x5A, 0x7C, 0xB0));
            }
        }
        CIcon::Opaque => {
            d.boxed(1.5, 1.5, 13.0, 13.0, rgb(0xF2, 0xF2, 0xF2), outline, 0.0);
            d.text(8.0, 8.5, 11.0, "A", rgb(0x1C, 0x2A, 0x48));
        }
        CIcon::Transparent => {
            for i in 0..4 {
                for j in 0..4 {
                    if (i + j) % 2 == 0 {
                        d.p.rect_filled(d.r(1.5 + i as f32 * 3.25, 1.5 + j as f32 * 3.25, 3.25, 3.25), 0.0, rgb(0xD8, 0xDC, 0xE2));
                    }
                }
            }
            d.p.rect_stroke(d.r(1.5, 1.5, 13.0, 13.0), 0.0, Stroke::new(0.8 * d.k, outline), StrokeKind::Inside);
            d.text(8.0, 8.5, 11.0, "A", rgb(0x1C, 0x2A, 0x48));
        }
        CIcon::StatusCursor => {
            // Arrow pointer: white with a dark outline.
            d.poly(&[(3.0, 1.5), (3.0, 13.0), (5.8, 10.4), (7.6, 14.6), (9.4, 13.8), (7.6, 9.7), (11.4, 9.7)], Color32::WHITE, rgb(0x20, 0x20, 0x20));
        }
        CIcon::StatusSelection => {
            let r = d.r(1.5, 3.5, 13.0, 9.0);
            d.p.rect_filled(r, 0.0, rgb(0xE6, 0xF0, 0xFB));
            let n = 13;
            for i in 0..n {
                if i % 2 == 0 {
                    let x0 = 1.5 + i as f32;
                    d.line(&[(x0, 3.5), (x0 + 1.0, 3.5)], 0.9, rgb(0x20, 0x20, 0x20));
                    d.line(&[(x0, 12.5), (x0 + 1.0, 12.5)], 0.9, rgb(0x20, 0x20, 0x20));
                }
            }
            for i in 0..9 {
                if i % 2 == 0 {
                    let y0 = 3.5 + i as f32;
                    d.line(&[(1.5, y0), (1.5, y0 + 1.0)], 0.9, rgb(0x20, 0x20, 0x20));
                    d.line(&[(14.5, y0), (14.5, y0 + 1.0)], 0.9, rgb(0x20, 0x20, 0x20));
                }
            }
        }
        CIcon::StatusCanvas => {
            // A sheet with a resize grip in the corner.
            d.boxed(1.5, 2.5, 11.0, 10.0, Color32::WHITE, rgb(0x6E, 0x78, 0x84), 0.0);
            for (x, y) in [(13.0, 13.0), (11.0, 13.0), (13.0, 11.0)] {
                d.p.rect_filled(d.r(x, y, 1.6, 1.6), 0.0, rgb(0x2B, 0x6C, 0xC4));
            }
        }
        CIcon::StatusDisk => {
            d.boxed(2.0, 2.0, 12.0, 12.0, rgb(0x3E, 0x7B, 0xD0), rgb(0x1F, 0x4E, 0x96), 1.0);
            d.boxed(4.5, 2.0, 7.0, 4.5, rgb(0xF4, 0xF6, 0xF9), rgb(0x1F, 0x4E, 0x96), 0.0);
            d.boxed(4.0, 9.0, 8.0, 5.0, Color32::WHITE, rgb(0x1F, 0x4E, 0x96), 0.0);
        }
    }
}
