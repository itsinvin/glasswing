//! The Settings page: the PSN account, stream defaults, performance switches and the system
//! check. Edits sit in a draft and only reach disk on Save.

use rune::tok::{radius, space};
use rune::{col, row, text, Tx};

use crate::parts;
use crate::state::Ui;
use crate::theme::{type_, Theme};

/// Build the page.
pub fn view(s: &Ui, th: &Theme) -> Vec<rune::El> {
    let dirty = s.draft_edited && s.draft != s.config;

    let head = row()
        .key("settings-head")
        .w_full()
        .gap(space::S3)
        .center_y()
        .justify_between()
        .child(parts::hint_line(
            th,
            "settings-hint",
            "Choose how the first stream should look and how the PC should work. Changes save to this PC only.",
        ))
        .child(
            row()
                .key("settings-actions")
                .gap(space::S2)
                .center_y()
                .children(dirty.then(|| parts::badge("dirty", "Unsaved changes", th.warn)))
                .child(parts::button(
                    th,
                    "settings-save",
                    "Save",
                    true,
                    !dirty,
                    |s: &mut Ui| crate::state::save_settings(s),
                )),
        );

    let resolution = if s.draft.stream.resolution == "720p" {
        720
    } else {
        1080
    };

    vec![
        head,
        parts::card(th, "account-card")
            .child(parts::card_head(
                th,
                "account-head",
                "PlayStation account",
                "Used to pair with your own PS5. It stays on this PC.",
            ))
            .child(parts::field(
                th,
                "psn-account",
                "PSN account ID",
                &s.psn,
                "Paste the base64 ID",
                true,
                |s: &mut Ui| &mut s.psn,
                |s: &mut Ui, _: &mut Tx| {
                    let typed = s.psn.text.trim().to_string();
                    s.draft.psn_account_id = (!typed.is_empty()).then_some(typed);
                    s.draft_edited = true;
                },
            ))
            .child(
                text("Pairing is the next milestone. You can paste it now; the app will use it then.")
                    .font(type_::SMALL)
                    .color(th.faint)
                    .wrap(),
            ),
        parts::card(th, "stream-card")
            .child(parts::card_head(
                th,
                "stream-head",
                "Stream defaults",
                "These are the settings for the first playable stream.",
            ))
            .child(
                row()
                    .key("stream-grid")
                    .w_full()
                    .gap(space::S3)
                    .child(
                        col()
                            .key("resolution-col")
                            .grow(1.0)
                            .gap(space::S1)
                            .child(text("Resolution").font(type_::LABEL).color(th.dim))
                            .child(parts::segmented(
                                th,
                                "resolution",
                                &[("1080p", 1080), ("720p", 720)],
                                resolution,
                                pick_resolution,
                            )),
                    )
                    .child(
                        col()
                            .key("fps-col")
                            .grow(1.0)
                            .gap(space::S1)
                            .child(text("Frame rate").font(type_::LABEL).color(th.dim))
                            .child(parts::segmented(
                                th,
                                "fps",
                                &[("30 fps", 30), ("60 fps", 60)],
                                s.draft.stream.fps,
                                pick_fps,
                            )),
                    ),
            )
            .child(
                col()
                    .key("bitrate-col")
                    .w_full()
                    .gap(space::S1)
                    .child(
                        row()
                            .key("bitrate-label")
                            .w_full()
                            .justify_between()
                            .center_y()
                            .child(text("Bitrate").font(type_::LABEL).color(th.dim))
                            .child(
                                text(format!("{} Mbps", s.draft.stream.bitrate_mbps))
                                    .font(type_::MONO)
                                    .color(th.dim),
                            ),
                    )
                    .child(parts::slider(
                        th,
                        "bitrate",
                        "Bitrate",
                        s.draft.stream.bitrate_mbps,
                        5,
                        50,
                        s.slider_rect,
                        |s: &mut Ui, value: u32, tx: &mut Tx| {
                            s.draft.stream.bitrate_mbps = value;
                            s.draft_edited = true;
                            tx.instant();
                        },
                    ))
                    .child(
                        text("Higher is sharper and uses more network. 15 Mbps is a good start for 1080p60.")
                            .font(type_::SMALL)
                            .color(th.faint)
                            .wrap(),
                    ),
            )
            .child(parts::divider(th, "stream-divider"))
            .child(parts::fact(th, "codec", "Codec", "H.264 (every PS5)", false))
            .child(parts::fact(th, "hdr", "HDR", "Coming after the first SDR stream", false)),
        parts::card(th, "performance-card")
            .child(parts::card_head(
                th,
                "performance-head",
                "Performance",
                "NVIDIA decode and a short present path keep the delay low.",
            ))
            .child(parts::toggle(
                th,
                "prefer-nvidia",
                "Prefer NVIDIA decode",
                "Use NVDEC or D3D11VA when the GPU supports it.",
                s.draft.performance.prefer_nvidia,
                |s: &mut Ui| {
                    s.draft.performance.prefer_nvidia = !s.draft.performance.prefer_nvidia;
                    s.draft_edited = true;
                },
            ))
            .child(parts::toggle(
                th,
                "low-latency-present",
                "Low-latency present",
                "Skip extra frame queues where the driver allows it.",
                s.draft.performance.low_latency_present,
                |s: &mut Ui| {
                    s.draft.performance.low_latency_present = !s.draft.performance.low_latency_present;
                    s.draft_edited = true;
                },
            ))
            .child(parts::divider(th, "performance-divider"))
            .child(system_check(s, th))
            .child(notes_link(th)),
        parts::card(th, "data-card")
            .child(parts::card_head(
                th,
                "data-head",
                "Data",
                "Everything stays on this PC. No cloud account is used.",
            ))
            .child(
                row()
                    .key("data-paths")
                    .w_full()
                    .gap(space::S3)
                    .child(
                        col()
                            .key("settings-path-col")
                            .grow(1.0)
                            .min_w(0.0)
                            .gap(space::S1)
                            .child(text("Settings file").font(type_::LABEL).color(th.dim))
                            .child(
                                text(s.config_path.display().to_string())
                                    .font(type_::MONO)
                                    .color(th.faint)
                                    .selectable(true)
                                    .wrap(),
                            )
                            .child(parts::link_row(
                                th,
                                "show-settings-path",
                                "Show in Explorer",
                                |s: &mut Ui, tx: &mut Tx| {
                                    let path = s.config_path.clone();
                                    let _ = tx.reveal_path(path);
                                },
                            )),
                    )
                    .child(
                        col()
                            .key("logs-path-col")
                            .grow(1.0)
                            .min_w(0.0)
                            .gap(space::S1)
                            .child(text("Logs").font(type_::LABEL).color(th.dim))
                            .child(
                                text(glasswing_core::config::log_dir().display().to_string())
                                    .font(type_::MONO)
                                    .color(th.faint)
                                    .selectable(true)
                                    .wrap(),
                            )
                            .child(parts::link_row(
                                th,
                                "show-logs-path",
                                "Show in Explorer",
                                |_: &mut Ui, tx: &mut Tx| {
                                    let path = glasswing_core::config::log_dir();
                                    let _ = tx.reveal_path(path);
                                },
                            )),
                    ),
            )
            .child(parts::fact(th, "version", "Version", glasswing_core::VERSION, false))
            .child(parts::fact(th, "license", "License", "AGPL-3.0", false)),
    ]
}

