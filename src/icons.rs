//! Vector icons in the Windows 11 Fluent line style, drawn with the egui painter on a
//! 24x24 design grid.

use crate::shapes;
use crate::tools::{BrushKind, ShapeKind};
use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, pos2, vec2};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Icon {
    SelectRect,
    SelectFree,
    Crop,
    Resize,
    Rotate,
    Pencil,
    Fill,
    Text,
    Eraser,
    Picker,
    Magnifier,
    Brush(BrushKind),
    Shape(ShapeKind),
    Outline,
    FillStyle,
    Size,
    Undo,
    Redo,
    Save,
    Gear,
    Chevron,
    Cursor,
    SelectionSize,
    CanvasSize,
    Disk,
    ZoomOut,
    ZoomIn,
    RotateRight,
    RotateLeft,
    Rotate180,
    FlipV,
    FlipH,
    SelectAll,
    Invert,
    Delete,
    Transparent,
    Paste,
    Cut,
    Copy,
    Minimize,
    Maximize,
    Restore,
    Close,
    AlignLeft,
    AlignCenter,
    AlignRight,
    Layers,
    Eye,
    EyeOff,
    Plus,
    Flip,
    RemoveBg,
    Palette,
    Picture,
    /// Nothing (keeps menu labels aligned).
    Blank,
    New,
    Open,
    Recent,
    Import,
    Print,
    Desktop,
    Info,
    Exit,
    FullScreen,
    Ruler,
    Grid,
    StatusBar,
    Duplicate,
    MergeDown,
    MoveUp,
    MoveDown,
}

fn g(r: Rect, x: f32, y: f32) -> Pos2 {
    pos2(r.min.x + x / 24.0 * r.width(), r.min.y + y / 24.0 * r.height())
}

fn pl(p: &Painter, r: Rect, pts: &[(f32, f32)], s: Stroke) {
    p.add(Shape::line(pts.iter().map(|&(x, y)| g(r, x, y)).collect(), s));
}

fn closed(p: &Painter, r: Rect, pts: &[(f32, f32)], s: Stroke) {
    p.add(Shape::closed_line(pts.iter().map(|&(x, y)| g(r, x, y)).collect(), s));
}

fn filled(p: &Painter, r: Rect, pts: &[(f32, f32)], c: Color32) {
    p.add(Shape::convex_polygon(pts.iter().map(|&(x, y)| g(r, x, y)).collect(), c, Stroke::NONE));
}

fn dashed_rect(p: &Painter, r: Rect, x0: f32, y0: f32, x1: f32, y1: f32, s: Stroke) {
    let pts = vec![g(r, x0, y0), g(r, x1, y0), g(r, x1, y1), g(r, x0, y1), g(r, x0, y0)];
    p.extend(Shape::dashed_line(&pts, s, 2.5 * r.width() / 24.0, 2.0 * r.width() / 24.0));
}

/// Circular arc around (cx, cy) on the design grid, angles in degrees.
#[allow(clippy::too_many_arguments)]
fn arc(p: &Painter, r: Rect, cx: f32, cy: f32, rad: f32, a0: f32, a1: f32, s: Stroke) {
    let n = 20;
    let pts: Vec<Pos2> = (0..=n)
        .map(|i| {
            let a = (a0 + (a1 - a0) * i as f32 / n as f32).to_radians();
            g(r, cx + rad * a.cos(), cy + rad * a.sin())
        })
        .collect();
    p.add(Shape::line(pts, s));
}

