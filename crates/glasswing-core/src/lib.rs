//! Glasswing core.
//!
//! This crate holds the parts that do not need a window: finding a PS5 on the
//! local network, waking it, and saving settings. The stream protocol will
//! live here too, so it can be tested without the GUI.
//!
//! Protocol details are based on chiaki-ng (AGPL-3.0) and KyberNow
//! (AGPL-3.0). See NOTICE at the repository root.

pub mod config;
pub mod discovery;
pub mod regist;
pub mod rpcrypt;
pub mod rpcrypt_tables;
pub mod sysinfo;
pub mod wake;

/// Name shown in the app and in logs.
pub const APP_NAME: &str = "Glasswing";

/// Version taken from Cargo.toml.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
