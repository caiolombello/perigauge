//! Producer-owned, consent-gated passive export. No collection or bus work.

use std::ffi::CString;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Component as PathComponent, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::config::{self, Config};
use crate::model::{Charging, Kind, Level, Link, Snapshot};

const FORMAT: &str = "0.1-experimental";
const EXPORT_FILE: &str = "orla-peripherals-v1.json";
const CONSENT_FILE: &str = "orla.json";
const MAX_BYTES: usize = 65_536;
const JSON_SAFE: u64 = 9_007_199_254_740_991;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Busy,
    UnsafePath,
    Io,
    InvalidData,
}

impl Error {
    /// Finite diagnostics: never include paths, device data or parser errors.
    pub fn code(self) -> &'static str {
        match self {
            Error::Busy => "export-busy",
            Error::UnsafePath => "export-unsafe-path",
            Error::Io => "export-io",
            Error::InvalidData => "export-invalid-data",
        }
    }
}

impl From<io::Error> for Error {
    fn from(_: io::Error) -> Self {
        Error::Io
    }
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Envelope {
    format_version: &'static str,
    producer: &'static str,
    producer_version: &'static str,
    generated_at_ms: u64,
    expires_at_ms: u64,
    interval_seconds: u64,
    devices: Vec<ExportDevice>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportDevice {
    kind: Kind,
    link: Link,
    present: bool,
    stale: bool,
    observed_at_ms: Option<u64>,
    battery: ExportBattery,
    components: Vec<ExportComponent>,
}

#[derive(Serialize)]
struct ExportBattery {
    percent: Option<u8>,
    level: Option<Level>,
    charging: Charging,
    estimated: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum Label {
    Left,
    Right,
    Case,
}

#[derive(Serialize)]
struct ExportComponent {
    label: Label,
    percent: Option<u8>,
    charging: Charging,
}

fn milliseconds(seconds: f64) -> Option<u64> {
    let value = (seconds * 1000.0).floor();
    (seconds.is_finite() && value >= 1.0 && value <= JSON_SAFE as f64).then_some(value as u64)
}

/// Pure projection of an already collected snapshot, never a new observation.
/// `current` is parsed afresh without Config::load's error-to-default fallback.
pub fn project(snapshot: &Snapshot, current: &Config, interval: u64) -> Result<Vec<u8>> {
    project_at(snapshot, current, interval, snapshot.generated_at)
}

fn project_at(snapshot: &Snapshot, current: &Config, interval: u64, projection_time: f64) -> Result<Vec<u8>> {
    if !(10..=3600).contains(&interval) {
        return Err(Error::InvalidData);
    }
    let generated = milliseconds(snapshot.generated_at).ok_or(Error::InvalidData)?;
    let lease = 3000 * interval;
    let expires = generated.checked_add(lease).filter(|at| *at <= JSON_SAFE).ok_or(Error::InvalidData)?;
    let projected_at = milliseconds(projection_time).ok_or(Error::InvalidData)?;
    if projected_at < generated || projected_at >= expires {
        return Err(Error::InvalidData);
    }
    let mut devices = Vec::new();
    for device in &snapshot.devices {
        if device.hidden || current.device(&device.id).hidden {
            continue;
        }
        if devices.len() == 128 {
            return Err(Error::InvalidData);
        }
        let observed = device.updated_at.filter(|at| *at <= snapshot.generated_at).and_then(milliseconds);
        let stale = device.stale
            || !device.present
            || device.error.is_some()
            || observed.is_none_or(|at| projected_at - at >= lease);
        let mut components = Vec::new();
        // Canonical, unique labels; unknown source strings never cross the boundary.
        for (id, label) in [("left", Label::Left), ("right", Label::Right), ("case", Label::Case)] {
            let mut matching = device.components.iter().filter(|component| component.id == id);
            if let Some(component) = matching.next() {
                if matching.next().is_some() {
                    return Err(Error::InvalidData);
                }
                components.push(ExportComponent {
                    label,
                    percent: component.percent.filter(|percent| *percent <= 100),
                    charging: component.charging,
                });
            }
        }
        // The model already owns earbud aggregation. In particular, a case-only
        // reading must never acquire a device observation time here.
        devices.push(ExportDevice {
            kind: device.kind,
            link: device.link,
            present: device.present,
            stale,
            observed_at_ms: observed,
            battery: ExportBattery {
                percent: device.battery.percent.filter(|percent| *percent <= 100),
                level: device.battery.level,
                charging: device.battery.charging,
                estimated: device.battery.estimated,
            },
            components,
        });
    }
    let bytes = serde_json::to_vec(&Envelope {
        format_version: FORMAT,
        producer: "perigauge",
        producer_version: env!("CARGO_PKG_VERSION"),
        generated_at_ms: generated,
        expires_at_ms: expires,
        interval_seconds: interval,
        devices,
    })
    .map_err(|_| Error::InvalidData)?;
    if bytes.len() > MAX_BYTES {
        return Err(Error::InvalidData);
    }
    Ok(bytes)
}

/// Explicit paths keep tests independent of process-global environment variables.
#[derive(Clone)]
pub struct Paths {
    pub config: PathBuf,
    pub state: PathBuf,
}

impl Paths {
    pub fn system() -> Self {
        Self { config: config::config_dir(), state: config::state_dir() }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Consent {
    format_version: String,
    enabled: bool,
}

#[derive(Clone, Copy)]
pub enum Action {
    Enable,
    Disable,
    Status,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub enabled: bool,
    pub export_present: bool,
    pub export_bytes: Option<u64>,
}

// The new paths use directory FDs all the way down, including temporary files,
// rename and removal. The existing cache/config writer is deliberately untouched.
struct Dir(File);

fn cstr(value: &std::ffi::OsStr) -> Result<CString> {
    CString::new(value.as_bytes()).map_err(|_| Error::UnsafePath)
}

fn owned_private(file: &File, directory: bool, private: bool) -> Result<()> {
    let metadata = file.metadata()?;
    // SAFETY: geteuid has no pointer arguments or side effects.
    let uid = unsafe { libc::geteuid() };
    if metadata.uid() != uid
        || (directory && !metadata.is_dir())
        || (!directory && (!metadata.is_file() || metadata.nlink() != 1))
        || (private && metadata.mode() & 0o777 != if directory { 0o700 } else { 0o600 })
    {
        return Err(Error::UnsafePath);
    }
    Ok(())
}

impl Dir {
    fn open(path: &Path, create: bool) -> Result<Option<Self>> {
        if !path.is_absolute() {
            return Err(Error::UnsafePath);
        }
        let mut dir = Dir(File::open("/")?);
        for component in path.components() {
            match component {
                PathComponent::RootDir => (),
                PathComponent::Normal(name) => {
                    let Some(next) = dir.child(name, create)? else { return Ok(None) };
                    dir = next;
                }
                _ => return Err(Error::UnsafePath),
            }
        }
        owned_private(&dir.0, true, true)?;
        Ok(Some(dir))
    }

    fn child(&self, name: &std::ffi::OsStr, create: bool) -> Result<Option<Self>> {
        let name = cstr(name)?;
        let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
        // SAFETY: live directory FD and a NUL-terminated single component.
        let mut fd = unsafe { libc::openat(self.0.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 && io::Error::last_os_error().kind() == io::ErrorKind::NotFound {
            if !create {
                return Ok(None);
            }
            // SAFETY: anchored mkdir of a single path component, no symlink walk.
            if unsafe { libc::mkdirat(self.0.as_raw_fd(), name.as_ptr(), 0o700) } != 0
                && io::Error::last_os_error().kind() != io::ErrorKind::AlreadyExists
            {
                return Err(Error::Io);
            }
            self.0.sync_all()?;
            // SAFETY: same anchored directory open as above.
            fd = unsafe { libc::openat(self.0.as_raw_fd(), name.as_ptr(), flags) };
        }
        if fd < 0 {
            return Err(Error::UnsafePath);
        }
        // SAFETY: openat returned a new owned descriptor.
        Ok(Some(Dir(unsafe { File::from_raw_fd(fd) })))
    }

    fn exports(&self, create: bool) -> Result<Option<Self>> {
        let dir = self.child(std::ffi::OsStr::new("exports"), create)?;
        if let Some(dir) = &dir {
            owned_private(&dir.0, true, true)?;
        }
        Ok(dir)
    }

    fn file(&self, name: &str, flags: i32) -> Result<Option<File>> {
        let name = cstr(std::ffi::OsStr::new(name))?;
        // O_PATH inspects type/ownership without opening a device or FIFO for IO.
        // SAFETY: valid directory FD and NUL-terminated single component.
        let probe_fd = unsafe {
            libc::openat(self.0.as_raw_fd(), name.as_ptr(), libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        };
        let probe = if probe_fd >= 0 {
            // SAFETY: openat returned a new owned descriptor.
            let file = unsafe { File::from_raw_fd(probe_fd) };
            owned_private(&file, false, false)?;
            Some(file)
        } else if io::Error::last_os_error().kind() == io::ErrorKind::NotFound {
            None
        } else {
            return Err(Error::UnsafePath);
        };
        let flags = if probe.is_some() {
            if flags & libc::O_EXCL != 0 {
                return Err(Error::Io);
            }
            flags & !libc::O_CREAT
        } else {
            if flags & libc::O_CREAT == 0 {
                return Ok(None);
            }
            flags | libc::O_EXCL
        };
        // A swapped special file cannot hang the open; identity is then checked
        // against the held O_PATH descriptor. No same-user sandbox is claimed.
        // SAFETY: valid FD/C string; the mode is used only for O_CREAT.
        let fd = unsafe {
            libc::openat(
                self.0.as_raw_fd(),
                name.as_ptr(),
                flags | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                0o600,
            )
        };
        if fd < 0 {
            return Err(Error::UnsafePath);
        }
        // SAFETY: openat returned a new owned descriptor.
        let file = unsafe { File::from_raw_fd(fd) };
        owned_private(&file, false, false)?;
        if let Some(probe) = probe {
            let before = probe.metadata()?;
            let after = file.metadata()?;
            if (before.dev(), before.ino()) != (after.dev(), after.ino()) {
                return Err(Error::UnsafePath);
            }
        }
        Ok(Some(file))
    }

    fn read(&self, name: &str, limit: usize, private: bool) -> Result<Option<Vec<u8>>> {
        let Some(file) = self.file(name, libc::O_RDONLY)? else { return Ok(None) };
        owned_private(&file, false, private)?;
        let metadata = file.metadata()?;
        if metadata.mode() & 0o400 == 0 {
            return Err(Error::UnsafePath);
        }
        if metadata.len() > limit as u64 {
            return Err(Error::InvalidData);
        }
        let mut bytes = Vec::new();
        file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > limit {
            return Err(Error::InvalidData);
        }
        Ok(Some(bytes))
    }

    fn remove(&self, name: &str) -> Result<()> {
        // Unlink the name itself: never open a special file or follow a link.
        let name = cstr(std::ffi::OsStr::new(name))?;
        // SAFETY: unlinkat removes one entry in this owned directory.
        if unsafe { libc::unlinkat(self.0.as_raw_fd(), name.as_ptr(), 0) } != 0
            && io::Error::last_os_error().kind() != io::ErrorKind::NotFound
        {
            return Err(Error::Io);
        }
        self.0.sync_all()?;
        Ok(())
    }

    fn write(&self, name: &str, bytes: &[u8]) -> Result<()> {
        // Refuse unsafe existing destinations, even though rename would replace
        // the entry itself. Fresh exclusive temp names never truncate a link.
        if let Some(file) = self.file(name, libc::O_RDONLY)? {
            owned_private(&file, false, true)?;
        }
        let mut selected = None;
        for _ in 0..16 {
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let temporary = format!(".export-{}-{sequence}.tmp", std::process::id());
            match self.file(&temporary, libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL) {
                Ok(Some(file)) => {
                    selected = Some((temporary, file));
                    break;
                }
                // Collisions are bounded and cannot overwrite an existing file.
                Err(_) => continue,
                _ => return Err(Error::Io),
            }
        }
        let (temporary, mut file) = selected.ok_or(Error::Io)?;
        let mut renamed = false;
        let result = (|| {
            file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
            file.write_all(bytes)?;
            file.sync_all()?;
            let from = cstr(std::ffi::OsStr::new(&temporary))?;
            let to = cstr(std::ffi::OsStr::new(name))?;
            // SAFETY: both names are single components relative to the same FD.
            if unsafe { libc::renameat(self.0.as_raw_fd(), from.as_ptr(), self.0.as_raw_fd(), to.as_ptr()) } != 0 {
                return Err(Error::Io);
            }
            renamed = true;
            self.0.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = self.remove(if renamed { name } else { &temporary });
        }
        result
    }
}

fn enabled(paths: &Paths) -> bool {
    let read = || -> Result<bool> {
        let Some(config) = Dir::open(&paths.config, false)? else { return Ok(false) };
        let Some(exports) = config.exports(false)? else { return Ok(false) };
        let Some(bytes) = exports.read(CONSENT_FILE, 256, true)? else { return Ok(false) };
        // Serde structs also accept positional arrays; consent must be a JSON object.
        if bytes.iter().find(|byte| !byte.is_ascii_whitespace()) != Some(&b'{') {
            return Err(Error::InvalidData);
        }
        let consent: Consent = serde_json::from_slice(&bytes).map_err(|_| Error::InvalidData)?;
        Ok(consent.format_version == FORMAT && consent.enabled)
    };
    read().unwrap_or(false)
}

fn remove_export(paths: &Paths) -> Result<()> {
    if let Some(state) = Dir::open(&paths.state, false)? {
        if let Some(exports) = state.exports(false)? {
            exports.remove(EXPORT_FILE)?;
        }
    }
    Ok(())
}

fn current_config(paths: &Paths) -> Result<Config> {
    let Some(dir) = Dir::open(&paths.config, false)? else { return Ok(Config::default()) };
    let Some(bytes) = dir.read("config.toml", MAX_BYTES, false)? else { return Ok(Config::default()) };
    let text = std::str::from_utf8(&bytes).map_err(|_| Error::InvalidData)?;
    Config::parse(text).map(|(config, _)| config).map_err(|_| Error::InvalidData)
}

fn try_lock(paths: &Paths) -> Result<File> {
    let dir = Dir::open(&paths.state, true)?.ok_or(Error::Io)?;
    let file = dir.file("collect.lock", libc::O_WRONLY | libc::O_CREAT)?.ok_or(Error::Io)?;
    owned_private(&file, false, true)?;
    // Same inode and advisory lock as normal collection; never change its
    // blocking behavior. CLI contention refuses immediately, without retries.
    // SAFETY: flock on an owned, checked regular-file descriptor.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(if io::Error::last_os_error().kind() == io::ErrorKind::WouldBlock {
            Error::Busy
        } else {
            Error::Io
        });
    }
    Ok(file)
}

fn status(paths: &Paths) -> Result<Status> {
    let mut status = Status { enabled: enabled(paths), export_present: false, export_bytes: None };
    if let Some(state) = Dir::open(&paths.state, false)? {
        if let Some(exports) = state.exports(false)? {
            if let Some(file) = exports.file(EXPORT_FILE, libc::O_RDONLY)? {
                owned_private(&file, false, true)?;
                let length = file.metadata()?.len();
                if length > MAX_BYTES as u64 {
                    return Err(Error::InvalidData);
                }
                status.export_present = true;
                status.export_bytes = Some(length);
            }
        }
    }
    Ok(status)
}

/// Passive CLI API. Does not read config.toml or state.json, instantiate a
/// Collector, refresh a cache, or connect to devices/buses/network.
pub fn command(paths: &Paths, action: Action) -> Result<Status> {
    if matches!(action, Action::Status) {
        return status(paths);
    }
    let _guard = try_lock(paths)?;
    if matches!(action, Action::Enable) {
        // Consent never makes an older cached/exported observation look new.
        remove_export(paths)?;
    } else if !enabled(paths) {
        // Absent/invalid permission is already off. Do not create consent/export
        // directories merely to acknowledge removal while default-off.
        remove_export(paths)?;
        return status(paths);
    }
    let config = Dir::open(&paths.config, true)?.ok_or(Error::Io)?;
    let exports = config.exports(true)?.ok_or(Error::Io)?;
    let consent = Consent { format_version: FORMAT.into(), enabled: matches!(action, Action::Enable) };
    let bytes = serde_json::to_vec(&consent).map_err(|_| Error::InvalidData)?;
    exports.write(CONSENT_FILE, &bytes)?;
    if !consent.enabled {
        remove_export(paths)?;
    }
    let result = status(paths)?;
    if result.enabled != consent.enabled || (!consent.enabled && result.export_present) {
        return Err(Error::Io);
    }
    Ok(result)
}

fn observe_disabled(paths: &Paths) -> Result<()> {
    if !enabled(paths) {
        remove_export(paths)?;
    }
    Ok(())
}

/// Call only while holding collect.lock and after a genuine pass + successful
/// save_snapshot. All failures leave the collection/notifications usable.
fn publish_after_save(paths: &Paths, snapshot: &Snapshot, interval: u64, publication_time: f64) -> Result<()> {
    if !enabled(paths) {
        return remove_export(paths);
    }
    let result = (|| {
        let config = current_config(paths)?;
        let bytes = project_at(snapshot, &config, interval, publication_time)?;
        // Recheck after projection, immediately before creating any export dir.
        if !enabled(paths) {
            return remove_export(paths);
        }
        let state = Dir::open(&paths.state, true)?.ok_or(Error::Io)?;
        let exports = state.exports(true)?.ok_or(Error::Io)?;
        exports.write(EXPORT_FILE, &bytes)
    })();
    if result.is_err() {
        let _ = remove_export(paths);
    }
    result
}

pub(crate) fn collection_start(_guard: &crate::collect::Lock) {
    if let Err(error) = observe_disabled(&Paths::system()) {
        eprintln!("perigauge: {}", error.code());
    }
}

pub(crate) fn collection_saved(_guard: &crate::collect::Lock, snapshot: &Snapshot, interval: u64) {
    if let Err(error) = publish_after_save(&Paths::system(), snapshot, interval, crate::model::now()) {
        eprintln!("perigauge: {}", error.code());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Battery, Device, Kind, SCHEMA_VERSION, Snapshot};

    fn sample() -> Snapshot {
        let mut device = Device::new("PRIVATE-ID", "PRIVATE-NAME", Kind::Mouse, "PRIVATE-BACKEND");
        device.battery = Battery { percent: Some(0), ..Battery::default() };
        device.updated_at = Some(1000.0);
        Snapshot {
            schema: SCHEMA_VERSION,
            version: "PRIVATE-VERSION".into(),
            generated_at: 1000.125,
            source: "PRIVATE-SOURCE".into(),
            devices: vec![device],
            issues: vec![],
        }
    }

    #[test]
    fn export_preserves_zero_original_time_and_only_public_fields() {
        let bytes = project(&sample(), &Config::default(), 30).unwrap();
        assert_eq!(bytes.as_slice(), &include_bytes!("../contrib/orla/fixtures/zero.json")[..]);
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(!text.contains("PRIVATE"));
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["formatVersion"], "0.1-experimental");
        assert_eq!(json["producer"], "perigauge");
        assert_eq!(json["generatedAtMs"], 1_000_125);
        assert_eq!(json["expiresAtMs"], 1_090_125);
        assert_eq!(json["devices"][0]["battery"]["percent"], 0);
        assert_eq!(json["devices"][0]["observedAtMs"], 1_000_000);
        assert_eq!(json["devices"][0]["stale"], false);
        let mut keys: Vec<_> = json["devices"][0].as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort();
        assert_eq!(keys, ["battery", "components", "kind", "link", "observedAtMs", "present", "stale"]);
    }
}

#[cfg(test)]
mod export_tests {
    use super::*;
    use crate::config::DeviceConfig;
    use crate::model::{Battery, Component, Device, Issue, SCHEMA_VERSION};
    use std::fs;
    use std::os::unix::fs::symlink;

    fn sample() -> Snapshot {
        let mut device = Device::new("PRIVATE-ID", "PRIVATE-NAME", Kind::Mouse, "PRIVATE-BACKEND");
        device.updated_at = Some(1000.0);
        device.battery.percent = Some(0);
        Snapshot {
            schema: SCHEMA_VERSION,
            version: "PRIVATE-VERSION".into(),
            generated_at: 1000.125,
            source: "PRIVATE-SOURCE".into(),
            devices: vec![device],
            issues: vec![Issue { code: "PRIVATE-ISSUE".into(), detail: "PRIVATE-DETAIL".into() }],
        }
    }

    fn value(snapshot: &Snapshot, config: &Config) -> serde_json::Value {
        serde_json::from_slice(&project(snapshot, config, 30).unwrap()).unwrap()
    }

    #[test]
    fn export_omits_hidden_from_source_or_current_config_and_all_identity_fields() {
        let mut snapshot = sample();
        snapshot.devices[0].vendor = Some("PRIVATE-VENDOR".into());
        snapshot.devices[0].via = Some("PRIVATE-VIA".into());
        snapshot.devices[0].error = Some("PRIVATE-ERROR".into());
        snapshot.devices[0].match_keys.push("PRIVATE-KEY".into());
        let mut config = Config::default();
        config.devices.insert(
            "PRIVATE-ID".into(),
            DeviceConfig { name: Some("PRIVATE-ALIAS".into()), ..DeviceConfig::default() },
        );
        let bytes = project(&snapshot, &config, 30).unwrap();
        assert!(!String::from_utf8(bytes).unwrap().contains("PRIVATE"));
        config.devices.get_mut("PRIVATE-ID").unwrap().hidden = true;
        assert_eq!(value(&snapshot, &config)["devices"], serde_json::json!([]));
        config.devices.get_mut("PRIVATE-ID").unwrap().hidden = false;
        snapshot.devices[0].hidden = true;
        assert_eq!(value(&snapshot, &config)["devices"], serde_json::json!([]));
    }

    #[test]
    fn export_invalid_future_missing_and_expired_observations_are_stale() {
        for observation in [None, Some(f64::NAN), Some(f64::INFINITY), Some(-1.0), Some(0.0), Some(1000.126)] {
            let mut snapshot = sample();
            snapshot.devices[0].updated_at = observation;
            let json = value(&snapshot, &Config::default());
            assert!(json["devices"][0]["observedAtMs"].is_null());
            assert_eq!(json["devices"][0]["stale"], true);
        }
        for observation in [910.125, 910.124, 1.0] {
            let mut snapshot = sample();
            snapshot.devices[0].updated_at = Some(observation);
            assert_eq!(value(&snapshot, &Config::default())["devices"][0]["stale"], true);
        }
        let mut snapshot = sample();
        snapshot.devices[0].updated_at = Some(910.126);
        assert_eq!(value(&snapshot, &Config::default())["devices"][0]["stale"], false);
        for reason in 0..3 {
            let mut snapshot = sample();
            match reason {
                0 => {
                    snapshot.devices[0].present = false;
                    assert_eq!(
                        project(&snapshot, &Config::default(), 30).unwrap().as_slice(),
                        &include_bytes!("../contrib/orla/fixtures/absent.json")[..]
                    );
                }
                1 => snapshot.devices[0].stale = true,
                _ => snapshot.devices[0].error = Some("PRIVATE-ERROR".into()),
            }
            assert_eq!(value(&snapshot, &Config::default())["devices"][0]["stale"], true);
        }
    }

    #[test]
    fn export_keeps_coarse_unknown_estimated_and_case_only_semantics() {
        let mut snapshot = sample();
        snapshot.devices[0].battery =
            Battery { percent: None, level: Some(Level::Low), charging: Charging::Unknown, estimated: true };
        assert_eq!(
            project(&snapshot, &Config::default(), 30).unwrap().as_slice(),
            &include_bytes!("../contrib/orla/fixtures/coarse.json")[..]
        );
        let battery = value(&snapshot, &Config::default())["devices"][0]["battery"].clone();
        assert_eq!(battery, serde_json::json!({"percent":null,"level":"low","charging":"unknown","estimated":true}));
        snapshot.devices[0].kind = Kind::Earbuds;
        snapshot.devices[0].updated_at = None;
        snapshot.devices[0].battery = Battery::default();
        snapshot.devices[0].components =
            vec![Component { id: "case".into(), percent: Some(80), charging: Charging::Charging }];
        assert_eq!(
            project(&snapshot, &Config::default(), 30).unwrap().as_slice(),
            &include_bytes!("../contrib/orla/fixtures/case-only.json")[..]
        );
        let json = value(&snapshot, &Config::default());
        assert_eq!(
            json["devices"][0]["components"],
            serde_json::json!([{"label":"case","percent":80,"charging":"charging"}])
        );
        assert!(json["devices"][0]["battery"]["percent"].is_null());
        assert!(json["devices"][0]["observedAtMs"].is_null());
        assert_eq!(json["devices"][0]["stale"], true);
    }

    #[test]
    fn export_components_are_bounded_canonical_and_not_averaged_with_case() {
        let mut snapshot = sample();
        let device = &mut snapshot.devices[0];
        device.kind = Kind::Earbuds;
        device.battery.percent = Some(20);
        device.components = vec![
            Component { id: "case".into(), percent: Some(100), charging: Charging::Charging },
            Component { id: "PRIVATE-COMPONENT".into(), percent: Some(70), charging: Charging::Unknown },
            Component { id: "right".into(), percent: Some(40), charging: Charging::Discharging },
            Component { id: "left".into(), percent: Some(20), charging: Charging::Discharging },
        ];
        let json = value(&snapshot, &Config::default());
        assert_eq!(json["devices"][0]["battery"]["percent"], 20);
        let parts = json["devices"][0]["components"].as_array().unwrap();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0]["label"], "left");
        assert_eq!(parts[1]["label"], "right");
        assert_eq!(parts[2]["label"], "case");
        snapshot.devices[0].components.push(Component {
            id: "left".into(),
            percent: Some(99),
            charging: Charging::Charging,
        });
        assert_eq!(project(&snapshot, &Config::default(), 30).unwrap_err(), Error::InvalidData);
    }

    #[test]
    fn export_rejects_bad_envelopes_and_overflow_without_truncating() {
        let mut snapshot = sample();
        for generated in [f64::NAN, f64::INFINITY, -1.0, 0.0, JSON_SAFE as f64 / 1000.0] {
            snapshot.generated_at = generated;
            assert_eq!(project(&snapshot, &Config::default(), 30).unwrap_err(), Error::InvalidData);
        }
        let mut snapshot = sample();
        for interval in [0, 9, 3601, u64::MAX] {
            assert_eq!(project(&snapshot, &Config::default(), interval).unwrap_err(), Error::InvalidData);
        }
        assert!(project(&snapshot, &Config::default(), 10).is_ok());
        assert!(project(&snapshot, &Config::default(), 3600).is_ok());
        snapshot.devices = vec![snapshot.devices[0].clone(); 128];
        assert!(project(&snapshot, &Config::default(), 30).is_ok());
        snapshot.devices.push(snapshot.devices[0].clone());
        assert_eq!(project(&snapshot, &Config::default(), 30).unwrap_err(), Error::InvalidData);
        snapshot.devices.truncate(128);
        for device in &mut snapshot.devices {
            device.components = ["left", "right", "case"]
                .into_iter()
                .map(|id| Component { id: id.into(), percent: Some(100), charging: Charging::Discharging })
                .collect();
        }
        let bytes = project(&snapshot, &Config::default(), 30).unwrap();
        assert!(bytes.len() <= MAX_BYTES);
    }

    #[test]
    fn export_invalid_percentage_stays_unknown_instead_of_zero() {
        let mut snapshot = sample();
        snapshot.devices[0].battery.percent = Some(255);
        snapshot.devices[0].components.push(Component {
            id: "left".into(),
            percent: Some(101),
            charging: Charging::Unknown,
        });
        let json = value(&snapshot, &Config::default());
        assert!(json["devices"][0]["battery"]["percent"].is_null());
        assert!(json["devices"][0]["components"][0]["percent"].is_null());
    }

    pub(super) struct Owned {
        root: PathBuf,
        pub(super) paths: Paths,
    }

    impl Owned {
        pub(super) fn new() -> Self {
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!("perigauge-export-test-{}-{sequence}", std::process::id()));
            fs::create_dir(&root).unwrap();
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
            let paths = Paths { config: root.join("config/perigauge"), state: root.join("state/perigauge") };
            Self { root, paths }
        }

        pub(super) fn export(&self) -> PathBuf {
            self.paths.state.join("exports").join(EXPORT_FILE)
        }
        fn consent(&self) -> PathBuf {
            self.paths.config.join("exports").join(CONSENT_FILE)
        }

        pub(super) fn publish(&self, snapshot: &Snapshot, now: f64) -> Result<()> {
            let _guard = try_lock(&self.paths)?;
            publish_after_save(&self.paths, snapshot, 30, now)
        }
    }

    impl Drop for Owned {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }

    #[test]
    fn export_default_off_and_status_never_create_export_directories() {
        let owned = Owned::new();
        let state = command(&owned.paths, Action::Status).unwrap();
        assert!(!state.enabled && !state.export_present);
        assert!(!owned.paths.config.exists() && !owned.paths.state.exists());
        owned.publish(&sample(), 1000.2).unwrap();
        assert!(!owned.paths.state.join("exports").exists());
        assert!(!owned.paths.config.exists());
        assert!(!command(&owned.paths, Action::Disable).unwrap().enabled);
        assert!(!owned.paths.config.exists());
        assert!(!owned.paths.state.join("exports").exists());
    }

    #[test]
    fn export_enable_waits_for_collection_disable_removes_and_blocks_republish() {
        let owned = Owned::new();
        let status = command(&owned.paths, Action::Enable).unwrap();
        assert!(status.enabled && !status.export_present);
        assert!(!owned.paths.state.join("exports").exists());
        assert_eq!(fs::read(owned.consent()).unwrap(), br#"{"formatVersion":"0.1-experimental","enabled":true}"#);
        owned.publish(&sample(), 1000.2).unwrap();
        assert!(owned.export().exists());
        let before = fs::read(owned.export()).unwrap();
        let status = command(&owned.paths, Action::Status).unwrap();
        assert_eq!(status.export_bytes, Some(before.len() as u64));
        assert_eq!(fs::read(owned.export()).unwrap(), before);
        let status = command(&owned.paths, Action::Disable).unwrap();
        assert!(!status.enabled && !status.export_present);
        owned.publish(&sample(), 1000.3).unwrap();
        assert!(!owned.export().exists());
        command(&owned.paths, Action::Enable).unwrap();
        assert!(!owned.export().exists());
    }

    #[test]
    fn export_fresh_consent_removes_old_export_without_republishing_cache() {
        let owned = Owned::new();
        command(&owned.paths, Action::Enable).unwrap();
        owned.publish(&sample(), 1000.2).unwrap();
        let old = fs::read(owned.export()).unwrap();
        command(&owned.paths, Action::Enable).unwrap();
        assert!(!owned.export().exists());
        // Genuine collection may preserve old observations: the timestamp stays old.
        let mut newer = sample();
        newer.generated_at = 1200.0;
        owned.publish(&newer, 1200.0).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&fs::read(owned.export()).unwrap()).unwrap();
        assert_eq!(json["generatedAtMs"], 1_200_000);
        assert_eq!(json["devices"][0]["observedAtMs"], 1_000_000);
        assert_eq!(json["devices"][0]["stale"], true);
        assert_ne!(fs::read(owned.export()).unwrap(), old);
    }

    #[test]
    fn export_invalid_or_unreadable_consent_is_off_and_cleans_old_export() {
        let owned = Owned::new();
        for contents in [
            "{}",
            "not json",
            "{\"formatVersion\":\"0.1-experimental\",\"enabled\":true,\"extra\":0}",
            "{\"formatVersion\":\"0.1-experimental\",\"enabled\":true,\"enabled\":false}",
            "{\"formatVersion\":\"other\",\"enabled\":true}",
            "{\"formatVersion\":\"0.1-experimental\",\"enabled\":\"true\"}",
        ] {
            command(&owned.paths, Action::Enable).unwrap();
            owned.publish(&sample(), 1000.2).unwrap();
            fs::write(owned.consent(), contents).unwrap();
            assert!(!command(&owned.paths, Action::Status).unwrap().enabled);
            let _guard = try_lock(&owned.paths).unwrap();
            observe_disabled(&owned.paths).unwrap();
            assert!(!owned.export().exists());
        }
        command(&owned.paths, Action::Enable).unwrap();
        owned.publish(&sample(), 1000.2).unwrap();
        fs::set_permissions(owned.consent(), fs::Permissions::from_mode(0o0)).unwrap();
        assert!(!enabled(&owned.paths));
        owned.publish(&sample(), 1000.3).unwrap();
        assert!(!owned.export().exists());
    }

    #[test]
    fn export_current_hidden_config_is_reread_and_errors_fail_closed() {
        let owned = Owned::new();
        command(&owned.paths, Action::Enable).unwrap();
        owned.publish(&sample(), 1000.2).unwrap();
        let config = owned.paths.config.join("config.toml");
        fs::write(&config, "# preserved comment\n[devices.\"PRIVATE-ID\"]\nhidden = true\n").unwrap();
        let original = fs::read(&config).unwrap();
        owned.publish(&sample(), 1000.2).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&fs::read(owned.export()).unwrap()).unwrap();
        assert_eq!(json["devices"], serde_json::json!([]));
        command(&owned.paths, Action::Disable).unwrap();
        command(&owned.paths, Action::Enable).unwrap();
        assert_eq!(fs::read(&config).unwrap(), original);
        owned.publish(&sample(), 1000.2).unwrap();
        assert!(owned.export().exists());
        for invalid in ["[broken", "interval_seconds = 'no'"] {
            fs::write(&config, invalid).unwrap();
            assert_eq!(owned.publish(&sample(), 1000.2).unwrap_err(), Error::InvalidData);
            assert!(!owned.export().exists());
        }
        fs::write(&config, "").unwrap();
        owned.publish(&sample(), 1000.2).unwrap();
        fs::set_permissions(&config, fs::Permissions::from_mode(0o0)).unwrap();
        assert!(owned.publish(&sample(), 1000.2).is_err());
        assert!(!owned.export().exists());
        fs::remove_file(&config).unwrap();
        symlink(owned.root.join("missing"), &config).unwrap();
        assert!(owned.publish(&sample(), 1000.2).is_err());
        assert!(!owned.export().exists());
    }

    #[test]
    fn export_never_publishes_future_expired_or_invalid_envelopes() {
        let owned = Owned::new();
        command(&owned.paths, Action::Enable).unwrap();
        for now in [1000.0, 1090.125, f64::NAN] {
            assert_eq!(owned.publish(&sample(), now).unwrap_err(), Error::InvalidData);
            assert!(!owned.export().exists());
        }
        assert!(owned.publish(&sample(), 1000.2).is_ok());
        // The envelope is not expired yet, but the observation reached its lease.
        owned.publish(&sample(), 1090.0).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&fs::read(owned.export()).unwrap()).unwrap();
        assert_eq!(json["generatedAtMs"], 1_000_125);
        assert_eq!(json["expiresAtMs"], 1_090_125);
        assert_eq!(json["devices"][0]["stale"], true);
    }

