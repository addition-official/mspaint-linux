//! The XDG desktop portal: the desktop's own service for things a sandboxed app can't
//! do itself. Paintbox uses it for the light/dark setting everywhere, and inside
//! Flatpak (where desktop tools like `lp` or `gsettings` aren't there) for the
//! wallpaper, printing and keeping a copy on the clipboard after closing. Printing
//! uses the desktop's print dialog everywhere it is available, as Windows does.

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

const PRINT: &str = "org.freedesktop.portal.Print";

/// Show the desktop's print dialog, then print the picture (RGB pixels on white) on the
/// paper chosen there. Blocks until the user is done, so call it off the UI thread.
/// Ok(false) when the user cancels.
pub fn print_dialog(title: &str, width: u32, height: u32, rgb: &[u8]) -> Result<bool, String> {
    let bus = bus().ok_or("Could not reach the desktop (no D-Bus session).")?;
    let unavailable = |e: zbus::Error| format!("The desktop's print dialog is not available.\n\n{e}");
    let (response, results) = request(bus, |token| {
        let mut options: HashMap<&str, Value> = HashMap::new();
        options.insert("handle_token", token.into());
        options.insert("modal", true.into());
        let settings: HashMap<&str, Value> = HashMap::new();
        // Start in landscape for a wide picture.
        let mut page_setup: HashMap<&str, Value> = HashMap::new();
        page_setup.insert("Orientation", if width > height { "landscape" } else { "portrait" }.into());
        bus.call_method(Some(DEST), PATH, Some(PRINT), "PreparePrint", &("", title, settings, page_setup, options))
    })
    .map_err(unavailable)?;
    match response {
        0 => {}
        1 => return Ok(false),
        _ => return Err("The print dialog could not be shown.".into()),
    }
    let token = match results.get("token").map(|v| &**v) {
        Some(Value::U32(t)) => *t,
        _ => return Err("The print dialog did not return a print job.".into()),
    };
    let page = results
        .get("page-setup")
        .and_then(|v| HashMap::<String, OwnedValue>::try_from(v.try_clone().ok()?).ok())
        .and_then(|setup| Page::from_setup(&setup))
        .unwrap_or_else(|| Page::default_for(width, height));
    let pdf = picture_pdf(width, height, rgb, page);
    let file = anonymous_file(&pdf).map_err(|e| format!("Could not prepare the picture for printing.\n\n{e}"))?;
    let (response, _) = request(bus, |handle| {
        let mut options: HashMap<&str, Value> = HashMap::new();
        options.insert("handle_token", handle.into());
        options.insert("token", token.into());
        bus.call_method(Some(DEST), PATH, Some(PRINT), "Print", &("", title, Fd::from(&file), options))
    })
    .map_err(unavailable)?;
    match response {
        0 => Ok(true),
        1 => Ok(false),
        _ => Err("The picture could not be sent to the printer.".into()),
    }
}

/// Call a portal method that answers later with a Response signal on a request
/// object, and wait for that answer. `call` gets the token to pass as handle_token;
/// the request's path is known from it, so the signal can't be missed.
fn request(bus: &Connection, call: impl FnOnce(&str) -> zbus::Result<zbus::Message>) -> zbus::Result<(u32, HashMap<String, OwnedValue>)> {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let token = format!("paintbox{}", NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    let sender = bus.unique_name().ok_or_else(|| zbus::Error::Failure("not connected".into()))?;
    let sender = sender.trim_start_matches(':').replace('.', "_");
    let path = format!("/org/freedesktop/portal/desktop/request/{sender}/{token}");
    let proxy = zbus::blocking::Proxy::new(bus, DEST, path, "org.freedesktop.portal.Request")?;
    let mut answers = proxy.receive_signal("Response")?;
    call(&token)?;
    let answer = answers.next().ok_or_else(|| zbus::Error::Failure("the desktop stopped answering".into()))?;
    answer.body().deserialize()
}

/// A sheet of paper and its margins, in points (1/72 inch).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Page {
    pub width: f64,
    pub height: f64,
    pub margin: [f64; 4], // left, right, top, bottom
}

