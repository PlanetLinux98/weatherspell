// An input's name is the label just before it (the lint's first rule).
// Windows' screen readers find that label themselves, from the window
// order; VoiceOver does not, and reads an unnamed "edit text" or "pop up
// button". On the Mac each input is therefore given the label's own text
// as its accessibility label, so the name is still the visible text.
// Linking the label instead (AXTitleUIElement) was tried first: VoiceOver
// ignored it when Tab landed on a pop-up and read only its value (Elliott,
// 2026-10-07). Call on a window once its controls are made; a label
// changed later is not followed. Elsewhere this does nothing.

use wxdragon::prelude::WxWidget;

pub fn name_inputs(window: &impl WxWidget) {
    #[cfg(target_os = "macos")]
    mac_impl::name_inputs(window.get_handle());
    #[cfg(not(target_os = "macos"))]
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
