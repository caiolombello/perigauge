#!/usr/bin/env bash
# Install only into the current user's XDG locations. It does not alter panels.
set -euo pipefail

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
helper_target="${HOME}/.local/bin/perif-battery"
package_id="com.opsteam.perifbattery"

install -d -m 700 "${HOME}/.local/bin"
install -m 700 "${repo_dir}/perif-battery" "${helper_target}"
if kpackagetool6 --type Plasma/Applet --list | grep -Fq "${package_id}"; then
    kpackagetool6 --type Plasma/Applet --upgrade "${repo_dir}/package"
else
    kpackagetool6 --type Plasma/Applet --install "${repo_dir}/package"
fi
printf 'Instalado: %s (nenhum painel foi alterado).\n' "${package_id}"