/// Paint `icon` inside `r` with foreground `fg` and accent color `accent`.
pub fn paint(p: &Painter, r: Rect, icon: Icon, fg: Color32, accent: Color32) {
    let w = (r.width() / 24.0 * 1.5).max(1.0);
    let s = Stroke::new(w, fg);
    let sa = Stroke::new(w, accent);
    match icon {
        Icon::SelectRect => dashed_rect(p, r, 3.0, 4.0, 21.0, 20.0, s),
        Icon::SelectFree => {
            let pts: Vec<Pos2> = (0..=28)
                .map(|i| {
                    let a = i as f32 / 28.0 * std::f32::consts::TAU;
                    let k = 1.0 + 0.18 * (3.0 * a).sin();
                    g(r, 12.0 + 8.5 * k * a.cos(), 12.0 + 6.5 * k * a.sin())
                })
                .collect();
            p.extend(Shape::dashed_line(&pts, s, 2.5 * r.width() / 24.0, 2.0 * r.width() / 24.0));
        }
        Icon::SelectAll => {
            dashed_rect(p, r, 3.0, 3.0, 21.0, 21.0, s);
            p.rect_filled(Rect::from_min_max(g(r, 7.0, 7.0), g(r, 17.0, 17.0)), 1.0, accent.gamma_multiply(0.6));
        }
        Icon::Invert => {
            dashed_rect(p, r, 3.0, 3.0, 21.0, 21.0, s);
            p.rect_filled(Rect::from_min_max(g(r, 3.5, 3.5), g(r, 20.5, 20.5)), 0.0, accent.gamma_multiply(0.4));
            p.rect_filled(Rect::from_min_max(g(r, 8.0, 8.0), g(r, 16.0, 16.0)), 0.0, Color32::from_gray(250));
            p.rect_stroke(Rect::from_min_max(g(r, 8.0, 8.0), g(r, 16.0, 16.0)), 0.0, s, StrokeKind::Middle);
        }
        Icon::Delete => {
            pl(p, r, &[(4.0, 6.0), (20.0, 6.0)], s);
            closed(p, r, &[(6.0, 6.0), (7.0, 21.0), (17.0, 21.0), (18.0, 6.0)], s);
            pl(p, r, &[(9.0, 6.0), (10.0, 3.0), (14.0, 3.0), (15.0, 6.0)], s);
            pl(p, r, &[(10.0, 10.0), (10.0, 17.0)], s);
            pl(p, r, &[(14.0, 10.0), (14.0, 17.0)], s);
        }
        Icon::Transparent => {
            for i in 0..4 {
                for j in 0..4 {
                    if (i + j) % 2 == 0 {
                        let x = 4.0 + i as f32 * 4.0;
                        let y = 4.0 + j as f32 * 4.0;
                        p.rect_filled(Rect::from_min_max(g(r, x, y), g(r, x + 4.0, y + 4.0)), 0.0, fg.gamma_multiply(0.35));
                    }
                }
            }
            dashed_rect(p, r, 4.0, 4.0, 20.0, 20.0, s);
        }
        Icon::Paste => {
            p.rect_stroke(Rect::from_min_max(g(r, 5.0, 4.0), g(r, 19.0, 21.0)), 2.0, s, StrokeKind::Middle);
            p.rect_filled(Rect::from_min_max(g(r, 9.0, 2.5), g(r, 15.0, 6.0)), 1.0, fg);
            pl(p, r, &[(8.5, 11.0), (15.5, 11.0)], s);
            pl(p, r, &[(8.5, 15.0), (13.5, 15.0)], s);
        }
        Icon::Cut => {
            p.circle_stroke(g(r, 7.5, 17.5), r.width() / 24.0 * 3.0, s);
            p.circle_stroke(g(r, 16.5, 17.5), r.width() / 24.0 * 3.0, s);
            pl(p, r, &[(9.3, 15.2), (17.0, 3.0)], s);
            pl(p, r, &[(14.7, 15.2), (7.0, 3.0)], s);
        }
        Icon::Copy => {
            p.rect_stroke(Rect::from_min_max(g(r, 8.0, 7.0), g(r, 19.0, 21.0)), 2.0, s, StrokeKind::Middle);
            pl(p, r, &[(5.0, 17.0), (5.0, 5.0), (7.0, 3.0), (15.0, 3.0)], s);
        }
        // Caption glyphs: 10px and pixel-snapped like Windows (1px lines on .5 coordinates).
        Icon::Minimize => {
            let c = r.center().round();
            p.line_segment([pos2(c.x - 5.0, c.y + 0.5), pos2(c.x + 5.0, c.y + 0.5)], Stroke::new(1.0, fg));
        }
        Icon::Maximize => {
            let c = r.center().round();
            let sq = Rect::from_min_max(pos2(c.x - 4.5, c.y - 4.5), pos2(c.x + 4.5, c.y + 4.5));
            p.rect_stroke(sq, 1.0, Stroke::new(1.0, fg), StrokeKind::Middle);
        }
        Icon::Restore => {
            let c = r.center().round();
            let front = Rect::from_min_max(pos2(c.x - 4.5, c.y - 2.5), pos2(c.x + 2.5, c.y + 4.5));
            p.rect_stroke(front, 1.0, Stroke::new(1.0, fg), StrokeKind::Middle);
            let back = [pos2(c.x - 2.5, c.y - 4.5), pos2(c.x + 4.5, c.y - 4.5), pos2(c.x + 4.5, c.y + 2.5)];
            p.add(Shape::line(back.to_vec(), Stroke::new(1.0, fg)));
        }
        Icon::Close => {
            pl(p, r, &[(6.5, 6.5), (17.5, 17.5)], Stroke::new(1.0, fg));
            pl(p, r, &[(17.5, 6.5), (6.5, 17.5)], Stroke::new(1.0, fg));
        }
        Icon::AlignLeft | Icon::AlignCenter | Icon::AlignRight => {
            for (i, w) in [16.0, 10.0, 16.0, 10.0].iter().enumerate() {
                let y = 6.0 + i as f32 * 4.0;
                let x0 = match icon {
                    Icon::AlignLeft => 4.0,
                    Icon::AlignCenter => 12.0 - w / 2.0,
                    _ => 20.0 - w,
                };
                pl(p, r, &[(x0, y), (x0 + w, y)], s);
            }
        }
        Icon::Layers => {
            closed(p, r, &[(12.0, 3.0), (21.0, 8.0), (12.0, 13.0), (3.0, 8.0)], s);
            pl(p, r, &[(3.0, 12.0), (12.0, 17.0), (21.0, 12.0)], s);
            pl(p, r, &[(3.0, 16.0), (12.0, 21.0), (21.0, 16.0)], s);
        }
        Icon::Eye | Icon::EyeOff => {
            let top: Vec<(f32, f32)> = (0..=12).map(|i| { let t = i as f32 / 12.0; (2.0 + 20.0 * t, 12.0 - 7.0 * (std::f32::consts::PI * t).sin()) }).collect();
            let bot: Vec<(f32, f32)> = (0..=12).map(|i| { let t = i as f32 / 12.0; (2.0 + 20.0 * t, 12.0 + 7.0 * (std::f32::consts::PI * t).sin()) }).collect();
            pl(p, r, &top, s);
            pl(p, r, &bot, s);
            p.circle_stroke(g(r, 12.0, 12.0), r.width() / 24.0 * 3.2, s);
            if icon == Icon::EyeOff {
                pl(p, r, &[(4.0, 20.0), (20.0, 4.0)], s);
            }
        }
        Icon::Plus => {
            pl(p, r, &[(12.0, 5.0), (12.0, 19.0)], s);
            pl(p, r, &[(5.0, 12.0), (19.0, 12.0)], s);
        }
        Icon::Flip => {
            closed(p, r, &[(3.0, 5.0), (3.0, 19.0), (10.0, 19.0)], s);
            filled(p, r, &[(21.0, 5.0), (21.0, 19.0), (14.0, 19.0)], accent);
            p.extend(Shape::dashed_line(&[g(r, 12.0, 2.0), g(r, 12.0, 22.0)], s, 2.0, 2.0));
        }
        Icon::RemoveBg => {
            dashed_rect(p, r, 2.5, 2.5, 21.5, 21.5, s);
            p.circle_filled(g(r, 12.0, 9.0), r.width() / 24.0 * 3.2, accent);
            filled(p, r, &[(6.0, 20.0), (7.5, 15.0), (12.0, 13.5), (16.5, 15.0), (18.0, 20.0)], accent);
        }
        Icon::New => {
            closed(p, r, &[(6.0, 3.0), (14.0, 3.0), (19.0, 8.0), (19.0, 21.0), (6.0, 21.0)], s);
            pl(p, r, &[(14.0, 3.0), (14.0, 8.0), (19.0, 8.0)], s);
        }
        Icon::Open => {
            closed(p, r, &[(3.0, 6.0), (9.0, 6.0), (11.0, 8.0), (20.0, 8.0), (20.0, 19.0), (3.0, 19.0)], s);
            pl(p, r, &[(3.0, 11.0), (20.0, 11.0)], s);
        }
        Icon::Recent => {
            p.circle_stroke(g(r, 12.0, 12.0), r.width() / 24.0 * 8.5, s);
            pl(p, r, &[(12.0, 7.0), (12.0, 12.0), (15.5, 14.0)], s);
        }
        Icon::Import => {
            p.rect_stroke(Rect::from_min_max(g(r, 3.0, 8.0), g(r, 16.0, 20.0)), 2.0, s, StrokeKind::Middle);
            pl(p, r, &[(5.0, 18.0), (8.5, 14.0), (11.0, 16.5), (14.0, 13.5)], s);
            pl(p, r, &[(21.0, 3.0), (14.0, 10.0)], sa);
            pl(p, r, &[(14.0, 5.5), (14.0, 10.0), (18.5, 10.0)], sa);
        }
        Icon::Print => {
            p.rect_stroke(Rect::from_min_max(g(r, 7.0, 3.0), g(r, 17.0, 8.0)), 1.0, s, StrokeKind::Middle);
            p.rect_stroke(Rect::from_min_max(g(r, 3.0, 8.0), g(r, 21.0, 17.0)), 2.0, s, StrokeKind::Middle);
            p.rect_stroke(Rect::from_min_max(g(r, 7.0, 13.0), g(r, 17.0, 21.0)), 1.0, s, StrokeKind::Middle);
        }
        Icon::Desktop => {
            p.rect_stroke(Rect::from_min_max(g(r, 3.0, 4.0), g(r, 21.0, 16.0)), 2.0, s, StrokeKind::Middle);
            pl(p, r, &[(12.0, 16.0), (12.0, 20.0)], s);
            pl(p, r, &[(8.0, 20.0), (16.0, 20.0)], s);
        }
        Icon::Info => {
            p.circle_stroke(g(r, 12.0, 12.0), r.width() / 24.0 * 8.5, s);
            p.circle_filled(g(r, 12.0, 8.0), r.width() / 24.0 * 1.1, fg);
            pl(p, r, &[(12.0, 11.0), (12.0, 17.0)], s);
        }
        Icon::Exit => {
            pl(p, r, &[(13.0, 4.0), (5.0, 4.0), (5.0, 20.0), (13.0, 20.0)], s);
            pl(p, r, &[(10.0, 12.0), (21.0, 12.0)], s);
            pl(p, r, &[(17.5, 8.5), (21.0, 12.0), (17.5, 15.5)], s);
        }
        Icon::FullScreen => {
            for (a, b, c) in [
                ((4.0, 9.0), (4.0, 4.0), (9.0, 4.0)),
                ((15.0, 4.0), (20.0, 4.0), (20.0, 9.0)),
                ((20.0, 15.0), (20.0, 20.0), (15.0, 20.0)),
                ((9.0, 20.0), (4.0, 20.0), (4.0, 15.0)),
            ] {
                pl(p, r, &[a, b, c], s);
            }
        }
        Icon::Ruler => {
            p.rect_stroke(Rect::from_min_max(g(r, 3.0, 8.0), g(r, 21.0, 16.0)), 1.5, s, StrokeKind::Middle);
            for (i, x) in [6.5, 9.5, 12.5, 15.5, 18.5].into_iter().enumerate() {
                pl(p, r, &[(x, 8.0), (x, if i % 2 == 0 { 12.0 } else { 10.5 })], s);
            }
        }
        Icon::Grid => {
            p.rect_stroke(Rect::from_min_max(g(r, 4.0, 4.0), g(r, 20.0, 20.0)), 1.5, s, StrokeKind::Middle);
            for v in [9.33, 14.67] {
                pl(p, r, &[(v, 4.0), (v, 20.0)], s);
                pl(p, r, &[(4.0, v), (20.0, v)], s);
            }
        }
        Icon::StatusBar => {
            p.rect_stroke(Rect::from_min_max(g(r, 3.0, 5.0), g(r, 21.0, 19.0)), 2.0, s, StrokeKind::Middle);
            pl(p, r, &[(3.0, 15.5), (21.0, 15.5)], s);
        }
        Icon::Duplicate => {
            p.rect_stroke(Rect::from_min_max(g(r, 8.0, 8.0), g(r, 20.0, 20.0)), 2.0, s, StrokeKind::Middle);
            pl(p, r, &[(4.0, 15.0), (4.0, 6.0), (6.0, 4.0), (15.0, 4.0)], s);
        }
        Icon::MergeDown => {
            pl(p, r, &[(12.0, 3.0), (12.0, 13.0)], s);
            pl(p, r, &[(8.5, 9.5), (12.0, 13.0), (15.5, 9.5)], s);
            pl(p, r, &[(4.0, 17.0), (20.0, 17.0)], s);
            pl(p, r, &[(4.0, 21.0), (20.0, 21.0)], s);
        }
        Icon::MoveUp => {
            pl(p, r, &[(12.0, 20.0), (12.0, 5.0)], s);
            pl(p, r, &[(6.5, 10.5), (12.0, 5.0), (17.5, 10.5)], s);
        }
        Icon::MoveDown => {
            pl(p, r, &[(12.0, 4.0), (12.0, 19.0)], s);
            pl(p, r, &[(6.5, 13.5), (12.0, 19.0), (17.5, 13.5)], s);
        }
        Icon::Blank => {}
        Icon::Palette => {
            // Painter's palette with a thumb hole and paint dots.
            let pts: Vec<Pos2> = (0..=32)
                .map(|i| {
                    let a = i as f32 / 32.0 * std::f32::consts::TAU;
                    g(r, 12.0 + 9.5 * a.cos(), 12.0 + 8.0 * a.sin())
                })
                .collect();
            p.add(Shape::line(pts, s));
            p.circle_stroke(g(r, 15.5, 16.0), r.width() / 24.0 * 1.8, s);
            for (x, y, c) in [
                (7.5, 11.0, Color32::from_rgb(0xE8, 0x3B, 0x3B)),
                (10.5, 7.5, Color32::from_rgb(0xF5, 0xB4, 0x00)),
                (15.0, 7.5, Color32::from_rgb(0x2E, 0xA0, 0x43)),
                (18.0, 11.0, Color32::from_rgb(0x1E, 0x6F, 0xD9)),
            ] {
                p.circle_filled(g(r, x, y), r.width() / 24.0 * 1.7, c);
            }
        }
        Icon::Picture => {
            p.rect_stroke(Rect::from_min_max(g(r, 3.0, 5.0), g(r, 21.0, 19.0)), 2.0, s, StrokeKind::Middle);
            pl(p, r, &[(5.0, 17.0), (10.0, 11.0), (13.5, 14.5), (16.0, 12.0), (19.5, 16.5)], s);
            p.circle_filled(g(r, 16.0, 8.5), r.width() / 24.0 * 1.6, accent);
        }
        Icon::Crop => {
            pl(p, r, &[(7.0, 2.0), (7.0, 17.0), (22.0, 17.0)], s);
            pl(p, r, &[(2.0, 7.0), (17.0, 7.0), (17.0, 22.0)], s);
        }
        Icon::Resize => {
            p.rect_stroke(Rect::from_min_max(g(r, 3.0, 9.0), g(r, 15.0, 21.0)), 2.0, s, StrokeKind::Middle);
            pl(p, r, &[(11.0, 13.0), (20.0, 4.0)], sa);
            pl(p, r, &[(14.0, 4.0), (20.0, 4.0), (20.0, 10.0)], sa);
        }
        Icon::Rotate | Icon::RotateRight => {
            arc(p, r, 12.0, 13.0, 7.5, -110.0, 160.0, s);
            pl(p, r, &[(18.5, 4.5), (19.0, 9.5), (14.0, 9.0)], s);
        }
        Icon::RotateLeft => {
            arc(p, r, 12.0, 13.0, 7.5, 200.0, 470.0, s);
            pl(p, r, &[(5.5, 4.5), (5.0, 9.5), (10.0, 9.0)], s);
        }
        Icon::Rotate180 => {
            arc(p, r, 12.0, 12.0, 7.5, 180.0, 360.0 + 90.0, s);
            pl(p, r, &[(1.5, 9.0), (4.5, 12.5), (7.5, 9.0)], s);
        }
        Icon::FlipV => {
            closed(p, r, &[(4.0, 3.0), (20.0, 3.0), (12.0, 10.0)], s);
            filled(p, r, &[(4.0, 21.0), (20.0, 21.0), (12.0, 14.0)], accent);
            p.extend(Shape::dashed_line(&[g(r, 2.0, 12.0), g(r, 22.0, 12.0)], s, 2.0, 2.0));
        }
        Icon::FlipH => {
            closed(p, r, &[(3.0, 4.0), (3.0, 20.0), (10.0, 12.0)], s);
            filled(p, r, &[(21.0, 4.0), (21.0, 20.0), (14.0, 12.0)], accent);
            p.extend(Shape::dashed_line(&[g(r, 12.0, 2.0), g(r, 12.0, 22.0)], s, 2.0, 2.0));
        }
        Icon::Pencil => {
            closed(p, r, &[(16.0, 3.5), (20.5, 8.0), (8.0, 20.5), (3.0, 21.0), (3.5, 16.0)], s);
            pl(p, r, &[(13.5, 6.0), (18.0, 10.5)], s);
            filled(p, r, &[(3.0, 21.0), (3.3, 18.0), (6.0, 20.7)], fg);
        }
        Icon::Fill => {
            closed(p, r, &[(10.0, 3.0), (18.0, 11.0), (11.0, 18.0), (3.0, 10.0)], s);
            pl(p, r, &[(4.0, 10.5), (17.0, 10.5)], s);
            filled(p, r, &[(4.0, 11.0), (17.0, 11.0), (11.0, 17.0)], accent);
            let c = g(r, 20.0, 17.5);
            p.circle_filled(c, r.width() / 24.0 * 2.2, accent);
            filled(p, r, &[(20.0, 12.0), (21.9, 16.5), (18.1, 16.5)], accent);
        }
        Icon::Text => {
            pl(p, r, &[(5.0, 20.0), (12.0, 3.5), (19.0, 20.0)], Stroke::new(w * 1.2, fg));
            pl(p, r, &[(8.0, 14.0), (16.0, 14.0)], Stroke::new(w * 1.2, fg));
        }
        Icon::Eraser => {
            closed(p, r, &[(14.0, 3.0), (21.0, 10.0), (11.0, 20.0), (7.0, 20.0), (3.0, 16.0)], s);
            pl(p, r, &[(8.5, 8.5), (15.5, 15.5)], s);
            filled(p, r, &[(14.0, 3.0), (21.0, 10.0), (15.5, 15.5), (8.5, 8.5)], accent.gamma_multiply(0.85));
            pl(p, r, &[(11.0, 20.0), (21.0, 20.0)], s);
        }
        Icon::Picker => {
            closed(p, r, &[(13.0, 7.0), (17.0, 11.0), (7.0, 21.0), (3.0, 21.0), (3.0, 17.0)], s);
            filled(p, r, &[(14.0, 3.5), (20.5, 10.0), (18.0, 12.5), (11.5, 6.0)], fg);
            p.circle_filled(g(r, 18.5, 5.5), r.width() / 24.0 * 3.0, fg);
        }
        Icon::Magnifier | Icon::ZoomIn | Icon::ZoomOut => {
            p.circle_stroke(g(r, 10.0, 10.0), r.width() / 24.0 * 6.5, s);
            pl(p, r, &[(15.0, 15.0), (21.0, 21.0)], Stroke::new(w * 1.4, fg));
            if icon != Icon::Magnifier {
                pl(p, r, &[(7.0, 10.0), (13.0, 10.0)], s);
            }
            if icon == Icon::ZoomIn {
                pl(p, r, &[(10.0, 7.0), (10.0, 13.0)], s);
            }
        }
        Icon::Brush(kind) => brush_icon(p, r, kind, fg, accent, w),
        Icon::Shape(kind) => shape_icon(p, r, kind, s),
        Icon::Outline => {
            p.rect_stroke(Rect::from_min_max(g(r, 4.0, 4.0), g(r, 20.0, 20.0)), 3.0, Stroke::new(w * 1.6, fg), StrokeKind::Middle);
        }
        Icon::FillStyle => {
            p.rect_filled(Rect::from_min_max(g(r, 4.0, 4.0), g(r, 20.0, 20.0)), 3.0, accent);
            p.rect_stroke(Rect::from_min_max(g(r, 4.0, 4.0), g(r, 20.0, 20.0)), 3.0, s, StrokeKind::Middle);
        }
        Icon::Size => {
            for (i, t) in [1.0, 2.0, 3.5, 5.0].iter().enumerate() {
                let y = 4.5 + i as f32 * 5.0;
                pl(p, r, &[(3.0, y), (21.0, y)], Stroke::new(r.width() / 24.0 * t, fg));
            }
        }
        Icon::Undo => {
            arc(p, r, 13.0, 13.0, 6.5, 200.0, 450.0, s);
            pl(p, r, &[(4.0, 7.0), (6.5, 11.0), (10.5, 9.0)], s);
        }
        Icon::Redo => {
            arc(p, r, 11.0, 13.0, 6.5, -20.0, -270.0, s);
            pl(p, r, &[(20.0, 7.0), (17.5, 11.0), (13.5, 9.0)], s);
        }
        Icon::Save => {
            closed(p, r, &[(4.0, 4.0), (17.0, 4.0), (20.0, 7.0), (20.0, 20.0), (4.0, 20.0)], s);
            p.rect_stroke(Rect::from_min_max(g(r, 8.0, 4.0), g(r, 15.0, 9.0)), 0.0, s, StrokeKind::Middle);
            p.rect_stroke(Rect::from_min_max(g(r, 7.0, 13.0), g(r, 17.0, 20.0)), 0.0, s, StrokeKind::Middle);
        }
        Icon::Gear => {
            let c = g(r, 12.0, 12.0);
            let k = r.width() / 24.0;
            let mut pts = Vec::new();
            for i in 0..48 {
                let a = i as f32 / 48.0 * std::f32::consts::TAU;
                let tooth = ((i / 3) % 2 == 0) as i32 as f32;
                let rad = (7.0 + 2.0 * tooth) * k;
                pts.push(c + vec2(a.cos(), a.sin()) * rad);
            }
            p.add(Shape::closed_line(pts, s));
            p.circle_stroke(c, 3.0 * k, s);
        }
        Icon::Chevron => pl(p, r, &[(6.0, 9.0), (12.0, 15.0), (18.0, 9.0)], s),
        Icon::Cursor => {
            closed(p, r, &[(6.0, 3.0), (6.0, 19.0), (10.0, 15.0), (13.0, 21.0), (15.5, 20.0), (12.5, 14.0), (18.0, 14.0)], s);
        }
        Icon::SelectionSize => dashed_rect(p, r, 3.0, 5.0, 21.0, 19.0, s),
        Icon::CanvasSize => {
            p.rect_stroke(Rect::from_min_max(g(r, 3.0, 5.0), g(r, 21.0, 19.0)), 1.5, s, StrokeKind::Middle);
            pl(p, r, &[(16.0, 15.0), (21.0, 19.0)], s);
        }
        Icon::Disk => {
            p.rect_stroke(Rect::from_min_max(g(r, 3.0, 6.0), g(r, 21.0, 18.0)), 2.0, s, StrokeKind::Middle);
            p.circle_filled(g(r, 17.0, 12.0), r.width() / 24.0 * 1.3, fg);
        }
    }
}

