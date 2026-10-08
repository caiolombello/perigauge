//! Keychron Ultra-Link 8K receiver (USB 3434:d028) vendor protocol.
//!
//! Reverse-engineered: the 0xFFC1 vendor interface answers a status request
//! (out report 0xB3, command 0x05) with a 20-byte frame whose last byte is the
//! battery percentage. Report 0xB4 (command 0x06) starts with the same block.
//! The receiver only answers while the radio link is up, so a silent receiver
//! means a sleeping or switched-off device, not a missing receiver.

use std::time::{Duration, Instant};

use crate::hidraw::{self, Hidraw, HidrawInfo};
use crate::model::{Device, Kind, Link, now};

const VENDOR: u16 = 0x3434;
const ULTRA_LINK_8K: u16 = 0xd028;
const VENDOR_PAGE: u16 = 0xFFC1;
const REPORT_OUT: u8 = 0xB3;
const REPORT_IN_STATUS: u8 = 0xB6;
const REPORT_IN_BIG: u8 = 0xB4;
const STATUS_CMD: u8 = 0x05;
const ATTEMPTS: usize = 3;
const TIMEOUT: Duration = Duration::from_millis(450);

pub const DEVICE_ID: &str = "keychron:3434:d028";

pub fn is_vendor_interface(info: &HidrawInfo) -> bool {
    info.vendor == VENDOR
        && info.product == ULTRA_LINK_8K
        && hidraw::usage_pages(&info.descriptor).first() == Some(&VENDOR_PAGE)
}

pub fn status_request() -> [u8; 64] {
    let mut report = [0u8; 64];
    report[0] = REPORT_OUT;
    report[1] = STATUS_CMD;
    report
}

/// Battery percentage from an input report, if it is a status reply.
pub fn parse_status(report: &[u8]) -> Option<Option<u8>> {
    let (&id, payload) = report.split_first()?;
    if !matches!(id, REPORT_IN_STATUS | REPORT_IN_BIG) || payload.len() < 20 || payload[0] != STATUS_CMD {
        return None;
    }
    let level = payload[19];
    Some((level <= 100).then_some(level))
}

fn query(device: &mut Hidraw) -> Result<Option<u8>, String> {
    for _ in 0..ATTEMPTS {
        device.drain();
        device.write(&status_request()).map_err(|e| hidraw::error_code(&e))?;
        let deadline = Instant::now() + TIMEOUT;
        while let Some(report) = device.read_until(deadline).map_err(|e| hidraw::error_code(&e))? {
            if let Some(level) = parse_status(&report) {
                return Ok(level);
            }
        }
    }
    Err("asleep".to_string())
}

pub fn collect(nodes: &[HidrawInfo]) -> Vec<Device> {
    let Some(info) = nodes.iter().find(|info| is_vendor_interface(info)) else { return Vec::new() };
    let mut device = Device::new(DEVICE_ID, "Keychron (Ultra-Link 8K)", Kind::Mouse, "keychron");
    device.vendor = Some("Keychron".into());
    device.link = Link::Receiver;
    device.via = Some("Keychron Ultra-Link 8K".into());
    match Hidraw::open(&info.node).map_err(|e| hidraw::error_code(&e)).and_then(|mut h| query(&mut h)) {
        Ok(Some(level)) => {
            device.battery.percent = Some(level);
            device.updated_at = Some(now());
        }
        Ok(None) => device.error = Some("invalid-reading".into()),
        Err(code) => device.error = Some(code),
    }
    vec![device]
}

#[cfg(test)]
mod tests {
    use super::*;

    // Captured from a Keychron M6 at 69% (0x45) through an Ultra-Link 8K.
    const STATUS: [u8; 21] = [
        0xb6, 0x05, 0x01, 0x11, 0x11, 0x01, 0x90, 0x01, 0x20, 0x03, 0xb0, 0x04, 0x40, 0x06, 0x80, 0x0c, 0x01, 0x05,
        0x03, 0x1e, 0x45,
    ];

    #[test]
    fn parses_captured_status_frame() {
        assert_eq!(parse_status(&STATUS), Some(Some(69)));
        let mut big = STATUS.to_vec();
        big[0] = 0xb4;
        big.extend([0u8; 40]);
        assert_eq!(parse_status(&big), Some(Some(69)));
    }

    #[test]
    fn rejects_other_frames_and_invalid_levels() {
        let mut other = STATUS;
        other[1] = 0x02;
        assert_eq!(parse_status(&other), None);
        assert_eq!(parse_status(&STATUS[..10]), None);
        let mut invalid = STATUS;
        invalid[20] = 0xff;
        assert_eq!(parse_status(&invalid), Some(None));
    }

    #[test]
    fn identifies_vendor_interface_by_first_usage_page() {
        let info = |descriptor: Vec<u8>| HidrawInfo {
            node: "/dev/hidraw0".into(),
            bus: hidraw::BUS_USB,
            vendor: VENDOR,
            product: ULTRA_LINK_8K,
            name: "Keychron Keychron Ultra-Link 8K".into(),
            uniq: String::new(),
            phys: String::new(),
            descriptor,
        };
        assert!(is_vendor_interface(&info(vec![0x06, 0xC1, 0xFF, 0x09, 0x01])));
        assert!(!is_vendor_interface(&info(vec![0x05, 0x01, 0x09, 0x02])));
    }
}
