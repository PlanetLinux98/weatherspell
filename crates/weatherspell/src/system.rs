// What the app asks the system: where its files live, the region's unit
// system and time format, and this PC's time zone. Windows for now; the
// Mac and Linux get theirs in their own steps.

use std::path::PathBuf;

use jiff::tz::TimeZone;
use weatherspell_core::clock::TimeFormat;
use weatherspell_core::units::UnitSystem;

// 0.1's folder and files: each app reads what the other writes (the core's
// reference tests check both ways), so moving between them keeps saved
// locations, nicknames and seen alerts. The previews before the switch had
// a folder of their own, "Weatherspell Preview", which is no longer read.
const FOLDER: &str = "Weatherspell";

pub fn data_folder() -> PathBuf {
    // Developer-only, like WEATHERSPELL_OFFLINE: another folder, so a first
    // run can be rehearsed without touching the real settings.
    if let Some(folder) = std::env::var_os("WEATHERSPELL_DATA").filter(|v| !v.is_empty()) {
        return PathBuf::from(folder);
    }
    app_data().join(FOLDER)
}

// One copy per Windows session: two would each announce every alert and
// save settings.json over each other's changes (#22). A second launch
// brings the first one's window forward instead, and ends. 0.1 uses the
// same name, since the two share settings.json, and both look for the
// other by exe name, as 0.1 does (Program.cs), so either brings the other
// forward.
pub fn another_copy_brought_forward() -> bool {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
        use windows::Win32::System::Threading::CreateMutexW;
        use windows::Win32::UI::WindowsAndMessaging::{
            IsIconic, SW_RESTORE, SetForegroundWindow, ShowWindow,
        };
        use windows::core::w;
        // Held until this copy ends, when Windows closes it.
        if CreateMutexW(None, true, w!("Local\\Weatherspell")).is_err()
            || GetLastError() != ERROR_ALREADY_EXISTS
        {
            return false;
        }
        if let Some(window) = other_copy_window() {
            if IsIconic(window).as_bool() {
                let _ = ShowWindow(window, SW_RESTORE);
            }
            let _ = SetForegroundWindow(window);
        }
        true
    }
    #[cfg(not(windows))]
    false
}

// The other copy's main window, as .NET's MainWindowHandle finds it: the
// first visible top-level window with no owner (a dialog has one) of a
// process whose exe has this one's name.
#[cfg(windows)]
fn other_copy_window() -> Option<windows::Win32::Foundation::HWND> {
    use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
    use windows::Win32::System::Threading::{
        GetCurrentProcessId, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GW_OWNER, GetWindow, GetWindowThreadProcessId, IsWindowVisible,
    };
    use windows::core::{BOOL, PWSTR};

    fn exe_name(pid: u32) -> Option<String> {
        unsafe {
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut buffer = [0u16; 1024];
            let mut length = buffer.len() as u32;
            let named = QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut length,
            );
            let _ = CloseHandle(process);
            named.ok()?;
            let path = PathBuf::from(String::from_utf16_lossy(&buffer[..length as usize]));
            Some(path.file_name()?.to_string_lossy().to_lowercase())
        }
    }

    struct Search {
        own_pid: u32,
        own_exe: String,
        found: Option<HWND>,
    }

    unsafe extern "system" fn visit(window: HWND, data: LPARAM) -> BOOL {
        unsafe {
            let search = &mut *(data.0 as *mut Search);
            let mut pid = 0;
            GetWindowThreadProcessId(window, Some(&mut pid));
            if pid != search.own_pid
                && IsWindowVisible(window).as_bool()
                && GetWindow(window, GW_OWNER).is_err()
                && exe_name(pid).as_deref() == Some(search.own_exe.as_str())
            {
                search.found = Some(window);
                return false.into();
            }
            true.into()
        }
    }

    unsafe {
        let own_pid = GetCurrentProcessId();
        let mut search = Search {
            own_pid,
            own_exe: exe_name(own_pid)?,
            found: None,
        };
        let _ = EnumWindows(Some(visit), LPARAM(&mut search as *mut Search as isize));
        search.found
    }
}

// Light or dark as the system is set, through wx 3.3's own dark mode on
// Windows (the Mac and GTK follow the system by themselves). wx is made to
// switch the open windows when the system's mode changes. Not while a high
// contrast theme is on: wx does not look for one and would paint its fixed
// dark colours over the theme's, so the system colours are left alone (a
// theme turned on while a dark copy runs shows once it is restarted). Must
// run before the first window. WEATHERSPELL_APPEARANCE, for developers,
// forces "dark" or "light" to see either without changing Windows.
pub fn follow_appearance() {
    use wxdragon::appearance::Appearance;
    let appearance = match std::env::var("WEATHERSPELL_APPEARANCE").ok().as_deref() {
        Some("dark") => Appearance::Dark,
        Some("light") => Appearance::Light,
        _ if high_contrast() => Appearance::Light,
        _ => Appearance::System,
    };
    let _ = wxdragon::app::set_appearance(appearance);
}

