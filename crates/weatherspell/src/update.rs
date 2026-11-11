// Check for Updates' windows (#11): Update Available, the download's
// progress, and the choice to relaunch. Which version, its notes and
// every sentence are the core's (updates.rs); putting the download in
// place is install.rs; the main window runs the steps (window.rs).
//
// Update Available opens like Alert details: a line of text, read as the
// dialog opens, then the notes as read-only text to arrow through and
// copy, focus in them. Its first button does what this copy can do:
// Install, or Copy Command for a copy winget installed, or Open Download
// Page for a copy with nothing to swap. Buttons are never disabled.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use weatherspell_core::net::UreqFetch;
use weatherspell_core::updates::{self, DownloadProblem, Update, Version, wording};
use wx_accessibility::Announcer;
use wxdragon::clipboard::Clipboard;
use wxdragon::prelude::*;

use crate::install::Way;
use crate::window::APP_NAME;
use crate::{about, dialogs, edit, on_ui_thread};

// The message boxes' title: what was asked for.
pub const TITLE: &str = "Check for Updates";

const ID_ACT: Id = ID_HIGHEST + 1;

// True when Install was chosen.
pub fn show(parent: &Frame, update: &Update, running: &Version, way: Way) -> bool {
    let dialog = build(parent, update, running, way);
    let install = dialog.show_modal() == ID_OK;
    dialog.destroy();
    install
}

// Made and ready to show, focus in place (also for the lint's test).
pub fn build(parent: &Frame, update: &Update, running: &Version, way: Way) -> Dialog {
    let dialog = Dialog::builder(parent, "Update Available")
        .with_style(
            DialogStyle::DefaultDialogStyle | DialogStyle::ResizeBorder | DialogStyle::MaximizeBox,
        )
        .build();
    dialogs::app_font(&dialog);
    let sizer = BoxSizer::builder(Orientation::Vertical).build();

    let (summary, action) = match way {
        Way::Install { with_permission } => {
            let mut line = wording::available(update, running);
            if with_permission {
                line.push(' ');
                line.push_str(wording::NEEDS_PERMISSION);
            }
            (line, "&Install")
        }
        Way::Winget => (wording::winget(update, running), "&Copy Command"),
        Way::ByHand => (wording::by_hand(update, running), "&Open Download Page"),
    };
    let line = StaticText::builder(&dialog).with_label(&summary).build();
    dialogs::wrap(&line, 82);
    // The label just before the text among the dialog's children names it.
    let label = StaticText::builder(&dialog)
        .with_label(wording::WHATS_NEW)
        .build();
    let text = TextCtrl::builder(&dialog)
        .with_style(TextCtrlStyle::MultiLine | TextCtrlStyle::ReadOnly | TextCtrlStyle::NoHideSel)
        .build();
    // Read-only text is grey by default; it is for reading.
    text.set_background_color(SystemSettings::get_colour(SystemColour::Window));
    text.change_value(&update.notes);
    edit::hook(&text, None, None);

    let act = Button::builder(&dialog)
        .with_id(ID_ACT)
        .with_label(action)
        .build();
    let close = Button::builder(&dialog)
        .with_id(ID_CANCEL)
        .with_label("Close")
        .build();
    act.set_default();

    sizer.add(
        &line,
        0,
        SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
        8,
    );
    sizer.add(
        &label,
        0,
        SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
        8,
    );
    sizer.add(&text, 1, SizerFlag::Expand | SizerFlag::All, 8);
    sizer.add_sizer(
        &dialogs::button_row(&[&act, &close]),
        0,
        SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Bottom,
        8,
    );
    dialog.set_sizer(sizer, true);
    wx_accessibility::name_inputs(&dialog);
    wx_accessibility::dialog(&dialog);
    // Alert details' size: the notes are a screen or so of text.
    dialogs::size(&dialog, (86, 31), (57, 20));

    let announcer = Rc::new(Announcer::new(&dialog, APP_NAME));
    let page = update.page.clone();
    let d = dialog;
    let acted: Rc<dyn Fn()> = Rc::new(move || match way {
        Way::Install { .. } => dialogs::end(&d, ID_OK),
        // Said, since nothing else tells a screen reader user it worked.
        Way::Winget => {
            if Clipboard::get().set_text(updates::WINGET_COMMAND) {
                announcer.say(wording::COPIED);
            }
        }
        Way::ByHand => about::open(&d, &page),
    });
    let a = acted.clone();
    act.on_click(move |_| a());
    // A multi-line text box keeps Enter for itself, so the default button
    // is pressed from here.
    text.on_key_down(move |event| {
        if let WindowEventData::Keyboard(key) = &event
            && matches!(key.get_key_code(), Some(WXK_RETURN | WXK_NUMPAD_ENTER))
            && !key.control_down()
            && !key.shift_down()
            && !key.alt_down()
        {
            acted();
            return;
        }
        event.skip(true);
    });

    // Focus in the notes, caret at the top, nothing selected.
    text.set_focus();
    text.set_selection(0, 0);
    dialog
}

