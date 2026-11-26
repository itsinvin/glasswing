//! Small command line tool for testing the core without the GUI.
//!
//! Examples:
//!   gw discover
//!   gw discover --wait 5 --json
//!   gw doctor
//!   gw config path

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use glasswing_core::config::Config;
use glasswing_core::discovery::{self, DiscoveredConsole};
use glasswing_core::wake;
use std::net::IpAddr;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "gw",
    version,
    about = "Find and wake PS5 consoles on your local network"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Look for PS5 consoles on the local network.
    Discover {
        /// How many seconds to listen.
        #[arg(long, default_value_t = 3)]
        wait: u64,
        /// Also try a known console address. Can be used more than once.
        #[arg(long = "direct")]
        direct: Vec<String>,
        /// Print JSON instead of a table.
        #[arg(long)]
        json: bool,
    },
    /// Wake a paired console from rest mode.
    Wake {
        /// Console IPv4 address or host name.
        address: String,
        /// Pairing key in hex (8 characters).
        #[arg(long)]
        regist_key: Option<String>,
    },
    /// Pair with a console and save the key it hands back.
    Pair {
        /// Console IPv4 address or host name.
        address: String,
        /// The PIN on the console's Link Device screen.
        #[arg(long)]
        pin: u32,
        /// PSN account ID in base64. Read from settings when not given.
        #[arg(long)]
        account_id: Option<String>,
        /// Regist port, for tests with a fake console.
        #[arg(long, default_value_t = glasswing_core::regist::REGIST_PORT)]
        port: u16,
    },
    /// Show or create saved settings.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Check for NVIDIA, FFmpeg, and NVDEC.
    Doctor,
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// Print the config file path.
    Path,
    /// Print the config file.
    Show,
    /// Create a config file with defaults.
    Init,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    match Cli::parse().command {
        Command::Discover { wait, direct, json } => discover(wait, direct, json),
        Command::Wake {
            address,
            regist_key,
        } => wake_console(address, regist_key),
        Command::Config { command } => config(command),
        Command::Doctor => doctor(),
        Command::Pair {
            address,
            pin,
            account_id,
            port,
        } => pair(address, pin, account_id, port),
    }
}

fn pair(address: String, pin: u32, account_id: Option<String>, port: u16) -> Result<()> {
    let saved = match account_id {
        Some(text) => text,
        None => Config::load(&Config::default_path())?
            .psn_account_id
            .ok_or_else(|| {
                anyhow!("no account ID saved: paste it in Settings or pass --account-id")
            })?,
    };
    let account_id =
        glasswing_core::regist::decode_account_id(&saved).map_err(|why| anyhow!("{why}"))?;

    println!("Pairing with {address}.");
    println!("Keep the console's Link Device screen open with its PIN showing.");

    let request = glasswing_core::regist::PairingRequest {
        address: &address,
        port,
        console: glasswing_core::regist::Console::Ps5,
        account_id,
        pin,
        on_step: None,
    };
    let host = glasswing_core::regist::pair(&request)?;

    println!("Paired with {}.", host.nickname);
    println!("Regist key: {}", host.regist_key_hex());
    println!("Console MAC: {}", glasswing_core::regist::hex(&host.mac));
    println!("RP key type: {}", host.rp_key_type);
    println!();
    println!("The app saves this key itself; the CLI prints it for bug reports.");

    // Save the key against the console, so wake and later streaming find it.
    let path = Config::default_path();
    let mut config = Config::load(&path).unwrap_or_default();
    config.upsert_host(glasswing_core::config::SavedHost {
        id: if host.nickname.is_empty() {
            format!("manual:{address}")
        } else {
            format!("ps5:{}", host.nickname.to_lowercase().replace(' ', "-"))
        },
        name: if host.nickname.is_empty() {
            address.clone()
        } else {
            host.nickname.clone()
        },
        address: address.clone(),
        regist_key: Some(host.regist_key_hex()),
        last_seen: None,
        manual: false,
    });
    config.save(&path)?;
    println!("Saved to {}", path.display());
    Ok(())
}

fn discover(wait_secs: u64, direct: Vec<String>, json: bool) -> Result<()> {
    let wait = Duration::from_secs(wait_secs.clamp(1, 30));
    let mut addresses = Vec::new();
    for text in direct {
        match text.trim().parse::<IpAddr>() {
            Ok(ip) => addresses.push(ip),
            Err(_) => eprintln!("Skipping '{text}': not an IP address."),
        }
    }

    let consoles = discovery::search(wait, &addresses).context("scan failed")?;

    if json {
        println!("{}", serde_json::to_string_pretty(&consoles)?);
    } else if consoles.is_empty() {
        println!("No consoles found.");
        println!("Check that the PS5 is on, on the same network, and has Remote Play turned on.");
    } else {
        print_table(&consoles);
    }

    Ok(())
}

fn print_table(consoles: &[DiscoveredConsole]) {
    println!(
        "{:<24} {:<10} {:<15} {:<12} APP",
        "NAME", "STATE", "ADDRESS", "SYSTEM"
    );
    println!("{}", "-".repeat(88));
    for console in consoles {
        println!(
            "{:<24} {:<10} {:<15} {:<12} {}",
            truncate(&console.name, 24),
            console.state.label(),
            console.address,
            truncate(&console.system_version, 12),
            truncate(console.running_app_name.as_deref().unwrap_or("-"), 30)
        );
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn wake_console(address: String, regist_key: Option<String>) -> Result<()> {
    let credential = match regist_key {
        Some(key) => wake::credential_from_hex(&key).map_err(|e| anyhow!(e))?,
        None => 0,
    };

    wake::wake(&address, credential)
        .with_context(|| format!("could not send a wake packet to {address}"))?;

    println!("Wake packet sent to {address}.");
    if credential == 0 {
        println!("No pairing key was given, so the console will probably stay asleep.");
        println!("Pair the console first, then pass --regist-key <hex>.");
    }
    Ok(())
}

fn config(command: ConfigCommand) -> Result<()> {
    let path = Config::default_path();
    match command {
        ConfigCommand::Path => println!("{}", path.display()),
        ConfigCommand::Show => {
            let config = Config::load(&path)?;
            println!("{}", toml::to_string_pretty(&config)?);
        }
        ConfigCommand::Init => {
            if path.exists() {
                println!("Settings already exist: {}", path.display());
            } else {
                Config::default().save(&path)?;
                println!("Created {}", path.display());
            }
        }
    }
    Ok(())
}

fn doctor() -> Result<()> {
    let info = glasswing_core::sysinfo::detect();
    println!("Glasswing {}", glasswing_core::VERSION);
    println!("System: {} {}", info.os, info.arch);

    match &info.gpu {
        Some(gpu) => {
            println!("GPU: {} (driver {})", gpu.name, gpu.driver);
            println!("VRAM: {}", gpu.memory);
        }
        None => println!("GPU: NVIDIA GPU not found (nvidia-smi is missing)"),
    }

    println!("FFmpeg: {}", info.ffmpeg.as_deref().unwrap_or("not found"));
    println!(
        "NVDEC: {}",
        if info.nvdec { "ready" } else { "not available" }
    );
    println!("Settings: {}", Config::default_path().display());
    Ok(())
}
