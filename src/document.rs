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
#[serde(default)]
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
    /// Pencil / brush opacity in percent.
    pub opacity: f32,
    /// Fill tolerance in percent.
    pub tolerance: f32,
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
            opacity: 100.0,
            tolerance: 10.0,
        }
    }
}

impl Settings {
    pub fn size_mut(&mut self) -> &mut f32 {
        match self.tool {
            Tool::Pencil => &mut self.pencil_size,
            Tool::Eraser => &mut self.eraser_size,
            Tool::Shape => &mut self.shape_size,
            _ => &mut self.brush_size,
        }
    }

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

#[derive(Clone)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    /// The pixels. For the active layer this is a placeholder: its pixels live in
    /// `Document::image` so every tool can keep drawing on `image`.
    pub pix: Pixmap,
}

/// Undo snapshot of the whole layer stack.
#[derive(Clone)]
struct Snapshot {
    layers: Vec<Layer>,
    active: usize,
}

impl Snapshot {
    fn bytes(&self) -> usize {
        self.layers.iter().map(|l| l.pix.data().len()).sum()
    }
}

fn placeholder() -> Pixmap {
    Pixmap::new(1, 1).unwrap()
}

pub struct Document {
    /// Pixels of the active layer.
    pub image: Pixmap,
    pub layers: Vec<Layer>,
    pub active: usize,
    /// Composites of the visible layers below / above the active one.
    below: Option<Pixmap>,
    above: Option<Pixmap>,
    /// The stroke layer erases (to transparency) instead of painting.
    pub layer_erase: bool,
    /// Bumped whenever layer pixels or the stack change (for thumbnails).
    pub layers_rev: u64,
    /// The Background tile under all layers (Win11 Paint: white by default,
    /// hide it with Ctrl+Shift+H to get a transparent canvas).
    pub background: Color32,
    pub background_visible: bool,
    pub layer: Pixmap,
    pub display: Pixmap,
    layer_bbox: Option<IRect>,
    pub layer_opacity: f32,
    /// Region of `display` that must be re-uploaded to the GPU.
    pub dirty_tex: Option<IRect>,
    /// The image size changed: the texture must be recreated.
    pub size_changed: bool,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
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
        Self::from_pixmap(new_pixmap(w, h, Color32::TRANSPARENT), None)
    }

    pub fn from_pixmap(image: Pixmap, path: Option<PathBuf>) -> Self {
        let (w, h) = (image.width(), image.height());
        let file_size = path.as_ref().and_then(|p| std::fs::metadata(p).ok()).map(|m| m.len());
        let has_alpha = path.is_some() && image.data().chunks_exact(4).any(|p| p[3] < 255);
        let mut doc = Self {
            display: image.clone(),
            layer: new_pixmap(w, h, Color32::TRANSPARENT),
            image,
            layers: vec![Layer { name: "Layer 1".into(), visible: true, pix: placeholder() }],
            active: 0,
            below: None,
            above: None,
            layer_erase: false,
            layers_rev: 0,
            background: Color32::WHITE,
            // Pictures with transparency open on a checkerboard; everything else on white.
            background_visible: !has_alpha,
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
        };
        doc.rebuild();
        doc.size_changed = true;
        doc
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

    /// Replace the active layer's pixels (keeps history). Other layers must already
    /// have the same size.
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
        self.render_key.clear();
        self.rebuild();
    }

    /// Apply a pixel transform to every layer (rotate, flip, resize the whole image).
    fn transform_all(&mut self, f: impl Fn(&Pixmap, usize) -> Pixmap) {
        let active = self.active;
        for (i, l) in self.layers.iter_mut().enumerate() {
            if i != active {
                l.pix = f(&l.pix, i);
            }
        }
        let img = f(&self.image, active);
        self.replace_image(img);
    }

    fn snapshot(&self) -> Snapshot {
        let mut layers = self.layers.clone();
        layers[self.active].pix = self.image.clone();
        Snapshot { layers, active: self.active }
    }

    fn restore(&mut self, mut snap: Snapshot) {
        let img = std::mem::replace(&mut snap.layers[snap.active].pix, placeholder());
        self.layers = snap.layers;
        self.active = snap.active;
        self.replace_image(img);
    }

    /// Recompute the below/above composites and the whole display.
    fn rebuild(&mut self) {
        let (w, h) = (self.width(), self.height());
        let below: Vec<&Pixmap> = self.layers[..self.active].iter().filter(|l| l.visible).map(|l| &l.pix).collect();
        let above: Vec<&Pixmap> = self.layers[self.active + 1..].iter().filter(|l| l.visible).map(|l| &l.pix).collect();
        let bg = self.background_visible.then(|| new_pixmap(w, h, self.background));
        self.below = (!below.is_empty() || bg.is_some()).then(|| pixels::flatten(w, h, bg.iter().chain(below)));
        self.above = (!above.is_empty()).then(|| pixels::flatten(w, h, above.into_iter()));
        if self.display.width() != w || self.display.height() != h {
            self.display = new_pixmap(w, h, Color32::TRANSPARENT);
        }
        self.refresh(self.bounds());
        self.layers_rev += 1;
    }

    /// Record the current state for undo before modifying it.
    pub fn begin_edit(&mut self) {
        self.undo.push(self.snapshot());
        self.redo.clear();
        self.modified = true;
        self.layers_rev += 1;
        let mut total: usize = self.undo.iter().map(Snapshot::bytes).sum();
        while (total > HISTORY_BYTES || self.undo.len() > HISTORY_MAX) && self.undo.len() > 1 {
            total -= self.undo.remove(0).bytes();
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
            self.redo.push(self.snapshot());
            self.restore(prev);
            self.modified = true;
        }
    }

    pub fn redo(&mut self, settings: &Settings) {
        self.commit_overlay(settings);
        if let Some(next) = self.redo.pop() {
            self.undo.push(self.snapshot());
            self.restore(next);
            self.modified = true;
        }
    }

    /// All visible layers flattened (what gets saved).
    pub fn flattened(&self) -> Pixmap {
        let (w, h) = (self.width(), self.height());
        let active = self.active;
        let bg = self.background_visible.then(|| new_pixmap(w, h, self.background));
        pixels::flatten(
            w,
            h,
            bg.iter().chain(
                self.layers
                    .iter()
                    .enumerate()
                    .filter(|(_, l)| l.visible)
                    .map(|(i, l)| if i == active { &self.image } else { &l.pix }),
            ),
        )
    }

    /// What erasing / clearing leaves behind: since Paint gained layers, erasing truly
    /// erases (to transparency) and the Background tile shows through.
    pub fn background_for(&self, _s: &Settings) -> Color32 {
        Color32::TRANSPARENT
    }

    pub fn set_background(&mut self, c: Color32, visible: bool) {
        if c == self.background && visible == self.background_visible {
            return;
        }
        self.background = c;
        self.background_visible = visible;
        self.modified = true;
        self.rebuild();
    }

    // -----------------------------------------------------------------------
    // Layer panel operations

    pub fn layer_pix(&self, i: usize) -> &Pixmap {
        if i == self.active { &self.image } else { &self.layers[i].pix }
    }

    pub fn set_active_layer(&mut self, s: &Settings, i: usize) {
        if i == self.active || i >= self.layers.len() {
            return;
        }
        self.commit_overlay(s);
        let img = std::mem::replace(&mut self.image, placeholder());
        self.layers[self.active].pix = img;
        self.image = std::mem::replace(&mut self.layers[i].pix, placeholder());
        self.active = i;
        self.rebuild();
    }

    fn next_layer_name(&self) -> String {
        let n = (1..).find(|n| !self.layers.iter().any(|l| l.name == format!("Layer {n}"))).unwrap();
        format!("Layer {n}")
    }

    pub fn add_layer(&mut self, s: &Settings) {
        self.commit_overlay(s);
        self.begin_edit();
        let name = self.next_layer_name();
        let blank = new_pixmap(self.width(), self.height(), Color32::TRANSPARENT);
        let old = std::mem::replace(&mut self.image, blank);
        self.layers[self.active].pix = old;
        self.active += 1;
        self.layers.insert(self.active, Layer { name, visible: true, pix: placeholder() });
        self.rebuild();
    }

    pub fn duplicate_layer(&mut self, s: &Settings) {
        self.commit_overlay(s);
        self.begin_edit();
        let name = format!("{} copy", self.layers[self.active].name);
        let copy = Layer { name, visible: self.layers[self.active].visible, pix: self.image.clone() };
        self.layers.insert(self.active, copy);
        self.active += 1;
        self.rebuild();
    }

    pub fn delete_layer(&mut self, s: &Settings) {
        if self.layers.len() < 2 {
            return;
        }
        self.commit_overlay(s);
        self.begin_edit();
        self.layers.remove(self.active);
        self.active = self.active.saturating_sub(1).min(self.layers.len() - 1);
        self.image = std::mem::replace(&mut self.layers[self.active].pix, placeholder());
        self.rebuild();
    }

    /// Merge the active layer into the one below it.
    pub fn merge_down(&mut self, s: &Settings) {
        if self.active == 0 {
            return;
        }
        self.commit_overlay(s);
        self.begin_edit();
        let top = std::mem::replace(&mut self.image, placeholder());
        let top_visible = self.layers[self.active].visible;
        self.layers.remove(self.active);
        self.active -= 1;
        let mut below = std::mem::replace(&mut self.layers[self.active].pix, placeholder());
        if top_visible {
            below.draw_pixmap(0, 0, top.as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), None);
        }
        self.image = below;
        self.rebuild();
    }

    /// Move the active layer up (+1) or down (-1) in the stack.
    pub fn move_layer(&mut self, s: &Settings, dir: i32) {
        let to = self.active as i32 + dir;
        if to < 0 || to >= self.layers.len() as i32 {
            return;
        }
        self.commit_overlay(s);
        self.begin_edit();
        self.layers.swap(self.active, to as usize);
        self.active = to as usize;
        self.rebuild();
    }

    /// Move layer `from` to stack position `to` (drag and drop in the panel).
    pub fn move_layer_to(&mut self, s: &Settings, from: usize, to: usize) {
        let n = self.layers.len();
        if from >= n || to >= n || from == to {
            return;
        }
        self.commit_overlay(s);
        self.begin_edit();
        // Put the active layer's pixels back so the stack can be reordered freely.
        let active = self.active;
        self.layers[active].pix = std::mem::replace(&mut self.image, placeholder());
        let l = self.layers.remove(from);
        self.layers.insert(to, l);
        self.active = if active == from {
            to
        } else if from < active && to >= active {
            active - 1
        } else if from > active && to <= active {
            active + 1
        } else {
            active
        };
        self.image = std::mem::replace(&mut self.layers[self.active].pix, placeholder());
        self.rebuild();
    }

    pub fn toggle_layer_visible(&mut self, s: &Settings, i: usize) {
        self.commit_overlay(s);
        self.begin_edit();
        self.layers[i].visible = !self.layers[i].visible;
        self.rebuild();
    }

    // -----------------------------------------------------------------------
    // Layer handling

    /// Recomposite `display` in a region from `image` + `layer`.
    pub fn refresh(&mut self, r: IRect) {
        let r = r.clamp_to(self.width(), self.height());
        if r.is_empty() {
            return;
        }
        if !self.layers[self.active].visible {
            pixels::clear_region(&mut self.display, r);
        } else if self.layer_erase {
            pixels::erase_region(&mut self.display, &self.image, &self.layer, r);
        } else {
            pixels::composite_region(&mut self.display, &self.image, &self.layer, self.layer_opacity, r);
        }
        if let Some(b) = &self.below {
            pixels::under_region(&mut self.display, b, r);
        }
        if let Some(a) = &self.above {
            pixels::over_region(&mut self.display, a, r);
        }
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
            let base = pixels::copy_rect(&self.image, bb);
            let mut stroke = pixels::copy_rect(&self.layer, bb);
            let local = IRect::from_size(bb.width() as u32, bb.height() as u32);
            let mut out = base.clone();
            if self.layer_erase {
                pixels::erase_region(&mut out, &base, &stroke, local);
            } else {
                pixels::composite_region(&mut out, &base, &stroke, self.layer_opacity, local);
            }
            pixels::draw_exact(&mut self.image, &out, bb.x0, bb.y0);
            pixels::clear_region(&mut self.layer, bb);
            stroke.fill(tiny_skia::Color::TRANSPARENT);
            self.layers_rev += 1;
        }
        self.layer_opacity = 1.0;
        self.layer_erase = false;
    }

    pub fn begin_stroke(&mut self, opacity: f32, erase: bool) {
        self.commit_overlay_noop();
        self.begin_edit();
        self.layer_opacity = opacity;
        self.layer_erase = erase;
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
        self.layer_erase = false;
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
            let (nw, nh) = (pw.max(self.width()), ph.max(self.height()));
            self.transform_all(|p, _| pixels::resize_canvas(p, nw, nh, Color32::TRANSPARENT));
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
        let c = self.background_for(s).to_array();
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
        self.transform_all(|p, _| f(p));
    }

    /// Apply a pixel operation to the selection, or to the active layer only.
    pub fn apply_active(&mut self, s: &Settings, f: impl Fn(&mut Pixmap)) {
        if matches!(self.overlay, Overlay::Shape(_) | Overlay::Text(_) | Overlay::Stroke) {
            self.commit_overlay(s);
        }
        if self.floating().is_some() {
            self.lift(s);
            let fl = self.floating_mut().unwrap();
            f(&mut fl.pix);
            fl.changed = true;
            fl.tex_dirty = true;
            return;
        }
        self.begin_edit();
        let mut img = self.image.clone();
        f(&mut img);
        self.replace_image(img);
    }

    pub fn crop_to_selection(&mut self, s: &Settings) {
        let Some(mut p) = self.effective_float(s) else { return };
        let f = self.floating().unwrap();
        let (w, h) = (f.rect.width().round().max(1.0) as u32, f.rect.height().round().max(1.0) as u32);
        if (w, h) != (p.width(), p.height()) {
            p = pixels::resize(&p, w, h);
        }
        let mut out = new_pixmap(w, h, self.background_for(s));
        out.draw_pixmap(0, 0, p.as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), None);
        // Other layers are cropped to the same rectangle.
        let r = f.rect;
        let crop = IRect::new(r.min.x.round() as i32, r.min.y.round() as i32, r.min.x.round() as i32 + w as i32, r.min.y.round() as i32 + h as i32);
        self.overlay = Overlay::None;
        self.begin_edit();
        let active = self.active;
        for (i, l) in self.layers.iter_mut().enumerate() {
            if i != active {
                l.pix = pixels::copy_rect(&l.pix, crop);
            }
        }
        self.replace_image(out);
    }

    pub fn resize_canvas(&mut self, s: &Settings, w: u32, h: u32) {
        self.commit_overlay(s);
        if w == self.width() && h == self.height() {
            return;
        }
        self.begin_edit();
        self.transform_all(|p, _| pixels::resize_canvas(p, w.max(1), h.max(1), Color32::TRANSPARENT));
    }

    /// Fill the image directly (fill tool).
    pub fn flood_fill(&mut self, x: i32, y: i32, c: Color32, tolerance: u8) {
        let mut img = self.image.clone();
        if let Some(r) = pixels::flood_fill(&mut img, x, y, c, tolerance) {
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

    /// Pixel of the active layer.
    fn px(d: &Document, x: i32, y: i32) -> Color32 {
        get_pixel(&d.image, x, y).unwrap()
    }

    /// What's visible on screen (layers over the Background tile).
    fn vis(d: &Document, x: i32, y: i32) -> Color32 {
        get_pixel(&d.display, x, y).unwrap()
    }

    #[test]
    fn flood_fill_and_undo_redo() {
        let s = Settings::default();
        let mut d = Document::new(20, 10);
        d.flood_fill(3, 3, Color32::RED, 0);
        assert_eq!(px(&d, 19, 9), Color32::RED);
        assert!(d.modified && d.can_undo());
        d.undo(&s);
        assert_eq!(px(&d, 19, 9), Color32::TRANSPARENT);
        assert_eq!(vis(&d, 19, 9), Color32::WHITE, "background tile shows through");
        d.redo(&s);
        assert_eq!(px(&d, 0, 0), Color32::RED);
    }

    #[test]
    fn fill_tolerance() {
        let mut d = Document::new(10, 1);
        let mut img = d.image.clone();
        for x in 0..10 {
            let g = 100 + x as u8 * 3; // a gentle gradient
            pixels::set_pixel(&mut img, x, 0, Color32::from_gray(g).to_array());
        }
        d.image = img.clone();
        d.flood_fill(0, 0, Color32::RED, 0);
        assert_eq!(px(&d, 1, 0), Color32::from_gray(103), "exact fill stops at the first change");
        d.image = img;
        d.flood_fill(0, 0, Color32::RED, 30);
        assert_eq!(px(&d, 9, 0), Color32::RED, "tolerance covers the gradient");
        // Filling with a colour inside the tolerance must terminate.
        d.flood_fill(0, 0, Color32::from_rgb(250, 10, 10), 255);
    }

    #[test]
    fn stroke_is_committed_through_layer() {
        let mut d = Document::new(30, 30);
        d.begin_stroke(1.0, false);
        let r = crate::brush::hard_line(&mut d.layer, pos2(2.0, 2.0), pos2(20.0, 2.0), 1.0, Color32::BLACK);
        d.layer_touched(r);
        // Not in the image until the stroke ends, but visible in the display buffer.
        assert_eq!(px(&d, 10, 2), Color32::TRANSPARENT);
        assert_eq!(get_pixel(&d.display, 10, 2).unwrap(), Color32::BLACK);
        d.end_stroke();
        assert_eq!(px(&d, 10, 2), Color32::BLACK);
        assert!(d.layer.data().iter().all(|b| *b == 0));
    }

    #[test]
    fn moving_selection_leaves_transparency() {
        let mut s = Settings::default();
        s.color2 = Color32::from_rgb(0, 0, 255);
        let mut d = Document::new(40, 40);
        d.flood_fill(0, 0, Color32::WHITE, 0);
        let mut img = d.image.clone();
        pixels::flood_fill(&mut img, 0, 0, Color32::GREEN, 0);
        d.image = img;
        d.select_rect(&s, IRect::new(0, 0, 10, 10));
        d.lift(&s);
        if let Some(f) = d.floating_mut() {
            f.rect = f.rect.translate(vec2(20.0, 20.0));
        }
        d.commit_overlay(&s);
        assert_eq!(px(&d, 5, 5), Color32::TRANSPARENT, "origin erased");
        assert_eq!(vis(&d, 5, 5), Color32::WHITE, "background tile shows through");
        assert_eq!(px(&d, 25, 25), Color32::GREEN, "pixels stamped at new place");
        d.undo(&s);
        assert_eq!(px(&d, 5, 5), Color32::GREEN);
    }

    #[test]
    fn ctrl_drag_leaves_a_copy() {
        let s = Settings::default();
        let mut d = Document::new(40, 40);
        let mut img = d.image.clone();
        pixels::flood_fill(&mut img, 0, 0, Color32::GREEN, 0);
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
        d.begin_stroke(1.0, false);
        let r = crate::brush::color_erase(&mut d.layer, &d.image, pos2(0.0, 1.0), pos2(19.0, 1.0), 1.0, Color32::RED, Color32::BLUE);
        d.layer_touched(r);
        d.end_stroke();
        assert_eq!(px(&d, 5, 1), Color32::BLUE);
        assert_eq!(px(&d, 15, 1), Color32::TRANSPARENT);
    }

    #[test]
    fn layers_composite_hide_merge_and_undo() {
        let s = Settings::default();
        let mut d = Document::new(10, 10);
        d.flood_fill(0, 0, Color32::RED, 0);
        d.add_layer(&s);
        assert_eq!((d.layers.len(), d.active), (2, 1));
        d.begin_stroke(1.0, false);
        let r = crate::brush::hard_line(&mut d.layer, pos2(0.0, 5.0), pos2(9.0, 5.0), 1.0, Color32::BLUE);
        d.layer_touched(r);
        d.end_stroke();
        assert_eq!(vis(&d, 3, 5), Color32::BLUE);
        assert_eq!(vis(&d, 3, 1), Color32::RED, "lower layer visible through the transparent one");
        d.toggle_layer_visible(&s, 1);
        assert_eq!(vis(&d, 3, 5), Color32::RED);
        d.toggle_layer_visible(&s, 1);
        // Erasing on the top layer reveals the layer below.
        d.begin_stroke(1.0, true);
        let r = crate::brush::hard_line(&mut d.layer, pos2(3.0, 5.0), pos2(3.0, 5.0), 1.0, Color32::BLACK);
        d.layer_touched(r);
        d.end_stroke();
        assert_eq!(vis(&d, 3, 5), Color32::RED);
        assert_eq!(vis(&d, 4, 5), Color32::BLUE);
        d.merge_down(&s);
        assert_eq!((d.layers.len(), d.active), (1, 0));
        assert_eq!(px(&d, 4, 5), Color32::BLUE);
        d.undo(&s);
        assert_eq!(d.layers.len(), 2, "undo restores the merged layer");
        // Hiding the background makes the canvas transparent.
        d.set_active_layer(&s, 1);
        d.set_background(Color32::WHITE, false);
        assert_eq!(get_pixel(&d.flattened(), 3, 5).unwrap(), Color32::RED);
        d.set_active_layer(&s, 0);
        d.toggle_layer_visible(&s, 0);
        assert_eq!(get_pixel(&d.flattened(), 0, 0).unwrap(), Color32::TRANSPARENT);
    }

    #[test]
    fn paste_grows_canvas() {
        let s = Settings::default();
        let mut d = Document::new(10, 10);
        d.paste(&s, new_pixmap(30, 5, Color32::RED), pos2(0.0, 0.0));
        d.commit_overlay(&s);
        assert_eq!((d.width(), d.height()), (30, 10));
        assert_eq!(px(&d, 29, 0), Color32::RED);
        assert_eq!(px(&d, 29, 9), Color32::TRANSPARENT);
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
            let changed = d.image.data().chunks_exact(4).filter(|p| p[3] > 0).count();
            assert!(changed > 50, "{kind:?} drew only {changed} pixels");
        }
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("mspaint-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut d = Document::new(16, 8);
        d.flood_fill(0, 0, Color32::from_rgb(10, 200, 30), 0);
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
