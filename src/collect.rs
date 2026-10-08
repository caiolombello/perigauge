//! One collection pass over all enabled backends, serialized across processes
//! so the daemon and a manual refresh never interleave HID requests.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;

use crate::backends::{galaxy_buds, hidpp, keychron, upower};
use crate::config::{self, Config};
use crate::hidraw;
use crate::model::{Device, Issue, SCHEMA_VERSION, Snapshot, now};
use crate::state;

pub struct Lock {
    _file: File,
}

/// Exclusive advisory lock held while reading, collecting and publishing.
/// It lives next to `state.json`, so every process sharing that file also
/// shares the lock, whatever its XDG_RUNTIME_DIR.
pub fn lock() -> io::Result<Lock> {
    let dir = config::state_dir();
    config::ensure_private_dir(&dir)?;
    let file =
        OpenOptions::new().create(true).truncate(false).write(true).mode(0o600).open(dir.join("collect.lock"))?;
    // SAFETY: flock on a valid, owned descriptor; released when `file` drops.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(Lock { _file: file })
}

pub fn snapshot_path() -> std::path::PathBuf {
    config::state_dir().join("state.json")
}

pub fn load_snapshot() -> Option<Snapshot> {
    let text = fs::read_to_string(snapshot_path()).ok()?;
    serde_json::from_str::<Snapshot>(&text).ok().filter(|s| s.schema == SCHEMA_VERSION)
}

pub fn save_snapshot(snapshot: &Snapshot) -> io::Result<()> {
    let json = serde_json::to_vec(snapshot).map_err(io::Error::other)?;
    config::write_private(&snapshot_path(), &json)
}

fn load_json<T: serde::de::DeserializeOwned + Default>(name: &str) -> T {
    fs::read_to_string(config::state_dir().join(name))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_json<T: serde::Serialize>(name: &str, value: &T) {
    if let Ok(json) = serde_json::to_vec_pretty(value) {
        if let Err(error) = config::write_private(&config::state_dir().join(name), &json) {
            eprintln!("perigauge: não foi possível gravar {name}: {error}");
        }
    }
}

pub struct Collector {
    hidpp_cache: hidpp::Cache,
    pub buds: Option<galaxy_buds::Monitor>,
}

impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector {
    pub fn new() -> Self {
        Collector { hidpp_cache: hidpp::Cache::default(), buds: None }
    }

    pub fn collect(&mut self, config: &Config) -> (Vec<Device>, Vec<Issue>) {
        let nodes = hidraw::enumerate();
        let mut devices = Vec::new();
        let mut issues = Vec::new();
        if config.backends.keychron {
            for device in keychron::collect(&nodes) {
                if device.error.as_deref() == Some("permission-denied") {
                    issues.push(Issue {
                        code: "hidraw-permission-denied".into(),
                        detail: "Keychron Ultra-Link 8K: instale a regra udev do PeriGauge".into(),
                    });
                }
                devices.push(device);
            }
        }
        if config.backends.hidpp {
            let before = self.hidpp_cache.clone();
            let outcome = hidpp::collect(&nodes, &mut self.hidpp_cache);
            if !outcome.permission_denied.is_empty() {
                issues.push(Issue {
                    code: "hidraw-permission-denied".into(),
                    detail: format!(
                        "Logitech HID++ ({}): instale a regra udev do PeriGauge",
                        outcome.permission_denied.join(", ")
                    ),
                });
            }
            devices.extend(outcome.devices);
            if self.hidpp_cache != before {
                save_json("hidpp.json", &self.hidpp_cache);
            }
        }
        if config.backends.galaxy_buds {
            if let Some(monitor) = &self.buds {
                devices.extend(monitor.devices());
            }
        }
        if config.backends.upower {
            match upower::collect() {
                Ok(found) => devices.extend(found),
                Err(error) => issues.push(Issue { code: "upower-unavailable".into(), detail: error }),
            }
        }
        (devices, issues)
    }

    /// Read, collect, merge and publish as one transaction under the lock.
    /// With `reuse_within`, a snapshot published by another process within
    /// that many seconds is returned instead of querying the hardware again.
    pub fn refresh(&mut self, config: &Config, source: &str, reuse_within: Option<f64>) -> Snapshot {
        let guard = match lock() {
            Ok(guard) => guard,
            Err(error) => {
                // Without the lock another process may be talking to the same
                // HID nodes; never query hardware unserialized.
                eprintln!("perigauge: lock: {error}");
                let mut snapshot = load_snapshot().unwrap_or_else(|| empty(source));
                snapshot.issues.push(Issue { code: "lock-unavailable".into(), detail: error.to_string() });
                return snapshot;
            }
        };
        let previous = load_snapshot();
        if let (Some(max_age), Some(stored)) = (reuse_within, &previous) {
            if now() - stored.generated_at < max_age {
                return stored.clone();
            }
        }
        self.hidpp_cache = load_json("hidpp.json");
        let (mut fresh, issues) = self.collect(config);
        let now = now();
        let previous = previous.map(|p| p.devices).unwrap_or_default();
        // Without its own earbud monitor (manual refresh), keep the daemon's
        // recent earbud readings instead of marking the earbuds as gone.
        if self.buds.is_none() {
            let window = 3.0 * config.interval_seconds as f64;
            fresh.extend(
                previous
                    .iter()
                    .filter(|d| d.backend == "galaxy-buds" && d.present)
                    .filter(|d| d.updated_at.is_some_and(|at| now - at < window))
                    .cloned(),
            );
        }
        let snapshot = Snapshot {
            schema: SCHEMA_VERSION,
            version: env!("CARGO_PKG_VERSION").to_string(),
            generated_at: now,
            source: source.to_string(),
            devices: state::merge(&previous, fresh, now, config),
            issues,
        };
        if let Err(error) = save_snapshot(&snapshot) {
            eprintln!("perigauge: state.json: {error}");
        }
        drop(guard);
        snapshot
    }
}

fn empty(source: &str) -> Snapshot {
    Snapshot {
        schema: SCHEMA_VERSION,
        version: env!("CARGO_PKG_VERSION").to_string(),
        generated_at: now(),
        source: source.to_string(),
        devices: Vec::new(),
        issues: Vec::new(),
    }
}

pub fn load_notify_memory() -> std::collections::BTreeMap<String, crate::notify::Memory> {
    load_json("notify.json")
}

pub fn save_notify_memory(memory: &std::collections::BTreeMap<String, crate::notify::Memory>) {
    save_json("notify.json", memory);
}
