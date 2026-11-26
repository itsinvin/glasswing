//! The Consoles page: find consoles on the network, save the ones you want, wake them.

use glasswing_core::discovery::ConsoleState;
use rune::tok::{radius, space};
use rune::{col, el, row, text, Color};

use crate::parts;
use crate::state::{Row, Ui};
use crate::theme::{type_, Theme};

/// Build the page.
pub fn view(s: &Ui, th: &Theme) -> Vec<rune::El> {
    let rows = s.rows();

    let actions = row()
        .key("console-actions")
        .gap(space::S2)
        .child(parts::button(
            th,
            "scan",
            if s.scanning {
                "Scanning…"
            } else {
                "Scan again"
            },
            true,
            s.scanning,
            |s: &mut Ui| crate::state::rescan(s),
        ))
        .child(parts::button(
            th,
            "add",
            "Add console",
            false,
            false,
            |s: &mut Ui| {
                s.add_open = true;
                s.add_error = None;
            },
        ));

    let mut out: Vec<rune::El> = vec![row()
        .key("consoles-head")
        .w_full()
        .gap(space::S3)
        .center_y()
        .justify_between()
        .child(parts::hint_line(th, "consoles-hint", &scan_line(s)))
        .child(actions)];

    if rows.is_empty() {
        out.push(parts::empty(
            th,
            "consoles-empty",
            "No consoles yet",
            if s.scanning {
                "Looking for consoles on this network…"
            } else {
                "Make sure the PS5 is on and on the same network, then press Scan again. You can also add it by address."
            },
        ));
        return out;
    }

    out.extend(
        rows.iter()
            .enumerate()
            .map(|(i, row_data)| card(s, th, &format!("console-{i}"), row_data)),
    );

    out.push(parts::card(th, "consoles-help").child(parts::card_head(
        th,
        "consoles-help-head",
        "How this works",
        "Glasswing sends one small packet to the network and consoles answer with their name, address and state. Nothing leaves your network.",
    )));

    out
}

/// "Scanned just now", "Scanned 12s ago", or a hint that no scan has run.
fn scan_line(s: &Ui) -> String {
    if s.scanning {
        return "Scanning the network…".to_string();
    }
    match s.last_scan {
        Some(instant) => {
            let secs = instant.elapsed().as_secs();
            if secs < 5 {
                "Scanned just now".to_string()
            } else if secs < 60 {
                format!("Scanned {secs} seconds ago")
            } else {
                format!("Scanned {} minutes ago", secs / 60)
            }
        }
        None => "Scan runs when the window opens".to_string(),
    }
}

/// The colour and words for a console's state.
fn state_of(row_data: &Row, th: &Theme) -> (Color, &'static str) {
    match &row_data.live {
        Some(live) => match live.state {
            ConsoleState::Ready => (th.good, "Ready"),
            ConsoleState::Standby => (th.warn, "Rest mode"),
            ConsoleState::Unknown => (th.faint, "Answering"),
        },
        None => (th.faint, "Not seen yet"),
    }
}

/// One console, one card.
fn card(s: &Ui, th: &Theme, key: &str, row_data: &Row) -> rune::El {
    let (color, words) = state_of(row_data, th);

    let mut meta = format!("{} · {}", type_word(row_data), row_data.address());
    if let Some(live) = &row_data.live {
        if !live.system_version.is_empty() {
            meta.push_str(" · ");
            meta.push_str(&live.system_version);
        }
        if let Some(title) = &live.running_app_name {
            meta.push_str(" · playing ");
            meta.push_str(title);
        }
    }

    let address = row_data.address().to_string();

    let mut actions = row()
        .key(format!("{key}-actions"))
        .gap(space::S2)
        .center_y()
        .child(parts::button(
            th,
            format!("{key}-wake"),
            "Wake",
            false,
            false,
            {
                let address = address.clone();
                move |s: &mut Ui| {
                    let regist_key = s
                        .config
                        .hosts
                        .iter()
                        .find(|host| host.address == address)
                        .and_then(|host| host.regist_key.clone());
                    crate::state::spawn_wake(address.clone(), regist_key, s.handle.clone());
                }
            },
        ));

    match &row_data.host {
        Some(host) => {
            if host.regist_key.is_none() {
                actions = actions
                    .child(parts::button(
                        th,
                        format!("{key}-pair"),
                        "Pair",
                        false,
                        false,
                        {
                            let id = host.id.clone();
                            move |s: &mut Ui| crate::state::open_pairing(s, id.clone())
                        },
                    ))
                    .child(
                        text("No pairing key")
                            .key(format!("{key}-nokey"))
                            .font(type_::SMALL)
                            .color(th.faint),
                    );
            }
            let id = host.id.clone();
            let confirm = s.confirm_remove.as_deref() == Some(id.as_str());
            let label = if confirm {
                "Remove for good?"
            } else {
                "Remove"
            };
            actions = actions.child(parts::button(
                th,
                format!("{key}-remove"),
                label,
                false,
                false,
                {
                    let id = id.clone();
                    move |s: &mut Ui| {
                        if s.confirm_remove.as_deref() == Some(id.as_str()) {
                            crate::state::remove_host(s, id.clone());
                            s.confirm_remove = None;
                        } else {
                            s.confirm_remove = Some(id.clone());
                        }
                    }
                },
            ));
        }
        None => {
            if let Some(live) = &row_data.live {
                actions = actions.child(parts::button(
                    th,
                    format!("{key}-save"),
                    "Save",
                    false,
                    false,
                    {
                        let live = live.clone();
                        move |s: &mut Ui| crate::state::save_console(s, live.clone())
                    },
                ));
            }
        }
    }

    row()
        .key(key)
        .w_full()
        .gap(space::S3)
        .py(space::S3)
        .px(space::S3)
        .radius(radius::MD)
        .fill(th.card)
        .border(1.0, th.line)
        .center_y()
        .child(
            el().key(format!("{key}-dot"))
                .size(10.0, 10.0)
                .radius(5.0)
                .fill(color),
        )
        .child(
            col()
                .key(format!("{key}-words"))
                .grow(1.0)
                .min_w(0.0)
                .gap(2.0)
                .child(
                    text(row_data.name().to_string())
                        .key(format!("{key}-name"))
                        .font(type_::BODY_STRONG)
                        .color(th.text),
                )
                .child(
                    text(meta)
                        .key(format!("{key}-meta"))
                        .font(type_::MONO)
                        .color(th.faint)
                        .truncate(),
                )
                .child(
                    text(words)
                        .key(format!("{key}-state"))
                        .font(type_::SMALL)
                        .color(color),
                ),
        )
        .child(actions)
}

/// "Console", "Added by hand", or whatever the console says it is.
fn type_word(row_data: &Row) -> String {
    match &row_data.host {
        Some(host) if host.manual => "Added by hand".to_string(),
        _ => match &row_data.live {
            Some(live) if !live.host_type.is_empty() => live.host_type.clone(),
            _ => "Console".to_string(),
        },
    }
}
