//! Desktop integration that differs between Linux desktops: setting the wallpaper and
//! reading the light/dark preference. KDE Plasma, GNOME (and Ubuntu, Budgie, Pantheon),
//! Cinnamon, MATE, Xfce and LXQt are supported.

use std::path::Path;
use std::process::Command;

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

fn run(cmd: &str, args: &[&str]) -> Result<(), String> {
    match Command::new(cmd).args(args).output() {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => Err(String::from_utf8_lossy(&o.stderr).trim().to_string()),
        Err(_) => Err(format!("{cmd} was not found.")),
    }
}

fn output(cmd: &str, args: &[&str]) -> Option<String> {
    let o = Command::new(cmd).args(args).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

pub fn set_wallpaper(path: &Path, fit: Fit) -> Result<(), String> {
    let file = path.to_string_lossy();
    let uri = format!("file://{file}");
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
                run("xfconf-query", &["-c", "xfce4-desktop", "-p", p, "-s", &file])?;
                let style_prop = p.replace("/last-image", "/image-style");
                let _ = run("xfconf-query", &["-c", "xfce4-desktop", "-p", &style_prop, "-s", style]);
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

/// Whether the desktop prefers a dark theme, or None if it can't tell.
pub fn prefers_dark() -> Option<bool> {
    match current() {
        Desktop::Kde => crate::theme::kde_prefers_dark(),
        Desktop::Gnome | Desktop::Cinnamon | Desktop::Mate | Desktop::Xfce | Desktop::Lxqt | Desktop::Other => {
            gnome_prefers_dark().or_else(crate::theme::kde_prefers_dark)
        }
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
    let theme = output("gsettings", &["get", "org.gnome.desktop.interface", "gtk-theme"])?;
    Some(theme.to_ascii_lowercase().contains("dark"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