    #[test]
    fn export_modes_atomic_replace_and_no_temporary_leftovers() {
        let owned = Owned::new();
        command(&owned.paths, Action::Enable).unwrap();
        owned.publish(&sample(), 1000.2).unwrap();
        let old_fd = File::open(owned.export()).unwrap();
        let old_inode = old_fd.metadata().unwrap().ino();
        let old_bytes = fs::read(owned.export()).unwrap();
        let mut changed = sample();
        changed.devices[0].battery.percent = Some(7);
        owned.publish(&changed, 1000.3).unwrap();
        assert_ne!(fs::metadata(owned.export()).unwrap().ino(), old_inode);
        let mut held_bytes = Vec::new();
        old_fd.take(MAX_BYTES as u64).read_to_end(&mut held_bytes).unwrap();
        assert_eq!(held_bytes, old_bytes);
        for path in [
            owned.paths.config.clone(),
            owned.paths.config.join("exports"),
            owned.paths.state.clone(),
            owned.paths.state.join("exports"),
        ] {
            assert_eq!(fs::metadata(path).unwrap().mode() & 0o777, 0o700);
        }
        for path in [owned.consent(), owned.export(), owned.paths.state.join("collect.lock")] {
            assert_eq!(fs::metadata(path).unwrap().mode() & 0o777, 0o600);
        }
        assert_eq!(fs::read_dir(owned.paths.state.join("exports")).unwrap().count(), 1);
        assert_eq!(fs::read_dir(owned.paths.config.join("exports")).unwrap().count(), 1);
    }

