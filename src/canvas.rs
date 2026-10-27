//! The canvas view: scrolling, zoom, rulers, gridlines, overlays and all mouse
//! interaction for the drawing tools.

use crate::app::{Action, Drag, Hit, PaintApp};
use crate::brush::{self, StrokeCtx};
use crate::cursor::{CanvasCursor, FILL_HOTSPOT, PICKER_HOTSPOT};
use crate::icons::Icon;
use crate::document::{Overlay, PendingShape, TextBox};
use crate::pixels::{self, IRect};
use crate::shapes::Geom;
use crate::tools::{BrushKind, SelectMode, ShapeKind, Tool};
use egui::{
    Color32, CursorIcon, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder, Vec2, pos2, vec2,
};

const MARGIN: f32 = 24.0;
const HANDLE: f32 = 7.0;

fn handle_points(r: Rect) -> [Pos2; 8] {
    let c = r.center();
    [
        r.left_top(),
        pos2(c.x, r.min.y),
        r.right_top(),
        pos2(r.max.x, c.y),
        r.right_bottom(),
        pos2(c.x, r.max.y),
        r.left_bottom(),
        pos2(r.min.x, c.y),
    ]
}

/// Screen position of the rotate handle above a selection.
fn rotate_handle(r: Rect) -> Pos2 {
    pos2(r.center().x, r.min.y - 24.0)
}

fn hit_box(r: Rect, p: Pos2, with_handles: bool) -> Option<Hit> {
    if with_handles {
        for (i, h) in handle_points(r).iter().enumerate() {
            if (p - *h).abs().max_elem() <= HANDLE {
                return Some(Hit::Handle(i as u8));
            }
        }
    }
    r.contains(p).then_some(Hit::Inside)
}

fn apply_hit(r: Rect, hit: Hit, d: Vec2) -> Rect {
    match hit {
        Hit::Inside => r.translate(d),
        Hit::Handle(i) => {
            let (mut min, mut max) = (r.min, r.max);
            if matches!(i, 0 | 6 | 7) {
                min.x += d.x;
            }
            if matches!(i, 2..=4) {
                max.x += d.x;
            }
            if matches!(i, 0..=2) {
                min.y += d.y;
            }
            if matches!(i, 4..=6) {
                max.y += d.y;
            }
            let mut out = Rect::from_two_pos(min, max);
            if out.width() < 1.0 {
                out.max.x = out.min.x + 1.0;
            }
            if out.height() < 1.0 {
                out.max.y = out.min.y + 1.0;
            }
            out
        }
    }
}

/// Resize `r` from corner handle `h` while keeping `start`'s aspect ratio,
/// anchored at the opposite corner.
fn keep_aspect(start: Rect, r: Rect, h: u8) -> Rect {
    let scale = (r.width() / start.width().max(1e-3)).max(r.height() / start.height().max(1e-3));
    let size = start.size() * scale;
    let anchor = match h {
        0 => start.right_bottom(),
        2 => start.left_bottom(),
        4 => start.left_top(),
        _ => start.right_top(),
    };
    let corner = match h {
        0 => anchor - size,
        2 => anchor + vec2(size.x, -size.y),
        4 => anchor + size,
        _ => anchor + vec2(-size.x, size.y),
    };
    Rect::from_two_pos(anchor, corner)
}

/// Canvas resize handles: 0 right edge, 1 bottom edge, 2 corner.
fn canvas_handle_cursor(h: u8) -> CursorIcon {
    match h {
        0 => CursorIcon::ResizeHorizontal,
        1 => CursorIcon::ResizeVertical,
        _ => CursorIcon::ResizeNwSe,
    }
}

fn handle_cursor(h: Hit) -> CursorIcon {
    match h {
        Hit::Inside => CursorIcon::Move,
        Hit::Handle(0 | 4) => CursorIcon::ResizeNwSe,
        Hit::Handle(2 | 6) => CursorIcon::ResizeNeSw,
        Hit::Handle(1 | 5) => CursorIcon::ResizeVertical,
        Hit::Handle(_) => CursorIcon::ResizeHorizontal,
    }
}

/// Map a point from one rect to another (used when resizing lines/curves by their box).
fn remap(p: Pos2, from: Rect, to: Rect) -> Pos2 {
    let fx = if from.width().abs() < 1e-3 { 0.5 } else { (p.x - from.min.x) / from.width() };
    let fy = if from.height().abs() < 1e-3 { 0.5 } else { (p.y - from.min.y) / from.height() };
    pos2(to.min.x + fx * to.width(), to.min.y + fy * to.height())
}

fn snap45(a: Pos2, b: Pos2) -> Pos2 {
    let d = b - a;
    let ang = (d.angle() / std::f32::consts::FRAC_PI_4).round() * std::f32::consts::FRAC_PI_4;
    let len = d.length();
    a + vec2(ang.cos(), ang.sin()) * len
}

fn square_from(a: Pos2, b: Pos2) -> Pos2 {
    let d = b - a;
    let m = d.x.abs().max(d.y.abs());
    a + vec2(m * d.x.signum(), m * d.y.signum())
}

fn dashed_rect(p: &egui::Painter, r: Rect, accent: Color32) {
    let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    p.extend(Shape::dashed_line(&pts, Stroke::new(1.0, Color32::WHITE), 4.0, 0.0));
    p.extend(Shape::dashed_line(&pts, Stroke::new(1.0, accent), 4.0, 4.0));
}

fn draw_handles(p: &egui::Painter, r: Rect, accent: Color32) {
    for h in handle_points(r) {
        let hr = Rect::from_center_size(h, vec2(HANDLE, HANDLE));
        p.rect_filled(hr, 1.0, Color32::WHITE);
        p.rect_stroke(hr, 1.0, Stroke::new(1.0, accent), StrokeKind::Inside);
    }
}

