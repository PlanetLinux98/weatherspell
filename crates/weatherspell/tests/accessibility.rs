// Every window and dialog of the app, built but not shown, checked by the
// accessibility lint (wx_accessibility::lint), as 0.1's tests linted every
// form; and a dialog made wrong on purpose, which the lint must catch. A
// main of its own (harness = false): wxWidgets wants the main thread.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use weatherspell::{about, add_location, details, manage, settings_dialog, window};
use weatherspell_core::location::Location;
use weatherspell_core::net::UreqFetch;
use weatherspell_core::search::LocationSearch;
use weatherspell_core::settings::{AppSettings, SavedLocation, SettingsStore};
use wx_accessibility::lint;
use wxdragon::prelude::*;

// How many windows were checked, their problems, and what the lint found
// in the one made wrong.
#[derive(Default)]
struct Results {
    windows: usize,
    problems: Vec<String>,
    caught: Vec<String>,
}

fn main() {
    // The app's own settings, made up: two saved locations for Manage
    // Locations to list. Nothing is fetched.
    let folder = std::env::temp_dir().join(format!("weatherspell-lint-{}", std::process::id()));
    let mut settings = AppSettings::default();
    let mut home = Location::new(
        "Peterborough",
        Some("Ontario"),
        Some("Canada"),
        44.3,
        -78.32,
    );
    home.nickname = Some("Home".to_string());
    settings.locations.push(SavedLocation::from_location(&home));
    settings
        .locations
        .push(SavedLocation::from_location(&Location::new(
            "Paris",
            Some("Ile-de-France"),
            Some("France"),
            48.85,
            2.35,
        )));
    SettingsStore::in_folder(&folder)
        .save(&mut settings)
        .unwrap();
    // Before any thread is started, so setting them is safe.
    unsafe {
        std::env::set_var("WEATHERSPELL_DATA", &folder);
        std::env::set_var("WEATHERSPELL_OFFLINE", "1");
    }

    let results: Rc<RefCell<Results>> = Rc::default();
    let r = results.clone();
    let saved = settings.locations.clone();
    let _ = wxdragon::main(move |_| {
        // As the app does, so WEATHERSPELL_APPEARANCE=dark lints dark mode.
        weatherspell::follow_appearance();
        let main = window::build();
        let frame = main.frame();
        let http = Arc::new(UreqFetch::new("Weatherspell-lint"));
        let search = Arc::new(LocationSearch::new());
        let dialogs = [
            add_location::build(&frame, &http, &search),
            manage::build(&frame, &saved, &http, &search),
            manage::build_edit(&frame, "Peterborough, Ontario, Canada"),
            settings_dialog::build(&frame, &AppSettings::default(), |_| {}),
            about::build(&frame),
            details::build(
                &frame,
                "Frost advisory",
                &["Frost advisory from Environment Canada.".to_string()],
                Some("https://weather.gc.ca/"),
            ),
        ];
        let mut problems = lint::check(&frame);
        for dialog in &dialogs {
            problems.extend(lint::check(dialog));
            dialog.destroy();
        }

        // Made wrong: a text box with nothing before it, and two buttons
        // with the same Alt key.
        let wrong = Dialog::builder(&frame, "Made wrong").build();
        let _unlabelled = TextCtrl::builder(&wrong).build();
        let _go = Button::builder(&wrong).with_label("&Go").build();
        let _get = Button::builder(&wrong).with_label("&Get").build();
        let caught = lint::check(&wrong);
        wrong.destroy();

        *r.borrow_mut() = Results {
            windows: 1 + dialogs.len(),
            problems,
            caught,
        };
        frame.destroy();
    });
    let _ = std::fs::remove_dir_all(&folder);

    let Results {
        windows,
        problems,
        caught,
    } = &*results.borrow();
    let mut failed = false;
    if *windows == 0 {
        eprintln!("No window was built.");
        failed = true;
    }
    for problem in problems {
        eprintln!("{problem}");
        failed = true;
    }
    let expected = [
        "Made wrong: the first text box has no label before it, so a screen reader has no name for it.",
        "Made wrong: Alt+G is given by \"Go\" and \"Get\".",
    ];
    // The lint reads Windows' controls only so far; elsewhere this still
    // proves every window builds.
    if cfg!(windows) && caught != &expected {
        eprintln!("The lint did not catch what was made wrong: {caught:?}");
        failed = true;
    }
    if failed {
        std::process::exit(1);
    }
    if cfg!(windows) {
        println!(
            "{windows} windows checked, none with a problem; the lint caught what was made wrong."
        );
    } else {
        println!("{windows} windows built; the lint checks Windows only so far.");
    }
}
