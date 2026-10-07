// The main window opens where it was last closed (#25), checked on the Mac
// only: it shows a real window, which elsewhere would take focus from
// whoever is at the computer, while the Mac's CI runner has a screen of its
// own. What the restore had to go on is printed, so a failure says why. A
// main of its own (harness = false): wxWidgets wants the main thread.

use std::cell::RefCell;
use std::rc::Rc;

use weatherspell::window;
use weatherspell_core::location::Location;
use weatherspell_core::settings::{AppSettings, SavedLocation, SavedWindow, SettingsStore};
use wxdragon::prelude::*;

#[derive(Default)]
struct Seen {
    saved: Option<SavedWindow>,
    built: Option<(Point, Size)>,
    shown: Option<(Point, Size)>,
}

fn main() {
    if !cfg!(target_os = "macos") {
        println!("Window placement is checked on the Mac only.");
        return;
    }
    let folder =
        std::env::temp_dir().join(format!("weatherspell-placement-{}", std::process::id()));
    // Before any thread is started, so setting them is safe.
    unsafe {
        std::env::set_var("WEATHERSPELL_DATA", &folder);
        std::env::set_var("WEATHERSPELL_OFFLINE", "1");
    }

    let seen: Rc<RefCell<Seen>> = Rc::default();
    let s = seen.clone();
    let data = folder.clone();
    let _ = wxdragon::main(move |_| {
        let areas: Vec<Rect> = Display::all().map(|d| d.client_area()).collect();
        println!("Working areas: {areas:?}");
        let Some(first) = areas.first() else {
            return;
        };
        // A place on the first screen whatever its size; no character size,
        // so the size is taken as it is.
        let saved = SavedWindow {
            left: first.x + 60,
            top: first.y + 80,
            width: 640,
            height: 480,
            maximized: false,
            char_width: 0.0,
            char_height: 0.0,
        };
        let mut settings = AppSettings::default();
        // A saved location, so no Add Location opens over the window.
        settings
            .locations
            .push(SavedLocation::from_location(&Location::new(
                "Peterborough",
                Some("Ontario"),
                Some("Canada"),
                44.3,
                -78.32,
            )));
        settings.window = Some(saved);
        SettingsStore::in_folder(&data).save(&mut settings).unwrap();
        s.borrow_mut().saved = Some(saved);

        let main = window::build();
        let frame = main.frame();
        println!(
            "Built: frame {:?} {:?}, client {:?}",
            frame.get_position(),
            frame.get_size(),
            frame.get_client_size()
        );
        s.borrow_mut().built = Some((frame.get_position(), frame.get_size()));
        main.show();

        let timer = Timer::new(&frame);
        let s = s.clone();
        timer.on_tick(move |_| {
            println!(
                "Shown: frame {:?} {:?}, client {:?}",
                frame.get_position(),
                frame.get_size(),
                frame.get_client_size()
            );
            s.borrow_mut().shown = Some((frame.get_position(), frame.get_size()));
            frame.destroy();
        });
        timer.start(1500, true);
        // Kept for the tick; the process ends soon after.
        std::mem::forget(timer);
        std::mem::forget(main);
    });
    let _ = std::fs::remove_dir_all(&folder);

    let seen = seen.borrow();
    let (Some(saved), Some(shown)) = (seen.saved, seen.shown) else {
        eprintln!("The window was not shown.");
        std::process::exit(1);
    };
    let (at, size) = shown;
    let close = |a: i32, b: i32| (a - b).abs() <= 2;
    if !(close(at.x, saved.left)
        && close(at.y, saved.top)
        && close(size.width, saved.width)
        && close(size.height, saved.height))
    {
        eprintln!(
            "Saved at {},{} {}x{}; built at {:?}; shown at {:?} {:?}.",
            saved.left, saved.top, saved.width, saved.height, seen.built, at, size
        );
        std::process::exit(1);
    }
    println!("The window opened where it was saved.");
}
