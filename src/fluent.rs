//! Windows 11 (Fluent) controls: radio buttons, check boxes, toggle switches, sliders,
//! the combo box chevron and the keyboard focus ring. egui's own widgets work the same
//! way but look different.

use crate::theme::Palette;
use egui::{Color32, FontId, Rect, Response, Sense, Stroke, Ui, pos2, vec2};
use std::ops::RangeInclusive;

/// Marks drawn on the accent color (the dot of a radio button, the check mark).
fn on_accent(pal: &Palette) -> Color32 {
    if pal.dark { Color32::BLACK } else { Color32::WHITE }
}

/// A 20px control followed by its label, as one clickable widget.
fn with_label(ui: &mut Ui, pal: &Palette, text: &str) -> (Rect, Response, Rect) {
    let galley = ui.painter().layout_no_wrap(text.to_string(), FontId::proportional(14.0), pal.text);
    let gap = if text.is_empty() { 0.0 } else { 8.0 };
    // Trailing room so options in a row don't run into each other.
    let size = vec2(20.0 + gap + galley.size().x + gap * 1.5, 26.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let control = Rect::from_center_size(pos2(rect.min.x + 10.0, rect.center().y), vec2(20.0, 20.0));
    ui.painter().galley(pos2(control.max.x + gap, rect.center().y - galley.size().y / 2.0), galley, pal.text);
    (rect, resp, control)
}

pub fn radio(ui: &mut Ui, pal: &Palette, selected: bool, text: &str) -> Response {
    let (_, resp, b) = with_label(ui, pal, text);
    let (c, p) = (b.center(), ui.painter());
    let pressed = resp.is_pointer_button_down_on();
    if selected {
        p.circle_filled(c, 10.0, pal.accent);
        let dot = if pressed { 3.0 } else if resp.hovered() { 5.0 } else { 4.0 };
        p.circle_filled(c, dot, on_accent(pal));
    } else {
        if resp.hovered() {
            p.circle_filled(c, 10.0, pal.hover);
        }
        p.circle_stroke(c, 9.5, Stroke::new(1.0, pal.text_dim));
        if pressed {
            p.circle_filled(c, 5.0, pal.text_dim);
        }
    }
    resp
}

pub fn radio_value<T: PartialEq>(ui: &mut Ui, pal: &Palette, current: &mut T, value: T, text: &str) -> Response {
    let mut resp = radio(ui, pal, *current == value, text);
    if resp.clicked() && *current != value {
        *current = value;
        resp.mark_changed();
    }
    resp
}

pub fn checkbox(ui: &mut Ui, pal: &Palette, checked: &mut bool, text: &str) -> Response {
    let (_, mut resp, b) = with_label(ui, pal, text);
    if resp.clicked() {
        *checked = !*checked;
        resp.mark_changed();
    }
    let p = ui.painter();
    if *checked {
        let fill = if resp.is_pointer_button_down_on() { pal.accent.gamma_multiply(0.8) } else { pal.accent };
        p.rect_filled(b, 4.0, fill);
        let pt = |x: f32, y: f32| pos2(b.min.x + x, b.min.y + y);
        p.add(egui::Shape::line(vec![pt(5.0, 10.5), pt(8.5, 14.0), pt(15.0, 6.5)], Stroke::new(1.5, on_accent(pal))));
    } else {
        if resp.hovered() {
            p.rect_filled(b, 4.0, pal.hover);
        }
        p.rect_stroke(b.shrink(0.5), 4.0, Stroke::new(1.0, pal.text_dim), egui::StrokeKind::Middle);
    }
    resp
}

/// On/off switch (Windows settings style).
pub fn toggle(ui: &mut Ui, pal: &Palette, on: &mut bool) -> Response {
    let (rect, mut resp) = ui.allocate_exact_size(vec2(40.0, 20.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let p = ui.painter();
    let knob = if resp.is_pointer_button_down_on() { 7.0 } else if resp.hovered() { 6.5 } else { 6.0 };
    if *on {
        p.rect_filled(rect, 10.0, pal.accent);
        p.circle_filled(pos2(rect.max.x - 10.0, rect.center().y), knob, on_accent(pal));
    } else {
        if resp.hovered() {
            p.rect_filled(rect, 10.0, pal.hover);
        }
        p.rect_stroke(rect.shrink(0.5), 10.0, Stroke::new(1.0, pal.text_dim), egui::StrokeKind::Middle);
        p.circle_filled(pos2(rect.min.x + 10.0, rect.center().y), knob, pal.text_dim);
    }
    resp
}

/// Slider with an accent track up to the value and a round thumb with an accent
/// center. `vertical` sliders grow upward. Arrow keys adjust it when focused.
pub fn slider(ui: &mut Ui, pal: &Palette, value: &mut f32, range: RangeInclusive<f32>, length: f32, vertical: bool) -> Response {
    let size = if vertical { vec2(22.0, length) } else { vec2(length, 22.0) };
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let (lo, hi) = (*range.start(), *range.end());
    let r = 10.0;
    let (a, b) = if vertical {
        (pos2(rect.center().x, rect.max.y - r), pos2(rect.center().x, rect.min.y + r))
    } else {
        (pos2(rect.min.x + r, rect.center().y), pos2(rect.max.x - r, rect.center().y))
    };
    let mut target = None;
    if let Some(p) = resp.interact_pointer_pos()
        && (resp.dragged() || resp.is_pointer_button_down_on())
    {
        let t = if vertical { (a.y - p.y) / (a.y - b.y) } else { (p.x - a.x) / (b.x - a.x) };
        target = Some(lo + t.clamp(0.0, 1.0) * (hi - lo));
    }
    if resp.has_focus() {
        // Whole units for sizes and percentages; 1% of the range for small ranges.
        let step = if hi - lo >= 20.0 { 1.0 } else { (hi - lo) / 100.0 };
        let (up, down) = ui.input(|i| {
            (i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::ArrowRight), i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::ArrowLeft))
        });
        if up {
            target = Some(*value + step);
        } else if down {
            target = Some(*value - step);
        }
    }
    if let Some(v) = target.map(|v| v.clamp(lo, hi))
        && v != *value
    {
        *value = v;
        resp.mark_changed();
    }
    let t = if hi > lo { ((*value - lo) / (hi - lo)).clamp(0.0, 1.0) } else { 0.0 };
    let at = a + (b - a) * t;
    let p = ui.painter();
    let rail = if pal.dark { Color32::from_white_alpha(140) } else { Color32::from_black_alpha(110) };
    for (from, to, c) in [(at, b, rail), (a, at, pal.accent)] {
        p.line_segment([from, to], Stroke::new(4.0, c));
        p.circle_filled(from, 2.0, c);
        p.circle_filled(to, 2.0, c);
    }
    let thumb = if pal.dark { Color32::from_rgb(0x45, 0x45, 0x45) } else { Color32::WHITE };
    p.circle_filled(at, r, thumb);
    p.circle_stroke(at, r, Stroke::new(1.0, pal.separator));
    let inner = if resp.is_pointer_button_down_on() || resp.dragged() { 5.0 } else if resp.hovered() { 7.0 } else { 6.0 };
    p.circle_filled(at, inner, pal.accent);
    resp
}

/// Fluent chevron for `ComboBox::icon`.
pub fn combo_chevron(ui: &Ui, rect: Rect, visuals: &egui::style::WidgetVisuals, _open: bool) {
    let c = rect.center();
    let s = Stroke::new(1.2, visuals.fg_stroke.color);
    let pts = vec![c + vec2(-4.0, -2.0), c + vec2(0.0, 2.0), c + vec2(4.0, -2.0)];
    ui.painter().add(egui::Shape::line(pts, s));
}

/// Draw the keyboard focus ring (Windows shows it only while using the keyboard).
pub fn focus_ring(ctx: &egui::Context, pal: &Palette) {
    let kb = egui::Id::new("keyboard_navigation");
    let (tab, pointer) = ctx.input(|i| {
        let tab = i.events.iter().any(|e| {
            matches!(e, egui::Event::Key { key: egui::Key::Tab | egui::Key::ArrowUp | egui::Key::ArrowDown, pressed: true, .. })
        });
        (tab, i.pointer.any_pressed())
    });
    let mut active = ctx.data(|d| d.get_temp::<bool>(kb)).unwrap_or(false);
    if tab {
        active = true;
    } else if pointer {
        active = false;
    }
    ctx.data_mut(|d| d.insert_temp(kb, active));
    if !active {
        return;
    }
    let Some(id) = ctx.memory(|m| m.focused()) else { return };
    let Some(resp) = ctx.read_response(id) else { return };
    let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("focus_ring")));
    p.rect_stroke(resp.rect.expand(2.0), 6.0, Stroke::new(2.0, pal.text), egui::StrokeKind::Outside);
}

