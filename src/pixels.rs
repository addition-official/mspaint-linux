//! Low-level pixel operations on premultiplied RGBA `tiny_skia::Pixmap`s.

use egui::Color32;
use tiny_skia::{
    BlendMode, Color, FilterQuality, IntSize, Paint, Pixmap, PixmapPaint, Transform,
};

/// Integer rectangle, half-open: `[x0, x1) x [y0, y1)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl IRect {
    pub fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self { x0, y0, x1, y1 }
    }
    pub fn from_size(w: u32, h: u32) -> Self {
        Self::new(0, 0, w as i32, h as i32)
    }
    pub fn from_points(a: (f32, f32), b: (f32, f32), pad: f32) -> Self {
        Self::new(
            (a.0.min(b.0) - pad).floor() as i32,
            (a.1.min(b.1) - pad).floor() as i32,
            (a.0.max(b.0) + pad).ceil() as i32 + 1,
            (a.1.max(b.1) + pad).ceil() as i32 + 1,
        )
    }
    pub fn width(&self) -> i32 {
        self.x1 - self.x0
    }
    pub fn height(&self) -> i32 {
        self.y1 - self.y0
    }
    pub fn is_empty(&self) -> bool {
        self.x1 <= self.x0 || self.y1 <= self.y0
    }
    pub fn union(&self, o: &IRect) -> IRect {
        if self.is_empty() {
            return *o;
        }
        if o.is_empty() {
            return *self;
        }
        IRect::new(
            self.x0.min(o.x0),
            self.y0.min(o.y0),
            self.x1.max(o.x1),
            self.y1.max(o.y1),
        )
    }
    pub fn intersect(&self, o: &IRect) -> IRect {
        IRect::new(
            self.x0.max(o.x0),
            self.y0.max(o.y0),
            self.x1.min(o.x1),
            self.y1.min(o.y1),
        )
    }
    pub fn clamp_to(&self, w: u32, h: u32) -> IRect {
        self.intersect(&IRect::from_size(w, h))
    }
}

pub fn union_opt(a: Option<IRect>, b: IRect) -> Option<IRect> {
    Some(match a {
        Some(a) => a.union(&b),
        None => b,
    })
}

pub fn sk_color(c: Color32) -> Color {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    Color::from_rgba8(r, g, b, a)
}

pub fn paint_for(c: Color32, anti_alias: bool) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(sk_color(c));
    p.anti_alias = anti_alias;
    p
}

pub fn new_pixmap(w: u32, h: u32, fill: Color32) -> Pixmap {
    let mut p = Pixmap::new(w.max(1), h.max(1)).expect("valid pixmap size");
    if fill != Color32::TRANSPARENT {
        p.fill(sk_color(fill));
    }
    p
}

/// Premultiplied RGBA of an (opaque or not) egui color.
pub fn premul(c: Color32) -> [u8; 4] {
    c.to_array()
}

#[inline]
pub fn idx(p: &Pixmap, x: i32, y: i32) -> Option<usize> {
    if x < 0 || y < 0 || x >= p.width() as i32 || y >= p.height() as i32 {
        None
    } else {
        Some((y as usize * p.width() as usize + x as usize) * 4)
    }
}

/// Unpremultiplied color at a pixel.
pub fn get_pixel(p: &Pixmap, x: i32, y: i32) -> Option<Color32> {
    let i = idx(p, x, y)?;
    let d = p.data();
    Some(Color32::from_rgba_premultiplied(d[i], d[i + 1], d[i + 2], d[i + 3]))
}

#[inline]
pub fn set_pixel(p: &mut Pixmap, x: i32, y: i32, c: [u8; 4]) {
    if let Some(i) = idx(p, x, y) {
        p.data_mut()[i..i + 4].copy_from_slice(&c);
    }
}

