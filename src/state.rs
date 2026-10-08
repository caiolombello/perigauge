//! Combining backend readings: duplicate removal, last-known values for
//! sleeping devices, retention of recently seen devices and user overrides.

use std::collections::{HashMap, HashSet};

use crate::config::Config;
use crate::model::{Charging, Device};

/// Devices not seen for longer than this disappear from the snapshot.
pub const RETENTION_SECONDS: f64 = 7.0 * 24.0 * 3600.0;

const GENERIC: usize = 3;

/// Specialized backends first: they know more than the generic fallback.
fn priority(backend: &str) -> usize {
    match backend {
        "keychron" => 0,
        "hidpp" => 1,
        "galaxy-buds" => 2,
        _ => GENERIC,
    }
}

/// Keys a generic reading may share with a specialized one. The model name is
/// only trusted when the generic reading has no strong identity (MAC/serial),
/// or for the kernel's Logitech batteries, whose serial format may differ.
fn generic_keys(device: &Device) -> Vec<String> {
    let mut keys = vec![format!("id:{}", device.id)];
    let strong: Vec<String> =
        device.match_keys.iter().filter(|k| k.starts_with("bt:") || k.starts_with("serial:")).cloned().collect();
    let kernel_logitech = device.id.starts_with("upower:hidpp_battery");
    if strong.is_empty() || kernel_logitech {
        keys.push(format!("name:{}", device.name.to_lowercase()));
    }
    keys.extend(strong);
    keys
}

fn specialized_keys(device: &Device) -> Vec<String> {
    let mut keys = vec![format!("id:{}", device.id), format!("name:{}", device.name.to_lowercase())];
    keys.extend(device.match_keys.iter().cloned());
    keys
}

/// Drop generic (UPower) readings of devices a specialized backend handles,
/// using the generic battery only while the specialized one has none yet.
pub fn dedupe(mut devices: Vec<Device>) -> Vec<Device> {
    devices.sort_by_key(|device| priority(&device.backend));
    let mut kept: Vec<Device> = Vec::new();
    let mut owners: HashMap<String, usize> = HashMap::new();
    for device in devices {
        let generic = priority(&device.backend) == GENERIC;
        let owner = if generic {
            generic_keys(&device).iter().find_map(|key| owners.get(key).copied())
        } else {
            // Distinct specialized devices may share a model name.
            owners.get(&format!("id:{}", device.id)).copied()
        };
        if let Some(index) = owner {
            let target = &mut kept[index];
            if generic && !target.battery.is_known() && device.battery.is_known() {
                target.battery = device.battery.clone();
                target.updated_at = device.updated_at;
                if target.error.as_deref() == Some("waiting-for-status") {
                    target.error = None;
                }
            }
            continue;
        }
        if generic {
            if kept.iter().any(|other| other.id == device.id) {
                continue;
            }
        } else {
            for key in specialized_keys(&device) {
                owners.entry(key).or_insert(kept.len());
            }
        }
        kept.push(device);
    }
    kept
}

/// Nothing current to show: no percentage, no level and no charging state.
fn empty_reading(device: &Device) -> bool {
    device.battery.percent.is_none() && device.battery.level.is_none() && device.battery.charging == Charging::Unknown
}

