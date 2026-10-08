//! Logitech HID++ 2.0 over hidraw, for Bolt/Unifying receivers and Bluetooth.
//!
//! Replaces the Solaar dependency for battery reading. Only read-only feature
//! functions are sent: root ping/getFeature, DEVICE_FW_VERSION (0x0003),
//! DEVICE_NAME (0x0005) and the battery features 0x1004, 0x1000 and 0x1001.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::hidraw::{self, BUS_BLUETOOTH, BUS_USB, Hidraw, HidrawInfo};
use crate::model::{Battery, Charging, Device, Kind, Level, Link, normalize_key, now};

const LOGITECH: u16 = 0x046d;
pub const SHORT: u8 = 0x10;
pub const LONG: u8 = 0x11;
const SHORT_LEN: usize = 7;
const LONG_LEN: usize = 20;
const DIRECT: u8 = 0xFF;
const ERROR_10: u8 = 0x8F;
const ERROR_20: u8 = 0xFF;
const PING_MARK: u8 = 0x5A;
const REQUEST_TIMEOUT: Duration = Duration::from_millis(1500);
/// Offline devices are reported by the receiver almost at once; a silent slot
/// should not hold the whole collection for long.
const PING_TIMEOUT: Duration = Duration::from_millis(1000);
/// Upper bound for one HID++ collection across every receiver and slot.
pub const COLLECTION_BUDGET: Duration = Duration::from_secs(6);

const FEATURE_FW_VERSION: u16 = 0x0003;
const FEATURE_DEVICE_NAME: u16 = 0x0005;
const FEATURE_UNIFIED_BATTERY: u16 = 0x1004;
const FEATURE_BATTERY_STATUS: u16 = 0x1000;
const FEATURE_BATTERY_VOLTAGE: u16 = 0x1001;

/// HID++ 1.0 error codes relevant to ping.
const ERR_INVALID_SUB_ID: u8 = 0x01;
const ERR_CONNECT_FAILED: u8 = 0x04;
const ERR_UNKNOWN_DEVICE: u8 = 0x08;
const ERR_RESOURCE: u8 = 0x09;

/// Receivers whose HID++ interface addresses paired devices by slot 1..=6.
const RECEIVERS: &[(u16, &str)] = &[
    (0xc548, "Logi Bolt"),
    (0xc52b, "Unifying"),
    (0xc532, "Unifying"),
    (0xc534, "Logitech nano receiver"),
    (0xc539, "Lightspeed"),
    (0xc53a, "Lightspeed"),
    (0xc53f, "Lightspeed"),
    (0xc547, "Lightspeed"),
    (0xc54d, "Lightspeed"),
];

pub fn receiver_name(product: u16) -> Option<&'static str> {
    RECEIVERS.iter().find(|(id, _)| *id == product).map(|(_, name)| *name)
}

/// Whether a hidraw node carries a HID++ collection (vendor page 0xFF00 or 0xFF43).
pub fn is_hidpp_interface(info: &HidrawInfo) -> bool {
    if info.vendor != LOGITECH {
        return false;
    }
    let pages = hidraw::usage_pages(&info.descriptor);
    let ids = hidraw::report_ids(&info.descriptor);
    pages.iter().any(|page| matches!(page, 0xFF00 | 0xFF43)) && ids.iter().any(|id| matches!(id, &SHORT | &LONG))
}

pub fn build_request(report: u8, device: u8, feature: u8, function_sw: u8, params: &[u8]) -> Vec<u8> {
    let length = if report == SHORT { SHORT_LEN } else { LONG_LEN };
    let mut buffer = vec![0u8; length];
    buffer[0] = report;
    buffer[1] = device;
    buffer[2] = feature;
    buffer[3] = function_sw;
    let room = length - 4;
    buffer[4..4 + params.len().min(room)].copy_from_slice(&params[..params.len().min(room)]);
    buffer
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Ok(Vec<u8>),
    Hidpp10Error(u8),
    Hidpp20Error(u8),
}

