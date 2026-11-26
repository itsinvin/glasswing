//! Small pieces every page is built from: nav rows, cards, buttons, switches, fields, the
//! bitrate slider, toasts and the add-console window.

use std::rc::Rc;

use rune::tok::{motion, radius, space};
use rune::{col, el, row, text, text_input, Color, Drag, El, PointerButton, TextEdit, Tx};

use crate::state::{Toast, ToastKind, Ui};
use crate::theme::{type_, Theme};

/// The near-white used for text and knobs on top of the accent colour.
const KNOB: Color = Color::WHITE;

/// The slider knob's diameter in points.
const KNOB_SIZE: f32 = 16.0;

// ------------------------------------------------------------------ nav and links

/// One row in the sidebar.
pub fn nav_row(
    th: &Theme,
    key: &'static str,
    label: &str,
    active: bool,
    on_press: impl Fn(&mut Ui) + 'static,
) -> El {
    let nav = row()
        .key(key)
        .w_full()
        .h(34.0)
        .px(space::S2)
        .center_y()
        .gap(space::S2)
        .radius(radius::SM)
        .role("button")
        .label(label)
        .focusable(true)
        .hit()
        .hover(th.hover)
        .press(th.press)
        .fill(if active { th.well } else { Color::TRANSPARENT })
        .on_press(on_press)
        .child(
            el().key(format!("{key}-mark"))
                .w(3.0)
                .h(16.0)
                .radius(1.5)
                .fill(if active {
                    th.accent
                } else {
                    Color::TRANSPARENT
                }),
        )
        .child(
            text(label)
                .font(type_::LABEL)
                .color(if active { th.text } else { th.dim }),
        );
    nav
}

/// A quiet text button, for footer and header links.
pub fn link_row(
    th: &Theme,
    key: &'static str,
    label: &str,
    on_press: impl Fn(&mut Ui, &mut Tx) + 'static,
) -> El {
    row()
        .key(key)
        .w_full()
        .h(30.0)
        .px(space::S2)
        .center_y()
        .gap(space::S2)
        .radius(radius::SM)
        .role("button")
        .label(label)
        .focusable(true)
        .hit()
        .hover(th.hover)
        .press(th.press)
        .on_press_tx(on_press)
        .child(text(label).font(type_::SMALL).color(th.dim))
}

// ------------------------------------------------------------------ containers

/// A rounded panel with a hairline edge.
pub fn card(th: &Theme, key: &'static str) -> El {
    col()
        .key(key)
        .w_full()
        .radius(radius::MD)
        .fill(th.card)
        .border(1.0, th.line)
        .padding(space::S3)
        .gap(space::S3)
}

/// The title and hint at the top of a card.
pub fn card_head(th: &Theme, key: &'static str, title: &str, hint: &str) -> El {
    col()
        .key(key)
        .w_full()
        .gap(2.0)
        .child(text(title).font(type_::H2).color(th.text))
        .child(text(hint).font(type_::SMALL).color(th.faint).wrap())
}

/// The quiet line under the header's page title, for pages that carry their own heading.
pub fn hint_line(th: &Theme, key: &'static str, words: &str) -> El {
    text(words)
        .key(key)
        .font(type_::SMALL)
        .color(th.faint)
        .wrap()
        .grow(1.0)
        .min_w(0.0)
}

/// A coloured pill, carrying its own key so callers can hand it a name.
pub fn badge(key: &str, label: impl Into<String>, color: Color) -> El {
    let label = label.into();
    row()
        .key(format!("badge-{key}"))
        .h(20.0)
        .px(space::S2)
        .center_y()
        .gap(space::S1)
        .radius(10.0)
        .fill(color.alpha(0.16))
        .border(1.0, color.alpha(0.38))
        .child(
            el().key(format!("badge-{key}-dot"))
                .size(6.0, 6.0)
                .radius(3.0)
                .fill(color),
        )
        .child(text(label).font(type_::SMALL).color(color))
}

/// A thin divider.
pub fn divider(th: &Theme, key: &'static str) -> El {
    el().key(format!("divider-{key}"))
        .w_full()
        .h(1.0)
        .fill(th.line)
}

