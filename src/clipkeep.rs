//! Keep a copied picture or text available after Paint closes.
//!
//! On Linux the clipboard is not a store: the program that copied something has to
//! hand it over whenever another program pastes. When Paint closes, its copy would
//! vanish (unless a clipboard manager grabbed it, which few do for pictures). So on
//! exit, if the clipboard still holds what Paint copied, Paint starts a small helper
//! (itself, with `--keep-clipboard`) that takes over the clipboard and exits as soon
//! as something else is copied, like Windows keeps it.

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

const FLAG: &str = "--keep-clipboard";
/// The helper gives up after this long even if nothing else is copied.
const MAX_KEEP: Duration = Duration::from_secs(24 * 60 * 60);

/// What Paint last put on the clipboard.
pub enum Held {
    Text(String),
    Image { width: usize, height: usize, bytes: Vec<u8> },
}

impl Held {
    fn encode(&self) -> Vec<u8> {
        match self {
            Held::Text(t) => [b"T".as_slice(), t.as_bytes()].concat(),
            Held::Image { width, height, bytes } => {
                let mut out = Vec::with_capacity(9 + bytes.len());
                out.push(b'I');
                out.extend((*width as u32).to_le_bytes());
                out.extend((*height as u32).to_le_bytes());
                out.extend_from_slice(bytes);
                out
            }
        }
    }

    fn decode(data: Vec<u8>) -> Option<Held> {
        match data.first()? {
            b'T' => String::from_utf8(data[1..].to_vec()).ok().map(Held::Text),
            b'I' if data.len() >= 9 => {
                let width = u32::from_le_bytes(data[1..5].try_into().ok()?) as usize;
                let height = u32::from_le_bytes(data[5..9].try_into().ok()?) as usize;
                let bytes = data[9..].to_vec();
                (width.checked_mul(height)?.checked_mul(4)? == bytes.len()).then_some(Held::Image { width, height, bytes })
            }
            _ => None,
        }
    }

    /// Whether the clipboard still holds this (nothing else was copied since).
    fn still_on(&self, cb: &mut arboard::Clipboard) -> bool {
        match self {
            Held::Text(t) => cb.get_text().is_ok_and(|now| now == *t),
            Held::Image { width, height, bytes } => {
                // Reading an image back can hit decoder bugs (see pixels::load).
                let img = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cb.get_image()));
                matches!(img, Ok(Ok(img)) if img.width == *width && img.height == *height && img.bytes.as_ref() == bytes.as_slice())
            }
        }
    }
}

/// Called when Paint exits: start the helper if the clipboard still holds our copy.
pub fn hand_off(cb: &mut arboard::Clipboard, held: &Held) {
    if !held.still_on(cb) {
        return;
    }
    // Inside Flatpak a child process would end with Paintbox's sandbox: the portal
    // starts the helper in a sandbox of its own.
    if crate::portal::in_flatpak() {
        if let Ok((reader, mut writer)) = std::io::pipe()
            && crate::portal::spawn_detached(&[FLAG], reader).is_ok()
        {
            let _ = writer.write_all(&held.encode());
        }
        return;
    }
    // Inside an AppImage the binary lives on a mount that goes away when Paint
    // exits, so the helper is started from the AppImage file itself.
    let exe = std::env::var_os("APPIMAGE").map(std::path::PathBuf::from).or_else(|| std::env::current_exe().ok());
    let Some(exe) = exe else { return };
    use std::os::unix::process::CommandExt;
    let child = Command::new(exe)
        .arg(FLAG)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        // Its own process group, so closing the terminal Paint ran in doesn't end it.
        .process_group(0)
        .spawn();
    if let Ok(mut child) = child
        && let Some(mut stdin) = child.stdin.take()
    {
        let _ = stdin.write_all(&held.encode());
        // Dropping stdin closes it; the helper is not waited for.
    }
}

/// `main` calls this first: when started as the helper, hold the clipboard and
/// return true (Paint then exits without opening a window).
pub fn run_if_helper() -> bool {
    if std::env::args().nth(1).as_deref() != Some(FLAG) {
        return false;
    }
    let mut data = Vec::new();
    if std::io::stdin().take(crate::pixels::MAX_PIXELS * 4 + 16).read_to_end(&mut data).is_err() {
        return true;
    }
    let Some(held) = Held::decode(data) else { return true };
    let Ok(mut cb) = arboard::Clipboard::new() else { return true };
    // Give up after a while even if nothing else is copied. (Not `wait_until`: on
    // Wayland arboard treats it as "don't wait", and the helper would quit at once,
    // taking the copy with it.)
    std::thread::spawn(|| {
        std::thread::sleep(MAX_KEEP);
        std::process::exit(0);
    });
    use arboard::SetExtLinux;
    // Blocks until something else is copied.
    let set = cb.set().wait();
    let _ = match held {
        Held::Text(t) => set.text(t),
        Held::Image { width, height, bytes } => set.image(arboard::ImageData { width, height, bytes: bytes.into() }),
    };
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_rejects_bad_data() {
        let t = Held::decode(Held::Text("héllo".into()).encode());
        assert!(matches!(t, Some(Held::Text(s)) if s == "héllo"));
        let img = Held::Image { width: 2, height: 1, bytes: vec![1, 2, 3, 4, 5, 6, 7, 8] };
        assert!(matches!(Held::decode(img.encode()), Some(Held::Image { width: 2, height: 1, .. })));
        let mut short = img.encode();
        short.pop();
        assert!(Held::decode(short).is_none(), "size must match");
        assert!(Held::decode(vec![]).is_none());
        assert!(Held::decode(b"X".to_vec()).is_none());
    }
}
