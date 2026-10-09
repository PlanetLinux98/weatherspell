// Manage Locations (ManageLocationsDialog.cs in 0.1): the saved locations
// in their order (Ctrl+1 to Ctrl+9 follow it), with Move Up, Move Down,
// Add, Edit and Remove. Edits go to a LocationEditor and reach the
// settings only on OK. The buttons act on the selected location and are
// never disabled: a button greyed out under focus throws focus somewhere
// else. Their Alt keys press them and leave focus in the list, so a
// location can be arrowed to and moved with Alt+U and Alt+D; Delete and F2
// in the list remove and edit, as in Explorer.
//
// What belongs to one location, its nickname and whether its new alerts
// are spoken, is set in Edit Location (below), which names the place. 0.1
// had the notify switch under the list, acting on whichever location was
// selected, which nothing said (Elliott, 2026-10-05).

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use weatherspell_core::location_editor::LocationEditor;
use weatherspell_core::net::UreqFetch;
use weatherspell_core::search::LocationSearch;
use weatherspell_core::settings::SavedLocation;
use wx_accessibility::Announcer;
use wxdragon::prelude::*;

use crate::{add_location, dialogs, window::APP_NAME};

struct Manage {
    dialog: Dialog,
    list: ListBox,
    add: Button,
    announcer: Announcer,
    editor: RefCell<LocationEditor>,
    http: Arc<UreqFetch>,
    locations: Arc<LocationSearch>,
    // When the list last lost focus, so a button pressed from the list
    // (its Alt key, or a click) can hand focus back to it.
    left_list: Cell<Option<Instant>>,
    // What to say once focus has settled (say_later).
    later: Timer<Dialog>,
    later_text: RefCell<String>,
}

// The editor as the dialog left it, on OK.
pub fn show(
    parent: &dyn WxWidget,
    saved: &[SavedLocation],
    selected: Option<usize>,
    http: &Arc<UreqFetch>,
    locations: &Arc<LocationSearch>,
) -> Option<LocationEditor> {
    let m = make(parent, saved, selected, http, locations);
    let result = m.dialog.show_modal();
    m.dialog.destroy();
    (result == ID_OK)
        .then(|| std::mem::replace(&mut *m.editor.borrow_mut(), LocationEditor::new(&[])))
}

// Made and ready to show, focus in place (for the lint's test).
pub fn build(
    parent: &dyn WxWidget,
    saved: &[SavedLocation],
    http: &Arc<UreqFetch>,
    locations: &Arc<LocationSearch>,
) -> Dialog {
    make(parent, saved, Some(0), http, locations).dialog
}