    #[test]
    fn export_symlink_parent_file_hardlink_and_fifo_are_refused_without_touching_target() {
        let owned = Owned::new();
        let target = owned.root.join("sentinel");
        fs::write(&target, b"PRIVATE-SENTINEL").unwrap();
        command(&owned.paths, Action::Enable).unwrap();
        fs::remove_file(owned.consent()).unwrap();
        symlink(&target, owned.consent()).unwrap();
        assert!(!enabled(&owned.paths));
        assert!(command(&owned.paths, Action::Enable).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"PRIVATE-SENTINEL");
        fs::remove_file(owned.consent()).unwrap();
        fs::hard_link(&target, owned.consent()).unwrap();
        assert!(command(&owned.paths, Action::Enable).is_err());
        fs::remove_file(owned.consent()).unwrap();
        let fifo = cstr(owned.consent().as_os_str()).unwrap();
        // SAFETY: synthetic test FIFO inside this test's owned tree only.
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
        assert!(!enabled(&owned.paths));
        assert!(command(&owned.paths, Action::Enable).is_err());
        fs::remove_file(owned.consent()).unwrap();
        fs::remove_dir(owned.paths.config.join("exports")).unwrap();
        let outside = owned.root.join("outside");
        fs::create_dir(&outside).unwrap();
        symlink(&outside, owned.paths.config.join("exports")).unwrap();
        assert!(command(&owned.paths, Action::Enable).is_err());
        assert_eq!(fs::read_dir(outside).unwrap().count(), 0);
        assert_eq!(fs::read(target).unwrap(), b"PRIVATE-SENTINEL");
    }

