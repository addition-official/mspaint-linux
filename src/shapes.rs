//! Shape geometry. Boxed shapes are described as closed polygons inside a rectangle,
//! which lets the same code render the canvas shape and the toolbar icon.

use crate::tools::ShapeKind;
use egui::{Pos2, Rect, pos2};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

fn ellipse(c: Pos2, rx: f32, ry: f32, n: usize) -> Vec<Pos2> {
    (0..n)
        .map(|i| {
            let a = i as f32 / n as f32 * TAU;
            pos2(c.x + rx * a.cos(), c.y + ry * a.sin())
        })
        .collect()
}

fn segments_for(r: Rect) -> usize {
    ((r.width() + r.height()).abs() as usize / 2).clamp(24, 360)
}

fn regular(r: Rect, sides: usize, rot: f32) -> Vec<Pos2> {
    let c = r.center();
    (0..sides)
        .map(|i| {
            let a = rot + i as f32 / sides as f32 * TAU;
            pos2(c.x + r.width() / 2.0 * a.cos(), c.y + r.height() / 2.0 * a.sin())
        })
        .collect()
}

fn star(r: Rect, points: usize, inner: f32) -> Vec<Pos2> {
    let c = r.center();
    (0..points * 2)
        .map(|i| {
            let a = -FRAC_PI_2 + i as f32 / (points * 2) as f32 * TAU;
            let k = if i % 2 == 0 { 1.0 } else { inner };
            pos2(c.x + r.width() / 2.0 * k * a.cos(), c.y + r.height() / 2.0 * k * a.sin())
        })
        .collect()
}

/// Map normalized coordinates (0..1) into the rect.
fn norm(r: Rect, pts: &[(f32, f32)]) -> Vec<Pos2> {
    pts.iter()
        .map(|&(x, y)| pos2(r.min.x + x * r.width(), r.min.y + y * r.height()))
        .collect()
}

fn rounded_rect(r: Rect, rad: f32) -> Vec<Pos2> {
    let rad = rad.min(r.width().abs() / 2.0).min(r.height().abs() / 2.0).max(0.0);
    let n = 10;
    let mut out = Vec::new();
    let corners = [
        (pos2(r.max.x - rad, r.min.y + rad), -FRAC_PI_2),
        (pos2(r.max.x - rad, r.max.y - rad), 0.0),
        (pos2(r.min.x + rad, r.max.y - rad), FRAC_PI_2),
        (pos2(r.min.x + rad, r.min.y + rad), PI),
    ];
    for (c, start) in corners {
        for i in 0..=n {
            let a = start + i as f32 / n as f32 * FRAC_PI_2;
            out.push(pos2(c.x + rad * a.cos(), c.y + rad * a.sin()));
        }
    }
    out
}