impl PaintApp {
    pub fn canvas(&mut self, ui: &mut Ui) {
        let full = ui.available_rect_before_wrap();
        let ruler = if self.show_rulers { 20.0 } else { 0.0 };
        let area = Rect::from_min_max(full.min + vec2(ruler, ruler), full.max);
        let img_size = vec2(self.doc.width() as f32, self.doc.height() as f32) * self.zoom;
        let content = vec2(
            area.width().max(img_size.x + 2.0 * MARGIN),
            area.height().max(img_size.y + 2.0 * MARGIN),
        );
        let offset = (content - img_size) / 2.0;

        let mut sa = egui::ScrollArea::both().id_salt("canvas_scroll").auto_shrink(false);
        if let Some((img_pt, anchor)) = self.pending_zoom_anchor.take() {
            let want = area.min.to_vec2() + offset + img_pt.to_vec2() * self.zoom - anchor.to_vec2();
            let max = (content - area.size()).max(Vec2::ZERO);
            sa = sa.scroll_offset(want.clamp(Vec2::ZERO, max));
        } else if let Some(s) = self.pending_scroll.take() {
            let max = (content - area.size()).max(Vec2::ZERO);
            sa = sa.scroll_offset(s.clamp(Vec2::ZERO, max));
        }
        let mut child = ui.new_child(UiBuilder::new().max_rect(area));
        let out = sa.show(&mut child, |ui| {
            let (rect, resp) = ui.allocate_exact_size(content, Sense::click_and_drag());
            let img = Rect::from_min_size(rect.min + offset, img_size);
            self.image_rect = img;
            self.canvas_input(ui, &resp, img);
            self.canvas_paint(ui, img);
            if let (Some(c), Some(at)) = (self.canvas_cursor, ui.input(|i| i.pointer.latest_pos())) {
                self.apply_canvas_cursor(ui.ctx(), c, at);
            }
            if self.s.tool == Tool::Select && self.drag.is_none() {
                resp.context_menu(|ui| self.context_menu(ui));
            }
        });
        self.scroll_offset = out.state.offset;
        self.viewport = out.inner_rect;
        if self.show_rulers {
            self.paint_rulers(ui, full, ruler);
        }
        ui.allocate_rect(full, Sense::hover());
    }

    fn context_menu(&mut self, ui: &mut Ui) {
        let has_sel = self.doc.floating().is_some();
        let items: [(&str, Action, bool); 7] = [
            ("Cut", Action::Cut, has_sel),
            ("Copy", Action::Copy, has_sel),
            ("Paste", Action::Paste, true),
            ("Select all", Action::SelectAll, true),
            ("Invert selection", Action::InvertSelection, true),
            ("Delete", Action::DeleteSelection, has_sel),
            ("Crop", Action::Crop, has_sel),
        ];
        for (label, a, en) in items {
            if ui.add_enabled(en, egui::Button::new(label)).clicked() {
                self.push(a);
            }
        }
        if ui.button("Copy visible layers").clicked() {
            self.push(Action::CopyVisible);
        }
        ui.separator();
        ui.menu_button("Rotate", |ui| {
            for (label, a) in [
                ("Rotate right 90°", Action::RotateRight),
                ("Rotate left 90°", Action::RotateLeft),
                ("Rotate 180°", Action::Rotate180),
                ("Custom rotate…", Action::CustomRotate),
                ("Flip vertical", Action::FlipVertical),
                ("Flip horizontal", Action::FlipHorizontal),
            ] {
                if ui.button(label).clicked() {
                    self.push(a);
                }
            }
        });
        if ui.button("Resize and skew").clicked() {
            self.push(Action::ResizeDialog);
        }
        if ui.button("Invert colour").clicked() {
            self.push(Action::InvertColors);
        }
        if ui.button("Remove background").clicked() {
            self.push(Action::RemoveBackground);
        }
    }

    // -----------------------------------------------------------------------
    // Input

    fn stroke_color(&self, secondary: bool) -> Color32 {
        match (self.s.tool, secondary) {
            (Tool::Eraser, _) => self.s.color2,
            (_, false) => self.s.color1,
            (_, true) => self.s.color2,
        }
    }

