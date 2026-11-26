//! The About page: what Glasswing is, what works, and who to thank.

use rune::tok::space;
use rune::{col, row, text, Tx};

use crate::parts;
use crate::state::Ui;
use crate::theme::{type_, Theme};

/// Build the page.
pub fn view(_: &Ui, th: &Theme) -> Vec<rune::El> {
    let status = [
        ("status-find", "Find consoles on the network", "Working"),
        ("status-wake", "Wake a console from rest mode", "Working"),
        ("status-save", "Save consoles and settings", "Working"),
        ("status-pair", "Pair with a PSN account", "In progress"),
        ("status-stream", "Video, audio and input", "Planned"),
        (
            "status-dualsense",
            "Full DualSense, haptics and triggers",
            "Planned",
        ),
    ]
    .into_iter()
    .map(|(key, label, state)| status_row(th, key, label, state))
    .collect::<Vec<_>>();

    vec![
        parts::card(th, "about-card")
            .child(
                col()
                    .key("about-words")
                    .w_full()
                    .gap(space::S2)
                    .child(text("Glasswing").font(type_::H1).color(th.text))
                    .child(
                        text("A PlayStation 5 Remote Play client for Windows, written in Rust. The window is drawn by the GPU with Rune, there is no webview, and the path from network to screen is built to keep the delay small.")
                            .font(type_::BODY)
                            .color(th.dim)
                            .wrap(),
                    )
                    .child(
                        text("The first version aims at one thing: a sharp 1080p60 stream over your own network, with a DualSense that behaves like the real controller.")
                            .font(type_::BODY)
                            .color(th.dim)
                            .wrap(),
                    ),
            )
            .child(parts::divider(th, "about-divider"))
            .child(parts::card_head(
                th,
                "about-status-head",
                "What works today",
                "Discovery, wake, settings and the system check run now. Pairing, the video pipeline and controller support are the next steps.",
            ))
            .child(col().key("about-status-list").w_full().gap(space::S1).children(status)),
        parts::card(th, "thanks-card")
            .child(parts::card_head(
                th,
                "thanks-head",
                "Thanks",
                "Glasswing follows the discovery and wake protocols these projects describe, and keeps an AGPL-3.0 license so their work stays free too.",
            ))
            .child(parts::fact(th, "chiaki", "chiaki-ng", "Protocol reference, AGPL-3.0", false))
            .child(parts::fact(th, "kyber", "KyberNow", "Protocol reference, AGPL-3.0", false))
            .child(parts::divider(th, "thanks-divider"))
            .child(
                text("PlayStation is a trademark of Sony Interactive Entertainment. Glasswing is not made by or endorsed by Sony.")
                    .font(type_::SMALL)
                    .color(th.faint)
                    .wrap(),
            ),
        parts::card(th, "license-card")
            .child(parts::card_head(
                th,
                "license-head",
                "License and source",
                "AGPL-3.0. The source is public, and so are the performance notes.",
            ))
            .child(parts::fact(th, "version", "Version", glasswing_core::VERSION, false))
            .child(parts::fact(th, "license", "License", "AGPL-3.0-only", false))
            .child(parts::link_row(
                th,
                "about-repo",
                "github.com/itsinvin/glasswing",
                |_: &mut Ui, tx: &mut Tx| {
                    let _ = tx.open_url("https://github.com/itsinvin/glasswing");
                },
            )),
    ]
}

/// One line of "what works today".
fn status_row(th: &Theme, key: &str, label: &str, state: &str) -> rune::El {
    let (color, words) = match state {
        "Working" => (th.good, "Working"),
        "In progress" => (th.warn, "In progress"),
        _ => (th.faint, "Planned"),
    };
    row()
        .key(format!("{key}-row"))
        .w_full()
        .gap(space::S2)
        .center_y()
        .child(parts::badge(key, words, color).w(96.0).justify_center())
        .child(text(label).font(type_::BODY).color(th.dim).wrap())
}
