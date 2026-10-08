//! Generic fallback through UPower: kernel power_supply devices (hidpp,
//! playstation, wacom...) and BlueZ Battery1 (BLE Battery Service, HFP).

use std::collections::HashMap;

use zbus::blocking::Connection;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

use crate::model::{Battery, Charging, Device, Kind, Level, Link, normalize_key};

const SERVICE: &str = "org.freedesktop.UPower";
const DEVICE_IFACE: &str = "org.freedesktop.UPower.Device";

/// Plain view of the UPower.Device properties we use, for testability.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Entry {
    pub path: String,
    pub native_path: String,
    pub model: String,
    pub serial: String,
    pub kind: u32,
    pub power_supply: bool,
    pub present: bool,
    pub percentage: f64,
    pub state: u32,
    pub battery_level: u32,
    pub update_time: u64,
}

fn kind(upower_type: u32) -> Option<Kind> {
    // UPower UpDeviceKind values.
    match upower_type {
        5 => Some(Kind::Mouse),
        6 => Some(Kind::Keyboard),
        12 => Some(Kind::Gamepad),
        14 => Some(Kind::Touchpad),
        17 | 19 => Some(Kind::Headset),
        8..=11 | 13 | 18 | 20..=28 => Some(Kind::Other),
        // 0 unknown, 1 line power, 2 battery, 3 ups, 4 monitor, 7 pda
        _ => None,
    }
}

fn charging(state: u32) -> Charging {
    match state {
        1 | 5 => Charging::Charging,
        2 | 3 | 6 => Charging::Discharging,
        4 => Charging::Full,
        _ => Charging::Unknown,
    }
}

fn level(battery_level: u32) -> Option<Level> {
    match battery_level {
        3 => Some(Level::Low),
        4 => Some(Level::Critical),
        6 | 7 => Some(Level::Good),
        8 => Some(Level::Full),
        _ => None,
    }
}

/// MAC address from a BlueZ native path such as `/org/bluez/hci0/dev_AA_BB_CC_00_11_22`.
pub fn bluez_mac(native_path: &str) -> Option<String> {
    let tail = native_path.rsplit('/').next()?.strip_prefix("dev_")?;
    (tail.len() == 17).then(|| normalize_key(tail))
}

pub fn device_from(entry: &Entry) -> Option<Device> {
    if entry.power_supply || entry.native_path.is_empty() {
        return None;
    }
    let kind = kind(entry.kind)?;
    let mac = bluez_mac(&entry.native_path);
    let id = match &mac {
        Some(mac) => format!("bt:{mac}"),
        None => format!("upower:{}", entry.native_path),
    };
    let name = if entry.model.is_empty() { entry.native_path.clone() } else { entry.model.clone() };
    let mut device = Device::new(id, name, kind, "upower");
    // Kernel power_supply entries do not tell the transport; leave it unknown.
    device.link = if mac.is_some() { Link::Bluetooth } else { Link::Unknown };
    if entry.native_path.starts_with("hidpp_battery") {
        device.vendor = Some("Logitech".into());
    }
    device.via = Some("UPower".into());
    device.present = entry.present;
    let coarse = level(entry.battery_level);
    device.battery = Battery {
        // battery_level "none" (1) means the percentage is real.
        percent: (entry.battery_level <= 1 && entry.present).then(|| entry.percentage.round().clamp(0.0, 100.0) as u8),
        level: coarse,
        charging: charging(entry.state),
        estimated: coarse.is_some(),
    };
    device.updated_at = (entry.update_time > 0).then_some(entry.update_time as f64);
    if let Some(mac) = &mac {
        device.match_keys.push(format!("bt:{mac}"));
    }
    if !entry.serial.is_empty() {
        device.match_keys.push(format!("serial:{}", normalize_key(&entry.serial)));
        if entry.serial.len() == 17 {
            device.match_keys.push(format!("bt:{}", normalize_key(&entry.serial)));
        }
    }
    if !entry.model.is_empty() {
        device.match_keys.push(format!("name:{}", entry.model.to_lowercase()));
    }
    Some(device)
}

fn get<T>(props: &HashMap<String, OwnedValue>, key: &str) -> Option<T>
where
    T: TryFrom<OwnedValue>,
{
    props.get(key).and_then(|value| value.try_clone().ok()).and_then(|value| T::try_from(value).ok())
}

