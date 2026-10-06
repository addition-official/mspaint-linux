//! Application state, menus, keyboard shortcuts, file handling and dialogs.

use crate::brush::StrokeCtx;
use crate::document::{Document, Overlay, Settings};
use crate::icons::{self, Icon};
use crate::pixels;
use crate::text::FontManager;
use crate::theme::{self, Palette, ThemePref};
use crate::tools::Tool;
use egui::{
    Align, Button, Color32, Key, KeyboardShortcut, Layout, Modifiers, Pos2, Rect, RichText, Sense, TextureHandle,
    TextureOptions, Vec2, ViewportCommand, vec2,
};
use std::path::PathBuf;

pub const DEFAULT_W: u32 = 1152;
pub const DEFAULT_H: u32 = 648;
pub const ZOOM_MIN: f32 = 0.1;
const WINDOW_RADIUS: u8 = 8;
pub const ZOOM_MAX: f32 = 8.0;
const ZOOM_STEPS: [f32; 14] = [0.1, 0.125, 0.25, 0.33, 0.5, 0.66, 0.75, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0];

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    New,
    Open,
    OpenPath(PathBuf),
    Import,
    Save,
    SaveAs(Option<&'static str>),
    Exit,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    InvertSelection,
    DeleteSelection,
    Crop,
    ResizeDialog,
    RotateRight,
    RotateLeft,
    Rotate180,
    FlipVertical,
    FlipHorizontal,
    InvertColors,
    Properties,
    ZoomIn,
    ZoomOut,
    Zoom100,
    ZoomFit,
    ToggleGrid,
    ToggleRulers,
    ToggleStatus,
    Fullscreen,
    Wallpaper(&'static str),
    EditColors,
    About,
    /// Close without asking (after the unsaved-changes prompt).
    ForceExit,
}

pub enum Dialog {
    Unsaved(Action),
    Resize(ResizeState),
    Properties { w: u32, h: u32 },
    About,
    Message(String),
}

pub struct ResizeState {
    pub by_percent: bool,
    pub w: f32,
    pub h: f32,
    pub keep_aspect: bool,
    pub skew_x: f32,
    pub skew_y: f32,
    pub base_w: f32,
    pub base_h: f32,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
struct Persisted {
    settings: Option<Settings>,
    theme: Option<ThemePref>,
    recent: Vec<PathBuf>,
    hide_status: bool,
    grid: bool,
    rulers: bool,
    system_titlebar: bool,
}

/// Mouse interaction in progress on the canvas.
#[derive(Clone, Debug)]
pub enum Drag {
    Stroke { secondary: bool, last: Pos2 },
    ShapeNew { start: Pos2 },
    CurveBend,
    PolygonPoint,
    Box { hit: Hit, start_ptr: Pos2, start_rect: Rect, start_geom: Option<crate::shapes::Geom> },
    LineEnd { which: u8 },
    Marquee { start: Pos2, cur: Pos2 },
    Lasso { pts: Vec<Pos2> },
    TextNew { start: Pos2, cur: Pos2 },
    CanvasResize { handle: u8, size: Vec2 },
    Pan { last: Pos2 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hit {
    Inside,
    /// 0 TL, 1 T, 2 TR, 3 R, 4 BR, 5 B, 6 BL, 7 L
    Handle(u8),
}

pub struct PaintApp {
    pub doc: Document,
    pub s: Settings,
    pub fm: FontManager,
    pub pal: Palette,
    pub theme_pref: ThemePref,
    applied_dark: Option<bool>,
    pub tex: Option<TextureHandle>,
    pub float_tex: Option<TextureHandle>,
    pub float_tex_key: String,
    pub zoom: f32,
    pub pending_scroll: Option<Vec2>,
    pub pending_zoom_anchor: Option<(Pos2, Pos2)>,
    pub image_rect: Rect,
    pub viewport: Rect,
    pub scroll_offset: Vec2,
    pub drag: Option<Drag>,
    pub prev_tool: Tool,
    pub stroke_ctx: Option<StrokeCtx>,
    pub cursor_img: Option<Pos2>,
    pub show_status: bool,
    pub show_grid: bool,
    pub show_rulers: bool,
    pub fullscreen: bool,
    pub dialog: Option<Dialog>,
    pub show_edit_colors: bool,
    pub edit_color: Color32,
    pub actions: Vec<Action>,
    pub recent: Vec<PathBuf>,
    clipboard: Option<arboard::Clipboard>,
    allow_close: bool,
    pub zoom_fit_pending: bool,
    pub active_slot: u8,
    system_dark: Option<bool>,
    pub system_titlebar: bool,
    decorations_applied: Option<bool>,
    pub on_window_edge: bool,
    /// Window corner radius we currently draw (0 when maximised or using system decorations).
    pub corner: u8,
    system_checked: f64,
}

impl PaintApp {
    pub fn new(cc: &eframe::CreationContext<'_>, open: Option<PathBuf>) -> Self {
        let fm = FontManager::new();
        theme::install_fonts(&cc.egui_ctx, &fm);
        cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
        let p: Persisted = cc
            .storage
            .and_then(|s| eframe::get_value(s, eframe::APP_KEY))
            .unwrap_or_default();
        let mut s = p.settings.unwrap_or_default();
        if s.text.family.is_empty() || !fm.families.contains(&s.text.family) {
            s.text.family = fm.default_family();
        }
        if s.tool == Tool::Picker || s.tool == Tool::Magnifier {
            s.tool = Tool::Pencil;
        }
        let mut app = Self {
            doc: Document::new(DEFAULT_W, DEFAULT_H),
            prev_tool: s.tool,
            s,
            fm,
            pal: Palette::new(false),
            theme_pref: p.theme.unwrap_or(ThemePref::System),
            applied_dark: None,
            tex: None,
            float_tex: None,
            float_tex_key: String::new(),
            zoom: 1.0,
            pending_scroll: None,
            pending_zoom_anchor: None,
            image_rect: Rect::NOTHING,
            viewport: Rect::NOTHING,
            scroll_offset: Vec2::ZERO,
            drag: None,
            stroke_ctx: None,
            cursor_img: None,
            show_status: !p.hide_status,
            show_grid: p.grid,
            show_rulers: p.rulers,
            fullscreen: false,
            dialog: None,
            show_edit_colors: false,
            edit_color: Color32::BLACK,
            actions: Vec::new(),
            recent: p.recent,
            clipboard: arboard::Clipboard::new().ok(),
            allow_close: false,
            zoom_fit_pending: false,
            active_slot: 1,
            system_dark: None,
            system_titlebar: p.system_titlebar,
            decorations_applied: None,
            on_window_edge: false,
            corner: WINDOW_RADIUS,
            system_checked: 0.0,
        };
        if let Some(path) = open {
            app.open_path(path);
        }
        app
    }

    fn title(&self) -> String {
        format!("{}{} - Paint", if self.doc.modified { "*" } else { "" }, self.doc.name())
    }

    pub fn set_tool(&mut self, t: Tool) {
        if self.s.tool != t {
            self.doc.commit_overlay(&self.s);
            self.drag = None;
            if t == Tool::Picker {
                self.prev_tool = self.s.tool;
            }
            self.s.tool = t;
        }
    }

    pub fn push(&mut self, a: Action) {
        self.actions.push(a);
    }

    // -----------------------------------------------------------------------
    // Zoom

    pub fn set_zoom(&mut self, z: f32, anchor_screen: Option<Pos2>) {
        let z = z.clamp(ZOOM_MIN, ZOOM_MAX);
        if (z - self.zoom).abs() < 1e-6 {
            return;
        }
        let anchor = anchor_screen.unwrap_or(self.viewport.center());
        let img_pt = ((anchor - self.image_rect.min) / self.zoom).to_pos2();
        self.zoom = z;
        self.pending_zoom_anchor = Some((img_pt, anchor));
    }

    pub fn zoom_step(&mut self, dir: i32, anchor: Option<Pos2>) {
        let z = self.zoom;
        let next = if dir > 0 {
            ZOOM_STEPS.iter().copied().find(|s| *s > z + 1e-3).unwrap_or(ZOOM_MAX)
        } else {
            ZOOM_STEPS.iter().rev().copied().find(|s| *s < z - 1e-3).unwrap_or(ZOOM_MIN)
        };
        self.set_zoom(next, anchor);
    }

    fn zoom_fit(&mut self) {
        let avail = self.viewport.size() - vec2(48.0, 48.0);
        if avail.x <= 0.0 || avail.y <= 0.0 {
            self.zoom_fit_pending = true;
            return;
        }
        let z = (avail.x / self.doc.width() as f32).min(avail.y / self.doc.height() as f32);
        self.set_zoom(z.clamp(ZOOM_MIN, 1.0), None);
    }

    // -----------------------------------------------------------------------
    // Files

    fn add_recent(&mut self, p: PathBuf) {
        self.recent.retain(|x| x != &p);
        self.recent.insert(0, p);
        self.recent.truncate(10);
    }

    pub fn open_path(&mut self, path: PathBuf) {
        match pixels::load(&path) {
            Ok(img) => {
                self.doc = Document::from_pixmap(img, Some(path.clone()));
                self.add_recent(path);
                self.zoom = 1.0;
                self.drag = None;
                self.zoom_fit_pending = true;
            }
            Err(e) => self.dialog = Some(Dialog::Message(format!("Paint cannot read this file.\n\n{}\n\n{e}", path.display()))),
        }
    }

    fn open_filter(dlg: rfd::FileDialog) -> rfd::FileDialog {
        dlg.add_filter("All picture files", pixels::OPEN_EXTENSIONS)
            .add_filter("PNG", &["png"])
            .add_filter("JPEG", &["jpg", "jpeg", "jpe", "jfif"])
            .add_filter("Bitmap", &["bmp", "dib"])
            .add_filter("GIF", &["gif"])
            .add_filter("TIFF", &["tif", "tiff"])
            .add_filter("WEBP", &["webp"])
            .add_filter("All files", &["*"])
    }

    fn start_dir(&self) -> Option<PathBuf> {
        self.doc
            .path
            .as_ref()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .or_else(dirs_pictures)
    }

    /// Returns true if saved.
    fn save_as(&mut self, ext: Option<&str>) -> bool {
        let base = self
            .doc
            .path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".into());
        let ext = ext.unwrap_or_else(|| {
            self.doc.path.as_ref().and_then(|p| p.extension()).and_then(|e| e.to_str()).unwrap_or("png")
        });
        let ext = ext.to_ascii_lowercase();
        let mut dlg = rfd::FileDialog::new().set_title("Save as").set_file_name(format!("{base}.{ext}"));
        if let Some(d) = self.start_dir() {
            dlg = dlg.set_directory(d);
        }
        // Put the requested format first.
        let mut formats: Vec<_> = pixels::SAVE_FORMATS.to_vec();
        formats.sort_by_key(|(_, exts)| !exts.contains(&ext.as_str()));
        for (name, exts) in formats {
            dlg = dlg.add_filter(name, exts);
        }
        let Some(mut path) = dlg.save_file() else { return false };
        if path.extension().is_none() {
            path.set_extension(ext);
        }
        self.save_to(path)
    }

    fn save_to(&mut self, path: PathBuf) -> bool {
        self.doc.commit_overlay(&self.s);
        match pixels::save(&self.doc.image, &path) {
            Ok(()) => {
                self.doc.mark_saved(path.clone());
                self.add_recent(path);
                true
            }
            Err(e) => {
                self.dialog = Some(Dialog::Message(format!("Could not save {}.\n\n{e}", path.display())));
                false
            }
        }
    }

    fn save(&mut self) -> bool {
        match self.doc.path.clone() {
            Some(p) => self.save_to(p),
            None => self.save_as(Some("png")),
        }
    }

    // -----------------------------------------------------------------------
    // Clipboard

    fn copy(&mut self) {
        let Some(p) = self.doc.effective_float(&self.s) else { return };
        let img = pixels::to_rgba_unmultiplied(&p);
        if let Some(cb) = &mut self.clipboard {
            let _ = cb.set_image(arboard::ImageData {
                width: img.width() as usize,
                height: img.height() as usize,
                bytes: std::borrow::Cow::Owned(img.into_raw()),
            });
        }
    }

    fn paste(&mut self) {
        let Some(cb) = &mut self.clipboard else { return };
        let pix = match cb.get_image() {
            Ok(img) => pixels::from_rgba_unmultiplied(img.width as u32, img.height as u32, img.bytes.into_owned()),
            Err(_) => {
                // Maybe a file path / URI to an image was copied (e.g. from Dolphin).
                let Ok(text) = cb.get_text() else { return };
                let Some(path) = text
                    .lines()
                    .next()
                    .map(|l| l.trim().trim_start_matches("file://").to_string())
                    .map(PathBuf::from)
                    .filter(|p| p.is_file())
                else {
                    return;
                };
                match pixels::load(&path) {
                    Ok(p) => p,
                    Err(_) => return,
                }
            }
        };
        self.paste_pixmap(pix);
    }

    fn paste_pixmap(&mut self, pix: tiny_skia::Pixmap) {
        // Paste at the top-left of the visible part of the canvas.
        let vis = ((self.viewport.min - self.image_rect.min) / self.zoom).max(Vec2::ZERO).to_pos2();
        self.set_tool(Tool::Select);
        self.doc.paste(&self.s, pix, vis);
    }

    // -----------------------------------------------------------------------
    // Actions

    fn needs_save_prompt(a: &Action) -> bool {
        matches!(a, Action::New | Action::Open | Action::OpenPath(_) | Action::Exit)
    }

    pub fn run_action(&mut self, ctx: &egui::Context, a: Action) {
        if self.doc.modified && Self::needs_save_prompt(&a) {
            self.dialog = Some(Dialog::Unsaved(a));
            return;
        }
        let s = self.s.clone();
        match a {
            Action::New => {
                self.doc = Document::new(DEFAULT_W, DEFAULT_H);
                self.zoom = 1.0;
                self.drag = None;
            }
            Action::Open => {
                let mut dlg = Self::open_filter(rfd::FileDialog::new().set_title("Open"));
                if let Some(d) = self.start_dir() {
                    dlg = dlg.set_directory(d);
                }
                if let Some(p) = dlg.pick_file() {
                    self.open_path(p);
                }
            }
            Action::OpenPath(p) => self.open_path(p),
            Action::Import => {
                let mut dlg = Self::open_filter(rfd::FileDialog::new().set_title("Import to canvas"));
                if let Some(d) = self.start_dir() {
                    dlg = dlg.set_directory(d);
                }
                if let Some(p) = dlg.pick_file() {
                    match pixels::load(&p) {
                        Ok(pix) => self.paste_pixmap(pix),
                        Err(e) => self.dialog = Some(Dialog::Message(format!("Paint cannot read this file.\n\n{e}"))),
                    }
                }
            }
            Action::Save => {
                self.save();
            }
            Action::SaveAs(ext) => {
                self.save_as(ext);
            }
            Action::Exit => {
                self.allow_close = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            Action::ForceExit => {
                self.allow_close = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            Action::Undo => {
                self.drag = None;
                self.doc.undo(&s);
            }
            Action::Redo => {
                self.drag = None;
                self.doc.redo(&s);
            }
            Action::Copy => self.copy(),
            Action::Cut => {
                self.copy();
                self.doc.delete_selection(&s);
            }
            Action::Paste => self.paste(),
            Action::SelectAll => {
                self.set_tool(Tool::Select);
                self.doc.select_all(&s);
            }
            Action::InvertSelection => {
                self.set_tool(Tool::Select);
                self.doc.invert_selection(&s);
            }
            Action::DeleteSelection => self.doc.delete_selection(&s),
            Action::Crop => self.doc.crop_to_selection(&s),
            Action::ResizeDialog => {
                let (bw, bh) = match self.doc.floating() {
                    Some(f) => (f.rect.width(), f.rect.height()),
                    None => (self.doc.width() as f32, self.doc.height() as f32),
                };
                self.dialog = Some(Dialog::Resize(ResizeState {
                    by_percent: true,
                    w: 100.0,
                    h: 100.0,
                    keep_aspect: true,
                    skew_x: 0.0,
                    skew_y: 0.0,
                    base_w: bw,
                    base_h: bh,
                }));
            }
            Action::RotateRight => self.doc.transform(&s, pixels::rotate_cw),
            Action::RotateLeft => self.doc.transform(&s, pixels::rotate_ccw),
            Action::Rotate180 => self.doc.transform(&s, pixels::rotate_180),
            Action::FlipVertical => self.doc.transform(&s, pixels::flip_v),
            Action::FlipHorizontal => self.doc.transform(&s, pixels::flip_h),
            Action::InvertColors => self.doc.transform(&s, |p| {
                let mut p = p.clone();
                pixels::invert_colors(&mut p);
                p
            }),
            Action::Properties => {
                self.doc.commit_overlay(&s);
                self.dialog = Some(Dialog::Properties { w: self.doc.width(), h: self.doc.height() });
            }
            Action::ZoomIn => self.zoom_step(1, None),
            Action::ZoomOut => self.zoom_step(-1, None),
            Action::Zoom100 => self.set_zoom(1.0, None),
            Action::ZoomFit => self.zoom_fit(),
            Action::ToggleGrid => self.show_grid = !self.show_grid,
            Action::ToggleRulers => self.show_rulers = !self.show_rulers,
            Action::ToggleStatus => self.show_status = !self.show_status,
            Action::Fullscreen => {
                self.fullscreen = !self.fullscreen;
                ctx.send_viewport_cmd(ViewportCommand::Fullscreen(self.fullscreen));
            }
            Action::Wallpaper(mode) => self.set_wallpaper(mode),
            Action::EditColors => {
                self.edit_color = self.s.color1;
                self.show_edit_colors = true;
            }
            Action::About => self.dialog = Some(Dialog::About),
        }
    }

    /// KDE Plasma: set the image as desktop background.
    fn set_wallpaper(&mut self, mode: &str) {
        self.doc.commit_overlay(&self.s);
        let dir = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
            .unwrap_or_else(std::env::temp_dir)
            .join("mspaint-linux");
        let _ = std::fs::create_dir_all(&dir);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let path = dir.join(format!("wallpaper-{stamp}.png"));
        if let Err(e) = pixels::save(&self.doc.image, &path) {
            self.dialog = Some(Dialog::Message(format!("Could not write the wallpaper image.\n\n{e}")));
            return;
        }
        // Remove older wallpapers we created (Plasma keeps its own reference to the new one).
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p != path && p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("wallpaper-")) {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
        let fill = match mode {
            "tile" => "tile",
            "centre" => "pad",
            _ => "preserveAspectCrop",
        };
        let res = std::process::Command::new("plasma-apply-wallpaperimage")
            .arg("--fill-mode")
            .arg(fill)
            .arg(&path)
            .output();
        match res {
            Ok(o) if o.status.success() => {}
            Ok(o) => {
                // Older Plasma without --fill-mode.
                let retry = std::process::Command::new("plasma-apply-wallpaperimage").arg(&path).status();
                if !retry.is_ok_and(|s| s.success()) {
                    self.dialog = Some(Dialog::Message(format!(
                        "Could not set the desktop background.\n\n{}",
                        String::from_utf8_lossy(&o.stderr)
                    )));
                }
            }
            Err(_) => {
                self.dialog = Some(Dialog::Message(
                    "Setting the desktop background needs KDE Plasma (plasma-apply-wallpaperimage was not found).".into(),
                ))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Keyboard

    fn handle_keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() || self.dialog.is_some() {
            return;
        }
        // Text box typing takes priority.
        if let Overlay::Text(_) = self.doc.overlay {
            self.text_input(ctx);
            if let Overlay::Text(_) = self.doc.overlay {
                // Only allow shortcuts with Ctrl while typing.
                let ctrl = ctx.input(|i| i.modifiers.command);
                if !ctrl {
                    return;
                }
            }
        }
        use Action as A;
        let cmd = Modifiers::COMMAND;
        let cs = Modifiers::COMMAND | Modifiers::SHIFT;
        let shortcuts: &[(Modifiers, Key, Action)] = &[
            (cmd, Key::N, A::New),
            (cmd, Key::O, A::Open),
            (cmd, Key::S, A::Save),
            (cs, Key::S, A::SaveAs(None)),
            (Modifiers::NONE, Key::F12, A::SaveAs(None)),
            (cmd, Key::Z, A::Undo),
            (cmd, Key::Y, A::Redo),
            (cs, Key::Z, A::Redo),
            (cmd, Key::A, A::SelectAll),
            (cmd, Key::E, A::Properties),
            (cmd, Key::W, A::ResizeDialog),
            (cmd, Key::Plus, A::ZoomIn),
            (cmd, Key::Equals, A::ZoomIn),
            (cmd, Key::Minus, A::ZoomOut),
            (cmd, Key::Num0, A::ZoomFit),
            (cmd, Key::Num1, A::Zoom100),
            (cmd, Key::G, A::ToggleGrid),
            (cmd, Key::R, A::ToggleRulers),
            (Modifiers::NONE, Key::F11, A::Fullscreen),
            (cmd, Key::Q, A::Exit),
            (cmd | Modifiers::SHIFT, Key::I, A::InvertColors),
            (cmd | Modifiers::SHIFT, Key::X, A::Crop),
        ];
        for (m, k, a) in shortcuts {
            if ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(*m, *k))) {
                self.push(a.clone());
            }
        }
        let events = ctx.input(|i| i.events.clone());
        for e in events {
            match e {
                egui::Event::Copy => self.push(Action::Copy),
                egui::Event::Cut => self.push(Action::Cut),
                // Ctrl+V with an image on the clipboard produces no Paste event, so we react
                // to the key release instead (and to Paste for text clipboards).
                egui::Event::Key { key: Key::V, pressed: false, modifiers, .. } if modifiers.command => {
                    self.push(Action::Paste)
                }
                egui::Event::Key { key: Key::Delete | Key::Backspace, pressed: true, .. } => self.push(Action::DeleteSelection),
                egui::Event::Key { key: Key::Escape, pressed: true, .. } => {
                    if self.fullscreen {
                        self.push(Action::Fullscreen);
                    } else {
                        self.doc.commit_overlay(&self.s);
                    }
                }
                egui::Event::Key { key: Key::Enter, pressed: true, .. } => self.doc.commit_overlay(&self.s),
                egui::Event::Key { key, pressed: true, modifiers, .. }
                    if matches!(key, Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown)
                        && !modifiers.command =>
                {
                    let d = match key {
                        Key::ArrowLeft => vec2(-1.0, 0.0),
                        Key::ArrowRight => vec2(1.0, 0.0),
                        Key::ArrowUp => vec2(0.0, -1.0),
                        _ => vec2(0.0, 1.0),
                    };
                    if self.doc.floating().is_some() {
                        self.doc.lift(&self.s);
                        if let Some(f) = self.doc.floating_mut() {
                            f.rect = f.rect.translate(d);
                            f.changed = true;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn text_input(&mut self, ctx: &egui::Context) {
        let events = ctx.input(|i| i.events.clone());
        let Overlay::Text(tb) = &mut self.doc.overlay else { return };
        for e in events {
            let n = tb.text.chars().count();
            let byte = |t: &str, c: usize| t.char_indices().nth(c).map(|(b, _)| b).unwrap_or(t.len());
            match e {
                egui::Event::Text(s) | egui::Event::Paste(s) => {
                    let b = byte(&tb.text, tb.caret);
                    tb.text.insert_str(b, &s);
                    tb.caret += s.chars().count();
                }
                egui::Event::Key { key, pressed: true, modifiers, .. } if !modifiers.command => match key {
                    Key::Backspace if tb.caret > 0 => {
                        let b = byte(&tb.text, tb.caret - 1);
                        tb.text.remove(b);
                        tb.caret -= 1;
                    }
                    Key::Delete if tb.caret < n => {
                        let b = byte(&tb.text, tb.caret);
                        tb.text.remove(b);
                    }
                    Key::Enter => {
                        let b = byte(&tb.text, tb.caret);
                        tb.text.insert(b, '\n');
                        tb.caret += 1;
                    }
                    Key::ArrowLeft => tb.caret = tb.caret.saturating_sub(1),
                    Key::ArrowRight => tb.caret = (tb.caret + 1).min(n),
                    Key::Home => {
                        let before: Vec<char> = tb.text.chars().take(tb.caret).collect();
                        tb.caret = before.iter().rposition(|c| *c == '\n').map(|p| p + 1).unwrap_or(0);
                    }
                    Key::End => {
                        let after = tb.text.chars().skip(tb.caret).position(|c| c == '\n');
                        tb.caret = after.map(|p| tb.caret + p).unwrap_or(n);
                    }
                    Key::Escape => {
                        self.doc.commit_overlay(&self.s);
                        return;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        // Grow the box downward to fit the text.
        if let Overlay::Text(tb) = &mut self.doc.overlay
            && let Some(font) = self.fm.font(&self.s.text.family, self.s.text.bold, self.s.text.italic)
        {
            let lay = crate::text::layout(&font, self.s.text.px(), &tb.text, tb.rect.min, tb.rect.width(), self.s.text.align);
            let need = lay.lines.last().map(|l| l.2).unwrap_or(0.0) + lay.line_height + 4.0;
            if tb.rect.height() < need {
                tb.rect.max.y = tb.rect.min.y + need;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Top bar (menus + quick access)

    /// Windows 11 style title bar: app icon and title on the left, caption buttons on
    /// the right. Only shown when we draw our own window decorations.
    fn title_bar(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        let ctx = ui.ctx().clone();
        let rect = ui.max_rect();
        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));

        // The whole bar drags the window; caption buttons are registered later so they win.
        let bar = ui.interact(rect, ui.id().with("titlebar"), Sense::click_and_drag());
        if bar.double_clicked() {
            ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
        } else if bar.drag_started_by(egui::PointerButton::Primary) {
            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
        }

        let p = ui.painter();
        app_icon(p, Rect::from_center_size(egui::pos2(rect.min.x + 24.0, rect.center().y), vec2(18.0, 18.0)));
        p.text(
            egui::pos2(rect.min.x + 46.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            self.title(),
            egui::FontId::proportional(12.5),
            pal.text,
        );

        let bw = 46.0;
        let buttons = [
            (Icon::Close, "Close"),
            (if maximized { Icon::Restore } else { Icon::Maximize }, if maximized { "Restore down" } else { "Maximise" }),
            (Icon::Minimize, "Minimise"),
        ];
        for (i, (icon, tip)) in buttons.into_iter().enumerate() {
            let br = Rect::from_min_size(egui::pos2(rect.max.x - bw * (i + 1) as f32, rect.min.y), vec2(bw, rect.height()));
            let resp = ui.interact(br, ui.id().with(("caption", i)), Sense::click());
            let close = icon == Icon::Close;
            let fg = if close && resp.hovered() { Color32::WHITE } else { pal.text };
            if resp.hovered() {
                let fill = if close { Color32::from_rgb(0xC4, 0x2B, 0x1C) } else { pal.hover };
                ui.painter().rect_filled(br, 0.0, fill);
            }
            icons::paint(ui.painter(), Rect::from_center_size(br.center(), vec2(16.0, 16.0)), icon, fg, pal.accent);
            if resp.on_hover_text(tip).clicked() {
                match icon {
                    Icon::Close => ctx.send_viewport_cmd(ViewportCommand::Close),
                    Icon::Minimize => ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
                    _ => ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized)),
                }
            }
        }
    }

    /// The row under the title bar: File and View menus, quick-access buttons, settings.
    fn menu_row(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        ui.spacing_mut().interact_size.y = 32.0;
        ui.horizontal_centered(|ui| {
            ui.set_min_height(40.0);
            ui.add_space(10.0);
            ui.spacing_mut().button_padding = vec2(12.0, 6.0);
            egui::MenuBar::new().ui(ui, |ui| {
                // Wide, airy menu titles like Win11 (the menu bar style overrides button padding).
                ui.menu_button(RichText::new("  File  ").size(14.0), |ui| self.file_menu(ui));
                ui.add_space(4.0);
                ui.menu_button(RichText::new("  View  ").size(14.0), |ui| self.view_menu(ui));
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(2.0);
                if self.quick_button(ui, Icon::Save, "Save (Ctrl+S)", true) {
                    self.push(Action::Save);
                }
                ui.add_space(2.0);
                ui.separator();
                ui.add_space(2.0);
                if self.quick_button(ui, Icon::Undo, "Undo (Ctrl+Z)", self.doc.can_undo()) {
                    self.push(Action::Undo);
                }
                if self.quick_button(ui, Icon::Redo, "Redo (Ctrl+Y)", self.doc.can_redo()) {
                    self.push(Action::Redo);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(8.0);
                    let resp = egui::containers::menu::MenuButton::from_button(
                        Button::new("").min_size(vec2(36.0, 32.0)).frame(false),
                    )
                    .ui(ui, |ui| {
                        ui.set_min_width(220.0);
                        ui.label(RichText::new("App theme").strong());
                        for (t, n) in [(ThemePref::Light, "Light"), (ThemePref::Dark, "Dark"), (ThemePref::System, "Use system setting")] {
                            if ui.radio(self.theme_pref == t, n).clicked() {
                                self.theme_pref = t;
                            }
                        }
                        ui.separator();
                        ui.checkbox(&mut self.system_titlebar, "Use system title bar")
                            .on_hover_text("Use KDE Plasma's window title bar instead of the Windows 11 style one");
                        ui.separator();
                        if ui.button("About Paint").clicked() {
                            self.push(Action::About);
                        }
                    });
                    let r = Rect::from_center_size(resp.0.rect.center(), vec2(20.0, 20.0));
                    icons::paint(ui.painter(), r, Icon::Gear, pal.text, pal.accent);
                    resp.0.on_hover_text("Settings");
                });
            });
        });
    }

    /// Without system decorations we have to offer resizing from the window edges.
    /// Returns true while the pointer is over a resize edge.
    fn window_edges(&mut self, ctx: &egui::Context) -> bool {
        let (maximized, fullscreen) =
            ctx.input(|i| (i.viewport().maximized.unwrap_or(false), i.viewport().fullscreen.unwrap_or(false)));
        if self.system_titlebar || maximized || fullscreen {
            return false;
        }
        let rect = ctx.viewport_rect();
        // Paint the 1px window border.
        ctx.layer_painter(egui::LayerId::new(egui::Order::Debug, egui::Id::new("window_border")))
            .rect_stroke(rect, self.corner, egui::Stroke::new(1.0, self.pal.border), egui::StrokeKind::Inside);
        let Some(p) = ctx.input(|i| i.pointer.latest_pos()) else { return false };
        let m = 5.0;
        let (l, r, t, b) = (p.x < rect.min.x + m, p.x > rect.max.x - m, p.y < rect.min.y + m, p.y > rect.max.y - m);
        use egui::viewport::ResizeDirection as D;
        let dir = match (l, r, t, b) {
            (true, _, true, _) => D::NorthWest,
            (_, true, true, _) => D::NorthEast,
            (true, _, _, true) => D::SouthWest,
            (_, true, _, true) => D::SouthEast,
            (true, ..) => D::West,
            (_, true, ..) => D::East,
            (_, _, true, _) => D::North,
            (_, _, _, true) => D::South,
            _ => return false,
        };
        ctx.set_cursor_icon(match dir {
            D::North | D::South => egui::CursorIcon::ResizeVertical,
            D::East | D::West => egui::CursorIcon::ResizeHorizontal,
            D::NorthWest | D::SouthEast => egui::CursorIcon::ResizeNwSe,
            D::NorthEast | D::SouthWest => egui::CursorIcon::ResizeNeSw,
        });
        if ctx.input(|i| i.pointer.primary_pressed()) {
            ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
        }
        true
    }

    fn quick_button(&self, ui: &mut egui::Ui, icon: Icon, tip: &str, enabled: bool) -> bool {
        let resp = ui.add_enabled(enabled, Button::new("").min_size(vec2(36.0, 32.0)).frame(false));
        let fg = if enabled { self.pal.text } else { self.pal.text_dim.gamma_multiply(0.6) };
        let r = Rect::from_center_size(resp.rect.center(), vec2(20.0, 20.0));
        icons::paint(ui.painter(), r, icon, fg, self.pal.accent);
        resp.on_hover_text(tip).clicked()
    }

    fn file_menu(&mut self, ui: &mut egui::Ui) {
        ui.set_min_width(240.0);
        let item = |ui: &mut egui::Ui, label: &str, sc: &str| ui.add(Button::new(label).shortcut_text(sc)).clicked();
        if item(ui, "New", "Ctrl+N") {
            self.push(Action::New);
        }
        if item(ui, "Open", "Ctrl+O") {
            self.push(Action::Open);
        }
        let recent = self.recent.clone();
        ui.add_enabled_ui(!recent.is_empty(), |ui| {
            ui.menu_button("Recent", |ui| {
                for p in recent {
                    let label = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    if ui.button(label).on_hover_text(p.display().to_string()).clicked() {
                        self.push(Action::OpenPath(p));
                    }
                }
            });
        });
        if item(ui, "Import to canvas", "") {
            self.push(Action::Import);
        }
        ui.separator();
        if item(ui, "Save", "Ctrl+S") {
            self.push(Action::Save);
        }
        ui.menu_button("Save as", |ui| {
            for (label, ext) in [
                ("PNG picture", "png"),
                ("JPEG picture", "jpg"),
                ("BMP picture", "bmp"),
                ("GIF picture", "gif"),
                ("TIFF picture", "tiff"),
                ("WEBP picture", "webp"),
                ("ICO picture", "ico"),
            ] {
                if ui.button(label).clicked() {
                    self.push(Action::SaveAs(Some(ext)));
                }
            }
        });
        ui.separator();
        ui.menu_button("Set as desktop background", |ui| {
            for (label, mode) in [("Fill", "fill"), ("Tile", "tile"), ("Centre", "centre")] {
                if ui.button(label).clicked() {
                    self.push(Action::Wallpaper(mode));
                }
            }
        });
        if item(ui, "Image properties", "Ctrl+E") {
            self.push(Action::Properties);
        }
        ui.separator();
        if item(ui, "About Paint", "") {
            self.push(Action::About);
        }
        if item(ui, "Exit", "Ctrl+Q") {
            self.push(Action::Exit);
        }
    }

    fn view_menu(&mut self, ui: &mut egui::Ui) {
        ui.set_min_width(220.0);
        ui.menu_button("Zoom", |ui| {
            if ui.add(Button::new("Zoom in").shortcut_text("Ctrl++")).clicked() {
                self.push(Action::ZoomIn);
            }
            if ui.add(Button::new("Zoom out").shortcut_text("Ctrl+-")).clicked() {
                self.push(Action::ZoomOut);
            }
            if ui.add(Button::new("Zoom to 100%").shortcut_text("Ctrl+1")).clicked() {
                self.push(Action::Zoom100);
            }
            if ui.add(Button::new("Fit to window").shortcut_text("Ctrl+0")).clicked() {
                self.push(Action::ZoomFit);
            }
        });
        ui.menu_button("Show", |ui| {
            if ui.add(Button::new("Rulers").selected(self.show_rulers).shortcut_text("Ctrl+R")).clicked() {
                self.push(Action::ToggleRulers);
            }
            if ui.add(Button::new("Gridlines").selected(self.show_grid).shortcut_text("Ctrl+G")).clicked() {
                self.push(Action::ToggleGrid);
            }
            if ui.add(Button::new("Status bar").selected(self.show_status)).clicked() {
                self.push(Action::ToggleStatus);
            }
        });
        if ui.add(Button::new("Full screen").shortcut_text("F11")).clicked() {
            self.push(Action::Fullscreen);
        }
    }

    // -----------------------------------------------------------------------
    // Status bar

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        ui.horizontal_centered(|ui| {
            let item = |ui: &mut egui::Ui, icon: Icon, text: String, width: f32| {
                ui.allocate_ui_with_layout(vec2(width, 24.0), Layout::left_to_right(Align::Center), |ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                    icons::paint(ui.painter(), r, icon, pal.text_dim, pal.accent);
                    ui.label(RichText::new(text).size(12.0).color(pal.text_dim));
                });
            };
            ui.add_space(8.0);
            let pos = self
                .cursor_img
                .filter(|p| p.x >= 0.0 && p.y >= 0.0 && p.x < self.doc.width() as f32 && p.y < self.doc.height() as f32)
                .map(|p| format!("{}, {}px", p.x.floor(), p.y.floor()))
                .unwrap_or_default();
            item(ui, Icon::Cursor, pos, 130.0);
            ui.separator();
            let sel = match (&self.drag, self.doc.floating()) {
                (Some(Drag::Marquee { start, cur }), _) => {
                    format!("{} × {}px", (cur.x - start.x).abs().round(), (cur.y - start.y).abs().round())
                }
                (_, Some(f)) => format!("{} × {}px", f.rect.width().round(), f.rect.height().round()),
                _ => String::new(),
            };
            item(ui, Icon::SelectionSize, sel, 130.0);
            ui.separator();
            item(ui, Icon::CanvasSize, format!("{} × {}px", self.doc.width(), self.doc.height()), 140.0);
            if let Some(size) = self.doc.file_size {
                ui.separator();
                item(ui, Icon::Disk, format!("Size: {}", human_size(size)), 120.0);
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(8.0);
                if self.quick_button(ui, Icon::ZoomIn, "Zoom in", self.zoom < ZOOM_MAX) {
                    self.zoom_step(1, None);
                }
                let mut z = self.zoom.ln();
                let slider = egui::Slider::new(&mut z, ZOOM_MIN.ln()..=ZOOM_MAX.ln()).show_value(false);
                ui.spacing_mut().slider_width = 120.0;
                if ui.add(slider).changed() {
                    let target = z.exp();
                    // Snap near 100%.
                    let target = if (target - 1.0).abs() < 0.05 { 1.0 } else { target };
                    self.set_zoom(target, None);
                }
                if self.quick_button(ui, Icon::ZoomOut, "Zoom out", self.zoom > ZOOM_MIN) {
                    self.zoom_step(-1, None);
                }
                let label = format!("{}%", (self.zoom * 100.0).round());
                ui.menu_button(label, |ui| {
                    for z in [8.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.75, 0.5, 0.25, 0.125] {
                        if ui.button(format!("{}%", z * 100.0)).clicked() {
                            self.set_zoom(z, None);
                        }
                    }
                    ui.separator();
                    if ui.button("Fit to window").clicked() {
                        self.push(Action::ZoomFit);
                    }
                });
            });
        });
    }

    // -----------------------------------------------------------------------
    // Dialogs

    fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(dialog) = self.dialog.take() else { return };
        let mut keep = true;
        let pal = self.pal;
        let modal = |id: &str, ctx: &egui::Context, f: &mut dyn FnMut(&mut egui::Ui)| {
            egui::Modal::new(egui::Id::new(id)).show(ctx, |ui| {
                // Dialog controls get visible borders, like Fluent text boxes and buttons.
                let w = &mut ui.visuals_mut().widgets;
                w.inactive.weak_bg_fill = w.inactive.bg_fill;
                w.inactive.bg_stroke = egui::Stroke::new(1.0, pal.separator);
                w.hovered.bg_stroke = egui::Stroke::new(1.0, pal.text_dim);
                f(ui)
            });
        };
        let mut dialog = dialog;
        match &mut dialog {
            Dialog::Unsaved(action) => {
                let name = self.doc.name();
                let mut choice = None;
                modal("unsaved", ctx, &mut |ui| {
                    ui.set_width(380.0);
                    ui.heading("Paint");
                    ui.add_space(8.0);
                    ui.label(format!("Do you want to save your work to {name}?"));
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        if ui.add(primary_button("Save", pal)).clicked() {
                            choice = Some(0);
                        }
                        if ui.add(Button::new("Don't save").min_size(vec2(100.0, 30.0))).clicked() {
                            choice = Some(1);
                        }
                        if ui.add(Button::new("Cancel").min_size(vec2(100.0, 30.0))).clicked() {
                            choice = Some(2);
                        }
                    });
                });
                match choice {
                    Some(0) => {
                        keep = false;
                        if self.save() {
                            self.doc.modified = false;
                            let a = if *action == Action::Exit { Action::ForceExit } else { action.clone() };
                            self.push(a);
                        }
                    }
                    Some(1) => {
                        keep = false;
                        self.doc.modified = false;
                        let a = if *action == Action::Exit { Action::ForceExit } else { action.clone() };
                        self.push(a);
                    }
                    Some(_) => keep = false,
                    None => {}
                }
            }
            Dialog::Resize(st) => {
                let mut result = None;
                modal("resize", ctx, &mut |ui| {
                    ui.set_width(340.0);
                    ui.heading("Resize and skew");
                    ui.add_space(6.0);
                    ui.label(RichText::new("Resize").strong());
                    ui.horizontal(|ui| {
                        if ui.radio(st.by_percent, "Percentage").clicked() && !st.by_percent {
                            st.by_percent = true;
                            st.w = st.w / st.base_w * 100.0;
                            st.h = st.h / st.base_h * 100.0;
                        }
                        if ui.radio(!st.by_percent, "Pixels").clicked() && st.by_percent {
                            st.by_percent = false;
                            st.w = (st.w * st.base_w / 100.0).round();
                            st.h = (st.h * st.base_h / 100.0).round();
                        }
                    });
                    let aspect = st.base_h / st.base_w;
                    egui::Grid::new("resize_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                        let max = if st.by_percent { 500.0 } else { 20000.0 };
                        ui.label("Horizontal");
                        if ui.add(egui::DragValue::new(&mut st.w).range(1.0..=max).speed(1.0)).changed() && st.keep_aspect {
                            st.h = if st.by_percent { st.w } else { (st.w * aspect).round() };
                        }
                        ui.end_row();
                        ui.label("Vertical");
                        if ui.add(egui::DragValue::new(&mut st.h).range(1.0..=max).speed(1.0)).changed() && st.keep_aspect {
                            st.w = if st.by_percent { st.h } else { (st.h / aspect).round() };
                        }
                        ui.end_row();
                    });
                    ui.checkbox(&mut st.keep_aspect, "Maintain aspect ratio");
                    ui.add_space(8.0);
                    ui.label(RichText::new("Skew (degrees)").strong());
                    egui::Grid::new("skew_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                        ui.label("Horizontal");
                        ui.add(egui::DragValue::new(&mut st.skew_x).range(-89.0..=89.0).speed(0.5));
                        ui.end_row();
                        ui.label("Vertical");
                        ui.add(egui::DragValue::new(&mut st.skew_y).range(-89.0..=89.0).speed(0.5));
                        ui.end_row();
                    });
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.add(primary_button("OK", pal)).clicked() {
                            result = Some(true);
                        }
                        if ui.add(Button::new("Cancel").min_size(vec2(100.0, 30.0))).clicked() {
                            result = Some(false);
                        }
                    });
                });
                if let Some(ok) = result {
                    keep = false;
                    if ok {
                        let (nw, nh) = if st.by_percent {
                            (st.base_w * st.w / 100.0, st.base_h * st.h / 100.0)
                        } else {
                            (st.w, st.h)
                        };
                        let (nw, nh) = (nw.round().max(1.0) as u32, nh.round().max(1.0) as u32);
                        let (sx, sy) = (st.skew_x, st.skew_y);
                        let bg = if self.doc.floating().is_some() { Color32::TRANSPARENT } else { self.s.color2 };
                        let s = self.s.clone();
                        self.doc.transform(&s, move |p| {
                            let mut out = if (nw, nh) != (p.width(), p.height()) { pixels::resize(p, nw, nh) } else { p.clone() };
                            if sx != 0.0 || sy != 0.0 {
                                out = pixels::skew(&out, sx, sy, bg);
                            }
                            out
                        });
                    }
                }
            }
            Dialog::Properties { w, h } => {
                let mut result = None;
                let doc = &self.doc;
                modal("props", ctx, &mut |ui| {
                    ui.set_width(340.0);
                    ui.heading("Image properties");
                    ui.add_space(6.0);
                    egui::Grid::new("props_grid").num_columns(2).spacing([16.0, 6.0]).show(ui, |ui| {
                        ui.label("File");
                        ui.label(doc.path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "Not saved".into()));
                        ui.end_row();
                        ui.label("Size on disk");
                        ui.label(doc.file_size.map(human_size).unwrap_or_else(|| "Not available".into()));
                        ui.end_row();
                        ui.label("Resolution");
                        ui.label("96 DPI");
                        ui.end_row();
                        ui.label("Width (px)");
                        ui.add(egui::DragValue::new(w).range(1..=20000));
                        ui.end_row();
                        ui.label("Height (px)");
                        ui.add(egui::DragValue::new(h).range(1..=20000));
                        ui.end_row();
                    });
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.add(primary_button("OK", pal)).clicked() {
                            result = Some(true);
                        }
                        if ui.add(Button::new("Cancel").min_size(vec2(100.0, 30.0))).clicked() {
                            result = Some(false);
                        }
                    });
                });
                if let Some(ok) = result {
                    keep = false;
                    if ok {
                        let s = self.s.clone();
                        self.doc.resize_canvas(&s, *w, *h);
                    }
                }
            }
            Dialog::About => {
                let mut close = false;
                modal("about", ctx, &mut |ui| {
                    ui.set_width(360.0);
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(48.0, 48.0), Sense::hover());
                        app_icon(ui.painter(), r);
                        ui.vertical(|ui| {
                            ui.heading("Paint");
                            ui.label(RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).color(pal.text_dim));
                        });
                    });
                    ui.add_space(8.0);
                    ui.label("A Windows 11 Paint-style image editor for Linux and KDE Plasma, written in Rust.");
                    ui.add_space(4.0);
                    ui.label(RichText::new("Not affiliated with or endorsed by Microsoft.").small().color(pal.text_dim));
                    ui.add_space(12.0);
                    if ui.add(primary_button("OK", pal)).clicked() {
                        close = true;
                    }
                });
                if close {
                    keep = false;
                }
            }
            Dialog::Message(msg) => {
                let mut close = false;
                let msg = msg.clone();
                modal("message", ctx, &mut |ui| {
                    ui.set_max_width(420.0);
                    ui.heading("Paint");
                    ui.add_space(6.0);
                    ui.label(&msg);
                    ui.add_space(12.0);
                    if ui.add(primary_button("OK", pal)).clicked() {
                        close = true;
                    }
                });
                if close {
                    keep = false;
                }
            }
        }
        if keep && self.dialog.is_none() {
            self.dialog = Some(dialog);
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) && !matches!(self.dialog, Some(Dialog::Unsaved(_))) {
            self.dialog = None;
        }
    }

    fn edit_colors_window(&mut self, ctx: &egui::Context) {
        if !self.show_edit_colors {
            return;
        }
        let mut open = true;
        let mut done = false;
        egui::Window::new("Edit colours")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::widgets::color_picker::color_picker_color32(ui, &mut self.edit_color, egui::color_picker::Alpha::Opaque);
                ui.horizontal(|ui| {
                    ui.label("Hex");
                    let mut hex = self.edit_color.to_hex()[..7].to_string();
                    if ui.add(egui::TextEdit::singleline(&mut hex).desired_width(80.0)).changed()
                        && let Ok(c) = Color32::from_hex(hex.trim())
                    {
                        self.edit_color = c;
                    }
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Set as colour 1").clicked() {
                        self.s.color1 = self.edit_color;
                        done = true;
                    }
                    if ui.button("Set as colour 2").clicked() {
                        self.s.color2 = self.edit_color;
                        done = true;
                    }
                });
            });
        if done {
            let c = self.edit_color;
            self.s.custom_colors.retain(|x| *x != c);
            self.s.custom_colors.insert(0, c);
            self.s.custom_colors.truncate(10);
        }
        self.show_edit_colors = open && !done;
    }

    // -----------------------------------------------------------------------
    // Textures

    fn upload_textures(&mut self, ctx: &egui::Context) {
        let opts = TextureOptions {
            magnification: egui::TextureFilter::Nearest,
            minification: egui::TextureFilter::Linear,
            ..Default::default()
        };
        if self.doc.size_changed || self.tex.is_none() {
            self.tex = Some(ctx.load_texture("canvas", pixels::to_color_image(&self.doc.display), opts));
            self.doc.size_changed = false;
            self.doc.dirty_tex = None;
        } else if let Some(r) = self.doc.dirty_tex.take() {
            let r = r.clamp_to(self.doc.width(), self.doc.height());
            if !r.is_empty()
                && let Some(t) = &mut self.tex
            {
                t.set_partial([r.x0 as usize, r.y0 as usize], pixels::region_color_image(&self.doc.display, r), opts);
            }
        }
        let key = format!("{}{:?}", self.s.transparent_selection, self.s.color2);
        let needs = match self.doc.floating() {
            Some(f) => f.tex_dirty || key != self.float_tex_key || self.float_tex.is_none(),
            None => false,
        };
        if needs {
            if let Some(p) = self.doc.effective_float(&self.s) {
                self.float_tex = Some(ctx.load_texture("selection", pixels::to_color_image(&p), opts));
            }
            self.float_tex_key = key;
            if let Some(f) = self.doc.floating_mut() {
                f.tex_dirty = false;
            }
        }
        if self.doc.floating().is_none() {
            self.float_tex = None;
        }
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        let dark = match self.theme_pref {
            ThemePref::Light => false,
            ThemePref::Dark => true,
            ThemePref::System => {
                // Re-check the Plasma colour scheme every few seconds so we follow changes live.
                let now = ctx.input(|i| i.time);
                if self.system_dark.is_none() || now - self.system_checked > 3.0 {
                    self.system_checked = now;
                    self.system_dark = Some(
                        theme::kde_prefers_dark()
                            .or_else(|| ctx.system_theme().map(|t| t == egui::Theme::Dark))
                            .unwrap_or(false),
                    );
                }
                ctx.request_repaint_after(std::time::Duration::from_secs(3));
                self.system_dark.unwrap_or(false)
            }
        };
        if self.applied_dark != Some(dark) {
            self.applied_dark = Some(dark);
            ctx.set_theme(if dark { egui::Theme::Dark } else { egui::Theme::Light });
            self.pal = Palette::new(dark);
            theme::apply(ctx, &self.pal);
        }
    }
}

