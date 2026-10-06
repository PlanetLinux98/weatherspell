// The accessibility lint: what can be checked mechanically about a window
// and every control in it, for a test to run over each window an app has
// (0.1's AccessibilityLint did the same for WinForms). On Windows it reads
// the native controls themselves, since they are what a screen reader is
// told about, rather than wxWidgets' view of them:
//
// - an input with no text of its own (an edit box, a list, a combo box)
//   has a label just before it among its siblings, which is what names it
//   for a screen reader (and its Alt key, through the label's);
// - a button or check box has text, which is its name;
// - no two Alt keys are the same within one window, the menu bar's
//   included, since a clash takes the user to the wrong control.
//
// Other systems: nothing yet; it reports no problems there.

use wxdragon::prelude::WxWidget;

// Each problem as a sentence naming the window and the control.
pub fn check(window: &impl WxWidget) -> Vec<String> {
    #[cfg(windows)]
    {
        windows_impl::check(window.get_handle())
    }
    #[cfg(not(windows))]
    {
        let _ = window;
        Vec::new()
    }
}

// The Alt key a label gives, if any: the letter after a single "&".
pub fn mnemonic(text: &str) -> Option<char> {
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            match chars.next() {
                Some('&') => {}
                Some(key) => return Some(key.to_ascii_lowercase()),
                None => return None,
            }
        }
    }
    None
}

// The text as it is shown and read: "&&" is "&", a single "&" goes.
pub fn shown(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
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

#[cfg(windows)]
mod windows_impl {
    use std::collections::HashMap;

    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumChildWindows, GW_HWNDPREV, GWL_STYLE, GetClassNameW, GetMenu, GetMenuItemCount,
        GetMenuStringW, GetParent, GetWindow, GetWindowLongW, GetWindowTextW, MF_BYPOSITION,
        WS_VISIBLE,
    };
    use windows::core::BOOL;

    use super::{mnemonic, shown};

    // Inputs a screen reader names from the label before them.
    const INPUTS: [&str; 3] = ["Edit", "ListBox", "ComboBox"];
    const BS_TYPEMASK: i32 = 0xF;
    const BS_GROUPBOX: i32 = 7;

    struct Control {
        class: String,
        text: String,
        style: i32,
    }

    fn control(hwnd: HWND) -> Control {
        let mut class = [0u16; 128];
        let mut text = [0u16; 512];
        unsafe {
            let n = GetClassNameW(hwnd, &mut class) as usize;
            let t = GetWindowTextW(hwnd, &mut text) as usize;
            Control {
                class: String::from_utf16_lossy(&class[..n]),
                text: String::from_utf16_lossy(&text[..t]),
                style: GetWindowLongW(hwnd, GWL_STYLE),
            }
        }
    }

    pub fn check(top: *mut std::ffi::c_void) -> Vec<String> {
        let top = HWND(top);
        let title = control(top).text;
        let mut children: Vec<HWND> = Vec::new();
        unsafe extern "system" fn collect(hwnd: HWND, list: LPARAM) -> BOOL {
            unsafe { (*(list.0 as *mut Vec<HWND>)).push(hwnd) };
            BOOL(1)
        }
        unsafe {
            let _ = EnumChildWindows(
                Some(top),
                Some(collect),
                LPARAM(&mut children as *mut _ as isize),
            );
        }

        let mut problems = Vec::new();
        // Alt keys and what gives them, menu titles first.
        let mut keys: HashMap<char, Vec<String>> = HashMap::new();
        unsafe {
            let menu = GetMenu(top);
            if !menu.is_invalid() {
                for i in 0..GetMenuItemCount(Some(menu)).max(0) {
                    let mut text = [0u16; 128];
                    let n = GetMenuStringW(menu, i as u32, Some(&mut text), MF_BYPOSITION) as usize;
                    let text = String::from_utf16_lossy(&text[..n]);
                    if let Some(key) = mnemonic(&text) {
                        keys.entry(key)
                            .or_default()
                            .push(format!("the {} menu", shown(&text)));
                    }
                }
            }
        }

        for hwnd in children {
            let c = control(hwnd);
            // A control of wx's own that never shows (one made and hidden)
            // is not seen by anyone.
            if c.style & WS_VISIBLE.0 as i32 == 0 {
                continue;
            }
            let parent = unsafe { GetParent(hwnd) }.map(control).ok();
            // The edit box inside a combo box is the combo box's.
            if parent.as_ref().is_some_and(|p| p.class == "ComboBox") {
                continue;
            }
            match c.class.as_str() {
                class if INPUTS.contains(&class) => {
                    let label = unsafe { GetWindow(hwnd, GW_HWNDPREV) }.ok().map(control);
                    match label {
                        Some(l) if l.class == "Static" && !shown(&l.text).trim().is_empty() => {}
                        Some(l) => problems.push(format!(
                            "{title}: the {} after {} \"{}\" has no label just before it, so a screen reader has no name for it.",
                            describe(&c.class),
                            describe(&l.class),
                            shown(&l.text)
                        )),
                        None => problems.push(format!(
                            "{title}: the first {} has no label before it, so a screen reader has no name for it.",
                            describe(&c.class)
                        )),
                    }
                }
                "Button"
                    if c.style & BS_TYPEMASK != BS_GROUPBOX && shown(&c.text).trim().is_empty() =>
                {
                    problems.push(format!(
                        "{title}: a button or check box has no text, so a screen reader has no name for it."
                    ));
                }
                _ => {}
            }
            if (c.class == "Static" || c.class == "Button")
                && let Some(key) = mnemonic(&c.text)
            {
                keys.entry(key)
                    .or_default()
                    .push(format!("\"{}\"", shown(&c.text)));
            }
        }

        let mut clashes: Vec<_> = keys.into_iter().filter(|(_, by)| by.len() > 1).collect();
        clashes.sort();
        for (key, by) in clashes {
            problems.push(format!(
                "{title}: Alt+{} is given by {}.",
                key.to_ascii_uppercase(),
                by.join(" and ")
            ));
        }
        problems
    }

    fn describe(class: &str) -> &str {
        match class {
            "Edit" => "text box",
            "ListBox" => "list",
            "ComboBox" => "combo box",
            "Static" => "text",
            "Button" => "button",
            _ => "control",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alt_keys_are_read_as_windows_reads_them() {
        assert_eq!(mnemonic("L&ocation"), Some('o'));
        assert_eq!(mnemonic("Move &Up"), Some('u'));
        assert_eq!(mnemonic("Fish && chips"), None);
        assert_eq!(mnemonic("&& &Cheese"), Some('c'));
        assert_eq!(mnemonic("Forecast"), None);
        assert_eq!(shown("Rena&me && more"), "Rename & more");
    }
}