/// Closed polygons for a boxed shape inside `r`.
pub fn polygons(kind: ShapeKind, r: Rect) -> Vec<Vec<Pos2>> {
    let r = Rect::from_two_pos(r.min, r.max);
    match kind {
        ShapeKind::Oval => vec![ellipse(r.center(), r.width() / 2.0, r.height() / 2.0, segments_for(r))],
        ShapeKind::Rectangle => vec![vec![r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()]],
        ShapeKind::RoundedRectangle => {
            vec![rounded_rect(r, r.width().min(r.height()) * 0.18)]
        }
        ShapeKind::Triangle => vec![norm(r, &[(0.5, 0.0), (1.0, 1.0), (0.0, 1.0)])],
        ShapeKind::RightTriangle => vec![norm(r, &[(0.0, 0.0), (1.0, 1.0), (0.0, 1.0)])],
        ShapeKind::Diamond => vec![norm(r, &[(0.5, 0.0), (1.0, 0.5), (0.5, 1.0), (0.0, 0.5)])],
        ShapeKind::Pentagon => {
            // Fit the regular pentagon to the box.
            let raw = regular(Rect::from_min_max(pos2(-1.0, -1.0), pos2(1.0, 1.0)), 5, -FRAC_PI_2);
            vec![fit(&raw, r)]
        }
        ShapeKind::Hexagon => vec![norm(
            r,
            &[(0.25, 0.0), (0.75, 0.0), (1.0, 0.5), (0.75, 1.0), (0.25, 1.0), (0.0, 0.5)],
        )],
        ShapeKind::RightArrow => vec![norm(
            r,
            &[(0.0, 0.25), (0.55, 0.25), (0.55, 0.0), (1.0, 0.5), (0.55, 1.0), (0.55, 0.75), (0.0, 0.75)],
        )],
        ShapeKind::LeftArrow => vec![norm(
            r,
            &[(1.0, 0.25), (0.45, 0.25), (0.45, 0.0), (0.0, 0.5), (0.45, 1.0), (0.45, 0.75), (1.0, 0.75)],
        )],
        ShapeKind::UpArrow => vec![norm(
            r,
            &[(0.25, 1.0), (0.25, 0.45), (0.0, 0.45), (0.5, 0.0), (1.0, 0.45), (0.75, 0.45), (0.75, 1.0)],
        )],
        ShapeKind::DownArrow => vec![norm(
            r,
            &[(0.25, 0.0), (0.25, 0.55), (0.0, 0.55), (0.5, 1.0), (1.0, 0.55), (0.75, 0.55), (0.75, 0.0)],
        )],
        ShapeKind::Star4 => vec![star(r, 4, 0.38)],
        ShapeKind::Star5 => {
            let raw = star(Rect::from_min_max(pos2(-1.0, -1.0), pos2(1.0, 1.0)), 5, 0.4);
            vec![fit(&raw, r)]
        }
        ShapeKind::Star6 => vec![star(r, 6, 0.55)],
        ShapeKind::RoundedCallout => {
            let body = Rect::from_min_max(r.min, pos2(r.max.x, r.min.y + r.height() * 0.72));
            let mut pts = rounded_rect(body, body.width().min(body.height()) * 0.2);
            // Insert the tail on the bottom edge (points after the 2nd corner arc run right->left).
            let tail_a = pos2(r.min.x + r.width() * 0.32, body.max.y);
            let tail_tip = pos2(r.min.x + r.width() * 0.12, r.max.y);
            let tail_b = pos2(r.min.x + r.width() * 0.2, body.max.y);
            let insert_at = 22; // after bottom-right corner arc (11 pts per corner)
            pts.splice(insert_at..insert_at, [tail_a, tail_tip, tail_b]);
            vec![pts]
        }
        ShapeKind::OvalCallout => {
            let body = Rect::from_min_max(r.min, pos2(r.max.x, r.min.y + r.height() * 0.8));
            let n = segments_for(body);
            let c = body.center();
            let mut pts = Vec::with_capacity(n + 1);
            let tip = pos2(r.min.x + r.width() * 0.15, r.max.y);
            let (a0, a1) = (0.58 * PI, 0.72 * PI);
            let mut tip_done = false;
            for i in 0..n {
                let a = i as f32 / n as f32 * TAU;
                if a > a0 && a < a1 {
                    if !tip_done {
                        pts.push(tip);
                        tip_done = true;
                    }
                    continue;
                }
                pts.push(pos2(c.x + body.width() / 2.0 * a.cos(), c.y + body.height() / 2.0 * a.sin()));
            }
            vec![pts]
        }
        ShapeKind::CloudCallout => {
            let body = Rect::from_min_max(r.min, pos2(r.max.x, r.min.y + r.height() * 0.78));
            let c = body.center();
            let (rx, ry) = (body.width() / 2.0 * 0.86, body.height() / 2.0 * 0.8);
            let bumps = 9;
            let per = 14;
            let mut pts = Vec::new();
            for b in 0..bumps {
                let a0 = b as f32 / bumps as f32 * TAU;
                let a1 = (b + 1) as f32 / bumps as f32 * TAU;
                let p0 = pos2(c.x + rx * a0.cos(), c.y + ry * a0.sin());
                let p1 = pos2(c.x + rx * a1.cos(), c.y + ry * a1.sin());
                let mid = (p0 + p1.to_vec2()) / 2.0;
                let rad = (p1 - p0).length() / 2.0;
                let base = (p1 - p0).angle();
                for i in 0..per {
                    // Outward semicircle between consecutive points.
                    let t = base + PI + i as f32 / per as f32 * PI;
                    pts.push(pos2(mid.x + rad * t.cos(), mid.y + rad * t.sin()));
                }
            }
            let small1 = ellipse(
                pos2(r.min.x + r.width() * 0.24, r.min.y + r.height() * 0.85),
                r.width() * 0.05,
                r.height() * 0.04,
                16,
            );
            let small2 = ellipse(
                pos2(r.min.x + r.width() * 0.12, r.min.y + r.height() * 0.96),
                r.width() * 0.03,
                r.height() * 0.025,
                12,
            );
            vec![pts, small1, small2]
        }
        ShapeKind::Heart => {
            let n = segments_for(r).max(64);
            let raw: Vec<Pos2> = (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32 * TAU;
                    let x = 16.0 * t.sin().powi(3);
                    let y = -(13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos());
                    pos2(x, y)
                })
                .collect();
            vec![fit(&raw, r)]
        }
        ShapeKind::Lightning => vec![norm(
            r,
            &[
                (0.38, 0.0),
                (0.62, 0.3),
                (0.53, 0.35),
                (0.8, 0.6),
                (0.7, 0.65),
                (1.0, 1.0),
                (0.5, 0.72),
                (0.6, 0.67),
                (0.22, 0.45),
                (0.34, 0.39),
                (0.0, 0.17),
            ],
        )],
        ShapeKind::Line | ShapeKind::Curve | ShapeKind::Polygon => {
            vec![vec![r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()]]
        }
    }
}