impl eframe::App for PaintApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.apply_theme(&ctx);

        // Closing with unsaved changes asks first.
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close && self.doc.modified {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            self.dialog = Some(Dialog::Unsaved(Action::Exit));
        }

        // Files dropped onto the window open (or import when Ctrl is held).
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty()).collect());
        if let Some(p) = dropped.into_iter().next() {
            self.push(Action::OpenPath(p));
        }

        self.handle_keys(&ctx);

        let pal = self.pal;
        // Our own decorations (Windows 11 look) unless the user prefers Plasma's title bar.
        if self.decorations_applied != Some(self.system_titlebar) {
            self.decorations_applied = Some(self.system_titlebar);
            ctx.send_viewport_cmd(ViewportCommand::Decorations(self.system_titlebar));
        }
        // Windows 11 windows have rounded corners. We draw them ourselves on a transparent
        // window whenever we own the decorations and the window isn't maximised/fullscreen.
        let (maximized, fullscreen) =
            ctx.input(|i| (i.viewport().maximized.unwrap_or(false), i.viewport().fullscreen.unwrap_or(false)));
        self.corner = if self.system_titlebar || maximized || fullscreen { 0 } else { WINDOW_RADIUS };
        self.on_window_edge = self.window_edges(&ctx);
        if !self.system_titlebar {
            let r = self.corner;
            egui::Panel::top("titlebar")
                .exact_size(32.0)
                .frame(egui::Frame::new().fill(pal.chrome).corner_radius(egui::CornerRadius { nw: r, ne: r, sw: 0, se: 0 }))
                .show(ui, |ui| self.title_bar(ui));
        }
        egui::Panel::top("menubar")
            .frame(egui::Frame::new().fill(pal.chrome))
            .show(ui, |ui| self.menu_row(ui));
        egui::Panel::top("ribbon")
            .frame(
                egui::Frame::new()
                    .fill(pal.ribbon)
                    .inner_margin(egui::Margin { left: 8, right: 8, top: 2, bottom: 4 })
                    .stroke(egui::Stroke::new(1.0, pal.separator)),
            )
            .show(ui, |ui| self.ribbon(ui));
        if self.show_status {
            egui::Panel::bottom("status")
                .exact_size(30.0)
                .frame(
                    egui::Frame::new()
                        .fill(pal.status)
                        .corner_radius(egui::CornerRadius { nw: 0, ne: 0, sw: self.corner, se: self.corner }),
                )
                .show(ui, |ui| self.status_bar(ui));
        }
        let bottom = if self.show_status { 0 } else { self.corner };
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(pal.workspace).corner_radius(egui::CornerRadius { nw: 0, ne: 0, sw: bottom, se: bottom }))
            .show(ui, |ui| self.canvas(ui));

        // Floating text formatting bar, like Windows 11 Paint.
        if matches!(self.doc.overlay, Overlay::Text(_)) && self.viewport.is_positive() {
            egui::Area::new(egui::Id::new("textbar"))
                .pivot(egui::Align2::CENTER_TOP)
                .fixed_pos(egui::pos2(self.viewport.center().x, self.viewport.min.y + 10.0))
                .order(egui::Order::Foreground)
                .show(&ctx, |ui| {
                    egui::Frame::popup(ui.style())
                        .inner_margin(egui::Margin::symmetric(10, 6))
                        .show(ui, |ui| self.text_bar(ui));
                });
        }

        self.doc.render_overlay(&self.s, &mut self.fm);
        self.upload_textures(&ctx);

        self.dialogs(&ctx);
        self.edit_colors_window(&ctx);

        for a in std::mem::take(&mut self.actions) {
            self.run_action(&ctx, a);
        }
        // Actions run after the canvas was drawn; make sure their effect is shown.
        if self.doc.dirty_tex.is_some() || self.doc.size_changed {
            self.doc.render_overlay(&self.s, &mut self.fm);
            self.upload_textures(&ctx);
            ctx.request_repaint();
        }
        if self.zoom_fit_pending && self.viewport.is_positive() {
            self.zoom_fit_pending = false;
            self.zoom_fit();
            ctx.request_repaint();
        }

        let title = self.title();
        ctx.send_viewport_cmd(ViewportCommand::Title(title));
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // Transparent so the rounded window corners show the desktop behind them.
        if self.corner > 0 {
            [0.0; 4]
        } else {
            self.pal.workspace.to_normalized_gamma_f32()
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let p = Persisted {
            settings: Some(self.s.clone()),
            theme: Some(self.theme_pref),
            recent: self.recent.clone(),
            hide_status: !self.show_status,
            grid: self.show_grid,
            rulers: self.show_rulers,
            system_titlebar: self.system_titlebar,
        };
        eframe::set_value(storage, eframe::APP_KEY, &p);
    }
}

