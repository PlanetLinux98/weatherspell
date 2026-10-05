// Weatherspell in Rust and wxWidgets (#24): the app, kept thin, over
// weatherspell-core, which holds all the logic. Windows first; the Mac and
// Linux follow once it works fully here.

#![windows_subsystem = "windows"]

mod details;
mod edit;
mod system;
mod window;

use std::cell::RefCell;
use std::rc::Rc;

use window::MainWindow;

thread_local! {
    static WINDOW: RefCell<Option<Rc<MainWindow>>> = const { RefCell::new(None) };
}

// Runs on the UI thread, from any thread: a fetch's result, an
// announcement that waited. Dropped once the window is closing.
pub fn on_ui(work: impl FnOnce(&MainWindow) + Send + 'static) {
    wxdragon::call_after(Box::new(move || {
        let window = WINDOW.with(|w| w.borrow().clone());
        if let Some(window) = window
            && !window.closed()
        {
            work(&window);
        }
    }));
    wxdragon::wake_up_idle();
}

fn main() {
    let _ = wxdragon::main(|_| {
        let window = window::build();
        WINDOW.with(|w| *w.borrow_mut() = Some(window.clone()));
        window.show();
    });
}
