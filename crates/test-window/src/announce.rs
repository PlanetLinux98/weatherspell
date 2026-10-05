// Speaks a line through UI Automation without moving focus, as the C#
// app's Announcer does with RaiseAutomationNotification. wxWidgets has no
// UI Automation at all, so the notification rides on a provider of our
// own that answers nothing but its window: UI Automation fills in the rest
// from the window itself (UiaHostProviderFromHwnd).
//
// Whether the window should also hand that provider out on WM_GETOBJECT
// is check 5's question. NVDA drops MSAA events from a window that
// advertises a UI Automation provider unless it knows the window's class
// (the C# app's combo box problem, NOTES.md), so by default it does not;
// "advertise" turns it on to compare.

use std::cell::Cell;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::WM_GETOBJECT;
use windows::core::{BSTR, Error, IUnknown, Interface, Result, implement};

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

pub struct Announcer {
    hwnd: HWND,
    provider: IRawElementProviderSimple,
    // The subclass's own reference to the provider while advertised.
    subclass_ref: Cell<usize>,
}

const SUBCLASS_ID: usize = 0x5753; // "WS"

impl Announcer {
    pub fn new(hwnd: *mut std::ffi::c_void) -> Announcer {
        let hwnd = HWND(hwnd);
        Announcer {
            hwnd,
            provider: Source { hwnd }.into(),
            subclass_ref: Cell::new(0),
        }
    }

    // A status line: a newer one replaces it.
    pub fn say(&self, text: &str) {
        self.raise(
            NotificationKind_ActionCompleted,
            NotificationProcessing_MostRecent,
            text,
        );
    }

    // An alert must not be dropped behind whatever else is being spoken.
    pub fn alert(&self, text: &str) {
        self.raise(
            NotificationKind_Other,
            NotificationProcessing_ImportantAll,
            text,
        );
    }

    fn raise(&self, kind: NotificationKind, processing: NotificationProcessing, text: &str) {
        // Errors are not worth more than a line on the console: an
        // announcement that cannot be made is one nobody is listening for.
        let result = unsafe {
            UiaRaiseNotificationEvent(
                &self.provider,
                kind,
                processing,
                &BSTR::from(text),
                &BSTR::from("Weatherspell"),
            )
        };
        if let Err(e) = result {
            eprintln!("UiaRaiseNotificationEvent: {e}");
        }
    }

    pub fn advertised(&self) -> bool {
        self.subclass_ref.get() != 0
    }

    // Hands the provider out (or stops) as the window's UI Automation root.
    pub fn advertise(&self, on: bool) {
        if on == self.advertised() {
            return;
        }
        unsafe {
            if on {
                let raw = self.provider.clone().into_raw();
                if SetWindowSubclass(self.hwnd, Some(root_proc), SUBCLASS_ID, raw as usize)
                    .as_bool()
                {
                    self.subclass_ref.set(raw as usize);
                } else {
                    drop(IRawElementProviderSimple::from_raw(raw));
                }
            } else {
                let _ = RemoveWindowSubclass(self.hwnd, Some(root_proc), SUBCLASS_ID);
                drop(IRawElementProviderSimple::from_raw(
                    self.subclass_ref.replace(0) as *mut _,
                ));
                // Tells UI Automation the window's provider is gone.
                UiaReturnRawElementProvider(self.hwnd, WPARAM(0), LPARAM(0), None);
                let _ = UiaDisconnectProvider(&self.provider);
            }
        }
    }
}

// A window destroyed while advertising takes the subclass with it and
// leaks the provider reference it held; the frame only goes at exit.
unsafe extern "system" fn root_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    data: usize,
) -> LRESULT {
    unsafe {
        if msg == WM_GETOBJECT && lparam.0 as i32 == UiaRootObjectId {
            let raw = data as *mut std::ffi::c_void;
            let provider = IRawElementProviderSimple::from_raw_borrowed(&raw).unwrap();
            return UiaReturnRawElementProvider(hwnd, wparam, lparam, provider);
        }
        DefSubclassProc(hwnd, msg, wparam, lparam)
    }
}
