// Prints every UI Automation notification raised on the desktop, with the
// window it came from, for the given number of seconds (default 60): a
// check that an announcement leaves the app, before a screen reader is
// asked to speak it. Also says whether that window answers UI Automation
// itself, which is what NVDA asks before trusting a window's events.
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
        println!("Listening for {seconds} seconds.");
        std::thread::sleep(std::time::Duration::from_secs(seconds));
        automation.RemoveNotificationEventHandler(&root, &handler)?;
    }
    Ok(())
}

#[cfg(not(windows))]
fn main() {}
