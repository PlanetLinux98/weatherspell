// Speaks a line without moving focus: a status a screen reader user would
// otherwise miss, or a new alert. On Windows this is a UI Automation
// notification, as the C# app raised with RaiseAutomationNotification.
// wxWidgets has no UI Automation at all, so the notification rides on a
// provider of our own that answers nothing but its window: UI Automation
// fills in the rest from the window itself (UiaHostProviderFromHwnd).
//
// The provider is never handed out on WM_GETOBJECT. A window that
// advertises one makes NVDA read it through UI Automation instead of MSAA,
// which the test window showed reads worse, and Narrator hears the
// notifications either way (NOTES.md, "Rust and wxWidgets").
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
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Variant::VARIANT;
    use windows::Win32::UI::Accessibility::*;
    use windows::core::{BSTR, Error, IUnknown, Result, implement};

    #[implement(IRawElementProviderSimple)]
    struct Source {
        hwnd: HWND,
    }

    impl IRawElementProviderSimple_Impl for Source_Impl {
        fn ProviderOptions(&self) -> Result<ProviderOptions> {
            Ok(ProviderOptions_ServerSideProvider)
        }

        // S_OK with no object: no patterns.
        fn GetPatternProvider(&self, _: UIA_PATTERN_ID) -> Result<IUnknown> {
            Err(Error::empty())
        }

        // An empty VARIANT: every property comes from the host window.
        fn GetPropertyValue(&self, _: UIA_PROPERTY_ID) -> Result<VARIANT> {
            Ok(VARIANT::default())
        }

        fn HostRawElementProvider(&self) -> Result<IRawElementProviderSimple> {
            unsafe { UiaHostProviderFromHwnd(self.hwnd) }
        }
    }

    pub struct Provider {
        source: IRawElementProviderSimple,
        activity: BSTR,
    }

    impl Provider {
        pub fn new(hwnd: *mut std::ffi::c_void, app_name: &str) -> Provider {
            Provider {
                source: Source { hwnd: HWND(hwnd) }.into(),
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
