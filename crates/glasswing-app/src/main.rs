//! Glasswing: the desktop app for PS5 Remote Play.
//!
//! The whole window is native Rust drawn on the GPU by [Rune](https://github.com/anomalyco/rune).
//! There is no webview, no browser engine and no JavaScript anywhere in the app.
//!
//! Protocol work builds on chiaki-ng (GPLv3) and KyberNow (MIT); see the NOTICE file.

mod pages;
mod parts;
mod state;
mod theme;

use glasswing_core::config::Config;
use tracing_subscriber::prelude::*;

fn main() {
    let guard = init_logging();
    // The guard must live for the whole run, or log writes stop.
    std::mem::forget(guard);

    let config_path = Config::default_path();
    let config = Config::load(&config_path).unwrap_or_else(|error| {
        tracing::warn!(%error, "could not load settings; starting from defaults");
        Config::default()
    });

    tracing::info!(
        version = glasswing_core::VERSION,
        path = %config_path.display(),
        "starting Glasswing"
    );

    state::run(config_path, config);
}

/// Log to stderr for the console and to a rolling file for bug reports.
fn init_logging() -> tracing_appender::non_blocking::WorkerGuard {
    let dir = Config::log_dir();
    let _ = std::fs::create_dir_all(&dir);

    let file = tracing_appender::rolling::daily(&dir, "glasswing.log");
    let (file, guard) = tracing_appender::non_blocking(file);

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,glasswing_core=debug"));

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(file)
                .with_ansi(false),
        )
        .try_init();

    guard
}
