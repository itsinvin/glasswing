//! The pairing window: read the PIN off the console's screen, trade it for the
//! key that wake packets and streaming need, and save that key.

use rune::tok::{radius, space};
use rune::{col, el, row, text, Color, Tx};

use crate::parts;
use crate::state::Ui;
use crate::theme::{type_, Theme};

/// Build the pairing window, a dimmed page with a card on top.
pub fn dialog(s: &Ui, th: &Theme) -> rune::El {
    let host_name = s
        .pair_host_id
        .as_deref()
        .and_then(|id| s.config.hosts.iter().find(|host| host.id == id))
        .map(|host| host.name.clone())
        .unwrap_or_else(|| "the console".to_string());

    let account_saved = !s.psn.text.trim().is_empty();

    let mut card = col()
        .key("pair-dialog")
        .w(470.0)
        .padding(space::S3)
        .gap(space::S2)
        .radius(radius::MD)
        .fill(th.bg)
        .border(1.0, th.line)
        .shadow(20.0, 48.0, th.shadow)
        .role("dialog")
        .label("Pair with a console")
        .modal(true, |s: &mut Ui| {
            if s.pair_busy {
                s.toast(crate::state::ToastKind::Warn, "Pairing is still running");
                return;
            }
            s.pair_open = false;
        })
        .child(text("Pair with a console").font(type_::H2).color(th.text))
        .child(
            text(format!(
                "Pairing {host_name} puts its key on this PC, so you never type a PIN again."
            ))
            .font(type_::SMALL)
            .color(th.faint)
            .wrap(),
        )
        .child(parts::divider(th, "pair-divider"))
        .child(
            col().key("pair-steps").w_full().gap(space::S1).children(
                [
                    "1. On the console, open Settings > Linked Devices > Link Device.",
                    "2. The console shows an 8-digit PIN. Type it here.",
                    "3. Press Pair. The key comes back on its own.",
                ]
                .into_iter()
                .enumerate()
                .map(|(i, words)| {
                    text(words.to_string())
                        .key(format!("pair-step-{i}"))
                        .font(type_::SMALL)
                        .color(th.dim)
                        .wrap()
                }),
            ),
        )
        .child(parts::fact(
            th,
            "pair-account",
            "PSN account",
            if account_saved {
                "saved"
            } else {
                "missing: paste it in Settings"
            },
            !account_saved,
        ))
        .child(parts::field(
            th,
            "pair-pin",
            "PIN from the console",
            &s.pair_pin,
            "12345678",
            true,
            |s: &mut Ui| &mut s.pair_pin,
            |s: &mut Ui, _: &mut Tx| s.pair_failure = None,
        ));

    if s.pair_busy || !s.pair_status.is_empty() {
        card = card.child(
            row()
                .key("pair-status")
                .w_full()
                .gap(space::S2)
                .center_y()
                .child(el().key("pair-status-dot").size(8.0, 8.0).radius(4.0).fill(
                    if s.pair_failure.is_some() {
                        th.bad
                    } else if s.pair_busy {
                        th.warn
                    } else {
                        th.good
                    },
                ))
                .child(
                    text(s.pair_failure.as_deref().unwrap_or(&s.pair_status))
                        .font(type_::SMALL)
                        .color(th.dim)
                        .wrap(),
                ),
        );
    }

    card = card.child(
        row()
            .key("pair-actions")
            .w_full()
            .mt(space::S1)
            .gap(space::S2)
            .justify_end()
            .child(parts::button(
                th,
                "pair-cancel",
                "Close",
                false,
                false,
                |s: &mut Ui| s.pair_open = false,
            ))
            .child(parts::button(
                th,
                "pair-start",
                if s.pair_busy { "Pairing…" } else { "Pair" },
                true,
                s.pair_busy,
                |s: &mut Ui| crate::state::start_pairing(s),
            )),
    );

    el().key("pair-scrim")
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
        .fill(Color::BLACK.alpha(0.45))
        .center()
        .child(card)
}
