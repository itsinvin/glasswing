//! The app's state, the work it does off the UI thread, and the window it opens.
//!
//! Network scans, wake packets and the system check all run on worker threads. Each one posts
//! its result back through the app handle, so the UI thread never blocks and the window stays
//! smooth.

use std::net::IpAddr;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use glasswing_core::config::{Config, SavedHost};
use glasswing_core::discovery::{self, ConsoleState, DiscoveredConsole};
use glasswing_core::regist::{self, PairedHost};
use glasswing_core::sysinfo::{self, SystemInfo};
use rune::tok::{radius, space, type_ as tt};
use rune::{
    col, el, hairline, oklch, row, text, App, AppHandle, AppSpec, ColorScheme, Cx, El, Key,
    KeyEvent, ScrollSpec, TextEdit, Tx,
};

use crate::pages;
use crate::parts;
use crate::theme::{type_, Theme};

/// Sidebar width in points.
const SIDEBAR_W: f32 = 236.0;
/// The window's first size and the smallest the user can drag it to.
const WINDOW: (f32, f32) = (1080.0, 720.0);
const MIN_WINDOW: (f32, f32) = (860.0, 600.0);
/// How long a toast stays on screen, in seconds.
const TOAST_SECS: u64 = 5;

/// Which page the window shows.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Consoles,
    Settings,
    About,
}

impl Page {
    pub fn title(self) -> &'static str {
        match self {
            Page::Consoles => "Consoles",
            Page::Settings => "Settings",
            Page::About => "About",
        }
    }
}

/// What a toast is for.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    /// It worked.
    Good,
    /// Something needs a look.
    Warn,
    /// Something failed.
    Bad,
}

/// A short message that fades away on its own.
pub struct Toast {
    pub kind: ToastKind,
    pub message: String,
    pub expires: Instant,
}

/// Everything the window shows and knows.
pub struct Ui {
    /// The page in view.
    pub page: Page,
    /// The settings as they are on disk.
    pub config: Config,
    /// Where the settings file lives.
    pub config_path: PathBuf,
    /// The settings the Settings page edits. Only written to disk on Save.
    pub draft: Config,
    /// True once the user edits the draft, which stops resets from the rest of the app.
    pub draft_edited: bool,
    /// The PSN account field, so Rune owns its caret and undo.
    pub psn: TextEdit,
    /// Consoles found by the last scan.
    pub discovered: Vec<DiscoveredConsole>,
    /// True while a scan runs.
    pub scanning: bool,
    /// When the last scan finished.
    pub last_scan: Option<Instant>,
    /// True while the add-console window is up.
    pub add_open: bool,
    pub add_name: TextEdit,
    pub add_address: TextEdit,
    pub add_key: TextEdit,
    /// Why the add-console window will not save, if it will not.
    pub add_error: Option<String>,
    /// True while the pairing window is up.
    pub pair_open: bool,
    /// Which console is being paired.
    pub pair_host_id: Option<String>,
    /// The PIN the user typed, taken from the console's screen.
    pub pair_pin: TextEdit,
    /// Where pairing is, in words for the window.
    pub pair_status: String,
    /// Why pairing stopped, if it did.
    pub pair_failure: Option<String>,
    /// True while a worker thread runs the exchange.
    pub pair_busy: bool,
    /// What the console handed back, once it worked.
    pub pair_result: Option<PairedHost>,
    /// What the system check found.
    pub system: Option<SystemInfo>,
    pub system_loading: bool,
    /// The message to show, if any.
    pub toast: Option<Toast>,
    /// The host id waiting for a second click on Remove.
    pub confirm_remove: Option<String>,
    /// The bitrate slider's box in window points, so a press knows where it landed.
    pub slider_rect: [f32; 4],
    /// A handle back to the running app, for worker threads to post results. `None` in tests.
    pub handle: Option<AppHandle<Ui>>,
}

impl Ui {
    fn new(config_path: PathBuf, config: Config, handle: AppHandle<Ui>) -> Ui {
        Ui {
            page: Page::Consoles,
            draft: config.clone(),
            config,
            config_path,
            draft_edited: false,
            psn: TextEdit::new(""),
            discovered: Vec::new(),
            scanning: false,
            last_scan: None,
            add_open: false,
            add_name: TextEdit::new(""),
            add_address: TextEdit::new(""),
            add_key: TextEdit::new(""),
            add_error: None,
            pair_open: false,
            pair_host_id: None,
            pair_pin: TextEdit::new(""),
            pair_status: String::new(),
            pair_failure: None,
            pair_busy: false,
            pair_result: None,
            system: None,
            system_loading: false,
            toast: None,
            confirm_remove: None,
            slider_rect: [0.0; 4],
            handle: Some(handle),
        }
    }

