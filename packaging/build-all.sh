#!/usr/bin/env bash
# Build every release package into dist/: .deb (Debian, Ubuntu, Mint), .rpm (Fedora,
# openSUSE), an AppImage (any distribution) and a .tar.gz with the plain binary.
# Needs cargo-deb and cargo-generate-rpm; appimagetool is downloaded unless
# APPIMAGETOOL points at one.
set -euo pipefail
cd "$(dirname "$0")/.."

version=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
rm -rf dist
mkdir -p dist
cargo build --release
cargo deb --no-build --output dist/
cargo generate-rpm --output dist/

# Plain binary with install.sh, which installs it without building.
name="mspaint-linux-$version-x86_64"
stage="dist/$name"
mkdir -p "$stage/assets"
cp target/release/mspaint-linux install.sh README.md "$stage/"
cp assets/mspaint-linux.desktop assets/mspaint-linux.svg "$stage/assets/"
tar -C dist -czf "dist/$name.tar.gz" "$name"
rm -rf "$stage"

# AppImage: the binary loads the system's graphics libraries at run time, so
# nothing else needs bundling.
appdir=dist/AppDir
install -Dm755 target/release/mspaint-linux "$appdir/usr/bin/mspaint-linux"
install -Dm644 assets/mspaint-linux.desktop "$appdir/usr/share/applications/mspaint-linux.desktop"
install -Dm644 assets/mspaint-linux.svg "$appdir/usr/share/icons/hicolor/scalable/apps/mspaint-linux.svg"
cp assets/mspaint-linux.desktop assets/mspaint-linux.svg "$appdir/"
ln -s usr/bin/mspaint-linux "$appdir/AppRun"
tool="${APPIMAGETOOL:-}"
if [ -z "$tool" ]; then
    tool=dist/appimagetool
    curl -fsSL -o "$tool" https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x "$tool"
fi
ARCH=x86_64 "$tool" --appimage-extract-and-run "$appdir" "dist/$name.AppImage"
rm -rf "$appdir" dist/appimagetool
ls -l dist
