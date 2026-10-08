# Changelog

## 0.1.0-alpha.2 — 2026-10-08

- Always-visible battery meters for each device on the desktop and panel,
  including separate left bud, right bud and case readings.
- Switch between circular gauges and battery bars from the widget menu,
  settings or popup header. Each widget keeps its own preference; the panel
  and its popup use the same style.
- Dedicated mouse, keyboard, earbuds and case icons. Unknown, estimated,
  stale and out-of-range readings remain identifiable in both styles.
- The desktop card stays expanded; automatic panel mode shows every device.
  When enabled, out-of-range readings also keep the tray applet visible.
- Galaxy Buds3 Pro battery readings (left, right and case) verified on
  hardware over Bluetooth, alongside the Keychron M6 receiver. Logitech
  hardware validation remains pending.

## 0.1.0-alpha.1 — 2026-10-08

First PeriGauge build, replacing the Python `perif-battery` helper.

- Rust core: one `perigauge` binary with `status [--json] [--refresh]`,
  `daemon`, `doctor` and `notify-test`. Versioned JSON contract (schema 1)
  shared by the frontends.
- Backends:
  - Keychron Ultra-Link 8K vendor protocol (ported; verified on a Keychron M6).
  - Logitech HID++ 2.0 over hidraw for Bolt/Unifying/Lightspeed receivers and
    Bluetooth: features 0x1004, 0x1000 and 0x1001, unit id for a stable
    identity across links. Removes the Solaar dependency. Fixture-tested only.
  - Samsung Galaxy Buds SPP status (left, right, case, charging) through a
    BlueZ client profile. Fixture-tested only.
  - UPower fallback for every other device the system already reports, with
    duplicates dropped in favor of the specialized backends.
- Sleeping devices keep their last reading, marked stale; devices out of
  range stay listed for 7 days.
- Notifications at 20 %, 10 % and 5 % and when charged, with hysteresis,
  persisted memory across restarts, per-device opt-out and never on stale or
  estimated percentages. Language follows the session or `language` in
  `config.toml`.
- Collections are one locked transaction (read, query, merge, publish), so a
  manual refresh and the daemon never interleave HID requests or overwrite a
  newer snapshot; D-Bus calls have a 5 s timeout and HID++ a 6 s budget.
- Notifications are remembered only after the desktop accepted them, so an
  alert lost to a missing notification server is retried.
- KDE Plasma 6 widget for the panel (compact + popup) and the desktop
  (gauge, list or grid layouts, configurable background).
- User service (`systemd --user`), own udev rule, installer and uninstaller
  with `--dry-run`, brand identity and a PT/EN landing page.

Known limitations: Logitech and Galaxy Buds paths are not yet validated on
hardware; if a Logitech device is first seen through the kernel/UPower and
later through HID++, its notification memory and overrides restart under the
new id.