    /// A state with no path to write to and no running app behind it, for tests.
    #[cfg(test)]
    pub fn for_test() -> Ui {
        Ui {
            page: Page::Consoles,
            draft: Config::default(),
            config: Config::default(),
            config_path: PathBuf::from("does-not-exist.toml"),
            draft_edited: false,
            psn: TextEdit::new(""),
            discovered: Vec::new(),
            scanning: false,
            last_scan: None,
            add_open: false,
            add_name: TextEdit::new(""),
            add_address: TextEdit::new(""),
            add_key: TextEdit::new(""),
            add_error: None,
            pair_open: false,
            pair_host_id: None,
            pair_pin: TextEdit::new(""),
            pair_status: String::new(),
            pair_failure: None,
            pair_busy: false,
            pair_result: None,
            system: None,
            system_loading: false,
            toast: None,
            confirm_remove: None,
            slider_rect: [0.0; 4],
            handle: None,
        }
    }

    /// Put a message on screen.
    pub fn toast(&mut self, kind: ToastKind, message: impl Into<String>) {
        self.toast = Some(Toast {
            kind,
            message: message.into(),
            expires: Instant::now() + Duration::from_secs(TOAST_SECS),
        });
    }

    /// Every console to show: saved hosts first, then consoles a scan found that are not saved.
    pub fn rows(&self) -> Vec<Row> {
        let mut rows: Vec<Row> = self
            .config
            .hosts
            .iter()
            .cloned()
            .map(|host| Row {
                live: self
                    .discovered
                    .iter()
                    .find(|live| {
                        live.address == host.address
                            || (!live.host_id.is_empty() && live.host_id == host.id)
                    })
                    .cloned(),
                host: Some(host),
            })
            .collect();

        rows.extend(
            self.discovered
                .iter()
                .filter(|live| {
                    !self.config.hosts.iter().any(|host| {
                        host.address == live.address
                            || (!live.host_id.is_empty() && host.id == live.host_id)
                    })
                })
                .cloned()
                .map(|live| Row {
                    live: Some(live),
                    host: None,
                }),
        );
        rows
    }
}

/// One line in the console list: a saved host, a console on the network, or both.
pub struct Row {
    pub host: Option<SavedHost>,
    pub live: Option<DiscoveredConsole>,
}

impl Row {
    pub fn name(&self) -> &str {
        match (&self.host, &self.live) {
            (Some(host), _) => host.name.as_str(),
            (None, Some(live)) => live.name.as_str(),
            _ => "Unknown console",
        }
    }

    pub fn address(&self) -> &str {
        match (&self.host, &self.live) {
            (Some(host), _) => host.address.as_str(),
            (None, Some(live)) => live.address.as_str(),
            _ => "",
        }
    }
}

/// Open the window and run until the user closes it.
pub fn run(config_path: PathBuf, config: Config) {
    let mut spec = AppSpec::new("Glasswing", WINDOW, new_dummy, view);
    spec.min_size = Some(MIN_WINDOW);
    spec.on_key = Some(on_key);
    // `setup` runs once before the first frame: kick off the first scan and the system check.
    spec.setup = Some(setup);
    spec.poll = Some(poll);
    // Headless snapshots: one per page, so `--snap` can show any of them without a window.
    spec.scenes = &[
        ("settings", |s: &mut Ui| {
            s.page = Page::Settings;
        }),
        ("about", |s: &mut Ui| {
            s.page = Page::About;
        }),
        ("empty", |s: &mut Ui| {
            s.config.hosts.clear();
            s.discovered.clear();
        }),
        ("add", |s: &mut Ui| {
            s.add_open = true;
        }),
        ("pair", |s: &mut Ui| {
            fill_demo(s);
            s.pair_open = true;
            s.pair_host_id = Some(s.config.hosts[0].id.clone());
            s.pair_status = "Enter the PIN the console shows, then press Pair.".to_string();
        }),
        ("found", |s: &mut Ui| fill_demo(s)),
        ("toast", |s: &mut Ui| {
            s.page = Page::Settings;
            s.toast(ToastKind::Good, "Saved");
        }),
    ];

    let app = App::new(spec)
        // The app is dark all the time, so the native title bar follows it on Windows and macOS.
        .color_scheme(ColorScheme::Dark);
    let handle = app.handle();
    app.init(move || Ui::new(config_path, config, handle)).run();
}

