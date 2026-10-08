//! Battery notifications: a pure threshold policy with hysteresis, persisted
//! memory so restarts do not repeat alerts, and the freedesktop D-Bus sender.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use zbus::blocking::Connection;
use zbus::zvariant::Value;

use crate::config::{Config, Notifications};
use crate::model::{Battery, Device, Kind, Level};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Warning,
    Critical,
    Shutdown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alert {
    Low(Severity),
    Charged,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Memory {
    /// Highest severity already announced in the current discharge cycle.
    pub announced: Option<Severity>,
    pub charged_announced: bool,
    /// Notification id, so a newer alert replaces the older bubble.
    #[serde(default)]
    pub notification_id: u32,
}

fn severity(policy: &Notifications, battery: &Battery) -> Option<Severity> {
    match (battery.percent, battery.estimated, battery.level) {
        (Some(percent), false, _) => match percent {
            p if p <= policy.shutdown => Some(Severity::Shutdown),
            p if p <= policy.critical => Some(Severity::Critical),
            p if p <= policy.warning => Some(Severity::Warning),
            _ => None,
        },
        // Estimated or coarse readings only alert on the device's own verdict.
        (_, _, Some(Level::Critical)) => Some(Severity::Critical),
        (_, _, Some(Level::Low)) => Some(Severity::Warning),
        _ => None,
    }
}

fn threshold(policy: &Notifications, severity: Severity) -> u8 {
    match severity {
        Severity::Warning => policy.warning,
        Severity::Critical => policy.critical,
        Severity::Shutdown => policy.shutdown,
    }
}

/// Decide whether this reading deserves an alert, updating `memory`.
pub fn evaluate(policy: &Notifications, device: &Device, memory: &mut Memory) -> Option<Alert> {
    if !device.present || device.stale || !device.battery.is_known() {
        return None;
    }
    let battery = &device.battery;
    if battery.charging.on_power() {
        memory.announced = None;
        let full = battery.charging == crate::model::Charging::Full || battery.percent == Some(100);
        if policy.charged && full && !memory.charged_announced {
            memory.charged_announced = true;
            return Some(Alert::Charged);
        }
        return None;
    }
    if battery.percent.is_none_or(|p| p + policy.hysteresis < 100) {
        memory.charged_announced = false;
    }
    let current = severity(policy, battery);
    if let (Some(announced), Some(percent)) = (memory.announced, battery.percent) {
        // Re-arm only after the level clearly recovers above the threshold.
        if percent >= threshold(policy, announced).saturating_add(policy.hysteresis) {
            memory.announced = current.filter(|s| *s < announced);
        }
    } else if memory.announced.is_some() && current.is_none() && battery.level.is_some_and(|l| l >= Level::Good) {
        memory.announced = None;
    }
    match current {
        Some(level) if memory.announced.is_none_or(|announced| level > announced) => {
            memory.announced = Some(level);
            Some(Alert::Low(level))
        }
        _ => None,
    }
}

pub struct Message {
    pub summary: String,
    pub body: String,
    pub icon: &'static str,
    /// freedesktop urgency: 0 low, 1 normal, 2 critical.
    pub urgency: u8,
}

pub fn portuguese() -> bool {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|var| std::env::var(var).ok().filter(|v| !v.is_empty()))
        .is_some_and(|value| value.starts_with("pt"))
}

fn icon(kind: Kind) -> &'static str {
    match kind {
        Kind::Mouse | Kind::Touchpad => "input-mouse",
        Kind::Keyboard => "input-keyboard",
        Kind::Earbuds | Kind::Headset => "audio-headset",
        Kind::Gamepad => "input-gaming",
        Kind::Other => "battery",
    }
}

pub fn message(device: &Device, alert: Alert, pt: bool) -> Message {
    let name = &device.name;
    let amount = match device.battery.percent {
        Some(percent) => format!("{percent}%"),
        None if pt => "nível baixo".to_string(),
        None => "low level".to_string(),
    };
    let (summary, body, urgency) = match (alert, pt) {
        (Alert::Low(Severity::Warning), true) => {
            (format!("{name}: bateria em {amount}"), "Carregue em breve.".to_string(), 1)
        }
        (Alert::Low(Severity::Warning), false) => {
            (format!("{name}: battery at {amount}"), "Charge it soon.".to_string(), 1)
        }
        (Alert::Low(Severity::Critical), true) => {
            (format!("{name}: bateria crítica ({amount})"), "Conecte o carregador assim que possível.".to_string(), 2)
        }
        (Alert::Low(Severity::Critical), false) => {
            (format!("{name}: battery critical ({amount})"), "Plug in the charger as soon as possible.".to_string(), 2)
        }
        (Alert::Low(Severity::Shutdown), true) => {
            (format!("{name}: {amount}, vai desligar"), "Conecte o carregador agora.".to_string(), 2)
        }
        (Alert::Low(Severity::Shutdown), false) => {
            (format!("{name}: {amount}, about to shut down"), "Plug in the charger now.".to_string(), 2)
        }
        (Alert::Charged, true) => {
            (format!("{name}: carregado"), "Bateria cheia; pode desconectar o carregador.".to_string(), 0)
        }
        (Alert::Charged, false) => {
            (format!("{name}: charged"), "Battery full; you can unplug the charger.".to_string(), 0)
        }
    };
    Message { summary, body, icon: icon(device.kind), urgency }
}

