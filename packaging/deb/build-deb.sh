#!/bin/sh
# Builds the .deb (D-145) from target/release/rustbot-creator and THIRD-PARTY-LICENSES.txt.
# Run it from the repository's root on Ubuntu 24.04, so the package also runs on later
# releases (D-146). Prints the .deb's path.
set -eu
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
arch=$(dpkg --print-architecture)
app=io.github.kotoanidev.RustBotCreator
out="$PWD/target/deb"
root="$out/rustbot-creator_${version}_${arch}"
doc="$root/usr/share/doc/rustbot-creator"

rm -rf -- "${root:?}"
install -Dm755 target/release/rustbot-creator "$root/usr/bin/rustbot-creator"
strip --strip-unneeded "$root/usr/bin/rustbot-creator"
install -Dm644 "packaging/linux/$app.desktop" "$root/usr/share/applications/$app.desktop"
install -Dm644 "packaging/linux/$app.metainfo.xml" "$root/usr/share/metainfo/$app.metainfo.xml"
install -Dm644 "packaging/linux/$app.svg" "$root/usr/share/icons/hicolor/scalable/apps/$app.svg"
install -Dm644 LICENSE-MIT "$doc/LICENSE-MIT"
install -Dm644 LICENSE-APACHE "$doc/LICENSE-APACHE"
install -Dm644 THIRD-PARTY-LICENSES.txt "$doc/THIRD-PARTY-LICENSES.txt"
cat > "$doc/copyright" <<COPYRIGHT
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: rustbot-creator
Source: https://github.com/kotoanidev/rustbot-creator

Files: *
Copyright: 2026 The RustBot Creator contributors
License: MIT or Apache-2.0
 The texts are in LICENSE-MIT and LICENSE-APACHE next to this file. The crates built
 into the program, and their licenses, are listed in THIRD-PARTY-LICENSES.txt.
COPYRIGHT

# The libraries the program links, as packages; then those the graphics stack opens at
# run time, which dpkg-shlibdeps cannot see.
shlibs="$out/shlibs"
mkdir -p "$shlibs/debian"
printf 'Source: rustbot-creator\n\nPackage: rustbot-creator\n' > "$shlibs/debian/control"
linked=$(cd "$shlibs" && dpkg-shlibdeps -O "$root/usr/bin/rustbot-creator" | sed 's/^shlibs:Depends=//')
depends="$linked, libvulkan1, libegl1, libwayland-client0, libwayland-egl1, libfontconfig1"

mkdir -p "$root/DEBIAN"
cat > "$root/DEBIAN/control" <<CONTROL
Package: rustbot-creator
Version: $version
Architecture: $arch
Maintainer: kotoanidev <kotoanidev@users.noreply.github.com>
Installed-Size: $(du -sk "$root" | cut -f1)
Depends: $depends
Recommends: gcc, libc6-dev, mesa-vulkan-drivers
Section: devel
Priority: optional
Homepage: https://github.com/kotoanidev/rustbot-creator
Description: Make Discord bots in Rust without writing code
 RustBot Creator builds Discord bots on the djs-rs library. Each slash command
 and event is a graph of nodes, and the app turns it into readable Rust, builds
 it and runs it, exports it, or hosts it on this PC. It is pre-alpha.
CONTROL

dpkg-deb --build --root-owner-group "$root" "$out/" > /dev/null
echo "$out/rustbot-creator_${version}_${arch}.deb"
