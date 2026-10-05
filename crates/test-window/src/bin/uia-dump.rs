// Dumps what a native UI Automation client (Narrator, NVDA's UIA side)
// sees in every visible top-level window of a process: control type,
// name, class, the patterns that matter here, and which provider answered
// (a Win32 proxy, the control's own, or ours). Dump-A11y's UIA half goes
// through .NET's managed client, whose proxies for Win32 controls are not
// the ones Narrator gets.
//
//     uia-dump [process name, default test-window]

#[cfg(windows)]
fn main() -> windows::core::Result<()> {
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
    };
    use windows::Win32::System::Variant::{VARIANT, VT_BSTR};
    use windows::Win32::UI::Accessibility::*;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
    };
    use windows::core::BOOL;

    fn text(v: &VARIANT) -> String {
        unsafe {
            if v.Anonymous.Anonymous.vt == VT_BSTR {
                v.Anonymous.Anonymous.Anonymous.bstrVal.to_string()
            } else {
                String::new()
            }
        }
    }

    fn short(s: &str, max: usize) -> String {
        let one_line = s.replace(['\r', '\n'], " ");
        if one_line.chars().count() > max {
            one_line.chars().take(max).collect::<String>() + "..."
        } else {
            one_line
        }
    }

    // The provider description names each provider in brackets; the
    // last "Main" one is what answered for the element itself.
    fn provider(e: &IUIAutomationElement) -> String {
        let d = unsafe { e.GetCurrentPropertyValue(UIA_ProviderDescriptionPropertyId) }
            .map(|v| text(&v))
            .unwrap_or_default();
        let mut found = Vec::new();
        for part in d.split(['[', ']', ';']) {
            if let Some(name) = part.trim().strip_prefix("Main:") {
                found.push(name.trim().to_string());
            }
            if let Some(name) = part.trim().strip_prefix("Hwnd(parent link):") {
                found.push(format!("hwnd: {}", name.trim()));
            }
        }
        found.join(" + ")
    }

    unsafe fn dump(walker: &IUIAutomationTreeWalker, e: &IUIAutomationElement, depth: usize) {
        unsafe {
            let kind = e
                .CurrentLocalizedControlType()
                .map(|b| b.to_string())
                .unwrap_or_default();
            let name = e.CurrentName().map(|b| b.to_string()).unwrap_or_default();
            let class = e
                .CurrentClassName()
                .map(|b| b.to_string())
                .unwrap_or_default();
            let mut facts = Vec::new();
            if e.CurrentHasKeyboardFocus().is_ok_and(|b| b.as_bool()) {
                facts.push("FOCUSED".to_string());
            }
            if let Ok(value) =
                e.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
            {
                facts.push(format!(
                    "value=\"{}\"",
                    short(
                        &value
                            .CurrentValue()
                            .map(|b| b.to_string())
                            .unwrap_or_default(),
                        50
                    )
                ));
            }
            if let Ok(t) = e.GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId) {
                let start = t
                    .DocumentRange()
                    .and_then(|r| r.GetText(40))
                    .map(|b| b.to_string())
                    .unwrap_or_default();
                facts.push(format!("TEXT PATTERN \"{}\"", short(&start, 40)));
            }
            if e.GetCurrentPatternAs::<IUIAutomationExpandCollapsePattern>(
                UIA_ExpandCollapsePatternId,
            )
            .is_ok()
            {
                facts.push("expand/collapse".to_string());
            }
            if let Ok(t) = e.GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId)
            {
                facts.push(format!(
                    "toggle={}",
                    t.CurrentToggleState().map(|s| s.0).unwrap_or(-1)
                ));
            }
            if let Ok(k) = e.CurrentAcceleratorKey()
                && !k.is_empty()
            {
                facts.push(format!("accelerator={k}"));
            }
            if let Ok(k) = e.CurrentAccessKey()
                && !k.is_empty()
            {
                facts.push(format!("access key={k}"));
            }
            println!(
                "{}{kind} \"{name}\" class={class} {} <{}>",
                "  ".repeat(depth),
                facts.join(" "),
                provider(e)
            );
            let mut child = walker.GetFirstChildElement(e).ok();
            while let Some(c) = child {
                dump(walker, &c, depth + 1);
                child = walker.GetNextSiblingElement(&c).ok();
            }
        }
    }

    let process = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "test-window".to_string());
    let pids: Vec<u32> = std::process::Command::new("tasklist")
        .args([
            "/FO",
            "CSV",
            "/NH",
            "/FI",
            &format!("IMAGENAME eq {process}.exe"),
        ])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|l| l.split("\",\"").nth(1)?.parse().ok())
                .collect()
        })
        .unwrap_or_default();
    if pids.is_empty() {
        eprintln!("No {process} process found.");
        std::process::exit(1);
    }

    unsafe extern "system" fn collect(hwnd: HWND, data: LPARAM) -> BOOL {
        unsafe {
            let found = &mut *(data.0 as *mut (Vec<u32>, Vec<HWND>));
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if found.0.contains(&pid) && IsWindowVisible(hwnd).as_bool() {
                found.1.push(hwnd);
            }
            true.into()
        }
    }

    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        let walker = automation.ControlViewWalker()?;
        let mut found = (pids, Vec::new());
        let _ = EnumWindows(Some(collect), LPARAM(&mut found as *mut _ as isize));
        for hwnd in found.1 {
            let e = automation.ElementFromHandle(hwnd)?;
            println!("== window {:?}", hwnd.0);
            dump(&walker, &e, 0);
        }
        if let Ok(focus) = automation.GetFocusedElement() {
            let kind = focus
                .CurrentLocalizedControlType()
                .map(|b| b.to_string())
                .unwrap_or_default();
            let name = focus
                .CurrentName()
                .map(|b| b.to_string())
                .unwrap_or_default();
            println!("\nFocus: {kind} \"{name}\" <{}>", provider(&focus));
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn main() {}