/// A row of label and value, used for paths and small facts.
pub fn fact(th: &Theme, key: &str, label: &str, value: &str, mono: bool) -> El {
    row()
        .key(format!("fact-{key}"))
        .w_full()
        .gap(space::S2)
        .center_y()
        .child(text(label).font(type_::SMALL).color(th.faint))
        .child(
            text(value)
                .font(if mono { type_::MONO } else { type_::SMALL })
                .color(th.dim)
                .selectable(true),
        )
}

/// The empty state for a page with nothing to show yet.
pub fn empty(th: &Theme, key: &'static str, headline: &str, hint: &str) -> El {
    col()
        .key(key)
        .w_full()
        .py(space::S6)
        .gap(space::S1)
        .center()
        .child(text(headline).font(type_::H2).color(th.dim))
        .child(
            text(hint)
                .font(type_::SMALL)
                .color(th.faint)
                .wrap()
                .max_w(420.0),
        )
}

// ------------------------------------------------------------------ controls

/// A pressable button.
#[allow(clippy::too_many_arguments)]
pub fn button(
    th: &Theme,
    key: impl Into<String>,
    label: &str,
    primary: bool,
    disabled: bool,
    on_press: impl Fn(&mut Ui) + 'static,
) -> El {
    let ink = if primary { KNOB } else { th.text };
    let key = key.into();
    let b = row()
        .key(key)
        .h(32.0)
        .px(space::S3)
        .center_y()
        .justify_center()
        .radius(radius::SM)
        .role("button")
        .label(label)
        .focusable(true)
        .hit()
        .fill(if primary { th.accent } else { th.well })
        .border(1.0, if primary { th.accent } else { th.line })
        .hover(Color::WHITE.alpha(0.08))
        .press(Color::WHITE.alpha(0.16));
    let b = if disabled {
        b.fill(th.well)
            .border(1.0, th.line)
            .aria_disabled(true)
            .disabled(true)
    } else {
        b.on_press(on_press)
    };
    b.child(
        text(label)
            .font(type_::LABEL)
            .color(if disabled { th.faint } else { ink }),
    )
}

/// A switch for a yes/no setting, with its title and hint beside it.
pub fn toggle(
    th: &Theme,
    key: impl Into<String>,
    title: &str,
    hint: &str,
    on: bool,
    on_press: impl Fn(&mut Ui) + 'static,
) -> El {
    let key = key.into();
    let knob = el()
        .key(format!("{key}-knob"))
        .size(16.0, 16.0)
        .radius(8.0)
        .fill(KNOB)
        .absolute(if on { 20.0 } else { 2.0 }, 2.0);
    let track = el()
        .key(format!("{key}-track"))
        .size(38.0, 22.0)
        .radius(11.0)
        .fill(if on { th.accent } else { th.track })
        .role("switch")
        .label(title)
        .checked(on)
        .focusable(true)
        .hit()
        .press_fill(if on {
            th.accent.alpha(0.85)
        } else {
            Color::WHITE.alpha(0.16)
        })
        .on_press(on_press)
        .child(knob)
        .motion(motion::SNAPPY);
    row()
        .key(key.clone())
        .w_full()
        .gap(space::S3)
        .py(space::S2)
        .px(space::S2)
        .center_y()
        .radius(radius::SM)
        .fill(th.well)
        .border(1.0, th.line)
        .child(
            col()
                .key(format!("{key}-words"))
                .grow(1.0)
                .gap(2.0)
                .child(text(title).font(type_::BODY_STRONG).color(th.text))
                .child(text(hint).font(type_::SMALL).color(th.faint).wrap()),
        )
        .child(track)
}

/// A set of side-by-side choices where one is picked, such as 30 or 60 fps.
pub fn segmented(
    th: &Theme,
    key: &'static str,
    options: &'static [(&'static str, u32)],
    current: u32,
    on_pick: fn(&mut Ui, u32),
) -> El {
    let choices: Vec<El> = options
        .iter()
        .map(|(label, value)| {
            let picked = *value == current;
            row()
                .key(format!("{key}-{value}"))
                .h(30.0)
                .grow(1.0)
                .center_y()
                .justify_center()
                .radius(radius::SM)
                .fill(if picked {
                    th.accent_tint
                } else {
                    Color::TRANSPARENT
                })
                .border(1.0, if picked { th.accent } else { th.line })
                .role("button")
                .label(*label)
                .focusable(true)
                .hit()
                .hover(th.hover)
                .press(th.press)
                .on_press(move |s: &mut Ui| on_pick(s, *value))
                .child(
                    text(*label)
                        .font(type_::LABEL)
                        .color(if picked { th.text } else { th.dim }),
                )
        })
        .collect();

    row()
        .key(key)
        .w_full()
        .h(32.0)
        .gap(2.0)
        .padding(1.0)
        .radius(radius::SM)
        .fill(th.well)
        .border(1.0, th.line)
        .children(choices)
}

