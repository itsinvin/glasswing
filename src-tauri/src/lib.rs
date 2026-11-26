//! Tauri backend for Glasswing.
//!
//! The web UI calls these commands through `invoke`. Network scans and system
//! checks run on worker threads so the window stays smooth.

use glasswing_core::config::{Config, SavedHost};
use glasswing_core::discovery::{self, DiscoveredConsole};
use glasswing_core::sysinfo::SystemInfo;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::State;
use tracing_subscriber::prelude::*;

struct AppState {
    config_path: PathBuf,
    config: Mutex<Config>,
}

#[derive(Serialize)]
struct AppInfo {
    version: String,
    config_path: String,
    log_path: String,
    license: String,
    repository: String,
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> Config {
    state
        .config
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

#[tauri::command]
fn save_config(state: State<'_, AppState>, config: Config) -> Result<Config, String> {
    config.validate()?;

    let mut current = state
        .config
        .lock()
        .map_err(|_| "Settings are busy. Try again.".to_string())?;
    config
        .save(&state.config_path)
        .map_err(|error| error.to_string())?;
    *current = config.clone();
    Ok(config)
}

#[tauri::command]
fn upsert_host(
    state: State<'_, AppState>,
    name: String,
    address: String,
    host_id: Option<String>,
    regist_key: Option<String>,
    manual: bool,
) -> Result<Config, String> {
    let name = name.trim().to_string();
    let address = address.trim().to_string();
    if name.is_empty() || address.is_empty() {
        return Err("Please enter a name and an address.".to_string());
    }

    let id = host_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("manual:{address}"));

    let regist_key = regist_key
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let mut current = state
        .config
        .lock()
        .map_err(|_| "Settings are busy. Try again.".to_string())?;
    let mut config = current.clone();
    config.upsert_host(SavedHost {
        id,
        name,
        address,
        regist_key,
        last_seen: Some(now()),
        manual,
    });
    config.validate()?;
    config
        .save(&state.config_path)
        .map_err(|error| error.to_string())?;
    *current = config.clone();
    Ok(config)
}

#[tauri::command]
fn remove_host(state: State<'_, AppState>, id: String) -> Result<Config, String> {
    let mut current = state
        .config
        .lock()
        .map_err(|_| "Settings are busy. Try again.".to_string())?;
    let mut config = current.clone();
    config.remove_host(&id);
    config
        .save(&state.config_path)
        .map_err(|error| error.to_string())?;
    *current = config.clone();
    Ok(config)
}

#[tauri::command]
async fn discover_consoles(
    wait_ms: Option<u64>,
    direct: Option<Vec<String>>,
) -> Result<Vec<DiscoveredConsole>, String> {
    let wait = Duration::from_millis(wait_ms.unwrap_or(3_000).clamp(500, 15_000));
    let direct: Vec<std::net::IpAddr> = direct
        .unwrap_or_default()
        .into_iter()
        .filter_map(|text| text.trim().parse().ok())
        .collect();

    tauri::async_runtime::spawn_blocking(move || discovery::search(wait, &direct))
        .await
        .map_err(|error| format!("Scan task failed: {error}"))?
        .map_err(|error| format!("Scan failed: {error}"))
}

#[tauri::command]
async fn wake_console(address: String, regist_key: Option<String>) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let credential = match regist_key {
            Some(key) if !key.trim().is_empty() => glasswing_core::wake::credential_from_hex(&key)?,
            _ => 0,
        };

        glasswing_core::wake::wake(&address, credential).map_err(|error| error.to_string())?;

        if credential == 0 {
            Ok(format!(
                "Wake packet sent to {address}. If it stays asleep, pair the console first."
            ))
        } else {
            Ok(format!("Wake packet sent to {address}."))
        }
    })
    .await
    .map_err(|error| format!("Wake task failed: {error}"))?
}

#[tauri::command]
async fn system_info() -> Result<SystemInfo, String> {
    tauri::async_runtime::spawn_blocking(glasswing_core::sysinfo::detect)
        .await
        .map_err(|error| format!("System check failed: {error}"))
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        version: glasswing_core::VERSION.to_string(),
        config_path: Config::default_path().display().to_string(),
        log_path: Config::log_dir().display().to_string(),
        license: "AGPL-3.0".to_string(),
        repository: "https://github.com/itsinvin/glasswing".to_string(),
    }
}

/// Start the desktop app.
pub fn run() {
    let log_guard = init_logging();
    // The guard must live for the whole app, or log writes stop.
    Box::leak(Box::new(log_guard));

    let config_path = Config::default_path();
    let config = match Config::load(&config_path) {
        Ok(config) => config,
        Err(error) => {
            tracing::warn!(%error, "could not load settings; using defaults");
            Config::default()
        }
    };

    tracing::info!(
        version = glasswing_core::VERSION,
        path = %config_path.display(),
        "starting Glasswing"
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Keep the native title bar dark even when Windows is set to tint
            // title bars with the user's accent color.
            #[cfg(windows)]
            {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    if let Ok(hwnd) = window.hwnd() {
                        set_dark_title_bar(hwnd.0 as isize);
                    }
                }
            }
            Ok(())
        })
        .manage(AppState {
            config_path,
            config: Mutex::new(config),
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            upsert_host,
            remove_host,
            discover_consoles,
            wake_console,
            system_info,
            app_info
        ])
        .run(tauri::generate_context!())
        .expect("Glasswing failed to start");
}

/// Ask DWM to draw the window frame in dark mode.
#[cfg(windows)]
fn set_dark_title_bar(hwnd: isize) {
    use std::ffi::c_void;

    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmSetWindowAttribute(
            hwnd: isize,
            attribute: u32,
            value: *const c_void,
            size: u32,
        ) -> i32;
    }

    // DWMWA_USE_IMMERSIVE_DARK_MODE.
    const DARK_MODE: u32 = 20;
    let enabled: i32 = 1;
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DARK_MODE,
            &enabled as *const i32 as *const c_void,
            std::mem::size_of::<i32>() as u32,
        );
    }
}

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

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
