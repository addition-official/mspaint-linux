//! The document: committed image, a transient drawing layer, the composited display
//! buffer, undo history and in-progress overlays (selection, pending shape, text box).

use crate::pixels::{self, IRect, new_pixmap};
use crate::shapes::{self, Geom, ShapeParams};
use crate::text::{self, FontManager, TextStyle};
use crate::tools::{BrushKind, ShapeKind, ShapeStyle, Tool};
use egui::{Color32, Pos2, Rect, pos2, vec2};
use std::path::PathBuf;
use tiny_skia::{FillRule, PathBuilder, Pixmap, Transform};

/// User-adjustable tool settings.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub tool: Tool,
    pub brush: BrushKind,
    pub shape: ShapeKind,
    pub outline: ShapeStyle,
    pub fill: ShapeStyle,
    pub color1: Color32,
    pub color2: Color32,
    pub pencil_size: f32,
    pub brush_size: f32,
    pub eraser_size: f32,
    pub shape_size: f32,
    pub text: TextStyle,
    pub transparent_selection: bool,
    pub select_mode: crate::tools::SelectMode,
    pub custom_colors: Vec<Color32>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            tool: Tool::Pencil,
            brush: BrushKind::Brush,
            shape: ShapeKind::Line,
            outline: ShapeStyle::Solid,
            fill: ShapeStyle::None,
            color1: Color32::BLACK,
            color2: Color32::WHITE,
            pencil_size: 1.0,
            brush_size: 5.0,
            eraser_size: 8.0,
            shape_size: 3.0,
            text: TextStyle::default(),
            transparent_selection: false,
            select_mode: crate::tools::SelectMode::Rectangle,
            custom_colors: Vec::new(),
        }
    }
}

impl Settings {
    pub fn size(&self) -> f32 {
        match self.tool {
            Tool::Pencil => self.pencil_size,
            Tool::Eraser => self.eraser_size,
            Tool::Shape => self.shape_size,
            _ => self.brush_size,
        }
    }
}

pub struct Floating {
    pub pix: Pixmap,
    /// Current placement in image coordinates.
    pub rect: Rect,
    /// Where the pixels were lifted from (rect + optional mask), if they came from the canvas.
    pub origin: Option<(IRect, Option<Vec<bool>>)>,
    pub origin_cleared: bool,
    pub changed: bool,
    pub tex_dirty: bool,
    /// Free-form outline in normalized (0..1) coordinates of `rect`.
    pub outline: Option<Vec<Pos2>>,
}

#[derive(Clone, Debug)]
pub struct PendingShape {
    pub kind: ShapeKind,
    pub geom: Geom,
    /// Curve tool: 0 = straight line, 1 = first bend set, 2 = done.
    pub stage: u8,
    /// Drawn with the right mouse button: colours swapped.
    pub swapped: bool,
    pub seed: u32,
}

#[derive(Clone, Debug)]
pub struct TextBox {
    pub rect: Rect,
    pub text: String,
    /// Caret position in chars.
    pub caret: usize,
}

pub enum Overlay {
    None,
    Stroke,
    Shape(PendingShape),
    Text(TextBox),
    Float(Floating),
}

pub struct Document {
    pub image: Pixmap,
    pub layer: Pixmap,
    pub display: Pixmap,
    layer_bbox: Option<IRect>,
    pub layer_opacity: f32,
    /// Region of `display` that must be re-uploaded to the GPU.
    pub dirty_tex: Option<IRect>,
    /// The image size changed: the texture must be recreated.
    pub size_changed: bool,
    undo: Vec<Pixmap>,
    redo: Vec<Pixmap>,
    pub modified: bool,
    pub path: Option<PathBuf>,
    pub file_size: Option<u64>,
    pub overlay: Overlay,
    render_key: String,
}

const HISTORY_BYTES: usize = 768 * 1024 * 1024;
const HISTORY_MAX: usize = 100;

impl Document {
    pub fn new(w: u32, h: u32) -> Self {
        Self::from_pixmap(new_pixmap(w, h, Color32::WHITE), None)
    }

