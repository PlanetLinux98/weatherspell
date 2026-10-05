// Reading and writing the app's own files (settings.json and the cache)
// the way 0.1 does, so each app reads what the other wrote.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

// As 0.1's File.ReadAllText reads it: a byte-order mark says which
// Unicode form (Notepad and PowerShell both write one, and the JSON
// reader would reject it); without one it is UTF-8, any bad bytes
// replaced rather than refused.
pub(crate) fn read_text(path: &Path) -> io::Result<String> {
    let bytes = fs::read(path)?;
    Ok(decode(&bytes))
}

fn decode(bytes: &[u8]) -> String {
    fn utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> String {
        let (pairs, _) = bytes.as_chunks::<2>();
        let units: Vec<u16> = pairs.iter().map(|pair| unit(*pair)).collect();
        String::from_utf16_lossy(&units)
    }
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(rest).into_owned()
    } else if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        utf16(rest, u16::from_le_bytes)
    } else if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        utf16(rest, u16::from_be_bytes)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

// Written to a temporary file beside it and swapped in, so a crash
// mid-write never leaves half a file behind. The folder is made first.
pub(crate) fn replace(path: &Path, contents: &[u8]) -> io::Result<()> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let temp = with_suffix(path, ".tmp");
    fs::write(&temp, contents)?;
    fs::rename(&temp, path).inspect_err(|_| {
        let _ = fs::remove_file(&temp);
    })
}

// "settings.json" to "settings.json.bad".
pub(crate) fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

pub(crate) fn name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_byte_order_mark_says_which_form_and_is_dropped() {
        assert_eq!(decode(b"\xEF\xBB\xBF{\"a\":1}"), "{\"a\":1}");
        assert_eq!(decode(b"\xFF\xFE{\0}\0"), "{}");
        assert_eq!(decode(b"\xFE\xFF\0{\0}"), "{}");
        assert_eq!(decode("\u{CE}le".as_bytes()), "\u{CE}le");
        assert_eq!(decode(b"a\xFFb"), "a\u{FFFD}b");
    }
}
