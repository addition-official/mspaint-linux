//! Desktop integration that differs between Linux desktops: setting the wallpaper and
//! reading the light/dark preference. KDE Plasma, GNOME (and Ubuntu, Budgie, Pantheon),
//! Cinnamon, MATE, Xfce and LXQt are supported.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, OnceLock};
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Desktop {
    Kde,
    Gnome,
    Cinnamon,
    Mate,
    Xfce,
    Lxqt,
    Other,
}

/// The running desktop, from `XDG_CURRENT_DESKTOP` (e.g. "KDE", "ubuntu:GNOME").
pub fn current() -> Desktop {
    detect(&std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default())
}

fn detect(value: &str) -> Desktop {
    for part in value.split(':').map(|p| p.trim().to_ascii_lowercase()) {
        match part.as_str() {
            "kde" => return Desktop::Kde,
            "gnome" | "unity" | "budgie" | "pantheon" | "gnome-classic" => return Desktop::Gnome,
            "x-cinnamon" | "cinnamon" => return Desktop::Cinnamon,
            "mate" => return Desktop::Mate,
            "xfce" => return Desktop::Xfce,
            "lxqt" => return Desktop::Lxqt,
            _ => {}
        }
    }
    Desktop::Other
}

/// How the picture fills the screen (File > Set as desktop background).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    Fill,
    Tile,
    Center,
}

/// Run a desktop tool and wait for it, at most a few seconds: a tool that stays
/// running (e.g. a file manager starting its desktop) must not freeze Paint.
fn run(cmd: &str, args: &[&str]) -> Result<(), String> {
    use std::io::Read;
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| format!("{cmd} was not found."))?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(_)) => {
                let mut err = String::new();
                if let Some(mut s) = child.stderr.take() {
                    let _ = s.read_to_string(&mut err);
                }
                return Err(if err.trim().is_empty() { format!("{cmd} failed.") } else { err.trim().to_string() });
            }
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            Ok(None) => {
                // Still running: it has most likely done its job and kept going.
                return Ok(());
            }
            Err(e) => return Err(e.to_string()),
        }
    }
}

fn output(cmd: &str, args: &[&str]) -> Option<String> {
    let o = Command::new(cmd).args(args).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// A file:// URI with the path percent-encoded (GNOME and Cinnamon cut a plain path at
/// '#' and decode '%').
pub fn file_uri(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut uri = String::from("file://");
    for &b in path.as_os_str().as_bytes() {
        if b.is_ascii_alphanumeric() || b"/-._~".contains(&b) {
            uri.push(b as char);
        } else {
            uri.push_str(&format!("%{b:02X}"));
        }
    }
    uri
}

/// The local path in a file:// URI (as file managers copy them), percent-decoded.
/// Plain absolute paths are accepted too.
pub fn path_from_uri(s: &str) -> Option<std::path::PathBuf> {
    use std::os::unix::ffi::OsStringExt;
    let s = s.trim();
    let rest = match s.strip_prefix("file://") {
        Some(r) => r.strip_prefix("localhost").unwrap_or(r),
        None if s.starts_with('/') => return Some(s.into()),
        None => return None,
    };
    if !rest.starts_with('/') {
        return None; // another host
    }
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(v) = rest.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(v);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    Some(std::ffi::OsString::from_vec(out).into())
}

/// Whether windows can have transparent (rounded) corners: always on Wayland, and on
/// X11 only while a compositor runs. Without one the corners would show as black.
pub fn has_compositor() -> bool {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        return true;
    }
    x11_compositor_running().unwrap_or(true)
}

fn x11_compositor_running() -> Option<bool> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::ConnectionExt;
    let (conn, screen) = x11rb::connect(None).ok()?;
    let name = format!("_NET_WM_CM_S{screen}");
    let atom = conn.intern_atom(false, name.as_bytes()).ok()?.reply().ok()?.atom;
    let owner = conn.get_selection_owner(atom).ok()?.reply().ok()?.owner;
    let _ = conn.flush();
    Some(owner != x11rb::NONE)
}

/// Whether a file dialog can be shown: the XDG portal's FileChooser, or zenity (the two
/// things the dialog library uses). Minimal window managers sometimes have neither, and
/// then Open and Save would silently do nothing. Checked once.
pub fn file_dialog_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        let portal = ["org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop", "org.freedesktop.portal.FileChooser"];
        let busctl = Command::new("busctl")
            .args(["--user", "get-property", portal[0], portal[1], portal[2], "version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let in_path = |name: &str| {
            std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(name).is_file()))
        };
        match busctl {
            Ok(status) => status.success() || in_path("zenity"),
            // Without busctl we can't tell: assume it works rather than block Open and Save.
            Err(_) => true,
        }
    })
}

