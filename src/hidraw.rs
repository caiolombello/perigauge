//! Minimal hidraw access: sysfs enumeration, report descriptor parsing and
//! request/response I/O with a deadline. No libudev or hidapi dependency.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const BUS_USB: u16 = 0x0003;
pub const BUS_BLUETOOTH: u16 = 0x0005;

#[derive(Clone, Debug, PartialEq)]
pub struct HidrawInfo {
    pub node: PathBuf,
    pub bus: u16,
    pub vendor: u16,
    pub product: u16,
    pub name: String,
    /// Bluetooth MAC for BT devices, serial for some USB devices, often empty.
    pub uniq: String,
    pub phys: String,
    pub descriptor: Vec<u8>,
}

/// Parse a hidraw `device/uevent` file. Returns (bus, vendor, product, name, uniq, phys).
pub fn parse_uevent(text: &str) -> Option<(u16, u16, u16, String, String, String)> {
    let mut id = None;
    let (mut name, mut uniq, mut phys) = (String::new(), String::new(), String::new());
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("HID_ID=") {
            let mut parts = value.split(':');
            let bus = u32::from_str_radix(parts.next()?, 16).ok()?;
            let vendor = u32::from_str_radix(parts.next()?, 16).ok()?;
            let product = u32::from_str_radix(parts.next()?, 16).ok()?;
            id = Some((bus as u16, vendor as u16, product as u16));
        } else if let Some(value) = line.strip_prefix("HID_NAME=") {
            name = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("HID_UNIQ=") {
            uniq = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("HID_PHYS=") {
            phys = value.trim().to_string();
        }
    }
    let (bus, vendor, product) = id?;
    Some((bus, vendor, product, name, uniq, phys))
}

pub fn enumerate() -> Vec<HidrawInfo> {
    enumerate_in(Path::new("/sys/class/hidraw"))
}

fn enumerate_in(base: &Path) -> Vec<HidrawInfo> {
    let Ok(entries) = fs::read_dir(base) else { return Vec::new() };
    let mut found: Vec<HidrawInfo> = entries
        .flatten()
        .filter_map(|entry| {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            if !file_name.starts_with("hidraw") {
                return None;
            }
            let device = entry.path().join("device");
            let uevent = fs::read_to_string(device.join("uevent")).ok()?;
            let (bus, vendor, product, name, uniq, phys) = parse_uevent(&uevent)?;
            let descriptor = fs::read(device.join("report_descriptor")).unwrap_or_default();
            Some(HidrawInfo {
                node: Path::new("/dev").join(&file_name),
                bus,
                vendor,
                product,
                name,
                uniq,
                phys,
                descriptor,
            })
        })
        .collect();
    found.sort_by(|a, b| a.node.cmp(&b.node));
    found
}

/// One short item of a HID report descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Item {
    pub kind: u8,
    pub tag: u8,
    pub data: u32,
}

const TYPE_GLOBAL: u8 = 1;
const TAG_USAGE_PAGE: u8 = 0;
const TAG_REPORT_ID: u8 = 8;

/// Iterate short items; long items are skipped, truncated input ends the walk.
pub fn items(descriptor: &[u8]) -> Vec<Item> {
    let mut result = Vec::new();
    let mut index = 0;
    while index < descriptor.len() {
        let prefix = descriptor[index];
        if prefix == 0xFE {
            let Some(&size) = descriptor.get(index + 1) else { break };
            index += 3 + size as usize;
            continue;
        }
        let size = match prefix & 0x03 {
            3 => 4,
            n => n as usize,
        };
        let Some(bytes) = descriptor.get(index + 1..index + 1 + size) else { break };
        let data = bytes.iter().rev().fold(0u32, |acc, b| (acc << 8) | *b as u32);
        result.push(Item { kind: (prefix >> 2) & 0x03, tag: prefix >> 4, data });
        index += 1 + size;
    }
    result
}

pub fn usage_pages(descriptor: &[u8]) -> Vec<u16> {
    items(descriptor)
        .into_iter()
        .filter(|item| item.kind == TYPE_GLOBAL && item.tag == TAG_USAGE_PAGE)
        .map(|item| item.data as u16)
        .collect()
}

