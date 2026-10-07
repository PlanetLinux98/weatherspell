// Add Location (AddLocationDialog.cs in 0.1): type a place, press Enter to
// search, arrow through the results, Enter to add. The list only changes
// when the user asks, never as they type. Coordinates go in the same field
// (coordinates.rs says what it reads) and come back as one result named
// after the place they fall in.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use weatherspell_core::coordinates;
use weatherspell_core::fetch::{FetchError, ServiceError};
use weatherspell_core::location::Location;
use weatherspell_core::net::UreqFetch;
use weatherspell_core::search::LocationSearch;
use wx_accessibility::Announcer;
use wxdragon::prelude::*;

use crate::{dialogs, on_ui_thread, window::APP_NAME};

thread_local! {
    // The dialog a search's answer goes to; one is open at a time.
    static OPEN: RefCell<Option<Rc<AddLocation>>> = const { RefCell::new(None) };
}

struct AddLocation {
    dialog: Dialog,
    query: TextCtrl,
    search: Button,
    results: ListBox,
    status: StaticText,
    add: Button,
    announcer: Announcer,
    http: Arc<UreqFetch>,
    locations: Arc<LocationSearch>,
    found: RefCell<Vec<Location>>,
    // Each search's number, so an answer that arrives after a newer search
    // or after the dialog has closed is dropped.
    generation: Cell<u64>,
    chosen: RefCell<Option<Location>>,
}

// What a search's thread hands back.
enum Answer {
    Places(Result<Vec<Location>, ServiceError>),
    Point(f64, f64, Result<Option<Location>, ServiceError>),
}

// The place chosen, or None.
pub fn show(
    parent: &dyn WxWidget,
    http: &Arc<UreqFetch>,
    locations: &Arc<LocationSearch>,
) -> Option<Location> {
    let d = make(parent, http, locations);
    OPEN.with(|o| *o.borrow_mut() = Some(d.clone()));
    d.dialog.show_modal();
    OPEN.with(|o| *o.borrow_mut() = None);
    d.generation.set(d.generation.get() + 1);
    d.dialog.destroy();
    d.chosen.borrow_mut().take()
}

// Made and ready to show, focus in place (for the lint's test).
pub fn build(
    parent: &dyn WxWidget,
    http: &Arc<UreqFetch>,
    locations: &Arc<LocationSearch>,
) -> Dialog {
    make(parent, http, locations).dialog
}

fn make(
    parent: &dyn WxWidget,
    http: &Arc<UreqFetch>,
    locations: &Arc<LocationSearch>,
) -> Rc<AddLocation> {
    let dialog = Dialog::builder(parent, "Add Location")
        .with_style(DialogStyle::DefaultDialogStyle | DialogStyle::ResizeBorder)
        .build();
    dialogs::developer_font(&dialog);
    let sizer = BoxSizer::builder(Orientation::Vertical).build();

    // Each label just before its input among the dialog's children, which
    // is what names the input.
    let query_label = StaticText::builder(&dialog)
        .with_label("&Place name, postal code or coordinates")
        .build();
    let query = TextCtrl::builder(&dialog)
        .with_style(TextCtrlStyle::ProcessEnter)
        .build();
    let search = Button::builder(&dialog).with_label("&Search").build();
    let results_label = StaticText::builder(&dialog).with_label("&Results").build();
    let results = ListBox::builder(&dialog).build();
    let status = StaticText::builder(&dialog)
        .with_label("Type a place name and press Enter.")
        .build();
    let add = Button::builder(&dialog).with_label("&Add").build();
    add.enable(false);
    let cancel = Button::builder(&dialog)
        .with_id(ID_CANCEL)
        .with_label("Cancel")
        .build();

    sizer.add(
        &query_label,
        0,
        SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
        10,
    );
    let row = BoxSizer::builder(Orientation::Horizontal).build();
    row.add(
        &query,
        1,
        SizerFlag::AlignCenterVertical | SizerFlag::Right,
        6,
    );
    row.add(&search, 0, SizerFlag::AlignCenterVertical, 0);
    sizer.add_sizer(&row, 0, SizerFlag::Expand | SizerFlag::All, 10);
    sizer.add(&results_label, 0, SizerFlag::Left | SizerFlag::Right, 10);
    sizer.add(&results, 1, SizerFlag::Expand | SizerFlag::All, 10);
    sizer.add(
        &status,
        0,
        SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right,
        13,
    );
    sizer.add_sizer(
        &dialogs::button_row(&[&add, &cancel]),
        0,
        SizerFlag::Expand | SizerFlag::All,
        10,
    );
    dialog.set_sizer(sizer, true);
    wx_accessibility::name_inputs(&dialog);
    // 0.1's 520 by 420, at least 420 by 360.
    dialogs::size(&dialog, (74, 28), (60, 24));

    let d = Rc::new(AddLocation {
        announcer: Announcer::new(&dialog, APP_NAME),
        dialog,
        query,
        search,
        results,
        status,
        add,
        http: http.clone(),
        locations: locations.clone(),
        found: RefCell::new(Vec::new()),
        generation: Cell::new(0),
        chosen: RefCell::new(None),
    });

    // Enter searches while typing and adds once a result is chosen; the
    // default button follows, so it is the one a reader hears as default.
    d.search.set_default();
    let a = d.clone();
    d.query.on_enter_pressed(move |_| a.start_search());
    let a = d.clone();
    d.search.on_click(move |_| a.start_search());
    let a = d.clone();
    d.add.on_click(move |_| a.choose());
    let a = d.clone();
    d.results
        .on_selection_changed(move |_| a.add.enable(a.results.get_selection().is_some()));
    let a = d.clone();
    d.results.on_item_double_clicked(move |_| a.choose());
    let a = d.clone();
    d.results.on_key_down(move |event| {
        if let WindowEventData::Keyboard(key) = &event
            && matches!(key.get_key_code(), Some(WXK_RETURN | WXK_NUMPAD_ENTER))
            && !key.control_down()
            && !key.shift_down()
            && !key.alt_down()
        {
            a.choose();
            return;
        }
        event.skip(true);
    });
    let a = d.clone();
    d.results.on_set_focus(move |event| {
        a.add.set_default();
        event.skip(true);
    });
    let a = d.clone();
    d.results.on_kill_focus(move |event| {
        a.search.set_default();
        event.skip(true);
    });

    d.query.set_focus();
    d
}