    fn canvas_input(&mut self, ui: &Ui, resp: &Response, img: Rect) {
        let z = self.zoom;
        let to_img = |p: Pos2| ((p - img.min) / z).to_pos2();
        let to_scr = |p: Pos2| img.min + p.to_vec2() * z;
        let (ptr, pp, ps, pm, down, shift, zoom_delta, dbl, ctrl) = ui.input(|i| {
            (
                i.pointer.latest_pos(),
                i.pointer.button_pressed(egui::PointerButton::Primary),
                i.pointer.button_pressed(egui::PointerButton::Secondary),
                i.pointer.button_pressed(egui::PointerButton::Middle),
                i.pointer.any_down(),
                i.modifiers.shift,
                i.zoom_delta(),
                i.pointer.button_double_clicked(egui::PointerButton::Primary),
                i.modifiers.command,
            )
        });
        let hovered = resp.contains_pointer();
        self.cursor_img = if hovered || self.drag.is_some() { ptr.map(to_img) } else { None };
        self.canvas_cursor = None;

        if hovered && (zoom_delta - 1.0).abs() > 1e-4 {
            self.set_zoom(self.zoom * zoom_delta, ptr);
        }

        // Continue an ongoing drag.
        if let Some(drag) = self.drag.clone() {
            if let Some(ps_) = ptr {
                // Keep the tool's cursor while the button is held.
                self.canvas_cursor = Some(self.drag_cursor(&drag, to_img(ps_), to_scr));
                self.drag_move(drag, to_img(ps_), ps_, shift);
            }
            if !down {
                self.drag_end();
            }
            ui.ctx().request_repaint();
            return;
        }

        let Some(p_scr) = ptr else { return };
        let p = to_img(p_scr);

        // Hover cursors (not over the floating sliders/popups, which take the pointer).
        if hovered && !self.on_window_edge {
            self.canvas_cursor = Some(self.hover_cursor(p_scr, p, img, to_scr));
        }
        if !hovered || ui.ctx().any_popup_open() || self.on_window_edge {
            return;
        }
        if pm {
            self.drag = Some(Drag::Pan { last: p_scr });
            return;
        }
        if !(pp || ps) {
            return;
        }
        let secondary = ps && !pp;

        // Canvas resize handles.
        if !secondary && let Some(h) = self.canvas_handle_hit(p_scr, img) {
            self.doc.commit_overlay(&self.s);
            self.drag = Some(Drag::CanvasResize {
                handle: h,
                size: vec2(self.doc.width() as f32, self.doc.height() as f32),
            });
            return;
        }

        // Interact with the active overlay.
        if !secondary && self.overlay_press(p, p_scr, to_scr, dbl, ctrl, shift) {
            return;
        }
        if secondary && matches!(self.doc.overlay, Overlay::Float(_)) && self.s.tool == Tool::Select {
            return; // context menu
        }
        // Clicking away from a text box only finishes it, as in Windows 11 Paint.
        let finished_text = matches!(self.doc.overlay, Overlay::Text(_));
        self.doc.commit_overlay(&self.s);
        if finished_text && self.s.tool == Tool::Text {
            return;
        }

        let inside = p.x >= 0.0 && p.y >= 0.0 && p.x < self.doc.width() as f32 && p.y < self.doc.height() as f32;
        match self.s.tool {
            Tool::Select => {
                if secondary {
                    return;
                }
                let start = self.clamp_img(p).round();
                self.drag = Some(match self.s.select_mode {
                    SelectMode::Rectangle => Drag::Marquee { start, cur: start },
                    SelectMode::FreeForm => Drag::Lasso { pts: vec![self.clamp_img(p)] },
                });
            }
            Tool::Pencil | Tool::Eraser | Tool::Brush => {
                let user = if matches!(self.s.tool, Tool::Pencil | Tool::Brush) { self.s.opacity / 100.0 } else { 1.0 };
                let opacity = user * if self.s.tool == Tool::Brush { self.s.brush.layer_opacity() } else { 1.0 };
                // The eraser erases to transparency (the Background tile shows through).
                let erase = self.s.tool == Tool::Eraser && !secondary;
                self.doc.begin_stroke(opacity, erase);
                self.stroke_ctx = Some(StrokeCtx::new(self.s.size()));
                self.drag = Some(Drag::Stroke { secondary, last: p });
                self.stroke_to(p, p, secondary);
            }
            Tool::Fill => {
                if inside {
                    let c = if secondary { self.s.color2 } else { self.s.color1 };
                    let tol = (self.s.tolerance / 100.0 * 255.0).round() as u8;
                    self.doc.flood_fill(p.x as i32, p.y as i32, c, tol);
                }
            }
            Tool::Picker => {
                if inside && let Some(c) = pixels::get_pixel(&self.doc.display, p.x as i32, p.y as i32) {
                    let [r, g, b, _] = c.to_srgba_unmultiplied();
                    let c = Color32::from_rgb(r, g, b);
                    if secondary {
                        self.s.color2 = c;
                    } else {
                        self.s.color1 = c;
                    }
                    let prev = self.prev_tool;
                    self.set_tool(if prev == Tool::Picker { Tool::Pencil } else { prev });
                }
            }
            Tool::Magnifier => self.zoom_step(if secondary { -1 } else { 1 }, Some(p_scr)),
            Tool::Text => {
                if !secondary {
                    self.drag = Some(Drag::TextNew { start: p, cur: p });
                }
            }
            Tool::Shape => {
                let kind = self.s.shape;
                let geom = match kind {
                    ShapeKind::Line => Geom::Line(p, p),
                    ShapeKind::Curve => Geom::Curve(p, p, p, p),
                    ShapeKind::Polygon => Geom::Polygon(vec![p, p], false),
                    _ => Geom::Boxed(Rect::from_two_pos(p, p)),
                };
                let seed = StrokeCtx::new(1.0).seed;
                self.doc.overlay = Overlay::Shape(PendingShape { kind, geom, stage: 0, swapped: secondary, seed });
                self.drag = Some(Drag::ShapeNew { start: p });
            }
        }
    }

    fn clamp_img(&self, p: Pos2) -> Pos2 {
        pos2(p.x.clamp(0.0, self.doc.width() as f32), p.y.clamp(0.0, self.doc.height() as f32))
    }

    fn canvas_handles(&self, img: Rect) -> [Pos2; 3] {
        let o = 5.0;
        [
            pos2(img.max.x + o, img.center().y),
            pos2(img.center().x, img.max.y + o),
            pos2(img.max.x + o, img.max.y + o),
        ]
    }

    fn canvas_handle_hit(&self, p: Pos2, img: Rect) -> Option<u8> {
        self.canvas_handles(img)
            .iter()
            .position(|h| (p - *h).abs().max_elem() <= HANDLE)
            .map(|i| i as u8)
    }

