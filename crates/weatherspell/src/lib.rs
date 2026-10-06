// Weatherspell in Rust and wxWidgets (#24): the app, kept thin, over
// weatherspell-core, which holds all the logic. Windows first; the Mac and
// Linux follow once it works fully here. A library under a small exe
// (main.rs), so tests can build its windows (tests/accessibility.rs).

pub mod about;
pub mod add_location;
pub mod details;
mod dialogs;
mod edit;
mod guide;
pub mod manage;
pub mod settings_dialog;
mod system;
pub mod window;

// The build scripts' own code, so their tests run with the app's.
#[cfg(test)]
#[path = "../build/guide.rs"]
mod guide_page;
#[cfg(test)]
#[path = "../build/version.rs"]
mod version;

use std::cell::RefCell;
use std::rc::Rc;

use window::MainWindow;

// The version from the git tag (build/version.rs).
pub const VERSION: &str = env!("WEATHERSPELL_VERSION");

thread_local! {
    static WINDOW: RefCell<Option<Rc<MainWindow>>> = const { RefCell::new(None) };
}

// Runs on the UI thread, from any thread: a fetch's result, an
// announcement that waited. Dropped once the window is closing.
pub fn on_ui(work: impl FnOnce(&MainWindow) + Send + 'static) {
    on_ui_thread(move || {
        let window = WINDOW.with(|w| w.borrow().clone());
        if let Some(window) = window
            && !window.closed()
        {
            work(&window);
        }
    });
}

// The same for a dialog, which finds its own state (add_location.rs). The
// queue is run in a modal dialog's loop too.
pub fn on_ui_thread(work: impl FnOnce() + Send + 'static) {
    wxdragon::call_after(Box::new(work));
    wxdragon::wake_up_idle();
}

pub fn run() {
    if system::another_copy_brought_forward() {
        return;
    }
    let _ = wxdragon::main(|_| {
        let window = window::build();
        WINDOW.with(|w| *w.borrow_mut() = Some(window.clone()));
        window.show();
    });
}