/// A labelled text field. `on_edit` hands Rune the field's text, `on_change` hears about typing.
#[allow(clippy::too_many_arguments)]
pub fn field(
    th: &Theme,
    key: impl Into<String>,
    label: &str,
    edit: &TextEdit,
    placeholder: &str,
    monospace: bool,
    on_edit: impl for<'a> Fn(&'a mut Ui) -> &'a mut TextEdit + 'static,
    on_change: impl Fn(&mut Ui, &mut Tx) + 'static,
) -> El {
    let key = key.into();
    col()
        .key(key.clone())
        .w_full()
        .gap(space::S1)
        .child(text(label).font(type_::LABEL).color(th.dim))
        .child(
            text_input(edit, if monospace { type_::MONO } else { type_::BODY })
                .key(format!("{key}-input"))
                .single_line()
                .label(label)
                .w_full()
                .h(34.0)
                .px(space::S2)
                .radius(radius::SM)
                .fill(th.well)
                .border(1.0, th.line)
                .focus_border(th.accent)
                .color(th.text)
                .placeholder(placeholder, th.faint)
                .on_edit(on_edit)
                .on_input_change(on_change),
        )
}

/// What a slider's track calls when it moves: the new number, and the frame to commit it in.
type SetValue = Rc<dyn Fn(&mut Ui, u32, &mut Tx)>;

/// A slider: drag the knob, or click anywhere on the track, to pick a number from `min` to `max`.
///
/// `rect` is the track's box in window points from the last layout, so a press knows where it
/// landed along the track.
#[allow(clippy::too_many_arguments)]
pub fn slider(
    th: &Theme,
    key: &'static str,
    label: &str,
    value: u32,
    min: u32,
    max: u32,
    rect: [f32; 4],
    on_edit: impl Fn(&mut Ui, u32, &mut Tx) + 'static,
) -> El {
    let set: SetValue = Rc::new(on_edit);
    let span = ((max - min) as f32).max(1.0);
    let frac = (((value - min) as f32) / span).clamp(0.0, 1.0);
    let knob_x = frac * (rect[2] - KNOB_SIZE).max(0.0);

    let (set_press, set_drag) = (Rc::clone(&set), set);
    let (rect_press, rect_drag) = (rect, rect);

    row()
        .key(key)
        .w_full()
        .h(20.0)
        .role("slider")
        .label(label)
        .range_value(f64::from(value), f64::from(min), f64::from(max))
        .a11y_value(format!("{value} Mbps"))
        .on_layout(|s: &mut Ui, r| s.slider_rect = r)
        .on_any_press(move |s: &mut Ui, button: PointerButton, p, tx: &mut Tx| {
            if button == PointerButton::Primary {
                let next = value_at(rect_press, p.window_x, min, max);
                set_press(s, next, tx);
                tx.instant();
            }
        })
        .on_drag(move |s: &mut Ui, drag: &mut Drag, tx: &mut Tx| {
            let x = drag.window_point()[0];
            let next = value_at(rect_drag, x, min, max);
            set_drag(s, next, tx);
            tx.instant();
        })
        .child(
            el().key(format!("{key}-track"))
                .absolute(0.0, 8.0)
                .w_full()
                .h(6.0)
                .radius(3.0)
                .fill(th.track),
        )
        .child(
            el().key(format!("{key}-fill"))
                .absolute(0.0, 8.0)
                .w_frac(frac)
                .h(6.0)
                .radius(3.0)
                .fill(th.accent),
        )
        .child(
            el().key(format!("{key}-knob"))
                .absolute(knob_x, 2.0)
                .size(KNOB_SIZE, KNOB_SIZE)
                .radius(KNOB_SIZE / 2.0)
                .fill(KNOB)
                .shadow(1.0, 3.0, th.shadow),
        )
}

