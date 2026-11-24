#!/bin/sh
# Writes the winget manifests of a release (D-152) for a pull request to
# microsoft/winget-pkgs, which the owner opens (docs/RELEASING.md):
#   make-manifests.sh <version> <installer URL> <installer SHA-256> <out folder>
set -eu
version=$1
url=$2
sha256=$(printf '%s' "$3" | tr 'a-f' 'A-F')
out="$4/manifests/k/kotoanidev/RustBotCreator/$version"
id=kotoanidev.RustBotCreator
mkdir -p "$out"

cat > "$out/$id.yaml" <<EOF
PackageIdentifier: $id
PackageVersion: $version
DefaultLocale: en-US
ManifestType: version
ManifestVersion: 1.9.0
EOF

cat > "$out/$id.installer.yaml" <<EOF
PackageIdentifier: $id
PackageVersion: $version
InstallerType: inno
Scope: user
InstallModes:
  - interactive
  - silent
UpgradeBehavior: install
Installers:
  - Architecture: x64
    InstallerUrl: $url
    InstallerSha256: $sha256
ManifestType: installer
ManifestVersion: 1.9.0
EOF

cat > "$out/$id.locale.en-US.yaml" <<EOF
PackageIdentifier: $id
PackageVersion: $version
PackageLocale: en-US
Publisher: kotoanidev
PublisherUrl: https://github.com/kotoanidev
PackageName: RustBot Creator
PackageUrl: https://github.com/kotoanidev/rustbot-creator
License: MIT OR Apache-2.0
LicenseUrl: https://github.com/kotoanidev/rustbot-creator#license
ShortDescription: Make Discord bots in Rust without writing code.
Description: RustBot Creator builds Discord bots on the djs-rs library from node graphs. It turns them into readable Rust, builds, runs, exports and hosts them. Pre-alpha.
Tags:
  - discord
  - bot
  - rust
  - no-code
ReleaseNotesUrl: https://github.com/kotoanidev/rustbot-creator/releases/tag/v$version
ManifestType: defaultLocale
ManifestVersion: 1.9.0
EOF

echo "$out"
