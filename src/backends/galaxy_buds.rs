//! Samsung Galaxy Buds (Buds+ and later) battery over the vendor SPP channel.
//!
//! Frame: SOM 0xFD, little-endian u16 header (bits 0-9 size, 0x1000 type,
//! 0x2000 fragment), message id, payload, CRC-16/XMODEM of id+payload stored
//! little-endian, EOM 0xDD. `size` counts id + payload + CRC. The earbuds push
//! EXTENDED_STATUS_UPDATED (0x61) after connecting and STATUS_UPDATED (0x60)
//! on every battery/placement change; we only read, never send commands.
//!
//! Protocol facts were cross-checked against the public notes of the
//! GalaxyBudsClient project (GPL-3.0); no code was copied from it.

use std::collections::HashMap;
use std::fs::File;
use std::io::{ErrorKind, Read};
use std::os::fd::{AsRawFd, OwnedFd};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use zbus::blocking::Connection;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

use crate::model::{Battery, Charging, Component, Device, Kind, Link, normalize_key, now};

pub const SPP_UUID: &str = "2e73a4ad-332d-41fc-90e2-16bef06523f2";
const SOM: u8 = 0xFD;
const EOM: u8 = 0xDD;
pub const STATUS_UPDATED: u8 = 0x60;
pub const EXTENDED_STATUS_UPDATED: u8 = 0x61;
const PROFILE_PATH: &str = "/io/github/caiolombello/perigauge/buds";
const MAX_BUFFER: usize = 4096;

pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for byte in data {
        crc ^= (*byte as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}

/// Build a frame; used by tests and to document the format.
pub fn encode(id: u8, payload: &[u8]) -> Vec<u8> {
    let size = (payload.len() + 3) as u16;
    let mut body = vec![id];
    body.extend_from_slice(payload);
    let crc = crc16(&body);
    let mut frame = vec![SOM];
    frame.extend_from_slice(&size.to_le_bytes());
    frame.extend_from_slice(&body);
    frame.extend_from_slice(&crc.to_le_bytes());
    frame.push(EOM);
    frame
}

enum Check {
    Incomplete,
    Invalid,
    Valid(usize),
}

/// Validate a frame starting at `buffer[0]` (which must be SOM).
fn check(buffer: &[u8]) -> Check {
    if buffer.len() < 3 {
        return Check::Incomplete;
    }
    let size = (u16::from_le_bytes([buffer[1], buffer[2]]) & 0x03FF) as usize;
    if size < 3 {
        return Check::Invalid;
    }
    let total = 1 + 2 + size + 1;
    if buffer.len() < total {
        return Check::Incomplete;
    }
    let body = &buffer[3..3 + size - 2];
    let stored = u16::from_le_bytes([buffer[3 + size - 2], buffer[3 + size - 1]]);
    if buffer[total - 1] != EOM || crc16(body) != stored {
        return Check::Invalid;
    }
    Check::Valid(total)
}

/// Incremental decoder: feed raw stream bytes, get complete (id, payload) messages.
#[derive(Default)]
pub struct Decoder {
    buffer: Vec<u8>,
}

impl Decoder {
    pub fn push(&mut self, data: &[u8]) -> Vec<(u8, Vec<u8>)> {
        self.buffer.extend_from_slice(data);
        let mut messages = Vec::new();
        loop {
            match self.buffer.iter().position(|b| *b == SOM) {
                Some(0) => {}
                Some(start) => {
                    self.buffer.drain(..start);
                }
                None => {
                    self.buffer.clear();
                    break;
                }
            }
            match check(&self.buffer) {
                Check::Valid(total) => {
                    let frame: Vec<u8> = self.buffer.drain(..total).collect();
                    let header = u16::from_le_bytes([frame[1], frame[2]]);
                    if header & 0x2000 == 0 {
                        messages.push((frame[3], frame[4..total - 3].to_vec()));
                    }
                }
                Check::Invalid => {
                    self.buffer.drain(..1);
                }
                Check::Incomplete => {
                    // A corrupted header can claim a long frame and stall the
                    // stream; prefer a later frame that is already complete.
                    let later = (1..self.buffer.len())
                        .filter(|&at| self.buffer[at] == SOM)
                        .find(|&at| matches!(check(&self.buffer[at..]), Check::Valid(_)));
                    match later {
                        Some(at) => {
                            self.buffer.drain(..at);
                        }
                        None => {
                            if self.buffer.len() > MAX_BUFFER {
                                self.buffer.clear();
                            }
                            break;
                        }
                    }
                }
            }
        }
        messages
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Status {
    pub left: Option<u8>,
    pub right: Option<u8>,
    pub case: Option<u8>,
    /// `None` until a STATUS_UPDATED frame reports the charging bits.
    pub left_charging: Option<bool>,
    pub right_charging: Option<bool>,
    pub case_charging: Option<bool>,
    /// Placement nibbles: 0 disconnected, 1 wearing, 2 idle, 3 in case, 4 closed case.
    pub left_placement: u8,
    pub right_placement: u8,
}

fn percent(value: u8) -> Option<u8> {
    (1..=100).contains(&value).then_some(value)
}

/// Decode the battery part of 0x60/0x61 messages (Buds+ layout and later).
pub fn parse(id: u8, payload: &[u8], previous: &Status) -> Option<Status> {
    let base = match id {
        STATUS_UPDATED => 1,
        EXTENDED_STATUS_UPDATED => 2,
        _ => return None,
    };
    let fields = payload.get(base..base + 6)?;
    let mut status = previous.clone();
    status.left_placement = fields[4] >> 4;
    status.right_placement = fields[4] & 0x0F;
    status.left = percent(fields[0]).filter(|_| status.left_placement != 0);
    status.right = percent(fields[1]).filter(|_| status.right_placement != 0);
    status.case = percent(fields[5]);
    // The extended status carries charging flags at model-specific offsets;
    // they are only taken from STATUS_UPDATED, which has a fixed layout.
    if id == STATUS_UPDATED {
        if let Some(bits) = payload.get(7) {
            status.left_charging = Some(bits & 0x10 != 0);
            status.right_charging = Some(bits & 0x04 != 0);
            status.case_charging = Some(bits & 0x01 != 0);
        }
    }
    Some(status)
}

fn bud_charging(flag: Option<bool>, placement: u8) -> Charging {
    match flag {
        Some(true) => Charging::Charging,
        Some(false) => Charging::Discharging,
        // Before the first STATUS_UPDATED: a bud in its case is charging.
        None if matches!(placement, 3 | 4) => Charging::Charging,
        None => Charging::Unknown,
    }
}

pub fn device_from(mac: &str, name: &str, status: &Status, updated_at: f64) -> Device {
    let mac_key = normalize_key(mac);
    let mut device = Device::new(format!("bt:{mac_key}"), name, Kind::Earbuds, "galaxy-buds");
    device.vendor = Some("Samsung".into());
    device.link = Link::Bluetooth;
    device.via = Some("Bluetooth".into());
    device.match_keys.push(format!("bt:{mac_key}"));
    let buds: Vec<(&str, u8, Charging)> = [
        ("left", status.left, bud_charging(status.left_charging, status.left_placement)),
        ("right", status.right, bud_charging(status.right_charging, status.right_placement)),
    ]
    .into_iter()
    .filter_map(|(id, level, charging)| level.map(|level| (id, level, charging)))
    .collect();
    for (id, level, charging) in &buds {
        device.components.push(Component { id: (*id).into(), percent: Some(*level), charging: *charging });
    }
    if let Some(case) = status.case {
        let charging = match status.case_charging {
            Some(true) => Charging::Charging,
            Some(false) => Charging::Discharging,
            None => Charging::Unknown,
        };
        device.components.push(Component { id: "case".into(), percent: Some(case), charging });
    }
    if !buds.is_empty() {
        let states: Vec<Charging> = buds.iter().map(|(_, _, charging)| *charging).collect();
        let charging = if states.iter().all(|c| *c == Charging::Charging) {
            Charging::Charging
        } else if states.contains(&Charging::Discharging) {
            Charging::Discharging
        } else {
            Charging::Unknown
        };
        device.battery =
            Battery { percent: buds.iter().map(|(_, level, _)| *level).min(), level: None, charging, estimated: false };
        device.updated_at = Some(updated_at);
    }
    device
}

#[derive(Debug)]
struct Entry {
    name: String,
    status: Status,
    updated_at: f64,
    connected: bool,
    /// Identifies the reader thread that owns the current connection.
    generation: u64,
    /// Duplicate of the socket, used to shut the connection down on request.
    control: Option<OwnedFd>,
}

type Shared = Arc<Mutex<HashMap<String, Entry>>>;

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

struct Profile {
    shared: Shared,
    names: Arc<Mutex<HashMap<String, String>>>,
    generations: AtomicU64,
}

fn mac_from_path(path: &str) -> Option<String> {
    let tail = path.rsplit('/').next()?.strip_prefix("dev_")?;
    (tail.len() == 17).then(|| tail.replace('_', ":"))
}

/// BlueZ hands over a non-blocking socket; the reader wants blocking reads.
fn set_blocking(fd: &OwnedFd) {
    // SAFETY: fcntl on a valid descriptor owned by the caller.
    unsafe {
        let flags = libc::fcntl(fd.as_raw_fd(), libc::F_GETFL);
        if flags >= 0 {
            libc::fcntl(fd.as_raw_fd(), libc::F_SETFL, flags & !libc::O_NONBLOCK);
        }
    }
}

fn shutdown(fd: &OwnedFd) {
    // SAFETY: shutdown on a valid descriptor; it wakes the blocked reader.
    unsafe {
        libc::shutdown(fd.as_raw_fd(), libc::SHUT_RDWR);
    }
}

fn read_stream(fd: OwnedFd, mac: String, generation: u64, shared: Shared) {
    let mut file = File::from(fd);
    let mut decoder = Decoder::default();
    let mut buffer = [0u8; 1024];
    loop {
        let n = match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => n,
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(200));
                continue;
            }
            Err(_) => break,
        };
        for (id, payload) in decoder.push(&buffer[..n]) {
            let mut map = lock(&shared);
            if let Some(entry) = map.get_mut(&mac).filter(|entry| entry.generation == generation) {
                if let Some(status) = parse(id, &payload, &entry.status) {
                    entry.status = status;
                    entry.updated_at = now();
                }
            }
        }
    }
    // A newer connection may already own this entry; leave it alone.
    if let Some(entry) = lock(&shared).get_mut(&mac).filter(|entry| entry.generation == generation) {
        entry.connected = false;
        entry.control = None;
    }
}

