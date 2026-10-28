//! Pictures are uploaded to the GPU as a grid of textures, because a single texture
//! can't be larger than the GPU's limit (often 16384 pixels per side).

use crate::pixels::{self, IRect};
use egui::{Color32, Painter, Rect, TextureHandle, TextureOptions, pos2, vec2};
use tiny_skia::Pixmap;

const TILE: u32 = 4096;

pub struct TiledTexture {
    size: (u32, u32),
    tiles: Vec<(IRect, TextureHandle)>,
}

impl TiledTexture {
    pub fn new(ctx: &egui::Context, name: &str, p: &Pixmap, opts: TextureOptions) -> Self {
        let tile = TILE.min(ctx.input(|i| i.max_texture_side) as u32).max(256);
        let (w, h) = (p.width(), p.height());
        let mut tiles = Vec::new();
        for y0 in (0..h).step_by(tile as usize) {
            for x0 in (0..w).step_by(tile as usize) {
                let r = IRect::new(x0 as i32, y0 as i32, (x0 + tile).min(w) as i32, (y0 + tile).min(h) as i32);
                tiles.push((r, ctx.load_texture(format!("{name}-{x0}-{y0}"), pixels::region_color_image(p, r), opts)));
            }
        }
        Self { size: (w, h), tiles }
    }

    /// Re-upload the `dirty` part of `p`, which must have the size the tiles were made for.
    pub fn update(&mut self, p: &Pixmap, dirty: IRect, opts: TextureOptions) {
        for (r, t) in &mut self.tiles {
            let i = r.intersect(&dirty);
            if !i.is_empty() {
                t.set_partial([(i.x0 - r.x0) as usize, (i.y0 - r.y0) as usize], pixels::region_color_image(p, i), opts);
            }
        }
    }

    /// Draw the whole picture stretched over `screen`.
    pub fn paint(&self, p: &Painter, screen: Rect) {
        let (w, h) = (self.size.0 as f32, self.size.1 as f32);
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        let at = |x: i32, y: i32| screen.min + vec2(x as f32 / w * screen.width(), y as f32 / h * screen.height());
        for (r, t) in &self.tiles {
            let tr = Rect::from_min_max(at(r.x0, r.y0), at(r.x1, r.y1));
            if p.clip_rect().intersects(tr) {
                p.image(t.id(), tr, uv, Color32::WHITE);
            }
        }
    }
}