/// Write `c` (premultiplied, alpha = full color alpha) scaled to `alpha`, keeping the max coverage.
#[inline]
pub fn max_pixel(p: &mut Pixmap, x: i32, y: i32, c: Color32, alpha: f32) {
    if let Some(i) = idx(p, x, y) {
        let a = (alpha.clamp(0.0, 1.0) * 255.0) as u8;
        let d = p.data_mut();
        if a > d[i + 3] {
            let s = a as f32 / 255.0;
            d[i] = (c.r() as f32 * s) as u8;
            d[i + 1] = (c.g() as f32 * s) as u8;
            d[i + 2] = (c.b() as f32 * s) as u8;
            d[i + 3] = a;
        }
    }
}

pub fn to_color_image(p: &Pixmap) -> egui::ColorImage {
    egui::ColorImage::from_rgba_premultiplied([p.width() as usize, p.height() as usize], p.data())
}

pub fn region_color_image(p: &Pixmap, r: IRect) -> egui::ColorImage {
    let w = r.width() as usize;
    let h = r.height() as usize;
    let pw = p.width() as usize;
    let d = p.data();
    let mut out = Vec::with_capacity(w * h * 4);
    for y in r.y0..r.y1 {
        let s = (y as usize * pw + r.x0 as usize) * 4;
        out.extend_from_slice(&d[s..s + w * 4]);
    }
    egui::ColorImage::from_rgba_premultiplied([w, h], &out)
}

/// `dst[r] = base[r] (src-over) layer[r] * opacity`
pub fn composite_region(dst: &mut Pixmap, base: &Pixmap, layer: &Pixmap, opacity: f32, r: IRect) {
    let r = r.clamp_to(dst.width(), dst.height());
    if r.is_empty() {
        return;
    }
    let w = dst.width() as usize;
    let op = (opacity.clamp(0.0, 1.0) * 256.0) as u32;
    let bd = base.data();
    let ld = layer.data();
    let dd = dst.data_mut();
    for y in r.y0..r.y1 {
        let row = y as usize * w;
        for x in r.x0..r.x1 {
            let i = (row + x as usize) * 4;
            let la = ld[i + 3] as u32;
            if la == 0 {
                dd[i..i + 4].copy_from_slice(&bd[i..i + 4]);
                continue;
            }
            let sa = (la * op) >> 8;
            let inv = 255 - sa;
            for k in 0..4 {
                let s = (ld[i + k] as u32 * op) >> 8;
                dd[i + k] = (s + (bd[i + k] as u32 * inv + 127) / 255).min(255) as u8;
            }
        }
    }
}

/// `dst[r] = base[r]` with the stroke `layer` *erasing* it: alpha is removed where the
/// layer has coverage (destination-out).
pub fn erase_region(dst: &mut Pixmap, base: &Pixmap, layer: &Pixmap, r: IRect) {
    let r = r.clamp_to(dst.width(), dst.height());
    if r.is_empty() {
        return;
    }
    let w = dst.width() as usize;
    let bd = base.data();
    let ld = layer.data();
    let dd = dst.data_mut();
    for y in r.y0..r.y1 {
        let row = y as usize * w;
        for x in r.x0..r.x1 {
            let i = (row + x as usize) * 4;
            let keep = 255 - ld[i + 3] as u32;
            for k in 0..4 {
                dd[i + k] = ((bd[i + k] as u32 * keep + 127) / 255) as u8;
            }
        }
    }
}

/// `dst[r] = src[r] over dst[r]` (premultiplied src-over).
pub fn over_region(dst: &mut Pixmap, src: &Pixmap, r: IRect) {
    blend_region(dst, src, r, false);
}

/// `dst[r] = dst[r] over src[r]` (src goes underneath).
pub fn under_region(dst: &mut Pixmap, src: &Pixmap, r: IRect) {
    blend_region(dst, src, r, true);
}