#[zbus::interface(name = "org.bluez.Profile1")]
impl Profile {
    fn release(&self) {}

    fn new_connection(
        &self,
        device: OwnedObjectPath,
        fd: zbus::zvariant::OwnedFd,
        _properties: HashMap<String, OwnedValue>,
    ) {
        let Some(mac) = mac_from_path(device.as_str()) else { return };
        let fd = OwnedFd::from(fd);
        set_blocking(&fd);
        let name = lock(&self.names).get(&mac).cloned().unwrap_or_else(|| "Galaxy Buds".into());
        let generation = self.generations.fetch_add(1, Ordering::Relaxed) + 1;
        let entry = Entry {
            name,
            status: Status::default(),
            updated_at: 0.0,
            connected: true,
            generation,
            control: fd.try_clone().ok(),
        };
        if let Some(old) = lock(&self.shared).insert(mac.clone(), entry) {
            if let Some(control) = old.control {
                shutdown(&control);
            }
        }
        let shared = self.shared.clone();
        let _ = thread::Builder::new().name("buds-spp".into()).spawn(move || read_stream(fd, mac, generation, shared));
    }

    fn request_disconnection(&self, device: OwnedObjectPath) {
        let Some(mac) = mac_from_path(device.as_str()) else { return };
        if let Some(entry) = lock(&self.shared).get_mut(&mac) {
            entry.connected = false;
            if let Some(control) = entry.control.take() {
                shutdown(&control);
            }
        }
    }
}