/// Pick 1080p or 720p.
fn pick_resolution(s: &mut Ui, value: u32) {
    s.draft.stream.resolution = if value == 720 { "720p" } else { "1080p" }.to_string();
    s.draft_edited = true;
}

/// Pick 30 or 60 fps.
fn pick_fps(s: &mut Ui, value: u32) {
    s.draft.stream.fps = value;
    s.draft_edited = true;
}

/// "FFmpeg 8.0.1" or "FFmpeg missing".
fn ffmpeg_label(found: &Option<String>) -> String {
    match found {
        Some(version) => format!("FFmpeg {version}"),
        None => "FFmpeg missing".to_string(),
    }
}

/// The system check: the GPU, whether NVDEC and FFmpeg are there, and a refresh button.
fn system_check(s: &Ui, th: &Theme) -> rune::El {
    let mut body = col()
        .key("system-body")
        .w_full()
        .py(space::S2)
        .px(space::S2)
        .gap(space::S2)
        .radius(radius::SM)
        .fill(th.well)
        .border(1.0, th.line);

    body = body.child(
        row()
            .key("system-head")
            .w_full()
            .justify_between()
            .center_y()
            .child(text("System check").font(type_::LABEL).color(th.dim))
            .child(parts::button(
                th,
                "system-refresh",
                "Refresh",
                false,
                s.system_loading,
                |s: &mut Ui| crate::state::refresh_system(s),
            )),
    );

    if s.system_loading && s.system.is_none() {
        return body.child(text("Checking this PC…").font(type_::SMALL).color(th.faint));
    }

    match &s.system {
        None => body.child(
            text("Could not run the check.")
                .font(type_::SMALL)
                .color(th.warn),
        ),
        Some(info) => body
            .child(
                col()
                    .key("system-gpu")
                    .w_full()
                    .gap(2.0)
                    .child(text("GPU").font(type_::SMALL).color(th.faint))
                    .child(
                        text(match &info.gpu {
                            Some(gpu) => gpu.name.clone(),
                            None => "No NVIDIA GPU found".to_string(),
                        })
                        .font(type_::BODY_STRONG)
                        .color(th.text)
                        .wrap(),
                    )
                    .children(
                        info.gpu
                            .as_ref()
                            .map(|gpu| {
                                text(format!("Driver {} · {}", gpu.driver, gpu.memory))
                                    .font(type_::MONO)
                                    .color(th.faint)
                                    .wrap()
                            }),
                    ),
            )
            .child(
                row()
                    .key("system-badges")
                    .gap(space::S1)
                    .child(parts::badge(
                        "nvdec",
                        if info.nvdec { "NVDEC ready" } else { "NVDEC missing" },
                        if info.nvdec { th.good } else { th.warn },
                    ))
                    .child(parts::badge(
                        "ffmpeg",
                        ffmpeg_label(&info.ffmpeg),
                        if info.ffmpeg.is_some() { th.good } else { th.warn },
                    )),
            )
            .child(
                text("Video decoding is not built yet. This check makes sure the PC is ready for it.")
                    .font(type_::SMALL)
                    .color(th.faint)
                    .wrap(),
            ),
    }
}

/// Settings can open the performance notes in a browser.
pub fn notes_link(th: &Theme) -> rune::El {
    parts::link_row(
        th,
        "perf-notes",
        "Read the performance notes",
        |_: &mut Ui, tx: &mut Tx| {
            let _ =
                tx.open_url("https://github.com/itsinvin/glasswing/blob/main/docs/PERFORMANCE.md");
        },
    )
}