/// Match an input report against a request; `None` means unrelated traffic.
pub fn match_reply(report: &[u8], device: u8, feature: u8, function_sw: u8) -> Option<Reply> {
    if report.len() < 4 || !matches!(report[0], SHORT | LONG) {
        return None;
    }
    // Bluetooth devices may answer a 0xFF request with device number 0x00.
    if report[1] != device && report[1] != device ^ 0xFF {
        return None;
    }
    if report.len() >= 6 && report[3] == feature && report[4] == function_sw {
        if report[2] == ERROR_10 && report[0] == SHORT {
            return Some(Reply::Hidpp10Error(report[5]));
        }
        if report[2] == ERROR_20 {
            return Some(Reply::Hidpp20Error(report[5]));
        }
    }
    (report[2] == feature && report[3] == function_sw).then(|| Reply::Ok(report[4..].to_vec()))
}

pub fn battery_from_unified(params: &[u8]) -> Option<Battery> {
    let [soc, level, status, ..] = params else { return None };
    let soc = if *soc == 0 && *status == 3 { 100 } else { *soc };
    Some(Battery {
        percent: (soc > 0).then_some(soc.min(100)),
        level: match level {
            8 => Some(Level::Full),
            4 => Some(Level::Good),
            2 => Some(Level::Low),
            1 => Some(Level::Critical),
            _ => None,
        },
        charging: charging_from_status(*status),
        estimated: soc == 0,
    })
}

pub fn battery_from_status(params: &[u8]) -> Option<Battery> {
    let [level, _next, status, ..] = params else { return None };
    // "Charge complete" may come without a level; the kernel reports it as 100 %.
    let level = if *level == 0 && *status == 3 { 100 } else { *level };
    Some(Battery {
        percent: (level > 0).then_some(level.min(100)),
        level: None,
        charging: charging_from_status(*status),
        estimated: false,
    })
}

/// Typical single-cell Li-ion discharge curve (mV, %), highest first.
const VOLTAGE_CURVE: &[(u16, u8)] = &[
    (4186, 100),
    (4067, 90),
    (3989, 80),
    (3922, 70),
    (3859, 60),
    (3811, 50),
    (3778, 40),
    (3751, 30),
    (3717, 20),
    (3671, 10),
    (3646, 5),
    (3579, 2),
    (3500, 0),
];

pub fn percent_from_voltage(millivolts: u16) -> u8 {
    if millivolts >= VOLTAGE_CURVE[0].0 {
        return 100;
    }
    for pair in VOLTAGE_CURVE.windows(2) {
        let ((high_mv, high_pct), (low_mv, low_pct)) = (pair[0], pair[1]);
        if millivolts >= low_mv {
            let span = (high_mv - low_mv) as f32;
            let offset = (millivolts - low_mv) as f32;
            return (low_pct as f32 + (high_pct - low_pct) as f32 * offset / span).round() as u8;
        }
    }
    0
}

pub fn battery_from_voltage(params: &[u8]) -> Option<Battery> {
    let [high, low, flags, ..] = params else { return None };
    let millivolts = u16::from_be_bytes([*high, *low]);
    if millivolts == 0 {
        return None;
    }
    // Bit 7: external power, low bits: 0 charging, 1 full. Bits 3/4 give the
    // charge speed; otherwise bit 5 flags a critical battery.
    let charging = match (flags & 0x80 != 0, flags & 0x03) {
        (false, _) => Charging::Discharging,
        (true, 0) => Charging::Charging,
        (true, 1) => Charging::Full,
        (true, _) => Charging::Unknown,
    };
    let critical = flags & 0x18 == 0 && flags & 0x20 != 0;
    Some(Battery {
        percent: Some(percent_from_voltage(millivolts)),
        level: critical.then_some(Level::Critical),
        charging,
        estimated: true,
    })
}

fn charging_from_status(status: u8) -> Charging {
    match status {
        0 => Charging::Discharging,
        1 | 2 | 4 => Charging::Charging,
        3 => Charging::Full,
        _ => Charging::Unknown,
    }
}