/// The number a press at `window_x` picks, given the track's box in window points.
fn value_at(rect: [f32; 4], window_x: f32, min: u32, max: u32) -> u32 {
    let frac = if rect[2] > 0.0 {
        ((window_x - rect[0]) / rect[2]).clamp(0.0, 1.0)
    } else {
        0.0
    };
    ((min as f32) + frac * ((max - min) as f32))
        .round()
        .clamp(min as f32, max as f32) as u32
}

// ------------------------------------------------------------------ toasts

/// A short message that fades away on its own.
pub fn toast(th: &Theme, toast: &Toast) -> El {
    let color = match toast.kind {
        ToastKind::Good => th.good,
        ToastKind::Warn => th.warn,
        ToastKind::Bad => th.bad,
    };
    row()
        .key("toast-row")
        .w_full()
        .px(space::S6)
        .pb(space::S3)
        .justify_center()
        .child(
            row()
                .key("toast")
                .max_w(560.0)
                .px(space::S3)
                .py(space::S2)
                .gap(space::S2)
                .center_y()
                .radius(radius::SM)
                .fill(th.card)
                .border(1.0, th.line)
                .shadow(10.0, 30.0, th.shadow)
                .child(el().key("toast-dot").size(8.0, 8.0).radius(4.0).fill(color))
                .child(
                    text(toast.message.clone())
                        .font(type_::SMALL)
                        .color(th.text)
                        .wrap(),
                ),
        )
}

// ------------------------------------------------------------------ add console

/// The add-console window: a dimmed page with a card on top.
pub fn add_dialog(s: &Ui, th: &Theme) -> El {
    let error = s.add_error.as_ref().map(|message| {
        text(message.clone())
            .font(type_::SMALL)
            .color(th.bad)
            .wrap()
    });

    el()
        .key("scrim")
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
        .fill(Color::BLACK.alpha(0.45))
        .center()
        .child(
            col()
                .key("add-dialog")
                .w(430.0)
                .padding(space::S3)
                .gap(space::S2)
                .radius(radius::MD)
                .fill(th.bg)
                .border(1.0, th.line)
                .shadow(20.0, 48.0, th.shadow)
                .role("dialog")
                .label("Add a console")
                .modal(true, |s: &mut Ui| s.add_open = false)
                .child(text("Add a console").font(type_::H2).color(th.text))
                .child(
                    text("Type the console's address, then save. From now on a scan keeps it up to date.")
                        .font(type_::SMALL)
                        .color(th.faint)
                        .wrap(),
                )
                .child(field(
                    th,
                    "add-name",
                    "Name",
                    &s.add_name,
                    "Living room PS5",
                    false,
                    |s: &mut Ui| &mut s.add_name,
                    |s: &mut Ui, _: &mut Tx| s.add_error = None,
                ))
                .child(field(
                    th,
                    "add-address",
                    "Address",
                    &s.add_address,
                    "192.168.1.20",
                    true,
                    |s: &mut Ui| &mut s.add_address,
                    |s: &mut Ui, _: &mut Tx| s.add_error = None,
                ))
                .child(field(
                    th,
                    "add-key",
                    "Pairing key (optional)",
                    &s.add_key,
                    "Filled in after pairing",
                    true,
                    |s: &mut Ui| &mut s.add_key,
                    |s: &mut Ui, _: &mut Tx| s.add_error = None,
                ))
                .child(
                    text("Leave the key empty for now. Pairing is the next milestone.")
                        .font(type_::SMALL)
                        .color(th.faint)
                        .wrap(),
                )
                .children(error)
                .child(
                    row()
                        .key("add-actions")
                        .w_full()
                        .mt(space::S1)
                        .gap(space::S2)
                        .justify_end()
                        .child(button(
                            th,
                            "add-cancel",
                            "Cancel",
                            false,
                            false,
                            |s: &mut Ui| s.add_open = false,
                        ))
                        .child(button(
                            th,
                            "add-save",
                            "Save console",
                            true,
                            false,
                            |s: &mut Ui| crate::state::commit_add(s),
                        )),
                ),
        )
}
