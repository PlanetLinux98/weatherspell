// Roles wxGTK leaves wrong for screen readers, set on the window's ATK
// object. Nothing to do elsewhere: Windows' controls carry their own, and
// the Mac's are left as they are.
//
// A status bar: wx's status bar on GTK and the Mac is wxStatusBarGeneric,
// which draws its text, so Orca and VoiceOver find nothing in it. An app
// shows a text line in its place; on GTK the window holding that line is
// given the status bar role, which is how Orca's status bar command (Orca+/
// twice on a laptop layout) finds it in the frame. VoiceOver has no such
// command: the Mac's line stays a plain text line.
//
// A dialog: a wx dialog on GTK is a plain GtkWindow, which ATK calls a
// frame. Orca reads a dialog's text that belongs to no control as it
// opens (About's lines, as a message box's), but not a frame's, so each
// dialog is given the dialog role, as GTK's own dialogs have.

use wxdragon::prelude::WxWidget;

pub fn status_bar(window: &impl WxWidget) {
    #[cfg(not(any(windows, target_os = "macos")))]
    gtk_impl::set_role(window.get_handle(), gtk_impl::ATK_ROLE_STATUSBAR);
    #[cfg(any(windows, target_os = "macos"))]
    let _ = window;
}

pub fn dialog(window: &impl WxWidget) {
    #[cfg(not(any(windows, target_os = "macos")))]
    gtk_impl::set_role(window.get_handle(), gtk_impl::ATK_ROLE_DIALOG);
    #[cfg(any(windows, target_os = "macos"))]
    let _ = window;
}

#[cfg(not(any(windows, target_os = "macos")))]
mod gtk_impl {
    use std::ffi::{c_int, c_void};

    // AtkRole.
    pub const ATK_ROLE_DIALOG: c_int = 16;
    pub const ATK_ROLE_STATUSBAR: c_int = 53;

    // wxGTK links both libraries already.
    #[link(name = "gtk-3")]
    unsafe extern "C" {
        fn gtk_widget_get_accessible(widget: *mut c_void) -> *mut c_void;
    }
    #[link(name = "atk-1.0")]
    unsafe extern "C" {
        fn atk_object_set_role(accessible: *mut c_void, role: c_int);
    }

    pub fn set_role(widget: *mut c_void, role: c_int) {
        if widget.is_null() {
            return;
        }
        unsafe {
            let accessible = gtk_widget_get_accessible(widget);
            if !accessible.is_null() {
                atk_object_set_role(accessible, role);
            }
        }
    }
}