pub fn kind_from_device_type(device_type: u8) -> Kind {
    match device_type {
        0 | 2 => Kind::Keyboard,
        3 | 5 => Kind::Mouse,
        4 => Kind::Touchpad,
        8 => Kind::Headset,
        12 => Kind::Gamepad,
        _ => Kind::Other,
    }
}

/// Identity remembered across runs so sleeping paired devices stay listed.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Known {
    pub id: String,
    pub name: String,
    pub kind: Option<Kind>,
    pub unit_id: Option<String>,
    #[serde(default)]
    pub features: BTreeMap<String, u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Cache {
    /// Keyed by "<receiver product>@<usb path>:<slot>" or "bt:<mac>".
    #[serde(default)]
    pub slots: BTreeMap<String, Known>,
}

#[derive(Debug)]
enum Failure {
    Timeout,
    Hidpp10(u8),
    Hidpp20,
    Io(String),
}

struct Channel {
    device: Hidraw,
    report: u8,
    sw_id: u8,
    timeout: Duration,
}

impl Channel {
    fn next_sw(&mut self) -> u8 {
        self.sw_id = if self.sw_id >= 0x0F { 0x02 } else { self.sw_id + 1 };
        self.sw_id
    }

    fn call(&mut self, number: u8, feature: u8, function: u8, params: &[u8]) -> Result<Vec<u8>, Failure> {
        let function_sw = (function << 4) | self.next_sw();
        self.device.drain();
        let request = build_request(self.report, number, feature, function_sw, params);
        self.device.write(&request).map_err(|e| Failure::Io(hidraw::error_code(&e)))?;
        let deadline = Instant::now() + self.timeout;
        loop {
            let report = self.device.read_until(deadline).map_err(|e| Failure::Io(hidraw::error_code(&e)))?;
            let Some(report) = report else { return Err(Failure::Timeout) };
            match match_reply(&report, number, feature, function_sw) {
                Some(Reply::Ok(params)) => return Ok(params),
                Some(Reply::Hidpp10Error(code)) => return Err(Failure::Hidpp10(code)),
                Some(Reply::Hidpp20Error(_)) => return Err(Failure::Hidpp20),
                None => {}
            }
        }
    }

    /// Returns the protocol major version if the device is online.
    fn ping(&mut self, number: u8) -> Result<u8, Failure> {
        self.timeout = PING_TIMEOUT;
        let reply = self.call(number, 0, 1, &[0, 0, PING_MARK]);
        self.timeout = REQUEST_TIMEOUT;
        let params = reply?;
        match params.as_slice() {
            [major, _minor, mark, ..] if *mark == PING_MARK => Ok(*major),
            _ => Err(Failure::Timeout),
        }
    }

    fn feature_index(&mut self, number: u8, feature: u16) -> Result<Option<u8>, Failure> {
        let [high, low] = feature.to_be_bytes();
        let params = self.call(number, 0, 0, &[high, low])?;
        Ok(params.first().copied().filter(|index| *index != 0))
    }
}

fn feature_key(feature: u16) -> String {
    format!("{feature:04x}")
}

/// Look up feature indexes once per device; they are stable per firmware.
fn features(channel: &mut Channel, number: u8, known: &mut Known) -> Result<(), Failure> {
    if !known.features.is_empty() {
        return Ok(());
    }
    // Publish the map only once every lookup succeeded, so a timeout halfway
    // through cannot leave a partial map that later probes would trust.
    let mut found = BTreeMap::new();
    for feature in [
        FEATURE_FW_VERSION,
        FEATURE_DEVICE_NAME,
        FEATURE_UNIFIED_BATTERY,
        FEATURE_BATTERY_STATUS,
        FEATURE_BATTERY_VOLTAGE,
    ] {
        if let Some(index) = channel.feature_index(number, feature)? {
            found.insert(feature_key(feature), index);
        }
    }
    // Remember that discovery ran even for devices without optional features.
    found.insert("0000".into(), 0);
    known.features = found;
    Ok(())
}

