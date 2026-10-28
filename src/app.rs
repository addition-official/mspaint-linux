//! Application state, menus, keyboard shortcuts, file handling and dialogs.

use crate::brush::StrokeCtx;
use crate::color_dialog::ColorEdit;
use crate::document::{Document, Overlay, Settings};
use crate::icons::{self, Icon};
use crate::pixels;
use crate::text::FontManager;
use crate::theme::{self, Palette, ThemePref};
use crate::tiles::TiledTexture;
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
/// Zoom presets (the same list as the zoom menu); zoom in/out and Ctrl+wheel step through them.
pub const ZOOM_STEPS: [f32; 13] = [0.1, 0.125, 0.25, 0.5, 0.75, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

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
    SizeUp,
    SizeDown,
    AddLayer,
    ToggleBackground,
    BackgroundColor,
    CopyVisible,
    SetTool(Tool),
    CustomRotate,
    RemoveBackground,
    ToggleThumbnail,
    Print,
}

pub enum Dialog {
    Unsaved(Action),
    Resize(ResizeState),
    /// Size in pixels; `unit`: 0 = pixels, 1 = inches, 2 = centimeters.
    Properties { w: u32, h: u32, unit: u8, bw: bool, last_saved: String },
    About,
    Message(String),
    CustomRotate { angle: f32 },
    Print(PrintState),
}

pub struct PrintState {
    pub printers: Vec<String>,
    pub printer: usize,
    pub copies: u32,
    pub fit: bool,
    pub landscape: bool,
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
    /// The first field has been given keyboard focus.
    pub focused: bool,
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
    layers: bool,
}

