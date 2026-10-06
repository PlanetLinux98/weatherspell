// wxWidgets 3.3 draws check boxes itself in dark mode on Windows
// (BS_OWNERDRAW), with an accessible object that still reports the role
// and the checked state, but it never says when that state changes: a
// native check box raises the event from BM_SETCHECK, while Windows' own
// events for the press come before wx turns its state over. A screen
// reader usually reads the new state anyway, since it asks once the click
// is handled, but only by timing; this raises the missing event after it.
// In light mode the native box raises its own, so nothing is added.

use wxdragon::prelude::*;

pub fn report_toggles(check: &CheckBox) {
    #[cfg(windows)]
    {
        let handle = check.get_handle() as isize;
        check.on_toggled(move |_| unsafe {
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::Accessibility::NotifyWinEvent;
            use windows::Win32::UI::WindowsAndMessaging::{
                BS_OWNERDRAW, BS_TYPEMASK, CHILDID_SELF, EVENT_OBJECT_STATECHANGE, GWL_STYLE,
                GetWindowLongW, OBJID_CLIENT,
            };
            let hwnd = HWND(handle as *mut _);
            if GetWindowLongW(hwnd, GWL_STYLE) & BS_TYPEMASK == BS_OWNERDRAW {
                NotifyWinEvent(
                    EVENT_OBJECT_STATECHANGE,
                    hwnd,
                    OBJID_CLIENT.0,
                    CHILDID_SELF as i32,
                );
            }
        });
    }
    #[cfg(not(windows))]
    let _ = check;
}