fn make(
    parent: &dyn WxWidget,
    saved: &[SavedLocation],
    selected: Option<usize>,
    http: &Arc<UreqFetch>,
    locations: &Arc<LocationSearch>,
) -> Rc<Manage> {
    let dialog = Dialog::builder(parent, "Manage Locations")
        .with_style(DialogStyle::DefaultDialogStyle | DialogStyle::ResizeBorder)
        .build();
    dialogs::app_font(&dialog);
    let sizer = BoxSizer::builder(Orientation::Vertical).build();
    let columns = BoxSizer::builder(Orientation::Horizontal).build();

    let left = BoxSizer::builder(Orientation::Vertical).build();
    let label = StaticText::builder(&dialog)
        .with_label("Saved &locations")
        .build();
    // A scroll bar for a line wider than the list, rather than cutting it
    // off, at the minimum width or a large text size.
    let list = ListBox::builder(&dialog)
        .with_style(ListBoxStyle::HorizontalScrollbar)
        .build();
    left.add(&label, 0, SizerFlag::Bottom, 4);
    left.add(&list, 1, SizerFlag::Expand, 0);

    // One button per row, each as wide as the widest.
    let side = BoxSizer::builder(Orientation::Vertical).build();
    let button = |text: &str| {
        let b = Button::builder(&dialog).with_label(text).build();
        side.add(&b, 0, SizerFlag::Expand | SizerFlag::Bottom, 4);
        b
    };
    let move_up = button("Move &Up");
    let move_down = button("Move &Down");
    let add = button("&Add...");
    let edit = button("&Edit...");
    let remove = button("&Remove");

    columns.add_sizer(&left, 1, SizerFlag::Expand | SizerFlag::Right, 8);
    columns.add_sizer(&side, 0, SizerFlag::empty(), 0);
    sizer.add_sizer(&columns, 1, SizerFlag::Expand | SizerFlag::All, 10);

    let ok = Button::builder(&dialog)
        .with_id(ID_OK)
        .with_label("OK")
        .build();
    let cancel = Button::builder(&dialog)
        .with_id(ID_CANCEL)
        .with_label("Cancel")
        .build();
    ok.set_default();
    sizer.add_sizer(
        &dialogs::button_row(&[&ok, &cancel]),
        0,
        SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Bottom,
        10,
    );
    dialog.set_sizer(sizer, true);
    wx_accessibility::name_inputs(&dialog);
    // 0.1's 560 by 380, at least 440 by 300.
    dialogs::size(&dialog, (80, 25), (63, 20));

    let m = Rc::new(Manage {
        announcer: Announcer::new(&dialog, APP_NAME),
        editor: RefCell::new(LocationEditor::new(saved)),
        dialog,
        list,
        add,
        http: http.clone(),
        locations: locations.clone(),
        left_list: Cell::new(None),
        later: Timer::new(&dialog),
        later_text: RefCell::new(String::new()),
    });
    let s = m.clone();
    m.later
        .on_tick(move |_| s.announcer.say(&s.later_text.borrow()));
    for entry in m.editor.borrow().entries() {
        m.list.append(&entry.to_string());
    }
    let count = m.list.get_count();
    if count > 0 {
        let index = selected.unwrap_or(0).min(count as usize - 1);
        m.list.set_selection(index as u32, true);
    }

    let s = m.clone();
    m.list.on_key_down(move |event| {
        if let WindowEventData::Keyboard(key) = &event
            && !key.control_down()
            && !key.shift_down()
            && !key.alt_down()
        {
            match key.get_key_code() {
                // The main block's Delete and the number pad's, as 0.1 took both.
                Some(WXK_DELETE | WXK_NUMPAD_DELETE) => return s.remove(),
                // The key a Mac labels Delete is Backspace (Fn+Delete is
                // the one above), and it deletes in a Mac list.
                Some(WXK_BACK) if cfg!(target_os = "macos") => return s.remove(),
                Some(WXK_F2) => return s.edit(),
                _ => {}
            }
        }
        event.skip(true);
    });
    let s = m.clone();
    m.list.on_kill_focus(move |event| {
        s.left_list.set(Some(Instant::now()));
        event.skip(true);
    });
    let s = m.clone();
    move_up.on_click(move |_| s.from_list(|s| s.move_selected(-1)));
    let s = m.clone();
    move_down.on_click(move |_| s.from_list(|s| s.move_selected(1)));
    let s = m.clone();
    m.add.on_click(move |_| s.add_location());
    let s = m.clone();
    edit.on_click(move |_| s.edit());
    let s = m.clone();
    remove.on_click(move |_| s.from_list(|s| s.remove()));

    if count > 0 {
        m.list.set_focus();
    } else {
        m.add.set_focus();
    }
    m
}

impl Manage {
    fn selected(&self) -> Option<usize> {
        self.list.get_selection().map(|i| i as usize)
    }

    // A button pressed straight from the list gives focus back to it, so
    // the list goes on reading the location where it now stands, as 0.1's
    // Alt keys left focus in the list.
    fn from_list(self: &Rc<Self>, act: impl FnOnce(&Rc<Self>)) {
        let was_in_list = self
            .left_list
            .get()
            .is_some_and(|at| at.elapsed() < Duration::from_millis(500));
        if was_in_list {
            self.list.set_focus();
        }
        act(self);
    }

    // With focus in the list, the list reports the location as it is
    // selected again at its new place, as it does for an arrow key; from a
    // button, nothing would, so the new position is spoken.
    fn move_selected(&self, by: isize) {
        let Some(index) = self.selected() else {
            self.announcer.say("No location selected.");
            return;
        };
        if !self.editor.borrow_mut().move_by(index, by) {
            self.announcer.say(if by < 0 {
                "Already at the top."
            } else {
                "Already at the bottom."
            });
            return;
        }
        let to = index.saturating_add_signed(by);
        let speak = !self.list.has_focus();
        self.show_entry(index, to);
        if speak {
            let name = self.editor.borrow().entries()[to].display_name();
            self.announcer
                .say(&format!("{name}, {} of {}.", to + 1, self.list.get_count()));
        }
    }

    fn add_location(&self) {
        let Some(place) = add_location::show(&self.dialog, &self.http, &self.locations) else {
            return;
        };
        let (index, added) = self.editor.borrow_mut().add(place);
        if added {
            let text = self.editor.borrow().entries()[index].to_string();
            self.list.append(&text);
        }
        self.list.set_selection(index as u32, true);
        // Into the list, where the new location is read out with its place
        // in the order and can be moved straight away.
        self.list.set_focus();
        if !added {
            let name = self.editor.borrow().entries()[index].display_name();
            self.say_later(format!("{name} is already in the list."));
        }
    }

