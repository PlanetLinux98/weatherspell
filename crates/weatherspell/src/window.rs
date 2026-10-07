// The main window (MainForm.cs in 0.1): the Location box, the forecast as
// text for reading, the native menu bar and the status bar. A launch or a
// switch says what it is fetching and speaks "forecast ready"; F5 and the
// timers rewrite the text under the reader without moving them; every
// saved location's alerts are checked and new ones announced; a failed
// fetch leaves the text there was, or the cached text, dated.
//
// The controls are built in code: every control's name, tab order and
// label is visible in one place and reviewable in a diff (NOTES.md).

use std::cell::{Cell, RefCell};
use std::io;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use jiff::Timestamp;
use jiff::tz::Offset;
use weatherspell_core::alerts::{self, AlertReport, WeatherAlert};
use weatherspell_core::cache::ForecastCache;
use weatherspell_core::clock::Clock;
use weatherspell_core::fetch::{FetchError, ForecastService, ServiceError, check_alerts};
use weatherspell_core::forecast::Forecast;
use weatherspell_core::layout::{RewritePlan, SectionLayout, SelectionHold};
use weatherspell_core::location::Location;
use weatherspell_core::net::UreqFetch;
use weatherspell_core::placement::{self, CharSize, Rect};
use weatherspell_core::search::LocationSearch;
use weatherspell_core::settings::{AppSettings, LoadProblem, SavedLocation, SettingsStore};
use weatherspell_core::units;
use weatherspell_core::writer::{self, Section, WriterOptions};
use wx_accessibility::{Announcer, ScreenReaders};
use wxdragon::prelude::*;

use crate::system::{self, Region};
use crate::{about, add_location, details, dialogs, edit, guide, manage, on_ui, settings_dialog};

pub const APP_NAME: &str = "Weatherspell";
const FORECAST_DAYS: u32 = 7;

const ID_REFRESH: Id = ID_HIGHEST + 1;
const ID_NEXT_SECTION: Id = ID_HIGHEST + 2;
const ID_PREVIOUS_SECTION: Id = ID_HIGHEST + 3;
const ID_ALERTS: Id = ID_HIGHEST + 4;
// wxID_PREFERENCES, which wxDragon does not name: the Mac moves it to the
// app menu as Settings with Cmd+comma.
const ID_PREFERENCES: Id = 5022;
const ID_MANAGE: Id = ID_HIGHEST + 5;
const ID_ADD: Id = ID_HIGHEST + 6;
const ID_GUIDE: Id = ID_HIGHEST + 7;
const ID_RESET_WINDOW: Id = ID_HIGHEST + 8;
// Ctrl+1 to Ctrl+9: the first nine saved locations.
const ID_LOCATION: Id = ID_HIGHEST + 10;
const MENU_LOCATIONS: usize = 9;

pub struct MainWindow {
    frame: Frame,
    choice: Choice,
    text: TextCtrl,
    announcer: Announcer,

    store: SettingsStore,
    load_problem: Option<LoadProblem>,
    settings: RefCell<AppSettings>,
    cache: ForecastCache,
    http: Arc<UreqFetch>,
    service: Arc<ForecastService>,
    search: Arc<LocationSearch>,
    region: Region,

    // What the text box shows, kept so it can be rewritten in place: the
    // Alerts section when a poll finds them changed, the age line as the
    // clock moves, all of it when a refresh brings new data.
    layout: RefCell<SectionLayout>,
    shown: RefCell<Option<Forecast>>,
    shown_alerts: RefCell<Option<AlertReport>>,
    // Why the last refresh left older text on screen; None after a success.
    refresh_problem: RefCell<Option<String>>,
    hold: RefCell<SelectionHold>,

    // Each refresh's number: a later one, a switch or closing the window
    // makes an earlier one's answer (and its announcement) stale.
    generation: Cell<u64>,
    polling: Cell<bool>,
    poll_changed: Cell<bool>,
    // The other locations' alerts are first checked once the one on
    // screen has its forecast.
    first_poll: Cell<bool>,
    shown_at: Cell<Instant>,
    closed: Cell<bool>,
    // The window's bounds when last neither maximized nor minimized, and
    // whether it was maximized when last not minimized: minimized from
    // maximized, the state alone no longer says which it returns to.
    normal: Cell<Rect>,
    maximized: Cell<bool>,

    forecast_timer: Timer<Frame>,
    alert_timer: Timer<Frame>,
    // Once a minute: the age line, and the day headings at midnight.
    clock_timer: Timer<Frame>,
}

// What a refresh's fetch thread hands back.
pub struct Fetched {
    generation: u64,
    index: usize,
    location: Location,
    forecast: Result<Forecast, ServiceError>,
    alerts: AlertReport,
    keep_caret: bool,
    automatic: bool,
    started: Instant,
}

