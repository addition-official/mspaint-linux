//! Text tool: system font discovery (fontconfig paths via fontdb) and rasterization.

use crate::pixels::{IRect, max_pixel, new_pixmap, paint_for};
use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use egui::{Color32, Pos2, Rect, pos2};
use std::collections::HashMap;
use std::sync::Arc;
use tiny_skia::{Pixmap, PixmapPaint, Transform};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct TextStyle {
    pub family: String,
    pub size_pt: f32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    /// Fill the text box with the secondary color.
    pub opaque: bool,
    /// 0 = left, 1 = center, 2 = right.
    pub align: u8,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            family: String::new(),
            size_pt: 11.0,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            opaque: false,
            align: 0,
        }
    }
}

impl TextStyle {
    pub fn px(&self) -> f32 {
        self.size_pt * 96.0 / 72.0
    }
}

pub struct FontManager {
    db: fontdb::Database,
    pub families: Vec<String>,
    cache: HashMap<(String, bool, bool), Option<Arc<FontVec>>>,
}

impl FontManager {
    pub fn new() -> Self {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        let mut families: Vec<String> = db
            .faces()
            .filter_map(|f| f.families.first().map(|(n, _)| n.clone()))
            .collect();
        families.sort_by_key(|a| a.to_lowercase());
        families.dedup();
        Self { db, families, cache: HashMap::new() }
    }

    /// A sensible default family for this desktop (Kubuntu ships Noto Sans).
    pub fn default_family(&self) -> String {
        for f in ["Noto Sans", "Segoe UI", "Calibri", "Ubuntu", "DejaVu Sans", "Liberation Sans"] {
            if self.families.iter().any(|x| x == f) {
                return f.to_string();
            }
        }
        self.families.first().cloned().unwrap_or_default()
    }

    /// Raw font bytes for a family, used to give the UI the desktop's font.
    pub fn font_bytes(&self, family: &str, bold: bool) -> Option<(Vec<u8>, u32)> {
        let id = self.query(family, bold, false)?;
        self.db.with_face_data(id, |d, i| (d.to_vec(), i))
    }

    fn query(&self, family: &str, bold: bool, italic: bool) -> Option<fontdb::ID> {
        let families = [fontdb::Family::Name(family), fontdb::Family::SansSerif];
        let q = fontdb::Query {
            families: &families,
            weight: if bold { fontdb::Weight::BOLD } else { fontdb::Weight::NORMAL },
            stretch: fontdb::Stretch::Normal,
            style: if italic { fontdb::Style::Italic } else { fontdb::Style::Normal },
        };
        self.db.query(&q).or_else(|| self.db.faces().next().map(|f| f.id))
    }

    pub fn font(&mut self, family: &str, bold: bool, italic: bool) -> Option<Arc<FontVec>> {
        let key = (family.to_string(), bold, italic);
        if let Some(f) = self.cache.get(&key) {
            return f.clone();
        }
        let font = self.query(family, bold, italic).and_then(|id| {
            self.db
                .with_face_data(id, |d, i| FontVec::try_from_vec_and_index(d.to_vec(), i).ok())
                .flatten()
                .map(Arc::new)
        });
        self.cache.insert(key, font.clone());
        font
    }
}

/// Position of each character (by char index) after layout.
pub struct Layout {
    /// For each char index (plus one past the end): top-left of the caret.
    pub caret: Vec<Pos2>,
    pub line_height: f32,
    /// (char index range, y) for each visual line.
    pub lines: Vec<(usize, usize, f32)>,
}

/// Greedy word-wrap layout within `width`.
pub fn layout(font: &FontVec, px: f32, text: &str, origin: Pos2, width: f32, align: u8) -> Layout {
    let sf = font.as_scaled(PxScale::from(px));
    let line_height = sf.height() + sf.line_gap();
    let chars: Vec<char> = text.chars().collect();
    let adv = |c: char| sf.h_advance(font.glyph_id(c));
    let mut caret = Vec::with_capacity(chars.len() + 1);
    let mut lines = Vec::new();
    let mut x = 0.0f32;
    let mut y = 0.0f32;
    let mut line_start = 0usize;
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            caret.push(pos2(origin.x + x, origin.y + y));
            lines.push((line_start, i, y));
            x = 0.0;
            y += line_height;
            line_start = i + 1;
            i += 1;
            continue;
        }
        // At the start of a word, wrap if the whole word does not fit.
        if c != ' ' && (i == 0 || chars[i - 1] == ' ' || chars[i - 1] == '\n') && x > 0.0 {
            let word_w: f32 = chars[i..].iter().take_while(|c| **c != ' ' && **c != '\n').map(|c| adv(*c)).sum();
            if x + word_w > width {
                lines.push((line_start, i, y));
                x = 0.0;
                y += line_height;
                line_start = i;
            }
        }
        let a = adv(c);
        // Break very long words character by character.
        if x > 0.0 && x + a > width && c != ' ' {
            lines.push((line_start, i, y));
            x = 0.0;
            y += line_height;
            line_start = i;
        }
        caret.push(pos2(origin.x + x, origin.y + y));
        x += a;
        i += 1;
    }
    caret.push(pos2(origin.x + x, origin.y + y));
    lines.push((line_start, chars.len(), y));
    if align > 0 {
        for &(s, e, ly) in &lines {
            // Trailing spaces don't count towards the line's width.
            let mut last = e;
            while last > s && chars[last - 1] == ' ' {
                last -= 1;
            }
            let w = if last > s { caret[last - 1].x + adv(chars[last - 1]) - origin.x } else { 0.0 };
            let shift = (width - w).max(0.0) * if align == 1 { 0.5 } else { 1.0 };
            for c in caret[s..=e].iter_mut() {
                if c.y == origin.y + ly {
                    c.x += shift;
                }
            }
        }
    }
    Layout { caret, line_height, lines }
}

