//! The XDG desktop portal: the desktop's own service for things a sandboxed app can't
//! do itself. Paintbox uses it for the light/dark setting everywhere, and inside
//! Flatpak (where desktop tools like `lp` or `gsettings` aren't there) for the
//! wallpaper, printing and keeping a copy on the clipboard after closing.

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;
use zbus::blocking::Connection;
use zbus::zvariant::{Fd, OwnedValue, Value};

const DEST: &str = "org.freedesktop.portal.Desktop";
const PATH: &str = "/org/freedesktop/portal/desktop";

/// Whether Paintbox runs inside a Flatpak sandbox.
pub fn in_flatpak() -> bool {
    static IN: OnceLock<bool> = OnceLock::new();
    *IN.get_or_init(|| Path::new("/.flatpak-info").exists())
}

fn bus() -> Option<&'static Connection> {
    static BUS: OnceLock<Option<Connection>> = OnceLock::new();
    BUS.get_or_init(|| Connection::session().ok()).as_ref()
}

/// The desktop's light/dark preference (org.freedesktop.appearance color-scheme:
/// 1 = dark, 2 = light, 0 = no preference).
pub fn prefers_dark() -> Option<bool> {
    let bus = bus()?;
    let key = ("org.freedesktop.appearance", "color-scheme");
    let iface = Some("org.freedesktop.portal.Settings");
    // ReadOne is newer; older portals only have Read, which wraps the value once more.
    let reply = bus.call_method(Some(DEST), PATH, iface, "ReadOne", &key).or_else(|_| bus.call_method(Some(DEST), PATH, iface, "Read", &key)).ok()?;
    let value: OwnedValue = reply.body().deserialize().ok()?;
    let n = match &*value {
        Value::U32(n) => *n,
        Value::Value(inner) => match **inner {
            Value::U32(n) => n,
            _ => return None,
        },
        _ => return None,
    };
    match n {
        1 => Some(true),
        2 => Some(false),
        _ => None,
    }
}

/// Whether the portal offers `interface` (it answers for its version).
fn has(interface: &str) -> bool {
    let Some(bus) = bus() else { return false };
    bus.call_method(Some(DEST), PATH, Some("org.freedesktop.DBus.Properties"), "Get", &(interface, "version")).is_ok()
}

/// Whether the portal offers file dialogs.
pub fn has_file_chooser() -> bool {
    has("org.freedesktop.portal.FileChooser")
}

/// Whether the desktop's own print dialog is available.
pub fn has_print() -> bool {
    has("org.freedesktop.portal.Print")
}

/// Ask the desktop to use the picture at `path` as the wallpaper. The desktop may ask
/// the user to confirm; this doesn't wait for that.
pub fn set_wallpaper(path: &Path) -> Result<(), String> {
    let bus = bus().ok_or("Could not reach the desktop (no D-Bus session).")?;
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut options: HashMap<&str, Value> = HashMap::new();
    options.insert("show-preview", false.into());
    options.insert("set-on", "both".into());
    bus.call_method(Some(DEST), PATH, Some("org.freedesktop.portal.Wallpaper"), "SetWallpaperFile", &("", Fd::from(&file), options))
        .map(|_| ())
        .map_err(|e| format!("The desktop could not set the wallpaper.\n\n{e}"))
}

/// Print `pdf` through the desktop's print dialog (which picks the printer, copies
/// and so on).
pub fn print(pdf: &[u8], title: &str) -> Result<(), String> {
    let bus = bus().ok_or("Could not reach the desktop (no D-Bus session).")?;
    let file = anonymous_file(pdf).map_err(|e| format!("Could not prepare the picture for printing.\n\n{e}"))?;
    let options: HashMap<&str, Value> = HashMap::new();
    bus.call_method(Some(DEST), PATH, Some("org.freedesktop.portal.Print"), "Print", &("", title, Fd::from(&file), options))
        .map(|_| ())
        .map_err(|e| format!("Printing is not available.\n\n{e}"))
}

/// A file holding `data` that no other program can open by name: written in the
/// private runtime folder, then removed from it (the open file stays readable).
fn anonymous_file(data: &[u8]) -> std::io::Result<std::fs::File> {
    use std::io::{Seek, Write};
    use std::os::unix::fs::OpenOptionsExt;
    let dir = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let path = dir.join(format!("paintbox-{}-{stamp}", std::process::id()));
    let mut file = std::fs::OpenOptions::new().read(true).write(true).create_new(true).mode(0o600).open(&path)?;
    let _ = std::fs::remove_file(&path);
    file.write_all(data)?;
    file.rewind()?;
    Ok(file)
}

