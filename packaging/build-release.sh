#!/usr/bin/env bash
# Build the release tarball (prebuilt binary + installer files) and SHA256SUMS
# into dist/. The tarball unpacks to perigauge-<version>/ and its install.sh
# finds target/release/perigauge, so no Rust toolchain is needed to install.
set -euo pipefail

repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "${repo}/Cargo.toml" | head -n 1)
arch=$(uname -m)
name="perigauge-${version}"
dist="${repo}/dist"
stage="${dist}/${name}"

cargo build --release --locked --manifest-path "${repo}/Cargo.toml"
"${repo}/target/release/perigauge" --version

rm -rf -- "${stage}"
install -d "${stage}/target/release" "${stage}/branding" "${stage}/frontends"
install -m 755 "${repo}/target/release/perigauge" "${stage}/target/release/perigauge"
install -m 755 "${repo}/install.sh" "${repo}/uninstall.sh" "${stage}/"
install -m 644 "${repo}/README.md" "${repo}/LICENSE" "${repo}/NOTICE" "${repo}/SECURITY.md" "${repo}/CHANGELOG.md" "${stage}/"
install -m 644 "${repo}/branding/perigauge.svg" "${repo}/branding/perigauge-symbolic.svg" "${stage}/branding/"
cp -R "${repo}/packaging" "${stage}/packaging"
cp -R "${repo}/frontends/plasma" "${stage}/frontends/plasma"
rm -f -- "${stage}/packaging/build-release.sh"

tarball="${name}-${arch}-linux.tar.gz"
tar -C "${dist}" --owner=0 --group=0 --numeric-owner -czf "${dist}/${tarball}" "${name}"
rm -rf -- "${stage}"
(cd "${dist}" && sha256sum "${tarball}" > SHA256SUMS)
cat "${dist}/SHA256SUMS"
