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
// On GTK it reads GTK's widgets in the order wx made them, the same rules
// in the same words, except that an input's name is the label linked to
// it (what ATK gives Orca), which wxGTK links only when the window is
// first idle: the lint lets that happen before it looks. The Mac: nothing
// yet; it reports no problems there.

use wxdragon::prelude::WxWidget;

// Each problem as a sentence naming the window and the control.
pub fn check(window: &impl WxWidget) -> Vec<String> {
    #[cfg(windows)]
    {
        windows_impl::check(window.get_handle())
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        gtk_impl::check(window.get_handle())
    }
    #[cfg(target_os = "macos")]
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

#[cfg(not(any(windows, target_os = "macos")))]
mod gtk_impl {
    use std::collections::HashMap;
    use std::ffi::{CStr, c_char, c_int, c_uint, c_void};

    type GType = usize;
    type Widget = *mut c_void;

    #[repr(C)]
    struct GList {
        data: *mut c_void,
        next: *mut GList,
        prev: *mut GList,
    }

    const GDK_KEY_VOID_SYMBOL: c_uint = 0xffffff;

    // wxGTK links all of these already.
    #[link(name = "gtk-3")]
    unsafe extern "C" {
        fn gtk_container_get_type() -> GType;
        fn gtk_scrolled_window_get_type() -> GType;
        fn gtk_label_get_type() -> GType;
        fn gtk_button_get_type() -> GType;
        fn gtk_entry_get_type() -> GType;
        fn gtk_text_view_get_type() -> GType;
        fn gtk_combo_box_get_type() -> GType;
        fn gtk_tree_view_get_type() -> GType;
        fn gtk_menu_bar_get_type() -> GType;
        fn gtk_container_get_children(container: Widget) -> *mut GList;
        fn gtk_bin_get_child(bin: Widget) -> Widget;
        fn gtk_widget_get_visible(widget: Widget) -> c_int;
        fn gtk_widget_get_accessible(widget: Widget) -> *mut c_void;
        fn gtk_window_get_title(window: Widget) -> *const c_char;
        fn gtk_label_get_text(label: Widget) -> *const c_char;
        fn gtk_label_get_mnemonic_keyval(label: Widget) -> c_uint;
        fn gtk_label_get_mnemonic_widget(label: Widget) -> Widget;
    }
    #[link(name = "gdk-3")]
    unsafe extern "C" {
        fn gdk_keyval_to_lower(keyval: c_uint) -> c_uint;
        fn gdk_keyval_to_unicode(keyval: c_uint) -> u32;
    }
    #[link(name = "atk-1.0")]
    unsafe extern "C" {
        fn atk_object_get_name(accessible: *mut c_void) -> *const c_char;
    }
    #[link(name = "gobject-2.0")]
    unsafe extern "C" {
        fn g_type_check_instance_is_a(instance: Widget, gtype: GType) -> c_int;
    }
    #[link(name = "glib-2.0")]
    unsafe extern "C" {
        fn g_list_free(list: *mut GList);
        fn g_main_context_iteration(context: *mut c_void, may_block: c_int) -> c_int;
    }

    unsafe fn is(widget: Widget, gtype: GType) -> bool {
        unsafe { g_type_check_instance_is_a(widget, gtype) != 0 }
    }

    unsafe fn text(s: *const c_char) -> String {
        if s.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(s) }.to_string_lossy().into_owned()
        }
    }

    unsafe fn children(widget: Widget) -> Vec<Widget> {
        let mut out = Vec::new();
        unsafe {
            if !is(widget, gtk_container_get_type()) {
                return out;
            }
            let list = gtk_container_get_children(widget);
            let mut item = list;
            while !item.is_null() {
                out.push((*item).data);
                item = (*item).next;
            }
            g_list_free(list);
        }
        out
    }

    // What a control is to the lint, and what it is called in a problem.
    #[derive(Clone, Copy, PartialEq)]
    enum Kind {
        Input(&'static str),
        Label,
        Button,
        Other,
    }

    unsafe fn kind(widget: Widget) -> Kind {
        unsafe {
            if is(widget, gtk_entry_get_type()) || is(widget, gtk_text_view_get_type()) {
                Kind::Input("text box")
            } else if is(widget, gtk_combo_box_get_type()) {
                Kind::Input("combo box")
            } else if is(widget, gtk_tree_view_get_type()) {
                Kind::Input("list")
            } else if is(widget, gtk_label_get_type()) {
                Kind::Label
            } else if is(widget, gtk_button_get_type()) {
                Kind::Button
            } else {
                Kind::Other
            }
        }
    }

    fn describe(kind: Kind) -> &'static str {
        match kind {
            Kind::Input(name) => name,
            Kind::Label => "text",
            Kind::Button => "button",
            Kind::Other => "control",
        }
    }

    // wx puts a list or a multi-line text box in a scrolled window; the
    // control inside is the one a reader is told about.
    unsafe fn control(widget: Widget) -> Widget {
        unsafe {
            if is(widget, gtk_scrolled_window_get_type()) {
                let inner = gtk_bin_get_child(widget);
                if !inner.is_null() && matches!(kind(inner), Kind::Input(_)) {
                    return inner;
                }
            }
        }
        widget
    }

    // The first label inside a button or menu title: its text and Alt key.
    unsafe fn inner_label(widget: Widget) -> Option<Widget> {
        unsafe {
            if is(widget, gtk_label_get_type()) {
                return Some(widget);
            }
            children(widget).into_iter().find_map(|c| inner_label(c))
        }
    }

    unsafe fn key(label: Widget) -> Option<char> {
        unsafe {
            let keyval = gtk_label_get_mnemonic_keyval(label);
            if keyval == GDK_KEY_VOID_SYMBOL {
                return None;
            }
            char::from_u32(gdk_keyval_to_unicode(gdk_keyval_to_lower(keyval)))
                .filter(|c| *c != '\0')
        }
    }

    struct Lint {
        title: String,
        // Each input and the text of the label linked to it.
        named: HashMap<usize, String>,
        keys: HashMap<char, Vec<String>>,
        problems: Vec<String>,
    }

    pub fn check(top: *mut c_void) -> Vec<String> {
        if top.is_null() {
            return Vec::new();
        }
        // wxGTK links labels to controls when the window is first idle,
        // and only then do inputs have their names. Idle uses up wx's
        // wake-up, which a window closed later needs to be deleted (and
        // the app to end), so it is asked for again.
        for _ in 0..1000 {
            if unsafe { g_main_context_iteration(std::ptr::null_mut(), 0) } == 0 {
                break;
            }
        }
        wxdragon::wake_up_idle();
        let mut lint = Lint {
            title: unsafe { text(gtk_window_get_title(top)) },
            named: HashMap::new(),
            keys: HashMap::new(),
            problems: Vec::new(),
        };
        unsafe {
            links(top, &mut lint.named);
            walk(top, &mut lint);
        }
        let Lint {
            title,
            keys,
            mut problems,
            ..
        } = lint;
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

    unsafe fn links(widget: Widget, named: &mut HashMap<usize, String>) {
        unsafe {
            if is(widget, gtk_label_get_type()) {
                let target = gtk_label_get_mnemonic_widget(widget);
                let shown = text(gtk_label_get_text(widget));
                if !target.is_null() && !shown.trim().is_empty() {
                    named.insert(target as usize, shown);
                }
                return;
            }
            for child in children(widget) {
                links(child, named);
            }
        }
    }

    unsafe fn walk(container: Widget, lint: &mut Lint) {
        unsafe {
            let mut previous: Option<Widget> = None;
            for child in children(container) {
                // A control wx made and hid is not seen by anyone.
                if gtk_widget_get_visible(child) == 0 {
                    continue;
                }
                let widget = control(child);
                let what = kind(widget);
                match what {
                    Kind::Input(_) => {
                        if !lint.named.contains_key(&(widget as usize)) {
                            let title = &lint.title;
                            lint.problems.push(match previous {
                                Some(p) => {
                                    let p = control(p);
                                    let shown = match kind(p) {
                                        Kind::Label => text(gtk_label_get_text(p)),
                                        _ => inner_label(p)
                                            .map(|l| text(gtk_label_get_text(l)))
                                            .unwrap_or_default(),
                                    };
                                    format!(
                                        "{title}: the {} after {} \"{shown}\" has no label just before it, so a screen reader has no name for it.",
                                        describe(what),
                                        describe(kind(p)),
                                    )
                                }
                                None => format!(
                                    "{title}: the first {} has no label before it, so a screen reader has no name for it.",
                                    describe(what)
                                ),
                            });
                        }
                    }
                    Kind::Label => {
                        if let Some(key) = key(widget) {
                            let shown = text(gtk_label_get_text(widget));
                            lint.keys
                                .entry(key)
                                .or_default()
                                .push(format!("\"{shown}\""));
                        }
                    }
                    Kind::Button => {
                        let name = text(atk_object_get_name(gtk_widget_get_accessible(widget)));
                        if name.trim().is_empty() {
                            let title = &lint.title;
                            lint.problems.push(format!(
                                "{title}: a button or check box has no text, so a screen reader has no name for it."
                            ));
                        }
                        if let Some(label) = inner_label(widget)
                            && let Some(key) = key(label)
                        {
                            let shown = text(gtk_label_get_text(label));
                            lint.keys
                                .entry(key)
                                .or_default()
                                .push(format!("\"{shown}\""));
                        }
                    }
                    Kind::Other => {
                        if is(widget, gtk_menu_bar_get_type()) {
                            for title in children(widget) {
                                if let Some(label) = inner_label(title)
                                    && let Some(key) = key(label)
                                {
                                    let shown = text(gtk_label_get_text(label));
                                    lint.keys
                                        .entry(key)
                                        .or_default()
                                        .push(format!("the {shown} menu"));
                                }
                            }
                        } else {
                            walk(widget, lint);
                        }
                    }
                }
                previous = Some(child);
            }
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
