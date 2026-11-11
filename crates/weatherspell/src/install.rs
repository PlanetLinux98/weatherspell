// Putting a downloaded update in place (#11), per system; which release
// and whether the download is good are the core's (updates.rs). Windows
// so far: the Mac's disk image and Linux's AppImage follow, and until
// then those copies are shown the notes and the download page.
//
// Windows: an exe cannot be deleted while it runs, but it can be renamed,
// so the running one steps aside as Weatherspell.exe.old and the new one
// takes its name; the next start removes the old one. Where the folder
// takes an administrator to change (Program Files), the same two moves go
// through the shell's own file operation, which asks for permission
// itself: Windows' prompt, with no helper program and no command-line
// argument for the exe to take.

use std::path::{Path, PathBuf};

use weatherspell_core::updates;

#[cfg(windows)]
pub const FILE_NAME: &str = updates::WINDOWS_FILE;
#[cfg(target_os = "macos")]
pub const FILE_NAME: &str = updates::MAC_FILE;
#[cfg(not(any(windows, target_os = "macos")))]
pub const FILE_NAME: &str = updates::LINUX_FILE;

// How this copy takes an update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    // It swaps itself; with_permission where its folder takes an
    // administrator to change.
    Install { with_permission: bool },
    // winget's copy: winget owns the folder, and is asked to upgrade.
    Winget,
    // Nothing this copy can swap: the download page.
    ByHand,
}

pub fn way() -> Way {
    #[cfg(windows)]
    {
        win::way()
    }
    #[cfg(not(windows))]
    {
        Way::ByHand
    }
}

// Where the download goes before it is checked: beside the exe when that
// folder can be written to, so putting it in place is a rename; otherwise
// the temporary folder. A new name each time, so a download cancelled
// while its connection hung cannot trip the next one.
pub fn download_path(way: Way) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        win::download_path(way)
    }
    #[cfg(not(windows))]
    {
        let _ = way;
        None
    }
}

// The checked download put in place of the running copy. The reason, in
// words for the user, when it could not be; empty when they declined.
pub fn swap(new: &Path, way: Way, owner: &dyn wxdragon::prelude::WxWidget) -> Result<(), String> {
    #[cfg(windows)]
    {
        win::swap(new, way, owner)
    }
    #[cfg(not(windows))]
    {
        let _ = (new, way, owner);
        Err("this copy can't update itself.".to_string())
    }
}

// At start-up: what the last update left behind, if anything. Only files
// this code names; whatever cannot be removed yet (the old copy still
// closing) goes at the next start.
pub fn clean_up() {
    #[cfg(windows)]
    win::clean_up();
}

// The new copy started, as the window closes. This copy's hold on being
// the only one is let go first, or the new one would take itself for a
// second copy and end.
pub fn relaunch() {
    #[cfg(windows)]
    {
        crate::system::let_another_copy_start();
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::process::Command::new(exe).spawn();
        }
    }
}

#[cfg(windows)]
mod win {
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::Way;
    use weatherspell_core::updates;

    const OLD: &str = ".old";
    const NEW: &str = ".new";
    const TEMP_FOLDER: &str = "Weatherspell-update";

    fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
        let mut name = path.as_os_str().to_owned();
        name.push(suffix);
        PathBuf::from(name)
    }

    fn temp_folder() -> PathBuf {
        std::env::temp_dir().join(TEMP_FOLDER)
    }

    pub fn way() -> Way {
        let Ok(exe) = std::env::current_exe() else {
            return Way::ByHand;
        };
        // As it was started and as it resolves: winget's Links folder
        // holds a link to the exe in Packages.
        let resolved = fs::canonicalize(&exe).unwrap_or_else(|_| exe.clone());
        if [&exe, &resolved]
            .iter()
            .any(|path| updates::is_winget_path(&path.to_string_lossy()))
        {
            return Way::Winget;
        }
        let Some(folder) = exe.parent() else {
            return Way::ByHand;
        };
        // Asked of the folder itself: its permissions say little that is
        // simple to read.
        let probe = folder.join(format!("Weatherspell-{}.tmp", std::process::id()));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&probe)
        {
            Ok(file) => {
                drop(file);
                let _ = fs::remove_file(&probe);
                Way::Install {
                    with_permission: false,
                }
            }
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => Way::Install {
                with_permission: true,
            },
            // A disc, a share that has gone: permission would not help.
            Err(_) => Way::ByHand,
        }
    }

    pub fn download_path(way: Way) -> Option<PathBuf> {
        static ATTEMPT: AtomicU32 = AtomicU32::new(0);
        let n = ATTEMPT.fetch_add(1, Ordering::Relaxed) + 1;
        let exe = std::env::current_exe().ok()?;
        match way {
            Way::Install {
                with_permission: false,
            } => Some(with_suffix(&exe, &format!("{NEW}{n}"))),
            Way::Install {
                with_permission: true,
            } => {
                let folder = temp_folder();
                fs::create_dir_all(&folder).ok()?;
                Some(folder.join(format!("{}{NEW}{n}", exe.file_name()?.to_string_lossy())))
            }
            Way::Winget | Way::ByHand => None,
        }
    }

    pub fn swap(
        new: &Path,
        way: Way,
        owner: &dyn wxdragon::prelude::WxWidget,
    ) -> Result<(), String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        match way {
            Way::Install {
                with_permission: false,
            } => rename_in(new, &exe).map_err(|e| e.to_string()),
            Way::Install {
                with_permission: true,
            } => with_permission(
                new,
                &exe,
                windows::Win32::Foundation::HWND(owner.get_handle()),
            ),
            Way::Winget | Way::ByHand => Err("this copy can't update itself.".to_string()),
        }
    }

    // The running exe steps aside and the new one takes its name; put back
    // if the second half fails, so there is always an exe to start.
    fn rename_in(new: &Path, exe: &Path) -> io::Result<()> {
        let old = with_suffix(exe, OLD);
        // One left by an update the app was never restarted after is in
        // use and stays: the rename below then says so.
        let _ = fs::remove_file(&old);
        fs::rename(exe, &old)?;
        fs::rename(new, exe).inspect_err(|_| {
            let _ = fs::rename(&old, exe);
        })
    }

    // The same through IFileOperation, which shows Windows' own prompt for
    // an administrator's permission when the folder needs it. The old exe
    // goes to the temporary folder when that is on the same drive (a
    // rename, which a running exe allows; nothing is left beside the new
    // one) and steps aside as .old otherwise, as above.
    fn with_permission(
        new: &Path,
        exe: &Path,
        owner: windows::Win32::Foundation::HWND,
    ) -> Result<(), String> {
        use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
        use windows::Win32::UI::Shell::{
            FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FOFX_EARLYFAILURE, FOFX_NOCOPYHOOKS,
            FOFX_SHOWELEVATIONPROMPT, FileOperation, IFileOperation, IShellItem,
            SHCreateItemFromParsingName,
        };
        use windows::core::HSTRING;

        let folder = exe.parent().ok_or("the app's folder was not found.")?;
        let name = exe.file_name().ok_or("the app's file was not found.")?;
        let old_name = with_suffix(Path::new(name), OLD);
        let temp = temp_folder();
        let aside_in_temp = same_drive(&temp, exe) && fs::create_dir_all(&temp).is_ok();
        if aside_in_temp {
            let _ = fs::remove_file(temp.join(&old_name));
        }

        let item = |path: &Path| -> windows::core::Result<IShellItem> {
            unsafe { SHCreateItemFromParsingName(&HSTRING::from(path.as_os_str()), None) }
        };
        let moved = (|| -> windows::core::Result<bool> {
            unsafe {
                let operation: IFileOperation = CoCreateInstance(&FileOperation, None, CLSCTX_ALL)?;
                operation.SetOperationFlags(
                    FOF_NOCONFIRMATION
                        | FOF_NOERRORUI
                        | FOF_SILENT
                        | FOFX_NOCOPYHOOKS
                        | FOFX_EARLYFAILURE
                        | FOFX_SHOWELEVATIONPROMPT,
                )?;
                // The prompt belongs to this window, so it opens in front
                // rather than waiting unseen on the taskbar.
                operation.SetOwnerWindow(owner)?;
                if aside_in_temp {
                    operation.MoveItem(
                        &item(exe)?,
                        &item(&temp)?,
                        &HSTRING::from(old_name.as_os_str()),
                        None,
                    )?;
                } else {
                    operation.RenameItem(
                        &item(exe)?,
                        &HSTRING::from(old_name.as_os_str()),
                        None,
                    )?;
                }
                operation.MoveItem(&item(new)?, &item(folder)?, &HSTRING::from(name), None)?;
                operation.PerformOperations()?;
                Ok(!operation.GetAnyOperationsAborted()?.as_bool())
            }
        })();

        if exe.exists() && !new.exists() {
            return Ok(());
        }
        // Half done: the old exe stepped aside and the new one did not
        // arrive. Put back, which may ask for permission again.
        if !exe.exists() {
            let aside = if aside_in_temp {
                temp.join(&old_name)
            } else {
                folder.join(&old_name)
            };
            let _ = (|| -> windows::core::Result<()> {
                unsafe {
                    let operation: IFileOperation =
                        CoCreateInstance(&FileOperation, None, CLSCTX_ALL)?;
                    operation.SetOperationFlags(
                        FOF_NOCONFIRMATION
                            | FOF_NOERRORUI
                            | FOF_SILENT
                            | FOFX_NOCOPYHOOKS
                            | FOFX_SHOWELEVATIONPROMPT,
                    )?;
                    operation.SetOwnerWindow(owner)?;
                    operation.MoveItem(
                        &item(&aside)?,
                        &item(folder)?,
                        &HSTRING::from(name),
                        None,
                    )?;
                    operation.PerformOperations()
                }
            })();
        }
        match moved {
            // Permission declined (ERROR_CANCELLED, as an HRESULT): nothing
            // to add to "Nothing was installed."
            Ok(_) => Err(String::new()),
            Err(e) if e.code().0 as u32 == 0x8007_04C7 => Err(String::new()),
            Err(e) => Err(e.message()),
        }
    }

    fn same_drive(a: &Path, b: &Path) -> bool {
        use std::path::Component;
        let drive = |path: &Path| match path.components().next() {
            Some(Component::Prefix(prefix)) => {
                Some(prefix.as_os_str().to_string_lossy().to_lowercase())
            }
            _ => None,
        };
        drive(a).is_some() && drive(a) == drive(b)
    }

    // Whatever cannot go yet (the copy that was just replaced is still
    // closing) is tried again for a few seconds, so nothing sits beside
    // the exe until the next start.
    pub fn clean_up() {
        if !sweep() {
            std::thread::spawn(|| {
                for _ in 0..10 {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    if sweep() {
                        break;
                    }
                }
            });
        }
    }

    // True when nothing of ours is left.
    fn sweep() -> bool {
        let Ok(exe) = std::env::current_exe() else {
            return true;
        };
        let Some(name) = exe.file_name().map(|n| n.to_string_lossy().into_owned()) else {
            return true;
        };
        let mut gone = true;
        for folder in [exe.parent().map(Path::to_path_buf), Some(temp_folder())]
            .into_iter()
            .flatten()
        {
            let Ok(entries) = fs::read_dir(&folder) else {
                continue;
            };
            for entry in entries.flatten() {
                if left_behind(&entry.file_name().to_string_lossy(), &name) {
                    gone &= fs::remove_file(entry.path()).is_ok();
                }
            }
        }
        // Gone only when empty.
        let _ = fs::remove_dir(temp_folder());
        gone
    }

    // "Weatherspell.exe.old", and a download's "Weatherspell.exe.new3".
    fn left_behind(file: &str, exe_name: &str) -> bool {
        file.strip_prefix(exe_name).is_some_and(|rest| {
            rest == OLD
                || rest
                    .strip_prefix(NEW)
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn folder(name: &str) -> PathBuf {
            let folder = std::env::temp_dir().join(format!(
                "weatherspell-install-{}-{name}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&folder);
            fs::create_dir_all(&folder).unwrap();
            folder
        }

        #[test]
        fn only_an_updates_own_files_are_swept() {
            for ours in [
                "Weatherspell.exe.old",
                "Weatherspell.exe.new1",
                "Weatherspell.exe.new27",
            ] {
                assert!(left_behind(ours, "Weatherspell.exe"), "{ours}");
            }
            for other in [
                "Weatherspell.exe",
                "Weatherspell.exe.new",
                "Weatherspell.exe.newer",
                "Weatherspell.exe.old.txt",
                "Weatherspell.exe.bak",
                "notes.old",
            ] {
                assert!(!left_behind(other, "Weatherspell.exe"), "{other}");
            }
        }

        #[test]
        fn the_new_exe_takes_the_name_and_the_old_one_steps_aside() {
            let folder = folder("rename");
            let (exe, new) = (
                folder.join("Weatherspell.exe"),
                folder.join("Weatherspell.exe.new1"),
            );
            fs::write(&exe, "old").unwrap();
            fs::write(&new, "new").unwrap();
            // One left from an earlier update is replaced.
            fs::write(folder.join("Weatherspell.exe.old"), "older").unwrap();
            rename_in(&new, &exe).unwrap();
            assert_eq!(fs::read_to_string(&exe).unwrap(), "new");
            assert_eq!(
                fs::read_to_string(folder.join("Weatherspell.exe.old")).unwrap(),
                "old"
            );
            assert!(!new.exists());
            let _ = fs::remove_dir_all(&folder);
        }

        #[test]
        fn a_download_that_has_gone_leaves_the_exe_as_it_was() {
            let folder = folder("missing");
            let exe = folder.join("Weatherspell.exe");
            fs::write(&exe, "old").unwrap();
            assert!(rename_in(&folder.join("Weatherspell.exe.new1"), &exe).is_err());
            assert_eq!(fs::read_to_string(&exe).unwrap(), "old");
            let _ = fs::remove_dir_all(&folder);
        }

        // The shell's file operation, in a folder that needs no permission:
        // the same two moves, with no prompt. Where it does need one,
        // Windows asks, which only a person can answer.
        #[test]
        fn the_shells_file_operation_makes_the_same_swap() {
            use windows::Win32::Foundation::HWND;
            use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
            unsafe {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            }
            let folder = folder("shell");
            let (exe, new) = (
                folder.join("Weatherspell.exe"),
                folder.join("Weatherspell.exe.new1"),
            );
            fs::write(&exe, "old").unwrap();
            fs::write(&new, "new").unwrap();
            let aside = temp_folder().join("Weatherspell.exe.old");
            with_permission(&new, &exe, HWND(std::ptr::null_mut())).unwrap();
            assert_eq!(fs::read_to_string(&exe).unwrap(), "new");
            assert!(!new.exists());
            // The temporary folder is on this folder's drive, so the old
            // exe went there and nothing is left beside the new one.
            assert_eq!(fs::read_to_string(&aside).unwrap(), "old");
            assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
            let _ = fs::remove_file(&aside);
            let _ = fs::remove_dir_all(&folder);
        }
    }
}