/// A throwaway state for `AppSpec::init`, replaced by `App::init` before the window opens.
fn new_dummy() -> Ui {
    unreachable!("App::init supplies the state")
}

/// Write the settings file, then say so.
fn write_config(s: &mut Ui, config: Config) {
    match config.save(&s.config_path) {
        Ok(()) => {
            s.config = config;
            s.draft = s.config.clone();
            s.draft_edited = false;
            s.toast(ToastKind::Good, "Saved");
        }
        Err(error) => s.toast(ToastKind::Bad, format!("Could not save: {error}")),
    }
}

/// Start a scan if one is not already running.
pub fn rescan(s: &mut Ui) {
    if s.scanning {
        return;
    }
    s.scanning = true;
    spawn_scan(s, 3_000);
}

/// Save the Settings page's draft.
pub fn save_settings(s: &mut Ui) {
    let draft = s.draft.clone();
    if let Err(error) = draft.validate() {
        s.toast(ToastKind::Warn, error);
        return;
    }
    write_config(s, draft);
}

/// Save a console the scan found, so it stays on the list.
pub fn save_console(s: &mut Ui, live: DiscoveredConsole) {
    let mut config = s.config.clone();
    let id = if live.host_id.is_empty() {
        format!("manual:{}", live.address)
    } else {
        live.host_id.clone()
    };
    config.upsert_host(SavedHost {
        id,
        name: live.name.clone(),
        address: live.address.clone(),
        regist_key: None,
        last_seen: Some(now()),
        manual: false,
    });
    write_config(s, config);
    s.toast(ToastKind::Good, format!("Saved {}", live.name));
}

/// Take a console off the list.
pub fn remove_host(s: &mut Ui, id: String) {
    let mut config = s.config.clone();
    config.remove_host(&id);
    write_config(s, config);
    s.toast(ToastKind::Good, "Console removed");
}

/// Save what the user typed into the add-console window.
pub fn commit_add(s: &mut Ui) {
    let name = s.add_name.text.trim().to_string();
    let address = s.add_address.text.trim().to_string();
    if name.is_empty() || address.is_empty() {
        s.add_error = Some("Please enter a name and an address.".to_string());
        return;
    }
    let regist_key = {
        let key = s.add_key.text.trim().to_string();
        (!key.is_empty()).then_some(key)
    };

    let mut config = s.config.clone();
    config.upsert_host(SavedHost {
        id: format!("manual:{address}"),
        name,
        address,
        regist_key,
        last_seen: Some(now()),
        manual: true,
    });
    if let Err(error) = config.validate() {
        s.add_error = Some(error);
        return;
    }

    s.add_error = None;
    s.add_name = TextEdit::new("");
    s.add_address = TextEdit::new("");
    s.add_key = TextEdit::new("");
    write_config(s, config);
    s.add_open = false;
    s.toast(ToastKind::Good, "Console added");
}

/// Look again at the machine's GPU and FFmpeg.
pub fn refresh_system(s: &mut Ui) {
    if s.system_loading {
        return;
    }
    s.system_loading = true;
    spawn_system(s);
}

/// Open the pairing window for one console, PIN in hand.
pub fn open_pairing(s: &mut Ui, id: String) {
    s.pair_open = true;
    s.pair_host_id = Some(id);
    s.pair_pin = TextEdit::new("");
    s.pair_failure = None;
    s.pair_busy = false;
    s.pair_result = None;
    let saved = s.psn.text.trim();
    s.pair_status = if saved.is_empty() {
        "Save your PSN account ID in Settings first.".to_string()
    } else {
        "Enter the PIN the console shows, then press Pair.".to_string()
    };
}