pub fn build() -> Rc<MainWindow> {
    let folder = system::data_folder();
    let store = SettingsStore::in_folder(&folder);
    let (settings, load_problem) = store.load();

    let frame = Frame::builder().with_title(APP_NAME).build();
    dialogs::developer_font(&frame);
    frame.set_menu_bar(menu_bar(&settings));
    system::set_window_icon(&frame);
    frame.create_status_bar(1, 0, -1, "statusBar");
    frame.set_status_text("Ready", 0);

    let panel = Panel::builder(&frame).build();
    let outer = BoxSizer::builder(Orientation::Vertical).build();

    // Alt+O: Alt+L belongs to the Locations menu. The label comes just
    // before the box among the panel's children, which is what names it.
    let header = BoxSizer::builder(Orientation::Horizontal).build();
    let location_label = StaticText::builder(&panel).with_label("L&ocation").build();
    let choice = Choice::builder(&panel).build();
    header.add(
        &location_label,
        0,
        SizerFlag::AlignCenterVertical | SizerFlag::Right,
        8,
    );
    header.add(&choice, 1, SizerFlag::Expand, 0);
    outer.add_sizer(&header, 0, SizerFlag::Expand | SizerFlag::All, 8);

    let forecast_label = StaticText::builder(&panel).with_label("Forecast").build();
    outer.add(
        &forecast_label,
        0,
        SizerFlag::Left | SizerFlag::Right | SizerFlag::Bottom,
        8,
    );
    // A plain EDIT control, which NVDA and Narrator both read by line
    // (NOTES.md, "Rust and wxWidgets").
    let text = TextCtrl::builder(&panel)
        .with_style(TextCtrlStyle::MultiLine | TextCtrlStyle::ReadOnly | TextCtrlStyle::NoHideSel)
        .build();
    // Read-only text is grey by default; it is for reading.
    text.set_background_color(SystemSettings::get_colour(SystemColour::Window));
    outer.add(&text, 1, SizerFlag::Expand, 0);
    panel.set_sizer(outer, true);
    wx_accessibility::name_inputs(&panel);

    frame.set_min_size(Size::new(
        frame.get_char_width() * 68,
        frame.get_char_height() * 24,
    ));
    // Where it was when it last closed, if a screen still shows enough of
    // it; otherwise centred, as on a first run.
    let restored = settings.window.and_then(|saved| {
        placement::restore(
            &saved,
            char_size(&frame),
            &working_areas(),
            caption_height(&frame),
        )
    });
    match restored {
        Some(r) => frame.set_size_with_pos(r.left, r.top, r.width, r.height),
        None => place_by_default(&frame),
    }
    let maximized = settings.window.is_some_and(|w| w.maximized);

    let window = Rc::new(MainWindow {
        announcer: Announcer::new(&frame, APP_NAME),
        forecast_timer: Timer::new(&frame),
        alert_timer: Timer::new(&frame),
        clock_timer: Timer::new(&frame),
        frame,
        choice,
        text,
        cache: weatherspell_core::cache::ForecastCache::in_folder(&folder),
        store,
        load_problem,
        settings: RefCell::new(settings),
        http: Arc::new(UreqFetch::new(&format!("Weatherspell/{}", crate::VERSION))),
        service: Arc::new(ForecastService::new()),
        search: Arc::new(LocationSearch::new()),
        region: system::region(),
        layout: RefCell::new(SectionLayout::default()),
        shown: RefCell::new(None),
        shown_alerts: RefCell::new(None),
        refresh_problem: RefCell::new(None),
        hold: RefCell::new(SelectionHold::default()),
        generation: Cell::new(0),
        polling: Cell::new(false),
        poll_changed: Cell::new(false),
        first_poll: Cell::new(false),
        shown_at: Cell::new(Instant::now()),
        closed: Cell::new(false),
        normal: Cell::new(Rect::default()),
        maximized: Cell::new(false),
    });
    window.populate_locations();

    let w = window.clone();
    window.frame.on_menu(move |event| {
        let id = event.get_id();
        // On the Mac, wx goes back to the menu item once this returns, and
        // a command that rebuilds the menus (adding a location) has freed
        // it by then: a crash. There a command runs once the menu is done.
        if cfg!(target_os = "macos") {
            crate::on_ui(move |w| w.command(id));
        } else {
            w.command(id);
        }
    });
    let w = window.clone();
    window
        .choice
        .on_selection_changed(move |_| w.location_changed());
    let w = window.clone();
    window.text.on_key_down(move |event| {
        if let WindowEventData::Keyboard(key) = &event
            && matches!(key.get_key_code(), Some(WXK_RETURN | WXK_NUMPAD_ENTER))
            && !key.control_down()
            && !key.shift_down()
            && !key.alt_down()
            && let Some(alert) = w.alert_at_caret()
        {
            w.show_details(&alert);
            return;
        }
        event.skip(true);
    });
    edit::hook(&window.text, Some(double_clicked));
    window.normal.set(bounds(&window.frame));
    if maximized {
        window.frame.maximize(true);
    }
    window.maximized.set(maximized);
    let w = window.clone();
    window.frame.on_size(move |event| {
        w.track_bounds();
        event.skip(true);
    });
    let w = window.clone();
    window.frame.on_move_event(move |event| {
        w.track_bounds();
        event.skip(true);
    });
    let w = window.clone();
    window.frame.on_close(move |event| {
        w.remember_window();
        w.closed.set(true);
        w.generation.set(w.generation.get() + 1);
        w.forecast_timer.stop();
        w.alert_timer.stop();
        w.clock_timer.stop();
        event.skip(true);
    });

    // The forecast on screen, then every saved location's alerts, not just
    // the one on screen, so a warning for home is spoken while reading
    // somewhere else. Neither moves focus or the caret.
    let w = window.clone();
    window
        .forecast_timer
        .on_tick(move |_| w.refresh(true, true));
    let w = window.clone();
    window.alert_timer.on_tick(move |_| w.poll_alerts(true));
    let w = window.clone();
    window.clock_timer.on_tick(move |_| w.rewrite_if_changed());
    window
}

// Scaled by font, as 0.1 is: 0.1's 720 by 560 (at least 480 by 360) at
// Segoe UI 9 point, in characters, within the working area, or on a small
// screen at a large scale the title bar opens off the top; centred.
fn place_by_default(frame: &Frame) {
    let (w, h) = (frame.get_char_width(), frame.get_char_height());
    let mut size = Size::new(w * 103, h * 37);
    if let Some(display) = Display::from_window(frame) {
        let area = display.client_area();
        size = Size::new(size.width.min(area.width), size.height.min(area.height));
    }
    frame.set_size(size);
    frame.centre();
}

