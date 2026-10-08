#!/usr/bin/env bash
# Remove what install.sh installed. Configuration and state are kept unless
# --purge is given. Never uses sudo and never edits Plasma panels.
set -euo pipefail

dry_run=0
purge=0
for arg in "$@"; do
    case "${arg}" in
        --dry-run) dry_run=1 ;;
        --purge) purge=1 ;;
        -h | --help) printf 'Usage: ./uninstall.sh [--dry-run] [--purge]\n'; exit 0 ;;
        *) printf 'uninstall.sh: unknown option: %s\n' "${arg}" >&2; exit 2 ;;
    esac
done

config_home="${XDG_CONFIG_HOME:-${HOME}/.config}"
data_home="${XDG_DATA_HOME:-${HOME}/.local/share}"
state_home="${XDG_STATE_HOME:-${HOME}/.local/state}"
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

if command -v systemctl >/dev/null && systemctl --user cat perigauge.service >/dev/null 2>&1; then
    run systemctl --user disable --now perigauge.service || true
fi
if command -v kpackagetool6 >/dev/null && kpackagetool6 --type Plasma/Applet --list 2>/dev/null | grep -Fqx "${plasmoid_id}"; then
    run kpackagetool6 --type Plasma/Applet --remove "${plasmoid_id}"
fi
for path in \
    "${HOME}/.local/bin/perigauge" \
    "${config_home}/systemd/user/perigauge.service" \
    "${data_home}/applications/perigauge.desktop" \
    "${data_home}/icons/hicolor/scalable/apps/perigauge.svg" \
    "${data_home}/icons/hicolor/symbolic/apps/perigauge-symbolic.svg"; do
    if [[ -e "${path}" ]]; then
        run rm -- "${path}"
    fi
done
if command -v systemctl >/dev/null; then
    run systemctl --user daemon-reload
fi
if (( purge )); then
    for dir in "${config_home}/perigauge" "${state_home}/perigauge"; do
        if [[ -d "${dir}" ]]; then
            run rm -r -- "${dir}"
        fi
    done
fi

if (( dry_run )); then
    printf '\nDry run: nothing was changed.\n'
    exit 0
fi
printf '\nPeriGauge removed.%s\n' "$( (( purge )) || printf ' Settings kept in %s/perigauge.' "${config_home}")"
if [[ -e /etc/udev/rules.d/70-perigauge.rules ]]; then
    printf 'The udev rule stays installed; remove it with:\n  sudo rm /etc/udev/rules.d/70-perigauge.rules && sudo udevadm control --reload\n'
fi
