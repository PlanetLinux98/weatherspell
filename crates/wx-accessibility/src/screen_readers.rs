// Which screen readers are running, for the few places where what one
// needs another says twice. After the app moves the caret itself (a
// section key, say), NVDA and JAWS say nothing, since they read the new
// line only after keys they know, so the app speaks it; Narrator reads it
// as its own caret reading, and with the app's words too heard the line
// twice ("Rest of today, Rest of today"; Elliott, 2026-10-05). Found by
// process name, on Windows only; elsewhere every check says the app
// speaks.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScreenReaders {
    pub nvda: bool,
    pub jaws: bool,
    pub narrator: bool,
}

impl ScreenReaders {
    pub fn running() -> ScreenReaders {
        #[cfg(windows)]
        {
            ScreenReaders::among(process_names())
        }
        #[cfg(not(windows))]
        ScreenReaders::default()
    }

    // An installed NVDA runs as nvda_uiAccess.exe (nvda.exe starts it, for
    // the access it needs), or nvda_noUIAccess.exe; a portable copy as
    // nvda.exe. Looking for nvda.exe alone missed it, so NVDA lost its
    // headings (Elliott, 2026-10-05): any "nvda" program counts.
    fn among(names: impl IntoIterator<Item = String>) -> ScreenReaders {
        let mut found = ScreenReaders::default();
        for name in names {
            let name = name.to_ascii_lowercase();
            if name.starts_with("nvda") && name.ends_with(".exe") {
                found.nvda = true;
            } else if name == "jfw.exe" {
                found.jaws = true;
            } else if name == "narrator.exe" {
                found.narrator = true;
            }
        }
        found
    }

    // Whether a line the app's own caret move put under the reader has to
    // be spoken by the app: yes unless Narrator is the only one listening.
    pub fn need_caret_moves_spoken(&self) -> bool {
        !(self.narrator && !self.nvda && !self.jaws)
    }
}

#[cfg(windows)]
fn process_names() -> Vec<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    let mut names = Vec::new();
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return names;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut more = Process32FirstW(snapshot, &mut entry).is_ok();
        while more {
            let len = entry
                .szExeFile
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(entry.szExeFile.len());
            names.push(String::from_utf16_lossy(&entry.szExeFile[..len]));
            more = Process32NextW(snapshot, &mut entry).is_ok();
        }
        let _ = CloseHandle(snapshot);
    }
    names
}

#[cfg(test)]
mod tests {
    use super::ScreenReaders;

    #[test]
    fn only_narrator_alone_reads_caret_moves_itself() {
        let with = |nvda, jaws, narrator| ScreenReaders {
            nvda,
            jaws,
            narrator,
        };
        assert!(!with(false, false, true).need_caret_moves_spoken());
        assert!(with(true, false, true).need_caret_moves_spoken());
        assert!(with(false, true, true).need_caret_moves_spoken());
        assert!(with(true, false, false).need_caret_moves_spoken());
        assert!(with(false, false, false).need_caret_moves_spoken());
    }

    #[test]
    fn nvda_is_found_under_each_of_its_names() {
        let names = |list: &[&str]| list.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        for nvda in [
            "nvda.exe",
            "nvda_uiAccess.exe",
            "nvda_noUIAccess.exe",
            "NVDA.EXE",
        ] {
            let found = ScreenReaders::among(names(&["explorer.exe", nvda, "Narrator.exe"]));
            assert!(found.nvda && found.narrator && !found.jaws, "{nvda}");
            assert!(found.need_caret_moves_spoken(), "{nvda}");
        }
        let alone = ScreenReaders::among(names(&["explorer.exe", "Narrator.exe"]));
        assert!(!alone.need_caret_moves_spoken());
        assert!(
            ScreenReaders::among(names(&["jfw.exe", "Narrator.exe"])).need_caret_moves_spoken()
        );
    }

    // The listing works at all: it finds this test itself.
    #[cfg(windows)]
    #[test]
    fn the_running_processes_are_listed() {
        let me = std::env::current_exe().unwrap();
        let me = me.file_name().unwrap().to_string_lossy().to_lowercase();
        assert!(
            super::process_names()
                .iter()
                .any(|n| n.to_lowercase() == me)
        );
    }
}
