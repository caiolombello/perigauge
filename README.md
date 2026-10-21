<picture>
  <source media="(prefers-color-scheme: dark)" srcset="branding/perigauge-wordmark-dark.svg">
  <img src="branding/perigauge-wordmark-light.svg" alt="PeriGauge" width="320">
</picture>

# PeriGauge — pre-alpha

[Website](https://caiolombello.github.io/perigauge/en/) ·
[Português](https://caiolombello.github.io/perigauge/) ·
[Releases](https://github.com/caiolombello/perigauge/releases)

Battery levels of your wireless mouse, keyboard and earbuds in the Linux
panel, with low-battery notifications. One Rust binary, local only, no Solaar.

**0.1.0-alpha.3.** Battery readings verified on hardware: Keychron M6 through
the Ultra-Link 8K receiver, MX Keys over Bluetooth and Galaxy Buds3 Pro over
Bluetooth (left, right and case), on KDE Plasma 6.6 / Ubuntu 26.04.
Logitech receiver connections and MX Master 3S still await hardware validation.
See [compatibility](docs/COMPATIBILITY.md).

| Device | Link | How |
|---|---|---|
| Keychron M6 | Ultra-Link 8K | vendor HID report |
| Logitech MX Keys | Bluetooth (verified), Unifying (hardware pending) | HID++ 2.0 (0x1004/0x1000/0x1001) |
| Logitech MX Master 3S | Logi Bolt, Bluetooth (hardware pending) | HID++ 2.0 (0x1004/0x1000/0x1001) |
| Samsung Galaxy Buds3 Pro | Bluetooth | Samsung SPP status: left, right, case |
| Anything UPower already knows | kernel `power_supply`, BLE Battery, HFP | UPower |

## Install

Linux x86_64 with systemd and D-Bus; KDE Plasma 6 for the widget. From the
[release](https://github.com/caiolombello/perigauge/releases):

```sh
v=0.1.0-alpha.3
curl -LO https://github.com/caiolombello/perigauge/releases/download/v$v/perigauge-$v-x86_64-linux.tar.gz
curl -LO https://github.com/caiolombello/perigauge/releases/download/v$v/SHA256SUMS
sha256sum -c SHA256SUMS
tar xf perigauge-$v-x86_64-linux.tar.gz && cd perigauge-$v
./install.sh --dry-run
./install.sh --enable-service
perigauge doctor
```

From source (Rust 1.85+): `cargo build --release --locked`, then the same
`./install.sh` steps from the repository root.

`install.sh` writes only to your XDG directories: the binary in
`~/.local/bin`, the user service, a desktop entry (notification identity),
icons and the Plasma widget. It never uses `sudo` and never edits panels; add
the **PeriGauge** widget yourself. HID access without root needs the udev
rule, a separate explicit step — read [SECURITY.md](SECURITY.md) first:

```sh
sudo install -m 644 packaging/udev/70-perigauge.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger --subsystem-match=hidraw
```

Remove with `./uninstall.sh` (`--purge` also deletes settings and state).

## Use

The Plasma widget shows a battery meter for every device on the desktop and
panel. Earbuds have separate left, right and case meters. Choose circular
gauges or bars in the widget menu or settings, or use the switch beside
Refresh in the popup. Each widget saves its own style; the panel and its
popup share the same choice.

Disconnected devices are hidden by default in the panel, popup and desktop
card. Their last reading is kept for reconnection; enable **Show devices that
are out of range** in the widget settings to display it. Upgrades preserve
saved preferences: if an earlier installation still shows disconnected
devices, uncheck that option.

```sh
perigauge status            # table
perigauge status --json     # contract consumed by the widgets
perigauge status --refresh  # read devices now instead of the recent cache
perigauge notify-test
```

The daemon (`systemctl --user status perigauge`) reads every 30 s, keeps the
earbuds' channel open and sends notifications: warning at 20 %, critical at
10 %, about to shut down at 5 %, and charged at 100 %. Each threshold fires
once per discharge and re-arms after the level recovers 5 points or charging
starts. Stale, absent or voltage-estimated readings never notify.

Optional `~/.config/perigauge/config.toml` (reloaded every cycle):

```toml
interval_seconds = 30
language = "auto"          # or "pt", "en"

[notifications]
enabled = true
warning = 20
critical = 10
shutdown = 5
charged = true
hysteresis = 5

[backends]                 # all enabled by default
galaxy_buds = true

[devices."keychron:3434:d028"]
name = "Keychron M6"       # the receiver does not report the model
# hidden = true
# notifications = false
```

Device ids come from `perigauge status --json`.

## Optional passive Orla export (experimental)

PeriGauge owns collection and export permission. Installing PeriGauge or the
JSON package in `contrib/orla/perigauge.orla-addon.json` enables neither export
nor Orla activation. Orla activation is a separate explicit host action.

```sh
perigauge export orla enable
perigauge export orla status
perigauge export orla disable
```

These commands only manage consent and bounded export metadata; they never
collect devices, read the private battery cache, or contact buses/network.
`status` prints `enabled`, `exportPresent` and `exportBytes` (null if absent).
Mutation refuses immediately with `export-busy` if normal collection holds
`collect.lock`; retry once that collection finishes. No process is terminated.
Enable records consent and removes any old export, then waits for the next
normal collection. It never presents cached status as a new observation.
Disable acknowledges success only after consent is off and the export removed.

Consent is a strict, private, separate JSON object at
`$XDG_CONFIG_HOME/perigauge/exports/orla.json`:
`{"formatVersion":"0.1-experimental","enabled":true}`. Missing, malformed,
unknown-key, duplicate-key or unreadable consent is disabled. These commands
never rewrite `config.toml`, including comments and unknown preferences.
An absolute XDG override is used; otherwise HOME/.config and HOME/.local/state
are the configuration/state bases. Export directories are never created while
permission is absent/off. Any old export observed under the collection lock
is removed. Existing producer/export directories must be owned, mode `0700`;
new files are `0600`, and symlink/special-file paths are refused.

The fixed export is
`$XDG_STATE_HOME/perigauge/exports/orla-peripherals-v1.json`. It uses
`formatVersion: "0.1-experimental"`, `producer: "perigauge"`, camelCase keys
and millisecond timestamps. It contains generic kinds/links, presence/staleness,
observation time, battery state and up to three unique left/right/case components;
no device IDs, names, aliases, backend identifiers, errors or issue details.
Devices hidden in either the collected snapshot or freshly parsed current
configuration are omitted. Malformed/unreadable current configuration fails the
export closed rather than exposing devices through default settings.

Publication follows a genuine normal collection and successful private cache
save, under the existing collection lock. Reuse and lock/save/export failures
never renew a lease. Export errors use finite diagnostic codes and do not stop
collection or notifications. Limits are 128 devices and 65,536 serialized bytes;
unsafe/oversized output is refused and the old export removed. Original collection
and observation times are preserved. `expiresAtMs` is `generatedAtMs + 3000 *
intervalSeconds` (10..3600 seconds). Absent/error/missing/future/old observations
are stale. Coarse, zero, estimated, unknown and case-only readings stay distinct.
The consumer must independently age both envelope and device observations,
including while its transport is stalled; a clock rollback makes data unavailable.
Removing a file cannot retract bytes a consumer already read.

Uninstall revokes through the compatible installed binary before removing it,
keeping unrelated preferences. Revocation failure aborts uninstall visibly.
The package/schema/golden examples are experimental contract material, not an
upstream acceptance, native integration or stable SDK guarantee.

## JSON contract

`perigauge status --json` prints schema 1: `devices[]` with `id`, `name`,
`kind`, `link`, `via`, `backend`, `present`, `battery` (`percent` or coarse
`level`, `charging`, `estimated`), optional `components` (earbuds: `left`,
`right`, `case`), `updated_at`, `stale`, `hidden`, `error`; and `issues[]`
for problems the user can fix. Fields are only added within a schema
version.

## Development

```sh
cargo test
cargo clippy --all-targets
cargo fmt --check
shellcheck install.sh uninstall.sh
udevadm verify packaging/udev/70-perigauge.rules
```

Layout: `src/` core and backends, `frontends/plasma/` widget,
`packaging/` systemd/udev/desktop files and the release script, `branding/`
identity, `docs/` landing page (`python3 docs/build.py`) and compatibility.

[MIT](LICENSE) · [NOTICE](NOTICE) · [Changelog](CHANGELOG.md)