// The keys each system expects, the Mac's own conventions where it has
// them (Elliott, 2026-10-07). wx reads Ctrl as Command on the Mac. Mac
// laptops have no Page Down, and Command and Option with the arrows
// already move through text there, so the sections are Cmd+] and Cmd+[.
#[cfg(target_os = "macos")]
mod keys {
    pub const REFRESH: &str = "	Ctrl+R";
    pub const NEXT_SECTION: &str = "	Ctrl+]";
    pub const PREVIOUS_SECTION: &str = "	Ctrl+[";
    pub const GUIDE: &str = "	Ctrl+?";
    pub const SETTINGS: &str = "	Ctrl+,";
}
// Settings has no shortcut: Windows has no convention for a settings
// dialog, and Alt+S, S is two keys.
#[cfg(not(target_os = "macos"))]
mod keys {
    pub const REFRESH: &str = "	F5";
    pub const NEXT_SECTION: &str = "	Ctrl+PageDown";
    pub const PREVIOUS_SECTION: &str = "	Ctrl+PageUp";
    pub const GUIDE: &str = "	F1";
    pub const SETTINGS: &str = "";
}

fn menu_bar(settings: &AppSettings) -> MenuBar {
    let settings_item = format!("&Settings...{}", keys::SETTINGS);
    let mut file =
        Menu::builder().append_item(ID_REFRESH, &format!("&Refresh{}", keys::REFRESH), "");
    // The Mac shows Settings, About and Quit in the app's own menu, which
    // wx fills from these items (taking the label, shortcut included) and
    // hides them where they are. Settings goes in File there rather than
    // in a Settings menu that would be left empty.
    if cfg!(target_os = "macos") {
        file = file.append_item(ID_PREFERENCES, &settings_item, "");
    }
    let file = file
        .append_separator()
        .append_item(ID_EXIT, "E&xit	Alt+F4", "")
        .build();
    let view = Menu::builder()
        .append_item(
            ID_NEXT_SECTION,
            &format!("&Next Section{}", keys::NEXT_SECTION),
            "",
        )
        .append_item(
            ID_PREVIOUS_SECTION,
            &format!("&Previous Section{}", keys::PREVIOUS_SECTION),
            "",
        )
        .append_separator()
        .append_item(ID_ALERTS, "&Alerts	Ctrl+Shift+A", "")
        .append_separator()
        .append_item(ID_RESET_WINDOW, "&Reset Window Size and Position", "")
        .build();
    let help = Menu::builder()
        .append_item(ID_GUIDE, &format!("&User Guide{}", keys::GUIDE), "")
        .append_separator()
        .append_item(ID_ABOUT, "&About Weatherspell", "")
        .build();
    let mut bar = MenuBar::builder()
        .append(file, "&File")
        .append(locations_menu(settings), LOCATIONS_TITLE)
        .append(view, "&View");
    if !cfg!(target_os = "macos") {
        let settings_menu = Menu::builder()
            .append_item(ID_PREFERENCES, &settings_item, "")
            .build();
        bar = bar.append(settings_menu, "&Settings");
    }
    bar.append(help, "&Help").build()
}

const LOCATIONS_TITLE: &str = "&Locations";
const LOCATIONS_MENU: usize = 1;

// "&1 Home" with Ctrl+1, to "&9" with Ctrl+9, below Manage and Add: the
// menu shows each shortcut, and a location past the ninth is reached in
// the box. The one on screen is checked.
fn locations_menu(settings: &AppSettings) -> Menu {
    let mut menu = Menu::builder()
        .append_item(ID_MANAGE, "&Manage Locations...\tCtrl+L", "")
        .append_item(ID_ADD, "&Add Location...\tCtrl+Shift+L", "");
    if !settings.locations.is_empty() {
        menu = menu.append_separator();
    }
    for (i, saved) in settings.locations.iter().take(MENU_LOCATIONS).enumerate() {
        let name = saved.to_location().display_name().replace('&', "&&");
        menu = menu.append_check_item(
            ID_LOCATION + i as Id,
            &format!("&{} {name}\tCtrl+{}", i + 1, i + 1),
            "",
        );
    }
    menu.build()
}

// The font's average character as WinForms measured it for 0.1 (the
// alphabet's width over 52, in whole pixels, and the line height), so a
// window saved by either app opens the same size in the other.
fn char_size(window: &Frame) -> CharSize {
    let alphabet = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    CharSize {
        width: (f64::from(window.get_text_extent(alphabet).width) / 52.0).round(),
        height: f64::from(window.get_char_height()),
    }
}

// What each screen leaves for windows, the taskbar taken off.
fn working_areas() -> Vec<Rect> {
    Display::all()
        .map(|d| {
            let a = d.client_area();
            Rect::new(a.x, a.y, a.width, a.height)
        })
        .collect()
}

fn caption_height(window: &Frame) -> i32 {
    SystemSettings::get_metric(SystemMetric::CaptionY, Some(window))
}

fn bounds(window: &Frame) -> Rect {
    let (at, size) = (window.get_position(), window.get_size());
    Rect::new(at.x, at.y, size.width, size.height)
}

// From the text box's subclass, as the control finishes selecting the
// word under the pointer.
fn double_clicked(position: usize) {
    on_ui(move |w| w.double_clicked(position));
}

// "; frost advisory in effect", or nothing.
fn in_effect(alerts: &AlertReport) -> String {
    alerts::in_effect(alerts).map_or_else(String::new, |words| format!("; {words}"))
}

// 0.1 named a timeout this way; other failures name the host.
fn reason(e: &ServiceError) -> String {
    match e {
        ServiceError::Fetch(FetchError::TimedOut { .. }) => {
            "the weather service took too long to answer".to_string()
        }
        e => e.to_string(),
    }
}