/// DEVICE_FW_VERSION getDeviceInfo: params[1..5] hold the unit id.
pub fn unit_id_from(params: &[u8]) -> Option<String> {
    params
        .get(1..5)
        .filter(|unit| unit.iter().any(|b| *b != 0))
        .map(|unit| unit.iter().map(|b| format!("{b:02x}")).collect())
}

fn identify(channel: &mut Channel, number: u8, known: &mut Known) -> Result<(), Failure> {
    // Read the unit id on every online probe: a different device may now be
    // paired in this receiver slot, and the cached identity must not leak.
    if let Some(&index) = known.features.get(&feature_key(FEATURE_FW_VERSION)) {
        let params = channel.call(number, index, 0, &[])?;
        let unit_id = unit_id_from(&params);
        if known.unit_id.is_some() && unit_id != known.unit_id {
            *known = Known::default();
            features(channel, number, known)?;
        }
        known.unit_id = unit_id;
    }
    if known.name.is_empty() {
        if let Some(&index) = known.features.get(&feature_key(FEATURE_DEVICE_NAME)) {
            let length = channel.call(number, index, 0, &[])?.first().copied().unwrap_or(0) as usize;
            let mut name = Vec::new();
            while name.len() < length {
                let fragment = channel.call(number, index, 1, &[name.len() as u8])?;
                let take = fragment.len().min(length - name.len());
                if take == 0 {
                    break;
                }
                name.extend_from_slice(&fragment[..take]);
            }
            known.name = String::from_utf8_lossy(&name).trim_end_matches('\0').trim().to_string();
            known.kind = channel.call(number, index, 2, &[])?.first().map(|t| kind_from_device_type(*t));
        }
    }
    Ok(())
}

fn read_battery(channel: &mut Channel, number: u8, known: &Known) -> Result<Option<Battery>, Failure> {
    let index = |feature| known.features.get(&feature_key(feature)).copied();
    if let Some(feature) = index(FEATURE_UNIFIED_BATTERY) {
        return Ok(battery_from_unified(&channel.call(number, feature, 1, &[])?));
    }
    if let Some(feature) = index(FEATURE_BATTERY_STATUS) {
        return Ok(battery_from_status(&channel.call(number, feature, 0, &[])?));
    }
    if let Some(feature) = index(FEATURE_BATTERY_VOLTAGE) {
        return Ok(battery_from_voltage(&channel.call(number, feature, 0, &[])?));
    }
    Ok(None)
}

fn device_from(known: &Known, link: Link, via: Option<&str>, mac: Option<&str>) -> Device {
    let name = if known.name.is_empty() { "Logitech".to_string() } else { known.name.clone() };
    let mut device = Device::new(known.id.clone(), name, known.kind.unwrap_or(Kind::Other), "hidpp");
    device.vendor = Some("Logitech".into());
    device.link = link;
    device.via = via.map(str::to_string);
    if let Some(unit) = &known.unit_id {
        device.match_keys.push(format!("serial:{unit}"));
    }
    if let Some(mac) = mac {
        device.match_keys.push(format!("bt:{}", normalize_key(mac)));
    }
    if !known.name.is_empty() {
        device.match_keys.push(format!("name:{}", known.name.to_lowercase()));
    }
    device
}

