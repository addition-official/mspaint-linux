//! Canvas cursors.
//!
//! Windows 11 Paint shows the brush footprint as the cursor (a circle for the pencil and
//! brushes, a square for the eraser) and tool-shaped cursors for the fill bucket and the
//! color picker. Desktop cursor themes have no such cursors, so these are drawn here,
//! black with a white halo so they stay visible on any color. Tools that do have a
//! fitting system cursor (crosshair, I-beam, move/resize arrows, magnifier) use it,
//! because system cursors never lag behind the pointer.

use crate::app::PaintApp;
use crate::icons::{self, Icon};
use egui::{Color32, CursorIcon, Painter, Pos2, Rect, Stroke, Vec2, vec2};

/// What the pointer should look like over the canvas this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CanvasCursor {
    Os(CursorIcon),
    /// Brush footprint in screen pixels: circle, or square (eraser) given as a rect.
    Circle { diameter: f32 },
    Square { rect: Rect },
    /// A drawn tool icon; `hotspot` is the point on the 24x24 design grid that sits
    /// under the pointer.
    Icon { icon: Icon, hotspot: Vec2 },
}

/// Footprints smaller than this (in screen pixels) are shown as a crosshair instead.
const MIN_FOOTPRINT: f32 = 4.0;
const ICON_SIZE: f32 = 24.0;

fn halo() -> Color32 {
    Color32::from_white_alpha(230)
}

/// A 1px black line on a 3px white one.
fn dual(p: &Painter, shape: impl Fn(&Painter, Stroke)) {
    shape(p, Stroke::new(3.0, halo()));
    shape(p, Stroke::new(1.0, Color32::BLACK));
}

/// Precision crosshair with a small gap so the exact pixel stays visible.
fn crosshair(p: &Painter, c: Pos2) {
    let (gap, len) = (3.0, 8.0);
    for d in [vec2(1.0, 0.0), vec2(-1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, -1.0)] {
        dual(p, |p, s| {
            p.line_segment([c + d * gap, c + d * (gap + len)], s);
        });
    }
    p.rect_filled(Rect::from_center_size(c, vec2(1.0, 1.0)), 0.0, Color32::BLACK);
}

impl PaintApp {
    /// Apply this frame's canvas cursor: set the system cursor, or hide it and draw ours.
    pub fn apply_canvas_cursor(&self, ctx: &egui::Context, cursor: CanvasCursor, at: Pos2) {
        if let CanvasCursor::Os(icon) = cursor {
            ctx.set_cursor_icon(icon);
            return;
        }
        ctx.set_cursor_icon(CursorIcon::None);
        // Topmost layer, so the cursor is drawn over selections, handles and text boxes.
        let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("paint_cursor")));
        match cursor {
            CanvasCursor::Circle { diameter } if diameter >= MIN_FOOTPRINT => {
                dual(&p, |p, s| {
                    p.circle_stroke(at, diameter / 2.0, s);
                });
            }
            CanvasCursor::Square { rect } if rect.width() >= MIN_FOOTPRINT => {
                dual(&p, |p, s| {
                    p.rect_stroke(rect, 0.0, s, egui::StrokeKind::Middle);
                });
            }
            CanvasCursor::Circle { .. } | CanvasCursor::Square { .. } => crosshair(&p, at),
            CanvasCursor::Icon { icon, hotspot } => {
                let r = Rect::from_min_size(at - hotspot, vec2(ICON_SIZE, ICON_SIZE));
                // White halo: the icon drawn around the outline, then the black icon on top.
                for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0), (-1.0, -1.0), (1.0, 1.0), (-1.0, 1.0), (1.0, -1.0)] {
                    icons::paint(&p, r.translate(vec2(dx, dy)), icon, halo(), halo());
                }
                // The bucket's paint shows the current color 1.
                icons::paint(&p, r, icon, Color32::BLACK, self.s.color1);
            }
            CanvasCursor::Os(_) => unreachable!(),
        }
    }
}

/// Hotspots on the 24x24 icon grid: the bucket's drip and the eyedropper's tip.
pub const FILL_HOTSPOT: Vec2 = vec2(20.0, 20.0);
pub const PICKER_HOTSPOT: Vec2 = vec2(3.0, 21.0);
