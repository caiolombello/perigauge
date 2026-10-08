# Security and privacy

PeriGauge 0.1.0-alpha.1 is a pre-alpha with no production support guarantee.

## What it accesses

- **Local only.** No network requests, no telemetry, no accounts. Bluetooth
  and USB traffic stays between your computer and your own devices.
- **HID (hidraw).** Reads battery from the Keychron Ultra-Link 8K vendor
  interface and from Logitech HID++ interfaces. PeriGauge only sends identity
  queries (ping, feature lookup, firmware/unit id, device name) and battery
  queries. It never sends configuration, pairing or firmware-update commands.
- **Bluetooth.** Registers a client profile with BlueZ for the Samsung Galaxy
  Buds SPP service and only reads the status frames the earbuds push. It does
  not connect devices that are not already connected.
- **D-Bus.** Reads UPower and BlueZ on the system bus; sends notifications on
  the session bus.
- **Files.** Configuration in `~/.config/perigauge/`, state and locks in
  `~/.local/state/perigauge/` (directories `0700`, files `0600`). State
  contains device names, Bluetooth addresses, Logitech unit ids and battery
  levels; do not paste it publicly without removing them.

## The udev rule

`packaging/udev/70-perigauge.rules` adds the `uaccess` tag so the logged-in
local session can open specific hidraw nodes without root:

- Keychron Ultra-Link 8K: vendor interface 4 only.
- Logitech Bolt/Unifying/nano/Lightspeed receivers: HID++ interface 2 only.
- Logitech Bluetooth devices: their single hidraw node, which also carries
  keyboard/mouse input.

`uaccess` grants **read and write** to those nodes for any program running in
your session, not only PeriGauge. Programs could, in principle, read input
from a Logitech Bluetooth keyboard or send HID++ commands (including firmware
update commands) to those devices. Solaar's own rule grants the same access
more broadly (every Logitech hidraw node). Review the rule before installing
it; installation is a separate step that requires `sudo` and is never done by
`install.sh`.

## Reporting

Report problems with a synthetic or redacted `perigauge status --json` and a
minimal description. Use GitHub private vulnerability reporting if it is
enabled on the repository; otherwise open an issue asking for a private
contact without disclosing details.
