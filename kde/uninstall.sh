#!/usr/bin/env bash
# Remove only files managed by install.sh. It never alters a Plasma panel.
set -euo pipefail

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
package_id="com.opsteam.perifbattery"
helper_target="${HOME}/.local/bin/perif-battery"
kpackagetool6 --type Plasma/Applet --remove "${package_id}" || true
if [[ -f "${helper_target}" ]] && cmp -s "${helper_target}" "${repo_dir}/perif-battery"; then
    rm -- "${helper_target}"
fi
printf 'Removido: %s.\n' "${package_id}"