fn primary_button(label: &str, pal: Palette) -> Button<'static> {
    let fg = if pal.dark { Color32::BLACK } else { Color32::WHITE };
    Button::new(RichText::new(label).color(fg)).fill(pal.accent).min_size(vec2(100.0, 30.0))
}

pub fn human_size(b: u64) -> String {
    if b < 1024 {
        format!("{b} bytes")
    } else if b < 1024 * 1024 {
        format!("{:.1}KB", b as f64 / 1024.0)
    } else {
        format!("{:.1}MB", b as f64 / 1024.0 / 1024.0)
    }
}

fn dirs_pictures() -> Option<PathBuf> {
    // Respect XDG user dirs (Pictures), falling back to $HOME.
    let home = PathBuf::from(std::env::var_os("HOME")?);
    let cfg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".config"));
    if let Ok(s) = std::fs::read_to_string(cfg.join("user-dirs.dirs")) {
        for line in s.lines() {
            if let Some(v) = line.strip_prefix("XDG_PICTURES_DIR=") {
                let v = v.trim_matches('"').replace("$HOME", &home.to_string_lossy());
                let p = PathBuf::from(v);
                if p.is_dir() {
                    return Some(p);
                }
            }
        }
    }
    let p = home.join("Pictures");
    Some(if p.is_dir() { p } else { home })
}