type ManagedObjects = HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>;

/// Connected BlueZ devices that advertise the Samsung SPP service: (path, mac, alias).
fn candidates(connection: &Connection) -> zbus::Result<Vec<(String, String, String)>> {
    let reply = connection.call_method(
        Some("org.bluez"),
        "/",
        Some("org.freedesktop.DBus.ObjectManager"),
        "GetManagedObjects",
        &(),
    )?;
    let objects: ManagedObjects = reply.body().deserialize()?;
    let text = |props: &HashMap<String, OwnedValue>, key: &str| {
        props.get(key).and_then(|v| v.try_clone().ok()).and_then(|v| String::try_from(v).ok())
    };
    Ok(objects
        .into_iter()
        .filter_map(|(path, interfaces)| {
            let props = interfaces.get("org.bluez.Device1")?;
            let connected =
                props.get("Connected").and_then(|v| bool::try_from(v.try_clone().ok()?).ok()).unwrap_or(false);
            let uuids: Vec<String> = props
                .get("UUIDs")
                .and_then(|v| v.try_clone().ok())
                .and_then(|v| Vec::<String>::try_from(v).ok())
                .unwrap_or_default();
            if !connected || !uuids.iter().any(|uuid| uuid.eq_ignore_ascii_case(SPP_UUID)) {
                return None;
            }
            let mac = text(props, "Address")?;
            let alias = text(props, "Alias").unwrap_or_else(|| "Galaxy Buds".into());
            Some((path.to_string(), mac, alias))
        })
        .collect())
}

fn bluez_owner(connection: &Connection) -> Option<String> {
    connection
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "GetNameOwner",
            &("org.bluez",),
        )
        .and_then(|reply| reply.body().deserialize::<String>())
        .ok()
}

fn register(connection: &Connection) -> zbus::Result<()> {
    let mut options: HashMap<&str, Value> = HashMap::new();
    options.insert("Name", Value::from("PeriGauge"));
    options.insert("Role", Value::from("client"));
    options.insert("AutoConnect", Value::from(false));
    let path = zbus::zvariant::ObjectPath::try_from(PROFILE_PATH)?;
    let result = connection.call_method(
        Some("org.bluez"),
        "/org/bluez",
        Some("org.bluez.ProfileManager1"),
        "RegisterProfile",
        &(path, SPP_UUID, options),
    );
    match result {
        Err(zbus::Error::MethodError(name, _, _)) if name.as_str() == "org.bluez.Error.AlreadyExists" => Ok(()),
        other => other.map(|_| ()),
    }
}

/// Log each distinct failure once instead of on every retry.
fn report(last_errors: &mut HashMap<String, String>, key: &str, text: String) {
    if last_errors.get(key) != Some(&text) {
        eprintln!("perigauge: Galaxy Buds {key}: {text}");
        last_errors.insert(key.to_string(), text);
    }
}

/// Background monitor: registers a BlueZ client profile and asks BlueZ to open
/// the SPP channel of connected Galaxy Buds. Readings arrive as pushed frames.
/// Registration is repeated whenever bluetoothd (re)appears on the bus.
pub struct Monitor {
    shared: Shared,
}

