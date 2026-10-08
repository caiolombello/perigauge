//! User configuration (`$XDG_CONFIG_HOME/perigauge/config.toml`) and XDG paths.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Notifications {
    pub enabled: bool,
    pub warning: u8,
    pub critical: u8,
    pub shutdown: u8,
    pub charged: bool,
    pub hysteresis: u8,
}

impl Default for Notifications {
    fn default() -> Self {
        Notifications { enabled: true, warning: 20, critical: 10, shutdown: 5, charged: true, hysteresis: 5 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Backends {
    pub keychron: bool,
    pub hidpp: bool,
    pub galaxy_buds: bool,
    pub upower: bool,
}

impl Default for Backends {
    fn default() -> Self {
        Backends { keychron: true, hidpp: true, galaxy_buds: true, upower: true }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceConfig {
    pub name: Option<String>,
    pub hidden: bool,
    /// `None` follows the global setting.
    pub notifications: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub interval_seconds: u64,
    /// "auto" follows the session locale; "pt" or "en" force a language.
    pub language: String,
    pub notifications: Notifications,
    pub backends: Backends,
    pub devices: BTreeMap<String, DeviceConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            interval_seconds: 30,
            language: "auto".into(),
            notifications: Notifications::default(),
            backends: Backends::default(),
            devices: BTreeMap::new(),
        }
    }
}

impl Config {
    /// Parse and repair out-of-range values, returning human-readable warnings.
    pub fn parse(text: &str) -> Result<(Config, Vec<String>), String> {
        let mut config: Config = toml::from_str(text).map_err(|e| e.to_string())?;
        let mut warnings = Vec::new();
        if !(10..=3600).contains(&config.interval_seconds) {
            warnings.push(format!("interval_seconds={} fora de 10..3600; usando 30", config.interval_seconds));
            config.interval_seconds = 30;
        }
        if !matches!(config.language.as_str(), "auto" | "pt" | "en") {
            warnings.push(format!("language={:?} inválido (auto, pt, en); usando auto", config.language));
            config.language = "auto".into();
        }
        let n = &config.notifications;
        let ordered = 0 < n.shutdown && n.shutdown < n.critical && n.critical < n.warning && n.warning < 100;
        if !ordered || n.hysteresis > 20 {
            warnings.push("limites de notificação inválidos (exige 0 < shutdown < critical < warning < 100, hysteresis <= 20); usando padrões".into());
            config.notifications = Notifications { enabled: n.enabled, charged: n.charged, ..Notifications::default() };
        }
        Ok((config, warnings))
    }

    pub fn load() -> (Config, Vec<String>) {
        let path = config_dir().join("config.toml");
        match fs::read_to_string(&path) {
            Ok(text) => Config::parse(&text).unwrap_or_else(|error| {
                (Config::default(), vec![format!("{}: {error}; usando padrões", path.display())])
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => (Config::default(), Vec::new()),
            Err(error) => (Config::default(), vec![format!("{}: {error}", path.display())]),
        }
    }

    pub fn portuguese(&self) -> bool {
        match self.language.as_str() {
            "pt" => true,
            "en" => false,
            _ => crate::notify::portuguese(),
        }
    }

    pub fn device(&self, id: &str) -> DeviceConfig {
        self.devices.get(id).cloned().unwrap_or_default()
    }
}

fn xdg(variable: &str, fallback: &str) -> PathBuf {
    env::var_os(variable).map(PathBuf::from).filter(|path| path.is_absolute()).unwrap_or_else(|| home().join(fallback))
}

fn home() -> PathBuf {
    env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

pub fn config_dir() -> PathBuf {
    xdg("XDG_CONFIG_HOME", ".config").join("perigauge")
}

pub fn state_dir() -> PathBuf {
    xdg("XDG_STATE_HOME", ".local/state").join("perigauge")
}

pub fn runtime_dir() -> PathBuf {
    env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .map(|path| path.join("perigauge"))
        .unwrap_or_else(|| state_dir().join("run"))
}

pub fn ensure_private_dir(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

/// Atomically replace `path` with `contents`, mode 0600.
pub fn write_private(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| io::Error::other("path without parent"))?;
    ensure_private_dir(parent)?;
    let temporary =
        parent.join(format!(".{}.{}.tmp", path.file_name().unwrap_or_default().to_string_lossy(), std::process::id()));
    let result = (|| {
        let mut file = fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&temporary)?;
        file.write_all(contents)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config_uses_defaults() {
        let (config, warnings) = Config::parse("").unwrap();
        assert_eq!(config, Config::default());
        assert!(warnings.is_empty());
    }

    #[test]
    fn parses_device_overrides() {
        let text = r#"
interval_seconds = 60
language = "pt"
[notifications]
warning = 25
[devices."keychron:3434:d028"]
name = "Keychron M6"
[devices."bt:aabbcc001122"]
hidden = true
notifications = false
"#;
        let (config, warnings) = Config::parse(text).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(config.interval_seconds, 60);
        assert!(config.portuguese());
        assert_eq!(config.notifications.warning, 25);
        assert_eq!(config.notifications.critical, 10);
        assert_eq!(config.device("keychron:3434:d028").name.as_deref(), Some("Keychron M6"));
        assert!(config.device("bt:aabbcc001122").hidden);
        assert_eq!(config.device("bt:aabbcc001122").notifications, Some(false));
        assert_eq!(config.device("missing"), DeviceConfig::default());
    }

    #[test]
    fn repairs_invalid_thresholds_and_interval() {
        let (config, warnings) = Config::parse(
            "interval_seconds = 1\nlanguage = \"de\"\n[notifications]\nwarning = 5\ncritical = 10\ncharged = false\n",
        )
        .unwrap();
        assert_eq!(warnings.len(), 3);
        assert_eq!(config.language, "auto");
        assert_eq!(config.interval_seconds, 30);
        assert_eq!(config.notifications.warning, 20);
        assert!(!config.notifications.charged, "unrelated choices are preserved");
    }

    #[test]
    fn rejects_malformed_toml() {
        assert!(Config::parse("interval_seconds = [").is_err());
    }

    #[test]
    fn writes_private_files_atomically() {
        let dir = std::env::temp_dir().join(format!("perigauge-test-{}", std::process::id()));
        let file = dir.join("state.json");
        write_private(&file, b"{}").unwrap();
        assert_eq!(fs::read(&file).unwrap(), b"{}");
        assert_eq!(fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(fs::metadata(&dir).unwrap().permissions().mode() & 0o777, 0o700);
        fs::remove_dir_all(dir).unwrap();
    }
}