/// The app's palette-and-brush icon, drawn in vector form.
pub fn app_icon(p: &egui::Painter, r: Rect) {
    let k = r.width() / 24.0;
    let c = r.center();
    let pal_pts: Vec<Pos2> = (0..40)
        .map(|i| {
            let a = i as f32 / 40.0 * std::f32::consts::TAU;
            let dent = if (a - 0.6).abs() < 0.45 { 0.72 } else { 1.0 };
            c + vec2(a.cos() * 10.5 * dent, a.sin() * 9.0 * dent) * k
        })
        .collect();
    p.add(egui::Shape::convex_polygon(pal_pts.clone(), Color32::from_rgb(0xF5, 0xE6, 0xC8), egui::Stroke::NONE));
    p.add(egui::Shape::closed_line(pal_pts, egui::Stroke::new(1.0 * k, Color32::from_rgb(0xB0, 0x8A, 0x55))));
    for (dx, dy, col) in [
        (-5.5, -3.5, Color32::from_rgb(0xE8, 0x3B, 0x3B)),
        (-0.5, -5.5, Color32::from_rgb(0xF5, 0xB7, 0x00)),
        (4.5, -3.5, Color32::from_rgb(0x2E, 0xA0, 0x43)),
        (-6.0, 2.0, Color32::from_rgb(0x1E, 0x6F, 0xD9)),
    ] {
        p.circle_filled(c + vec2(dx, dy) * k, 2.1 * k, col);
    }
    p.line_segment([c + vec2(9.0, -9.5) * k, c + vec2(1.0, 4.5) * k], egui::Stroke::new(2.2 * k, Color32::from_rgb(0x8B, 0x5A, 0x2B)));
    p.circle_filled(c + vec2(0.5, 5.5) * k, 1.8 * k, Color32::from_rgb(0x1E, 0x6F, 0xD9));
}