pub fn merge(previous: &[Device], fresh: Vec<Device>, now: f64, config: &Config) -> Vec<Device> {
    let mut devices: Vec<Device> = dedupe(fresh)
        .into_iter()
        .map(|mut device| {
            let earlier = previous.iter().find(|old| old.id == device.id);
            if empty_reading(&device) {
                if let Some(old) = earlier.filter(|old| old.battery.is_known()) {
                    device.battery = old.battery.clone();
                    device.updated_at = old.updated_at;
                    device.stale = true;
                }
            }
            if device.components.is_empty() && device.stale {
                if let Some(old) = earlier {
                    device.components = old.components.clone();
                }
            }
            device
        })
        .collect();

    let fresh_ids: HashSet<String> = devices.iter().map(|d| d.id.clone()).collect();
    let specialized_names: HashSet<String> =
        devices.iter().filter(|d| priority(&d.backend) != GENERIC).map(|d| d.name.to_lowercase()).collect();
    for old in previous {
        let recent = old.updated_at.is_some_and(|at| now - at < RETENTION_SECONDS);
        // A generic entry now reported by a specialized backend is the same device.
        let migrated = priority(&old.backend) == GENERIC && specialized_names.contains(&old.name.to_lowercase());
        if fresh_ids.contains(&old.id) || migrated || !recent {
            continue;
        }
        let mut gone = old.clone();
        gone.present = false;
        gone.stale = true;
        gone.error = None;
        devices.push(gone);
    }

    for device in &mut devices {
        let overrides = config.device(&device.id);
        if let Some(name) = overrides.name.filter(|name| !name.trim().is_empty()) {
            device.name = name;
        }
        device.hidden = overrides.hidden;
    }
    devices.sort_by(|a, b| {
        b.present.cmp(&a.present).then(a.kind.cmp(&b.kind)).then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    devices
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DeviceConfig;
    use crate::model::{Battery, Kind};

    fn device(id: &str, name: &str, backend: &str, percent: Option<u8>) -> Device {
        let mut device = Device::new(id, name, Kind::Mouse, backend);
        device.battery = Battery { percent, charging: Charging::Discharging, ..Battery::default() };
        device.updated_at = percent.map(|_| 1000.0);
        device
    }

    #[test]
    fn specialized_backend_wins_over_upower() {
        let mut generic = device("bt:aabbccddeeff", "Galaxy Buds3 Pro", "upower", Some(60));
        generic.match_keys.push("bt:aabbccddeeff".into());
        let mut buds = device("bt:aabbccddeeff", "Galaxy Buds3 Pro", "galaxy-buds", Some(62));
        buds.match_keys.push("bt:aabbccddeeff".into());
        let mut kernel = device("upower:hidpp_battery_0", "MX Master 3S", "upower", Some(40));
        kernel.match_keys.push("serial:4ab21c9f".into());
        let mut hidpp = device("hidpp:4ab21c9f", "MX Master 3S", "hidpp", Some(41));
        hidpp.match_keys.push("serial:4ab21c9f".into());
        let result = dedupe(vec![generic, kernel, buds, hidpp]);
        let backends: Vec<_> = result.iter().map(|d| d.backend.as_str()).collect();
        assert_eq!(backends, vec!["hidpp", "galaxy-buds"]);
    }

    #[test]
    fn generic_devices_with_the_same_model_name_are_distinct() {
        let mut a = device("bt:111111111111", "WH-1000XM5", "upower", Some(50));
        a.match_keys.push("bt:111111111111".into());
        let mut b = device("bt:222222222222", "WH-1000XM5", "upower", Some(70));
        b.match_keys.push("bt:222222222222".into());
        assert_eq!(dedupe(vec![a, b]).len(), 2);
    }

    #[test]
    fn specialized_devices_with_the_same_model_name_are_distinct() {
        let a = device("hidpp:aaaa", "MX Keys", "hidpp", Some(50));
        let b = device("hidpp:bbbb", "MX Keys", "hidpp", Some(70));
        assert_eq!(dedupe(vec![a, b]).len(), 2);
    }

    #[test]
    fn specialized_device_without_reading_borrows_the_generic_battery() {
        let mut buds = device("bt:aabbccddeeff", "Galaxy Buds3 Pro", "galaxy-buds", None);
        buds.error = Some("waiting-for-status".into());
        let mut generic = device("bt:aabbccddeeff", "Galaxy Buds3 Pro", "upower", Some(60));
        generic.match_keys.push("bt:aabbccddeeff".into());
        let result = dedupe(vec![generic, buds]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].backend, "galaxy-buds");
        assert_eq!(result[0].battery.percent, Some(60));
        assert_eq!(result[0].error, None);
    }

    #[test]
    fn current_charging_state_is_not_replaced_by_an_old_percentage() {
        let previous = vec![device("hidpp:1", "MX Keys", "hidpp", Some(40))];
        let mut charging = device("hidpp:1", "MX Keys", "hidpp", None);
        charging.battery.charging = Charging::Charging;
        let merged = merge(&previous, vec![charging], 2000.0, &Config::default());
        assert_eq!((merged[0].battery.percent, merged[0].stale), (None, false));
        assert_eq!(merged[0].battery.charging, Charging::Charging);
    }

    #[test]
    fn unrelated_generic_devices_are_kept() {
        let result = dedupe(vec![
            device("keychron:3434:d028", "Keychron", "keychron", Some(69)),
            device("upower:ps", "DualSense", "upower", Some(80)),
        ]);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn sleeping_device_keeps_last_known_value_as_stale() {
        let previous = vec![device("keychron:3434:d028", "Keychron", "keychron", Some(69))];
        let mut asleep = device("keychron:3434:d028", "Keychron", "keychron", None);
        asleep.battery.charging = Charging::Unknown;
        asleep.error = Some("asleep".into());
        let merged = merge(&previous, vec![asleep], 2000.0, &Config::default());
        assert_eq!(merged[0].battery.percent, Some(69));
        assert!(merged[0].stale);
        assert_eq!(merged[0].updated_at, Some(1000.0));
        assert_eq!(merged[0].error.as_deref(), Some("asleep"));
    }

    #[test]
    fn recently_seen_devices_are_retained_as_absent() {
        let previous =
            [device("hidpp:1", "MX Keys", "hidpp", Some(50)), device("hidpp:2", "Old Mouse", "hidpp", Some(50))];
        let mut ancient = previous[1].clone();
        ancient.updated_at = Some(0.0);
        let now = RETENTION_SECONDS + 10.0;
        let fresh_previous = vec![Device { updated_at: Some(now - 60.0), ..previous[0].clone() }, ancient];
        let merged = merge(&fresh_previous, Vec::new(), now, &Config::default());
        assert_eq!(merged.len(), 1);
        assert!(!merged[0].present);
        assert!(merged[0].stale);
    }

    #[test]
    fn device_migrating_backend_is_not_duplicated() {
        let previous = vec![device("upower:hidpp_battery_0", "MX Master 3S", "upower", Some(40))];
        let merged = merge(
            &previous,
            vec![device("hidpp:4ab21c9f", "MX Master 3S", "hidpp", Some(41))],
            1001.0,
            &Config::default(),
        );
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].backend, "hidpp");
    }

    #[test]
    fn applies_overrides_and_sorts_present_first() {
        let mut config = Config::default();
        config.devices.insert(
            "keychron:3434:d028".into(),
            DeviceConfig { name: Some("Keychron M6".into()), ..DeviceConfig::default() },
        );
        config.devices.insert("upower:x".into(), DeviceConfig { hidden: true, ..DeviceConfig::default() });
        let previous = vec![device("hidpp:1", "A keyboard", "hidpp", Some(50))];
        let merged = merge(
            &previous,
            vec![
                device("upower:x", "Gadget", "upower", Some(10)),
                device("keychron:3434:d028", "Keychron", "keychron", Some(69)),
            ],
            1001.0,
            &config,
        );
        assert_eq!(merged[0].name, "Gadget");
        assert!(merged[0].hidden);
        assert_eq!(merged[1].name, "Keychron M6");
        assert!(!merged[2].present);
    }
}
