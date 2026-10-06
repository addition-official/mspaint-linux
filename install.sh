#!/usr/bin/env bash
# Build and install Paint for Linux.
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

BIN="$PREFIX/bin/mspaint-linux"
DESKTOP="$PREFIX/share/applications/mspaint-linux.desktop"
ICON="$PREFIX/share/icons/hicolor/scalable/apps/mspaint-linux.svg"

refresh_menus() {
    command -v update-desktop-database >/dev/null && $SUDO update-desktop-database "$PREFIX/share/applications" 2>/dev/null || true
    command -v gtk-update-icon-cache >/dev/null && $SUDO gtk-update-icon-cache -q -t "$PREFIX/share/icons/hicolor" 2>/dev/null || true
    # Refresh the KDE Plasma application menu.
    for k in kbuildsycoca6 kbuildsycoca5; do
        command -v "$k" >/dev/null && { "$k" >/dev/null 2>&1 || true; break; }
    done
}

if [ "$UNINSTALL" = 1 ]; then
    $SUDO rm -f "$BIN" "$DESKTOP" "$ICON"
    refresh_menus
    echo "Paint has been removed from $PREFIX."
    exit 0
fi

if ! command -v cargo >/dev/null; then
    echo "Rust is not installed. Install it with:" >&2
    echo "  sudo apt install build-essential curl" >&2
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" >&2
    exit 1
fi

echo "Building (release)..."
cargo build --release

echo "Installing to $PREFIX..."
$SUDO install -Dm755 target/release/mspaint-linux "$BIN"
$SUDO install -Dm644 assets/mspaint-linux.desktop "$DESKTOP"
$SUDO install -Dm644 assets/mspaint-linux.svg "$ICON"
refresh_menus

echo
echo "Done. Launch \"Paint\" from the application launcher, or run: mspaint-linux [image]"
case ":$PATH:" in
    *":$PREFIX/bin:"*) ;;
    *) echo "Note: $PREFIX/bin is not on your PATH." ;;
esac