/// Probe one addressed device. Returns `None` when the slot is empty.
fn probe(
    channel: &mut Channel,
    number: u8,
    slot_key: &str,
    cache: &mut Cache,
    link: Link,
    via: Option<&str>,
    mac: Option<&str>,
) -> Option<Device> {
    let mut known = cache.slots.get(slot_key).cloned().unwrap_or_default();
    let offline = |known: &Known, error: &str| -> Option<Device> {
        if known.id.is_empty() {
            return None;
        }
        let mut device = device_from(known, link, via, mac);
        device.error = Some(error.into());
        Some(device)
    };
    match channel.ping(number) {
        Ok(major) if major >= 2 => {}
        Ok(_) | Err(Failure::Hidpp10(ERR_INVALID_SUB_ID)) => return None, // HID++ 1.0 device, unsupported
        Err(Failure::Hidpp10(ERR_UNKNOWN_DEVICE)) => {
            cache.slots.remove(slot_key);
            return None;
        }
        Err(Failure::Hidpp10(ERR_RESOURCE | ERR_CONNECT_FAILED)) | Err(Failure::Timeout) => {
            return offline(&known, "asleep");
        }
        Err(Failure::Io(code)) => return offline(&known, &code),
        Err(_) => return offline(&known, "protocol-error"),
    }
    let result = features(channel, number, &mut known)
        .and_then(|_| identify(channel, number, &mut known))
        .and_then(|_| read_battery(channel, number, &known));
    // A provisional id (unit id unknown at first contact) is replaced as soon
    // as the unit id is read, so the device keeps one identity across links.
    if let Some(unit) = &known.unit_id {
        known.id = format!("hidpp:{unit}");
    }
    if known.id.is_empty() {
        known.id = match (&known.unit_id, mac) {
            (Some(unit), _) => format!("hidpp:{unit}"),
            (None, Some(mac)) => format!("bt:{}", normalize_key(mac)),
            (None, None) => format!("hidpp:{slot_key}"),
        };
    }
    let mut device = device_from(&known, link, via, mac);
    match result {
        Ok(Some(battery)) => {
            device.battery = battery;
            device.updated_at = Some(now());
        }
        Ok(None) => device.error = Some("no-battery-info".into()),
        Err(Failure::Hidpp20) => {
            // Firmware may have changed feature indexes; rediscover next time.
            known.features.clear();
            device.error = Some("protocol-error".into());
        }
        Err(Failure::Timeout) => device.error = Some("asleep".into()),
        Err(Failure::Io(code)) => device.error = Some(code),
        Err(Failure::Hidpp10(_)) => device.error = Some("protocol-error".into()),
    }
    cache.slots.insert(slot_key.to_string(), known);
    Some(device)
}

/// Problems that the user can fix, reported once per collection.
#[derive(Default)]
pub struct Outcome {
    pub devices: Vec<Device>,
    pub permission_denied: Vec<String>,
}

/// Physical USB path of a receiver without the interface suffix, so two
/// identical receivers keep separate slot identities.
fn receiver_path(phys: &str) -> &str {
    phys.rsplit_once('/').map_or(phys, |(path, _)| path)
}