fn blend_region(dst: &mut Pixmap, src: &Pixmap, r: IRect, under: bool) {
    let r = r.clamp_to(dst.width(), dst.height());
    if r.is_empty() {
        return;
    }
    let w = dst.width() as usize;
    let sd = src.data();
    let dd = dst.data_mut();
    for y in r.y0..r.y1 {
        let row = y as usize * w;
        for x in r.x0..r.x1 {
            let i = (row + x as usize) * 4;
            let (top, bottom) = if under { (&dd[i..i + 4], &sd[i..i + 4]) } else { (&sd[i..i + 4], &dd[i..i + 4]) };
            let ta = top[3] as u32;
            if ta == 255 {
                let t = [top[0], top[1], top[2], top[3]];
                dd[i..i + 4].copy_from_slice(&t);
                continue;
            }
            let inv = 255 - ta;
            let mut out = [0u8; 4];
            for k in 0..4 {
                out[k] = (top[k] as u32 + (bottom[k] as u32 * inv + 127) / 255).min(255) as u8;
            }
            dd[i..i + 4].copy_from_slice(&out);
        }
    }
}

/// Composite `layers` (bottom first) into one pixmap.
pub fn flatten<'a>(w: u32, h: u32, layers: impl Iterator<Item = &'a Pixmap>) -> Pixmap {
    let mut out = new_pixmap(w, h, Color32::TRANSPARENT);
    for l in layers {
        out.draw_pixmap(0, 0, l.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
    }
    out
}

pub fn clear_region(p: &mut Pixmap, r: IRect) {
    let r = r.clamp_to(p.width(), p.height());
    if r.is_empty() {
        return;
    }
    let w = p.width() as usize;
    let d = p.data_mut();
    for y in r.y0..r.y1 {
        let s = (y as usize * w + r.x0 as usize) * 4;
        d[s..s + r.width() as usize * 4].fill(0);
    }
}

/// Flood fill (4-connected) from (x, y). Pixels whose channels all differ from the
/// start pixel by at most `tolerance` (0-255) are filled. Returns the touched bounding box.
pub fn flood_fill(p: &mut Pixmap, x: i32, y: i32, color: Color32, tolerance: u8) -> Option<IRect> {
    let start = idx(p, x, y)?;
    let w = p.width() as i32;
    let h = p.height() as i32;
    let fill = premul(color);
    let d = p.data_mut();
    let target = [d[start], d[start + 1], d[start + 2], d[start + 3]];
    if target == fill && tolerance == 0 {
        return None;
    }
    let tol = tolerance as i16;
    let matches = |px: &[u8]| (0..4).all(|k| (px[k] as i16 - target[k] as i16).abs() <= tol);
    let mut seen = vec![false; (w * h) as usize];
    let at = |x: i32, y: i32| (y * w + x) as usize;
    let mut bbox = IRect::new(x, y, x + 1, y + 1);
    let mut stack = vec![(x, y)];
    while let Some((sx, sy)) = stack.pop() {
        if seen[at(sx, sy)] || !matches(&d[at(sx, sy) * 4..at(sx, sy) * 4 + 4]) {
            continue;
        }
        let ok = |d: &[u8], seen: &[bool], x: i32, y: i32| !seen[at(x, y)] && matches(&d[at(x, y) * 4..at(x, y) * 4 + 4]);
        let mut lx = sx;
        while lx > 0 && ok(d, &seen, lx - 1, sy) {
            lx -= 1;
        }
        let mut rx = sx;
        while rx < w - 1 && ok(d, &seen, rx + 1, sy) {
            rx += 1;
        }
        for xx in lx..=rx {
            let i = at(xx, sy);
            seen[i] = true;
            d[i * 4..i * 4 + 4].copy_from_slice(&fill);
        }
        bbox = bbox.union(&IRect::new(lx, sy, rx + 1, sy + 1));
        for ny in [sy - 1, sy + 1] {
            if ny < 0 || ny >= h {
                continue;
            }
            let mut xx = lx;
            while xx <= rx {
                if ok(d, &seen, xx, ny) {
                    stack.push((xx, ny));
                    while xx <= rx && ok(d, &seen, xx, ny) {
                        xx += 1;
                    }
                } else {
                    xx += 1;
                }
            }
        }
    }
    Some(bbox)
}

pub fn copy_rect(p: &Pixmap, r: IRect) -> Pixmap {
    let mut out = new_pixmap(r.width().max(1) as u32, r.height().max(1) as u32, Color32::TRANSPARENT);
    draw_exact(&mut out, p, -r.x0, -r.y0);
    out
}

/// Copy `src` into `dst` at (x, y) replacing pixels (no blending).
pub fn draw_exact(dst: &mut Pixmap, src: &Pixmap, x: i32, y: i32) {
    let paint = PixmapPaint {
        blend_mode: BlendMode::Source,
        quality: FilterQuality::Nearest,
        ..Default::default()
    };
    dst.draw_pixmap(x, y, src.as_ref(), &paint, Transform::identity(), None);
}

/// Draw `src` scaled into the destination rectangle (x, y, w, h) with src-over blending.
pub fn draw_scaled(dst: &mut Pixmap, src: &Pixmap, x: f32, y: f32, w: f32, h: f32) {
    let sx = w / src.width() as f32;
    let sy = h / src.height() as f32;
    let exact = (sx - 1.0).abs() < 1e-4 && (sy - 1.0).abs() < 1e-4;
    let paint = PixmapPaint {
        quality: if exact { FilterQuality::Nearest } else { FilterQuality::Bilinear },
        ..Default::default()
    };
    if exact {
        dst.draw_pixmap(x.round() as i32, y.round() as i32, src.as_ref(), &paint, Transform::identity(), None);
    } else {
        let t = Transform::from_row(sx, 0.0, 0.0, sy, x, y);
        dst.draw_pixmap(0, 0, src.as_ref(), &paint, t, None);
    }
}

pub fn resize(src: &Pixmap, w: u32, h: u32) -> Pixmap {
    let mut out = new_pixmap(w, h, Color32::TRANSPARENT);
    let paint = PixmapPaint {
        quality: FilterQuality::Bicubic,
        blend_mode: BlendMode::Source,
        ..Default::default()
    };
    let t = Transform::from_scale(w as f32 / src.width() as f32, h as f32 / src.height() as f32);
    out.draw_pixmap(0, 0, src.as_ref(), &paint, t, None);
    out
}

pub fn skew(src: &Pixmap, deg_x: f32, deg_y: f32, bg: Color32) -> Pixmap {
    let kx = deg_x.to_radians().tan();
    let ky = deg_y.to_radians().tan();
    let (w, h) = (src.width() as f32, src.height() as f32);
    let nw = (w + (h * kx).abs()).round().max(1.0) as u32;
    let nh = (h + (w * ky).abs()).round().max(1.0) as u32;
    let mut out = new_pixmap(nw, nh, bg);
    let tx = if kx < 0.0 { -h * kx } else { 0.0 };
    let ty = if ky < 0.0 { -w * ky } else { 0.0 };
    let paint = PixmapPaint {
        quality: FilterQuality::Bilinear,
        ..Default::default()
    };
    out.draw_pixmap(0, 0, src.as_ref(), &paint, Transform::from_row(1.0, ky, kx, 1.0, tx, ty), None);
    out
}

fn remap(src: &Pixmap, w: u32, h: u32, f: impl Fn(i32, i32) -> (i32, i32)) -> Pixmap {
    let mut out = new_pixmap(w, h, Color32::TRANSPARENT);
    let sw = src.width() as usize;
    let sd = src.data();
    let od = out.data_mut();
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let (sx, sy) = f(x, y);
            let si = (sy as usize * sw + sx as usize) * 4;
            let oi = (y as usize * w as usize + x as usize) * 4;
            od[oi..oi + 4].copy_from_slice(&sd[si..si + 4]);
        }
    }
    out
}