/// Whether Paint knows how to set the wallpaper on this desktop.
pub fn can_set_wallpaper() -> bool {
    current() != Desktop::Other
}

pub fn set_wallpaper(path: &Path, fit: Fit) -> Result<(), String> {
    let file = path.to_string_lossy();
    let uri = file_uri(path);
    match current() {
        Desktop::Kde => {
            let mode = match fit {
                Fit::Tile => "tile",
                Fit::Center => "pad",
                Fit::Fill => "preserveAspectCrop",
            };
            // Older Plasma has no --fill-mode.
            run("plasma-apply-wallpaperimage", &["--fill-mode", mode, &file]).or_else(|_| run("plasma-apply-wallpaperimage", &[&file]))
        }
        Desktop::Gnome => {
            let opt = match fit {
                Fit::Tile => "wallpaper",
                Fit::Center => "centered",
                Fit::Fill => "zoom",
            };
            let schema = "org.gnome.desktop.background";
            run("gsettings", &["set", schema, "picture-uri", &uri])?;
            // GNOME 42+ keeps a separate picture for dark mode; older versions lack the key.
            let _ = run("gsettings", &["set", schema, "picture-uri-dark", &uri]);
            run("gsettings", &["set", schema, "picture-options", opt])
        }
        Desktop::Cinnamon => {
            let opt = match fit {
                Fit::Tile => "wallpaper",
                Fit::Center => "centered",
                Fit::Fill => "zoom",
            };
            run("gsettings", &["set", "org.cinnamon.desktop.background", "picture-uri", &uri])?;
            run("gsettings", &["set", "org.cinnamon.desktop.background", "picture-options", opt])
        }
        Desktop::Mate => {
            let opt = match fit {
                Fit::Tile => "wallpaper",
                Fit::Center => "centered",
                Fit::Fill => "zoom",
            };
            run("gsettings", &["set", "org.mate.background", "picture-filename", &file])?;
            run("gsettings", &["set", "org.mate.background", "picture-options", opt])
        }
        Desktop::Xfce => {
            // Every monitor and workspace has its own last-image property.
            let props = output("xfconf-query", &["-c", "xfce4-desktop", "-l"]).ok_or("xfconf-query was not found.")?;
            let style = match fit {
                Fit::Tile => "2",
                Fit::Center => "1",
                Fit::Fill => "5",
            };
            let mut any = false;
            for p in props.lines().filter(|p| p.ends_with("/last-image")) {
                // -n -t creates the property if it doesn't exist yet.
                run("xfconf-query", &["-c", "xfce4-desktop", "-p", p, "-n", "-t", "string", "-s", &file])?;
                let style_prop = p.replace("/last-image", "/image-style");
                let _ = run("xfconf-query", &["-c", "xfce4-desktop", "-p", &style_prop, "-n", "-t", "int", "-s", style]);
                any = true;
            }
            if any { Ok(()) } else { Err("No Xfce desktop backgrounds were found to change.".into()) }
        }
        Desktop::Lxqt => {
            let mode = match fit {
                Fit::Tile => "tile",
                Fit::Center => "center",
                Fit::Fill => "zoom",
            };
            run("pcmanfm-qt", &[&format!("--set-wallpaper={file}"), &format!("--wallpaper-mode={mode}")])
        }
        Desktop::Other => Err(
            "Paint can set the desktop background on KDE Plasma, GNOME, Cinnamon, MATE, Xfce and LXQt. \
             On this desktop, set it in your desktop's own settings."
                .into(),
        ),
    }
}