// The download under way: its dialog, where the thread's news goes.
struct Download {
    dialog: Dialog,
    gauge: Gauge,
    result: RefCell<Option<Result<(), DownloadProblem>>>,
}

thread_local! {
    static DOWNLOAD: RefCell<Option<Rc<Download>>> = const { RefCell::new(None) };
}

// One line, a native progress bar and Cancel: the line is read as the
// dialog opens, and readers report the bar as the user has set them to
// (NVDA beeps or speaks percentages). Made and ready to show (also for
// the lint's test).
pub fn build_progress(parent: &dyn WxWidget, update: &Update) -> (Dialog, Gauge) {
    let dialog = Dialog::builder(parent, "Downloading Update").build();
    dialogs::app_font(&dialog);
    let sizer = BoxSizer::builder(Orientation::Vertical).build();
    let line = StaticText::builder(&dialog)
        .with_label(&wording::downloading(update))
        .build();
    let gauge = Gauge::builder(&dialog).with_range(100).build();
    let cancel = Button::builder(&dialog)
        .with_id(ID_CANCEL)
        .with_label("Cancel")
        .build();
    cancel.set_default();
    sizer.add(
        &line,
        0,
        SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
        12,
    );
    sizer.add(&gauge, 0, SizerFlag::Expand | SizerFlag::All, 12);
    sizer.add_sizer(
        &dialogs::button_row(&[&cancel]),
        0,
        SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Bottom,
        12,
    );
    dialog.set_sizer(sizer, true);
    wx_accessibility::name_inputs(&dialog);
    wx_accessibility::dialog(&dialog);
    dialogs::fit(&dialog, 56);
    cancel.set_focus();
    (dialog, gauge)
}

// The update's file fetched to `to` and checked, with the progress dialog
// up meanwhile. Cancel (or Escape) closes it at once, even when the
// connection has hung: the thread is told to stop, and removes what it
// had written when it does.
pub fn download(
    parent: &Frame,
    http: Arc<UreqFetch>,
    update: &Update,
    to: &Path,
) -> Result<(), DownloadProblem> {
    let (dialog, gauge) = build_progress(parent, update);
    let state = Rc::new(Download {
        dialog,
        gauge,
        result: RefCell::new(None),
    });
    DOWNLOAD.with(|d| *d.borrow_mut() = Some(state.clone()));

    let cancelled = Arc::new(AtomicBool::new(false));
    let (stop, update, to) = (cancelled.clone(), update.clone(), to.to_path_buf());
    std::thread::spawn(move || {
        let size = update.file.size.max(1);
        let mut shown = 0;
        let result = updates::download(&*http, &update, &to, &mut |received| {
            let percent = (received.min(size) * 100 / size) as i32;
            if percent != shown {
                shown = percent;
                on_ui_thread(move || {
                    if let Some(d) = DOWNLOAD.with(|d| d.borrow().clone()) {
                        d.gauge.set_value(percent);
                    }
                });
            }
            !stop.load(Ordering::Relaxed)
        });
        on_ui_thread(move || {
            if let Some(d) = DOWNLOAD.with(|d| d.borrow().clone()) {
                *d.result.borrow_mut() = Some(result);
                dialogs::end(&d.dialog, ID_OK);
            }
        });
    });

    dialog.show_modal();
    cancelled.store(true, Ordering::Relaxed);
    DOWNLOAD.with(|d| *d.borrow_mut() = None);
    dialog.destroy();
    state
        .result
        .borrow_mut()
        .take()
        .unwrap_or(Err(DownloadProblem::Cancelled))
}

// A message box, read aloud as it opens.
pub fn tell(parent: &dyn WxWidget, text: &str, warning: bool) {
    let icon = if warning {
        MessageDialogStyle::IconWarning
    } else {
        MessageDialogStyle::IconInformation
    };
    MessageDialog::builder(parent, text, TITLE)
        .with_style(MessageDialogStyle::OK | icon)
        .build()
        .show_modal();
}

// "Weatherspell 0.3.1 is installed. Relaunch to start using it.", with
// Relaunch Now and Later (Escape). Later leaves this copy running; the
// new one starts next time.
pub fn relaunch_now(parent: &dyn WxWidget, version: &Version) -> bool {
    let dialog = MessageDialog::builder(parent, &wording::installed(version), TITLE)
        .with_style(
            MessageDialogStyle::OK
                | MessageDialogStyle::Cancel
                | MessageDialogStyle::IconInformation,
        )
        .build();
    dialog.set_ok_cancel_labels(wording::RELAUNCH_NOW, wording::LATER);
    dialog.show_modal() == ID_OK
}
