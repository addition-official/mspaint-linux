//! Freehand drawing: pencil, eraser and the artistic brushes. Everything draws into a
//! transparent stroke layer which is later composited over the image.

use crate::pixels::{IRect, Rng, max_pixel, noise, paint_for, set_pixel};
use crate::tools::BrushKind;
use egui::{Color32, Pos2, pos2};
use tiny_skia::{FillRule, LineCap, LineJoin, PathBuilder, Pixmap, Stroke, Transform};

/// Per-stroke state that has to stay stable for the whole stroke.
pub struct StrokeCtx {
    pub rng: Rng,
    pub seed: u32,
    /// Oil brush bristle offsets and strengths.
    pub bristles: Vec<(egui::Vec2, f32)>,
    /// Distance painted so far (oil and watercolour run out of paint).
    pub distance: f32,
}

impl StrokeCtx {
    pub fn new(size: f32) -> Self {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1);
        let mut rng = Rng(t | 1);
        let n = ((size * 0.8) as usize).clamp(4, 24);
        let bristles = (0..n)
            .map(|_| {
                let a = rng.f32() * std::f32::consts::TAU;
                let d = rng.f32().sqrt() * size * 0.5;
                (egui::vec2(a.cos() * d, a.sin() * d), 0.45 + rng.f32() * 0.55)
            })
            .collect();
        let seed = rng.next_u32();
        Self { rng, seed, bristles, distance: 0.0 }
    }
}

/// Bresenham line, calling `f` for each pixel.
pub fn line_pixels(a: Pos2, b: Pos2, mut f: impl FnMut(i32, i32)) {
    let (mut x0, mut y0) = (a.x.floor() as i32, a.y.floor() as i32);
    let (x1, y1) = (b.x.floor() as i32, b.y.floor() as i32);
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        f(x0, y0);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// Hard-edged square stamps along a line (pencil and eraser).
pub fn hard_line(layer: &mut Pixmap, a: Pos2, b: Pos2, size: f32, color: Color32) -> IRect {
    let s = size.max(1.0).round() as i32;
    let off = (s - 1) / 2;
    let c = color.to_array();
    line_pixels(a, b, |x, y| {
        for yy in 0..s {
            for xx in 0..s {
                set_pixel(layer, x - off + xx, y - off + yy, c);
            }
        }
    });
    IRect::from_points((a.x, a.y), (b.x, b.y), s as f32 + 1.0)
}

/// Colour eraser: square stamps that replace pixels of `from` (in `image`) with `to`.
pub fn color_erase(layer: &mut Pixmap, image: &Pixmap, a: Pos2, b: Pos2, size: f32, from: Color32, to: Color32) -> IRect {
    let s = size.max(1.0).round() as i32;
    let off = (s - 1) / 2;
    let (from, to) = (from.to_array(), to.to_array());
    let w = image.width() as i32;
    let h = image.height() as i32;
    line_pixels(a, b, |x, y| {
        for yy in 0..s {
            for xx in 0..s {
                let (px, py) = (x - off + xx, y - off + yy);
                if px < 0 || py < 0 || px >= w || py >= h {
                    continue;
                }
                let i = ((py * w + px) * 4) as usize;
                if image.data()[i..i + 4] == from {
                    set_pixel(layer, px, py, to);
                }
            }
        }
    });
    IRect::from_points((a.x, a.y), (b.x, b.y), s as f32 + 1.0)
}

fn stroke_line(layer: &mut Pixmap, a: Pos2, b: Pos2, width: f32, color: Color32) {
    let mut pb = PathBuilder::new();
    pb.move_to(a.x, a.y);
    if a == b {
        pb.line_to(b.x + 0.01, b.y);
    } else {
        pb.line_to(b.x, b.y);
    }
    if let Some(path) = pb.finish() {
        let stroke = Stroke {
            width: width.max(0.5),
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Default::default()
        };
        layer.stroke_path(&path, &paint_for(color, true), &stroke, Transform::identity(), None);
    }
}

/// Soft round stamp writing max coverage (no build-up within one stroke).
fn soft_stamp(layer: &mut Pixmap, c: Pos2, r: f32, color: Color32, hardness: f32, seed: u32, grain: f32) {
    let r = r.max(0.5);
    let x0 = (c.x - r).floor() as i32;
    let x1 = (c.x + r).ceil() as i32;
    let y0 = (c.y - r).floor() as i32;
    let y1 = (c.y + r).ceil() as i32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let d = ((x as f32 + 0.5 - c.x).powi(2) + (y as f32 + 0.5 - c.y).powi(2)).sqrt() / r;
            if d > 1.0 {
                continue;
            }
            let mut a = if d < hardness { 1.0 } else { 1.0 - (d - hardness) / (1.0 - hardness).max(1e-3) };
            if grain > 0.0 {
                a *= 1.0 - grain * noise(x, y, seed);
            }
            max_pixel(layer, x, y, color, a);
        }
    }
}