    pub fn from_pixmap(image: Pixmap, path: Option<PathBuf>) -> Self {
        let (w, h) = (image.width(), image.height());
        let file_size = path.as_ref().and_then(|p| std::fs::metadata(p).ok()).map(|m| m.len());
        Self {
            display: image.clone(),
            layer: new_pixmap(w, h, Color32::TRANSPARENT),
            image,
            layer_bbox: None,
            layer_opacity: 1.0,
            dirty_tex: None,
            size_changed: true,
            undo: Vec::new(),
            redo: Vec::new(),
            modified: false,
            path,
            file_size,
            overlay: Overlay::None,
            render_key: String::new(),
        }
    }

    pub fn width(&self) -> u32 {
        self.image.width()
    }
    pub fn height(&self) -> u32 {
        self.image.height()
    }
    pub fn bounds(&self) -> IRect {
        IRect::from_size(self.width(), self.height())
    }

    pub fn name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".into())
    }

    /// Replace the whole image (keeps history).
    fn replace_image(&mut self, img: Pixmap) {
        let size_changed = img.width() != self.width() || img.height() != self.height();
        self.image = img;
        if size_changed {
            self.layer = new_pixmap(self.width(), self.height(), Color32::TRANSPARENT);
            self.size_changed = true;
        } else {
            self.layer.fill(tiny_skia::Color::TRANSPARENT);
        }
        self.layer_bbox = None;
        self.display = self.image.clone();
        self.dirty_tex = Some(self.bounds());
        self.render_key.clear();
    }

    /// Record the current image for undo before modifying it.
    pub fn begin_edit(&mut self) {
        self.undo.push(self.image.clone());
        self.redo.clear();
        self.modified = true;
        let mut total: usize = self.undo.iter().map(|p| p.data().len()).sum();
        while (total > HISTORY_BYTES || self.undo.len() > HISTORY_MAX) && self.undo.len() > 1 {
            total -= self.undo.remove(0).data().len();
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self, settings: &Settings) {
        // Undoing while a selection is being moved reverts the move.
        self.commit_overlay(settings);
        if let Some(prev) = self.undo.pop() {
            self.redo.push(self.image.clone());
            self.replace_image(prev);
            self.modified = true;
        }
    }

    pub fn redo(&mut self, settings: &Settings) {
        self.commit_overlay(settings);
        if let Some(next) = self.redo.pop() {
            self.undo.push(self.image.clone());
            self.replace_image(next);
            self.modified = true;
        }
    }

    // -----------------------------------------------------------------------
    // Layer handling

    /// Recomposite `display` in a region from `image` + `layer`.
    pub fn refresh(&mut self, r: IRect) {
        let r = r.clamp_to(self.width(), self.height());
        if r.is_empty() {
            return;
        }
        pixels::composite_region(&mut self.display, &self.image, &self.layer, self.layer_opacity, r);
        self.dirty_tex = pixels::union_opt(self.dirty_tex, r);
    }

    pub fn layer_touched(&mut self, r: IRect) {
        let r = r.clamp_to(self.width(), self.height());
        if r.is_empty() {
            return;
        }
        self.layer_bbox = pixels::union_opt(self.layer_bbox, r);
        self.refresh(r);
    }

    /// Clear the layer; returns the region that was cleared (needs refresh).
    fn layer_clear(&mut self) -> Option<IRect> {
        let bb = self.layer_bbox.take()?;
        pixels::clear_region(&mut self.layer, bb);
        Some(bb)
    }

    /// Bake the layer into the image.
    fn commit_layer(&mut self) {
        if let Some(bb) = self.layer_bbox.take() {
            let bb = bb.clamp_to(self.width(), self.height());
            let w = self.width() as usize;
            for y in bb.y0..bb.y1 {
                let s = (y as usize * w + bb.x0 as usize) * 4;
                let e = s + bb.width() as usize * 4;
                self.image.data_mut()[s..e].copy_from_slice(&self.display.data()[s..e]);
            }
            pixels::clear_region(&mut self.layer, bb);
        }
        self.layer_opacity = 1.0;
    }

    pub fn begin_stroke(&mut self, opacity: f32) {
        self.commit_overlay_noop();
        self.begin_edit();
        self.layer_opacity = opacity;
        self.overlay = Overlay::Stroke;
    }

    pub fn end_stroke(&mut self) {
        self.commit_layer();
        self.overlay = Overlay::None;
    }

    fn commit_overlay_noop(&mut self) {
        // Strokes never start while another overlay is active (the UI commits first).
        if let Some(r) = self.layer_clear() {
            self.refresh(r);
        }
    }

    // -----------------------------------------------------------------------
    // Pending shapes and text

    /// Re-render a pending shape/text box into the layer if anything changed.
    pub fn render_overlay(&mut self, s: &Settings, fm: &mut FontManager) {
        let key = match &self.overlay {
            Overlay::Shape(sh) => format!(
                "S{:?}{:?}{:?}{:?}{:?}{:?}{}",
                sh.geom, sh.swapped, s.outline, s.fill, s.color1, s.color2, s.shape_size
            ),
            Overlay::Text(t) => format!("T{:?}{:?}{:?}{:?}{}", t.rect, t.text, s.text, s.color1, s.color2.to_hex()),
            _ => return,
        };
        if key == self.render_key {
            return;
        }
        self.render_key = key;
        let cleared = self.layer_clear();
        self.layer_opacity = 1.0;
        let dirty = match &self.overlay {
            Overlay::Shape(sh) => {
                let (oc, fc) = if sh.swapped { (s.color2, s.color1) } else { (s.color1, s.color2) };
                let sp = ShapeParams {
                    outline: s.outline,
                    fill: s.fill,
                    width: s.shape_size,
                    outline_color: oc,
                    fill_color: fc,
                    seed: sh.seed,
                };
                shapes::render(&mut self.layer, sh.kind, &sh.geom, &sp)
            }
            Overlay::Text(t) => {
                let (rect, txt) = (t.rect, t.text.clone());
                text::render(&mut self.layer, fm, &s.text, &txt, rect, s.color1, s.color2)
            }
            _ => unreachable!(),
        };
        if let Some(c) = cleared {
            self.refresh(c);
        }
        self.layer_touched(dirty);
    }

    // -----------------------------------------------------------------------
    // Selection

    pub fn floating(&self) -> Option<&Floating> {
        match &self.overlay {
            Overlay::Float(f) => Some(f),
            _ => None,
        }
    }
    pub fn floating_mut(&mut self) -> Option<&mut Floating> {
        match &mut self.overlay {
            Overlay::Float(f) => Some(f),
            _ => None,
        }
    }

    pub fn select_rect(&mut self, s: &Settings, r: IRect) {
        self.commit_overlay(s);
        let r = r.clamp_to(self.width(), self.height());
        if r.width() < 1 || r.height() < 1 {
            return;
        }
        self.overlay = Overlay::Float(Floating {
            pix: pixels::copy_rect(&self.image, r),
            rect: irect_to_rect(r),
            origin: Some((r, None)),
            origin_cleared: false,
            changed: false,
            tex_dirty: true,
            outline: None,
        });
    }

    pub fn select_all(&mut self, s: &Settings) {
        self.select_rect(s, self.bounds());
    }

    pub fn select_freeform(&mut self, s: &Settings, pts: &[Pos2]) {
        self.commit_overlay(s);
        if pts.len() < 3 {
            return;
        }
        let bb = Rect::from_points(pts);
        let r = IRect::new(
            bb.min.x.floor() as i32,
            bb.min.y.floor() as i32,
            bb.max.x.ceil() as i32 + 1,
            bb.max.y.ceil() as i32 + 1,
        )
        .clamp_to(self.width(), self.height());
        if r.width() < 1 || r.height() < 1 {
            return;
        }
        let mut maskpix = new_pixmap(r.width() as u32, r.height() as u32, Color32::TRANSPARENT);
        let mut pb = PathBuilder::new();
        pb.move_to(pts[0].x - r.x0 as f32, pts[0].y - r.y0 as f32);
        for p in &pts[1..] {
            pb.line_to(p.x - r.x0 as f32, p.y - r.y0 as f32);
        }
        pb.close();
        if let Some(path) = pb.finish() {
            maskpix.fill_path(&path, &pixels::paint_for(Color32::WHITE, false), FillRule::Winding, Transform::identity(), None);
        }
        let mask: Vec<bool> = maskpix.data().chunks_exact(4).map(|p| p[3] > 127).collect();
        let mut pix = pixels::copy_rect(&self.image, r);
        for (px, m) in pix.data_mut().chunks_exact_mut(4).zip(&mask) {
            if !m {
                px.fill(0);
            }
        }
        let rect = irect_to_rect(r);
        let outline = pts
            .iter()
            .map(|p| pos2((p.x - rect.min.x) / rect.width(), (p.y - rect.min.y) / rect.height()))
            .collect();
        self.overlay = Overlay::Float(Floating {
            pix,
            rect,
            origin: Some((r, Some(mask))),
            origin_cleared: false,
            changed: false,
            tex_dirty: true,
            outline: Some(outline),
        });
    }

    pub fn invert_selection(&mut self, s: &Settings) {
        let Some(f) = self.floating() else {
            self.select_all(s);
            return;
        };
        let Some((r, mask)) = f.origin.clone() else { return };
        if f.changed {
            return;
        }
        let (w, h) = (self.width() as i32, self.height() as i32);
        let mut full = vec![true; (w * h) as usize];
        for y in r.y0..r.y1 {
            for x in r.x0..r.x1 {
                let inside = match &mask {
                    Some(m) => m[((y - r.y0) * r.width() + (x - r.x0)) as usize],
                    None => true,
                };
                if inside {
                    full[(y * w + x) as usize] = false;
                }
            }
        }
        let b = self.bounds();
        let mut pix = self.image.clone();
        for (px, m) in pix.data_mut().chunks_exact_mut(4).zip(&full) {
            if !m {
                px.fill(0);
            }
        }
        self.overlay = Overlay::Float(Floating {
            pix,
            rect: irect_to_rect(b),
            origin: Some((b, Some(full))),
            origin_cleared: false,
            changed: false,
            tex_dirty: true,
            outline: None,
        });
    }

    /// Paste a pixmap as a new floating selection at `at`.
    pub fn paste(&mut self, s: &Settings, pix: Pixmap, at: Pos2) {
        self.commit_overlay(s);
        // Grow the canvas if the pasted image is larger, as Paint does.
        let (pw, ph) = (pix.width(), pix.height());
        if pw > self.width() || ph > self.height() {
            self.begin_edit();
            let img = pixels::resize_canvas(&self.image, pw.max(self.width()), ph.max(self.height()), s.color2);
            self.replace_image(img);
        } else {
            self.begin_edit();
        }
        let at = pos2(
            at.x.clamp(0.0, (self.width() - pw) as f32).round(),
            at.y.clamp(0.0, (self.height() - ph) as f32).round(),
        );
        self.overlay = Overlay::Float(Floating {
            rect: Rect::from_min_size(at, vec2(pw as f32, ph as f32)),
            pix,
            origin: None,
            origin_cleared: true,
            changed: true,
            tex_dirty: true,
            outline: None,
        });
        self.modified = true;
    }

    /// Clear the area a selection was lifted from (first modification of the selection).
    pub fn lift(&mut self, s: &Settings) {
        let Some(f) = self.floating() else { return };
        if f.origin_cleared {
            return;
        }
        let (r, mask) = f.origin.clone().expect("origin");
        self.begin_edit();
        let c = s.color2.to_array();
        for y in r.y0..r.y1 {
            for x in r.x0..r.x1 {
                let inside = match &mask {
                    Some(m) => m[((y - r.y0) * r.width() + (x - r.x0)) as usize],
                    None => true,
                };
                if inside {
                    pixels::set_pixel(&mut self.image, x, y, c);
                }
            }
        }
        self.refresh(r);
        if let Some(f) = self.floating_mut() {
            f.origin_cleared = true;
            f.changed = true;
        }
    }

    /// Selection pixels as they will be stamped (respecting transparent selection).
    pub fn effective_float(&self, s: &Settings) -> Option<Pixmap> {
        let f = self.floating()?;
        let mut p = f.pix.clone();
        if s.transparent_selection {
            pixels::key_out(&mut p, s.color2);
        }
        Some(p)
    }

    /// Stamp the selection onto the canvas at its current place and keep it floating
    /// (Ctrl+drag copies, Shift+drag smears). `record` adds an undo step.
    pub fn stamp_float(&mut self, s: &Settings, record: bool) {
        let Some(f) = self.floating() else { return };
        if !f.origin_cleared {
            // The pixels are still in the canvas: leaving them there *is* the copy.
            if record {
                self.begin_edit();
            }
            let f = self.floating_mut().unwrap();
            f.origin_cleared = true;
            f.changed = true;
            return;
        }
        let Some(p) = self.effective_float(s) else { return };
        let r = self.floating().unwrap().rect;
        if record {
            self.begin_edit();
        }
        pixels::draw_scaled(&mut self.image, &p, r.min.x, r.min.y, r.width(), r.height());
        self.refresh(IRect::from_points((r.min.x, r.min.y), (r.max.x, r.max.y), 2.0));
        self.modified = true;
    }

    pub fn delete_selection(&mut self, s: &Settings) {
        if self.floating().is_none() {
            return;
        }
        self.lift(s);
        self.overlay = Overlay::None;
    }

    /// Finish whatever overlay is active, baking it into the image.
    pub fn commit_overlay(&mut self, s: &Settings) {
        match std::mem::replace(&mut self.overlay, Overlay::None) {
            Overlay::None => {}
            Overlay::Stroke => self.commit_layer(),
            Overlay::Shape(_) => {
                self.begin_edit();
                self.commit_layer();
            }
            Overlay::Text(t) => {
                if t.text.is_empty() {
                    if let Some(r) = self.layer_clear() {
                        self.refresh(r);
                    }
                } else {
                    self.begin_edit();
                    self.commit_layer();
                }
            }
            Overlay::Float(f) => {
                if f.changed {
                    let mut p = f.pix;
                    if s.transparent_selection {
                        pixels::key_out(&mut p, s.color2);
                    }
                    let r = f.rect;
                    pixels::draw_scaled(&mut self.image, &p, r.min.x, r.min.y, r.width(), r.height());
                    self.refresh(IRect::from_points((r.min.x, r.min.y), (r.max.x, r.max.y), 2.0));
                    self.modified = true;
                }
            }
        }
        self.render_key.clear();
    }

    /// Drop a pending shape/text without committing.
    pub fn cancel_overlay(&mut self) {
        match &self.overlay {
            Overlay::Shape(_) | Overlay::Text(_) => {
                if let Some(r) = self.layer_clear() {
                    self.refresh(r);
                }
                self.overlay = Overlay::None;
                self.render_key.clear();
            }
            Overlay::Float(f) if !f.changed => self.overlay = Overlay::None,
            _ => {}
        }
    }

    // -----------------------------------------------------------------------
    // Whole-image / selection transforms

    /// Apply a pixel transform to the selection if there is one, otherwise to the whole image.
    pub fn transform(&mut self, s: &Settings, f: impl Fn(&Pixmap) -> Pixmap) {
        if matches!(self.overlay, Overlay::Shape(_) | Overlay::Text(_) | Overlay::Stroke) {
            self.commit_overlay(s);
        }
        if self.floating().is_some() {
            self.lift(s);
            let fl = self.floating_mut().unwrap();
            let np = f(&fl.pix);
            let c = fl.rect.center();
            fl.rect = Rect::from_center_size(c, vec2(np.width() as f32, np.height() as f32));
            fl.rect = Rect::from_min_size(fl.rect.min.round(), fl.rect.size());
            fl.pix = np;
            fl.changed = true;
            fl.tex_dirty = true;
            fl.outline = None;
            return;
        }
        self.begin_edit();
        let img = f(&self.image);
        self.replace_image(img);
    }

    pub fn crop_to_selection(&mut self, s: &Settings) {
        let Some(mut p) = self.effective_float(s) else { return };
        let f = self.floating().unwrap();
        let (w, h) = (f.rect.width().round().max(1.0) as u32, f.rect.height().round().max(1.0) as u32);
        if (w, h) != (p.width(), p.height()) {
            p = pixels::resize(&p, w, h);
        }
        let mut out = new_pixmap(w, h, s.color2);
        out.draw_pixmap(0, 0, p.as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), None);
        self.overlay = Overlay::None;
        self.begin_edit();
        self.replace_image(out);
    }

    pub fn resize_canvas(&mut self, s: &Settings, w: u32, h: u32) {
        self.commit_overlay(s);
        if w == self.width() && h == self.height() {
            return;
        }
        self.begin_edit();
        let img = pixels::resize_canvas(&self.image, w.max(1), h.max(1), s.color2);
        self.replace_image(img);
    }

    /// Fill the image directly (fill tool).
    pub fn flood_fill(&mut self, x: i32, y: i32, c: Color32) {
        let mut img = self.image.clone();
        if let Some(r) = pixels::flood_fill(&mut img, x, y, c) {
            self.begin_edit();
            self.image = img;
            self.refresh(r);
        }
    }

    pub fn mark_saved(&mut self, path: PathBuf) {
        self.file_size = std::fs::metadata(&path).ok().map(|m| m.len());
        self.path = Some(path);
        self.modified = false;
    }
}