    #[test]
    fn export_cli_busy_refuses_without_changing_consent_or_existing_export() {
        let owned = Owned::new();
        command(&owned.paths, Action::Enable).unwrap();
        owned.publish(&sample(), 1000.2).unwrap();
        let consent = fs::read(owned.consent()).unwrap();
        let export = fs::read(owned.export()).unwrap();
        let guard = try_lock(&owned.paths).unwrap();
        for action in [Action::Disable, Action::Enable] {
            assert_eq!(command(&owned.paths, action).unwrap_err(), Error::Busy);
        }
        assert_eq!(fs::read(owned.consent()).unwrap(), consent);
        assert_eq!(fs::read(owned.export()).unwrap(), export);
        drop(guard);
        command(&owned.paths, Action::Disable).unwrap();
        owned.publish(&sample(), 1000.3).unwrap();
        assert!(!owned.export().exists());
    }

    #[test]
    fn export_disable_does_not_acknowledge_failed_removal_but_permission_is_off() {
        let owned = Owned::new();
        command(&owned.paths, Action::Enable).unwrap();
        owned.publish(&sample(), 1000.2).unwrap();
        fs::remove_file(owned.export()).unwrap();
        fs::create_dir(owned.export()).unwrap();
        assert!(command(&owned.paths, Action::Disable).is_err());
        assert!(!enabled(&owned.paths));
        fs::remove_dir(owned.export()).unwrap();
        assert!(!command(&owned.paths, Action::Disable).unwrap().enabled);
        owned.publish(&sample(), 1000.3).unwrap();
        assert!(!owned.export().exists());
    }

