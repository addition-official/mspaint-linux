# Paint for Linux

A clone of **Windows 11 Paint** for Linux, built for **Kubuntu / KDE Plasma**. It's written in Rust with
[egui](https://github.com/emilk/egui) and compiles to a single binary.

It recreates the Windows 11 Paint layout: the Windows 11 style title bar, the File / View row with quick
Save, Undo and Redo buttons, the toolbar (Clipboard, Image, Tools, Brushes, Shapes, Size, Colours), the
centred canvas with resize handles, and the status bar with a zoom slider. Light and dark themes are
included.

## Features

| Area | What's there |
| --- | --- |
| **Tools** | Pencil, Fill with colour, Text, Eraser, Colour picker, Magnifier |
| **Brushes** | Brush, Calligraphy 1 and 2, Airbrush, Oil, Crayon, Marker, Natural pencil, Watercolour |
| **Shapes** | Line, Curve (two bends), Oval, Rectangle, Rounded rectangle, Polygon, Triangle, Right triangle, Diamond, Pentagon, Hexagon, 4 arrows, 4-, 5- and 6-point stars, 3 callouts, Heart, Lightning |
| **Shape styles** | Outline and fill: none, solid, crayon, marker, oil, natural pencil, watercolour. Shapes stay editable (move or resize with handles) until you click away. Hold Shift for squares, circles and 45° lines. |
| **Selection** | Rectangle and free-form, select all, invert selection, delete, transparent selection, move, resize with handles, nudge with the arrow keys |
| **Image** | Crop to selection, Resize and skew (percent or pixels), Rotate 90°/180°, Flip, Invert colours, drag-to-resize canvas |
| **Text** | Any installed font, size, bold, italic, underline, strikethrough, left/centre/right alignment, background fill, word wrap |
| **Colours** | Colour 1 / Colour 2 (right-click draws with Colour 2), the classic 20-colour palette, 10 custom colours, Edit colours (picker + hex) |
| **View** | Zoom 10% to 800% (Ctrl+wheel, slider, magnifier), fit to window, rulers, gridlines, status bar, full screen |
| **Files** | Open / Save / Save as PNG, JPEG, BMP, GIF, TIFF, WEBP and ICO. Also opens TGA, PNM, QOI, HDR and more. Recent files, Import to canvas, drag and drop, open from the command line |
| **Clipboard** | Paste, Paste from file, Cut and Copy, shared with other apps (Wayland and X11). Pasting a larger image grows the canvas, as in Paint |
| **History** | Undo / redo (up to 100 steps) |

### KDE / Kubuntu integration

- **Native KDE file dialogs** through the XDG desktop portal (`xdg-desktop-portal-kde`).
- **Set as desktop background** (Fill, Tile, Centre) through Plasma's `plasma-apply-wallpaperimage`.
- Uses the desktop font (**Noto Sans** on Kubuntu) and follows the Plasma light/dark colour scheme.
- Draws a Windows 11 style title bar by default (drag to move, double-click to maximise, resize from
  the window edges). If you prefer Plasma's own title bar, turn on **Settings (gear) → Use system title bar**.
- Native Wayland and X11 support, with a `.desktop` entry so Paint shows up in the launcher and under "Open with" for images.
- Opens files in your XDG Pictures folder by default.
- Asks to save unsaved work when you close the window.

## Install on Kubuntu

### 1. Install Rust (one time)

Kubuntu's packaged `rustc` is too old, so use rustup:

```bash
sudo apt install build-essential curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### 2. Build and install

```bash
git clone https://github.com/addition-official/mspaint-linux.git
cd mspaint-linux
./install.sh            # installs to ~/.local for the current user
# ./install.sh --system # or for all users, into /usr/local
```

Then open **Paint** from the application launcher, or run `mspaint-linux [image]`.
To remove it later: `./install.sh --uninstall`.

### Or build a .deb

```bash
cargo install cargo-deb
cargo deb
sudo apt install ./target/debian/mspaint-linux_*.deb
```

### Just run it

```bash
cargo run --release -- path/to/picture.png
```

The binary only links against libc. It loads OpenGL, Wayland/X11 and xkbcommon at runtime, and a
standard Kubuntu install already has them.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| Ctrl+N / Ctrl+O / Ctrl+S | New / Open / Save |
| Ctrl+Shift+S or F12 | Save as |
| Ctrl+Z / Ctrl+Y | Undo / Redo |
| Ctrl+X / Ctrl+C / Ctrl+V | Cut / Copy / Paste |
| Ctrl+A | Select all |
| Delete | Delete selection |
| Ctrl+Shift+X | Crop to selection |
| Ctrl+W | Resize and skew |
| Ctrl+E | Image properties (canvas size) |
| Ctrl+Shift+I | Invert colours |
| Ctrl++ / Ctrl+- / Ctrl+wheel | Zoom in / out |
| Ctrl+1 / Ctrl+0 | Zoom to 100% / Fit to window |
| Ctrl+R / Ctrl+G | Rulers / Gridlines |
| F11 | Full screen |
| Middle-drag | Pan the canvas |
| Shift (while drawing shapes) | Squares, circles, 45° lines |
| Enter / Esc | Finish the current shape, text or selection |

## Development

```bash
cargo run          # debug build (dependencies are still optimized)
cargo test         # unit tests for the document model, shapes, text and file formats
```

Source layout:

| File | Purpose |
| --- | --- |
| `src/app.rs` | App state, menus, shortcuts, files, dialogs, status bar |
| `src/ribbon.rs` | The Windows 11 style toolbar |
| `src/canvas.rs` | Canvas view, zoom/scroll, rulers, gridlines, mouse interaction |
| `src/document.rs` | Image, stroke layer, undo history, selection, pending shapes and text |
| `src/brush.rs` | Pencil, eraser and the artistic brushes |
| `src/shapes.rs` | Shape geometry and rasterization |
| `src/text.rs` | System font discovery and text layout and rendering |
| `src/pixels.rs` | Pixel operations, transforms and file I/O |
| `src/icons.rs` | Vector toolbar icons |
| `src/theme.rs` | Fluent-style light and dark themes |

## Not included (yet)

Windows 11 Paint's cloud features (Image Creator, Cocreator, generative erase), layers, background
removal and printing.

## Disclaimer

This is an independent project that recreates the look and workflow of Windows 11 Paint. It is not
affiliated with or endorsed by Microsoft, and it uses no Microsoft code or assets. All icons are
drawn in code.