pub fn irect_to_rect(r: IRect) -> Rect {
    Rect::from_min_max(pos2(r.x0 as f32, r.y0 as f32), pos2(r.x1 as f32, r.y1 as f32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixels::get_pixel;

    fn px(d: &Document, x: i32, y: i32) -> Color32 {
        get_pixel(&d.image, x, y).unwrap()
    }

    #[test]
    fn flood_fill_and_undo_redo() {
        let s = Settings::default();
        let mut d = Document::new(20, 10);
        d.flood_fill(3, 3, Color32::RED);
        assert_eq!(px(&d, 19, 9), Color32::RED);
        assert!(d.modified && d.can_undo());
        d.undo(&s);
        assert_eq!(px(&d, 19, 9), Color32::WHITE);
        d.redo(&s);
        assert_eq!(px(&d, 0, 0), Color32::RED);
    }

    #[test]
    fn stroke_is_committed_through_layer() {
        let mut d = Document::new(30, 30);
        d.begin_stroke(1.0);
        let r = crate::brush::hard_line(&mut d.layer, pos2(2.0, 2.0), pos2(20.0, 2.0), 1.0, Color32::BLACK);
        d.layer_touched(r);
        // Not in the image until the stroke ends, but visible in the display buffer.
        assert_eq!(px(&d, 10, 2), Color32::WHITE);
        assert_eq!(get_pixel(&d.display, 10, 2).unwrap(), Color32::BLACK);
        d.end_stroke();
        assert_eq!(px(&d, 10, 2), Color32::BLACK);
        assert!(d.layer.data().iter().all(|b| *b == 0));
    }

    #[test]
    fn moving_selection_leaves_background_colour() {
        let mut s = Settings::default();
        s.color2 = Color32::from_rgb(0, 0, 255);
        let mut d = Document::new(40, 40);
        d.flood_fill(0, 0, Color32::WHITE);
        let mut img = d.image.clone();
        pixels::flood_fill(&mut img, 0, 0, Color32::GREEN);
        d.image = img;
        d.select_rect(&s, IRect::new(0, 0, 10, 10));
        d.lift(&s);
        if let Some(f) = d.floating_mut() {
            f.rect = f.rect.translate(vec2(20.0, 20.0));
        }
        d.commit_overlay(&s);
        assert_eq!(px(&d, 5, 5), s.color2, "origin cleared with colour 2");
        assert_eq!(px(&d, 25, 25), Color32::GREEN, "pixels stamped at new place");
        d.undo(&s);
        assert_eq!(px(&d, 5, 5), Color32::GREEN);
    }

    #[test]
    fn ctrl_drag_leaves_a_copy() {
        let s = Settings::default();
        let mut d = Document::new(40, 40);
        let mut img = d.image.clone();
        pixels::flood_fill(&mut img, 0, 0, Color32::GREEN);
        d.image = img;
        d.select_rect(&s, IRect::new(0, 0, 10, 10));
        d.stamp_float(&s, true);
        d.lift(&s);
        if let Some(f) = d.floating_mut() {
            f.rect = f.rect.translate(vec2(20.0, 20.0));
        }
        d.commit_overlay(&s);
        assert_eq!(px(&d, 5, 5), Color32::GREEN, "original stays");
        assert_eq!(px(&d, 25, 25), Color32::GREEN, "copy placed");
        d.undo(&s);
        assert!(!d.can_undo(), "one undo step for the whole copy-move");
    }

    #[test]
    fn colour_eraser_only_replaces_colour_1() {
        let mut d = Document::new(20, 3);
        let mut img = d.image.clone();
        for x in 0..10 {
            pixels::set_pixel(&mut img, x, 1, Color32::RED.to_array());
        }
        d.image = img;
        d.begin_stroke(1.0);
        let r = crate::brush::color_erase(&mut d.layer, &d.image, pos2(0.0, 1.0), pos2(19.0, 1.0), 1.0, Color32::RED, Color32::BLUE);
        d.layer_touched(r);
        d.end_stroke();
        assert_eq!(px(&d, 5, 1), Color32::BLUE);
        assert_eq!(px(&d, 15, 1), Color32::WHITE);
    }

    #[test]
    fn paste_grows_canvas() {
        let s = Settings::default();
        let mut d = Document::new(10, 10);
        d.paste(&s, new_pixmap(30, 5, Color32::RED), pos2(0.0, 0.0));
        d.commit_overlay(&s);
        assert_eq!((d.width(), d.height()), (30, 10));
        assert_eq!(px(&d, 29, 0), Color32::RED);
        assert_eq!(px(&d, 29, 9), s.color2);
    }

    #[test]
    fn rotate_and_resize_canvas() {
        let s = Settings::default();
        let mut d = Document::new(30, 10);
        d.transform(&s, pixels::rotate_cw);
        assert_eq!((d.width(), d.height()), (10, 30));
        d.resize_canvas(&s, 50, 60);
        assert_eq!((d.width(), d.height()), (50, 60));
        d.undo(&s);
        d.undo(&s);
        assert_eq!((d.width(), d.height()), (30, 10));
    }

    #[test]
    fn crop_to_selection() {
        let s = Settings::default();
        let mut d = Document::new(40, 40);
        d.select_rect(&s, IRect::new(5, 5, 25, 15));
        d.crop_to_selection(&s);
        assert_eq!((d.width(), d.height()), (20, 10));
    }

    #[test]
    fn shapes_render() {
        let mut s = Settings::default();
        let mut fm = FontManager::new();
        for kind in ShapeKind::ALL {
            let mut d = Document::new(120, 120);
            s.fill = ShapeStyle::Solid;
            let geom = match kind {
                ShapeKind::Line => Geom::Line(pos2(10.0, 10.0), pos2(100.0, 100.0)),
                ShapeKind::Curve => Geom::Curve(pos2(10.0, 10.0), pos2(60.0, 0.0), pos2(60.0, 110.0), pos2(100.0, 100.0)),
                ShapeKind::Polygon => Geom::Polygon(vec![pos2(10.0, 10.0), pos2(100.0, 20.0), pos2(50.0, 100.0)], true),
                _ => Geom::Boxed(Rect::from_min_max(pos2(10.0, 10.0), pos2(110.0, 100.0))),
            };
            d.overlay = Overlay::Shape(PendingShape { kind, geom, stage: 2, swapped: false, seed: 1 });
            d.render_overlay(&s, &mut fm);
            d.commit_overlay(&s);
            let changed = d.image.data().chunks_exact(4).filter(|p| p != &[255, 255, 255, 255]).count();
            assert!(changed > 50, "{kind:?} drew only {changed} pixels");
        }
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("mspaint-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut d = Document::new(16, 8);
        d.flood_fill(0, 0, Color32::from_rgb(10, 200, 30));
        for ext in ["png", "bmp", "jpg", "gif", "tiff", "webp", "ico"] {
            let p = dir.join(format!("t.{ext}"));
            pixels::save(&d.image, &p).unwrap_or_else(|e| panic!("{ext}: {e}"));
            let back = pixels::load(&p).unwrap();
            assert_eq!((back.width(), back.height()), (16, 8), "{ext}");
            let c = get_pixel(&back, 4, 4).unwrap();
            assert!((c.g() as i32 - 200).abs() < 12, "{ext}: {c:?}");
        }
        let _ = std::fs::remove_dir_all(dir);
    }
}