/// Whether the desktop prefers a dark theme, or None if it can't tell. Runs desktop
/// tools, so call it off the UI thread (see `watch_dark`).
pub fn prefers_dark() -> Option<bool> {
    let desktop = current();
    if desktop == Desktop::Kde {
        return crate::theme::kde_prefers_dark();
    }
    // The XDG portal's appearance setting is the common answer on modern desktops.
    portal_prefers_dark().or_else(|| match desktop {
        Desktop::Mate => theme_is_dark(output("gsettings", &["get", "org.mate.interface", "gtk-theme"])),
        Desktop::Cinnamon => theme_is_dark(output("gsettings", &["get", "org.cinnamon.desktop.interface", "gtk-theme"])),
        Desktop::Xfce => theme_is_dark(output("xfconf-query", &["-c", "xsettings", "-p", "/Net/ThemeName"])),
        Desktop::Lxqt => {
            let home = std::env::var_os("HOME")?;
            let conf = std::fs::read_to_string(Path::new(&home).join(".config/lxqt/lxqt.conf")).ok()?;
            theme_is_dark(conf.lines().find(|l| l.starts_with("theme=")).map(str::to_string))
        }
        Desktop::Gnome | Desktop::Other => gnome_prefers_dark(),
        Desktop::Kde => None,
    })
}

fn theme_is_dark(name: Option<String>) -> Option<bool> {
    name.map(|n| n.to_ascii_lowercase().contains("dark"))
}

/// org.freedesktop.appearance color-scheme: 1 = dark, 2 = light, 0 = no preference.
fn portal_prefers_dark() -> Option<bool> {
    let args = ["org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop", "org.freedesktop.portal.Settings"];
    let value = output("busctl", &["--user", "call", args[0], args[1], args[2], "ReadOne", "ss", "org.freedesktop.appearance", "color-scheme"])
        .or_else(|| {
            output(
                "gdbus",
                &["call", "--session", "--dest", args[0], "--object-path", args[1], "--method", "org.freedesktop.portal.Settings.ReadOne", "org.freedesktop.appearance", "color-scheme"],
            )
        })?;
    let n: u32 = value.chars().filter(char::is_ascii_digit).collect::<String>().parse().ok()?;
    match n {
        1 => Some(true),
        2 => Some(false),
        _ => None,
    }
}

/// GNOME's color scheme (GNOME 42+), else a theme name containing "dark".
fn gnome_prefers_dark() -> Option<bool> {
    if let Some(scheme) = output("gsettings", &["get", "org.gnome.desktop.interface", "color-scheme"]) {
        if scheme.contains("prefer-dark") {
            return Some(true);
        }
        if scheme.contains("prefer-light") {
            return Some(false);
        }
    }
    theme_is_dark(output("gsettings", &["get", "org.gnome.desktop.interface", "gtk-theme"]))
}

/// Follow the desktop's light/dark preference on a background thread: checking runs
/// desktop tools, which must not stall frames. The value is 0 (unknown), 1 (light) or
/// 2 (dark); the UI is repainted when it changes.
pub fn watch_dark(ctx: egui::Context) -> Arc<AtomicU8> {
    let value = Arc::new(AtomicU8::new(0));
    let shared = value.clone();
    std::thread::Builder::new()
        .name("theme-watch".into())
        .spawn(move || {
            loop {
                let v = match prefers_dark() {
                    Some(true) => 2,
                    Some(false) => 1,
                    None => 0,
                };
                if shared.swap(v, Ordering::Relaxed) != v {
                    ctx.request_repaint();
                }
                std::thread::sleep(Duration::from_secs(3));
            }
        })
        .ok();
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_file_uris() {
        assert_eq!(file_uri(Path::new("/home/a b/#1 50%.png")), "file:///home/a%20b/%231%2050%25.png");
        assert_eq!(file_uri(Path::new("/tmp/é.png")), "file:///tmp/%C3%A9.png");
    }

    #[test]
    fn decodes_file_uris() {
        assert_eq!(path_from_uri("file:///home/a%20b/%231.png"), Some("/home/a b/#1.png".into()));
        assert_eq!(path_from_uri("file://localhost/tmp/x.png"), Some("/tmp/x.png".into()));
        assert_eq!(path_from_uri("/tmp/y.png"), Some("/tmp/y.png".into()));
        assert_eq!(path_from_uri("file://server/x.png"), None);
        assert_eq!(path_from_uri("hello"), None);
    }

    #[test]
    fn detects_desktops() {
        assert_eq!(detect("KDE"), Desktop::Kde);
        assert_eq!(detect("ubuntu:GNOME"), Desktop::Gnome);
        assert_eq!(detect("X-Cinnamon"), Desktop::Cinnamon);
        assert_eq!(detect("XFCE"), Desktop::Xfce);
        assert_eq!(detect("sway"), Desktop::Other);
        assert_eq!(detect(""), Desktop::Other);
    }
}