/// Read the saved account ID and PIN, and start the exchange on a worker thread.
pub fn start_pairing(s: &mut Ui) {
    if s.pair_busy {
        return;
    }
    let Some(host) = s
        .pair_host_id
        .as_deref()
        .and_then(|id| s.config.hosts.iter().find(|host| host.id == id))
        .cloned()
    else {
        s.pair_failure = Some("That console is not saved any more.".to_string());
        return;
    };
    let account_id = match regist::decode_account_id(s.psn.text.trim()) {
        Ok(id) => id,
        Err(why) => {
            s.pair_failure = Some(why);
            return;
        }
    };
    let pin_text = s.pair_pin.text.trim().to_string();
    let Ok(pin) = pin_text.parse::<u32>() else {
        s.pair_failure = Some("The PIN is the 8 digits the console shows.".to_string());
        return;
    };

    s.pair_busy = true;
    s.pair_failure = None;
    s.pair_status = "Looking for the console's pairing screen…".to_string();
    let address = host.address.clone();
    let id = host.id.clone();
    let Some(handle) = s.handle.clone() else {
        s.pair_busy = false;
        return;
    };

    std::thread::spawn(move || {
        // Each step owns a handle of its own, so the closure can borrow nothing.
        let handle_for_steps = handle.clone();
        let on_step = move |step: regist::PairingStep| {
            let text = step.text().to_string();
            let handle = handle_for_steps.clone();
            handle.update(move |s: &mut Ui, _: &mut Tx| {
                s.pair_status = text.clone();
            });
        };
        let request = regist::PairingRequest {
            address: &address,
            port: regist::REGIST_PORT,
            console: regist::Console::Ps5,
            account_id,
            pin,
            on_step: Some(Box::new(on_step)),
        };
        let outcome = regist::pair(&request);
        handle.update(move |s: &mut Ui, tx: &mut Tx| {
            s.pair_busy = false;
            match outcome {
                Ok(host_paired) => {
                    s.pair_result = Some(host_paired.clone());
                    s.pair_status = "Paired.".to_string();
                    // Keep the key against this console, so wake and later
                    // streaming find it.
                    let mut config = s.config.clone();
                    if let Some(entry) = config.hosts.iter_mut().find(|entry| entry.id == id) {
                        entry.regist_key = Some(host_paired.regist_key_hex());
                    }
                    write_config(s, config);
                    s.toast(
                        ToastKind::Good,
                        format!("Paired with {}", host_paired.nickname),
                    );
                }
                Err(error) => {
                    s.pair_failure = Some(error.to_string());
                    s.pair_status = "Pairing stopped.".to_string();
                }
            }
            tx.instant();
        });
    });
}

/// Seconds since the epoch, for `last_seen`.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

/// Stand-in consoles for the "found" scene, so a headless snapshot shows a full page.
fn fill_demo(s: &mut Ui) {
    s.discovered = vec![
        console(
            "192.168.1.24",
            "Living room PS5",
            "7c:ab:12:34:56:78",
            ConsoleState::Ready,
        ),
        console(
            "192.168.1.31",
            "Bedroom PS5",
            "9e:f3:aa:bb:cc:dd",
            ConsoleState::Standby,
        ),
    ];
    s.config.hosts = vec![SavedHost {
        id: "7c:ab:12:34:56:78".to_string(),
        name: "Living room PS5".to_string(),
        address: "192.168.1.24".to_string(),
        regist_key: None,
        last_seen: Some(now()),
        manual: false,
    }];
    s.psn = TextEdit::new("base64-account-id-here");
}

fn console(address: &str, name: &str, host_id: &str, state: ConsoleState) -> DiscoveredConsole {
    DiscoveredConsole {
        address: address.to_string(),
        state,
        host_id: host_id.to_string(),
        name: name.to_string(),
        host_type: "PS5".to_string(),
        system_version: "09060000".to_string(),
        protocol_version: "00030010".to_string(),
        request_port: 9295,
        running_app_title_id: None,
        running_app_name: None,
    }
}

/// Runs once on the UI thread before the first frame.
fn setup(s: &mut Ui, _: rune::Waker) {
    spawn_scan(s, 3_000);
    spawn_system(s);
}

