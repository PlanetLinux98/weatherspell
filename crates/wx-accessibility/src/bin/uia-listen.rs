// Prints every UI Automation notification raised on the desktop, with the
// window it came from, for the given number of seconds (default 60): a
// check that an announcement leaves the app, before a screen reader is
// asked to speak it. Also says whether that window answers UI Automation
// itself, which is what NVDA asks before trusting a window's events, and
// prints each check box's toggle state change.
//
//     uia-listen [seconds]

#[cfg(windows)]
fn main() -> windows::core::Result<()> {
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
    };
    use windows::Win32::UI::Accessibility::*;
    use windows::core::{BSTR, Interface, Ref, Result, implement};

    #[implement(IUIAutomationNotificationEventHandler)]
    struct Printer;

    impl IUIAutomationNotificationEventHandler_Impl for Printer_Impl {
        fn HandleNotificationEvent(
            &self,
            sender: Ref<IUIAutomationElement>,
            kind: NotificationKind,
            processing: NotificationProcessing,
            text: &BSTR,
            activity: &BSTR,
        ) -> Result<()> {
            let mut from = String::from("unknown sender");
            if let Some(element) = sender.as_ref() {
                unsafe {
                    let hwnd = element.CurrentNativeWindowHandle().unwrap_or_default();
                    let pid = element.CurrentProcessId().unwrap_or_default();
                    let class = element
                        .CurrentClassName()
                        .map(|b| b.to_string())
                        .unwrap_or_default();
                    let name = element
                        .CurrentName()
                        .map(|b| b.to_string())
                        .unwrap_or_default();
                    let native = !hwnd.is_invalid() && UiaHasServerSideProvider(hwnd).as_bool();
                    from = format!(
                        "process {pid}, window {:?} class \"{class}\" name \"{name}\", answers UI Automation itself: {native}",
                        hwnd.0
                    );
                }
            }
            println!(
                "\"{text}\" (kind {}, processing {}, activity \"{activity}\") from {from}",
                kind.0, processing.0
            );
            Ok(())
        }
    }

    // A check box's new state, as Narrator learns it: it speaks a toggle
    // from this property change, not from the MSAA state change NVDA reads.
    // Also whether UI Automation takes the sender for the box's own element
    // (the one focus lands on), which a screen reader may compare.
    #[implement(IUIAutomationPropertyChangedEventHandler)]
    struct Toggles {
        automation: IUIAutomation,
    }

    impl IUIAutomationPropertyChangedEventHandler_Impl for Toggles_Impl {
        fn HandlePropertyChangedEvent(
            &self,
            sender: Ref<IUIAutomationElement>,
            _property: UIA_PROPERTY_ID,
            value: &windows::Win32::System::Variant::VARIANT,
        ) -> Result<()> {
            let element = sender.as_ref();
            let name = element
                .and_then(|e| unsafe { e.CurrentName().ok() })
                .map(|b| b.to_string())
                .unwrap_or_default();
            let kind = element
                .and_then(|e| unsafe { e.CurrentLocalizedControlType().ok() })
                .map(|b| b.to_string())
                .unwrap_or_default();
            let state = i32::try_from(value).unwrap_or(-1);
            let state = match state {
                0 => "off",
                1 => "on",
                2 => "indeterminate",
                _ => "unreadable",
            };
            let same = element.is_some_and(|e| unsafe {
                let own = e
                    .CurrentNativeWindowHandle()
                    .and_then(|hwnd| self.automation.ElementFromHandle(hwnd));
                own.and_then(|own| self.automation.CompareElements(e, &own))
                    .is_ok_and(|same| same.as_bool())
            });
            // What a reader gets if it asks the sender rather than trusting
            // the event's value.
            let asked = element
                .and_then(|e| unsafe {
                    e.GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId)
                        .ok()
                })
                .and_then(|p| unsafe { p.CurrentToggleState().ok() })
                .map_or_else(|| "nothing".to_string(), |s| s.0.to_string());
            println!(
                "Toggle state of {kind} \"{name}\" changed to {state} (asked: {asked}); the window's own element: {same}"
            );
            Ok(())
        }
    }

    let seconds: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        let automation: IUIAutomation5 = automation.cast()?;
        let root = automation.GetRootElement()?;
        let handler: IUIAutomationNotificationEventHandler = Printer.into();
        automation.AddNotificationEventHandler(&root, TreeScope_Subtree, None, &handler)?;
        let toggles: IUIAutomationPropertyChangedEventHandler = Toggles {
            automation: automation.clone().into(),
        }
        .into();
        automation.AddPropertyChangedEventHandlerNativeArray(
            &root,
            TreeScope_Subtree,
            None,
            &toggles,
            &[UIA_ToggleToggleStatePropertyId],
        )?;
        println!("Listening for {seconds} seconds.");
        std::thread::sleep(std::time::Duration::from_secs(seconds));
        automation.RemovePropertyChangedEventHandler(&root, &toggles)?;
        automation.RemoveNotificationEventHandler(&root, &handler)?;
    }
    Ok(())
}

#[cfg(not(windows))]
fn main() {}