pub fn flip_h(p: &Pixmap) -> Pixmap {
    let w = p.width() as i32;
    remap(p, p.width(), p.height(), |x, y| (w - 1 - x, y))
}
pub fn flip_v(p: &Pixmap) -> Pixmap {
    let h = p.height() as i32;
    remap(p, p.width(), p.height(), |x, y| (x, h - 1 - y))
}
pub fn rotate_cw(p: &Pixmap) -> Pixmap {
    let h = p.height() as i32;
    remap(p, p.height(), p.width(), |x, y| (y, h - 1 - x))
}
pub fn rotate_ccw(p: &Pixmap) -> Pixmap {
    let w = p.width() as i32;
    remap(p, p.height(), p.width(), |x, y| (w - 1 - y, x))
}
pub fn rotate_180(p: &Pixmap) -> Pixmap {
    let (w, h) = (p.width() as i32, p.height() as i32);
    remap(p, p.width(), p.height(), |x, y| (w - 1 - x, h - 1 - y))
}

/// Rotate by an arbitrary angle (degrees, clockwise). The result grows to the rotated
/// bounding box; uncovered corners are transparent.
pub fn rotate_any(src: &Pixmap, deg: f32) -> Pixmap {
    let a = deg.to_radians();
    let (w, h) = (src.width() as f32, src.height() as f32);
    let (c, s) = (a.cos().abs(), a.sin().abs());
    // Small epsilon: cos(90°) is not exactly zero in floating point.
    let nw = (w * c + h * s - 1e-3).ceil().max(1.0) as u32;
    let nh = (w * s + h * c - 1e-3).ceil().max(1.0) as u32;
    let mut out = new_pixmap(nw, nh, Color32::TRANSPARENT);
    let t = Transform::from_translate(nw as f32 / 2.0, nh as f32 / 2.0)
        .pre_rotate(deg)
        .pre_translate(-w / 2.0, -h / 2.0);
    let paint = PixmapPaint { quality: FilterQuality::Bilinear, ..Default::default() };
    out.draw_pixmap(0, 0, src.as_ref(), &paint, t, None);
    out
}