/// Scale points so their bounding box fills `r`.
fn fit(pts: &[Pos2], r: Rect) -> Vec<Pos2> {
    let b = Rect::from_points(pts);
    let sx = r.width() / b.width().max(1e-6);
    let sy = r.height() / b.height().max(1e-6);
    pts.iter()
        .map(|p| pos2(r.min.x + (p.x - b.min.x) * sx, r.min.y + (p.y - b.min.y) * sy))
        .collect()
}

/// Cubic bezier points for the curve tool, flattened.
pub fn curve_points(p0: Pos2, c1: Pos2, c2: Pos2, p1: Pos2) -> Vec<Pos2> {
    let len = (c1 - p0).length() + (c2 - c1).length() + (p1 - c2).length();
    let n = (len / 2.0).clamp(8.0, 400.0) as usize;
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            let p = p0.to_vec2() * (u * u * u)
                + c1.to_vec2() * (3.0 * u * u * t)
                + c2.to_vec2() * (3.0 * u * t * t)
                + p1.to_vec2() * (t * t * t);
            p.to_pos2()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Rasterization

use crate::pixels::{IRect, noise, paint_for};
use crate::tools::ShapeStyle;
use egui::Color32;
use tiny_skia::{FillRule, LineCap, LineJoin, Path, PathBuilder, Pixmap, PixmapPaint, Stroke, Transform};

#[derive(Clone, Debug)]
pub enum Geom {
    Boxed(Rect),
    Line(Pos2, Pos2),
    Curve(Pos2, Pos2, Pos2, Pos2),
    Polygon(Vec<Pos2>, bool),
}

impl Geom {
    pub fn bounds(&self) -> Rect {
        match self {
            Geom::Boxed(r) => Rect::from_two_pos(r.min, r.max),
            Geom::Line(a, b) => Rect::from_two_pos(*a, *b),
            Geom::Curve(a, b, c, d) => Rect::from_points(&[*a, *b, *c, *d]),
            Geom::Polygon(p, _) => Rect::from_points(p),
        }
    }
}

pub struct ShapeParams {
    pub outline: ShapeStyle,
    pub fill: ShapeStyle,
    pub width: f32,
    pub outline_color: Color32,
    pub fill_color: Color32,
    pub seed: u32,
}

fn path_from(polys: &[Vec<Pos2>], close: bool) -> Option<Path> {
    let mut pb = PathBuilder::new();
    for poly in polys {
        let Some(first) = poly.first() else { continue };
        pb.move_to(first.x, first.y);
        for p in &poly[1..] {
            pb.line_to(p.x, p.y);
        }
        if close {
            pb.close();
        }
    }
    pb.finish()
}

fn apply_style(p: &mut Pixmap, style: ShapeStyle, ox: i32, oy: i32, seed: u32) {
    if style == ShapeStyle::Solid {
        return;
    }
    let w = p.width() as i32;
    for (i, px) in p.data_mut().chunks_exact_mut(4).enumerate() {
        if px[3] == 0 {
            continue;
        }
        let x = ox + i as i32 % w;
        let y = oy + i as i32 / w;
        let k = match style {
            ShapeStyle::Crayon => 1.0 - 0.95 * noise(x, y, seed),
            ShapeStyle::Marker => 0.55,
            ShapeStyle::Oil => 0.6 + 0.4 * noise(x / 3, y, seed),
            ShapeStyle::NaturalPencil => 0.75 * (1.0 - 0.5 * noise(x, y, seed)),
            ShapeStyle::Watercolor => 0.4 * (0.75 + 0.25 * noise(x / 2, y / 2, seed)),
            _ => 1.0,
        };
        for c in px.iter_mut() {
            *c = (*c as f32 * k) as u8;
        }
    }
}

/// Render a shape into `layer`. Returns the dirty rect.
pub fn render(layer: &mut Pixmap, kind: ShapeKind, geom: &Geom, sp: &ShapeParams) -> IRect {
    let b = geom.bounds();
    let pad = sp.width + 4.0;
    let dirty = IRect::from_points((b.min.x, b.min.y), (b.max.x, b.max.y), pad)
        .clamp_to(layer.width(), layer.height());
    if dirty.is_empty() {
        return dirty;
    }
    let (closed_polys, open_line): (Vec<Vec<Pos2>>, Option<Vec<Pos2>>) = match geom {
        Geom::Boxed(r) => (polygons(kind, *r), None),
        Geom::Line(a, c) => (vec![], Some(vec![*a, *c])),
        Geom::Curve(a, c1, c2, d) => (vec![], Some(curve_points(*a, *c1, *c2, *d))),
        Geom::Polygon(pts, true) => (vec![pts.clone()], None),
        Geom::Polygon(pts, false) => (vec![], Some(pts.clone())),
    };
    let t = Transform::from_translate(-dirty.x0 as f32, -dirty.y0 as f32);
    let draw_part = |layer: &mut Pixmap, style: ShapeStyle, color: Color32, f: &dyn Fn(&mut Pixmap)| {
        if style == ShapeStyle::None {
            return;
        }
        let Some(mut tmp) = Pixmap::new(dirty.width() as u32, dirty.height() as u32) else { return };
        let _ = color;
        f(&mut tmp);
        apply_style(&mut tmp, style, dirty.x0, dirty.y0, sp.seed);
        layer.draw_pixmap(dirty.x0, dirty.y0, tmp.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
    };
    let stroke = Stroke {
        width: sp.width.max(1.0),
        line_cap: LineCap::Round,
        line_join: if kind == ShapeKind::Rectangle { LineJoin::Miter } else { LineJoin::Round },
        ..Default::default()
    };
    if !closed_polys.is_empty()
        && let Some(path) = path_from(&closed_polys, true) {
            draw_part(layer, sp.fill, sp.fill_color, &|tmp| {
                tmp.fill_path(&path, &paint_for(sp.fill_color, true), FillRule::Winding, t, None);
            });
            draw_part(layer, sp.outline, sp.outline_color, &|tmp| {
                tmp.stroke_path(&path, &paint_for(sp.outline_color, true), &stroke, t, None);
            });
        }
    if let Some(line) = open_line {
        let line = if line.len() == 2 && line[0] == line[1] {
            vec![line[0], line[1] + egui::vec2(0.01, 0.0)]
        } else {
            line
        };
        if let Some(path) = path_from(&[line], false) {
            // Lines and curves always use the outline colour; "No outline" still draws them solid.
            let style = if sp.outline == ShapeStyle::None { ShapeStyle::Solid } else { sp.outline };
            draw_part(layer, style, sp.outline_color, &|tmp| {
                tmp.stroke_path(&path, &paint_for(sp.outline_color, true), &stroke, t, None);
            });
        }
    }
    dirty
}