    /// Returns true if the press was consumed by the active overlay.
    #[allow(clippy::too_many_arguments)]
    fn overlay_press(&mut self, p: Pos2, p_scr: Pos2, to_scr: impl Fn(Pos2) -> Pos2, dbl: bool, ctrl: bool, shift: bool) -> bool {
        let z = self.zoom;
        match &mut self.doc.overlay {
            Overlay::Float(f) => {
                let r = Rect::from_min_max(to_scr(f.rect.min), to_scr(f.rect.max));
                // Free rotation (Paint 11.2601+): drag the handle above the selection.
                if (rotate_handle(r) - p_scr).length() <= 9.0 {
                    let (w, h) = (f.rect.width().round().max(1.0) as u32, f.rect.height().round().max(1.0) as u32);
                    let base = if (w, h) == (f.pix.width(), f.pix.height()) { f.pix.clone() } else { pixels::resize(&f.pix, w, h) };
                    let center = f.rect.center();
                    let start = (p - center).angle().to_degrees();
                    self.rotate_base = Some(base);
                    let s = self.s.clone();
                    self.doc.lift(&s);
                    self.drag = Some(Drag::Rotate { center, start, angle: 0.0 });
                    return true;
                }
                if let Some(hit) = hit_box(r, p_scr, true) {
                    let start_rect = f.rect;
                    // Ctrl+drag leaves a copy behind; Shift+drag smears a trail of copies.
                    if hit == Hit::Inside && (ctrl || shift) {
                        let s = self.s.clone();
                        self.doc.stamp_float(&s, true);
                    }
                    self.drag = Some(Drag::Box { hit, start_ptr: p, start_rect, start_geom: None });
                    return true;
                }
            }
            Overlay::Shape(sh) => match &mut sh.geom {
                Geom::Curve(a, _, _, b) if sh.stage < 2 => {
                    let _ = (a, b);
                    self.drag = Some(Drag::CurveBend);
                    self.drag_move(Drag::CurveBend, p, p_scr, false);
                    return true;
                }
                Geom::Polygon(pts, false) => {
                    let first = pts[0];
                    if dbl || ((to_scr(first) - p_scr).length() < 8.0 && pts.len() > 2) {
                        if dbl && pts.len() > 2 {
                            pts.pop();
                        }
                        let pts2 = pts.clone();
                        sh.geom = Geom::Polygon(pts2, true);
                        // Render the closed polygon then bake it.
                        let s = self.s.clone();
                        self.doc.render_overlay(&s, &mut self.fm);
                        self.doc.commit_overlay(&s);
                    } else {
                        pts.push(p);
                        self.drag = Some(Drag::PolygonPoint);
                    }
                    return true;
                }
                Geom::Line(a, b) => {
                    for (i, e) in [*a, *b].iter().enumerate() {
                        if (to_scr(*e) - p_scr).length() <= HANDLE + 1.0 {
                            self.drag = Some(Drag::LineEnd { which: i as u8 });
                            return true;
                        }
                    }
                    // Grab the line itself to move it.
                    let seg = *b - *a;
                    let t = ((p - *a).dot(seg) / seg.length_sq().max(1e-6)).clamp(0.0, 1.0);
                    let d = (*a + seg * t - p).length() * z;
                    if d < 6.0 + self.s.shape_size * z / 2.0 {
                        let start_rect = sh.geom.bounds();
                        self.drag = Some(Drag::Box { hit: Hit::Inside, start_ptr: p, start_rect, start_geom: Some(sh.geom.clone()) });
                        return true;
                    }
                }
                Geom::Boxed(r) => {
                    let sr = Rect::from_min_max(to_scr(r.min), to_scr(r.max));
                    if let Some(hit) = hit_box(sr, p_scr, true) {
                        self.drag = Some(Drag::Box { hit, start_ptr: p, start_rect: *r, start_geom: None });
                        return true;
                    }
                }
                _ => {}
            },
            Overlay::Text(tb) => {
                let sr = Rect::from_min_max(to_scr(tb.rect.min), to_scr(tb.rect.max));
                if let Some(Hit::Handle(h)) = hit_box(sr, p_scr, true) {
                    self.drag = Some(Drag::Box { hit: Hit::Handle(h), start_ptr: p, start_rect: tb.rect, start_geom: None });
                    return true;
                }
                // Border band moves the box.
                if sr.expand(4.0).contains(p_scr) && !sr.shrink(4.0).contains(p_scr) {
                    self.drag = Some(Drag::Box { hit: Hit::Inside, start_ptr: p, start_rect: tb.rect, start_geom: None });
                    return true;
                }
                if sr.contains(p_scr) {
                    // Place the caret.
                    if let Some(font) = self.fm.font(&self.s.text.family, self.s.text.bold, self.s.text.italic) {
                        let lay = crate::text::layout(&font, self.s.text.px(), &tb.text, tb.rect.min, tb.rect.width(), self.s.text.align);
                        let best = lay
                            .caret
                            .iter()
                            .enumerate()
                            .min_by(|(_, a), (_, b)| {
                                let da = (a.y + lay.line_height / 2.0 - p.y).abs() * 1000.0 + (a.x - p.x).abs();
                                let db = (b.y + lay.line_height / 2.0 - p.y).abs() * 1000.0 + (b.x - p.x).abs();
                                da.total_cmp(&db)
                            })
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                        tb.caret = best;
                    }
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    fn stroke_to(&mut self, a: Pos2, b: Pos2, secondary: bool) {
        let color = self.stroke_color(secondary);
        let size = self.s.size();
        let r = match self.s.tool {
            // Right-dragging the eraser only replaces colour 1 with colour 2.
            Tool::Eraser if secondary => {
                brush::color_erase(&mut self.doc.layer, &self.doc.image, a, b, size, self.s.color1, self.s.color2)
            }
            Tool::Pencil | Tool::Eraser => brush::hard_line(&mut self.doc.layer, a, b, size, color),
            _ => {
                let ctx = self.stroke_ctx.get_or_insert_with(|| StrokeCtx::new(size));
                brush::brush_segment(&mut self.doc.layer, self.s.brush, a, b, size, color, ctx)
            }
        };
        self.doc.layer_touched(r);
    }

    fn drag_move(&mut self, drag: Drag, p: Pos2, p_scr: Pos2, shift: bool) {
        match drag {
            Drag::Stroke { secondary, last } => {
                if p != last {
                    self.stroke_to(last, p, secondary);
                    self.drag = Some(Drag::Stroke { secondary, last: p });
                } else if self.s.tool == Tool::Brush && self.s.brush == BrushKind::Airbrush {
                    let color = self.stroke_color(secondary);
                    let size = self.s.size();
                    let ctx = self.stroke_ctx.get_or_insert_with(|| StrokeCtx::new(size));
                    let r = brush::spray(&mut self.doc.layer, p, size, color, ctx);
                    self.doc.layer_touched(r);
                }
            }
            Drag::ShapeNew { start } => {
                if let Overlay::Shape(sh) = &mut self.doc.overlay {
                    match &mut sh.geom {
                        Geom::Line(_, b) => *b = if shift { snap45(start, p) } else { p },
                        Geom::Curve(_, c1, c2, b) => {
                            let e = if shift { snap45(start, p) } else { p };
                            *b = e;
                            *c1 = start;
                            *c2 = e;
                        }
                        Geom::Polygon(pts, _) => {
                            *pts.last_mut().unwrap() = if shift { snap45(start, p) } else { p };
                        }
                        Geom::Boxed(r) => {
                            let e = if shift { square_from(start, p) } else { p };
                            *r = Rect::from_two_pos(start, e);
                        }
                    }
                }
            }
            Drag::CurveBend => {
                if let Overlay::Shape(sh) = &mut self.doc.overlay
                    && let Geom::Curve(_, c1, c2, _) = &mut sh.geom
                {
                    if sh.stage == 0 {
                        *c1 = p;
                        *c2 = p;
                    } else {
                        *c2 = p;
                    }
                }
            }
            Drag::PolygonPoint => {
                if let Overlay::Shape(sh) = &mut self.doc.overlay
                    && let Geom::Polygon(pts, _) = &mut sh.geom
                {
                    let prev = pts[pts.len() - 2];
                    *pts.last_mut().unwrap() = if shift { snap45(prev, p) } else { p };
                }
            }
            Drag::Box { hit, start_ptr, start_rect, start_geom } => {
                let d = p - start_ptr;
                let mut nr = apply_hit(start_rect, hit, d);
                if hit == Hit::Inside {
                    nr = Rect::from_min_size(nr.min.round(), nr.size());
                }
                // Shift on a corner handle keeps the aspect ratio.
                if shift && let Hit::Handle(h @ (0 | 2 | 4 | 6)) = hit {
                    nr = keep_aspect(start_rect, nr, h);
                }
                let is_float = matches!(self.doc.overlay, Overlay::Float(_));
                if is_float && d != Vec2::ZERO {
                    if shift && hit == Hit::Inside {
                        let s = self.s.clone();
                        self.doc.stamp_float(&s, false);
                    }
                    self.doc.lift(&self.s);
                }
                match &mut self.doc.overlay {
                    Overlay::Float(f) => {
                        if f.rect != nr {
                            f.rect = nr;
                            f.changed = true;
                        }
                    }
                    Overlay::Shape(sh) => match (&mut sh.geom, start_geom) {
                        (Geom::Boxed(r), _) => *r = nr,
                        (g, Some(sg)) => {
                            let from = sg.bounds();
                            *g = match sg {
                                Geom::Line(a, b) => Geom::Line(remap(a, from, nr), remap(b, from, nr)),
                                Geom::Curve(a, b, c, e) => {
                                    Geom::Curve(remap(a, from, nr), remap(b, from, nr), remap(c, from, nr), remap(e, from, nr))
                                }
                                Geom::Polygon(pts, c) => Geom::Polygon(pts.iter().map(|q| remap(*q, from, nr)).collect(), c),
                                other => other,
                            };
                        }
                        _ => {}
                    },
                    Overlay::Text(tb) => {
                        tb.rect = nr;
                        if tb.rect.width() < 20.0 {
                            tb.rect.max.x = tb.rect.min.x + 20.0;
                        }
                    }
                    _ => {}
                }
            }
            Drag::Rotate { center, start, angle } => {
                let mut a = (p - center).angle().to_degrees() - start;
                if shift {
                    a = (a / 15.0).round() * 15.0;
                }
                if (a - angle).abs() >= 0.5
                    && let Some(base) = &self.rotate_base
                {
                    let rotated = pixels::rotate_any(base, a);
                    if let Some(f) = self.doc.floating_mut() {
                        let size = vec2(rotated.width() as f32, rotated.height() as f32);
                        f.rect = Rect::from_min_size((center - size / 2.0).round(), size);
                        f.pix = rotated;
                        f.tex_dirty = true;
                        f.changed = true;
                        f.outline = None;
                    }
                    self.drag = Some(Drag::Rotate { center, start, angle: a });
                }
            }
            Drag::LineEnd { which } => {
                if let Overlay::Shape(sh) = &mut self.doc.overlay
                    && let Geom::Line(a, b) = &mut sh.geom
                {
                    if which == 0 {
                        *a = if shift { snap45(*b, p) } else { p };
                    } else {
                        *b = if shift { snap45(*a, p) } else { p };
                    }
                }
            }
            Drag::Marquee { start, .. } => {
                let mut cur = self.clamp_img(p).round();
                if shift {
                    cur = self.clamp_img(square_from(start, cur));
                }
                self.drag = Some(Drag::Marquee { start, cur });
            }
            Drag::Lasso { mut pts } => {
                let q = self.clamp_img(p);
                if pts.last().is_none_or(|l| (*l - q).length() >= 1.0) {
                    pts.push(q);
                }
                self.drag = Some(Drag::Lasso { pts });
            }
            Drag::TextNew { start, .. } => self.drag = Some(Drag::TextNew { start, cur: p }),
            Drag::CanvasResize { handle, .. } => {
                let w = if handle == 1 { self.doc.width() as f32 } else { p.x.round().max(1.0) };
                let h = if handle == 0 { self.doc.height() as f32 } else { p.y.round().max(1.0) };
                self.drag = Some(Drag::CanvasResize { handle, size: vec2(w, h) });
            }
            Drag::Pan { last } => {
                let d = p_scr - last;
                self.pending_scroll = Some(self.scroll_offset - d);
                self.drag = Some(Drag::Pan { last: p_scr });
            }
        }
    }

    fn drag_end(&mut self) {
        let Some(drag) = self.drag.take() else { return };
        match drag {
            Drag::Stroke { .. } => {
                self.doc.end_stroke();
                self.stroke_ctx = None;
            }
            Drag::ShapeNew { .. } => {
                let mut cancel = false;
                if let Overlay::Shape(sh) = &self.doc.overlay {
                    let b = sh.geom.bounds();
                    if sh.kind.is_boxed() && (b.width() < 1.0 || b.height() < 1.0) {
                        cancel = true;
                    }
                    if sh.kind == ShapeKind::Line && b.width() < 0.5 && b.height() < 0.5 {
                        cancel = true;
                    }
                }
                if cancel {
                    self.doc.cancel_overlay();
                }
            }
            Drag::CurveBend => {
                let mut done = false;
                if let Overlay::Shape(sh) = &mut self.doc.overlay {
                    sh.stage += 1;
                    done = sh.stage >= 2;
                }
                if done {
                    let s = self.s.clone();
                    self.doc.render_overlay(&s, &mut self.fm);
                    self.doc.commit_overlay(&s);
                }
            }
            Drag::Marquee { start, cur } => {
                let r = IRect::from_points((start.x, start.y), (cur.x, cur.y), 0.0);
                let r = IRect::new(r.x0, r.y0, r.x1 - 1, r.y1 - 1);
                if r.width() >= 1 && r.height() >= 1 {
                    self.doc.select_rect(&self.s, r);
                }
            }
            Drag::Lasso { pts } => self.doc.select_freeform(&self.s, &pts),
            Drag::TextNew { start, cur } => {
                let mut r = Rect::from_two_pos(start, cur);
                if r.width() < 16.0 || r.height() < 10.0 {
                    let lh = self.s.text.px() * 1.4 + 6.0;
                    r = Rect::from_min_size(start, vec2(240.0_f32.max(self.s.text.px() * 8.0), lh));
                }
                self.doc.overlay = Overlay::Text(TextBox { rect: r, text: String::new(), caret: 0 });
            }
            Drag::CanvasResize { size, .. } => {
                let s = self.s.clone();
                self.doc.resize_canvas(&s, size.x as u32, size.y as u32);
            }
            Drag::Rotate { .. } => self.rotate_base = None,
            Drag::PolygonPoint | Drag::Box { .. } | Drag::LineEnd { .. } | Drag::Pan { .. } => {}
        }
    }

    /// The tool's own cursor at image point `c`.
    fn tool_cursor(&self, c: Pos2, to_scr: impl Fn(Pos2) -> Pos2) -> CanvasCursor {
        let z = self.zoom;
        match self.s.tool {
            Tool::Pencil | Tool::Brush => CanvasCursor::Circle { diameter: self.s.size() * z },
            Tool::Eraser => {
                // The eraser stamps whole pixels: show exactly the square it will erase.
                let s = self.s.size().round().max(1.0);
                let tl = pos2(c.x.floor() - ((s - 1.0) / 2.0).floor(), c.y.floor() - ((s - 1.0) / 2.0).floor());
                CanvasCursor::Square { rect: Rect::from_min_size(to_scr(tl), vec2(s, s) * z) }
            }
            Tool::Fill => CanvasCursor::Icon { icon: Icon::Fill, hotspot: FILL_HOTSPOT },
            Tool::Picker => CanvasCursor::Icon { icon: Icon::Picker, hotspot: PICKER_HOTSPOT },
            Tool::Text => CanvasCursor::Os(CursorIcon::Text),
            Tool::Magnifier => CanvasCursor::Os(CursorIcon::ZoomIn),
            Tool::Select | Tool::Shape => CanvasCursor::Os(CursorIcon::Crosshair),
        }
    }

    /// Cursor while a mouse button is held.
    fn drag_cursor(&self, drag: &Drag, c: Pos2, to_scr: impl Fn(Pos2) -> Pos2) -> CanvasCursor {
        match drag {
            Drag::Stroke { .. } => self.tool_cursor(c, to_scr),
            Drag::Box { hit, .. } => CanvasCursor::Os(handle_cursor(*hit)),
            Drag::CanvasResize { handle, .. } => CanvasCursor::Os(canvas_handle_cursor(*handle)),
            Drag::Pan { .. } | Drag::Rotate { .. } => CanvasCursor::Os(CursorIcon::Grabbing),
            Drag::ShapeNew { .. }
            | Drag::CurveBend
            | Drag::PolygonPoint
            | Drag::LineEnd { .. }
            | Drag::Marquee { .. }
            | Drag::Lasso { .. }
            | Drag::TextNew { .. } => CanvasCursor::Os(CursorIcon::Crosshair),
        }
    }

    /// Cursor while hovering: handles first, then the tool's cursor.
    fn hover_cursor(&self, p_scr: Pos2, c: Pos2, img: Rect, to_scr: impl Fn(Pos2) -> Pos2) -> CanvasCursor {
        let os = CanvasCursor::Os;
        if let Some(h) = self.canvas_handle_hit(p_scr, img) {
            return os(canvas_handle_cursor(h));
        }
        match &self.doc.overlay {
            Overlay::Float(f) => {
                let r = Rect::from_min_max(to_scr(f.rect.min), to_scr(f.rect.max));
                if (rotate_handle(r) - p_scr).length() <= 9.0 {
                    return os(CursorIcon::Grab);
                }
                if let Some(h) = hit_box(r, p_scr, true) {
                    return os(handle_cursor(h));
                }
            }
            Overlay::Shape(sh) => {
                if let Geom::Boxed(r) = &sh.geom {
                    let r = Rect::from_min_max(to_scr(r.min), to_scr(r.max));
                    if let Some(h) = hit_box(r, p_scr, true) {
                        return os(handle_cursor(h));
                    }
                }
            }
            Overlay::Text(tb) => {
                let r = Rect::from_min_max(to_scr(tb.rect.min), to_scr(tb.rect.max));
                match hit_box(r, p_scr, true) {
                    Some(Hit::Handle(h)) => return os(handle_cursor(Hit::Handle(h))),
                    _ if r.expand(4.0).contains(p_scr) && !r.shrink(4.0).contains(p_scr) => return os(CursorIcon::Move),
                    _ if r.contains(p_scr) => return os(CursorIcon::Text),
                    _ => {}
                }
            }
            _ => {}
        }
        self.tool_cursor(c, to_scr)
    }

    // -----------------------------------------------------------------------
    // Painting

    fn canvas_paint(&mut self, ui: &Ui, img: Rect) {
        let pal = self.pal;
        let p = ui.painter();
        let z = self.zoom;
        let to_scr = |q: Pos2| img.min + q.to_vec2() * z;

        // Drop shadow.
        for i in 1..=4 {
            let f = i as f32;
            p.rect_filled(img.expand(f).translate(vec2(0.0, f * 0.5)), 2.0 + f, pal.canvas_shadow.gamma_multiply(0.25));
        }
        // Transparent pixels show a checkerboard, as in Paint.
        self.paint_checker(p, img);
        if let Some(t) = &self.tex {
            p.image(t.id(), img, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }

        if self.show_grid {
            self.paint_grid(p, img);
        }

        // Floating selection.
        if let Some(f) = self.doc.floating() {
            let r = Rect::from_min_max(to_scr(f.rect.min), to_scr(f.rect.max));
            if let Some(t) = &self.float_tex {
                p.image(t.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            if let Some(outline) = &f.outline {
                let pts: Vec<Pos2> = outline
                    .iter()
                    .map(|q| pos2(r.min.x + q.x * r.width(), r.min.y + q.y * r.height()))
                    .chain(outline.first().map(|q| pos2(r.min.x + q.x * r.width(), r.min.y + q.y * r.height())))
                    .collect();
                p.extend(Shape::dashed_line(&pts, Stroke::new(1.0, pal.accent), 4.0, 4.0));
            }
            dashed_rect(p, r, pal.accent);
            draw_handles(p, r, pal.accent);
            let rh = rotate_handle(r);
            p.line_segment([pos2(r.center().x, r.min.y), rh], Stroke::new(1.0, pal.accent));
            p.circle_filled(rh, 6.0, Color32::WHITE);
            p.circle_stroke(rh, 6.0, Stroke::new(1.5, pal.accent));
            if let Some(Drag::Rotate { angle, .. }) = &self.drag {
                p.text(
                    rh + vec2(12.0, -10.0),
                    egui::Align2::LEFT_CENTER,
                    format!("{:.0}°", angle),
                    egui::FontId::proportional(12.0),
                    pal.text,
                );
            }
        }

        // Pending shape.
        if let Overlay::Shape(sh) = &self.doc.overlay
            && self.drag.as_ref().is_none_or(|d| !matches!(d, Drag::ShapeNew { .. }))
        {
            match &sh.geom {
                Geom::Boxed(r) => {
                    let r = Rect::from_min_max(to_scr(r.min), to_scr(r.max));
                    dashed_rect(p, r, pal.accent);
                    draw_handles(p, r, pal.accent);
                }
                Geom::Line(a, b) => {
                    for e in [a, b] {
                        p.circle_filled(to_scr(*e), 4.5, Color32::WHITE);
                        p.circle_stroke(to_scr(*e), 4.5, Stroke::new(1.0, pal.accent));
                    }
                }
                Geom::Polygon(pts, false) => {
                    let f = to_scr(pts[0]);
                    p.circle_stroke(f, 6.0, Stroke::new(1.0, pal.accent));
                }
                _ => {}
            }
        }
        if let Some(Drag::ShapeNew { .. }) = &self.drag
            && let Overlay::Shape(sh) = &self.doc.overlay
            && let Geom::Boxed(r) = &sh.geom
        {
            dashed_rect(p, Rect::from_min_max(to_scr(r.min), to_scr(r.max)), pal.accent);
        }

        // Text box.
        if let Overlay::Text(tb) = &self.doc.overlay {
            let r = Rect::from_min_max(to_scr(tb.rect.min), to_scr(tb.rect.max));
            dashed_rect(p, r, pal.accent);
            draw_handles(p, r, pal.accent);
            let blink = (ui.input(|i| i.time) * 2.0) as i64 % 2 == 0;
            if blink && let Some(font) = self.fm.font(&self.s.text.family, self.s.text.bold, self.s.text.italic) {
                let lay = crate::text::layout(&font, self.s.text.px(), &tb.text, tb.rect.min, tb.rect.width(), self.s.text.align);
                if let Some(c) = lay.caret.get(tb.caret) {
                    let a = to_scr(*c);
                    let b = to_scr(*c + vec2(0.0, lay.line_height));
                    p.line_segment([a, b], Stroke::new(1.5, self.s.color1));
                }
            }
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
        }

        // In-progress selection gestures.
        match &self.drag {
            Some(Drag::Marquee { start, cur }) => dashed_rect(p, Rect::from_two_pos(to_scr(*start), to_scr(*cur)), pal.accent),
            Some(Drag::Lasso { pts }) => {
                let s: Vec<Pos2> = pts.iter().map(|q| to_scr(*q)).collect();
                p.extend(Shape::dashed_line(&s, Stroke::new(1.0, pal.accent), 4.0, 4.0));
            }
            Some(Drag::TextNew { start, cur }) => dashed_rect(p, Rect::from_two_pos(to_scr(*start), to_scr(*cur)), pal.accent),
            Some(Drag::CanvasResize { size, .. }) => {
                let r = Rect::from_min_size(img.min, *size * z);
                dashed_rect(p, r, pal.accent);
            }
            _ => {}
        }

        // Canvas resize handles.
        for h in self.canvas_handles(img) {
            let hr = Rect::from_center_size(h, vec2(HANDLE, HANDLE));
            p.rect_filled(hr, 1.5, if pal.dark { Color32::from_gray(60) } else { Color32::WHITE });
            p.rect_stroke(hr, 1.5, Stroke::new(1.0, pal.text_dim), StrokeKind::Inside);
        }
    }

    fn paint_grid(&self, p: &egui::Painter, img: Rect) {
        let z = self.zoom;
        let step = [1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0]
            .into_iter()
            .find(|s| s * z >= 8.0)
            .unwrap_or(500.0);
        let vis = img.intersect(p.clip_rect());
        if !vis.is_positive() {
            return;
        }
        let col = Color32::from_rgba_unmultiplied(128, 128, 128, 90);
        let x0 = ((vis.min.x - img.min.x) / z / step).floor() * step;
        let mut x = x0;
        while img.min.x + x * z <= vis.max.x {
            let sx = img.min.x + x * z;
            p.line_segment([pos2(sx, vis.min.y), pos2(sx, vis.max.y)], Stroke::new(1.0, col));
            x += step;
        }
        let y0 = ((vis.min.y - img.min.y) / z / step).floor() * step;
        let mut y = y0;
        while img.min.y + y * z <= vis.max.y {
            let sy = img.min.y + y * z;
            p.line_segment([pos2(vis.min.x, sy), pos2(vis.max.x, sy)], Stroke::new(1.0, col));
            y += step;
        }
    }

    fn paint_rulers(&self, ui: &Ui, full: Rect, size: f32) {
        let pal = self.pal;
        let p = ui.painter_at(full);
        let top = Rect::from_min_max(full.min, pos2(full.max.x, full.min.y + size));
        let left = Rect::from_min_max(full.min, pos2(full.min.x + size, full.max.y));
        let bg = if pal.dark { Color32::from_gray(36) } else { Color32::WHITE };
        p.rect_filled(top, 0.0, bg);
        p.rect_filled(left, 0.0, bg);
        let z = self.zoom;
        let img = self.image_rect;
        let major = [1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0]
            .into_iter()
            .find(|s| s * z >= 60.0)
            .unwrap_or(5000.0);
        let minor = major / 5.0;
        let font = egui::FontId::proportional(9.0);
        let tick = Stroke::new(1.0, pal.text_dim);
        // Horizontal ruler.
        let mut v = ((top.min.x + size - img.min.x) / z / minor).floor() * minor;
        while img.min.x + v * z < top.max.x {
            let x = img.min.x + v * z;
            if x >= top.min.x + size {
                let is_major = (v / major).round() * major == v;
                let len = if is_major { size * 0.6 } else { size * 0.25 };
                p.line_segment([pos2(x, top.max.y - len), pos2(x, top.max.y)], tick);
                if is_major {
                    p.text(pos2(x + 2.0, top.min.y + 1.0), egui::Align2::LEFT_TOP, format!("{}", v as i64), font.clone(), pal.text_dim);
                }
            }
            v += minor;
        }
        // Vertical ruler.
        let mut v = ((left.min.y + size - img.min.y) / z / minor).floor() * minor;
        while img.min.y + v * z < left.max.y {
            let y = img.min.y + v * z;
            if y >= left.min.y + size {
                let is_major = (v / major).round() * major == v;
                let len = if is_major { size * 0.6 } else { size * 0.25 };
                p.line_segment([pos2(left.max.x - len, y), pos2(left.max.x, y)], tick);
                if is_major {
                    p.text(pos2(left.min.x + 1.0, y + 1.0), egui::Align2::LEFT_TOP, format!("{}", v as i64), font.clone(), pal.text_dim);
                }
            }
            v += minor;
        }
        // Pointer position markers.
        if let Some(c) = self.cursor_img {
            let x = img.min.x + c.x * z;
            let y = img.min.y + c.y * z;
            p.line_segment([pos2(x, top.min.y), pos2(x, top.max.y)], Stroke::new(1.0, pal.accent));
            p.line_segment([pos2(left.min.x, y), pos2(left.max.x, y)], Stroke::new(1.0, pal.accent));
        }
        p.rect_filled(Rect::from_min_size(full.min, vec2(size, size)), 0.0, bg);
        p.line_segment([top.left_bottom(), top.right_bottom()], Stroke::new(1.0, pal.separator));
        p.line_segment([left.right_top(), left.right_bottom()], Stroke::new(1.0, pal.separator));
    }
}
