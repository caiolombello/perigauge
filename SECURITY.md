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

## Optional passive export

The default is off. A producer-operated `export orla enable` records separate
JSON consent; normal collection alone may publish the fixed minimal export.
Orla never runs producer commands, requests credentials or rewrites its config.
The exported projection omits stable IDs, device names/aliases, backend details,
raw errors and issues, and filters hidden devices again immediately before
publication. A malformed/unreadable main configuration fails export closed.
Original collection/observation times and a three-interval lease are preserved;
new consent never refreshes old hardware readings. Consumers must age readings
independently and discard expired data or data from a future/rolled-back clock.

Only the new consent/export paths use descriptor-anchored nofollow operations,
owned `0700` directories, `0600` files, bounded reads, exclusive temporary files,
atomic same-directory replacement, and file/directory fsync. Symlinks, hardlinked
files and special files are refused without reading their content. This does
not harden the existing private cache writer, create a sandbox or protect against
a malicious program running as the same user. The export is an explicit local
privacy reduction, not a publisher authentication or hardware accuracy claim.

Enable/disable share normal collection's advisory lock and refuse busy immediately.
Disable is acknowledged only after permission-off and export removal; subsequent
collections cannot publish until fresh consent. A consent file edited manually
to off/invalid/absent is observed on the next locked collection (including cache
reuse), whereas the disable CLI provides immediate serialized acknowledgement.
Uninstall uses that revoke operation and preserves unrelated preferences.
Already-read bytes cannot be revoked, so consumers must enforce the lease.

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