    fn edit(&self) {
        let Some(index) = self.selected() else {
            self.announcer.say("No location selected.");
            return;
        };
        let (full_name, nickname, notify) = {
            let editor = self.editor.borrow();
            let entry = &editor.entries()[index];
            (
                entry.place.full_name(),
                entry.nickname.clone(),
                entry.notify_alerts,
            )
        };
        let Some(edited) = edit(&self.dialog, &full_name, nickname.as_deref(), notify) else {
            return;
        };
        {
            let mut editor = self.editor.borrow_mut();
            editor.rename(index, &edited.nickname);
            editor.set_notify(index, edited.notify_alerts);
        }
        self.show_entry(index, index);
        // Back in the list, which reads the location as it now is: its new
        // name, and "no alert notifications" when they are off.
        self.list.set_focus();
    }

    fn remove(&self) {
        let Some(index) = self.selected() else {
            self.announcer.say("No location selected.");
            return;
        };
        let name = self.editor.borrow().entries()[index].display_name();
        self.editor.borrow_mut().remove(index);
        self.list.delete(index as u32);
        let count = self.list.get_count();
        if count > 0 {
            self.list
                .set_selection(index.min(count as usize - 1) as u32, true);
        }
        self.announcer.say(&if count > 0 {
            format!("Removed {name}.")
        } else {
            format!("Removed {name}. No saved locations.")
        });
    }

    // The list item at from, taken out and put back at to with its current
    // text, and selected.
    fn show_entry(&self, from: usize, to: usize) {
        let text = self.editor.borrow().entries()[to].to_string();
        self.list.delete(from as u32);
        self.list.insert(&text, to);
        self.list.set_selection(to as u32, true);
    }

    // After a dialog of its own closes, NVDA reads where focus lands; the
    // pause lets that come first.
    fn say_later(&self, text: String) {
        *self.later_text.borrow_mut() = text;
        self.later.start(400, true);
    }
}

// What Edit Location gives back on OK.
pub struct Edited {
    // Empty for the full name.
    pub nickname: String,
    pub notify_alerts: bool,
}

// One location's own settings: a nickname ("Home"), which the Location box
// and the announcements then use while Manage Locations keeps the full
// name beside it, and whether its new alerts are spoken. The field's label
// names the place, so the name a screen reader gives the field says which
// location is being edited; the hint after it is the dialog's own text,
// which NVDA reads as the dialog opens.
pub fn edit(
    parent: &dyn WxWidget,
    full_name: &str,
    nickname: Option<&str>,
    notify_alerts: bool,
) -> Option<Edited> {
    let (dialog, field, notify) = make_edit(parent, full_name, nickname, notify_alerts);
    let result = dialog.show_modal();
    let edited = Edited {
        nickname: field.get_value(),
        notify_alerts: notify.get_value(),
    };
    dialog.destroy();
    (result == ID_OK).then_some(edited)
}

// Made and ready to show, focus in place (for the lint's test).
pub fn build_edit(parent: &dyn WxWidget, full_name: &str) -> Dialog {
    make_edit(parent, full_name, None, true).0
}

fn make_edit(
    parent: &dyn WxWidget,
    full_name: &str,
    nickname: Option<&str>,
    notify_alerts: bool,
) -> (Dialog, TextCtrl, CheckBox) {
    let dialog = Dialog::builder(parent, "Edit Location").build();
    dialogs::app_font(&dialog);
    let sizer = BoxSizer::builder(Orientation::Vertical).build();
    let label = StaticText::builder(&dialog)
        .with_label(&format!("&Nickname for {}", full_name.replace('&', "&&")))
        .build();
    // Wrapped, so a long name wraps within the window rather than widening
    // it.
    dialogs::wrap(&label, 60);
    let field = TextCtrl::builder(&dialog)
        .with_value(nickname.unwrap_or(""))
        .build();
    let help = "Leave it empty to use the full name.";
    let hint = StaticText::builder(&dialog).with_label(help).build();
    wx_accessibility::describe(&field, help);
    let notify = CheckBox::builder(&dialog)
        .with_label("Notify me about &alerts for this location")
        .build();
    notify.set_value(notify_alerts);
    wx_accessibility::report_toggles(&notify);
    let ok = Button::builder(&dialog)
        .with_id(ID_OK)
        .with_label("OK")
        .build();
    let cancel = Button::builder(&dialog)
        .with_id(ID_CANCEL)
        .with_label("Cancel")
        .build();
    ok.set_default();

    sizer.add(
        &label,
        0,
        SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
        10,
    );
    sizer.add(&field, 0, SizerFlag::Expand | SizerFlag::All, 10);
    sizer.add(&hint, 0, SizerFlag::Left | SizerFlag::Right, 13);
    sizer.add(&notify, 0, SizerFlag::All, 10);
    sizer.add_sizer(
        &dialogs::button_row(&[&ok, &cancel]),
        0,
        SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Bottom,
        10,
    );
    dialog.set_sizer(sizer, true);
    wx_accessibility::name_inputs(&dialog);
    dialogs::fit(&dialog, 64);

    field.set_focus();
    field.select_all();
    (dialog, field, notify)
}
