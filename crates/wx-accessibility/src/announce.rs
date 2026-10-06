// Speaks a line without moving focus: a status a screen reader user would
// otherwise miss, or a new alert. On Windows this is a UI Automation
// notification, as the C# app raised with RaiseAutomationNotification.
// wxWidgets has no UI Automation at all, so the notification rides on a
// provider hosted on the window (hosted.rs); Narrator hears it although
// the window never advertises the provider.
//
// Other systems: not yet (NSAccessibility on the Mac, AT-SPI on Linux);
// there an announcement does nothing.

use wxdragon::prelude::WxWidget;

pub struct Announcer {
    #[cfg(windows)]
    provider: windows_impl::Provider,
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
        #[cfg(not(windows))]
        {
            let _ = (window, app_name);
            Announcer {}
        }
    }

    // A status line: a newer one replaces it.
    pub fn say(&self, text: &str) {
        #[cfg(windows)]
        self.provider.raise(false, text);
        #[cfg(not(windows))]
        let _ = text;
    }

    // An alert must not be dropped behind whatever else is being spoken.
    pub fn alert(&self, text: &str) {
        #[cfg(windows)]
        self.provider.raise(true, text);
        #[cfg(not(windows))]
        let _ = text;
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
