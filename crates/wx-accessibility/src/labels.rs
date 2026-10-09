// An input's name is the label just before it (the lint's first rule).
// Windows' screen readers find that label themselves, from the window
// order; VoiceOver does not, and reads an unnamed "edit text" or "pop up
// button". On the Mac each input is therefore given the label's own text
// as its accessibility label, so the name is still the visible text.
// Linking the label instead (AXTitleUIElement) was tried first: VoiceOver
// ignored it when Tab landed on a pop-up and read only its value (Elliott,
// 2026-10-07). Call on a window once its controls are made; a label
// changed later is not followed.
//
// GTK: wxGTK links every label to the next control in tab order, input or
// not, and ATK makes that link the control's name, so Orca read a hint
// line as part of the check box after it ("Leave it empty to use the full
// name. Notify me about alerts..."; Elliott, 2026-10-08), and a group
// box's title went to the button after the group. A list box's link went
// to the scrolled window around it, so Orca's "table" had no name. Each
// label is watched (wx links them later, once the tab order is built,
// and again when it changes): a link to an input is kept, moved onto the
// list inside a scrolled window, and any other link is dropped.

use wxdragon::prelude::WxWidget;

pub fn name_inputs(window: &impl WxWidget) {
    #[cfg(target_os = "macos")]
    mac_impl::name_inputs(window.get_handle());
    #[cfg(not(any(windows, target_os = "macos")))]
    gtk_impl::name_inputs(window.get_handle());
    #[cfg(windows)]
    let _ = window;
}

#[cfg(target_os = "macos")]
mod mac_impl {
    use crate::mac::{self, Id};

    pub fn name_inputs(view: Id) {
        // A window's handle may be its NSWindow rather than a view.
        let view = unsafe {
            if mac::is(view, c"NSWindow") {
                mac::id(view, c"contentView")
            } else {
                view
            }
        };
        if unsafe { mac::is(view, c"NSView") } {
            walk(view);
        }
    }

    // Siblings in the order they were made, which is wxWidgets' order;
    // a group box's controls are inside it, so every level is walked.
    fn walk(view: Id) {
        let children = unsafe { mac::id(view, c"subviews") };
        let n = unsafe { mac::count(children) };
        let mut previous: Id = std::ptr::null_mut();
        for i in 0..n {
            let child = unsafe { mac::id_with_number(children, c"objectAtIndex:", i as isize) };
            if is_label(previous)
                && let Some(input) = input(child)
            {
                unsafe {
                    let text = mac::id(previous, c"stringValue");
                    mac::id_with(input, c"setAccessibilityLabel:", text);
                }
            }
            walk(child);
            previous = child;
        }
    }

    // wxStaticText is a text field that cannot be edited.
    // A combo box is a text field too, and may be read-only.
    fn is_label(view: Id) -> bool {
        unsafe {
            mac::is(view, c"NSTextField")
                && !mac::is(view, c"NSComboBox")
                && !mac::yes(view, c"isEditable")
        }
    }

    // What a screen reader lands on for an input: an edit box, a pop-up or
    // combo box, or, for a list or a multi-line text, the view inside its
    // scroll view.
    fn input(view: Id) -> Option<Id> {
        unsafe {
            if mac::is(view, c"NSComboBox") {
                Some(view)
            } else if mac::is(view, c"NSTextField") {
                mac::yes(view, c"isEditable").then_some(view)
            } else if mac::is(view, c"NSPopUpButton") || mac::is(view, c"NSTableView") {
                Some(view)
            } else if mac::is(view, c"NSScrollView") {
                let inner = mac::id(view, c"documentView");
                (!inner.is_null()).then_some(inner)
            } else {
                None
            }
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod gtk_impl {
    use std::ffi::{c_int, c_ulong, c_void};

    type GType = usize;

    #[repr(C)]
    struct GList {
        data: *mut c_void,
        next: *mut GList,
        prev: *mut GList,
    }

    // wxGTK links all three libraries already.
    #[link(name = "gtk-3")]
    unsafe extern "C" {
        fn gtk_label_get_type() -> GType;
        fn gtk_container_get_type() -> GType;
        fn gtk_scrolled_window_get_type() -> GType;
        fn gtk_entry_get_type() -> GType;
        fn gtk_text_view_get_type() -> GType;
        fn gtk_combo_box_get_type() -> GType;
        fn gtk_tree_view_get_type() -> GType;
        fn gtk_container_get_children(container: *mut c_void) -> *mut GList;
        fn gtk_bin_get_child(bin: *mut c_void) -> *mut c_void;
        fn gtk_label_get_mnemonic_widget(label: *mut c_void) -> *mut c_void;
        fn gtk_label_set_mnemonic_widget(label: *mut c_void, widget: *mut c_void);
    }
    #[link(name = "gobject-2.0")]
    unsafe extern "C" {
        fn g_type_check_instance_is_a(instance: *mut c_void, gtype: GType) -> c_int;
        fn g_signal_connect_data(
            instance: *mut c_void,
            signal: *const std::ffi::c_char,
            handler: *const c_void,
            data: *mut c_void,
            destroy: *const c_void,
            flags: c_int,
        ) -> c_ulong;
    }
    #[link(name = "glib-2.0")]
    unsafe extern "C" {
        fn g_list_free(list: *mut GList);
    }

    pub fn name_inputs(widget: *mut c_void) {
        if !widget.is_null() {
            unsafe { walk(widget) };
        }
    }

    unsafe fn is(widget: *mut c_void, gtype: GType) -> bool {
        unsafe { g_type_check_instance_is_a(widget, gtype) != 0 }
    }

    // The controls the lint calls inputs: an edit box (GtkEntry, a combo's
    // entry included), the forecast's text view, a combo box, a list.
    unsafe fn is_input(widget: *mut c_void) -> bool {
        unsafe {
            is(widget, gtk_entry_get_type())
                || is(widget, gtk_text_view_get_type())
                || is(widget, gtk_combo_box_get_type())
                || is(widget, gtk_tree_view_get_type())
        }
    }

    unsafe fn walk(widget: *mut c_void) {
        unsafe {
            if is(widget, gtk_label_get_type()) {
                g_signal_connect_data(
                    widget,
                    c"notify::mnemonic-widget".as_ptr(),
                    linked as *const c_void,
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    0,
                );
                fix(widget);
                return;
            }
            if !is(widget, gtk_container_get_type()) {
                return;
            }
            let children = gtk_container_get_children(widget);
            let mut item = children;
            while !item.is_null() {
                walk((*item).data);
                item = (*item).next;
            }
            g_list_free(children);
        }
    }

    unsafe extern "C" fn linked(label: *mut c_void, _spec: *mut c_void, _data: *mut c_void) {
        unsafe { fix(label) };
    }

    // Each change it makes comes back here once, and is then left alone.
    unsafe fn fix(label: *mut c_void) {
        unsafe {
            let target = gtk_label_get_mnemonic_widget(label);
            if target.is_null() || is_input(target) {
                return;
            }
            let inner = if is(target, gtk_scrolled_window_get_type()) {
                gtk_bin_get_child(target)
            } else {
                std::ptr::null_mut()
            };
            if !inner.is_null() && is_input(inner) {
                gtk_label_set_mnemonic_widget(label, inner);
            } else {
                gtk_label_set_mnemonic_widget(label, std::ptr::null_mut());
            }
        }
    }
}