pub fn invert_colors(p: &mut Pixmap) {
    for px in p.data_mut().chunks_exact_mut(4) {
        let a = px[3];
        for c in &mut px[..3] {
            *c = a - (*c).min(a);
        }
    }
}

/// New canvas of the given size with the old content at the top-left.
pub fn resize_canvas(p: &Pixmap, w: u32, h: u32, bg: Color32) -> Pixmap {
    let mut out = new_pixmap(w, h, bg);
    draw_exact(&mut out, p, 0, 0);
    out
}

/// Make pixels matching `key` fully transparent.
pub fn key_out(p: &mut Pixmap, key: Color32) {
    let k = premul(key);
    for px in p.data_mut().chunks_exact_mut(4) {
        if px == k {
            px.fill(0);
        }
    }
}

// ---------------------------------------------------------------------------
// File I/O

pub fn load(path: &std::path::Path) -> Result<Pixmap, String> {
    let img = image::ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())?;
    Ok(from_dynamic(img))
}

pub fn from_dynamic(img: image::DynamicImage) -> Pixmap {
    let rgba = img.to_rgba8();
    from_rgba_unmultiplied(rgba.width(), rgba.height(), rgba.into_raw())
}

pub fn from_rgba_unmultiplied(w: u32, h: u32, mut data: Vec<u8>) -> Pixmap {
    for px in data.chunks_exact_mut(4) {
        let a = px[3] as u32;
        if a < 255 {
            for c in &mut px[..3] {
                *c = ((*c as u32 * a + 127) / 255) as u8;
            }
        }
    }
    Pixmap::from_vec(data, IntSize::from_wh(w.max(1), h.max(1)).unwrap()).expect("pixmap")
}

