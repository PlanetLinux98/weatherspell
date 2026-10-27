// What the app asks the system: where its files live, the region's unit
// system and time format, and this computer's time zone.

use std::path::PathBuf;

use jiff::tz::TimeZone;
use weatherspell_core::clock::TimeFormat;
use weatherspell_core::units::UnitSystem;

// 0.1's folder and files: each app reads what the other writes (the core's
// reference tests check both ways), so moving between them keeps saved
// locations, nicknames and seen alerts. The previews before the switch had
// a folder of their own, "Weatherspell Preview", which is no longer read.
#[cfg(any(windows, target_os = "macos"))]
const FOLDER: &str = "Weatherspell";
// Lower case on Linux, as the program's own name and most of the desktop's
// folders are (Elliott, 2026-10-12). No Linux copy has shipped, so nothing
// reads the capitalized one of the port's own testing.
#[cfg(not(any(windows, target_os = "macos")))]
const FOLDER: &str = "weatherspell";

pub fn data_folder() -> PathBuf {
    // Developer-only, like WEATHERSPELL_OFFLINE: another folder, so a first
    // run can be rehearsed without touching the real settings.
    if let Some(folder) = std::env::var_os("WEATHERSPELL_DATA").filter(|v| !v.is_empty()) {
        return PathBuf::from(folder);
    }
    app_data().join(FOLDER)
}

// The forecasts kept for when a fetch fails, and the Bureau of
// Meteorology's warning areas (a folder of their own in it). Windows keeps
// them in the app's folder, as 0.1 does; the Mac in its Caches folder and
// Linux in XDG's cache folder, which are for whatever can be fetched
// again.
pub fn cache_folder() -> PathBuf {
    #[cfg(not(windows))]
    let own = std::env::var_os("WEATHERSPELL_DATA").is_some_and(|v| !v.is_empty());
    #[cfg(target_os = "macos")]
    if !own && let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join("Library/Caches").join(FOLDER);
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    if !own {
        return xdg("XDG_CACHE_HOME", ".cache").join(FOLDER);
    }
    data_folder().join(weatherspell_core::cache::FOLDER_NAME)
}

// Whether the app can place its windows and learn where they are: not on
// Wayland, which leaves the place to the desktop (an app sets only its
// size and whether it is maximized). Asked of GDK rather than the session,
// since GDK_BACKEND=x11 runs the app through XWayland, where X11's rules
// hold. Asked once GTK is up, as the main window is built.
pub fn places_windows() -> bool {
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        use std::ffi::{CStr, c_char, c_void};
        #[link(name = "gdk-3")]
        unsafe extern "C" {
            fn gdk_display_get_default() -> *mut c_void;
        }
        #[link(name = "gobject-2.0")]
        unsafe extern "C" {
            fn g_type_name_from_instance(instance: *mut c_void) -> *const c_char;
        }
        unsafe {
            let display = gdk_display_get_default();
            display.is_null()
                || CStr::from_ptr(g_type_name_from_instance(display)).to_bytes()
                    != b"GdkWaylandDisplay"
        }
    }
    #[cfg(any(windows, target_os = "macos"))]
    true
}

// GNOME's Large Text (GTK's text scaling) or interface font changed while
// the app is open: GTK draws every control's text at the new size at once,
// but wx kept each control's size, so labels were cut off (2026-10-13).
// `changed` runs once GTK has taken the change in. Windows and the Mac
// need nothing here: the Mac's own text size is the app's View menu.
pub fn on_text_scale_change(changed: fn()) {
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        use std::ffi::{c_char, c_ulong, c_void};
        #[link(name = "gtk-3")]
        unsafe extern "C" {
            fn gtk_settings_get_default() -> *mut c_void;
        }
        #[link(name = "gobject-2.0")]
        unsafe extern "C" {
            fn g_signal_connect_data(
                instance: *mut c_void,
                signal: *const c_char,
                handler: *const c_void,
                data: *mut c_void,
                destroy: *const c_void,
                flags: u32,
            ) -> c_ulong;
        }
        unsafe extern "C" fn notify(
            _settings: *mut c_void,
            _property: *mut c_void,
            data: *mut c_void,
        ) {
            let changed: fn() = unsafe { std::mem::transmute(data) };
            // After GTK's own handlers, so the new size is what wx measures.
            crate::on_ui_thread(changed);
        }
        unsafe {
            let settings = gtk_settings_get_default();
            if settings.is_null() {
                return;
            }
            for signal in [c"notify::gtk-xft-dpi", c"notify::gtk-font-name"] {
                g_signal_connect_data(
                    settings,
                    signal.as_ptr(),
                    notify as *const c_void,
                    changed as *mut c_void,
                    std::ptr::null(),
                    0,
                );
            }
        }
    }
    #[cfg(any(windows, target_os = "macos"))]
    let _ = changed;
}

