#!/usr/bin/env bash
# Build and install Paintbox. In the release .tar.gz, which holds a ready-made
# binary, it installs that binary instead of building.
#
#   ./install.sh              install for the current user (~/.local)
#   ./install.sh --system     install for all users (/usr/local, uses sudo)
#   ./install.sh --uninstall  remove a per-user install (add --system for a system one)
set -euo pipefail

cd "$(dirname "$0")"

PREFIX="$HOME/.local"
SUDO=""
UNINSTALL=0
for arg in "$@"; do
    case "$arg" in
        --system) PREFIX="/usr/local"; SUDO="sudo" ;;
        --uninstall) UNINSTALL=1 ;;
        -h|--help) sed -n '2,7p' "$0"; exit 0 ;;
        *) echo "Unknown option: $arg" >&2; exit 1 ;;
    esac
done

ID=io.github.addition_official.Paintbox
BIN="$PREFIX/bin/paintbox"
DESKTOP="$PREFIX/share/applications/$ID.desktop"
ICON="$PREFIX/share/icons/hicolor/scalable/apps/$ID.svg"
ICON_PNG="$PREFIX/share/icons/hicolor/256x256/apps/$ID.png"
METAINFO="$PREFIX/share/metainfo/$ID.metainfo.xml"

# Paintbox was called mspaint-linux before 0.4.0.
remove_old_name() {
    local old=mspaint-linux
    $SUDO rm -f "$PREFIX/bin/$old" "$PREFIX/share/applications/$old.desktop" \
        "$PREFIX/share/icons/hicolor/scalable/apps/$old.svg" "$PREFIX/share/icons/hicolor/256x256/apps/$old.png" \
        "$PREFIX/share/metainfo/$old.metainfo.xml"
}

refresh_menus() {
    command -v update-desktop-database >/dev/null && $SUDO update-desktop-database "$PREFIX/share/applications" 2>/dev/null || true
    command -v gtk-update-icon-cache >/dev/null && $SUDO gtk-update-icon-cache -q -t "$PREFIX/share/icons/hicolor" 2>/dev/null || true
    # Refresh the KDE Plasma application menu.
    for k in kbuildsycoca6 kbuildsycoca5; do
        command -v "$k" >/dev/null && { "$k" >/dev/null 2>&1 || true; break; }
    done
}

if [ "$UNINSTALL" = 1 ]; then
    $SUDO rm -f "$BIN" "$DESKTOP" "$ICON" "$ICON_PNG" "$METAINFO"
    remove_old_name
    refresh_menus
    echo "Paintbox has been removed from $PREFIX."
    exit 0
fi

if [ -f paintbox ] && [ ! -f Cargo.toml ]; then
    # Unpacked release tarball: the binary is already built.
    SRC=paintbox
else
    if ! command -v cargo >/dev/null; then
        echo "Rust is not installed. Install a C compiler and curl with your package manager" >&2
        echo "(build-essential on Debian/Ubuntu, gcc on Fedora/openSUSE, base-devel on Arch), then:" >&2
        echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" >&2
        exit 1
    fi
    echo "Building (release)..."
    cargo build --release
    SRC=target/release/paintbox
fi

echo "Installing to $PREFIX..."
$SUDO install -Dm755 "$SRC" "$BIN"
$SUDO install -Dm644 "assets/$ID.desktop" "$DESKTOP"
$SUDO install -Dm644 "assets/$ID.svg" "$ICON"
$SUDO install -Dm644 "assets/$ID.png" "$ICON_PNG"
$SUDO install -Dm644 "assets/$ID.metainfo.xml" "$METAINFO"
remove_old_name
refresh_menus

echo
echo "Done. Launch \"Paintbox\" from the application launcher, or run: paintbox [image]"
case ":$PATH:" in
    *":$PREFIX/bin:"*) ;;
    *) echo "Note: $PREFIX/bin is not on your PATH." ;;
esac
