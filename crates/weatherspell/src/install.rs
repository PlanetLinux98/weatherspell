// Putting a downloaded update in place (#11), per system; which release
// and whether the download is good are the core's (updates.rs).
//
// Windows: an exe cannot be deleted while it runs, but it can be renamed,
// so the running one steps aside as Weatherspell.exe.old and the new one
// takes its name; the next start removes the old one. Where the folder
// takes an administrator to change (Program Files), the same two moves go
// through the shell's own file operation, which asks for permission
// itself: Windows' prompt, with no helper program and no command-line
// argument for the exe to take.
//
// Linux: the AppImage's runtime names the file it was started from
// ($APPIMAGE). The new one is renamed over it; the running copy keeps the
// old file until it ends, so nothing steps aside. Any other copy (built
// from source, a distro's package, an AppImage in a folder this account
// can't write to) is shown the download page: there is no prompt for
// permission that every desktop has.
//
// The Mac: the download is the disk image. It is mounted, the app in it
// checked against its own signature and copied beside the running bundle,
// and the two change places; the old bundle waits in the temporary folder
// for the next start to remove. Where the Applications folder is another
// account's to change, the same moves are one shell script run through
// the Mac's own administrator prompt. A copy running from the disk image,
// or one macOS runs from a randomized place, has nothing to swap.

use std::path::{Path, PathBuf};

use weatherspell_core::updates;

#[cfg(windows)]
pub const FILE_NAME: &str = updates::WINDOWS_FILE;
#[cfg(target_os = "macos")]
pub const FILE_NAME: &str = updates::MAC_FILE;
#[cfg(not(any(windows, target_os = "macos")))]
pub const FILE_NAME: &str = updates::LINUX_FILE;

#[cfg(not(any(windows, target_os = "macos")))]
use appimage as platform;
#[cfg(target_os = "macos")]
use mac as platform;
#[cfg(windows)]
use win as platform;

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
    platform::way()
}

// Where the download goes before it is checked: beside the app when that
// folder can be written to, so putting it in place is a rename; otherwise
// (and for the Mac's disk image) the temporary folder. A new name each
// time, so a download cancelled while its connection hung cannot trip the
// next one.
pub fn download_path(way: Way) -> Option<PathBuf> {
    platform::download_path(way)
}

// The checked download put in place of the running copy. The reason, in
// words for the user, when it could not be; empty when they declined.
pub fn swap(new: &Path, way: Way, owner: &dyn wxdragon::prelude::WxWidget) -> Result<(), String> {
    platform::swap(new, way, owner)
}

// At start-up: what the last update left behind, if anything. Only files
// this code names; whatever cannot be removed yet (the old copy still
// closing) goes at the next start.
pub fn clean_up() {
    platform::clean_up();
}

// The new copy started, as the window closes. On Windows this copy's hold
// on being the only one is let go first, or the new one would take itself
// for a second copy and end.
pub fn relaunch() {
    platform::relaunch();
}

#[cfg(not(target_os = "macos"))]
const NEW: &str = ".new";
const CANNOT: &str = "this copy can't update itself.";

// A download's own name beside what it replaces: "Weatherspell.exe.new3".
#[cfg(not(target_os = "macos"))]
fn is_download(file: &str, target_name: &str) -> bool {
    file.strip_prefix(target_name)
        .and_then(|rest| rest.strip_prefix(NEW))
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(not(target_os = "macos"))]
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

// Each download's number.
fn attempt() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static ATTEMPT: AtomicU32 = AtomicU32::new(0);
    ATTEMPT.fetch_add(1, Ordering::Relaxed) + 1
}

// Whether a file can be made in a folder, asked of the folder itself: its
// permissions say little that is simple to read. The error when not.
fn can_write_in(folder: &Path) -> std::io::Result<()> {
    let probe = folder.join(format!("Weatherspell-{}.tmp", std::process::id()));
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .map(|file| {
            drop(file);
            let _ = std::fs::remove_file(&probe);
        })
}