impl Monitor {
    pub fn start() -> Result<Monitor, String> {
        let shared: Shared = Arc::default();
        let names: Arc<Mutex<HashMap<String, String>>> = Arc::default();
        let profile = Profile { shared: shared.clone(), names: names.clone(), generations: AtomicU64::new(0) };
        let connection = zbus::blocking::connection::Builder::system()
            .and_then(|builder| builder.serve_at(PROFILE_PATH, profile))
            .map(|builder| builder.method_timeout(Duration::from_secs(30)))
            .and_then(|builder| builder.build())
            .map_err(|e| e.to_string())?;
        let worker_shared = shared.clone();
        thread::Builder::new()
            .name("buds-connect".into())
            .spawn(move || {
                let mut last_errors: HashMap<String, String> = HashMap::new();

                let mut registered_with: Option<String> = None;
                loop {
                    let owner = bluez_owner(&connection);
                    if owner.is_none() {
                        registered_with = None;
                    } else if owner != registered_with {
                        match register(&connection) {
                            Ok(()) => registered_with = owner,
                            Err(error) => report(&mut last_errors, "RegisterProfile", error.to_string()),
                        }
                    }
                    if registered_with.is_some() {
                        if let Ok(found) = candidates(&connection) {
                            for (path, mac, alias) in found {
                                lock(&names).insert(mac.clone(), alias);
                                if lock(&worker_shared).get(&mac).is_some_and(|e| e.connected) {
                                    continue;
                                }
                                let result = connection.call_method(
                                    Some("org.bluez"),
                                    path.as_str(),
                                    Some("org.bluez.Device1"),
                                    "ConnectProfile",
                                    &(SPP_UUID,),
                                );
                                match result {
                                    Ok(_) => {
                                        last_errors.remove(&mac);
                                    }
                                    Err(error) => report(&mut last_errors, &mac, error.to_string()),
                                }
                            }
                        }
                    }
                    thread::sleep(Duration::from_secs(20));
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Monitor { shared })
    }

    pub fn devices(&self) -> Vec<Device> {
        lock(&self.shared)
            .iter()
            .filter(|(_, entry)| entry.connected)
            .map(|(mac, entry)| {
                let mut device = device_from(mac, &entry.name, &entry.status, entry.updated_at);
                if device.updated_at.is_none() {
                    device.error = Some("waiting-for-status".into());
                }
                device
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_matches_xmodem_check_value() {
        assert_eq!(crc16(b"123456789"), 0x31C3);
    }

    fn extended(left: u8, right: u8, placement: u8, case: u8) -> Vec<u8> {
        // revision, ear type, L, R, coupled, main connection, placement, case, extra settings...
        vec![13, 4, left, right, 1, 0, placement, case, 0, 3, 0, 1]
    }

    #[test]
    fn decodes_extended_status_frame() {
        let mut decoder = Decoder::default();
        let messages = decoder.push(&encode(EXTENDED_STATUS_UPDATED, &extended(80, 74, 0x11, 55)));
        assert_eq!(messages.len(), 1);
        let (id, payload) = &messages[0];
        let status = parse(*id, payload, &Status::default()).unwrap();
        assert_eq!((status.left, status.right, status.case), (Some(80), Some(74), Some(55)));
        assert_eq!((status.left_placement, status.right_placement), (1, 1));
    }

    #[test]
    fn status_update_carries_charging_bits() {
        // revision, L, R, coupled, main, placement (both in case), case, charging bits (L, R, case)
        let payload = [1, 90, 92, 1, 0, 0x33, 60, 0x15];
        let status = parse(STATUS_UPDATED, &payload, &Status::default()).unwrap();
        assert_eq!(
            (status.left_charging, status.right_charging, status.case_charging),
            (Some(true), Some(true), Some(true))
        );
        let device = device_from("AA:BB:CC:00:11:22", "Galaxy Buds3 Pro", &status, 10.0);
        assert_eq!(device.id, "bt:aabbcc001122");
        assert_eq!(device.battery.percent, Some(90));
        assert_eq!(device.battery.charging, Charging::Charging);
        let ids: Vec<_> = device.components.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["left", "right", "case"]);
    }

    #[test]
    fn disconnected_bud_is_not_reported() {
        let payload = [1, 50, 0, 0, 0, 0x10, 0, 0];
        let status = parse(STATUS_UPDATED, &payload, &Status::default()).unwrap();
        assert_eq!((status.left, status.right, status.case), (Some(50), None, None));
        let device = device_from("aa:bb:cc:dd:ee:ff", "Buds", &status, 1.0);
        assert_eq!(device.battery.percent, Some(50));
        assert_eq!(device.battery.charging, Charging::Discharging);
        assert_eq!(device.components.len(), 1);
    }

    #[test]
    fn decoder_handles_split_garbage_and_bad_crc() {
        let good = encode(STATUS_UPDATED, &[1, 40, 41, 1, 0, 0x22, 70, 0]);
        let mut corrupt = good.clone();
        corrupt[5] ^= 0xFF;
        let mut stream = vec![0x00, 0x13, 0x37];
        stream.extend_from_slice(&corrupt);
        stream.extend_from_slice(&good);
        let mut decoder = Decoder::default();
        let (first, second) = stream.split_at(9);
        let mut messages = decoder.push(first);
        messages.extend(decoder.push(second));
        assert_eq!(messages, vec![(STATUS_UPDATED, vec![1, 40, 41, 1, 0, 0x22, 70, 0])]);
    }

    #[test]
    fn false_som_claiming_a_long_frame_does_not_hide_a_complete_one() {
        let good = encode(STATUS_UPDATED, &[1, 40, 41, 1, 0, 0x22, 70, 0]);
        let mut stream = vec![SOM, 0xFF, 0x03];
        stream.extend_from_slice(&good);
        let mut decoder = Decoder::default();
        assert_eq!(decoder.push(&stream), vec![(STATUS_UPDATED, vec![1, 40, 41, 1, 0, 0x22, 70, 0])]);
    }

    #[test]
    fn skips_fragments_and_keeps_following_frames() {
        let mut fragment = encode(0x10, &[1, 2, 3]);
        fragment[2] |= 0x20; // fragment flag lives in the header's high byte
        let crc_input = [&fragment[3..4], &fragment[4..7]].concat();
        let crc = crc16(&crc_input).to_le_bytes();
        fragment[7] = crc[0];
        fragment[8] = crc[1];
        let mut stream = fragment;
        stream.extend_from_slice(&encode(STATUS_UPDATED, &[1, 60, 61, 1, 0, 0x11, 50, 0]));
        let messages = Decoder::default().push(&stream);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].0, STATUS_UPDATED);
    }

    #[test]
    fn extended_status_without_charging_bits_is_not_reported_as_discharging() {
        // Both buds in the case (placement 3), no STATUS_UPDATED yet.
        let in_case = parse(EXTENDED_STATUS_UPDATED, &extended(15, 12, 0x33, 60), &Status::default()).unwrap();
        let device = device_from("aa:bb:cc:dd:ee:ff", "Buds", &in_case, 1.0);
        assert_eq!(device.battery.charging, Charging::Charging);
        assert_eq!(device.components[2].charging, Charging::Unknown);
        // Worn buds with unknown charging state stay unknown, not discharging.
        let worn = parse(EXTENDED_STATUS_UPDATED, &extended(15, 12, 0x11, 60), &Status::default()).unwrap();
        assert_eq!(device_from("aa:bb:cc:dd:ee:ff", "Buds", &worn, 1.0).battery.charging, Charging::Unknown);
    }

    #[test]
    fn ignores_other_messages_and_short_payloads() {
        assert!(parse(0x77, &[1, 2, 3, 4, 5, 6, 7, 8], &Status::default()).is_none());
        assert!(parse(STATUS_UPDATED, &[1, 2, 3], &Status::default()).is_none());
    }

    #[test]
    fn maps_bluez_paths_to_macs() {
        assert_eq!(mac_from_path("/org/bluez/hci0/dev_AA_BB_CC_00_11_22").as_deref(), Some("AA:BB:CC:00:11:22"));
        assert_eq!(mac_from_path("/org/bluez/hci0"), None);
    }
}