/// Send through org.freedesktop.Notifications; returns the server's id.
pub fn send(connection: &Connection, message: &Message, replaces: u32) -> zbus::Result<u32> {
    let mut hints: HashMap<&str, Value> = HashMap::new();
    hints.insert("urgency", Value::U8(message.urgency));
    hints.insert("desktop-entry", Value::from("perigauge"));
    hints.insert("category", Value::from("device"));
    let actions: Vec<&str> = Vec::new();
    let reply = connection.call_method(
        Some("org.freedesktop.Notifications"),
        "/org/freedesktop/Notifications",
        Some("org.freedesktop.Notifications"),
        "Notify",
        &("PeriGauge", replaces, message.icon, message.summary.as_str(), message.body.as_str(), actions, hints, -1i32),
    )?;
    reply.body().deserialize()
}

/// Evaluate every device and send what is due through `sender`, which gets
/// the message and the id to replace and returns the server's id. Memory is
/// only committed for alerts that were actually delivered, so an alert lost to
/// a missing or failing notification server is retried on the next cycle.
pub fn process(
    config: &Config,
    devices: &[Device],
    memory: &mut BTreeMap<String, Memory>,
    mut sender: impl FnMut(&Message, u32) -> Result<u32, String>,
) -> Vec<(String, Alert)> {
    let mut sent = Vec::new();
    if !config.notifications.enabled {
        return sent;
    }
    let pt = config.portuguese();
    for device in devices {
        if config.device(&device.id).notifications == Some(false) || device.hidden {
            continue;
        }
        let mut candidate = memory.get(&device.id).cloned().unwrap_or_default();
        let Some(alert) = evaluate(&config.notifications, device, &mut candidate) else {
            // Re-arming without an alert is still state worth keeping.
            memory.insert(device.id.clone(), candidate);
            continue;
        };
        match sender(&message(device, alert, pt), candidate.notification_id) {
            Ok(id) => {
                candidate.notification_id = id;
                memory.insert(device.id.clone(), candidate);
                sent.push((device.id.clone(), alert));
            }
            Err(error) => eprintln!("perigauge: {}: {error}", device.id),
        }
    }
    sent
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Charging;

    fn reading(percent: Option<u8>, charging: Charging) -> Device {
        let mut device = Device::new("hidpp:1", "MX Keys", Kind::Keyboard, "hidpp");
        device.battery = Battery { percent, charging, ..Battery::default() };
        device
    }

    fn run(levels: &[(u8, Charging)]) -> Vec<Option<Alert>> {
        let policy = Notifications::default();
        let mut memory = Memory::default();
        levels.iter().map(|(p, c)| evaluate(&policy, &reading(Some(*p), *c), &mut memory)).collect()
    }

    use Charging::{Charging as On, Discharging as Off, Full};
    use Severity::*;

    #[test]
    fn escalates_once_per_threshold_while_discharging() {
        let alerts = run(&[(50, Off), (20, Off), (19, Off), (10, Off), (9, Off), (5, Off), (4, Off)]);
        assert_eq!(
            alerts,
            vec![
                None,
                Some(Alert::Low(Warning)),
                None,
                Some(Alert::Low(Critical)),
                None,
                Some(Alert::Low(Shutdown)),
                None
            ]
        );
    }

    #[test]
    fn jitter_around_threshold_does_not_repeat() {
        let alerts = run(&[(21, Off), (20, Off), (21, Off), (22, Off), (20, Off), (24, Off), (19, Off)]);
        assert_eq!(alerts.iter().filter(|a| a.is_some()).count(), 1);
    }

    #[test]
    fn rearms_after_clear_recovery_or_charging() {
        let alerts = run(&[(18, Off), (26, Off), (19, Off)]);
        assert_eq!(alerts, vec![Some(Alert::Low(Warning)), None, Some(Alert::Low(Warning))]);
        let alerts = run(&[(9, Off), (12, On), (9, Off)]);
        assert_eq!(alerts, vec![Some(Alert::Low(Critical)), None, Some(Alert::Low(Critical))]);
    }

    #[test]
    fn first_reading_below_threshold_alerts_with_its_real_severity() {
        assert_eq!(run(&[(8, Off), (7, Off)]), vec![Some(Alert::Low(Critical)), None]);
    }

    #[test]
    fn charged_alert_once_per_charge() {
        let alerts =
            run(&[(90, On), (100, On), (100, Full), (100, Off), (99, Off), (100, Full), (80, Off), (100, Full)]);
        assert_eq!(alerts, vec![None, Some(Alert::Charged), None, None, None, None, None, Some(Alert::Charged)]);
    }

    #[test]
    fn stale_absent_and_estimated_percentages_never_alert() {
        let policy = Notifications::default();
        let mut memory = Memory::default();
        let mut stale = reading(Some(3), Off);
        stale.stale = true;
        assert_eq!(evaluate(&policy, &stale, &mut memory), None);
        let mut absent = reading(Some(3), Off);
        absent.present = false;
        assert_eq!(evaluate(&policy, &absent, &mut memory), None);
        let mut estimated = reading(Some(3), Off);
        estimated.battery.estimated = true;
        assert_eq!(evaluate(&policy, &estimated, &mut memory), None);
        assert_eq!(memory, Memory::default());
    }

    #[test]
    fn coarse_levels_alert_from_device_verdict() {
        let policy = Notifications::default();
        let mut memory = Memory::default();
        let mut coarse = reading(None, Off);
        coarse.battery.level = Some(Level::Low);
        coarse.battery.estimated = true;
        assert_eq!(evaluate(&policy, &coarse, &mut memory), Some(Alert::Low(Warning)));
        assert_eq!(evaluate(&policy, &coarse, &mut memory), None);
        coarse.battery.level = Some(Level::Critical);
        assert_eq!(evaluate(&policy, &coarse, &mut memory), Some(Alert::Low(Critical)));
        coarse.battery.level = Some(Level::Good);
        assert_eq!(evaluate(&policy, &coarse, &mut memory), None);
        coarse.battery.level = Some(Level::Low);
        assert_eq!(evaluate(&policy, &coarse, &mut memory), Some(Alert::Low(Warning)));
    }

    #[test]
    fn disabled_devices_and_global_switch_are_respected() {
        let mut config = Config::default();
        let devices = vec![reading(Some(5), Off)];
        let mut memory = BTreeMap::new();
        let ok = |_: &Message, _: u32| Ok(7);
        config
            .devices
            .insert("hidpp:1".into(), crate::config::DeviceConfig { notifications: Some(false), ..Default::default() });
        assert!(process(&config, &devices, &mut memory, ok).is_empty());
        config.devices.clear();
        config.notifications.enabled = false;
        assert!(process(&config, &devices, &mut memory, ok).is_empty());
        config.notifications.enabled = true;
        assert_eq!(process(&config, &devices, &mut memory, ok), vec![("hidpp:1".to_string(), Alert::Low(Shutdown))]);
        assert_eq!(memory["hidpp:1"].notification_id, 7);
    }

    #[test]
    fn failed_delivery_is_retried_next_cycle() {
        let config = Config::default();
        let devices = vec![reading(Some(15), Off)];
        let mut memory = BTreeMap::new();
        let failing = |_: &Message, _: u32| Err::<u32, String>("no server".into());
        assert!(process(&config, &devices, &mut memory, failing).is_empty());
        assert_eq!(memory.get("hidpp:1").and_then(|m| m.announced), None);
        let mut replaced = None;
        let sent = process(&config, &devices, &mut memory, |_: &Message, replaces: u32| {
            replaced = Some(replaces);
            Ok(42)
        });
        assert_eq!(sent, vec![("hidpp:1".to_string(), Alert::Low(Warning))]);
        assert_eq!(replaced, Some(0));
        // Delivered: the same level does not alert again.
        assert!(process(&config, &devices, &mut memory, |_: &Message, _: u32| Ok(43)).is_empty());
    }

    #[test]
    fn messages_are_localized_and_urgent_when_critical() {
        let device = reading(Some(9), Off);
        let pt = message(&device, Alert::Low(Critical), true);
        assert_eq!(pt.summary, "MX Keys: bateria crítica (9%)");
        assert_eq!(pt.urgency, 2);
        assert_eq!(pt.icon, "input-keyboard");
        let en = message(&device, Alert::Charged, false);
        assert_eq!(en.summary, "MX Keys: charged");
        assert_eq!(en.urgency, 0);
    }
}