fn brush_icon(p: &Painter, r: Rect, kind: BrushKind, fg: Color32, accent: Color32, w: f32) {
    let s = Stroke::new(w, fg);
    match kind {
        BrushKind::Brush => {
            // Handle + bristles.
            pl(p, r, &[(20.0, 3.0), (11.0, 13.0)], Stroke::new(w * 1.6, fg));
            filled(p, r, &[(9.0, 12.0), (12.5, 15.5), (9.5, 19.5), (4.0, 21.0), (5.5, 15.0)], accent);
        }
        BrushKind::CalligraphyLeft | BrushKind::CalligraphyRight => {
            let flip = kind == BrushKind::CalligraphyRight;
            let f = |x: f32| if flip { 24.0 - x } else { x };
            closed(p, r, &[(f(12.0), 2.0), (f(17.0), 9.0), (f(12.0), 21.0), (f(7.0), 9.0)], s);
            pl(p, r, &[(f(12.0), 9.0), (f(12.0), 15.0)], s);
            p.circle_filled(g(r, f(12.0), 9.0), r.width() / 24.0 * 1.3, accent);
        }
        BrushKind::Airbrush => {
            p.rect_stroke(Rect::from_min_max(g(r, 9.0, 9.0), g(r, 15.0, 21.0)), 2.0, s, StrokeKind::Middle);
            pl(p, r, &[(12.0, 9.0), (12.0, 6.0)], s);
            for (x, y) in [(5.0, 3.0), (8.0, 2.0), (11.0, 3.0), (14.0, 2.0), (17.0, 3.0), (7.0, 5.0), (15.0, 5.0), (19.0, 5.0), (4.0, 6.0)] {
                p.circle_filled(g(r, x, y), r.width() / 24.0 * 0.9, accent);
            }
        }
        BrushKind::Oil => {
            pl(p, r, &[(20.0, 3.0), (12.0, 11.0)], Stroke::new(w * 1.6, fg));
            filled(p, r, &[(10.0, 10.0), (14.0, 14.0), (8.0, 21.0), (3.0, 16.0)], accent);
            pl(p, r, &[(6.0, 15.0), (9.0, 18.0)], Stroke::new(w * 0.7, fg));
        }
        BrushKind::Crayon => {
            closed(p, r, &[(17.0, 3.0), (21.0, 7.0), (10.0, 18.0), (6.0, 14.0)], s);
            filled(p, r, &[(6.0, 14.0), (10.0, 18.0), (4.0, 20.0)], accent);
            pl(p, r, &[(14.0, 6.0), (18.0, 10.0)], s);
        }
        BrushKind::Marker => {
            closed(p, r, &[(16.0, 3.0), (21.0, 8.0), (11.0, 18.0), (6.0, 13.0)], s);
            filled(p, r, &[(6.0, 13.0), (11.0, 18.0), (7.0, 21.0), (3.0, 17.0)], accent);
        }
        BrushKind::NaturalPencil => {
            closed(p, r, &[(17.0, 3.0), (21.0, 7.0), (8.0, 20.0), (4.0, 16.0)], s);
            filled(p, r, &[(4.0, 16.0), (8.0, 20.0), (3.0, 21.0)], accent);
        }
        BrushKind::Watercolor => {
            pl(p, r, &[(20.0, 3.0), (12.0, 11.0)], Stroke::new(w * 1.2, fg));
            filled(p, r, &[(12.0, 11.0), (13.0, 12.0), (9.0, 17.0), (7.0, 15.0)], fg);
            p.circle_filled(g(r, 6.5, 18.5), r.width() / 24.0 * 3.5, accent.gamma_multiply(0.55));
        }
    }
}

fn shape_icon(p: &Painter, r: Rect, kind: ShapeKind, s: Stroke) {
    let inner = Rect::from_min_max(g(r, 3.0, 4.0), g(r, 21.0, 20.0));
    match kind {
        ShapeKind::Line => pl(p, r, &[(4.0, 20.0), (20.0, 4.0)], s),
        ShapeKind::Curve => {
            let pts = shapes::curve_points(g(r, 3.0, 19.0), g(r, 8.0, 0.0), g(r, 15.0, 26.0), g(r, 21.0, 5.0));
            p.add(Shape::line(pts, s));
        }
        ShapeKind::Polygon => closed(p, r, &[(4.0, 9.0), (11.0, 4.0), (20.0, 7.0), (17.0, 13.0), (20.0, 20.0), (6.0, 20.0)], s),
        _ => {
            for poly in shapes::polygons(kind, inner) {
                p.add(Shape::closed_line(poly, s));
            }
        }
    }
}