pub fn to_rgba_unmultiplied(p: &Pixmap) -> image::RgbaImage {
    let mut data = p.data().to_vec();
    for px in data.chunks_exact_mut(4) {
        let a = px[3] as u32;
        if a > 0 && a < 255 {
            for c in &mut px[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    image::RgbaImage::from_raw(p.width(), p.height(), data).expect("rgba image")
}

pub const SAVE_FORMATS: &[(&str, &[&str])] = &[
    ("PNG", &["png"]),
    ("JPEG", &["jpg", "jpeg", "jpe", "jfif"]),
    ("BMP", &["bmp", "dib"]),
    ("GIF", &["gif"]),
    ("TIFF", &["tif", "tiff"]),
    ("WEBP", &["webp"]),
    ("ICO", &["ico"]),
];

pub const OPEN_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "jpe", "jfif", "bmp", "dib", "gif", "tif", "tiff", "webp", "ico", "tga",
    "pnm", "ppm", "pgm", "pbm", "qoi", "hdr", "exr", "dds", "ff",
];

pub fn save(p: &Pixmap, path: &std::path::Path) -> Result<(), String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_ascii_lowercase();
    let rgba = to_rgba_unmultiplied(p);
    let flatten = || {
        // Composite over white for formats without alpha.
        let mut rgb = image::RgbImage::new(rgba.width(), rgba.height());
        for (o, i) in rgb.pixels_mut().zip(rgba.pixels()) {
            let a = i[3] as u32;
            for k in 0..3 {
                o[k] = ((i[k] as u32 * a + 255 * (255 - a)) / 255) as u8;
            }
        }
        rgb
    };
    let res = match ext.as_str() {
        "jpg" | "jpeg" | "jpe" | "jfif" => {
            let f = std::fs::File::create(path).map_err(|e| e.to_string())?;
            let mut w = std::io::BufWriter::new(f);
            let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut w, 95);
            flatten().write_with_encoder(enc)
        }
        "bmp" | "dib" => flatten().save_with_format(path, image::ImageFormat::Bmp),
        "ico" => {
            let (w, h) = (rgba.width(), rgba.height());
            let img = if w > 256 || h > 256 {
                image::DynamicImage::ImageRgba8(rgba).resize(256, 256, image::imageops::FilterType::Lanczos3)
            } else {
                image::DynamicImage::ImageRgba8(rgba)
            };
            img.save_with_format(path, image::ImageFormat::Ico)
        }
        "gif" => rgba.save_with_format(path, image::ImageFormat::Gif),
        "tif" | "tiff" => rgba.save_with_format(path, image::ImageFormat::Tiff),
        "webp" => rgba.save_with_format(path, image::ImageFormat::WebP),
        _ => rgba.save_with_format(path, image::ImageFormat::Png),
    };
    res.map_err(|e| e.to_string())
}

/// Hash-based deterministic noise in [0, 1).
#[inline]
pub fn noise(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6b343) ^ (y as u32).wrapping_mul(0xd8163841) ^ seed.wrapping_mul(0xcb1ab31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1e995);
    h ^= h >> 15;
    (h & 0xffffff) as f32 / 16777216.0
}

/// Tiny xorshift RNG.
#[derive(Clone)]
pub struct Rng(pub u64);
impl Rng {
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 32) as u32
    }
    pub fn f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotate_any_grows_and_keeps_content() {
        let p = new_pixmap(40, 20, Color32::RED);
        let r = rotate_any(&p, 90.0);
        assert_eq!((r.width(), r.height()), (20, 40));
        assert_eq!(get_pixel(&r, 10, 20).unwrap(), Color32::RED);
        let r = rotate_any(&p, 45.0);
        assert!(r.width() > 40 && r.height() > 40);
        assert_eq!(get_pixel(&r, 0, 0).unwrap().a(), 0, "corners are transparent");
        assert_eq!(get_pixel(&r, r.width() as i32 / 2, r.height() as i32 / 2).unwrap(), Color32::RED);
    }
}