/// Mouse interaction in progress on the canvas.
#[derive(Clone, Debug)]
pub enum Drag {
    Stroke { secondary: bool, last: Pos2 },
    ShapeNew { start: Pos2 },
    CurveBend,
    PolygonPoint,
    /// `copy`: Ctrl/Shift+drag of a selection, which leaves a copy once it moves.
    Box { hit: Hit, start_ptr: Pos2, start_rect: Rect, start_geom: Option<crate::shapes::Geom>, copy: bool },
    LineEnd { which: u8 },
    Marquee { start: Pos2, cur: Pos2 },
    Lasso { pts: Vec<Pos2> },
    TextNew { start: Pos2, cur: Pos2 },
    CanvasResize { handle: u8, size: Vec2, grab: Vec2 },
    Pan { last: Pos2 },
    /// Free-rotating the selection around `center`; `start` is the pointer angle at
    /// the start of the drag, `angle` the current rotation (degrees).
    Rotate { center: Pos2, start: f32, angle: f32 },
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
    pub tex: Option<TiledTexture>,
    pub float_tex: Option<TiledTexture>,
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
    pub dialog: Option<Dialog>,
    /// Open Edit colors dialog and the background color dialog.
    pub color_edit: Option<ColorEdit>,
    pub bg_edit: Option<ColorEdit>,
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
    /// Window corner radius we currently draw (0 when maximized or using system decorations).
    pub corner: u8,
    /// Window title last sent to the window manager.
    last_title: String,
    system_checked: f64,
    pub show_layers: bool,
    pub checker_tex: Option<TextureHandle>,
    pub layer_thumbs: Vec<TextureHandle>,
    pub thumbs_rev: u64,
    pub thumbs_time: f64,
    pub layer_drag: Option<usize>,
    /// Unrotated selection pixels while free-rotating.
    pub rotate_base: Option<tiny_skia::Pixmap>,
    pub brush_preview_tex: Vec<TextureHandle>,
    pub brush_preview_dark: Option<bool>,
    pub show_thumbnail: bool,
    pub edit_hsv: bool,
    /// Cursor to show over the canvas this frame (None: leave the pointer alone).
    pub canvas_cursor: Option<crate::cursor::CanvasCursor>,
    /// A plain V key press was seen (so its release is not a Ctrl+V paste).
    v_press_seen: bool,
    /// The Background color picker already recorded an undo step.
    /// Accumulated Ctrl+wheel zoom, applied as preset steps.
    pub zoom_accum: f32,
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
            dialog: None,
            color_edit: None,
            bg_edit: None,
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
            last_title: String::new(),
            show_layers: p.layers,
            checker_tex: None,
            layer_thumbs: Vec::new(),
            thumbs_rev: u64::MAX,
            thumbs_time: 0.0,
            layer_drag: None,
            rotate_base: None,
            brush_preview_tex: Vec::new(),
            brush_preview_dark: None,
            show_thumbnail: false,
            edit_hsv: false,
            canvas_cursor: None,
            v_press_seen: false,
            zoom_accum: 1.0,
            system_checked: 0.0,
        };
        if let Some(path) = open {
            app.open_path(path);
        }
        app
    }

    fn title(&self) -> String {
        format!("{}{} - Paint", if self.doc.has_unsaved_work() { "*" } else { "" }, self.doc.name())
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
                // Layer revisions restart with each document: rebuild the thumbnails.
                self.thumbs_rev = u64::MAX;
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
        match pixels::save(&self.doc.flattened(), &path) {
            Ok(()) if pixels::saves_smaller(&path, self.doc.width(), self.doc.height()) => {
                // The full-size picture is not on disk: keep it counted as unsaved.
                self.dialog = Some(Dialog::Message(format!(
                    "Icons can be at most 256 x 256 pixels, so {} was saved as a smaller copy. Your picture is still full size.",
                    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
                )));
                true
            }
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
            // Formats we can open but not write (e.g. TGA, QOI) go through Save as,
            // instead of silently writing PNG data under the old name.
            Some(p) if pixels::can_save(&p) => self.save_to(p),
            _ => self.save_as(Some("png")),
        }
    }

    // -----------------------------------------------------------------------
    // Clipboard

    fn copy(&mut self) {
        let Some(p) = self.doc.effective_float(&self.s) else { return };
        self.copy_pixmap(&p);
    }

    /// Ctrl+Shift+C: copy what's visible (all visible layers) under the selection,
    /// or the whole canvas.
    fn copy_visible(&mut self) {
        // The display already holds visible layers, background and any pending
        // shape or text; add a floating (moved) selection on top.
        let mut flat = self.doc.display.clone();
        if let (Some(f), Some(p)) = (self.doc.floating(), self.doc.effective_float(&self.s)) {
            pixels::draw_scaled(&mut flat, &p, f.rect.min.x, f.rect.min.y, f.rect.width(), f.rect.height());
        }
        let pix = match self.doc.floating() {
            Some(f) => {
                let r = crate::pixels::IRect::new(
                    f.rect.min.x.round() as i32,
                    f.rect.min.y.round() as i32,
                    f.rect.max.x.round() as i32,
                    f.rect.max.y.round() as i32,
                );
                pixels::copy_rect(&flat, r)
            }
            None => flat,
        };
        self.copy_pixmap(&pix);
    }

    fn copy_pixmap(&mut self, p: &tiny_skia::Pixmap) {
        let img = pixels::to_rgba_unmultiplied(p);
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
            Ok(img) => {
                let (w, h) = (img.width as u32, img.height as u32);
                if let Err(e) = pixels::check_size(w, h) {
                    self.dialog = Some(Dialog::Message(format!("Paint cannot paste this picture.\n\n{e}")));
                    return;
                }
                match pixels::from_rgba_unmultiplied(w, h, img.bytes.into_owned()) {
                    Some(p) => p,
                    None => return,
                }
            }
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
        if self.doc.has_unsaved_work() && Self::needs_save_prompt(&a) {
            self.dialog = Some(Dialog::Unsaved(a));
            return;
        }
        let s = self.s.clone();
        match a {
            Action::New => {
                self.doc = Document::new(DEFAULT_W, DEFAULT_H);
                self.thumbs_rev = u64::MAX;
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
                    focused: false,
                }));
            }
            Action::RotateRight => self.doc.transform(&s, pixels::rotate_cw),
            Action::RotateLeft => self.doc.transform(&s, pixels::rotate_ccw),
            Action::Rotate180 => self.doc.transform(&s, pixels::rotate_180),
            Action::FlipVertical => self.doc.transform(&s, pixels::flip_v),
            Action::FlipHorizontal => self.doc.transform(&s, pixels::flip_h),
            Action::InvertColors => {
                // On the bottom layer the white Background tile is part of the picture, as
                // the white canvas is in Paint: invert it too (a black canvas).
                let bake = self.doc.floating().is_none() && self.doc.active == 0 && self.doc.background_visible;
                let bg = self.doc.background;
                self.doc.apply_active(&s, move |p| {
                    if bake {
                        pixels::under_color_region(p, bg, pixels::IRect::from_size(p.width(), p.height()));
                    }
                    pixels::invert_colors(p);
                });
            }
            Action::Properties => {
                self.doc.commit_overlay(&s);
                // Computed once: it runs `date`, which is too slow for every frame.
                let last_saved = self
                    .doc
                    .path
                    .as_ref()
                    .and_then(|p| std::fs::metadata(p).ok())
                    .and_then(|m| m.modified().ok())
                    .map(format_time)
                    .unwrap_or_else(|| "Not available".into());
                self.dialog = Some(Dialog::Properties { w: self.doc.width(), h: self.doc.height(), unit: 0, bw: false, last_saved });
            }
            Action::ZoomIn => self.zoom_step(1, None),
            Action::ZoomOut => self.zoom_step(-1, None),
            Action::Zoom100 => self.set_zoom(1.0, None),
            Action::ZoomFit => self.zoom_fit(),
            Action::ToggleGrid => self.show_grid = !self.show_grid,
            Action::ToggleRulers => self.show_rulers = !self.show_rulers,
            Action::ToggleStatus => self.show_status = !self.show_status,
            Action::Fullscreen => {
                // Use the window's real state: eframe restores fullscreen across restarts.
                let full = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                ctx.send_viewport_cmd(ViewportCommand::Fullscreen(!full));
            }
            Action::Wallpaper(mode) => self.set_wallpaper(mode),
            Action::EditColors => {
                let c = if self.active_slot == 2 { self.s.color2 } else { self.s.color1 };
                self.color_edit = Some(ColorEdit::new(c));
            }
            Action::About => self.dialog = Some(Dialog::About),
            Action::SizeUp | Action::SizeDown => {
                let d = if a == Action::SizeUp { 1.0 } else { -1.0 };
                let size = self.s.size_mut();
                *size = (*size + d).clamp(1.0, 100.0);
            }
            Action::AddLayer => {
                self.show_layers = true;
                self.doc.add_layer(&s);
            }
            Action::ToggleBackground => {
                let (c, v) = (self.doc.background, self.doc.background_visible);
                self.doc.set_background(c, !v, true);
            }
            Action::BackgroundColor => {
                self.show_layers = true;
                self.bg_edit = Some(ColorEdit::new(self.doc.background));
            }
            Action::CopyVisible => self.copy_visible(),
            Action::SetTool(t) => self.set_tool(t),
            Action::CustomRotate => self.dialog = Some(Dialog::CustomRotate { angle: 0.0 }),
            Action::RemoveBackground => {
                let whole = self.doc.floating().is_none();
                if self.doc.apply_active(&s, pixels::remove_background) && whole {
                    // Show the removed area as transparent (checkerboard), in the same undo step.
                    let c = self.doc.background;
                    self.doc.set_background(c, false, false);
                }
            }
            Action::ToggleThumbnail => self.show_thumbnail = !self.show_thumbnail,
            Action::Print => {
                self.doc.commit_overlay(&s);
                match list_printers() {
                    Some((printers, default)) if !printers.is_empty() => {
                        let landscape = self.doc.width() > self.doc.height();
                        self.dialog = Some(Dialog::Print(PrintState { printers, printer: default, copies: 1, fit: true, landscape }));
                    }
                    Some(_) => self.dialog = Some(Dialog::Message("No printers are set up. Add one in System Settings > Printers.".into())),
                    None => {
                        self.dialog = Some(Dialog::Message(
                            "Printing needs CUPS (the lp and lpstat commands), which Kubuntu installs by default.".into(),
                        ))
                    }
                }
            }
        }
    }

    /// Send the flattened picture to a CUPS printer with `lp`, as PNG on its standard
    /// input (no temporary file other users could tamper with or read).
    fn print(&mut self, st: &PrintState) -> Result<(), String> {
        use std::io::Write;
        let png = pixels::encode(&self.doc.flattened(), "png").map_err(|e| e.to_string())?;
        let mut cmd = std::process::Command::new("lp");
        cmd.arg("-d").arg(&st.printers[st.printer]).arg("-n").arg(st.copies.to_string());
        cmd.arg("-t").arg(self.doc.name());
        if st.fit {
            cmd.arg("-o").arg("fit-to-page");
        }
        cmd.arg("-o").arg(if st.landscape { "landscape" } else { "portrait" });
        cmd.stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
        let mut child = cmd.spawn().map_err(|e| e.to_string())?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&png).map_err(|e| e.to_string())?;
        }
        let out = child.wait_with_output().map_err(|e| e.to_string())?;
        if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stderr).into_owned()) }
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
        if let Err(e) = pixels::save(&self.doc.flattened(), &path) {
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
            "center" => "pad",
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
        if ctx.egui_wants_keyboard_input() || self.modal_open() {
            return;
        }
        // Text box typing takes priority; Ctrl+B/I/U format the text.
        if let Overlay::Text(_) = self.doc.overlay {
            for (key, field) in [(Key::B, 0), (Key::I, 1), (Key::U, 2)] {
                if ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, key))) {
                    let t = &mut self.s.text;
                    match field {
                        0 => t.bold = !t.bold,
                        1 => t.italic = !t.italic,
                        _ => t.underline = !t.underline,
                    }
                }
            }
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
        // Shift combinations first: egui matches Shift loosely, so Ctrl+S would also
        // swallow Ctrl+Shift+S.
        let shortcuts: &[(Modifiers, Key, Action)] = &[
            (cs, Key::S, A::SaveAs(None)),
            (cs, Key::Z, A::Redo),
            (cs, Key::I, A::InvertColors),
            (cs, Key::N, A::AddLayer),
            (cs, Key::H, A::ToggleBackground),
            (cs, Key::B, A::BackgroundColor),
            (cmd, Key::P, A::Print),
            (cmd, Key::N, A::New),
            (cmd, Key::O, A::Open),
            (cmd, Key::S, A::Save),
            (Modifiers::NONE, Key::F12, A::SaveAs(None)),
            (cmd, Key::Z, A::Undo),
            (cmd, Key::Y, A::Redo),
            (cmd, Key::A, A::SelectAll),
            (cmd, Key::E, A::Properties),
            (cmd, Key::W, A::ResizeDialog),
            // As in Paint: Ctrl+Plus/Minus change the brush / line width,
            // Ctrl+PageUp/PageDown (and Ctrl+wheel) zoom.
            (cmd, Key::Plus, A::SizeUp),
            (cmd, Key::Equals, A::SizeUp),
            (cmd, Key::Minus, A::SizeDown),
            (cmd, Key::PageUp, A::ZoomIn),
            (cmd, Key::PageDown, A::ZoomOut),
            (cmd, Key::Num0, A::ZoomFit),
            (cmd, Key::Num1, A::Zoom100),
            (cmd, Key::G, A::ToggleGrid),
            (cmd, Key::R, A::ToggleRulers),
            (Modifiers::NONE, Key::F11, A::Fullscreen),
            (cmd, Key::Q, A::Exit),
        ];
        for (m, k, a) in shortcuts {
            if ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(*m, *k))) {
                self.push(a.clone());
            }
        }
        let (events, shift) = ctx.input(|i| (i.events.clone(), i.modifiers.shift));
        for e in events {
            match e {
                // egui-winit turns Ctrl+(Shift+)C/X into these events and sends no key
                // event, so Ctrl+Shift+C (copy visible layers) and Ctrl+Shift+X (crop)
                // are recognized here.
                egui::Event::Copy => self.push(if shift { Action::CopyVisible } else { Action::Copy }),
                egui::Event::Cut => self.push(if shift { Action::Crop } else { Action::Cut }),
                // egui swallows the key *press* of Ctrl+V (and only emits a Paste event when
                // the clipboard holds text), so an image paste is recognized by a V release
                // without a matching press. Checking the modifiers on release is unreliable:
                // Ctrl is often let go before V.
                egui::Event::Key { key: Key::V, pressed: true, .. } => self.v_press_seen = true,
                egui::Event::Key { key: Key::V, pressed: false, .. } => {
                    if !std::mem::take(&mut self.v_press_seen) {
                        self.push(Action::Paste);
                    }
                }
                egui::Event::Key { key: Key::Delete | Key::Backspace, pressed: true, .. } => self.push(Action::DeleteSelection),
                // Single-letter tool keys.
                egui::Event::Key { key, pressed: true, modifiers, .. } if modifiers.is_none() && tool_for_key(key).is_some() => {
                    self.push(Action::SetTool(tool_for_key(key).unwrap()));
                }
                egui::Event::Key { key: Key::Escape, pressed: true, .. } => {
                    if matches!(self.drag, Some(Drag::ShapeNew { .. } | Drag::TextNew { .. })) {
                        // Escape while still dragging out a shape or text box cancels it.
                        self.doc.cancel_overlay();
                        self.drag = None;
                    } else if ctx.input(|i| i.viewport().fullscreen.unwrap_or(false)) {
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
            (if maximized { Icon::Restore } else { Icon::Maximize }, if maximized { "Restore down" } else { "Maximize" }),
            (Icon::Minimize, "Minimize"),
        ];
        for (i, (icon, tip)) in buttons.into_iter().enumerate() {
            let br = Rect::from_min_size(egui::pos2(rect.max.x - bw * (i + 1) as f32, rect.min.y), vec2(bw, rect.height()));
            let resp = ui.interact(br, ui.id().with(("caption", i)), Sense::click());
            let close = icon == Icon::Close;
            let pressed = resp.is_pointer_button_down_on();
            let fg = if close && resp.hovered() { Color32::WHITE } else { pal.icon_fg(true, pressed) };
            if resp.hovered() {
                let fill = match (close, pressed) {
                    (true, false) => Color32::from_rgb(0xC4, 0x2B, 0x1C),
                    (true, true) => Color32::from_rgb(0xC8, 0x40, 0x31),
                    (false, false) => pal.hover,
                    (false, true) => pal.pressed,
                };
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
                    let gear = self.icon_button(ui, Icon::Gear, true);
                    egui::Popup::menu(&gear).show(|ui| {
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
                    gear.on_hover_text("Settings");
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

    /// Flat icon button in the Win11 style (subtle hover and pressed fills).
    fn quick_button(&self, ui: &mut egui::Ui, icon: Icon, tip: &str, enabled: bool) -> bool {
        let resp = self.icon_button(ui, icon, enabled);
        enabled && resp.on_hover_text(tip).clicked()
    }

    fn icon_button(&self, ui: &mut egui::Ui, icon: Icon, enabled: bool) -> egui::Response {
        let sense = if enabled { Sense::click() } else { Sense::hover() };
        let (rect, resp) = ui.allocate_exact_size(vec2(36.0, 32.0), sense);
        let pressed = enabled && resp.is_pointer_button_down_on();
        self.pal.paint_button(ui.painter(), rect, enabled && resp.hovered(), pressed, false);
        let r = Rect::from_center_size(rect.center(), vec2(20.0, 20.0));
        icons::paint(ui.painter(), r, icon, self.pal.icon_fg(enabled, pressed), self.pal.accent);
        resp
    }

    /// A submenu row plus its items (icon, label, action).
    fn submenu(&mut self, ui: &mut egui::Ui, icon: Icon, label: &str, items: &[(Icon, String, Action)]) {
        let row = self.submenu_row(ui, icon, label);
        egui::containers::menu::SubMenu::new().show(ui, &row, |ui| {
            for (icon, label, a) in items {
                if self.menu_item(ui, *icon, label, "", false, true) {
                    self.push(a.clone());
                }
            }
        });
    }

    fn file_menu(&mut self, ui: &mut egui::Ui) {
        ui.set_min_width(250.0);
        for (icon, label, sc, a) in [(Icon::New, "New", "Ctrl+N", Action::New), (Icon::Open, "Open", "Ctrl+O", Action::Open)] {
            if self.menu_item(ui, icon, label, sc, false, true) {
                self.push(a);
            }
        }
        let recent: Vec<(Icon, String, Action)> = self
            .recent
            .iter()
            .map(|p| (Icon::Picture, p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), Action::OpenPath(p.clone())))
            .collect();
        if recent.is_empty() {
            self.menu_item(ui, Icon::Recent, "Recent", "", false, false);
        } else {
            self.submenu(ui, Icon::Recent, "Recent", &recent);
        }
        if self.menu_item(ui, Icon::Import, "Import to canvas", "", false, true) {
            self.push(Action::Import);
        }
        ui.separator();
        if self.menu_item(ui, Icon::Save, "Save", "Ctrl+S", false, true) {
            self.push(Action::Save);
        }
        let formats: Vec<(Icon, String, Action)> = [
            ("PNG picture", "png"),
            ("JPEG picture", "jpg"),
            ("BMP picture", "bmp"),
            ("GIF picture", "gif"),
            ("TIFF picture", "tiff"),
            ("WEBP picture", "webp"),
            ("ICO picture", "ico"),
        ]
        .into_iter()
        .map(|(label, ext)| (Icon::Picture, label.to_string(), Action::SaveAs(Some(ext))))
        .collect();
        self.submenu(ui, Icon::Save, "Save as", &formats);
        if self.menu_item(ui, Icon::Print, "Print", "Ctrl+P", false, true) {
            self.push(Action::Print);
        }
        ui.separator();
        let modes: Vec<(Icon, String, Action)> =
            [("Fill", "fill"), ("Tile", "tile"), ("Center", "center")].into_iter().map(|(l, m)| (Icon::Desktop, l.to_string(), Action::Wallpaper(m))).collect();
        self.submenu(ui, Icon::Desktop, "Set as desktop background", &modes);
        if self.menu_item(ui, Icon::Info, "Image properties", "Ctrl+E", false, true) {
            self.push(Action::Properties);
        }
        ui.separator();
        if self.menu_item(ui, Icon::Exit, "Exit", "", false, true) {
            self.push(Action::Exit);
        }
    }

    fn view_menu(&mut self, ui: &mut egui::Ui) {
        ui.set_min_width(230.0);
        let zoom = self.submenu_row(ui, Icon::Magnifier, "Zoom");
        egui::containers::menu::SubMenu::new().show(ui, &zoom, |ui| {
            for (icon, label, sc, a) in [
                (Icon::ZoomIn, "Zoom in", "Ctrl+PgUp", Action::ZoomIn),
                (Icon::ZoomOut, "Zoom out", "Ctrl+PgDn", Action::ZoomOut),
                (Icon::Magnifier, "Zoom to 100%", "Ctrl+1", Action::Zoom100),
                (Icon::FullScreen, "Fit to window", "Ctrl+0", Action::ZoomFit),
            ] {
                if self.menu_item(ui, icon, label, sc, false, true) {
                    self.push(a);
                }
            }
        });
        let show = self.submenu_row(ui, Icon::Eye, "Show");
        egui::containers::menu::SubMenu::new().show(ui, &show, |ui| {
            for (icon, label, sc, on, a) in [
                (Icon::Ruler, "Rulers", "Ctrl+R", self.show_rulers, Action::ToggleRulers),
                (Icon::Grid, "Gridlines", "Ctrl+G", self.show_grid, Action::ToggleGrid),
                (Icon::StatusBar, "Status bar", "", self.show_status, Action::ToggleStatus),
                (Icon::Picture, "Thumbnail", "", self.show_thumbnail, Action::ToggleThumbnail),
            ] {
                if self.menu_item(ui, icon, label, sc, on, true) {
                    self.push(a);
                }
            }
        });
        if self.menu_item(ui, Icon::FullScreen, "Full screen", "F11", false, true) {
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
                    // Fixed width so the fields don't shift as the text changes.
                    ui.set_min_width(width);
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
                (Some(Drag::TextNew { start, cur }), _) => {
                    format!("{} × {}px", (cur.x - start.x).abs().round(), (cur.y - start.y).abs().round())
                }
                (_, Some(f)) => format!("{} × {}px", f.rect.width().round(), f.rect.height().round()),
                // Shapes and text boxes show their size while drawn or edited, as in Paint.
                _ => match &self.doc.overlay {
                    Overlay::Shape(sh) => {
                        let b = sh.geom.bounds();
                        format!("{} × {}px", b.width().round(), b.height().round())
                    }
                    Overlay::Text(tb) => format!("{} × {}px", tb.rect.width().round(), tb.rect.height().round()),
                    _ => String::new(),
                },
            };
            item(ui, Icon::SelectionSize, sel, 130.0);
            ui.separator();
            let canvas = match &self.drag {
                Some(Drag::CanvasResize { size, .. }) => *size,
                _ => vec2(self.doc.width() as f32, self.doc.height() as f32),
            };
            item(ui, Icon::CanvasSize, format!("{} × {}px", canvas.x, canvas.y), 140.0);
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
                    for z in ZOOM_STEPS.iter().rev().copied() {
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
        // Enter presses the default button; Esc cancels (handled at the end).
        let enter = ctx.input(|i| i.key_pressed(Key::Enter));
        let modal = |id: &str, ctx: &egui::Context, width: f32, title: &str, buttons: &[&str], f: &mut dyn FnMut(&mut egui::Ui)| {
            content_dialog(ctx, pal, id, width, title, buttons, f)
        };
        let mut dialog = dialog;
        match &mut dialog {
            Dialog::Unsaved(action) => {
                let name = self.doc.name();
                let choice = modal("unsaved", ctx, 420.0, "Paint", &["Save", "Don't save", "Cancel"], &mut |ui| {
                    ui.label(format!("Do you want to save your work to {name}?"));
                });
                let choice = choice.or(enter.then_some(0));
                match choice {
                    Some(0) => {
                        keep = false;
                        if self.save() {
                            let a = if *action == Action::Exit { Action::ForceExit } else { action.clone() };
                            self.push(a);
                        }
                    }
                    Some(1) => {
                        keep = false;
                        self.doc.forget_changes();
                        let a = if *action == Action::Exit { Action::ForceExit } else { action.clone() };
                        self.push(a);
                    }
                    Some(_) => keep = false,
                    None => {}
                }
            }
            Dialog::Resize(st) => {
                let clicked = modal("resize", ctx, 360.0, "Resize and skew", &["OK", "Cancel"], &mut |ui| {
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
                        let first = ui.add_sized([90.0, 26.0], egui::DragValue::new(&mut st.w).range(1.0..=max).speed(1.0));
                        if !st.focused {
                            first.request_focus();
                            st.focused = true;
                        }
                        if first.changed() && st.keep_aspect {
                            st.h = if st.by_percent { st.w } else { (st.w * aspect).round() };
                        }
                        ui.end_row();
                        ui.label("Vertical");
                        if ui.add_sized([90.0, 26.0], egui::DragValue::new(&mut st.h).range(1.0..=max).speed(1.0)).changed() && st.keep_aspect {
                            st.w = if st.by_percent { st.h } else { (st.h / aspect).round() };
                        }
                        ui.end_row();
                    });
                    ui.checkbox(&mut st.keep_aspect, "Maintain aspect ratio");
                    ui.add_space(8.0);
                    ui.label(RichText::new("Skew (degrees)").strong());
                    egui::Grid::new("skew_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                        ui.label("Horizontal");
                        ui.add_sized([90.0, 26.0], egui::DragValue::new(&mut st.skew_x).range(-89.0..=89.0).speed(0.5));
                        ui.end_row();
                        ui.label("Vertical");
                        ui.add_sized([90.0, 26.0], egui::DragValue::new(&mut st.skew_y).range(-89.0..=89.0).speed(0.5));
                        ui.end_row();
                    });
                });
                let result = clicked.map(|i| i == 0).or(enter.then_some(true));
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
                        let bg = Color32::TRANSPARENT;
                        let s = self.s.clone();
                        let unchanged = (nw, nh) == (st.base_w.round() as u32, st.base_h.round() as u32) && sx == 0.0 && sy == 0.0;
                        // Size after the skew, as pixels::skew computes it.
                        let (kx, ky) = (sx.to_radians().tan().abs(), sy.to_radians().tan().abs());
                        let out_w = (nw as f32 + nh as f32 * kx).round() as u32;
                        let out_h = (nh as f32 + nw as f32 * ky).round() as u32;
                        if let Err(e) = pixels::check_size(out_w, out_h) {
                            self.dialog = Some(Dialog::Message(e));
                        } else if !unchanged { self.doc.transform(&s, move |p| {
                            let mut out = if (nw, nh) != (p.width(), p.height()) { pixels::resize(p, nw, nh) } else { p.clone() };
                            if sx != 0.0 || sy != 0.0 {
                                out = pixels::skew(&out, sx, sy, bg);
                            }
                            out
                        }); }
                    }
                }
            }
            Dialog::Properties { w, h, unit, bw, last_saved } => {
                let doc = &self.doc;
                let last_saved = last_saved.clone();
                let clicked = modal("props", ctx, 400.0, "Image properties", &["OK", "Cancel"], &mut |ui| {
                    ui.label(RichText::new("File attributes").strong());
                    egui::Grid::new("props_info").num_columns(2).spacing([16.0, 4.0]).show(ui, |ui| {
                        ui.label("Last saved");
                        ui.label(&last_saved);
                        ui.end_row();
                        ui.label("Size on disk");
                        ui.label(doc.file_size.map(human_size).unwrap_or_else(|| "Not available".into()));
                        ui.end_row();
                        ui.label("Resolution");
                        ui.label("96 DPI");
                        ui.end_row();
                    });
                    ui.add_space(8.0);
                    ui.label(RichText::new("Units").strong());
                    ui.horizontal(|ui| {
                        ui.radio_value(unit, 1, "Inches");
                        ui.radio_value(unit, 2, "Centimeters");
                        ui.radio_value(unit, 0, "Pixels");
                    });
                    ui.add_space(4.0);
                    ui.label(RichText::new("Color").strong());
                    ui.horizontal(|ui| {
                        ui.radio_value(bw, true, "Black and white");
                        ui.radio_value(bw, false, "Color");
                    });
                    ui.add_space(8.0);
                    // Show the size in the chosen unit; store pixels.
                    let per_px = match *unit {
                        1 => 1.0 / 96.0,
                        2 => 2.54 / 96.0,
                        _ => 1.0,
                    };
                    let (suffix, speed, decimals) = match *unit {
                        1 => (" in", 0.01, 2),
                        2 => (" cm", 0.02, 2),
                        _ => (" px", 1.0, 0),
                    };
                    egui::Grid::new("props_size").num_columns(2).spacing([16.0, 6.0]).show(ui, |ui| {
                        for (label, v) in [("Width", &mut *w), ("Height", &mut *h)] {
                            ui.label(label);
                            let mut val = *v as f64 * per_px;
                            if ui
                                .add(egui::DragValue::new(&mut val).speed(speed).fixed_decimals(decimals).suffix(suffix).range(per_px..=20000.0 * per_px))
                                .changed()
                            {
                                *v = (val / per_px).round().clamp(1.0, 20000.0) as u32;
                            }
                            ui.end_row();
                        }
                    });
                    ui.add_space(8.0);
                    if ui.add(Button::new("Default size").min_size(vec2(110.0, 30.0))).clicked() {
                        *w = DEFAULT_W;
                        *h = DEFAULT_H;
                    }
                });
                let result = clicked.map(|i| i == 0).or(enter.then_some(true));
                if let Some(ok) = result {
                    keep = false;
                    if ok {
                        let s = self.s.clone();
                        if let Err(e) = pixels::check_size(*w, *h) {
                            self.dialog = Some(Dialog::Message(e));
                            return;
                        }
                        self.doc.resize_canvas(&s, *w, *h);
                        if *bw {
                            self.doc.transform(&s, |p| {
                                let mut p = p.clone();
                                pixels::black_and_white(&mut p);
                                p
                            });
                        }
                    }
                }
            }
            Dialog::About => {
                let clicked = modal("about", ctx, 400.0, "About Paint", &["OK"], &mut |ui| {
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
                });
                if clicked.is_some() || enter {
                    keep = false;
                }
            }
            Dialog::CustomRotate { angle } => {
                let clicked = modal("custom_rotate", ctx, 340.0, "Custom rotate", &["OK", "Cancel"], &mut |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Angle");
                        ui.add(egui::DragValue::new(angle).range(-360.0..=360.0).speed(0.5).suffix("°"));
                    });
                    ui.label(RichText::new("Positive angles rotate clockwise.").small().color(pal.text_dim));
                });
                let result = clicked.map(|i| i == 0).or(enter.then_some(true));
                if let Some(ok) = result {
                    keep = false;
                    if ok && *angle != 0.0 {
                        let a = *angle;
                        let s = self.s.clone();
                        let (w, h) = match self.doc.floating() {
                            Some(f) => (f.rect.width(), f.rect.height()),
                            None => (self.doc.width() as f32, self.doc.height() as f32),
                        };
                        let (c, sn) = (a.to_radians().cos().abs(), a.to_radians().sin().abs());
                        match pixels::check_size((w * c + h * sn).ceil() as u32, (w * sn + h * c).ceil() as u32) {
                            Ok(()) => self.doc.transform(&s, move |p| pixels::rotate_any(p, a)),
                            Err(e) => self.dialog = Some(Dialog::Message(e)),
                        }
                    }
                }
            }
            Dialog::Print(st) => {
                let clicked = modal("print", ctx, 400.0, "Print", &["Print", "Cancel"], &mut |ui| {
                    egui::Grid::new("print_grid").num_columns(2).spacing([16.0, 8.0]).show(ui, |ui| {
                        ui.label("Printer");
                        egui::ComboBox::from_id_salt("printer").selected_text(st.printers[st.printer].clone()).width(220.0).show_ui(ui, |ui| {
                            for (i, p) in st.printers.iter().enumerate() {
                                ui.selectable_value(&mut st.printer, i, p);
                            }
                        });
                        ui.end_row();
                        ui.label("Copies");
                        ui.add(egui::DragValue::new(&mut st.copies).range(1..=99));
                        ui.end_row();
                        ui.label("Orientation");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut st.landscape, false, "Portrait");
                            ui.radio_value(&mut st.landscape, true, "Landscape");
                        });
                        ui.end_row();
                        ui.label("Scaling");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut st.fit, true, "Fit to page");
                            ui.radio_value(&mut st.fit, false, "Actual size");
                        });
                        ui.end_row();
                    });
                });
                let result = clicked.map(|i| i == 0).or(enter.then_some(true));
                if let Some(ok) = result {
                    keep = false;
                    if ok {
                        let msg = self.print(st);
                        if let Err(e) = msg {
                            self.dialog = Some(Dialog::Message(format!("Could not print.\n\n{e}")));
                        }
                    }
                }
            }
            Dialog::Message(msg) => {
                let msg = msg.clone();
                let clicked = modal("message", ctx, 420.0, "Paint", &["OK"], &mut |ui| {
                    ui.label(&msg);
                });
                if clicked.is_some() || enter {
                    keep = false;
                }
            }
        }
        if keep && self.dialog.is_none() {
            self.dialog = Some(dialog);
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.dialog = None;
        }
    }

    /// View > Thumbnail: a small overview of the whole picture; click to scroll there.
    fn thumbnail_window(&mut self, ctx: &egui::Context) {
        if !self.show_thumbnail {
            return;
        }
        let mut open = true;
        let (w, h) = (self.doc.width() as f32, self.doc.height() as f32);
        let scale = (240.0 / w).min(180.0 / h);
        let size = vec2(w * scale, h * scale);
        let mut jump = None;
        egui::Window::new("Thumbnail")
            .collapsible(false)
            .resizable(false)
            .fixed_size(size)
            .open(&mut open)
            .default_pos(egui::pos2(self.viewport.max.x - size.x - 40.0, self.viewport.min.y + 20.0))
            .show(ctx, |ui| {
                let (r, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
                self.paint_checker(ui.painter(), r);
                if let Some(t) = &self.tex {
                    t.paint(ui.painter(), r);
                }
                // The part of the picture currently visible.
                let vis_min = ((self.viewport.min - self.image_rect.min) / self.zoom).max(Vec2::ZERO);
                let vis_max = ((self.viewport.max - self.image_rect.min) / self.zoom).min(vec2(w, h));
                let vr = Rect::from_min_max(r.min + vis_min * scale, r.min + vis_max * scale);
                ui.painter().rect_stroke(vr, 0.0, egui::Stroke::new(1.5, self.pal.accent), egui::StrokeKind::Inside);
                if (resp.clicked() || resp.dragged())
                    && let Some(p) = resp.interact_pointer_pos()
                {
                    jump = Some(((p - r.min) / scale).to_pos2());
                }
            });
        if let Some(img_pt) = jump {
            self.pending_zoom_anchor = Some((img_pt, self.viewport.center()));
        }
        self.show_thumbnail = open;
    }

    /// Whether a modal dialog is up, so shortcuts and canvas overlays stay out of the way.
    pub fn modal_open(&self) -> bool {
        self.dialog.is_some() || self.color_edit.is_some() || self.bg_edit.is_some()
    }

    /// Edit colors: OK sets the selected color slot (Color 1 or 2) and adds the color
    /// to the custom colors row.
    fn edit_colors_window(&mut self, ctx: &egui::Context) {
        let Some(mut edit) = self.color_edit.take() else { return };
        let pal = self.pal;
        let mut hsv_fields = self.edit_hsv;
        let clicked = content_dialog(ctx, pal, "edit_colors", 420.0, "Edit colors", &["OK", "Cancel"], &mut |ui| {
            edit.ui(ui, &pal, 372.0, &mut hsv_fields);
        });
        self.edit_hsv = hsv_fields;
        let (enter, esc) = ctx.input(|i| (i.key_pressed(Key::Enter), i.key_pressed(Key::Escape)));
        match clicked.map(|i| i == 0).or(enter.then_some(true)).or(esc.then_some(false)) {
            Some(true) => {
                let c = edit.color();
                if self.active_slot == 2 {
                    self.s.color2 = c;
                } else {
                    self.s.color1 = c;
                }
                self.s.custom_colors.retain(|x| *x != c);
                self.s.custom_colors.insert(0, c);
                self.s.custom_colors.truncate(10);
            }
            Some(false) => {}
            None => self.color_edit = Some(edit),
        }
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
            self.tex = Some(TiledTexture::new(ctx, "canvas", &self.doc.display, opts));
            self.doc.size_changed = false;
            self.doc.dirty_tex = None;
        } else if let Some(r) = self.doc.dirty_tex.take() {
            let r = r.clamp_to(self.doc.width(), self.doc.height());
            if !r.is_empty()
                && let Some(t) = &mut self.tex
            {
                t.update(&self.doc.display, r, opts);
            }
        }
        let key = format!("{}{:?}", self.s.transparent_selection, self.s.color2);
        let needs = match self.doc.floating() {
            Some(f) => f.tex_dirty || key != self.float_tex_key || self.float_tex.is_none(),
            None => false,
        };
        if needs {
            if let Some(p) = self.doc.effective_float(&self.s) {
                self.float_tex = Some(TiledTexture::new(ctx, "selection", &p, opts));
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
                // Re-check the Plasma color scheme every few seconds so we follow changes live.
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
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close && self.doc.has_unsaved_work() {
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
        // window whenever we own the decorations and the window isn't maximized/fullscreen.
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
        self.ensure_checker(&ctx);
        if self.show_layers {
            egui::Panel::right("layers")
                .exact_size(250.0)
                .frame(
                    egui::Frame::new()
                        .fill(pal.ribbon)
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .stroke(egui::Stroke::new(1.0, pal.separator)),
                )
                .show(ui, |ui| self.layers_panel(ui));
        }
        // Without the status bar, a thin rounded strip keeps canvas content out of the
        // rounded bottom corners.
        if !self.show_status && self.corner > 0 {
            egui::Panel::bottom("corner_pad")
                .exact_size(self.corner as f32)
                .frame(egui::Frame::new().fill(pal.workspace).corner_radius(egui::CornerRadius {
                    nw: 0,
                    ne: 0,
                    sw: self.corner,
                    se: self.corner,
                }))
                .show(ui, |_| {});
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(pal.workspace))
            .show(ui, |ui| self.canvas(ui));

        self.tool_sliders(&ctx);

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
        self.background_picker(&ctx);
        self.thumbnail_window(&ctx);

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

        // Only on change: every viewport command triggers another repaint, which would
        // keep the app busy (and the CPU awake) even when nothing happens.
        let title = self.title();
        if title != self.last_title {
            self.last_title = title.clone();
            ctx.send_viewport_cmd(ViewportCommand::Title(title));
        }
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
            layers: self.show_layers,
        };
        eframe::set_value(storage, eframe::APP_KEY, &p);
    }
}

/// Local date and time for display (falls back to UTC).
fn format_time(t: std::time::SystemTime) -> String {
    let secs = t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    // The system's `date` knows the local time zone and the user's locale.
    if let Ok(out) = std::process::Command::new("date").arg("-d").arg(format!("@{secs}")).arg("+%x %H:%M").output()
        && out.status.success()
    {
        return String::from_utf8_lossy(&out.stdout).trim().to_string();
    }
    // Civil-from-days (Howard Hinnant), UTC.
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{m:02}/{d:02}/{y} {:02}:{:02} UTC", rem / 3600, rem % 3600 / 60)
}

/// Windows 11 content dialog: title and content on the dialog color, buttons of equal
/// width in a footer band, the first one the accent (default) button. Returns the index
/// of the clicked button.
pub fn content_dialog(
    ctx: &egui::Context,
    pal: Palette,
    id: &str,
    width: f32,
    title: &str,
    buttons: &[&str],
    f: &mut dyn FnMut(&mut egui::Ui),
) -> Option<usize> {
    let mut clicked = None;
    let visuals = ctx.global_style().visuals.clone();
    let frame = egui::Frame::new()
        .fill(visuals.window_fill)
        .corner_radius(8)
        .stroke(egui::Stroke::new(1.0, pal.separator))
        .shadow(visuals.popup_shadow);
    egui::Modal::new(egui::Id::new(id)).frame(frame).show(ctx, |ui| {
        // Dialog controls get visible borders, like Fluent text boxes and buttons.
        let w = &mut ui.visuals_mut().widgets;
        w.inactive.weak_bg_fill = w.inactive.bg_fill;
        w.inactive.bg_stroke = egui::Stroke::new(1.0, pal.separator);
        w.hovered.bg_stroke = egui::Stroke::new(1.0, pal.text_dim);
        let spacing = ui.spacing().item_spacing;
        ui.spacing_mut().item_spacing.y = 0.0;
        let inner = width - 48.0;
        egui::Frame::new().inner_margin(egui::Margin { left: 24, right: 24, top: 20, bottom: 24 }).show(ui, |ui| {
            ui.spacing_mut().item_spacing = spacing;
            ui.set_width(inner);
            ui.label(RichText::new(title).size(20.0).strong());
            ui.add_space(12.0);
            f(ui);
        });
        let footer = if pal.dark { Color32::from_rgb(0x20, 0x20, 0x20) } else { Color32::from_rgb(0xF3, 0xF3, 0xF3) };
        let band = egui::Frame::new()
            .fill(footer)
            .corner_radius(egui::CornerRadius { nw: 0, ne: 0, sw: 8, se: 8 })
            .inner_margin(egui::Margin::same(24))
            .show(ui, |ui| {
                ui.set_width(inner);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let n = buttons.len() as f32;
                    let bw = (inner - 8.0 * (n - 1.0)) / n;
                    for (i, label) in buttons.iter().enumerate() {
                        let b = if i == 0 { primary_button(label, pal) } else { Button::new(*label) };
                        if ui.add_sized([bw, 32.0], b).clicked() {
                            clicked = Some(i);
                        }
                    }
                });
            });
        let top = band.response.rect;
        ui.painter().hline(top.x_range(), top.min.y, egui::Stroke::new(1.0, pal.separator));
    });
    clicked
}