pub fn report_ids(descriptor: &[u8]) -> Vec<u8> {
    items(descriptor)
        .into_iter()
        .filter(|item| item.kind == TYPE_GLOBAL && item.tag == TAG_REPORT_ID)
        .map(|item| item.data as u8)
        .collect()
}

pub fn error_code(error: &io::Error) -> String {
    match error.raw_os_error() {
        Some(libc::EACCES) | Some(libc::EPERM) => "permission-denied".to_string(),
        Some(libc::ENOENT) | Some(libc::ENODEV) => "gone".to_string(),
        Some(code) => format!("io-{code}"),
        None => "io".to_string(),
    }
}

pub struct Hidraw {
    file: File,
}

impl Hidraw {
    pub fn open(node: &Path) -> io::Result<Self> {
        let file =
            OpenOptions::new().read(true).write(true).custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC).open(node)?;
        Ok(Hidraw { file })
    }

    /// Discard input reports queued before our request.
    pub fn drain(&mut self) {
        let mut buffer = [0u8; 256];
        while let Ok(n) = self.file.read(&mut buffer) {
            if n == 0 {
                break;
            }
        }
    }

    pub fn write(&mut self, report: &[u8]) -> io::Result<()> {
        self.file.write_all(report)
    }

    /// Read one input report, waiting until `deadline`. `Ok(None)` on timeout.
    pub fn read_until(&mut self, deadline: Instant) -> io::Result<Option<Vec<u8>>> {
        let mut buffer = [0u8; 256];
        loop {
            match self.file.read(&mut buffer) {
                Ok(n) => return Ok(Some(buffer[..n].to_vec())),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(None);
            }
            let mut poll = libc::pollfd { fd: self.file.as_raw_fd(), events: libc::POLLIN, revents: 0 };
            let timeout = remaining.min(Duration::from_millis(250)).as_millis().max(1) as libc::c_int;
            // SAFETY: `poll` points to one valid pollfd for the duration of the call.
            let ready = unsafe { libc::poll(&mut poll, 1, timeout) };
            if ready < 0 {
                let error = io::Error::last_os_error();
                if error.kind() != io::ErrorKind::Interrupted {
                    return Err(error);
                }
            } else if ready > 0 && poll.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
                return Err(io::Error::from_raw_os_error(libc::ENODEV));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_uevent() {
        let text = "DRIVER=hid-generic\nHID_ID=0005:0000046D:0000B034\nHID_NAME=MX Master 3S\nHID_PHYS=aa:bb\nHID_UNIQ=d4:12:34:56:78:9a\n";
        let (bus, vendor, product, name, uniq, phys) = parse_uevent(text).unwrap();
        assert_eq!((bus, vendor, product), (BUS_BLUETOOTH, 0x046d, 0xb034));
        assert_eq!(name, "MX Master 3S");
        assert_eq!(uniq, "d4:12:34:56:78:9a");
        assert_eq!(phys, "aa:bb");
        assert!(parse_uevent("HID_NAME=x\n").is_none());
    }

    #[test]
    fn walks_descriptor_items() {
        // Usage Page (0xFF00), Usage (1), Collection, Report ID 0x10, ..., Report ID 0x11
        let descriptor = [0x06, 0x00, 0xFF, 0x09, 0x01, 0xA1, 0x01, 0x85, 0x10, 0x75, 0x08, 0x85, 0x11, 0xC0];
        assert_eq!(usage_pages(&descriptor), vec![0xFF00]);
        assert_eq!(report_ids(&descriptor), vec![0x10, 0x11]);
    }

    #[test]
    fn truncated_descriptor_does_not_panic() {
        assert_eq!(report_ids(&[0x85]), Vec::<u8>::new());
        assert_eq!(items(&[0xFE, 0x10]).len(), 0);
    }

    #[test]
    fn maps_permission_errors() {
        assert_eq!(error_code(&io::Error::from_raw_os_error(libc::EACCES)), "permission-denied");
        assert_eq!(error_code(&io::Error::from_raw_os_error(libc::EIO)), format!("io-{}", libc::EIO));
    }
}