impl Page {
    /// Letter in the Americas that use it, A4 elsewhere, turned to match the picture,
    /// with half-inch margins.
    pub fn default_for(width: u32, height: u32) -> Page {
        let letter = ["LC_ALL", "LC_PAPER", "LANG"]
            .iter()
            .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
            .is_some_and(|l| ["_US", "_CA", "_MX", "_PH", "_CL", "_CO", "_VE"].iter().any(|c| l.contains(c)));
        let (w, h) = if letter { (612.0, 792.0) } else { (595.0, 842.0) };
        let (w, h) = if width > height { (h, w) } else { (w, h) };
        Page { width: w, height: h, margin: [36.0; 4] }
    }

    /// From the print dialog's page setup (sizes in millimeters, as GTK names them).
    fn from_setup(setup: &HashMap<String, OwnedValue>) -> Option<Page> {
        let mm = |key: &str| match setup.get(key).map(|v| &**v) {
            Some(Value::F64(v)) if v.is_finite() && *v >= 0.0 => Some(v * 72.0 / 25.4),
            _ => None,
        };
        let (mut w, mut h) = (mm("Width")?, mm("Height")?);
        if w < 72.0 || h < 72.0 {
            return None;
        }
        let landscape = matches!(setup.get("Orientation").map(|v| &**v), Some(Value::Str(o)) if o.as_str().contains("landscape"));
        if landscape == (w < h) {
            std::mem::swap(&mut w, &mut h);
        }
        let m = |key| mm(key).unwrap_or(18.0).min(w.min(h) / 4.0);
        Some(Page { width: w, height: h, margin: [m("MarginLeft"), m("MarginRight"), m("MarginTop"), m("MarginBottom")] })
    }
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

/// A one-page PDF of the picture (RGB pixels, transparency already on white), as
/// large as fits inside the page's margins, centered.
pub fn picture_pdf(width: u32, height: u32, rgb: &[u8], page: Page) -> Vec<u8> {
    use std::io::Write;
    let (pw, ph) = (page.width, page.height);
    let [left, right, top, bottom] = page.margin;
    let scale = ((pw - left - right) / width as f64).min((ph - top - bottom) / height as f64).max(0.01);
    let (iw, ih) = (width as f64 * scale, height as f64 * scale);
    let x = left + (pw - left - right - iw) / 2.0;
    let y = bottom + (ph - top - bottom - ih) / 2.0;

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
    fn reads_the_print_dialogs_paper() {
        let setup = |orientation: &str| {
            let mut s: HashMap<String, OwnedValue> = HashMap::new();
            s.insert("Width".into(), Value::F64(210.0).try_into().unwrap());
            s.insert("Height".into(), Value::F64(297.0).try_into().unwrap());
            s.insert("MarginLeft".into(), Value::F64(25.4).try_into().unwrap());
            s.insert("Orientation".into(), Value::from(orientation).try_into().unwrap());
            Page::from_setup(&s).unwrap()
        };
        let portrait = setup("portrait");
        assert!((portrait.width - 595.3).abs() < 0.1 && (portrait.height - 841.9).abs() < 0.1);
        assert!((portrait.margin[0] - 72.0).abs() < 0.01, "25.4 mm is an inch");
        assert_eq!(portrait.margin[1], 18.0, "missing margins default to a quarter inch");
        let landscape = setup("landscape");
        assert!(landscape.width > landscape.height);
        assert!(Page::from_setup(&HashMap::new()).is_none());
    }

    #[test]
    fn pdf_has_a_valid_cross_reference_table() {
        let pdf = picture_pdf(3, 2, &[255; 18], Page::default_for(3, 2));
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