/// Number box like a WinUI NumberBox: type a value (the whole text is selected when it
/// gets focus), Enter or leaving the box applies it, clamped to `range`. `suffix` (such
/// as "%") is shown after the number and ignored when typed.
pub fn number_field(
    ui: &mut Ui,
    id_salt: &str,
    value: &mut f32,
    range: RangeInclusive<f32>,
    decimals: usize,
    suffix: &str,
    width: f32,
) -> Response {
    number_box(ui, id_salt, value, range, decimals, suffix, width, true)
}

/// Like `number_field`, but the value only changes when Enter is pressed or the box
/// loses focus (for the zoom box, where partial input like "2" of "200" would jump).
pub fn number_entry(
    ui: &mut Ui,
    id_salt: &str,
    value: &mut f32,
    range: RangeInclusive<f32>,
    decimals: usize,
    suffix: &str,
    width: f32,
) -> Response {
    number_box(ui, id_salt, value, range, decimals, suffix, width, false)
}

#[allow(clippy::too_many_arguments)]
fn number_box(
    ui: &mut Ui,
    id_salt: &str,
    value: &mut f32,
    range: RangeInclusive<f32>,
    decimals: usize,
    suffix: &str,
    width: f32,
    live: bool,
) -> Response {
    let id = ui.make_persistent_id(id_salt);
    let edit_id = id.with("edit");
    let format = |v: f32| format!("{v:.decimals$}{suffix}");
    let editing = ui.memory(|m| m.has_focus(edit_id));
    let mut text = if editing { ui.data(|d| d.get_temp::<String>(id)).unwrap_or_else(|| format(*value)) } else { format(*value) };
    let mut resp = ui.add_sized(
        [width, 26.0],
        egui::TextEdit::singleline(&mut text).id(edit_id).horizontal_align(egui::Align::Center).margin(vec2(6.0, 4.0)),
    );
    if resp.gained_focus()
        && let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), edit_id)
    {
        let n = text.chars().count();
        state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
            egui::text::CCursor::new(0),
            egui::text::CCursor::new(n),
        )));
        state.store(ui.ctx(), edit_id);
    }
    let mut changed = false;
    if if live { resp.changed() } else { resp.lost_focus() } {
        let typed = text.trim().trim_end_matches(suffix).trim().replace(',', ".");
        if let Ok(v) = typed.parse::<f32>()
            && v.is_finite()
        {
            let v = v.clamp(*range.start(), *range.end());
            if v != *value {
                *value = v;
                changed = true;
            }
        }
    }
    ui.data_mut(|d| d.insert_temp(id, text));
    // `changed` means the value changed (not just the text).
    resp.flags.set(egui::response::Flags::CHANGED, changed);
    resp
}