    #[test]
    fn review_non_object_consent_must_be_disabled() {
        let owned = Owned::new();
        command(&owned.paths, Action::Enable).unwrap();
        fs::write(owned.consent(), br#"["0.1-experimental",true]"#).unwrap();
        assert!(!command(&owned.paths, Action::Status).unwrap().enabled);
    }

    #[test]
    fn review_invalid_array_consent_must_not_publish_after_disable() {
        let owned = Owned::new();
        command(&owned.paths, Action::Enable).unwrap();
        let disabled = command(&owned.paths, Action::Disable).unwrap();
        assert!(!disabled.enabled && !disabled.export_present);
        fs::write(owned.consent(), br#"["0.1-experimental",true]"#).unwrap();
        owned.publish(&sample(), 1000.2).unwrap();
        assert!(!owned.export().exists());
    }

    #[test]
    fn review_array_consent_must_trigger_collection_start_cleanup() {
        let owned = Owned::new();
        command(&owned.paths, Action::Enable).unwrap();
        owned.publish(&sample(), 1000.2).unwrap();
        assert!(owned.export().exists());
        fs::write(owned.consent(), br#"["0.1-experimental",true]"#).unwrap();
        let _guard = try_lock(&owned.paths).unwrap();
        observe_disabled(&owned.paths).unwrap();
        assert!(!owned.export().exists());
    }

    #[test]
    fn review_object_consent_preserves_field_order_whitespace_and_false() {
        let cases: &[(&[u8], bool)] = &[
            (br#"{"formatVersion":"0.1-experimental","enabled":true}"#, true),
            (br#"{"enabled":true,"formatVersion":"0.1-experimental"}"#, true),
            (b" \t\r\n{ \"enabled\" : true, \"formatVersion\" : \"0.1-experimental\" }\n\t ", true),
            (br#"{"enabled":false,"formatVersion":"0.1-experimental"}"#, false),
        ];
        for (bytes, expected) in cases {
            let owned = Owned::new();
            command(&owned.paths, Action::Enable).unwrap();
            fs::write(owned.consent(), bytes).unwrap();
            assert_eq!(command(&owned.paths, Action::Status).unwrap().enabled, *expected);
            owned.publish(&sample(), 1000.2).unwrap();
            assert_eq!(owned.export().exists(), *expected);
            assert_eq!(fs::read(owned.consent()).unwrap(), *bytes);
        }
    }

    #[test]
    fn review_invalid_consent_matrix_fails_closed_for_cleanup_and_publish() {
        let cases: &[&[u8]] = &[
            br#"["0.1-experimental",true]"#,
            b" \t[\"0.1-experimental\",true]\n",
            b"[]",
            b"[[]]",
            br#"[{"formatVersion":"0.1-experimental","enabled":true}]"#,
            b"true",
            b"false",
            b"null",
            b"1",
            br#""consent""#,
            b"",
            b" \t\r\n",
            b"{}",
            br#"{"formatVersion":"0.1-experimental"}"#,
            br#"{"enabled":true}"#,
            br#"{"formatVersion":"other","enabled":true}"#,
            br#"{"formatVersion":"0.1-experimental","enabled":"true"}"#,
            br#"{"formatVersion":"0.1-experimental","enabled":1}"#,
            br#"{"formatVersion":"0.1-experimental","enabled":null}"#,
            br#"{"formatVersion":"0.1-experimental","enabled":[]}"#,
            br#"{"formatVersion":true,"enabled":true}"#,
            br#"{"formatVersion":"0.1-experimental","enabled":true,"enabled":true}"#,
            br#"{"formatVersion":"0.1-experimental","enabled":false,"enabled":true}"#,
            br#"{"formatVersion":"0.1-experimental","formatVersion":"0.1-experimental","enabled":true}"#,
            br#"{"formatVersion":"0.1-experimental","enabled":true,"enabl\u0065d":true}"#,
            br#"{"formatVersion":"0.1-experimental","enabled":true,"extra":0}"#,
            br#"{"formatVersion":"0.1-experimental","enabled":NaN}"#,
            br#"{"formatVersion":"0.1-experimental","enabled":Infinity}"#,
            br#"{"formatVersion":"0.1-experimental","enabled":-Infinity}"#,
            br#"{"formatVersion":"0.1-experimental","enabled":1e999}"#,
            br#"{"formatVersion":"0.1-experimental","enabled":true}{}"#,
            br#"{"formatVersion":"0.1-experimental","enabled":true}[]"#,
            br#"{"formatVersion":"0.1-experimental","enabled":true}x"#,
            b"\x0b{\"formatVersion\":\"0.1-experimental\",\"enabled\":true}",
            b"\x0c{\"formatVersion\":\"0.1-experimental\",\"enabled\":true}",
            &[0xff],
            &[b' '; 257],
        ];
        for (index, bytes) in cases.iter().enumerate() {
            let owned = Owned::new();
            command(&owned.paths, Action::Enable).unwrap();
            owned.publish(&sample(), 1000.2).unwrap();
            fs::write(owned.consent(), bytes).unwrap();
            {
                let _guard = try_lock(&owned.paths).unwrap();
                observe_disabled(&owned.paths).unwrap();
            }
            assert!(!owned.export().exists(), "cleanup case {index}");
            assert!(!command(&owned.paths, Action::Status).unwrap().enabled, "status case {index}");

            command(&owned.paths, Action::Enable).unwrap();
            let disabled = command(&owned.paths, Action::Disable).unwrap();
            assert!(!disabled.enabled && !disabled.export_present);
            fs::write(owned.consent(), bytes).unwrap();
            owned.publish(&sample(), 1000.2).unwrap();
            assert!(!owned.export().exists(), "publication case {index}");
            assert!(!command(&owned.paths, Action::Status).unwrap().enabled, "revoked case {index}");
        }
    }
}

#[cfg(test)]
mod export_concurrency_tests {
    use super::export_tests::Owned;
    use super::*;
    use crate::model::{Device, SCHEMA_VERSION};
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn export_inflight_publication_finishes_before_acknowledged_revoke_and_cannot_republish() {
        let owned = Owned::new();
        command(&owned.paths, Action::Enable).unwrap();
        let paths = owned.paths.clone();
        let (locked_tx, locked_rx) = mpsc::channel();
        let (continue_tx, continue_rx) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let guard = try_lock(&paths).unwrap();
            locked_tx.send(()).unwrap();
            continue_rx.recv_timeout(Duration::from_secs(3)).unwrap();
            let mut device = Device::new("PRIVATE-ID", "PRIVATE-NAME", Kind::Mouse, "PRIVATE-BACKEND");
            device.updated_at = Some(1000.0);
            let snapshot = Snapshot {
                schema: SCHEMA_VERSION,
                version: "PRIVATE-VERSION".into(),
                generated_at: 1000.125,
                source: "PRIVATE-SOURCE".into(),
                devices: vec![device],
                issues: vec![],
            };
            publish_after_save(&paths, &snapshot, 30, 1000.2).unwrap();
            drop(guard);
            snapshot
        });
        locked_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(command(&owned.paths, Action::Disable).unwrap_err(), Error::Busy);
        continue_tx.send(()).unwrap();
        let snapshot = worker.join().unwrap();
        assert!(owned.export().exists());
        command(&owned.paths, Action::Disable).unwrap();
        owned.publish(&snapshot, 1000.3).unwrap();
        assert!(!owned.export().exists());
    }
}