/// Inside Flatpak, start `paintbox <args>` in a sandbox of its own that outlives this
/// one (a plain child process would end with Paintbox), with `stdin` as its input.
pub fn spawn_detached(args: &[&str], stdin: std::io::PipeReader) -> Result<(), String> {
    let bus = bus().ok_or("no D-Bus session")?;
    let bytes = |s: &str| [s.as_bytes(), b"\0"].concat();
    let cwd = bytes("/");
    let argv: Vec<Vec<u8>> = std::iter::once("paintbox").chain(args.iter().copied()).map(bytes).collect();
    let mut fds: HashMap<u32, Fd> = HashMap::new();
    fds.insert(0, Fd::from(&stdin));
    let envs: HashMap<&str, &str> = HashMap::new();
    let options: HashMap<&str, Value> = HashMap::new();
    bus.call_method(
        Some("org.freedesktop.portal.Flatpak"),
        "/org/freedesktop/portal/Flatpak",
        Some("org.freedesktop.portal.Flatpak"),
        "Spawn",
        &(cwd, argv, fds, envs, 0u32, options),
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// A one-page PDF of the picture (RGB pixels, transparency already on white), fitted
/// on Letter paper in the US and Canada and A4 elsewhere, turned to match the picture.
pub fn picture_pdf(width: u32, height: u32, rgb: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let letter = ["LC_ALL", "LC_PAPER", "LANG"]
        .iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
        .is_some_and(|l| ["_US", "_CA", "_MX", "_PH", "_CL", "_CO", "_VE"].iter().any(|c| l.contains(c)));
    let (mut pw, mut ph) = if letter { (612.0, 792.0) } else { (595.0, 842.0) };
    if width > height {
        std::mem::swap(&mut pw, &mut ph);
    }
    // Fit inside half-inch margins, keeping the shape.
    let margin = 36.0;
    let scale = ((pw - 2.0 * margin) / width as f64).min((ph - 2.0 * margin) / height as f64);
    let (iw, ih) = (width as f64 * scale, height as f64 * scale);
    let (x, y) = ((pw - iw) / 2.0, (ph - ih) / 2.0);

    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    let _ = z.write_all(rgb);
    let image = z.finish().unwrap_or_default();
    let content = format!("q {iw:.3} 0 0 {ih:.3} {x:.3} {y:.3} cm /Im0 Do Q\n");

    let mut out: Vec<u8> = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::new();
    let mut object = |out: &mut Vec<u8>, head: String, stream: Option<&[u8]>| {
        offsets.push(out.len());
        out.extend(format!("{} 0 obj\n{head}\n", offsets.len()).as_bytes());
        if let Some(s) = stream {
            out.extend(b"stream\n");
            out.extend(s);
            out.extend(b"\nendstream\n");
        }
        out.extend(b"endobj\n");
    };
    object(&mut out, "<< /Type /Catalog /Pages 2 0 R >>".into(), None);
    object(&mut out, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(), None);
    object(
        &mut out,
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {pw} {ph}] /Resources << /XObject << /Im0 4 0 R >> >> /Contents 5 0 R >>"),
        None,
    );
    object(
        &mut out,
        format!(
            "<< /Type /XObject /Subtype /Image /Width {width} /Height {height} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode /Length {} >>",
            image.len()
        ),
        Some(&image),
    );
    object(&mut out, format!("<< /Length {} >>", content.len()), Some(content.as_bytes()));
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
    for o in &offsets {
        out.extend(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", offsets.len() + 1).as_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_has_a_valid_cross_reference_table() {
        let pdf = picture_pdf(3, 2, &[255; 18]);
        let find = |from: usize, what: &[u8]| pdf[from..].windows(what.len()).position(|w| w == what).map(|p| p + from);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
        // startxref points at the xref table, and every entry at its object.
        let s = find(0, b"startxref\n").unwrap() + 10;
        let start: usize = String::from_utf8_lossy(&pdf[s..]).lines().next().unwrap().parse().unwrap();
        assert!(pdf[start..].starts_with(b"xref"));
        let table = String::from_utf8_lossy(&pdf[start..]).into_owned();
        for (i, line) in table.lines().skip(3).take(5).enumerate() {
            let off: usize = line[..10].parse().unwrap();
            assert!(pdf[off..].starts_with(format!("{} 0 obj", i + 1).as_bytes()), "object {}", i + 1);
        }
    }
}