fn entry(connection: &Connection, path: &OwnedObjectPath) -> zbus::Result<Entry> {
    let reply = connection.call_method(
        Some(SERVICE),
        path.as_str(),
        Some("org.freedesktop.DBus.Properties"),
        "GetAll",
        &(DEVICE_IFACE,),
    )?;
    let props: HashMap<String, OwnedValue> = reply.body().deserialize()?;
    Ok(Entry {
        path: path.to_string(),
        native_path: get::<String>(&props, "NativePath").unwrap_or_default(),
        model: get::<String>(&props, "Model").unwrap_or_default(),
        serial: get::<String>(&props, "Serial").unwrap_or_default(),
        kind: get::<u32>(&props, "Type").unwrap_or(0),
        power_supply: get::<bool>(&props, "PowerSupply").unwrap_or(false),
        present: get::<bool>(&props, "IsPresent").unwrap_or(true),
        percentage: get::<f64>(&props, "Percentage").unwrap_or(0.0),
        state: get::<u32>(&props, "State").unwrap_or(0),
        battery_level: get::<u32>(&props, "BatteryLevel").unwrap_or(0),
        update_time: get::<u64>(&props, "UpdateTime").unwrap_or(0),
    })
}

pub fn collect() -> Result<Vec<Device>, String> {
    let connection = crate::bus::system().map_err(|e| e.to_string())?;
    let reply = connection
        .call_method(Some(SERVICE), "/org/freedesktop/UPower", Some(SERVICE), "EnumerateDevices", &())
        .map_err(|e| e.to_string())?;
    let paths: Vec<OwnedObjectPath> = reply.body().deserialize().map_err(|e| e.to_string())?;
    Ok(paths.iter().filter_map(|path| entry(&connection, path).ok()).filter_map(|entry| device_from(&entry)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buds() -> Entry {
        Entry {
            path: "/org/freedesktop/UPower/devices/headset_dev_AA_BB_CC_00_11_22".into(),
            native_path: "/org/bluez/hci0/dev_AA_BB_CC_00_11_22".into(),
            model: "Galaxy Buds3 Pro (1122)".into(),
            kind: 17,
            present: true,
            percentage: 70.0,
            state: 2,
            battery_level: 1,
            update_time: 1_791_428_533,
            ..Entry::default()
        }
    }

    #[test]
    fn maps_bluez_headset() {
        let device = device_from(&buds()).unwrap();
        assert_eq!(device.id, "bt:aabbcc001122");
        assert_eq!(device.kind, Kind::Headset);
        assert_eq!(device.link, Link::Bluetooth);
        assert_eq!(device.battery.percent, Some(70));
        assert_eq!(device.battery.charging, Charging::Discharging);
        assert!(device.match_keys.contains(&"bt:aabbcc001122".to_string()));
    }

    #[test]
    fn skips_system_batteries_and_line_power() {
        let laptop = Entry { native_path: "BAT0".into(), kind: 2, power_supply: true, ..Entry::default() };
        assert!(device_from(&laptop).is_none());
        let ac = Entry { native_path: "AC".into(), kind: 1, power_supply: true, ..Entry::default() };
        assert!(device_from(&ac).is_none());
        let unknown = Entry { native_path: "x".into(), kind: 0, ..Entry::default() };
        assert!(device_from(&unknown).is_none());
    }

    #[test]
    fn coarse_levels_do_not_invent_percentages() {
        let coarse = Entry { battery_level: 3, percentage: 40.0, ..buds() };
        let device = device_from(&coarse).unwrap();
        assert_eq!(device.battery.percent, None);
        assert_eq!(device.battery.level, Some(Level::Low));
        assert!(device.battery.estimated);
    }

    #[test]
    fn kernel_hidpp_battery_keeps_serial_for_dedupe() {
        let mx = Entry {
            native_path: "hidpp_battery_0".into(),
            model: "MX Keys".into(),
            serial: "4A-B2-1C-9F".into(),
            kind: 6,
            present: true,
            percentage: 55.0,
            state: 2,
            battery_level: 1,
            ..Entry::default()
        };
        let device = device_from(&mx).unwrap();
        assert_eq!(device.id, "upower:hidpp_battery_0");
        assert!(device.match_keys.contains(&"serial:4ab21c9f".to_string()));
        assert!(device.match_keys.contains(&"name:mx keys".to_string()));
    }

    #[test]
    fn extracts_mac_from_native_path() {
        assert_eq!(bluez_mac("/org/bluez/hci0/dev_AA_BB_CC_00_11_22").as_deref(), Some("aabbcc001122"));
        assert_eq!(bluez_mac("hidpp_battery_0"), None);
    }
}