impl MainWindow {
    pub fn closed(&self) -> bool {
        self.closed.get()
    }

    // For the lint's test, and the dialogs' parent.
    pub fn frame(&self) -> Frame {
        self.frame
    }

    fn track_bounds(&self) {
        if self.frame.is_iconized() {
            return;
        }
        let maximized = self.frame.is_maximized();
        self.maximized.set(maximized);
        if !maximized {
            self.normal.set(bounds(&self.frame));
        }
    }

    // Written only when it changed, so an ordinary close does not rewrite
    // settings.json, and never after settings that could not be read: the
    // user may be mending that file by hand while the app is open.
    fn remember_window(&self) {
        if self.load_problem.is_some() {
            return;
        }
        let window = placement::save(
            self.normal.get(),
            self.maximized.get(),
            char_size(&self.frame),
        );
        if self.settings.borrow().window == Some(window) {
            return;
        }
        self.settings.borrow_mut().window = Some(window);
        self.save_settings();
    }

    // As a first run places it, for a window that has been lost or made
    // awkward; remembered from then on as any other move is. Said, since
    // a screen reader user has nothing else to tell them it happened.
    fn reset_window(&self) {
        if self.frame.is_maximized() {
            self.frame.maximize(false);
        }
        place_by_default(&self.frame);
        self.announcer
            .say("The window is back to its default size and position.");
    }