impl AddLocation {
    fn start_search(&self) {
        let query = self.query.get_value().trim().to_string();
        let point = coordinates::read(&query);
        if let Some(problem) = point.as_ref().and_then(|p| p.problem.clone()) {
            self.set_status(&problem);
            return;
        }
        if point.is_none() && query.chars().count() < 2 {
            self.set_status("Type at least two characters, then press Enter.");
            return;
        }

        let generation = self.generation.get() + 1;
        self.generation.set(generation);
        match &point {
            None => self.set_status(&format!("Searching for {query}...")),
            Some(p) => self.set_status(&format!(
                "Looking up {}...",
                coordinates::words(p.latitude, p.longitude)
            )),
        }
        self.search.enable(false);

        let http = self.http.clone();
        let locations = self.locations.clone();
        let point = point.map(|p| (p.latitude, p.longitude));
        std::thread::spawn(move || {
            let answer = match point {
                None => Answer::Places(locations.search(&*http, &query)),
                Some((lat, lon)) => Answer::Point(lat, lon, locations.name_point(&*http, lat, lon)),
            };
            on_ui_thread(move || {
                if let Some(d) = OPEN.with(|o| o.borrow().clone()) {
                    d.answered(generation, &query, answer);
                }
            });
        });
    }

    fn answered(&self, generation: u64, query: &str, answer: Answer) {
        if generation != self.generation.get() {
            return;
        }
        self.search.enable(true);
        let (failed, result) = match answer {
            Answer::Places(result) => (
                "Couldn't search",
                result.map(|places| {
                    let count = if places.len() == 1 {
                        "1 place found.".to_string()
                    } else {
                        format!("{} places found.", places.len())
                    };
                    let items = places
                        .into_iter()
                        .map(|p| {
                            let text = p.search_result_text();
                            (p, text)
                        })
                        .collect::<Vec<_>>();
                    (items, count)
                }),
            ),
            Answer::Point(lat, lon, result) => (
                "Couldn't look up those coordinates",
                result.map(|named| match named {
                    Some(place) => {
                        let text = place.near_text();
                        (vec![(place, text)], "1 place found.".to_string())
                    }
                    None => {
                        let place = Location::at_point(lat, lon);
                        let text = place.name.clone();
                        (
                            vec![(place, text)],
                            "Nothing nearby has a name; the point itself can be added.".to_string(),
                        )
                    }
                }),
            ),
        };
        let (items, count) = match result {
            Ok(found) => found,
            Err(e) => {
                let reason = match e {
                    ServiceError::Fetch(FetchError::TimedOut { .. }) => {
                        "the search service took too long to answer".to_string()
                    }
                    e => e.to_string(),
                };
                self.set_status(&format!("{failed}: {reason}"));
                self.query.set_focus();
                return;
            }
        };

        self.results.clear();
        for (_, text) in &items {
            self.results.append(text);
        }
        *self.found.borrow_mut() = items.into_iter().map(|(place, _)| place).collect();
        if self.found.borrow().is_empty() {
            self.add.enable(false);
            self.set_status(&format!("No places found for {query}."));
            self.query.set_focus();
            return;
        }
        // Focus first and the count after it: said before the move, the
        // count was cut into NVDA's report of the list and its first
        // result, which it then read a second time (#17).
        self.show_status(&count);
        self.results.set_selection(0, true);
        self.add.enable(true);
        self.results.set_focus();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(400));
            on_ui_thread(move || {
                if let Some(d) = OPEN.with(|o| o.borrow().clone())
                    && d.generation.get() == generation
                {
                    d.announcer.say(&count);
                }
            });
        });
    }

    fn show_status(&self, text: &str) {
        self.status.set_label(text);
        dialogs::wrap(&self.status, 66);
        self.dialog.layout();
    }

    fn set_status(&self, text: &str) {
        self.show_status(text);
        self.announcer.say(text);
    }

    fn choose(&self) {
        let Some(index) = self.results.get_selection() else {
            return;
        };
        let place = self.found.borrow().get(index as usize).cloned();
        if let Some(place) = place {
            *self.chosen.borrow_mut() = Some(place);
            dialogs::end(&self.dialog, ID_OK);
        }
    }
}
