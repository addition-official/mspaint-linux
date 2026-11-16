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
fn with_label(ui: &mut Ui, pal: &Palette, text: &str, sense: Sense) -> (Rect, Response, Rect) {
    let size = if pal.classic != 0 { 12.5 } else { 14.0 };
    let galley = ui.painter().layout_no_wrap(text.to_string(), FontId::proportional(size), pal.text);
    let gap = if text.is_empty() { 0.0 } else { 8.0 };
    // Trailing room so options in a row don't run into each other.
    let size = vec2(20.0 + gap + galley.size().x + gap * 1.5, 26.0);
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    let control = Rect::from_center_size(pos2(rect.min.x + 10.0, rect.center().y), vec2(20.0, 20.0));
    ui.painter().galley(pos2(control.max.x + gap, rect.center().y - galley.size().y / 2.0), galley, pal.text);
    (rect, resp, control)
}

/// A radio button. The buttons placed in the same `Ui` (one row or column) form a
/// group, as in Windows: Tab stops only at the selected one, and the arrow keys
/// select the previous or next one.
pub fn radio(ui: &mut Ui, pal: &Palette, selected: bool, text: &str) -> Response {
    // Unselected buttons are skipped by Tab, but keep focus while the arrow keys
    // move it to them (until they become selected).
    let id = ui.next_auto_id();
    let pending = ui.data(|d| d.get_temp::<egui::Id>(egui::Id::new("radio_activate"))) == Some(id);
    let focusable = selected || pending || ui.memory(|m| m.has_focus(id));
    let sense = if focusable { Sense::click() } else { Sense::CLICK };
    let (_, mut resp, b) = with_label(ui, pal, text, sense);
    radio_group_keys(ui, &mut resp);
    if pal.classic != 0 {
        classic_radio(ui, pal, b.center(), &resp, selected);
        return resp;
    }
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

/// The buttons of each radio group, in order: the ones placed so far this frame and
/// the full list from the last frame (the arrow keys can move to a later button).
#[derive(Clone, Default)]
struct RadioGroup {
    frame: u64,
    now: Vec<egui::Id>,
    last: Vec<egui::Id>,
}

fn radio_group_keys(ui: &mut Ui, resp: &mut Response) {
    let key = egui::Id::new(("radio_group", ui.unique_id()));
    let frame = ui.ctx().cumulative_pass_nr();
    let group = ui.data_mut(|d| {
        let g = d.get_temp_mut_or_default::<RadioGroup>(key);
        if g.frame != frame {
            g.last = std::mem::take(&mut g.now);
            g.frame = frame;
        }
        g.now.push(resp.id);
        g.last.clone()
    });
    // A button the arrow keys moved to last frame acts as clicked.
    let activate = egui::Id::new("radio_activate");
    if ui.data(|d| d.get_temp::<egui::Id>(activate)) == Some(resp.id) {
        ui.data_mut(|d| d.remove::<egui::Id>(activate));
        resp.flags |= egui::response::Flags::FAKE_PRIMARY_CLICKED;
    }
    if !resp.has_focus() {
        return;
    }
    let filter = egui::EventFilter { horizontal_arrows: true, vertical_arrows: true, ..Default::default() };
    ui.memory_mut(|m| m.set_focus_lock_filter(resp.id, filter));
    // Consumed, so the button moved to (it may come later this frame) doesn't see it.
    let step = ui.input_mut(|i| {
        use egui::{Key::*, Modifiers};
        if i.consume_key(Modifiers::NONE, ArrowDown) || i.consume_key(Modifiers::NONE, ArrowRight) {
            1
        } else if i.consume_key(Modifiers::NONE, ArrowUp) || i.consume_key(Modifiers::NONE, ArrowLeft) {
            -1
        } else {
            0
        }
    });
    if step == 0 {
        return;
    }
    if let Some(i) = group.iter().position(|id| *id == resp.id) {
        let n = group.len() as i32;
        let target = group[(i as i32 + step).rem_euclid(n) as usize];
        ui.memory_mut(|m| m.request_focus(target));
        ui.data_mut(|d| d.insert_temp(activate, target));
        ui.ctx().request_repaint();
    }
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
    let (_, mut resp, b) = with_label(ui, pal, text, Sense::click());
    if resp.clicked() {
        *checked = !*checked;
        resp.mark_changed();
    }
    if pal.classic != 0 {
        classic_check(ui, pal, b, &resp, *checked);
        return resp;
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
    if pal.classic != 0 {
        // Classic Windows used a check box for this.
        return checkbox(ui, pal, on, "");
    }
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
        // Keep focus here while the arrow keys adjust the value (egui would move focus).
        let filter = egui::EventFilter { horizontal_arrows: true, vertical_arrows: true, ..Default::default() };
        ui.memory_mut(|m| m.set_focus_lock_filter(resp.id, filter));
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
    if pal.classic != 0 {
        classic_slider(ui, pal, a, b, at, &resp, vertical);
        return resp;
    }
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

/// Classic radio button: a white circle with a gray rim, and a dot when selected.
fn classic_radio(ui: &Ui, pal: &Palette, c: egui::Pos2, resp: &Response, selected: bool) {
    let win7 = pal.classic == 7;
    let p = ui.painter();
    let hot = resp.hovered() || resp.is_pointer_button_down_on();
    let rim = if hot { if win7 { Color32::from_rgb(0x3C, 0x7F, 0xB1) } else { Color32::from_rgb(0x00, 0x78, 0xD7) } } else { Color32::from_rgb(0x8E, 0x8F, 0x8F) };
    let fill = if resp.is_pointer_button_down_on() { Color32::from_rgb(0xD9, 0xEC, 0xFF) } else if hot { Color32::from_rgb(0xEA, 0xF6, 0xFD) } else { Color32::WHITE };
    p.circle(c, 6.5, fill, Stroke::new(1.0, rim));
    if selected {
        let dot = if win7 { Color32::from_rgb(0x1E, 0x5A, 0x96) } else { Color32::from_rgb(0x33, 0x33, 0x33) };
        p.circle_filled(c, 3.2, dot);
        if win7 {
            p.circle_filled(c + vec2(-1.0, -1.0), 1.1, Color32::from_rgb(0x9C, 0xCB, 0xEE));
        }
    }
}

/// Classic check box: a 13 px white square with a gray rim and a check mark.
fn classic_check(ui: &Ui, pal: &Palette, b: Rect, resp: &Response, checked: bool) {
    let win7 = pal.classic == 7;
    let p = ui.painter();
    let bx = Rect::from_center_size(b.center(), vec2(13.0, 13.0));
    let hot = resp.hovered() || resp.is_pointer_button_down_on();
    let rim = if hot { if win7 { Color32::from_rgb(0x3C, 0x7F, 0xB1) } else { Color32::from_rgb(0x00, 0x78, 0xD7) } } else if win7 { Color32::from_rgb(0x8E, 0x8F, 0x8F) } else { Color32::from_rgb(0x33, 0x33, 0x33) };
    let fill = if resp.is_pointer_button_down_on() { Color32::from_rgb(0xD9, 0xEC, 0xFF) } else if hot { Color32::from_rgb(0xEA, 0xF6, 0xFD) } else { Color32::WHITE };
    p.rect_filled(bx, 0.0, fill);
    p.rect_stroke(bx, 0.0, Stroke::new(1.0, rim), egui::StrokeKind::Inside);
    if checked {
        let c = bx.center();
        let ink = if win7 { Color32::from_rgb(0x1E, 0x3A, 0x6A) } else { Color32::from_rgb(0x20, 0x20, 0x20) };
        p.add(egui::Shape::line(vec![c + vec2(-3.5, 0.0), c + vec2(-1.0, 3.0), c + vec2(4.0, -3.5)], Stroke::new(1.7, ink)));
    }
}

/// Classic slider: a thin sunken groove and a small rectangular thumb.
fn classic_slider(ui: &Ui, pal: &Palette, a: egui::Pos2, b: egui::Pos2, at: egui::Pos2, resp: &Response, vertical: bool) {
    let win7 = pal.classic == 7;
    let p = ui.painter();
    let n = if vertical { vec2(1.0, 0.0) } else { vec2(0.0, 1.0) };
    p.line_segment([a - n, b - n], Stroke::new(1.0, Color32::from_rgb(0xA0, 0xA0, 0xA0)));
    p.line_segment([a + n * 0.0, b + n * 0.0], Stroke::new(1.0, Color32::from_rgb(0xE6, 0xE6, 0xE6)));
    p.line_segment([a + n, b + n], Stroke::new(1.0, Color32::WHITE));
    let size = if vertical { vec2(16.0, 8.0) } else { vec2(8.0, 16.0) };
    let thumb = Rect::from_center_size(at, size);
    let hot = resp.hovered() || resp.dragged();
    let (fill, rim) = if win7 {
        (if hot { Color32::from_rgb(0xDC, 0xEE, 0xFB) } else { Color32::from_rgb(0xEC, 0xEC, 0xEC) }, if hot { Color32::from_rgb(0x3C, 0x7F, 0xB1) } else { Color32::from_rgb(0x70, 0x70, 0x70) })
    } else {
        (if hot { Color32::from_rgb(0x17, 0x17, 0x17) } else { Color32::from_rgb(0x00, 0x7A, 0xD9) }, Color32::TRANSPARENT)
    };
    p.rect_filled(thumb, if win7 { 2.0 } else { 0.0 }, fill);
    if rim != Color32::TRANSPARENT {
        p.rect_stroke(thumb, 2.0, Stroke::new(1.0, rim), egui::StrokeKind::Inside);
    }
}

/// Fluent chevron for `ComboBox::icon`.
pub fn combo_chevron(ui: &Ui, rect: Rect, visuals: &egui::style::WidgetVisuals, _open: bool) {
    let c = rect.center();
    if is_classic(ui.visuals()) {
        // Classic combo boxes have a small solid triangle.
        let pts = vec![c + vec2(-3.5, -1.5), c + vec2(3.5, -1.5), c + vec2(0.0, 2.0)];
        ui.painter().add(egui::Shape::convex_polygon(pts, visuals.fg_stroke.color, Stroke::NONE));
        return;
    }
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
    if pal.classic != 0 {
        // Classic Windows marks keyboard focus with a thin dotted rectangle.
        let r = resp.rect.shrink(2.0);
        let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
        p.extend(egui::Shape::dotted_line(&pts, pal.text, 2.0, 0.5));
    } else {
        p.rect_stroke(resp.rect.expand(2.0), 6.0, Stroke::new(2.0, pal.text), egui::StrokeKind::Outside);
    }
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

/// Whether the classic styles' visuals are active (only they draw selected text white
/// on blue; Windows 11 uses the accent color).
fn is_classic(v: &egui::Visuals) -> bool {
    v.selection.stroke.color == Color32::WHITE
}

/// The classic styles keep selected text white on blue, which makes egui draw a focused
/// text box's border white too; this draws it blue again, as in Windows.
pub fn focus_border(ui: &Ui, resp: &Response) {
    let v = ui.visuals();
    if resp.has_focus() && is_classic(v) {
        ui.painter().rect_stroke(resp.rect, v.widgets.inactive.corner_radius, Stroke::new(1.0, v.selection.bg_fill), egui::StrokeKind::Inside);
    }
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
    // Until the user types, the box keeps showing the current value (e.g. while zooming
    // with the wheel), and leaving it changes nothing.
    let typed_id = id.with("typed");
    let typed_before = editing && ui.data(|d| d.get_temp::<bool>(typed_id).unwrap_or(false));
    let mut text = if typed_before { ui.data(|d| d.get_temp::<String>(id)).unwrap_or_else(|| format(*value)) } else { format(*value) };
    let mut resp = ui.add_sized(
        [width, 26.0],
        egui::TextEdit::singleline(&mut text).id(edit_id).horizontal_align(egui::Align::Min).margin(vec2(8.0, 4.0)),
    );
    focus_border(ui, &resp);
    let typed = typed_before || resp.changed();
    // Ask for the focus outside `data_mut`: both take egui's lock, and asking inside
    // would wait on itself forever (the app hung on the first typed key).
    let keep = typed && resp.has_focus();
    ui.data_mut(|d| d.insert_temp(typed_id, keep));
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
    if if live { resp.changed() } else { resp.lost_focus() && typed } {
        let raw = text.trim().trim_end_matches(suffix).trim();
        // "1,920" is a thousands separator; "1,5" a decimal comma.
        let typed = if decimals == 0 || raw.contains('.') || raw.split(',').skip(1).any(|g| g.len() == 3) {
            raw.replace(',', "")
        } else {
            raw.replace(',', ".")
        };
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
