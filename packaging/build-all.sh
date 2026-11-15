#!/usr/bin/env bash
# Build every release package into dist/: .deb (Debian, Ubuntu, Mint), .rpm (Fedora,
# openSUSE), an AppImage (any distribution) and a .tar.gz with the plain binary.
# Needs cargo-deb and cargo-generate-rpm; appimagetool (a pinned release, checked
# against its SHA-256) is downloaded unless APPIMAGETOOL points at one.
set -euo pipefail
cd "$(dirname "$0")/.."

version=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
rm -rf dist
mkdir -p dist
cargo build --release
cargo deb --no-build --output dist/
# xz: also readable by the older rpm of openSUSE Leap (the default zstd may not be).
cargo generate-rpm --payload-compress xz --output dist/

# Plain binary with install.sh, which installs it without building.
id=io.github.addition_official.Paintbox
name="paintbox-$version-x86_64"
stage="dist/$name"
mkdir -p "$stage/assets"
cp target/release/paintbox install.sh README.md "$stage/"
cp "assets/$id.desktop" "assets/$id.svg" "assets/$id.png" "assets/$id.metainfo.xml" "$stage/assets/"
tar -C dist -czf "dist/$name.tar.gz" "$name"
rm -rf "$stage"

# AppImage: the binary loads the system's graphics libraries at run time, so
# nothing else needs bundling.
appdir=dist/AppDir
install -Dm755 target/release/paintbox "$appdir/usr/bin/paintbox"
install -Dm644 "assets/$id.desktop" "$appdir/usr/share/applications/$id.desktop"
install -Dm644 "assets/$id.svg" "$appdir/usr/share/icons/hicolor/scalable/apps/$id.svg"
install -Dm644 "assets/$id.png" "$appdir/usr/share/icons/hicolor/256x256/apps/$id.png"
install -Dm644 "assets/$id.metainfo.xml" "$appdir/usr/share/metainfo/$id.appdata.xml"
# A PNG icon: AppImage launchers and file managers show .DirIcon, and many can't
# read SVG.
cp "assets/$id.desktop" "assets/$id.png" "$appdir/"
cp "assets/$id.png" "$appdir/.DirIcon"
ln -s usr/bin/paintbox "$appdir/AppRun"
tool="${APPIMAGETOOL:-}"
if [ -z "$tool" ]; then
    tool=dist/appimagetool
    curl -fsSL -o "$tool" https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage
    echo "ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0  $tool" | sha256sum -c -
    chmod +x "$tool"
fi
ARCH=x86_64 "$tool" --appimage-extract-and-run "$appdir" "dist/$name.AppImage"
rm -rf "$appdir" dist/appimagetool

# Checksums so downloads can be verified: sha256sum -c SHA256SUMS
(cd dist && sha256sum -- *.deb *.rpm *.AppImage *.tar.gz > SHA256SUMS)
ls -l dist
