// Hour by Hour (View > Hour by Hour, Ctrl+H; Cmd+Shift+H on the Mac, where
// Cmd+H hides the app): the hours as read-only text, like Alert details,
// opened on the hour the forecast's caret was on. Ctrl+PageDown and
// Ctrl+PageUp (Cmd+] and Cmd+[ on the Mac) move a day at a time, as they
// move between sections in the main window, the day spoken the same way.
// Enter or Escape closes, back to the forecast where it was. The words are
// the core's (hourly.rs).

use std::rc::Rc;

use weatherspell_core::hourly::HourlyText;
use weatherspell_core::layout::SectionLayout;
use wx_accessibility::{Announcer, ScreenReaders};
use wxdragon::prelude::*;
use wxdragon::scrollable::WxScrollable;

use crate::window::APP_NAME;
use crate::{dialogs, edit};

pub fn show(parent: &Frame, title: &str, text: &HourlyText, caret: usize) {
    let dialog = build(parent, title, text, caret);
    dialog.show_modal();
    dialog.destroy();
}

// Made and ready to show, focus in place (also for the lint's test).
pub fn build(parent: &Frame, title: &str, hours: &HourlyText, caret: usize) -> Dialog {
    let dialog = Dialog::builder(parent, title)
        .with_style(
            DialogStyle::DefaultDialogStyle | DialogStyle::ResizeBorder | DialogStyle::MaximizeBox,
        )
        .build();
    dialogs::app_font(&dialog);
    let sizer = BoxSizer::builder(Orientation::Vertical).build();

    // The label just before the text among the dialog's children names it.
    let label = StaticText::builder(&dialog).with_label("&Hours").build();
    let text = TextCtrl::builder(&dialog)
        .with_style(TextCtrlStyle::MultiLine | TextCtrlStyle::ReadOnly | TextCtrlStyle::NoHideSel)
        .build();
    // Read-only text is grey by default; it is for reading.
    text.set_background_color(SystemSettings::get_colour(SystemColour::Window));
    text.change_value(&hours.layout.text);
    edit::hook(&text, None, None);

    let close = Button::builder(&dialog)
        .with_id(ID_CANCEL)
        .with_label("Close")
        .build();
    close.set_default();

    sizer.add(
        &label,
        0,
        SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
        8,
    );
    sizer.add(&text, 1, SizerFlag::Expand | SizerFlag::All, 8);
    sizer.add_sizer(
        &dialogs::button_row(&[&close]),
        0,
        SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Bottom,
        8,
    );
    dialog.set_sizer(sizer, true);
    wx_accessibility::name_inputs(&dialog);
    wx_accessibility::dialog(&dialog);
    // A line is about a hundred characters; wider than Alert details so
    // most hours fit on one line at the usual size.
    dialogs::size(&dialog, (100, 32), (57, 20));

    let layout = Rc::new(hours.layout.clone());
    let announcer = Rc::new(Announcer::new(&dialog, APP_NAME));
    let d = dialog;
    text.on_key_down(move |event| {
        if let WindowEventData::Keyboard(key) = &event {
            let code = key.get_key_code();
            let plain = !key.control_down() && !key.shift_down() && !key.alt_down();
            // A multi-line text box keeps Enter for itself, so it closes
            // here.
            if plain && matches!(code, Some(WXK_RETURN | WXK_NUMPAD_ENTER)) {
                dialogs::end(&d, ID_CANCEL);
                return;
            }
            if let Some(forward) =
                section_key(code, key.control_down(), key.shift_down(), key.alt_down())
            {
                jump(&text, &layout, &announcer, forward);
                return;
            }
        }
        event.skip(true);
    });

    // Focus in the text, on the hour asked for, in sight.
    text.set_focus();
    let at = native(&hours.layout, caret.min(hours.layout.length));
    text.set_insertion_point(at);
    text.show_position(at);
    dialog
}

// The main window's section keys: Ctrl+PageDown and Ctrl+PageUp, and on
// the Mac Cmd+] and Cmd+[ (wx reads Cmd as Ctrl there). Some(true) is
// forward.
fn section_key(code: Option<i32>, control: bool, shift: bool, alt: bool) -> Option<bool> {
    if !control || shift || alt {
        return None;
    }
    match code? {
        WXK_PAGEDOWN | WXK_NUMPAD_PAGEDOWN if !cfg!(target_os = "macos") => Some(true),
        WXK_PAGEUP | WXK_NUMPAD_PAGEUP if !cfg!(target_os = "macos") => Some(false),
        c if cfg!(target_os = "macos") && c == ']' as i32 => Some(true),
        c if cfg!(target_os = "macos") && c == '[' as i32 => Some(false),
        _ => None,
    }
}

// As the main window's jump: the heading is spoken where the reader would
// not say the line itself (ScreenReaders).
fn jump(text: &TextCtrl, layout: &SectionLayout, announcer: &Announcer, forward: bool) {
    let here = offset(layout, text.get_selection().0);
    let target = if forward {
        layout.headings.iter().find(|(_, at)| *at > here)
    } else {
        layout.headings.iter().rev().find(|(_, at)| *at < here)
    };
    match target {
        Some((heading, at)) => {
            let at = native(layout, *at);
            text.set_insertion_point(at);
            text.show_position(at);
            if ScreenReaders::running().need_caret_moves_spoken() {
                announcer.say(heading);
            }
        }
        None => announcer.say(if forward {
            "No next section."
        } else {
            "No previous section."
        }),
    }
}

// The text box counts positions its own way (edit.rs); these are the
// layout's offsets.
fn native(layout: &SectionLayout, offset: usize) -> i64 {
    if cfg!(windows) {
        layout.crlf_position(offset) as i64
    } else {
        offset as i64
    }
}

fn offset(layout: &SectionLayout, native: i64) -> usize {
    let native = native.max(0) as usize;
    if cfg!(windows) {
        layout.offset_from_crlf(native)
    } else {
        native
    }
}
