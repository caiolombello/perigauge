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