/// Draw one segment of a brush stroke. Returns the dirty rect.
pub fn brush_segment(
    layer: &mut Pixmap,
    kind: BrushKind,
    a: Pos2,
    b: Pos2,
    size: f32,
    color: Color32,
    ctx: &mut StrokeCtx,
) -> IRect {
    let pad = size + 4.0;
    let dirty = IRect::from_points((a.x, a.y), (b.x, b.y), pad);
    // Like real paint, the oil and watercolour brushes run dry over a long stroke;
    // start a new stroke to "reload" the brush.
    ctx.distance += (b - a).length();
    let paint_left = (1.0 - ctx.distance / (500.0 + size * 20.0)).clamp(0.0, 1.0);
    match kind {
        BrushKind::Brush => stroke_line(layer, a, b, size, color),
        BrushKind::CalligraphyLeft | BrushKind::CalligraphyRight => {
            let ang: f32 = if kind == BrushKind::CalligraphyLeft { 45f32 } else { 135f32 }.to_radians();
            let n = egui::vec2(ang.cos(), -ang.sin()) * (size.max(2.0) / 2.0);
            let mut pb = PathBuilder::new();
            pb.move_to(a.x + n.x, a.y + n.y);
            pb.line_to(b.x + n.x, b.y + n.y);
            pb.line_to(b.x - n.x, b.y - n.y);
            pb.line_to(a.x - n.x, a.y - n.y);
            pb.close();
            if let Some(path) = pb.finish() {
                let paint = paint_for(color, true);
                layer.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
                let stroke = Stroke { width: 1.0, ..Default::default() };
                layer.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }
        BrushKind::Airbrush => {
            // Movement sprays along the segment; idle spraying is done by `spray`.
            let steps = ((b - a).length() / (size * 0.5).max(1.0)).ceil().max(1.0) as usize;
            for i in 0..=steps {
                let p = a + (b - a) * (i as f32 / steps as f32);
                spray(layer, p, size, color, ctx);
            }
        }
        BrushKind::Oil => {
            let bristles = ctx.bristles.clone();
            for (off, strength) in bristles {
                let c = color.gamma_multiply(strength * (0.25 + 0.75 * paint_left).min(paint_left * 4.0));
                stroke_line(layer, a + off, b + off, (size / 5.0).max(1.0), c);
            }
        }
        BrushKind::Crayon => stamp_along(a, b, size * 0.25, |p| {
            soft_stamp(layer, p, size / 2.0, color, 0.85, ctx.seed, 0.95)
        }),
        BrushKind::Marker => {
            let mut pb = PathBuilder::new();
            pb.move_to(a.x, a.y);
            pb.line_to(b.x + if a == b { 0.01 } else { 0.0 }, b.y);
            if let Some(path) = pb.finish() {
                let stroke = Stroke {
                    width: size.max(1.0),
                    line_cap: LineCap::Square,
                    line_join: LineJoin::Round,
                    ..Default::default()
                };
                layer.stroke_path(&path, &paint_for(color, true), &stroke, Transform::identity(), None);
            }
        }
        BrushKind::NaturalPencil => stamp_along(a, b, 0.5, |p| {
            soft_stamp(layer, p, (size * 0.5).max(0.8), color, 0.4, ctx.seed, 0.6)
        }),
        BrushKind::Watercolor => stamp_along(a, b, size * 0.15, |p| {
            soft_stamp(layer, p, size * 0.75, color.gamma_multiply(paint_left), 0.2, ctx.seed, 0.3)
        }),
    }
    dirty
}

fn stamp_along(a: Pos2, b: Pos2, spacing: f32, mut f: impl FnMut(Pos2)) {
    let len = (b - a).length();
    let steps = (len / spacing.max(0.5)).ceil().max(1.0) as usize;
    for i in 0..=steps {
        f(a + (b - a) * (i as f32 / steps as f32));
    }
}

/// Airbrush particles around `p`.
pub fn spray(layer: &mut Pixmap, p: Pos2, size: f32, color: Color32, ctx: &mut StrokeCtx) -> IRect {
    let r = size.max(2.0) * 1.5;
    let count = (r * 0.9) as usize + 3;
    let c = color.to_array();
    for _ in 0..count {
        let a = ctx.rng.f32() * std::f32::consts::TAU;
        let d = ctx.rng.f32().sqrt() * r;
        let q = pos2(p.x + a.cos() * d, p.y + a.sin() * d);
        set_pixel(layer, q.x.floor() as i32, q.y.floor() as i32, c);
    }
    IRect::from_points((p.x, p.y), (p.x, p.y), r + 2.0)
}