/// Render the text box into `layer`. Returns the dirty rect.
pub fn render(
    layer: &mut Pixmap,
    fm: &mut FontManager,
    style: &TextStyle,
    text: &str,
    rect: Rect,
    color: Color32,
    bg: Color32,
) -> IRect {
    let dirty = IRect::from_points((rect.min.x, rect.min.y), (rect.max.x, rect.max.y), 2.0)
        .clamp_to(layer.width(), layer.height());
    if style.opaque
        && let Some(r) = tiny_skia::Rect::from_ltrb(rect.min.x, rect.min.y, rect.max.x, rect.max.y) {
            layer.fill_rect(r, &paint_for(bg, false), Transform::identity(), None);
        }
    let Some(font) = fm.font(&style.family, style.bold, style.italic) else { return dirty };
    let px = style.px();
    let lay = layout(&font, px, text, rect.min, rect.width(), style.align);
    let sf = font.as_scaled(PxScale::from(px));
    let ascent = sf.ascent();
    let chars: Vec<char> = text.chars().collect();
    // Render glyphs into a temporary pixmap clipped to the text box.
    let w = rect.width().ceil().max(1.0) as u32;
    let h = rect.height().ceil().max(1.0) as u32;
    let mut tmp = new_pixmap(w, h, Color32::TRANSPARENT);
    for (i, &c) in chars.iter().enumerate() {
        if c == '\n' || c == ' ' {
            continue;
        }
        let p = lay.caret[i];
        let gid = font.glyph_id(c);
        let glyph = gid.with_scale_and_position(PxScale::from(px), ab_glyph::point(p.x - rect.min.x, p.y - rect.min.y + ascent));
        if let Some(og) = font.outline_glyph(glyph) {
            let bb = og.px_bounds();
            og.draw(|gx, gy, cov| {
                max_pixel(&mut tmp, bb.min.x as i32 + gx as i32, bb.min.y as i32 + gy as i32, color, cov);
            });
        }
    }
    let thickness = (px / 14.0).max(1.0);
    for &(s, e, y) in &lay.lines {
        if e <= s {
            continue;
        }
        let x0 = lay.caret[s].x - rect.min.x;
        let last = e - 1;
        let x1 = lay.caret[last].x - rect.min.x + sf.h_advance(font.glyph_id(chars[last]));
        let mut bar = |yy: f32| {
            if let Some(r) = tiny_skia::Rect::from_xywh(x0, yy, (x1 - x0).max(0.5), thickness) {
                tmp.fill_rect(r, &paint_for(color, true), Transform::identity(), None);
            }
        };
        if style.underline {
            bar(y + ascent + thickness * 1.5);
        }
        if style.strikethrough {
            bar(y + ascent * 0.62);
        }
    }
    layer.draw_pixmap(
        rect.min.x.round() as i32,
        rect.min.y.round() as i32,
        tmp.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    dirty
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_glyphs() {
        let mut fm = FontManager::new();
        let style = TextStyle { family: fm.default_family(), ..Default::default() };
        let mut layer = new_pixmap(300, 60, Color32::TRANSPARENT);
        let rect = Rect::from_min_size(pos2(10.0, 10.0), egui::vec2(250.0, 40.0));
        render(&mut layer, &mut fm, &style, "Hello", rect, Color32::BLACK, Color32::WHITE);
        let inked = layer.data().as_chunks::<4>().0.iter().filter(|p| p[3] > 0).count();
        assert!(inked > 50, "only {inked} pixels inked");
    }

    #[test]
    fn wraps_long_text() {
        let mut fm = FontManager::new();
        let font = fm.font(&fm.default_family(), false, false).expect("a system font");
        let lay = layout(&font, 15.0, "one two three four five six seven", pos2(0.0, 0.0), 60.0, 0);
        assert!(lay.lines.len() > 2);
        assert_eq!(lay.caret.len(), 34);
    }

    #[test]
    fn centers_and_right_aligns() {
        let mut fm = FontManager::new();
        let font = fm.font(&fm.default_family(), false, false).expect("a system font");
        let left = layout(&font, 15.0, "hi", pos2(0.0, 0.0), 200.0, 0);
        let center = layout(&font, 15.0, "hi", pos2(0.0, 0.0), 200.0, 1);
        let right = layout(&font, 15.0, "hi", pos2(0.0, 0.0), 200.0, 2);
        let w = left.caret[2].x;
        assert!((center.caret[0].x - (200.0 - w) / 2.0).abs() < 0.5);
        assert!((right.caret[2].x - 200.0).abs() < 0.5);
    }
}