pub fn collect(nodes: &[HidrawInfo], cache: &mut Cache) -> Outcome {
    let deadline = Instant::now() + COLLECTION_BUDGET;
    let mut outcome = Outcome::default();
    for info in nodes.iter().filter(|info| is_hidpp_interface(info)) {
        let receiver = (info.bus == BUS_USB).then(|| receiver_name(info.product)).flatten();
        let bluetooth = info.bus == BUS_BLUETOOTH;
        if receiver.is_none() && !bluetooth {
            continue;
        }
        let device = match Hidraw::open(&info.node) {
            Ok(device) => device,
            Err(error) => {
                if hidraw::error_code(&error) == "permission-denied" {
                    outcome.permission_denied.push(info.node.display().to_string());
                }
                continue;
            }
        };
        let ids = hidraw::report_ids(&info.descriptor);
        let report = if ids.contains(&SHORT) { SHORT } else { LONG };
        let mut channel = Channel { device, report, sw_id: 0x01, timeout: REQUEST_TIMEOUT };
        if let Some(via) = receiver {
            for slot in 1..=6u8 {
                let key = format!("{:04x}@{}:{slot}", info.product, receiver_path(&info.phys));
                if Instant::now() >= deadline {
                    // Out of time: list known devices without querying them.
                    if let Some(known) = cache.slots.get(&key).filter(|known| !known.id.is_empty()) {
                        let mut device = device_from(known, Link::Receiver, Some(via), None);
                        device.error = Some("not-queried".into());
                        outcome.devices.push(device);
                    }
                    continue;
                }
                if let Some(device) = probe(&mut channel, slot, &key, cache, Link::Receiver, Some(via), None) {
                    outcome.devices.push(device);
                }
            }
        } else {
            let mac = (!info.uniq.is_empty()).then_some(info.uniq.as_str());
            let key = format!("bt:{}", normalize_key(mac.unwrap_or(&info.node.display().to_string())));
            if let Some(device) = probe(&mut channel, DIRECT, &key, cache, Link::Bluetooth, Some("Bluetooth"), mac) {
                outcome.devices.push(device);
            }
        }
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_short_and_long_requests() {
        assert_eq!(build_request(SHORT, 1, 0, 0x1a, &[0, 0, 0x5a]), vec![0x10, 1, 0, 0x1a, 0, 0, 0x5a]);
        let long = build_request(LONG, 0xff, 8, 0x13, &[]);
        assert_eq!(long.len(), 20);
        assert_eq!(&long[..4], &[0x11, 0xff, 8, 0x13]);
    }

    #[test]
    fn matches_success_and_errors() {
        let ok = [0x11, 0x02, 0x00, 0x1a, 0x04, 0x05, 0x5a, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(match_reply(&ok, 2, 0, 0x1a), Some(Reply::Ok(ok[4..].to_vec())));
        // Unrelated device, function or software id.
        assert_eq!(match_reply(&ok, 3, 0, 0x1a), None);
        assert_eq!(match_reply(&ok, 2, 0, 0x1b), None);
        // HID++ 1.0 error from the receiver for an unreachable device.
        let err10 = [0x10, 0x02, 0x8f, 0x00, 0x1a, 0x09, 0x00];
        assert_eq!(match_reply(&err10, 2, 0, 0x1a), Some(Reply::Hidpp10Error(0x09)));
        // HID++ 2.0 error.
        let err20 = [0x11, 0xff, 0xff, 0x08, 0x13, 0x05, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(match_reply(&err20, 0xff, 8, 0x13), Some(Reply::Hidpp20Error(0x05)));
        // Bluetooth device answering a 0xFF request as device 0x00.
        let bt = [0x11, 0x00, 0x08, 0x13, 55, 4, 0, 0];
        assert_eq!(match_reply(&bt, 0xff, 8, 0x13), Some(Reply::Ok(vec![55, 4, 0, 0])));
    }

    #[test]
    fn decodes_unified_battery() {
        let battery = battery_from_unified(&[55, 4, 0, 0]).unwrap();
        assert_eq!(battery.percent, Some(55));
        assert_eq!(battery.level, Some(Level::Good));
        assert_eq!(battery.charging, Charging::Discharging);
        assert!(!battery.estimated);
        let coarse = battery_from_unified(&[0, 2, 1, 1]).unwrap();
        assert_eq!(coarse.percent, None);
        assert_eq!(coarse.level, Some(Level::Low));
        assert_eq!(coarse.charging, Charging::Charging);
        assert_eq!(battery_from_unified(&[100, 8, 3, 1]).unwrap().charging, Charging::Full);
        assert!(battery_from_unified(&[1, 2]).is_none());
    }

    #[test]
    fn decodes_battery_status_and_voltage() {
        let status = battery_from_status(&[30, 20, 0]).unwrap();
        assert_eq!((status.percent, status.charging), (Some(30), Charging::Discharging));
        assert_eq!(battery_from_status(&[0, 0, 1]).unwrap().percent, None);
        let voltage = battery_from_voltage(&[0x0E, 0xE3, 0x00]).unwrap(); // 3811 mV
        assert_eq!(voltage.percent, Some(50));
        assert!(voltage.estimated);
        assert_eq!(battery_from_voltage(&[0x10, 0x68, 0x80]).unwrap().charging, Charging::Charging);
        assert!(battery_from_voltage(&[0, 0, 0]).is_none());
    }

    #[test]
    fn voltage_curve_is_monotonic_and_clamped() {
        assert_eq!(percent_from_voltage(4300), 100);
        assert_eq!(percent_from_voltage(3000), 0);
        let mut previous = 0;
        for mv in (3400..4300).step_by(10) {
            let pct = percent_from_voltage(mv);
            assert!(pct >= previous, "{mv} mV -> {pct}%");
            previous = pct;
        }
    }

    #[test]
    fn detects_hidpp_interfaces() {
        let base = HidrawInfo {
            node: "/dev/hidraw3".into(),
            bus: BUS_USB,
            vendor: LOGITECH,
            product: 0xc548,
            name: "Logitech USB Receiver".into(),
            uniq: String::new(),
            phys: String::new(),
            descriptor: vec![0x06, 0x00, 0xFF, 0x09, 0x01, 0xA1, 0x01, 0x85, 0x10, 0x85, 0x11, 0xC0],
        };
        assert!(is_hidpp_interface(&base));
        let ble = HidrawInfo {
            descriptor: vec![0x06, 0x43, 0xFF, 0x0A, 0x02, 0x02, 0xA1, 0x01, 0x85, 0x11, 0xC0],
            ..base.clone()
        };
        assert!(is_hidpp_interface(&ble));
        let keyboard =
            HidrawInfo { descriptor: vec![0x05, 0x01, 0x09, 0x06, 0xA1, 0x01, 0x85, 0x01, 0xC0], ..base.clone() };
        assert!(!is_hidpp_interface(&keyboard));
        let other_vendor = HidrawInfo { vendor: 0x3434, ..base };
        assert!(!is_hidpp_interface(&other_vendor));
        assert_eq!(receiver_name(0xc548), Some("Logi Bolt"));
        assert_eq!(receiver_name(0x0943), None);
    }

    #[test]
    fn full_charge_without_level_reads_as_100() {
        let status = battery_from_status(&[0, 0, 3]).unwrap();
        assert_eq!((status.percent, status.charging), (Some(100), Charging::Full));
        let unified = battery_from_unified(&[0, 8, 3, 1]).unwrap();
        assert_eq!((unified.percent, unified.charging), (Some(100), Charging::Full));
    }

    #[test]
    fn decodes_voltage_charge_substate_and_critical_flag() {
        assert_eq!(battery_from_voltage(&[0x10, 0x68, 0x81]).unwrap().charging, Charging::Full);
        assert_eq!(battery_from_voltage(&[0x10, 0x68, 0x82]).unwrap().charging, Charging::Unknown);
        let critical = battery_from_voltage(&[0x0E, 0x00, 0x20]).unwrap();
        assert_eq!((critical.level, critical.charging), (Some(Level::Critical), Charging::Discharging));
        // Bit 5 is not "critical" when a charge-speed bit is set.
        assert_eq!(battery_from_voltage(&[0x0E, 0x00, 0xA8]).unwrap().level, None);
    }

    #[test]
    fn receiver_path_drops_interface_suffix() {
        assert_eq!(receiver_path("usb-0000:07:00.3-1.1.2/input2"), "usb-0000:07:00.3-1.1.2");
        assert_eq!(receiver_path(""), "");
    }

    #[test]
    fn extracts_unit_id() {
        assert_eq!(unit_id_from(&[1, 0x4a, 0xb2, 0x1c, 0x9f, 0, 0x0e]).as_deref(), Some("4ab21c9f"));
        assert_eq!(unit_id_from(&[1, 0, 0, 0, 0, 0, 0]), None);
        assert_eq!(unit_id_from(&[1, 2]), None);
    }

    #[test]
    fn maps_device_types() {
        assert_eq!(kind_from_device_type(0), Kind::Keyboard);
        assert_eq!(kind_from_device_type(3), Kind::Mouse);
        assert_eq!(kind_from_device_type(19), Kind::Other);
    }
}
