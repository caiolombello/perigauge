//! Device model and the versioned JSON contract consumed by the frontends.

use serde::{Deserialize, Serialize};

/// Bumped only on incompatible changes; additive fields keep the version.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Mouse,
    Keyboard,
    Earbuds,
    Headset,
    Touchpad,
    Gamepad,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Link {
    Receiver,
    Bluetooth,
    Usb,
    Unknown,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Charging {
    Discharging,
    Charging,
    Full,
    #[default]
    Unknown,
}

impl Charging {
    pub fn on_power(self) -> bool {
        matches!(self, Charging::Charging | Charging::Full)
    }
}

/// Coarse level reported by devices that do not expose a percentage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Level {
    Critical,
    Low,
    Good,
    Full,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Battery {
    pub percent: Option<u8>,
    pub level: Option<Level>,
    pub charging: Charging,
    /// True when the percentage is derived (voltage curve, coarse level).
    #[serde(default)]
    pub estimated: bool,
}

impl Battery {
    pub fn is_known(&self) -> bool {
        self.percent.is_some() || self.level.is_some()
    }
}

/// Part of a device with its own battery (earbud left/right, charging case).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub percent: Option<u8>,
    #[serde(default)]
    pub charging: Charging,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Device {
    /// Stable across restarts and, where the hardware allows it, across links.
    pub id: String,
    pub name: String,
    pub kind: Kind,
    pub vendor: Option<String>,
    pub link: Link,
    /// Receiver or transport detail shown to the user, e.g. "Logi Bolt".
    pub via: Option<String>,
    pub backend: String,
    /// The link (receiver, Bluetooth) currently sees the device.
    pub present: bool,
    pub battery: Battery,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<Component>,
    /// Unix time of the last real battery reading.
    pub updated_at: Option<f64>,
    /// The battery shown is the last known value, not a current reading.
    #[serde(default)]
    pub stale: bool,
    #[serde(default)]
    pub hidden: bool,
    /// Machine-readable reason, e.g. "asleep", "permission-denied".
    pub error: Option<String>,
    /// Identifiers used to drop duplicates reported by generic backends.
    #[serde(skip)]
    pub match_keys: Vec<String>,
}

impl Device {
    pub fn new(id: impl Into<String>, name: impl Into<String>, kind: Kind, backend: &str) -> Self {
        Device {
            id: id.into(),
            name: name.into(),
            kind,
            vendor: None,
            link: Link::Unknown,
            via: None,
            backend: backend.to_string(),
            present: true,
            battery: Battery::default(),
            components: Vec::new(),
            updated_at: None,
            stale: false,
            hidden: false,
            error: None,
            match_keys: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub schema: u32,
    pub version: String,
    pub generated_at: f64,
    /// "daemon" or "oneshot".
    pub source: String,
    pub devices: Vec<Device>,
    /// Problems the user can act on, e.g. a missing udev rule.
    #[serde(default)]
    pub issues: Vec<Issue>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Issue {
    /// Stable code for frontends, e.g. "hidraw-permission-denied".
    pub code: String,
    pub detail: String,
}

pub fn now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// Lowercase, separator-free form used for MAC addresses and serials.
pub fn normalize_key(raw: &str) -> String {
    raw.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_contract_field_names() {
        let mut device = Device::new("keychron:3434:d028", "Keychron", Kind::Mouse, "keychron");
        device.battery.percent = Some(69);
        device.match_keys.push("secret-ish".into());
        let json = serde_json::to_value(&device).unwrap();
        assert_eq!(json["kind"], "mouse");
        assert_eq!(json["link"], "unknown");
        assert_eq!(json["battery"]["percent"], 69);
        assert_eq!(json["battery"]["charging"], "unknown");
        assert!(json.get("match_keys").is_none());
        assert!(json.get("components").is_none());
    }

    #[test]
    fn normalizes_mac_and_serial() {
        assert_eq!(normalize_key("AA:BB:CC:00:11:22"), "aabbcc001122");
        assert_eq!(normalize_key("4A-B2 1C"), "4ab21c");
    }
}