fn primary_button(label: &str, pal: Palette) -> Button<'static> {
    let fg = if pal.dark { Color32::BLACK } else { Color32::WHITE };
    Button::new(RichText::new(label.to_string()).color(fg)).fill(pal.accent).min_size(vec2(100.0, 30.0))
}

/// CUPS printers and the index of the default one. None if CUPS isn't installed.
fn list_printers() -> Option<(Vec<String>, usize)> {
    let out = std::process::Command::new("lpstat").arg("-e").output().ok()?;
    let printers: Vec<String> = String::from_utf8_lossy(&out.stdout).lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
    let default = std::process::Command::new("lpstat")
        .arg("-d")
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).rsplit(':').next().map(|s| s.trim().to_string()))
        .and_then(|d| printers.iter().position(|p| *p == d))
        .unwrap_or(0);
    Some((printers, default))
}

/// Paint's single-letter tool shortcuts.
fn tool_for_key(k: Key) -> Option<Tool> {
    Some(match k {
        Key::P => Tool::Pencil,
        Key::E => Tool::Eraser,
        Key::B => Tool::Fill,
        Key::I => Tool::Picker,
        Key::T => Tool::Text,
        Key::Z => Tool::Magnifier,
        Key::S => Tool::Select,
        _ => return None,
    })
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