fn high_contrast() -> bool {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
        use windows::Win32::UI::WindowsAndMessaging::{
            SPI_GETHIGHCONTRAST, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
        };
        let mut contrast = HIGHCONTRASTW {
            cbSize: size_of::<HIGHCONTRASTW>() as u32,
            ..Default::default()
        };
        SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            contrast.cbSize,
            Some(&mut contrast as *mut _ as *mut std::ffi::c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .is_ok()
            && contrast.dwFlags.contains(HCF_HIGHCONTRASTON)
    }
    #[cfg(not(windows))]
    false
}

// The exe's own icon (resource 1, build.rs) on a window, at the sizes the
// title bar and Alt+Tab take at the window's scale, rather than one size
// scaled. Windows only for now; the Mac gives a window no icon of its own.
pub fn set_window_icon(window: &impl wxdragon::prelude::WxWidget) {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM};
        use windows::Win32::System::LibraryLoader::GetModuleHandleW;
        use windows::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
        use windows::Win32::UI::WindowsAndMessaging::{
            ICON_BIG, ICON_SMALL, IMAGE_ICON, LR_DEFAULTCOLOR, LoadImageW, SM_CXICON, SM_CXSMICON,
            SendMessageW, WM_SETICON,
        };
        use windows::core::PCWSTR;
        let hwnd = HWND(window.get_handle());
        let Ok(module) = GetModuleHandleW(None) else {
            return;
        };
        let dpi = GetDpiForWindow(hwnd);
        for (which, metric) in [(ICON_SMALL, SM_CXSMICON), (ICON_BIG, SM_CXICON)] {
            let size = GetSystemMetricsForDpi(metric, dpi);
            // Resource 1, as MAKEINTRESOURCE names it.
            if let Ok(icon) = LoadImageW(
                Some(HINSTANCE(module.0)),
                PCWSTR(std::ptr::without_provenance(1)),
                IMAGE_ICON,
                size,
                size,
                LR_DEFAULTCOLOR,
            ) {
                SendMessageW(
                    hwnd,
                    WM_SETICON,
                    Some(WPARAM(which as usize)),
                    Some(LPARAM(icon.0 as isize)),
                );
            }
        }
    }
    #[cfg(not(windows))]
    let _ = window;
}

// Before any window is made. The Mac would otherwise offer window tabs
// (View > Show Tab Bar) in an app with one window.
pub fn before_windows() {
    #[cfg(target_os = "macos")]
    unsafe {
        use wx_accessibility::mac;
        mac::id_with_number(
            mac::class(c"NSWindow"),
            c"setAllowsAutomaticWindowTabbing:",
            0,
        );
    }
}

// The region's own ways, as 0.1 read them from .NET's current culture.
pub struct Region {
    pub units: UnitSystem,
    pub time_format: TimeFormat,
    pub zone: TimeZone,
}

pub fn region() -> Region {
    let zone = TimeZone::try_system().unwrap_or(TimeZone::UTC);
    #[cfg(windows)]
    {
        use windows::Win32::Globalization::{
            LOCALE_IMEASURE, LOCALE_S1159, LOCALE_S2359, LOCALE_SSHORTTIME,
        };
        let pattern = locale(LOCALE_SSHORTTIME).unwrap_or_else(|| "h:mm tt".to_string());
        let am = locale(LOCALE_S1159).unwrap_or_default();
        let pm = locale(LOCALE_S2359).unwrap_or_default();
        Region {
            // "1" is the US system; "0", metric.
            units: if locale(LOCALE_IMEASURE).as_deref() == Some("1") {
                UnitSystem::Imperial
            } else {
                UnitSystem::Metric
            },
            time_format: TimeFormat::new(&pattern, &am, &pm),
            zone,
        }
    }
    #[cfg(not(windows))]
    {
        Region {
            units: UnitSystem::Metric,
            time_format: TimeFormat::new("h:mm tt", "AM", "PM"),
            zone,
        }
    }
}

// The user's own setting, overrides included (Region settings in Windows).
#[cfg(windows)]
fn locale(kind: u32) -> Option<String> {
    use windows::Win32::Globalization::GetLocaleInfoEx;
    use windows::core::PCWSTR;
    let mut buffer = [0u16; 100];
    // A null name is the user's default locale.
    let written = unsafe { GetLocaleInfoEx(PCWSTR::null(), kind, Some(&mut buffer)) };
    (written > 1).then(|| String::from_utf16_lossy(&buffer[..written as usize - 1]))
}

// %APPDATA%: settings that roam with the user, as 0.1 keeps them.
#[cfg(windows)]
fn app_data() -> PathBuf {
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{
        FOLDERID_RoamingAppData, KF_FLAG_DEFAULT, SHGetKnownFolderPath,
    };
    unsafe {
        match SHGetKnownFolderPath(&FOLDERID_RoamingAppData, KF_FLAG_DEFAULT, None) {
            Ok(path) => {
                let folder = PathBuf::from(path.to_string().unwrap_or_default());
                CoTaskMemFree(Some(path.0 as *const _));
                folder
            }
            Err(_) => std::env::var_os("APPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir),
        }
    }
}

#[cfg(not(windows))]
fn app_data() -> PathBuf {
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(".config"))
        .unwrap_or_else(std::env::temp_dir)
}