// Has a control measure itself again, its text drawn at a new size. wx
// works out a control's best size once and keeps it, forgetting it only
// when wx itself changes the font; wxDragon does not bind wx's
// InvalidateBestSize, so it is called by its C++ name (a plain member
// function: the window first, as GCC's ABI passes it). wxDragon's
// pointer is the wxWindow, whose wxWindowBase is at the same address.
pub fn forget_best_size(window: &dyn wxdragon::prelude::WxWidget) {
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        unsafe extern "C" {
            #[link_name = "_ZN12wxWindowBase18InvalidateBestSizeEv"]
            fn invalidate_best_size(window: *mut std::ffi::c_void);
        }
        unsafe { invalidate_best_size(window.handle_ptr().cast()) }
    }
    #[cfg(any(windows, target_os = "macos"))]
    let _ = window;
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
// scaled. The Mac gives a window no icon of its own. On Linux, X11
// desktops show the window's icon; GNOME on Wayland ignores it and takes
// the icon of the desktop entry whose StartupWMClass is the program's name.
pub fn set_window_icon(window: &wxdragon::prelude::Frame) {
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
        use wxdragon::prelude::WxWidget;
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
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        use wxdragon::prelude::{BitmapBundle, Size};
        // One size, which the window manager scales: 128 pixels covers
        // Alt+Tab at twice the scale.
        let size = Size::new(128, 128);
        if let Some(icon) =
            BitmapBundle::from_svg_data(crate::about::LOGO, size).and_then(|b| b.get_bitmap(size))
        {
            window.set_icon(&icon);
        }
    }
    #[cfg(target_os = "macos")]
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
    #[cfg(not(any(windows, target_os = "macos")))]
    free_section_keys();
}

// wxGTK lets the focused control have a key before the menus' shortcuts,
// and GTK's text view (the forecast) binds Ctrl+PageDown and Ctrl+PageUp
// (a horizontal page) and Ctrl+Shift+A (select nothing) itself, so the
// section keys did nothing there (Elliott, 2026-10-08). Taken from every
// text view the app makes; the Shift forms, which select, stay.
#[cfg(not(any(windows, target_os = "macos")))]
fn free_section_keys() {
    use std::ffi::c_void;
    #[link(name = "gtk-3")]
    unsafe extern "C" {
        fn gtk_text_view_get_type() -> usize;
        fn gtk_binding_set_by_class(class: *mut c_void) -> *mut c_void;
        fn gtk_binding_entry_remove(set: *mut c_void, keyval: u32, modifiers: u32);
    }
    #[link(name = "gobject-2.0")]
    unsafe extern "C" {
        fn g_type_class_ref(gtype: usize) -> *mut c_void;
    }
    const SHIFT: u32 = 1 << 0;
    const CONTROL: u32 = 1 << 2;
    const KEYS: [(u32, u32); 5] = [
        (0xff55, CONTROL),        // Page_Up
        (0xff56, CONTROL),        // Page_Down
        (0xff9a, CONTROL),        // KP_Page_Up
        (0xff9b, CONTROL),        // KP_Page_Down
        (0x061, CONTROL | SHIFT), // a
    ];
    unsafe {
        // The class is kept for the life of the app, as GTK keeps it.
        let class = g_type_class_ref(gtk_text_view_get_type());
        let set = gtk_binding_set_by_class(class);
        for (key, modifiers) in KEYS {
            gtk_binding_entry_remove(set, key, modifiers);
        }
    }
}

// A Mac push button keeps its standard height whatever its font, so at a
// bigger text size its text spilled over the bezel (Elliott, 2026-10-07).
// There each one becomes a flexible push button (NSBezelStyleFlexiblePush,
// 2), which grows with its text, as Apple's own tall buttons do. Called
// on a window before it is laid out; at the Mac's own size, and elsewhere,
// nothing changes.
pub fn fit_buttons(window: &dyn wxdragon::prelude::WxWidget) {
    #[cfg(target_os = "macos")]
    unsafe {
        use wx_accessibility::mac::{self, Id};
        fn walk(view: Id, standard: f64) {
            unsafe {
                let children = mac::id(view, c"subviews");
                for i in 0..mac::count(children) {
                    let child = mac::id_with_number(children, c"objectAtIndex:", i as isize);
                    // A check box is a button too, with its box as the
                    // image; a pop-up is one that already grows.
                    if mac::is(child, c"NSButton")
                        && !mac::is(child, c"NSPopUpButton")
                        && mac::integer(child, c"bezelStyle") == 1
                        && mac::integer(child, c"imagePosition") == 0
                        && mac::float(mac::id(child, c"font"), c"pointSize") > standard + 0.5
                    {
                        mac::id_with_number(child, c"setBezelStyle:", 2);
                    }
                    walk(child, standard);
                }
            }
        }
        let mut view: Id = window.get_handle();
        if mac::is(view, c"NSWindow") {
            view = mac::id(view, c"contentView");
        }
        walk(view, mac::float(mac::class(c"NSFont"), c"systemFontSize"));
    }
    #[cfg(not(target_os = "macos"))]
    let _ = window;
}

