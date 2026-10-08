#!/usr/bin/env bash
# Install PeriGauge for the current user only (XDG locations). Never uses sudo
# and never edits Plasma panels; the udev rule is a separate, explicit step.
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: ./install.sh [--dry-run] [--enable-service]

  --dry-run         print what would be done, change nothing
  --enable-service  enable and start the user service (notifications)
EOF
}

dry_run=0
enable_service=0
for arg in "$@"; do
    case "${arg}" in
        --dry-run) dry_run=1 ;;
        --enable-service) enable_service=1 ;;
        -h | --help) usage; exit 0 ;;
        *) printf 'install.sh: unknown option: %s\n' "${arg}" >&2; usage >&2; exit 2 ;;
    esac
done

repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
bin_dir="${HOME}/.local/bin"
config_home="${XDG_CONFIG_HOME:-${HOME}/.config}"
data_home="${XDG_DATA_HOME:-${HOME}/.local/share}"
binary="${repo}/target/release/perigauge"
plasmoid="${repo}/frontends/plasma/package"
plasmoid_id="io.github.caiolombello.perigauge"

run() {
    if (( dry_run )); then
        printf '+'
        printf ' %q' "$@"
        printf '\n'
    else
        "$@"
    fi
}

if [[ ! -x "${binary}" ]]; then
    if ! command -v cargo >/dev/null; then
        printf 'install.sh: %s not built and cargo is not available\n' "${binary}" >&2
        exit 1
    fi
    run cargo build --release --locked --manifest-path "${repo}/Cargo.toml"
fi

run install -d -m 700 "${bin_dir}"
run install -m 755 "${binary}" "${bin_dir}/perigauge"
run install -D -m 644 "${repo}/packaging/systemd/perigauge.service" "${config_home}/systemd/user/perigauge.service"
run install -D -m 644 "${repo}/packaging/perigauge.desktop" "${data_home}/applications/perigauge.desktop"
run install -D -m 644 "${repo}/branding/perigauge.svg" "${data_home}/icons/hicolor/scalable/apps/perigauge.svg"
run install -D -m 644 "${repo}/branding/perigauge-symbolic.svg" "${data_home}/icons/hicolor/symbolic/apps/perigauge-symbolic.svg"

if [[ -d "${plasmoid}" ]] && command -v kpackagetool6 >/dev/null; then
    if kpackagetool6 --type Plasma/Applet --list 2>/dev/null | grep -Fqx "${plasmoid_id}"; then
        run kpackagetool6 --type Plasma/Applet --upgrade "${plasmoid}"
    else
        run kpackagetool6 --type Plasma/Applet --install "${plasmoid}"
    fi
fi

if command -v systemctl >/dev/null; then
    run systemctl --user daemon-reload
    if (( enable_service )); then
        run systemctl --user enable --now perigauge.service
    fi
fi

if (( dry_run )); then
    printf '\nDry run: nothing was changed.\n'
    exit 0
fi

cat <<EOF

PeriGauge installed for ${USER:-this user}. No panel was changed.
Next steps:
  - Check access:        perigauge doctor
  - Notifications:       systemctl --user enable --now perigauge.service
  - Widget:              add "PeriGauge" in the panel edit mode
  - HID access (sudo, review SECURITY.md first):
      sudo install -m 644 ${repo}/packaging/udev/70-perigauge.rules /etc/udev/rules.d/
      sudo udevadm control --reload && sudo udevadm trigger --subsystem-match=hidraw
EOF