/// Ask every console on the network to say hello. Runs on a worker thread.
fn spawn_scan(s: &Ui, wait_ms: u64) {
    let direct: Vec<IpAddr> = s
        .config
        .hosts
        .iter()
        .filter_map(|host| host.address.parse().ok())
        .collect();
    let Some(handle) = s.handle.clone() else {
        return;
    };

    std::thread::spawn(move || {
        let result = discovery::search(Duration::from_millis(wait_ms), &direct);
        handle.update(move |s: &mut Ui, tx: &mut Tx| {
            s.scanning = false;
            s.last_scan = Some(Instant::now());
            match result {
                Ok(found) => {
                    let count = found.len();
                    let names = found
                        .iter()
                        .map(|console| console.name.clone())
                        .collect::<Vec<_>>()
                        .join(", ");
                    s.discovered = found;
                    if count == 0 {
                        s.toast(
                            ToastKind::Warn,
                            "No consoles found. Check that the PS5 is on the same network.",
                        );
                    } else {
                        s.toast(
                            ToastKind::Good,
                            format!(
                                "Found {count} console{}: {names}",
                                if count == 1 { "" } else { "s" }
                            ),
                        );
                    }
                }
                Err(error) => s.toast(ToastKind::Bad, format!("Scan failed: {error}")),
            }
            tx.instant();
        });
    });
}

/// Send a wake packet to one console. Runs on a worker thread.
pub fn spawn_wake(address: String, regist_key: Option<String>, handle: Option<AppHandle<Ui>>) {
    let Some(handle) = handle else {
        return;
    };

    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(|| {
            let credential = match &regist_key {
                Some(key) if !key.trim().is_empty() => {
                    glasswing_core::wake::credential_from_hex(key)?
                }
                _ => 0,
            };
            glasswing_core::wake::wake(&address, credential).map_err(|error| error.to_string())?;
            Ok::<String, String>(if credential == 0 {
                format!(
                    "Wake packet sent to {address}. A console that stays asleep needs pairing first."
                )
            } else {
                format!("Wake packet sent to {address}.")
            })
        })
        .unwrap_or_else(|_| Err("The wake task stopped unexpectedly.".to_string()));

        handle.update(move |s: &mut Ui, tx: &mut Tx| {
            match result {
                Ok(message) => s.toast(ToastKind::Good, message),
                Err(error) => s.toast(
                    ToastKind::Bad,
                    format!("Could not wake the console: {error}"),
                ),
            }
            tx.instant();
        });
    });
}

/// Look for an NVIDIA GPU and FFmpeg. Runs on a worker thread.
fn spawn_system(s: &Ui) {
    let Some(handle) = s.handle.clone() else {
        return;
    };

    std::thread::spawn(move || {
        let info = sysinfo::detect();
        handle.update(move |s: &mut Ui, _: &mut Tx| {
            s.system_loading = false;
            s.system = Some(info);
        });
    });
}

/// Every frame: keep the Settings page's draft in step with the rest of the app and retire old
/// toasts. Returns true when the view should run again.
fn poll(s: &mut Ui, _now: f64, _: &mut Tx) -> bool {
    sync_draft(s) | retire_toast(s)
}

/// Take the Settings page's draft back to the saved settings, unless the user has typed into it.
/// Returns true when the draft changed.
fn sync_draft(s: &mut Ui) -> bool {
    if !s.draft_edited && s.draft != s.config {
        s.draft = s.config.clone();
        true
    } else {
        false
    }
}

/// Drop a toast once its time is up. Returns true when one goes.
fn retire_toast(s: &mut Ui) -> bool {
    if let Some(toast) = &s.toast {
        if toast.expires <= Instant::now() {
            s.toast = None;
            return true;
        }
    }
    false
}

/// Keys the whole window handles. Escape closes the add-console window first.
fn on_key(s: &mut Ui, event: &KeyEvent, _: &mut Tx) -> bool {
    if s.add_open && event.key == Key::Escape {
        s.add_open = false;
        return true;
    }
    if s.pair_open && event.key == Key::Escape {
        if s.pair_busy {
            s.toast(ToastKind::Warn, "Pairing is still running");
        } else {
            s.pair_open = false;
        }
        return true;
    }
    false
}