// A Mac group box's title keeps its own small font whatever the dialog's
// (wx leaves it so). At a chosen text size it is scaled by the same share;
// wx leaves room above the box's controls for the small title only, so the
// answer is the room to add below them, in pixels. Elsewhere the title
// follows the dialog's font already: 0.
pub fn scale_box_title(group: &wxdragon::prelude::StaticBox, percent: i32) -> i32 {
    #[cfg(target_os = "macos")]
    if percent != 100 {
        use wx_accessibility::mac;
        use wxdragon::prelude::WxWidget;
        unsafe {
            let group = group.get_handle();
            let font = mac::id(group, c"titleFont");
            let small = mac::float(mac::class(c"NSFont"), c"smallSystemFontSize");
            let scaled =
                mac::id_with_float(font, c"fontWithSize:", small * f64::from(percent) / 100.0);
            if !font.is_null() && !scaled.is_null() {
                let line = |f| {
                    mac::float(f, c"ascender") - mac::float(f, c"descender")
                        + mac::float(f, c"leading")
                };
                let extra = line(scaled) - line(font);
                mac::id_with(group, c"setTitleFont:", scaled);
                return extra.ceil().max(0.0) as i32;
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (group, percent);
    0
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
    #[cfg(target_os = "macos")]
    unsafe {
        use wx_accessibility::mac;
        let locale = mac::id(mac::class(c"NSLocale"), c"currentLocale");
        // "Metric", "U.S." or "U.K."; only the US system is imperial, as
        // on Windows, where Britain's region is metric. Asked by key: the
        // measurementSystem property is not answered on macOS 15 and later.
        let system = mac::text(mac::id_with(
            locale,
            c"objectForKey:",
            mac::NSLocaleMeasurementSystem,
        ));
        // A formatter's short time follows the user's own settings (the
        // 24-hour switch included), where the locale alone would not.
        let formatter = mac::id(mac::class(c"NSDateFormatter"), c"new");
        mac::id_with_number(formatter, c"setDateStyle:", 0);
        mac::id_with_number(formatter, c"setTimeStyle:", 1);
        let pattern = mac::text(mac::id(formatter, c"dateFormat"));
        let am = mac::text(mac::id(formatter, c"AMSymbol"));
        let pm = mac::text(mac::id(formatter, c"PMSymbol"));
        mac::id(formatter, c"release");
        Region {
            units: if system == "U.S." {
                UnitSystem::Imperial
            } else {
                UnitSystem::Metric
            },
            time_format: if pattern.is_empty() {
                TimeFormat::new("h:mm tt", "AM", "PM")
            } else {
                TimeFormat::from_unicode(&pattern, &am, &pm)
            },
            zone,
        }
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let (units, time_format) = posix_locale();
        Region {
            units,
            time_format,
            zone,
        }
    }
}

// The session's locale (LC_ALL, then LC_MEASUREMENT or LC_TIME, then
// LANG), which GNOME's Formats and KDE's Region and Language settings set.
// Read through a locale of its own: setlocale would change the whole
// app's.
#[cfg(not(any(windows, target_os = "macos")))]
fn posix_locale() -> (UnitSystem, TimeFormat) {
    use std::ffi::CStr;
    // glibc's _NL_MEASUREMENT_MEASUREMENT, which the libc crate does not
    // name: one byte, 1 for metric, 2 for the US system.
    const MEASUREMENT: libc::nl_item = libc::LC_MEASUREMENT << 16;
    let mask = libc::LC_TIME_MASK | libc::LC_MEASUREMENT_MASK;
    unsafe {
        let mut locale = libc::newlocale(mask, c"".as_ptr(), std::ptr::null_mut());
        // A locale that is named but not installed: C's, as setlocale
        // would fall back to.
        if locale.is_null() {
            locale = libc::newlocale(mask, c"C".as_ptr(), std::ptr::null_mut());
        }
        let item = |item| {
            let text = libc::nl_langinfo_l(item, locale);
            if text.is_null() {
                String::new()
            } else {
                CStr::from_ptr(text).to_string_lossy().into_owned()
            }
        };
        let units = if item(MEASUREMENT).as_bytes().first() == Some(&2) {
            UnitSystem::Imperial
        } else {
            UnitSystem::Metric
        };
        let time_format = TimeFormat::from_posix(
            &item(libc::T_FMT),
            &item(libc::T_FMT_AMPM),
            &item(libc::AM_STR),
            &item(libc::PM_STR),
            desktop_twelve_hour(),
        );
        if !locale.is_null() {
            libc::freelocale(locale);
        }
        (units, time_format)
    }
}

// The desktop's own 12 or 24-hour switch, where it keeps one apart from the
// locale: GNOME's Time Format (Settings, Date & Time), which its top bar
// and its own apps follow, and Cinnamon's. Elsewhere None, and the locale
// decides: KDE, Xfce and the rest keep the choice there. Each read only if
// its schema is installed (GSettings ends the app over a missing one).
#[cfg(not(any(windows, target_os = "macos")))]
fn desktop_twelve_hour() -> Option<bool> {
    use std::ffi::{CStr, c_char, c_void};
    #[link(name = "gio-2.0")]
    unsafe extern "C" {
        fn g_settings_schema_source_get_default() -> *mut c_void;
        fn g_settings_schema_source_lookup(
            source: *mut c_void,
            id: *const c_char,
            recursive: i32,
        ) -> *mut c_void;
        fn g_settings_schema_has_key(schema: *mut c_void, key: *const c_char) -> i32;
        fn g_settings_schema_unref(schema: *mut c_void);
        fn g_settings_new_full(
            schema: *mut c_void,
            backend: *mut c_void,
            path: *const c_char,
        ) -> *mut c_void;
        fn g_settings_get_string(settings: *mut c_void, key: *const c_char) -> *mut c_char;
        fn g_settings_get_boolean(settings: *mut c_void, key: *const c_char) -> i32;
    }
    #[link(name = "glib-2.0")]
    unsafe extern "C" {
        fn g_free(memory: *mut c_void);
    }
    #[link(name = "gobject-2.0")]
    unsafe extern "C" {
        fn g_object_unref(object: *mut c_void);
    }
    enum Key {
        Text(&'static CStr, &'static str),
        Switch(&'static CStr, bool),
    }
    let desktops = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let desktops: Vec<&str> = desktops.split(':').collect();
    let (schema_id, key) = if desktops.contains(&"GNOME") {
        // "12h" or "24h".
        (
            c"org.gnome.desktop.interface",
            Key::Text(c"clock-format", "12h"),
        )
    } else if desktops.contains(&"X-Cinnamon") {
        // On for 24 hours.
        (
            c"org.cinnamon.desktop.interface",
            Key::Switch(c"clock-use-24h", false),
        )
    } else {
        return None;
    };
    let name = match key {
        Key::Text(name, _) | Key::Switch(name, _) => name,
    };
    unsafe {
        let source = g_settings_schema_source_get_default();
        if source.is_null() {
            return None;
        }
        let schema = g_settings_schema_source_lookup(source, schema_id.as_ptr(), 1);
        if schema.is_null() {
            return None;
        }
        let mut twelve = None;
        if g_settings_schema_has_key(schema, name.as_ptr()) != 0 {
            let settings = g_settings_new_full(schema, std::ptr::null_mut(), std::ptr::null());
            twelve = Some(match key {
                Key::Text(_, twelve) => {
                    let text = g_settings_get_string(settings, name.as_ptr());
                    let value = CStr::from_ptr(text).to_bytes() == twelve.as_bytes();
                    g_free(text.cast());
                    value
                }
                Key::Switch(_, twelve) => {
                    (g_settings_get_boolean(settings, name.as_ptr()) != 0) == twelve
                }
            });
            g_object_unref(settings);
        }
        g_settings_schema_unref(schema);
        twelve
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

// Application Support, where a Mac app keeps its settings.
#[cfg(target_os = "macos")]
fn app_data() -> PathBuf {
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join("Library/Application Support"))
        .unwrap_or_else(std::env::temp_dir)
}

// XDG's config folder, where Linux apps keep their settings.
#[cfg(not(any(windows, target_os = "macos")))]
fn app_data() -> PathBuf {
    xdg("XDG_CONFIG_HOME", ".config")
}

// An XDG base folder: the variable when it names an absolute path (the
// spec says to ignore a relative one), else its default under HOME.
#[cfg(not(any(windows, target_os = "macos")))]
fn xdg(variable: &str, default: &str) -> PathBuf {
    std::env::var_os(variable)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(default)))
        .unwrap_or_else(std::env::temp_dir)
}
