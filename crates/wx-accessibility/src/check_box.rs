// wxWidgets 3.3 draws check boxes itself in dark mode on Windows
// (BS_OWNERDRAW), with an accessible object that still reports the role
// and the checked state, but it never says when that state changes: a
// native check box raises the events from BM_SETCHECK, while Windows' own
// events for the press come before wx turns its state over. So both are
// raised here after it: MSAA's state change, which NVDA reads, and UI
// Automation's toggle state change, without which Narrator said nothing
// until focus came back to the box. In light mode the native box raises
// its own, so nothing is added.

use wxdragon::prelude::*;

pub fn report_toggles(check: &CheckBox) {
    #[cfg(windows)]
    windows_impl::report_toggles(check);
    #[cfg(not(windows))]
    let _ = check;
}

#[cfg(windows)]
mod windows_impl {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicI32, Ordering};

    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::System::Variant::VARIANT;
    use windows::Win32::UI::Accessibility::*;
    use windows::Win32::UI::WindowsAndMessaging::{
        BM_CLICK, BS_OWNERDRAW, BS_TYPEMASK, CHILDID_SELF, EVENT_OBJECT_STATECHANGE, GWL_STYLE,
        GetWindowLongW, OBJID_CLIENT, PostMessageW,
    };
    use windows::core::{BSTR, Error, IUnknown, IUnknownImpl, Interface, Result, implement};
    use wxdragon::prelude::*;

    // The event's sender. Hosted on the box's window, it is the box's own
    // element to UI Automation, but the sender's properties come from here
    // and the host window alone, which would make it a pane named with the
    // Alt key's "&" and no state; so it answers those itself. Like the
    // announcer's provider, it is never handed out on WM_GETOBJECT, so the
    // box still reads through wx's own accessible object everywhere else.
    #[implement(IRawElementProviderSimple, IToggleProvider)]
    struct Sender {
        hwnd: isize,
        name: BSTR,
        state: Arc<AtomicI32>,
    }

    impl IRawElementProviderSimple_Impl for Sender_Impl {
        fn ProviderOptions(&self) -> Result<ProviderOptions> {
            Ok(ProviderOptions_ServerSideProvider)
        }

        fn GetPatternProvider(&self, pattern: UIA_PATTERN_ID) -> Result<IUnknown> {
            if pattern == UIA_TogglePatternId {
                let toggle: IToggleProvider = self.to_interface();
                toggle.cast()
            } else {
                Err(Error::empty())
            }
        }

        // An empty VARIANT for the rest: it comes from the host window.
        fn GetPropertyValue(&self, property: UIA_PROPERTY_ID) -> Result<VARIANT> {
            Ok(if property == UIA_ControlTypePropertyId {
                VARIANT::from(UIA_CheckBoxControlTypeId.0)
            } else if property == UIA_NamePropertyId {
                VARIANT::from(self.name.clone())
            } else {
                VARIANT::default()
            })
        }

        fn HostRawElementProvider(&self) -> Result<IRawElementProviderSimple> {
            unsafe { UiaHostProviderFromHwnd(HWND(self.hwnd as *mut _)) }
        }
    }

    impl IToggleProvider_Impl for Sender_Impl {
        // As a press would, on the box's own thread.
        fn Toggle(&self) -> Result<()> {
            unsafe {
                PostMessageW(
                    Some(HWND(self.hwnd as *mut _)),
                    BM_CLICK,
                    WPARAM(0),
                    LPARAM(0),
                )
            }
        }

        fn ToggleState(&self) -> Result<ToggleState> {
            Ok(ToggleState(self.state.load(Ordering::Relaxed)))
        }
    }

    // "Notify me about &alerts" reads "Notify me about alerts"; "&&" is a
    // literal "&".
    fn without_mnemonic(label: &str) -> String {
        let mut out = String::new();
        let mut chars = label.chars();
        while let Some(c) = chars.next() {
            if c == '&' {
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    fn state_of(check: &CheckBox) -> ToggleState {
        if check.is_checked() {
            ToggleState_On
        } else {
            ToggleState_Off
        }
    }

    pub fn report_toggles(check: &CheckBox) {
        let hwnd = check.get_handle() as isize;
        let state = Arc::new(AtomicI32::new(state_of(check).0));
        let sender: IRawElementProviderSimple = Sender {
            hwnd,
            name: BSTR::from(without_mnemonic(&check.get_label().unwrap_or_default())),
            state: state.clone(),
        }
        .into();
        let check_ = *check;
        check.on_toggled(move |_| unsafe {
            let window = HWND(hwnd as *mut _);
            let new = state_of(&check_);
            let old = ToggleState(state.swap(new.0, Ordering::Relaxed));
            if GetWindowLongW(window, GWL_STYLE) & BS_TYPEMASK != BS_OWNERDRAW {
                return;
            }
            NotifyWinEvent(
                EVENT_OBJECT_STATECHANGE,
                window,
                OBJID_CLIENT.0,
                CHILDID_SELF as i32,
            );
            let result = UiaRaiseAutomationPropertyChangedEvent(
                &sender,
                UIA_ToggleToggleStatePropertyId,
                &VARIANT::from(old.0),
                &VARIANT::from(new.0),
            );
            if let Err(e) = result {
                eprintln!("UiaRaiseAutomationPropertyChangedEvent: {e}");
            }
        });
    }

    #[cfg(test)]
    mod tests {
        use super::without_mnemonic;

        #[test]
        fn mnemonics_are_dropped_and_doubled_ampersands_kept() {
            assert_eq!(
                without_mnemonic("Notify me about &alerts for this location"),
                "Notify me about alerts for this location"
            );
            assert_eq!(without_mnemonic("Wind && rain"), "Wind & rain");
            assert_eq!(without_mnemonic("Trailing &"), "Trailing ");
        }
    }
}