/// The whole window.
fn view(s: &Ui, _cx: &mut Cx<Ui>) -> El {
    let th = Theme::dark();

    let sidebar = col()
        .key("sidebar")
        .w(SIDEBAR_W)
        .h_full()
        .fill(th.sidebar)
        .py(space::S3)
        .gap(2.0)
        .child(logo(&th))
        .child(col().key("nav").mt(space::S3).gap(2.0).children([
            parts::nav_row(
                &th,
                "nav-consoles",
                "Consoles",
                s.page == Page::Consoles,
                |s: &mut Ui| s.page = Page::Consoles,
            ),
            parts::nav_row(
                &th,
                "nav-settings",
                "Settings",
                s.page == Page::Settings,
                |s: &mut Ui| s.page = Page::Settings,
            ),
            parts::nav_row(
                &th,
                "nav-about",
                "About",
                s.page == Page::About,
                |s: &mut Ui| s.page = Page::About,
            ),
        ]))
        .child(el().grow(1.0))
        .child(
            col()
                .key("early")
                .mx(space::S3)
                .padding(space::S3)
                .gap(space::S1)
                .radius(radius::SM)
                .fill(th.well)
                .border(1.0, th.line)
                .child(text("Early build").font(type_::LABEL).color(th.dim))
                .child(
                    text(
                        "Discovery, wake and wake settings work. Pairing and streaming come next.",
                    )
                    .font(type_::SMALL)
                    .color(th.faint)
                    .wrap(),
                ),
        )
        .child(parts::link_row(
            &th,
            "repo",
            "Source on GitHub",
            |_: &mut Ui, tx: &mut Tx| {
                let _ = tx.open_url("https://github.com/itsinvin/glasswing");
            },
        ))
        .child(
            text(format!("AGPL-3.0 · v{}", glasswing_core::VERSION))
                .font(type_::SMALL)
                .color(th.faint)
                .px(space::S3),
        );

    let header = row()
        .key("header")
        .w_full()
        .min_h(64.0)
        .px(space::S6)
        .py(space::S3)
        .center_y()
        .justify_between()
        .child(
            col()
                .key("titles")
                .gap(2.0)
                .child(text(s.page.title()).font(type_::H1).color(th.text))
                .child(
                    text("Local network only · Windows 10/11 · PS5")
                        .font(type_::SMALL)
                        .color(th.faint),
                ),
        )
        .child(
            row()
                .key("badges")
                .gap(space::S2)
                .center_y()
                .child(parts::badge("badge-local", "Local only", th.good))
                .child(parts::link_row(
                    &th,
                    "repo-header",
                    "GitHub",
                    |_: &mut Ui, tx: &mut Tx| {
                        let _ = tx.open_url("https://github.com/itsinvin/glasswing");
                    },
                )),
        );

    let body = col()
        .key("page")
        .grow(1.0)
        .min_h(0.0)
        .w_full()
        .scroll(ScrollSpec::default())
        .px(space::S6)
        .pb(space::S6)
        .gap(space::S3)
        .children(match s.page {
            Page::Consoles => pages::consoles::view(s, &th),
            Page::Settings => pages::settings::view(s, &th),
            Page::About => pages::about::view(s, &th),
        });

    let main = col()
        .key("main")
        .grow(1.0)
        .min_w(0.0)
        .fill(th.bg)
        .child(hairline(th.line).w_full())
        .child(header)
        .child(col().key("header-line").h(1.0).fill(th.line).w_full())
        .child(body)
        .children(s.toast.as_ref().map(|toast| parts::toast(&th, toast)));

    let mut root = row()
        .key("root")
        .w_full()
        .h_full()
        .fill(th.bg)
        .child(sidebar)
        .child(col().key("side-line").w(1.0).h_full().fill(th.line))
        .child(main);

    if s.add_open {
        root = root.child(parts::add_dialog(s, &th));
    }
    if s.pair_open {
        root = root.child(pages::pairing::dialog(s, &th));
    }
    root
}

