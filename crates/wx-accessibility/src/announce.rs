// Speaks a line without moving focus: a status a screen reader user would
// otherwise miss, or a new alert. On Windows this is a UI Automation
// notification, as the C# app raised with RaiseAutomationNotification.
// wxWidgets has no UI Automation at all, so the notification rides on a
// provider hosted on the window (hosted.rs); Narrator hears it although
// the window never advertises the provider. On the Mac it is AppKit's
// announcement request, which VoiceOver speaks. On Linux it is ATK's
// "notification" signal (ATK 2.50) on the window's accessible, which the
// AT-SPI bridge sends on as an announcement and Orca speaks; GTK 3 has no
// call of its own for it. An ATK older than 2.50 has no such signal and
// GLib warns on the console instead.

use wxdragon::prelude::WxWidget;

pub struct Announcer {
    #[cfg(windows)]
    provider: windows_impl::Provider,
    // The window's view, whose window posts the announcement.
    #[cfg(target_os = "macos")]
    view: crate::mac::Id,
    // The window's GtkWidget, whose accessible makes the announcement.
    #[cfg(not(any(windows, target_os = "macos")))]
    widget: *mut std::ffi::c_void,
}

impl Announcer {
    // On the window the announcements come from (the app's main window,
    // or a dialog while one is open), with the app's name as UI Automation
    // asks for one; screen readers do not say it.
    pub fn new(window: &impl WxWidget, app_name: &str) -> Announcer {
        #[cfg(windows)]
        {
            Announcer {
                provider: windows_impl::Provider::new(window.get_handle(), app_name),
            }
        }
        #[cfg(target_os = "macos")]
        {
            let _ = app_name;
            Announcer {
                view: window.get_handle(),
            }
        }
        #[cfg(not(any(windows, target_os = "macos")))]
        {
            let _ = app_name;
            Announcer {
                widget: window.get_handle(),
            }
        }
    }

    // A status line: a newer one replaces it.
    pub fn say(&self, text: &str) {
        #[cfg(windows)]
        self.provider.raise(false, text);
        #[cfg(target_os = "macos")]
        mac_impl::post(self.view, text);
        #[cfg(not(any(windows, target_os = "macos")))]
        gtk_impl::notify(self.widget, false, text);
    }

    // An alert must not be dropped behind whatever else is being spoken.
    pub fn alert(&self, text: &str) {
        #[cfg(windows)]
        self.provider.raise(true, text);
        #[cfg(target_os = "macos")]
        mac_impl::post(self.view, text);
        #[cfg(not(any(windows, target_os = "macos")))]
        gtk_impl::notify(self.widget, true, text);
    }
}

#[cfg(windows)]
mod windows_impl {
    use windows::Win32::UI::Accessibility::*;
    use windows::core::BSTR;

    pub struct Provider {
        source: IRawElementProviderSimple,
        activity: BSTR,
    }

    impl Provider {
        pub fn new(hwnd: *mut std::ffi::c_void, app_name: &str) -> Provider {
            Provider {
                source: crate::hosted::provider(hwnd),
                activity: BSTR::from(app_name),
            }
        }

        pub fn raise(&self, important: bool, text: &str) {
            let (kind, processing) = if important {
                (NotificationKind_Other, NotificationProcessing_ImportantAll)
            } else {
                (
                    NotificationKind_ActionCompleted,
                    NotificationProcessing_MostRecent,
                )
            };
            // An announcement that cannot be made is one nobody is
            // listening for; a line on the console is all it is worth.
            let result = unsafe {
                UiaRaiseNotificationEvent(
                    &self.source,
                    kind,
                    processing,
                    &BSTR::from(text),
                    &self.activity,
                )
            };
            if let Err(e) = result {
                eprintln!("UiaRaiseNotificationEvent: {e}");
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod mac_impl {
    use crate::mac::{self, Id};

    // NSAccessibilityPriorityHigh: said at once, cutting off whatever is
    // being said, as on Windows a newer status replaces an older one and
    // an alert is never left behind. It also cuts off what VoiceOver says
    // of a caret the app moved itself (a section key), so the heading is
    // what is heard. Lower priorities wait their turn.
    const PRIORITY_HIGH: isize = 90;

    pub fn post(view: Id, text: &str) {
        unsafe {
            let app = mac::id(mac::class(c"NSApplication"), c"sharedApplication");
            let window = if mac::is(view, c"NSView") {
                mac::id(view, c"window")
            } else if mac::is(view, c"NSWindow") {
                view
            } else {
                std::ptr::null_mut()
            };
            let element = if window.is_null() { app } else { window };
            let priority = mac::id_with_number(
                mac::class(c"NSNumber"),
                c"numberWithInteger:",
                PRIORITY_HIGH,
            );
            let info = mac::dictionary(
                &[mac::string(text), priority],
                &[
                    mac::NSAccessibilityAnnouncementKey,
                    mac::NSAccessibilityPriorityKey,
                ],
            );
            if info.is_null() {
                return;
            }
            mac::NSAccessibilityPostNotificationWithUserInfo(
                element,
                mac::NSAccessibilityAnnouncementRequestedNotification,
                info,
            );
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod gtk_impl {
    use std::ffi::{CString, c_char, c_int, c_void};

    // AtkLive. Orca puts an assertive announcement ahead of other events
    // and a polite one after focus changes.
    const ATK_LIVE_POLITE: c_int = 1;
    const ATK_LIVE_ASSERTIVE: c_int = 2;

    // wxGTK links both libraries already.
    #[link(name = "gtk-3")]
    unsafe extern "C" {
        fn gtk_widget_get_accessible(widget: *mut c_void) -> *mut c_void;
    }
    #[link(name = "gobject-2.0")]
    unsafe extern "C" {
        fn g_signal_emit_by_name(instance: *mut c_void, signal: *const c_char, ...);
    }

    pub fn notify(widget: *mut c_void, important: bool, text: &str) {
        let Ok(text) = CString::new(text) else {
            return;
        };
        if widget.is_null() {
            return;
        }
        let live = if important {
            ATK_LIVE_ASSERTIVE
        } else {
            ATK_LIVE_POLITE
        };
        unsafe {
            let accessible = gtk_widget_get_accessible(widget);
            if !accessible.is_null() {
                g_signal_emit_by_name(accessible, c"notification".as_ptr(), text.as_ptr(), live);
            }
        }
    }
}