    pub fn show(&self) {
        self.frame.show(true);
        self.shown_at.set(Instant::now());
        self.text.set_focus();
        if let Some(problem) = &self.load_problem {
            // Said before anything else opens over it, where the status
            // bar alone went unnoticed (#22).
            self.status(&format!("Settings could not be read: {}", problem.error));
            MessageDialog::builder(&self.frame, &problem.message, APP_NAME)
                .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconWarning)
                .build()
                .show_modal();
        }
        self.start_forecast_timer();
        self.start_alert_timer();
        self.clock_timer.start(60_000, false);
        // Not after settings that could not be read: no location is
        // loaded, so every location's file would go.
        if self.load_problem.is_none() {
            let keep: Vec<Location> = self
                .settings
                .borrow()
                .locations
                .iter()
                .map(|s| s.to_location())
                .collect();
            self.try_cache(|c| c.prune(&keep));
        }
        // A first run: the window with Add Location open over it, focus in
        // the search field.
        if self.settings.borrow().locations.is_empty() {
            self.show_welcome();
            self.add_location();
            return;
        }
        self.first_poll.set(true);
        self.refresh(false, false);
    }

    // The intervals from the settings; a running timer restarts from now.
    fn start_forecast_timer(&self) {
        let minutes = self.settings.borrow().forecast_refresh_minutes;
        self.forecast_timer.start(minutes as i32 * 60_000, false);
    }

    fn start_alert_timer(&self) {
        let minutes = self.settings.borrow().alert_check_minutes;
        self.alert_timer.start(minutes as i32 * 60_000, false);
    }

    fn command(&self, id: Id) {
        match id {
            ID_REFRESH => self.refresh(true, false),
            ID_EXIT => {
                self.frame.close(false);
            }
            ID_NEXT_SECTION => self.jump(true),
            ID_PREVIOUS_SECTION => self.jump(false),
            ID_ALERTS => {
                let first = self.layout.borrow().headings.first().cloned();
                if let Some((heading, at)) = first {
                    self.move_caret(&heading, at);
                }
            }
            ID_MANAGE => self.manage_locations(),
            ID_ADD => self.add_location(),
            ID_PREFERENCES => self.show_settings(),
            ID_GUIDE => guide::open(&self.frame, None),
            ID_RESET_WINDOW => self.reset_window(),
            ID_ABOUT => about::show(&self.frame),
            _ if (ID_LOCATION..ID_LOCATION + MENU_LOCATIONS as Id).contains(&id) => {
                self.show_location((id - ID_LOCATION) as usize);
            }
            _ => {}
        }
    }

    // A place already saved is switched to, not saved twice under the same
    // name (#22).
    fn add_location(&self) {
        let Some(place) = add_location::show(&self.frame, &self.http, &self.search) else {
            return;
        };
        let saved = self.settings.borrow().index_of(&place);
        if let Some(index) = saved {
            let name = self.settings.borrow().locations[index]
                .to_location()
                .display_name();
            self.say_later(format!("{name} is already saved."), Instant::now(), None);
            if self.current_index() != Some(index) {
                self.choice.set_selection(index as u32);
                self.location_changed();
            }
            self.text.set_focus();
            return;
        }
        {
            let mut settings = self.settings.borrow_mut();
            settings
                .locations
                .push(SavedLocation::from_location(&place));
            settings.last_location = settings.locations.len() - 1;
        }
        self.save_settings();
        self.populate_locations();
        self.refresh(false, false);
        self.text.set_focus();
    }

    // The edits land only on OK (see LocationEditor). The location on
    // screen stays on screen wherever it has moved, and a rename leaves its
    // text as it is; if it was removed, the one now in its place is shown.
    fn manage_locations(&self) {
        let before = self.current_index();
        let saved = self.settings.borrow().locations.clone();
        let Some(editor) = manage::show(&self.frame, &saved, before, &self.http, &self.search)
        else {
            return;
        };
        // From the saved list as it is now: an alert check may have marked
        // alerts seen while the dialog was open.
        let committed = editor.commit(&self.settings.borrow().locations);
        let show = editor.shown(before);
        let still_shown =
            before.is_some() && show.is_some_and(|i| editor.entries()[i].original == before);
        {
            let mut settings = self.settings.borrow_mut();
            settings.locations = committed;
            settings.last_location = show.unwrap_or(0);
        }
        self.save_settings();
        // A removed location's file goes now rather than at the next
        // launch; not after settings that could not be read (see show).
        if self.load_problem.is_none() {
            let keep: Vec<Location> = self
                .settings
                .borrow()
                .locations
                .iter()
                .map(|s| s.to_location())
                .collect();
            self.try_cache(|c| c.prune(&keep));
        }
        self.populate_locations();
        if show.is_none() {
            self.show_welcome();
            self.status("Ready");
        } else if !still_shown {
            self.refresh(false, false);
        }
    }

    fn show_settings(&self) {
        let settings = self.settings.borrow().clone();
        settings_dialog::show(&self.frame, &settings, |chosen| {
            on_ui(move |w| w.apply_settings(chosen))
        });
    }

    // The new intervals start from now.
    fn apply_settings(&self, chosen: settings_dialog::Chosen) {
        {
            let mut settings = self.settings.borrow_mut();
            settings.forecast_refresh_minutes = chosen.forecast_minutes;
            settings.alert_check_minutes = chosen.alert_minutes;
            settings.alert_announcements = chosen.announcements;
        }
        self.save_settings();
        self.start_forecast_timer();
        self.start_alert_timer();
    }

    fn status(&self, text: &str) {
        self.frame.set_status_text(text, 0);
    }

    fn populate_locations(&self) {
        let settings = self.settings.borrow();
        self.choice.clear();
        for saved in &settings.locations {
            self.choice.append(&saved.to_location().display_name());
        }
        if let Some(bar) = self.frame.get_menu_bar()
            && let Some(mut old) =
                bar.replace(LOCATIONS_MENU, locations_menu(&settings), LOCATIONS_TITLE)
        {
            old.destroy_menu();
        }
        if !settings.locations.is_empty() {
            let index = settings.last_location.min(settings.locations.len() - 1);
            self.choice.set_selection(index as u32);
            drop(settings);
            self.check_location(index);
        }
    }

    fn check_location(&self, index: usize) {
        if let Some(bar) = self.frame.get_menu_bar() {
            let count = self.settings.borrow().locations.len().min(MENU_LOCATIONS);
            for i in 0..count {
                bar.check_item(ID_LOCATION + i as Id, i == index);
            }
        }
    }

    fn current_index(&self) -> Option<usize> {
        let count = self.settings.borrow().locations.len();
        self.choice
            .get_selection()
            .map(|i| i as usize)
            .filter(|i| *i < count)
    }

    // Ctrl+1 to Ctrl+9. Focus stays where it is, so the location is named;
    // the refresh then says when its forecast is ready, as for the box.
    fn show_location(&self, index: usize) {
        let name = match self.settings.borrow().locations.get(index) {
            Some(saved) => saved.to_location().display_name(),
            None => return,
        };
        self.choice.set_selection(index as u32);
        self.announcer.say(&name);
        self.location_changed();
    }

    fn location_changed(&self) {
        let Some(index) = self.current_index() else {
            return;
        };
        self.check_location(index);
        self.settings.borrow_mut().last_location = index;
        self.save_settings();
        self.refresh(false, false);
    }

    fn show_welcome(&self) {
        self.generation.set(self.generation.get() + 1);
        *self.shown.borrow_mut() = None;
        *self.shown_alerts.borrow_mut() = None;
        *self.refresh_problem.borrow_mut() = None;
        self.set_text(&[Section::new(
            "Welcome",
            vec![
                "No location yet. Press Ctrl+Shift+L, or use Locations > Add Location, to add one."
                    .to_string(),
            ],
        )]);
    }

    // keep_caret: F5 and the timer replace the text under the reader, who
    // stays on the same words; a location switch starts from the top.
    fn refresh(&self, keep_caret: bool, automatic: bool) {
        let Some(index) = self.current_index() else {
            return;
        };
        let location = self.settings.borrow().locations[index].to_location();
        let generation = self.generation.get() + 1;
        self.generation.set(generation);
        let started = Instant::now();

        if !automatic {
            self.status(&format!(
                "Fetching the forecast for {}...",
                location.display_name()
            ));
        }
        // A launch or a switch says what is happening rather than showing
        // an empty box or the previous location's text; F5 and the timer
        // keep the text, and the reader's place in it, until there is
        // something new.
        if !keep_caret {
            self.show_fetching(&location);
        }

        let units = units::for_location(&location, self.region.units);
        let http = self.http.clone();
        let service = self.service.clone();
        std::thread::spawn(move || {
            let work = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let now = Timestamp::now();
                // The alert check reduces its own failures to a report, so
                // it is waited for either way: an alert service that answers
                // while the forecast's does not still gives live alerts.
                std::thread::scope(|s| {
                    let alerts = s.spawn(|| check_alerts(&*http, &location, now));
                    let forecast = service.forecast(&*http, &location, units, FORECAST_DAYS, now);
                    (forecast, alerts.join())
                })
            }));
            match work {
                Ok((forecast, Ok(alerts))) => on_ui(move |w| {
                    w.fetched(Fetched {
                        generation,
                        index,
                        location,
                        forecast,
                        alerts,
                        keep_caret,
                        automatic,
                        started,
                    })
                }),
                _ => on_ui(move |w| w.went_wrong(generation, automatic)),
            }
        });
    }

    // The last resort for a refresh: a failure no expected case covers
    // ends in the status bar and, while the text is still waiting, a
    // Problem section, rather than the text left at "Fetching" (#22).
    fn went_wrong(&self, generation: u64, automatic: bool) {
        if generation != self.generation.get() {
            return;
        }
        self.status("Something went wrong.");
        if self.shown.borrow().is_none() {
            self.set_text(&[Section::new(
                "Problem",
                vec![
                    "Something went wrong.".to_string(),
                    "Press F5 to try again.".to_string(),
                ],
            )]);
        }
        if !automatic {
            self.announcer
                .say("Something went wrong. Press F5 to try again.");
        }
    }

    fn fetched(&self, f: Fetched) {
        if f.generation != self.generation.get() {
            return;
        }
        // The saved list cannot change under a refresh yet, but a location
        // that is no longer where it was is not written to.
        let same = self
            .settings
            .borrow()
            .locations
            .get(f.index)
            .is_some_and(|s| s.to_location().is_same_place(&f.location));
        if !same {
            return;
        }
        let name = f.location.display_name();
        match f.forecast {
            Err(e) => {
                let failure = self.show_failure(
                    f.index,
                    &f.location,
                    &reason(&e),
                    &f.alerts,
                    f.keep_caret,
                    f.automatic,
                );
                if let Some(text) = failure {
                    self.say_later(text, f.started, Some(f.generation));
                }
            }
            Ok(forecast) => {
                self.settings.borrow_mut().locations[f.index].utc_offset_seconds =
                    Some(forecast.utc_offset.seconds());
                *self.refresh_problem.borrow_mut() = None;
                self.status(&format!(
                    "{name}: updated {} from {}",
                    self.pc_time(Timestamp::now()),
                    forecast.source_name
                ));
                let cached = if f.alerts.checked() || !f.alerts.is_available() {
                    Some(&f.alerts)
                } else {
                    None
                };
                self.try_cache(|c| c.save(&f.location, &forecast, cached));
                if f.keep_caret {
                    self.rewrite(forecast, Some(f.alerts.clone()), f.automatic);
                } else {
                    self.show_forecast(forecast, f.alerts.clone());
                }
                // The next automatic refresh a full interval from this one.
                self.start_forecast_timer();
                self.track_alerts(f.index, &f.alerts, f.keep_caret);
                self.save_settings();
                // Neither the new text nor F5's rewrite makes a sound of its
                // own (NVDA ignores an edit control's text changing); the
                // timer's rewrites stay silent by design. A launch or a
                // switch names the alerts in effect, which track_alerts has
                // marked seen unspoken; F5's new ones were announced there.
                if !f.automatic {
                    let text = if f.keep_caret {
                        format!("{name}: forecast updated.")
                    } else {
                        format!("{name}: forecast ready{}.", in_effect(&f.alerts))
                    };
                    self.say_later(text, f.started, Some(f.generation));
                }
            }
        }
        if self.first_poll.replace(false) {
            self.poll_alerts(false);
        }
    }

    // NVDA drops a notification from a window it has not yet seen come to
    // the front, and speaks one raised straight after a key ahead of its
    // own report of that key (the box's new location). A fetch that fails
    // at once did both, so what a refresh has to say waits until the window
    // has been up for a moment and the key that started it has been
    // answered (#17).
    // generation: dropped if a later refresh has started; None for what
    // holds whatever happens next.
    fn say_later(&self, text: String, started: Instant, generation: Option<u64>) {
        let wait = Duration::from_millis(1500)
            .saturating_sub(self.shown_at.get().elapsed())
            .max(Duration::from_millis(400).saturating_sub(started.elapsed()));
        if wait.is_zero() {
            self.announcer.say(&text);
            return;
        }
        std::thread::spawn(move || {
            std::thread::sleep(wait);
            on_ui(move |w| {
                if generation.is_none_or(|g| g == w.generation.get()) {
                    w.announcer.say(&text);
                }
            });
        });
    }

    fn show_fetching(&self, location: &Location) {
        *self.shown.borrow_mut() = None;
        *self.shown_alerts.borrow_mut() = None;
        *self.refresh_problem.borrow_mut() = None;
        self.set_text(&[Section::new(
            &format!("Fetching the forecast for {}...", location.display_name()),
            Vec::new(),
        )]);
    }

    // What a failed fetch leaves on screen, dated by the age line under
    // Right now rather than giving way to an error: the text already there
    // (F5, the timer), else the cached text, else a Problem section. A
    // failure the user asked for is spoken (the sentence is returned for
    // the caller to say); the timer's is left for the age line to tell.
    // Live alerts go with whichever text is shown; a failed check keeps
    // the last known alerts, dated.
    fn show_failure(
        &self,
        index: usize,
        location: &Location,
        reason: &str,
        alerts: &AlertReport,
        keep_caret: bool,
        automatic: bool,
    ) -> Option<String> {
        let name = location.display_name();
        *self.refresh_problem.borrow_mut() = Some("couldn't reach the weather service".to_string());
        let shown = self
            .shown
            .borrow()
            .clone()
            .filter(|f| f.location.is_same_place(location));
        let spoken = if let Some(shown) = shown {
            let fetched_at = shown.fetched_at;
            let earlier = self.shown_alerts.borrow().clone();
            self.rewrite(
                shown,
                Some(alerts.or_last_known(earlier.as_ref())),
                automatic,
            );
            self.status(&format!("Couldn't fetch the forecast for {name}: {reason}"));
            format!(
                "Couldn't fetch the forecast for {name}. Showing the forecast from {}.",
                self.pc_time_on_day(fetched_at)
            )
        } else if let Some(cached) = self.cache.load(location) {
            let fetched_at = cached.forecast.fetched_at;
            self.show_forecast(
                cached.forecast,
                alerts.or_last_known(cached.alerts.as_ref()),
            );
            self.status(&format!("Couldn't fetch the forecast for {name}: {reason}"));
            format!(
                "Couldn't fetch the forecast for {name}. Showing the forecast from {}{}.",
                self.pc_time_on_day(fetched_at),
                if keep_caret {
                    String::new()
                } else {
                    in_effect(alerts)
                }
            )
        } else {
            *self.refresh_problem.borrow_mut() = None;
            self.set_text(&[Section::new(
                "Problem",
                vec![
                    format!("Couldn't fetch the forecast for {name}: {reason}"),
                    "Press F5 to try again.".to_string(),
                ],
            )]);
            self.status(&format!("Couldn't fetch the forecast for {name}."));
            format!("Couldn't fetch the forecast for {name}.")
        };
        if alerts.checked() {
            self.track_alerts(index, alerts, keep_caret);
            self.save_settings();
            self.try_cache(|c| c.save_alerts(location, alerts));
        }
        (!automatic).then_some(spoken)
    }

    // announce: F5 keeps the caret, so an alert that has appeared above it
    // is spoken; a location shown from the top is read from its Alerts
    // line anyway.
    fn track_alerts(&self, index: usize, alerts: &AlertReport, announce: bool) {
        if !alerts.checked() {
            return;
        }
        let (fresh, notify) = {
            let mut settings = self.settings.borrow_mut();
            let saved = &mut settings.locations[index];
            (
                alerts::track(&mut saved.seen_alert_ids, &alerts.alerts),
                saved.notify_alerts,
            )
        };
        if announce && notify {
            self.announce(index, &fresh);
        }
    }

    fn announce(&self, index: usize, fresh: &[WeatherAlert]) {
        let settings = self.settings.borrow();
        let spoken: Vec<WeatherAlert> = fresh
            .iter()
            .filter(|a| settings.announces(a.severity))
            .cloned()
            .collect();
        if spoken.is_empty() {
            return;
        }
        let saved = &settings.locations[index];
        // The location's own time, from its last forecast; a location whose
        // forecast never loaded is announced in this PC's time.
        let now = Timestamp::now();
        let offset = saved
            .utc_offset_seconds
            .and_then(|s| Offset::from_seconds(s).ok())
            .unwrap_or_else(|| self.region.zone.to_offset(now));
        let clock = Clock::new(
            offset,
            self.region.zone.clone(),
            self.region.time_format.clone(),
        );
        let text = alerts::announcement(
            &saved.to_location().display_name(),
            &spoken,
            &clock,
            offset.to_datetime(now),
        );
        drop(settings);
        self.announcer.alert(&text);
    }

    fn options(&self) -> WriterOptions {
        WriterOptions {
            now: Timestamp::now(),
            pc_zone: self.region.zone.clone(),
            time_format: self.region.time_format.clone(),
            refresh_problem: self.refresh_problem.borrow().clone(),
        }
    }

    // The text for what is shown, as it reads now.
    fn render(&self) -> Option<SectionLayout> {
        let shown = self.shown.borrow();
        let forecast = shown.as_ref()?;
        let alerts = self.shown_alerts.borrow();
        Some(SectionLayout::build(&writer::write(
            forecast,
            &self.options(),
            alerts.as_ref(),
        )))
    }

    fn show_forecast(&self, forecast: Forecast, alerts: AlertReport) {
        *self.shown.borrow_mut() = Some(forecast);
        *self.shown_alerts.borrow_mut() = Some(alerts);
        if let Some(layout) = self.render() {
            self.apply(layout, 0);
        }
    }

    // New text with the caret kept on the same words (map_caret) and the
    // view where it was, so nothing the user did not ask for moves them;
    // see SectionLayout::plan for when the text box is left alone.
    fn rewrite(&self, forecast: Forecast, alerts: Option<AlertReport>, automatic: bool) {
        *self.shown.borrow_mut() = Some(forecast);
        *self.shown_alerts.borrow_mut() = alerts;
        let Some(layout) = self.render() else {
            return;
        };
        let selecting = self
            .hold
            .borrow_mut()
            .holds(self.selecting(), Timestamp::now());
        let plan = SectionLayout::plan(&self.layout.borrow(), &layout, selecting, automatic);
        match plan {
            RewritePlan::KeepText => {
                *self.layout.borrow_mut() = layout;
                self.hold.borrow_mut().release();
            }
            RewritePlan::Replace => self.replace(layout),
            RewritePlan::Wait => {}
        }
    }

    // The clock's tick: only when the text has changed (the age line from
    // 30 minutes, a day heading at midnight, or a rewrite that waited for
    // a selection), and not under a selection the user may be about to
    // copy, unless it has been there too long (SelectionHold).
    fn rewrite_if_changed(&self) {
        let Some(layout) = self.render() else {
            return;
        };
        if layout.text == self.layout.borrow().text {
            self.hold.borrow_mut().release();
            return;
        }
        if self
            .hold
            .borrow_mut()
            .holds(self.selecting(), Timestamp::now())
        {
            return;
        }
        self.replace(layout);
    }

    fn replace(&self, layout: SectionLayout) {
        self.hold.borrow_mut().release();
        let caret = layout.map_caret(&self.layout.borrow(), self.caret());
        let view = edit::view(&self.text);
        self.text.change_value(&layout.text);
        *self.layout.borrow_mut() = layout;
        self.set_caret(caret);
        edit::restore(&self.text, view);
    }

    fn set_text(&self, sections: &[Section]) {
        self.apply(SectionLayout::build(sections), 0);
    }

    fn apply(&self, layout: SectionLayout, caret: usize) {
        self.hold.borrow_mut().release();
        self.text.change_value(&layout.text);
        *self.layout.borrow_mut() = layout;
        self.set_caret(caret);
    }

    // The text box counts positions its own way (edit.rs); these are the
    // layout's offsets.
    fn native(&self, offset: usize) -> i64 {
        if cfg!(windows) {
            self.layout.borrow().crlf_position(offset) as i64
        } else {
            offset as i64
        }
    }

    fn offset(&self, native: i64) -> usize {
        let native = native.max(0) as usize;
        if cfg!(windows) {
            self.layout.borrow().offset_from_crlf(native)
        } else {
            native
        }
    }

    // Where the selection starts, which is the caret when nothing is
    // selected.
    fn caret(&self) -> usize {
        self.offset(self.text.get_selection().0)
    }

    fn selecting(&self) -> bool {
        let (start, end) = self.text.get_selection();
        start != end
    }

    fn set_caret(&self, offset: usize) {
        let offset = offset.min(self.layout.borrow().length);
        self.text.set_insertion_point(self.native(offset));
    }

    fn alert_at_caret(&self) -> Option<WeatherAlert> {
        self.layout.borrow().alert_at(self.caret()).cloned()
    }

    fn double_clicked(&self, position: usize) {
        let offset = self.offset(position as i64);
        let alert = self.layout.borrow().alert_at(offset).cloned();
        if let Some(alert) = alert {
            // A selection left behind would hold off the next automatic
            // refresh.
            self.set_caret(offset);
            self.show_details(&alert);
        }
    }

    fn show_details(&self, alert: &WeatherAlert) {
        let Some(offset) = self.shown.borrow().as_ref().map(|f| f.utc_offset) else {
            return;
        };
        let clock = Clock::new(
            offset,
            self.region.zone.clone(),
            self.region.time_format.clone(),
        );
        let paragraphs = alerts::details(alert, &clock, offset.to_datetime(Timestamp::now()));
        details::show(&self.frame, &alert.event, &paragraphs, alert.url.as_deref());
    }

    fn jump(&self, forward: bool) {
        let here = self.caret();
        let target = {
            let layout = self.layout.borrow();
            if forward {
                layout.headings.iter().find(|(_, at)| *at > here).cloned()
            } else {
                layout
                    .headings
                    .iter()
                    .rev()
                    .find(|(_, at)| *at < here)
                    .cloned()
            }
        };
        match target {
            Some((heading, at)) => self.move_caret(&heading, at),
            None => self.announcer.say(if forward {
                "No next section."
            } else {
                "No previous section."
            }),
        }
    }

    // NVDA reads the new line only after the navigation keys it knows, so
    // the heading is spoken (#17); when the jump brings focus into the
    // text, NVDA reads the line itself as focus arrives. Narrator reads
    // the line at any caret move, so with Narrator alone the heading was
    // heard twice (ScreenReaders).
    fn move_caret(&self, heading: &str, at: usize) {
        let speak = self.text.has_focus();
        self.text.set_focus();
        self.set_caret(at);
        if speak && ScreenReaders::running().need_caret_moves_spoken() {
            self.announcer.say(heading);
        }
    }

    // Every saved location that asks to be notified, plus the one on screen
    // so its Alerts section stays current. A source that cannot be reached
    // leaves that location's seen list alone, so nothing is announced twice
    // after an outage. Never moves focus or the caret.
    fn poll_alerts(&self, include_current: bool) {
        if self.polling.replace(true) {
            return;
        }
        let current = self.current_index();
        let targets: Vec<(usize, Location)> = self
            .settings
            .borrow()
            .locations
            .iter()
            .enumerate()
            .filter(|(i, s)| {
                if Some(*i) == current {
                    include_current
                } else {
                    s.notify_alerts
                }
            })
            .map(|(i, s)| (i, s.to_location()))
            .collect();
        let http = self.http.clone();
        std::thread::spawn(move || {
            for (index, location) in targets {
                let report = check_alerts(&*http, &location, Timestamp::now());
                on_ui(move |w| w.polled(index, &location, &report));
            }
            on_ui(|w| w.poll_done());
        });
    }

    fn polled(&self, index: usize, location: &Location, report: &AlertReport) {
        let saved = self.settings.borrow().locations.get(index).cloned();
        let Some(saved) = saved.filter(|s| s.to_location().is_same_place(location)) else {
            return;
        };
        if report.checked() {
            let fresh = {
                let mut settings = self.settings.borrow_mut();
                let seen = &mut settings.locations[index].seen_alert_ids;
                let before = seen.clone();
                let fresh = alerts::track(seen, &report.alerts);
                if *seen != before {
                    self.poll_changed.set(true);
                }
                fresh
            };
            if saved.notify_alerts {
                self.announce(index, &fresh);
            }
            self.try_cache(|c| c.save_alerts(location, report));
        }
        // Only if the text on screen is still this location's: the user may
        // have switched while the check was in flight. A check that failed
        // dates the alerts it leaves on screen.
        let shown = self
            .shown
            .borrow()
            .clone()
            .filter(|f| f.location.is_same_place(location));
        let earlier = self.shown_alerts.borrow().clone();
        if self.current_index() == Some(index)
            && let (Some(shown), Some(earlier)) = (shown, earlier)
        {
            self.rewrite(shown, Some(report.or_last_known(Some(&earlier))), true);
        }
    }

    fn poll_done(&self) {
        self.polling.set(false);
        if self.poll_changed.replace(false) {
            self.save_settings();
        }
    }

    fn pc_time(&self, t: Timestamp) -> String {
        self.region
            .time_format
            .format(t.to_zoned(self.region.zone.clone()).datetime())
    }

    fn pc_time_on_day(&self, t: Timestamp) -> String {
        let today = Timestamp::now().to_zoned(self.region.zone.clone()).date();
        self.region
            .time_format
            .format_on_day(t.to_zoned(self.region.zone.clone()).datetime(), today)
    }

    // The cache is a convenience: a folder that cannot be written costs the
    // offline view, not the forecast.
    fn try_cache(&self, write: impl FnOnce(&ForecastCache) -> io::Result<()>) {
        if let Err(e) = write(&self.cache) {
            self.status(&format!("Couldn't keep the forecast for offline use: {e}"));
        }
    }

    fn save_settings(&self) {
        let result = self.store.save(&mut self.settings.borrow_mut());
        if let Err(e) = result {
            self.status(&format!("Couldn't save settings: {e}"));
        }
    }
}