/// The mark in the sidebar's top corner: a rounded tile with a G in it.
fn logo(th: &Theme) -> El {
    row()
        .key("logo")
        .w_full()
        .px(space::S3)
        .gap(space::S2)
        .center_y()
        .child(
            col()
                .key("logo-tile")
                .size(28.0, 28.0)
                .radius(radius::SM)
                .fill(th.accent)
                .center()
                .child(text("G").font(tt::TITLE).color(oklch(0.985, 0.0, 0.0))),
        )
        .child(
            col()
                .key("wordmark")
                .child(text("Glasswing").font(type_::H2).color(th.text))
                .child(text("PS5 Remote Play").font(type_::SMALL).color(th.faint)),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Env, KeyEvent as RuneKeyEvent, Mods, Runtime};

    /// Draw a window without a real one and hand the state back for checks.
    fn runtime() -> Runtime<Ui> {
        Runtime::new(Ui::for_test(), view, WINDOW, Env::new(true, 1.0))
    }

    #[test]
    fn every_page_builds_a_tree() {
        let mut rt = runtime();
        rt.frame(0.0);
        assert!(rt.find("nav-consoles").is_some());
        assert!(rt.find("nav-settings").is_some());
        assert!(rt.find("nav-about").is_some());
        assert!(rt.find("console-actions").is_some());

        rt.update(|s: &mut Ui, _| s.page = Page::Settings);
        rt.frame(0.0);
        assert!(rt.find("prefer-nvidia").is_some());

        rt.update(|s: &mut Ui, _| s.page = Page::About);
        rt.frame(0.0);
        assert!(rt.find("about-card").is_some());
    }

    #[test]
    fn escape_closes_the_add_window_and_nothing_else() {
        let mut rt = runtime();
        rt.frame(0.0);
        rt.update(|s: &mut Ui, _| s.add_open = true);
        rt.frame(0.0);
        assert!(rt.find("add-dialog").is_some());

        rt.key(RuneKeyEvent {
            key: Key::Escape,
            mods: Mods::default(),
        });
        rt.frame(0.0);
        assert!(!rt.state.add_open);
        assert!(rt.find("add-dialog").is_none());
    }

    #[test]
    fn a_toast_goes_away_once_it_is_old() {
        let mut s = Ui::for_test();
        s.toast(ToastKind::Good, "Saved");
        assert!(s.toast.is_some());
        assert!(!retire_toast(&mut s), "a fresh toast stays");

        s.toast.as_mut().unwrap().expires = Instant::now() - Duration::from_secs(1);
        assert!(retire_toast(&mut s), "an old toast goes");
        assert!(s.toast.is_none());
    }

    #[test]
    fn a_fresh_draft_is_replaced_by_a_change_from_the_rest_of_the_app() {
        let mut s = Ui::for_test();
        assert!(
            !sync_draft(&mut s),
            "a fresh draft already matches the settings"
        );

        s.config.stream.fps = 30;
        assert!(
            sync_draft(&mut s),
            "a change from the rest of the app flows in"
        );
        assert_eq!(s.draft.stream.fps, 30);

        s.draft.stream.bitrate_mbps = 9;
        s.draft_edited = true;
        assert!(!sync_draft(&mut s), "an edited draft stands its ground");
        assert_eq!(s.draft.stream.bitrate_mbps, 9);

        s.draft_edited = false;
        s.config.stream.fps = 60;
        assert!(sync_draft(&mut s), "a saved change flows back in");
        assert_eq!(s.draft.stream.fps, 60);
    }

    #[test]
    fn the_settings_draft_follows_the_saved_settings_until_edited() {
        let mut rt = runtime();
        rt.frame(0.0);
        rt.state.config_path = std::env::temp_dir().join("glasswing-test-config.toml");

        rt.update(|s: &mut Ui, _| s.draft.stream.fps = 30);
        rt.frame(0.0);
        // An edit stands until it is saved.
        assert_eq!(rt.state.draft.stream.fps, 30);

        rt.update(|s: &mut Ui, _| s.draft.stream.fps = 120);
        rt.frame(0.0);
        // 120 fps is not a real choice, so validation refuses it.
        rt.update(|s: &mut Ui, _| save_settings(s));
        rt.frame(0.0);
        assert_eq!(rt.state.draft.stream.fps, 120);
        assert!(
            rt.state.toast.is_some(),
            "a bad choice warns instead of saving"
        );

        rt.update(|s: &mut Ui, _| s.draft.stream.fps = 60);
        rt.update(|s: &mut Ui, _| save_settings(s));
        rt.frame(0.0);
        assert_eq!(rt.state.config.stream.fps, 60);
        assert!(!rt.state.draft_edited);
    }

    #[test]
    fn a_saved_console_shows_its_state() {
        let mut rt = runtime();
        fill_demo(&mut rt.state);
        rt.frame(0.0);
        assert!(rt.find("console-1").is_some());
        // One saved console with its live partner, plus one the scan found.
        assert_eq!(rt.state.rows().len(), 2);
    }
}
