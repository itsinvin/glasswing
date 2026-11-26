//! Saved settings and console list.
//!
//! Settings live in `%APPDATA%\Glasswing\config.toml` on Windows. The file is
//! plain TOML so users can edit or back it up.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Everything Glasswing remembers between runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    /// PSN account ID in base64. Needed for pairing a PS5.
    pub psn_account_id: Option<String>,
    /// Consoles the user saved.
    pub hosts: Vec<SavedHost>,
    /// Default stream settings.
    pub stream: StreamSettings,
    /// Performance switches.
    pub performance: PerformanceSettings,
}

/// A console the user added or saved from a scan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedHost {
    /// Stable ID: the console's host ID, or `manual:<address>`.
    pub id: String,
    /// Friendly name.
    pub name: String,
    /// IPv4 address or host name.
    pub address: String,
    /// Pairing key in hex. Filled in after pairing.
    #[serde(default)]
    pub regist_key: Option<String>,
    /// Unix time when the console was last seen.
    #[serde(default)]
    pub last_seen: Option<u64>,
    /// True when the user typed the address instead of finding it.
    #[serde(default)]
    pub manual: bool,
}

/// Stream defaults for the first version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StreamSettings {
    /// "720p" or "1080p".
    pub resolution: String,
    /// 30 or 60.
    pub fps: u32,
    /// 5 to 50 Mbps.
    pub bitrate_mbps: u32,
    /// Only "h264" for now.
    pub codec: String,
    /// HDR is a later milestone.
    pub hdr: bool,
}

impl Default for StreamSettings {
    fn default() -> Self {
        Self {
            resolution: "1080p".to_string(),
            fps: 60,
            bitrate_mbps: 15,
            codec: "h264".to_string(),
            hdr: false,
        }
    }
}

/// Performance switches for the video path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PerformanceSettings {
    /// Use NVDEC / D3D11VA when the PC has an NVIDIA GPU.
    pub prefer_nvidia: bool,
    /// Avoid extra present queues where the driver allows it.
    pub low_latency_present: bool,
}

impl Default for PerformanceSettings {
    fn default() -> Self {
        Self {
            prefer_nvidia: true,
            low_latency_present: true,
        }
    }
}

/// Folder for config and logs.
pub fn data_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Glasswing")
}

/// Default config file path.
pub fn default_path() -> PathBuf {
    data_dir().join("config.toml")
}

/// Folder for log files.
pub fn log_dir() -> PathBuf {
    data_dir().join("logs")
}

/// Errors that can happen while loading or saving settings.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not read settings: {0}")]
    Read(#[from] std::io::Error),
    #[error("settings file is not valid TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("could not write settings: {0}")]
    Write(String),
}

impl Config {
    /// Default config file path.
    pub fn default_path() -> PathBuf {
        crate::config::default_path()
    }

    /// Folder for log files.
    pub fn log_dir() -> PathBuf {
        crate::config::log_dir()
    }

    /// Load settings. A missing file is not an error; it returns defaults.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        match fs::read_to_string(path) {
            Ok(text) => Ok(toml::from_str(&text)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(ConfigError::Read(e)),
        }
    }

    /// Save settings, creating the folder when needed.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let text = toml::to_string_pretty(self).map_err(|e| ConfigError::Write(e.to_string()))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let temp = path.with_extension("toml.tmp");
        fs::write(&temp, text)?;

        // Windows will not rename over an existing file.
        if path.exists() {
            fs::remove_file(path)?;
        }
        fs::rename(&temp, path).map_err(|e| ConfigError::Write(e.to_string()))?;
        Ok(())
    }

    /// Check values before saving. Returns a message the UI can show.
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.stream.resolution.as_str(), "720p" | "1080p") {
            return Err("Resolution must be 720p or 1080p.".to_string());
        }
        if !matches!(self.stream.fps, 30 | 60) {
            return Err("Frame rate must be 30 or 60.".to_string());
        }
        if !(5..=50).contains(&self.stream.bitrate_mbps) {
            return Err("Bitrate must be between 5 and 50 Mbps.".to_string());
        }
        if self.stream.codec != "h264" {
            return Err("Only H.264 is supported in this version.".to_string());
        }
        if let Some(account) = &self.psn_account_id {
            if account.len() > 200 {
                return Err("PSN account ID is too long.".to_string());
            }
        }
        for host in &self.hosts {
            if host.id.trim().is_empty()
                || host.name.trim().is_empty()
                || host.address.trim().is_empty()
            {
                return Err("Every console needs a name and an address.".to_string());
            }
        }
        Ok(())
    }

    /// Add a console, or replace the one with the same ID or address.
    pub fn upsert_host(&mut self, host: SavedHost) {
        if let Some(existing) = self
            .hosts
            .iter_mut()
            .find(|h| h.id == host.id || h.address == host.address)
        {
            *existing = host;
        } else {
            self.hosts.push(host);
        }
    }

    /// Remove a console by ID. Returns true when one was removed.
    pub fn remove_host(&mut self, id: &str) -> bool {
        let before = self.hosts.len();
        self.hosts.retain(|host| host.id != id);
        self.hosts.len() != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_round_trip() {
        let config = Config::default();
        let text = toml::to_string_pretty(&config).unwrap();
        let back: Config = toml::from_str(&text).unwrap();
        assert_eq!(config, back);
    }

    #[test]
    fn missing_file_returns_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::load(&dir.path().join("config.toml")).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn saves_and_loads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("config.toml");
        let mut config = Config::default();
        config.stream.bitrate_mbps = 25;
        config.upsert_host(SavedHost {
            id: "abc".into(),
            name: "Living Room".into(),
            address: "192.168.1.50".into(),
            regist_key: None,
            last_seen: None,
            manual: true,
        });

        config.save(&path).unwrap();
        assert_eq!(Config::load(&path).unwrap(), config);
    }

    #[test]
    fn replaces_host_with_same_address() {
        let mut config = Config::default();
        let host = SavedHost {
            id: "one".into(),
            name: "Old".into(),
            address: "192.168.1.50".into(),
            regist_key: None,
            last_seen: None,
            manual: true,
        };
        config.upsert_host(host);
        config.upsert_host(SavedHost {
            id: "two".into(),
            name: "New".into(),
            address: "192.168.1.50".into(),
            regist_key: None,
            last_seen: None,
            manual: true,
        });
        assert_eq!(config.hosts.len(), 1);
        assert_eq!(config.hosts[0].name, "New");
    }

    #[test]
    fn rejects_bad_values() {
        let mut config = Config::default();
        config.stream.bitrate_mbps = 100;
        assert!(config.validate().is_err());

        config.stream.bitrate_mbps = 15;
        config.stream.fps = 120;
        assert!(config.validate().is_err());
    }
}