#[cfg(windows)]
mod win {
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};

    use super::{CANNOT, NEW, Way, attempt, can_write_in, is_download, with_suffix};
    use weatherspell_core::updates;

    const OLD: &str = ".old";
    const TEMP_FOLDER: &str = "Weatherspell-update";

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
        match can_write_in(folder) {
            Ok(()) => Way::Install {
                with_permission: false,
            },
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => Way::Install {
                with_permission: true,
            },
            // A disc, a share that has gone: permission would not help.
            Err(_) => Way::ByHand,
        }
    }

    pub fn download_path(way: Way) -> Option<PathBuf> {
        let n = attempt();
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
            Way::Winget | Way::ByHand => Err(CANNOT.to_string()),
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

    pub fn relaunch() {
        crate::system::let_another_copy_start();
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::process::Command::new(exe).spawn();
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
        file.strip_prefix(exe_name) == Some(OLD) || is_download(file, exe_name)
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

#[cfg(not(any(windows, target_os = "macos")))]
mod appimage {
    use std::ffi::OsString;
    use std::fs;
    use std::io;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};

    use super::{CANNOT, NEW, Way, attempt, can_write_in, is_download, with_suffix};

    // The AppImage this copy runs from. Its runtime sets APPIMAGE (the
    // file, links resolved) and APPDIR (where it is mounted); both are
    // inherited by anything an AppImage starts, so another AppImage's
    // (a terminal's, a launcher's) must not be taken for ours: this
    // program has to be inside the mount.
    fn image_of(image: Option<OsString>, mount: Option<OsString>, exe: &Path) -> Option<PathBuf> {
        let (image, mount) = (PathBuf::from(image?), PathBuf::from(mount?));
        (image.is_absolute() && mount.is_absolute() && exe.starts_with(&mount)).then_some(image)
    }

    fn image() -> Option<PathBuf> {
        let image = image_of(
            std::env::var_os("APPIMAGE"),
            std::env::var_os("APPDIR"),
            &std::env::current_exe().ok()?,
        )?;
        image.is_file().then_some(image)
    }

    pub fn way() -> Way {
        match image().as_deref().and_then(Path::parent).map(can_write_in) {
            Some(Ok(())) => Way::Install {
                with_permission: false,
            },
            _ => Way::ByHand,
        }
    }

    pub fn download_path(way: Way) -> Option<PathBuf> {
        match way {
            Way::Install { .. } => Some(with_suffix(&image()?, &format!("{NEW}{}", attempt()))),
            Way::Winget | Way::ByHand => None,
        }
    }

    pub fn swap(
        new: &Path,
        way: Way,
        _owner: &dyn wxdragon::prelude::WxWidget,
    ) -> Result<(), String> {
        match (way, image()) {
            (Way::Install { .. }, Some(image)) => {
                rename_over(new, &image).map_err(|e| format!("{e}."))
            }
            _ => Err(CANNOT.to_string()),
        }
    }

    // The download takes the AppImage's name, and its permissions with
    // run added: a download is not a program until it is allowed to be.
    // The rename is one step, so there is always a whole file under the
    // name; the running copy reads on from the old one.
    fn rename_over(new: &Path, image: &Path) -> io::Result<()> {
        let mode = fs::metadata(image).map_or(0o755, |m| m.permissions().mode() & 0o777);
        fs::set_permissions(new, fs::Permissions::from_mode(mode | 0o111))?;
        fs::rename(new, image)
    }

    // A download left by a copy that was ended part-way.
    pub fn clean_up() {
        let Some(image) = image() else {
            return;
        };
        let (Some(folder), Some(name)) = (image.parent(), image.file_name()) else {
            return;
        };
        let name = name.to_string_lossy();
        for entry in fs::read_dir(folder).into_iter().flatten().flatten() {
            if is_download(&entry.file_name().to_string_lossy(), &name) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    // The AppImage started again, as a program of its own: the runtime
    // sets its variables afresh for the new copy. Two copies may run on
    // Linux, so the new one starting as this one closes is no trouble.
    pub fn relaunch() {
        if let Some(image) = image() {
            let _ = std::process::Command::new(image).spawn();
        }
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
        fn only_this_programs_own_appimage_is_taken() {
            let some = |text: &str| Some(OsString::from(text));
            let exe = Path::new("/tmp/.mount_WeathAbCd/usr/bin/weatherspell");
            assert_eq!(
                image_of(
                    some("/home/me/Apps/Weatherspell-x86_64.AppImage"),
                    some("/tmp/.mount_WeathAbCd"),
                    exe
                ),
                Some(PathBuf::from("/home/me/Apps/Weatherspell-x86_64.AppImage"))
            );
            // Started from another AppImage (a terminal's variables are
            // inherited), built from source, or a distro's package.
            assert_eq!(
                image_of(
                    some("/home/me/Apps/Terminal.AppImage"),
                    some("/tmp/.mount_TermiXyZ"),
                    Path::new("/usr/bin/weatherspell")
                ),
                None
            );
            assert_eq!(image_of(None, None, exe), None);
            assert_eq!(image_of(some("Weatherspell.AppImage"), None, exe), None);
        }

        #[test]
        fn the_download_takes_the_appimages_name_and_can_run() {
            let folder = folder("appimage");
            let (image, new) = (
                folder.join("Weatherspell-x86_64.AppImage"),
                folder.join("Weatherspell-x86_64.AppImage.new1"),
            );
            fs::write(&image, "old").unwrap();
            fs::set_permissions(&image, fs::Permissions::from_mode(0o700)).unwrap();
            fs::write(&new, "new").unwrap();
            fs::set_permissions(&new, fs::Permissions::from_mode(0o644)).unwrap();
            rename_over(&new, &image).unwrap();
            assert_eq!(fs::read_to_string(&image).unwrap(), "new");
            assert_eq!(
                fs::metadata(&image).unwrap().permissions().mode() & 0o777,
                0o711
            );
            assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
            let _ = fs::remove_dir_all(&folder);
        }

        #[test]
        fn a_download_that_has_gone_leaves_the_appimage_as_it_was() {
            let folder = folder("missing");
            let image = folder.join("Weatherspell-x86_64.AppImage");
            fs::write(&image, "old").unwrap();
            assert!(
                rename_over(&folder.join("Weatherspell-x86_64.AppImage.new1"), &image).is_err()
            );
            assert_eq!(fs::read_to_string(&image).unwrap(), "old");
            let _ = fs::remove_dir_all(&folder);
        }

        #[test]
        fn only_downloads_are_cleaned_up() {
            let name = "Weatherspell-x86_64.AppImage";
            assert!(is_download("Weatherspell-x86_64.AppImage.new2", name));
            for other in [
                name,
                "Weatherspell-x86_64.AppImage.new",
                "Weatherspell-x86_64.AppImage.newer",
                "notes.new1",
            ] {
                assert!(!is_download(other, name), "{other}");
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::fs;
    use std::io;
    use std::os::unix::fs::MetadataExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use super::{CANNOT, Way, attempt, can_write_in};

    const TEMP_FOLDER: &str = "Weatherspell-update";
    const APP: &str = "Weatherspell.app";
    // Beside the bundle while the swap is made, hidden from Finder.
    const STAGED: &str = ".Weatherspell.app.new";
    const ASIDE: &str = ".Weatherspell.app.old";
    // In the temporary folder.
    const OLD: &str = "Weatherspell.app.old";

    fn temp_folder() -> PathBuf {
        std::env::temp_dir().join(TEMP_FOLDER)
    }

    // "/Applications/Weatherspell.app" of
    // ".../Weatherspell.app/Contents/MacOS/Weatherspell".
    fn bundle_of(exe: &Path) -> Option<PathBuf> {
        let macos = exe.parent()?;
        let contents = macos.parent()?;
        let bundle = contents.parent()?;
        (macos.file_name()? == "MacOS"
            && contents.file_name()? == "Contents"
            && bundle.extension()? == "app")
            .then(|| bundle.to_path_buf())
    }

    fn bundle() -> Option<PathBuf> {
        bundle_of(&std::env::current_exe().ok()?)
    }

    // A copy opened straight from a download macOS runs from a read-only
    // place of its own choosing (App Translocation): the real bundle is
    // somewhere this program is not told.
    fn translocated(bundle: &Path) -> bool {
        bundle
            .components()
            .any(|part| part.as_os_str() == "AppTranslocation")
    }

    pub fn way() -> Way {
        let Some(bundle) = bundle().filter(|b| !translocated(b)) else {
            return Way::ByHand;
        };
        let Some(folder) = bundle.parent() else {
            return Way::ByHand;
        };
        match can_write_in(folder) {
            // The folder is this account's to change; the bundle must be
            // too, to be moved out of it (another administrator's copy in
            // Applications is not).
            Ok(()) => Way::Install {
                with_permission: !owned_by_this_account(&bundle),
            },
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => Way::Install {
                with_permission: true,
            },
            // The disk image itself, read-only: permission would not help.
            Err(_) => Way::ByHand,
        }
    }

    // Against a folder this account certainly owns, which saves asking
    // the system for the account's number.
    fn owned_by_this_account(bundle: &Path) -> bool {
        let folder = temp_folder();
        let _ = fs::create_dir_all(&folder);
        match (fs::metadata(bundle), fs::metadata(&folder)) {
            (Ok(bundle), Ok(mine)) => bundle.uid() == mine.uid(),
            _ => false,
        }
    }

    // The disk image, always in the temporary folder: it is not what
    // takes the bundle's place.
    pub fn download_path(way: Way) -> Option<PathBuf> {
        match way {
            Way::Install { .. } => {
                let folder = temp_folder();
                fs::create_dir_all(&folder).ok()?;
                Some(folder.join(format!("Weatherspell-{}.dmg", attempt())))
            }
            Way::Winget | Way::ByHand => None,
        }
    }

    pub fn swap(
        new: &Path,
        way: Way,
        _owner: &dyn wxdragon::prelude::WxWidget,
    ) -> Result<(), String> {
        let (Way::Install { with_permission }, Some(bundle)) = (way, bundle()) else {
            return Err(CANNOT.to_string());
        };
        let done = install_from(new, &bundle, with_permission);
        let _ = fs::remove_file(new);
        if done.is_ok() {
            // Spotlight and Launch Services find the new version on their
            // own, but not always straight away (install.sh does the
            // same). Neither is needed for the app to run.
            let _ = Command::new(
                "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister",
            )
            .arg("-f")
            .arg(&bundle)
            .output();
        }
        done
    }

    // The words a tool gave for failing, as the end of a sentence.
    fn said(output: &std::process::Output) -> String {
        let text = String::from_utf8_lossy(&output.stderr);
        match text.lines().map(str::trim).rfind(|line| !line.is_empty()) {
            Some(line) => format!(": {}", line.trim_end_matches('.')),
            None => String::new(),
        }
    }

    fn run(command: &mut Command, failed: &str) -> Result<(), String> {
        match command.output() {
            Ok(output) if output.status.success() => Ok(()),
            Ok(output) => Err(format!("{failed}{}.", said(&output))),
            Err(e) => Err(format!("{failed}: {e}.")),
        }
    }

    // The disk image mounted where Finder does not show it, the app in it
    // put in the bundle's place, and the image let go whatever happened.
    fn install_from(image: &Path, bundle: &Path, with_permission: bool) -> Result<(), String> {
        let volume = temp_folder().join(format!("volume-{}-{}", std::process::id(), attempt()));
        let _ = fs::create_dir_all(&volume);
        run(
            Command::new("/usr/bin/hdiutil")
                .args(["attach", "-quiet", "-nobrowse", "-readonly", "-noautoopen"])
                .arg("-mountpoint")
                .arg(&volume)
                .arg(image),
            "the downloaded disk image couldn't be opened",
        )?;
        let done = (|| {
            let app = volume.join(APP);
            if !app.is_dir() {
                return Err("the download has no Weatherspell in it.".to_string());
            }
            // The seal the build put on the app: a copy that fails it is
            // not run by macOS on Apple silicon at all.
            run(
                Command::new("/usr/bin/codesign")
                    .args(["--verify", "--deep", "--strict"])
                    .arg(&app),
                "the downloaded app didn't pass its signature check",
            )?;
            if with_permission {
                replace_with_permission(&app, bundle)
            } else {
                replace(&app, bundle)
            }
        })();
        let _ = Command::new("/usr/bin/hdiutil")
            .args(["detach", "-quiet", "-force"])
            .arg(&volume)
            .output();
        let _ = fs::remove_dir(&volume);
        done
    }

    // Copied beside the bundle (so the swap is two renames on one disk),
    // then the running bundle steps aside and the new one takes its name;
    // put back if the second half fails, so there is always an app to
    // open. The old one goes to the temporary folder when that is on the
    // same disk, as it nearly always is, and waits beside otherwise.
    fn replace(app: &Path, bundle: &Path) -> Result<(), String> {
        let folder = bundle.parent().ok_or("the app's folder was not found.")?;
        let staged = folder.join(STAGED);
        let _ = fs::remove_dir_all(&staged);
        run(
            Command::new("/usr/bin/ditto").arg(app).arg(&staged),
            "the new version couldn't be copied",
        )
        .inspect_err(|_| {
            let _ = fs::remove_dir_all(&staged);
        })?;
        let mut aside = temp_folder().join(OLD);
        let _ = fs::remove_dir_all(&aside);
        if fs::rename(bundle, &aside).is_err() {
            aside = folder.join(ASIDE);
            let _ = fs::remove_dir_all(&aside);
            fs::rename(bundle, &aside).map_err(|e| {
                let _ = fs::remove_dir_all(&staged);
                format!("{e}.")
            })?;
        }
        fs::rename(&staged, bundle).map_err(|e| {
            let _ = fs::rename(&aside, bundle);
            let _ = fs::remove_dir_all(&staged);
            format!("{e}.")
        })
    }

    // The same as one shell script, run as an administrator through the
    // Mac's own prompt (AppleScript's "with administrator privileges"):
    // no helper program, and the paths go in as arguments, never pasted
    // into the script. The old bundle is handed to this account so the
    // next start can remove it.
    const SCRIPT: &str = r#"set -e
app=$1; bundle=$2; staged=$3; aside=$4; owner=$5
/bin/rm -rf "$staged" "$aside"
/usr/bin/ditto "$app" "$staged" || { /bin/rm -rf "$staged"; exit 1; }
/bin/mv "$bundle" "$aside" || { /bin/rm -rf "$staged"; exit 1; }
if ! /bin/mv "$staged" "$bundle"; then
    /bin/mv "$aside" "$bundle"
    /bin/rm -rf "$staged"
    exit 1
fi
/usr/sbin/chown -R "$owner" "$aside" || true"#;

    const ASK: &str = r#"on run arguments
    set command to "/bin/sh -c " & quoted form of (item 1 of arguments) & " sh"
    repeat with n from 2 to count of arguments
        set command to command & " " & quoted form of (item n of arguments)
    end repeat
    do shell script command with administrator privileges
end run"#;

    fn replace_with_permission(app: &Path, bundle: &Path) -> Result<(), String> {
        let folder = bundle.parent().ok_or("the app's folder was not found.")?;
        let temp = temp_folder();
        let owner = fs::metadata(&temp).map_err(|e| format!("{e}."))?.uid();
        let output = Command::new("/usr/bin/osascript")
            .args(["-e", ASK, SCRIPT])
            .arg(app)
            .arg(bundle)
            .arg(folder.join(STAGED))
            .arg(temp.join(OLD))
            .arg(owner.to_string())
            .output()
            .map_err(|e| format!("{e}."))?;
        if output.status.success() {
            return Ok(());
        }
        // The prompt cancelled (AppleScript's error -128): nothing to add
        // to "Nothing was installed."
        if String::from_utf8_lossy(&output.stderr).contains("-128") {
            return Err(String::new());
        }
        Err(format!(
            "the new version couldn't be put in place{}.",
            said(&output)
        ))
    }

    // The bundle that stepped aside, a copy that never took its place, and
    // disk images, from this run's update or one cut short. A copy that
    // installed and was not relaunched is still running from the old
    // bundle, but it is only removed when a new copy starts.
    pub fn clean_up() {
        let temp = temp_folder();
        let _ = fs::remove_dir_all(temp.join(OLD));
        if let Some(folder) = bundle().as_deref().and_then(Path::parent) {
            for name in [STAGED, ASIDE] {
                let _ = fs::remove_dir_all(folder.join(name));
            }
        }
        for entry in fs::read_dir(&temp).into_iter().flatten().flatten() {
            if entry.path().extension().is_some_and(|e| e == "dmg") {
                let _ = fs::remove_file(entry.path());
            }
        }
        // Gone only when empty.
        let _ = fs::remove_dir(&temp);
    }

    // Through Launch Services, as Finder opens it, and as a new copy
    // (-n): this one is still closing, and would only be brought forward.
    // Launch Services passes no environment on, so the developer's
    // variables are named (a rehearsal's copy must stay on its own data).
    pub fn relaunch() {
        let Some(bundle) = bundle() else {
            return;
        };
        let mut open = Command::new("/usr/bin/open");
        open.arg("-n").arg(&bundle);
        for name in ["WEATHERSPELL_DATA", "WEATHERSPELL_UPDATE_FEED"] {
            if let Ok(value) = std::env::var(name) {
                open.arg("--env").arg(format!("{name}={value}"));
            }
        }
        let _ = open.spawn();
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

        const SAYS: &str = "Contents/Resources/says.txt";

        // A bundle with a real program in it and a file that says which
        // bundle it is, signed ad hoc as make-app.sh signs the real one.
        fn app(at: &Path, says: &str) {
            fs::create_dir_all(at.join("Contents/MacOS")).unwrap();
            fs::create_dir_all(at.join("Contents/Resources")).unwrap();
            fs::copy("/usr/bin/true", at.join("Contents/MacOS/Weatherspell")).unwrap();
            fs::write(at.join(SAYS), says).unwrap();
            fs::write(
                at.join("Contents/Info.plist"),
                r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>io.github.planetlinux98.weatherspell.test</string>
<key>CFBundleExecutable</key><string>Weatherspell</string>
<key>CFBundlePackageType</key><string>APPL</string>
</dict></plist>"#,
            )
            .unwrap();
            let signed = Command::new("/usr/bin/codesign")
                .args(["--force", "--sign", "-"])
                .arg(at)
                .output()
                .unwrap();
            assert!(signed.status.success(), "{signed:?}");
        }

        fn disk_image(folder: &Path, says: &str) -> PathBuf {
            let stage = folder.join("stage");
            app(&stage.join(APP), says);
            let image = folder.join("Weatherspell-1.dmg");
            let made = Command::new("/usr/bin/hdiutil")
                .args([
                    "create",
                    "-quiet",
                    "-volname",
                    "Weatherspell",
                    "-format",
                    "UDZO",
                ])
                .arg("-srcfolder")
                .arg(&stage)
                .arg(&image)
                .output()
                .unwrap();
            assert!(made.status.success(), "{made:?}");
            image
        }

        fn says(bundle: &Path) -> String {
            fs::read_to_string(bundle.join(SAYS)).unwrap()
        }

        #[test]
        fn the_bundle_is_found_from_its_program() {
            assert_eq!(
                bundle_of(Path::new(
                    "/Applications/Weatherspell.app/Contents/MacOS/Weatherspell"
                )),
                Some(PathBuf::from("/Applications/Weatherspell.app"))
            );
            // A build run from target/release has none.
            assert_eq!(
                bundle_of(Path::new(
                    "/Users/me/weatherspell/target/release/weatherspell"
                )),
                None
            );
            assert!(translocated(Path::new(
                "/private/var/folders/ab/cd/T/AppTranslocation/1234-ABCD/d/Weatherspell.app"
            )));
            assert!(!translocated(Path::new("/Applications/Weatherspell.app")));
        }

        #[test]
        fn the_app_in_the_disk_image_takes_the_bundles_place() {
            let folder = folder("swap");
            let image = disk_image(&folder, "new");
            let bundle = folder.join("Applications").join(APP);
            app(&bundle, "old");
            install_from(&image, &bundle, false).unwrap();
            assert_eq!(says(&bundle), "new");
            // Nothing beside it, and the old one waiting to be removed.
            assert_eq!(fs::read_dir(bundle.parent().unwrap()).unwrap().count(), 1);
            let aside = temp_folder().join(OLD);
            assert_eq!(says(&aside), "old");
            let verified = Command::new("/usr/bin/codesign")
                .args(["--verify", "--deep", "--strict"])
                .arg(&bundle)
                .output()
                .unwrap();
            assert!(verified.status.success(), "{verified:?}");
            let _ = fs::remove_dir_all(&aside);
            let _ = fs::remove_dir_all(&folder);
        }

        #[test]
        fn an_app_changed_after_signing_is_not_installed() {
            let folder = folder("tampered");
            let stage = folder.join("stage");
            app(&stage.join(APP), "new");
            fs::write(stage.join(APP).join(SAYS), "other").unwrap();
            let image = folder.join("Weatherspell-1.dmg");
            let made = Command::new("/usr/bin/hdiutil")
                .args([
                    "create",
                    "-quiet",
                    "-volname",
                    "Weatherspell",
                    "-format",
                    "UDZO",
                ])
                .arg("-srcfolder")
                .arg(&stage)
                .arg(&image)
                .output()
                .unwrap();
            assert!(made.status.success(), "{made:?}");
            let bundle = folder.join("Applications").join(APP);
            app(&bundle, "old");
            let refused = install_from(&image, &bundle, false).unwrap_err();
            assert!(refused.contains("signature check"), "{refused}");
            assert_eq!(says(&bundle), "old");
            let _ = fs::remove_dir_all(&folder);
        }

        #[test]
        fn a_file_that_is_no_disk_image_leaves_the_bundle_as_it_was() {
            let folder = folder("broken");
            let image = folder.join("Weatherspell-1.dmg");
            fs::write(&image, "not a disk image").unwrap();
            let bundle = folder.join("Applications").join(APP);
            app(&bundle, "old");
            assert!(install_from(&image, &bundle, false).is_err());
            assert_eq!(says(&bundle), "old");
            let _ = fs::remove_dir_all(&folder);
        }
    }
}
