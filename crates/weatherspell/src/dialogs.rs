// What every dialog does the same way: sizes in average characters of its
// own font, as 0.1 sized every window (Segoe UI 9 point's 7 by 15 pixels
// at 96 DPI), always within the screen it opens on, and a row of buttons
// at the bottom right.

use std::cell::Cell;

use wxdragon::prelude::*;

thread_local! {
    // Points set by the Mac's View > Bigger and Smaller, for the windows
    // opened after.
    static CHOSEN_POINTS: Cell<Option<i32>> = const { Cell::new(None) };
}

pub fn choose_points(points: i32) {
    CHOSEN_POINTS.set(Some(points));
}

// Every window's font, called on a window before its controls are made,
// which take its font: the size chosen on the Mac or, developer-only as
// 0.1's was, every window in WEATHERSPELL_FONT_POINTS points, as a large
// Windows Text size gives it, so layouts can be checked at large sizes
// (tools/Capture-Windows.ps1).
pub fn app_font(window: &dyn WxWidget) {
    let points = CHOSEN_POINTS.get().or_else(|| {
        std::env::var("WEATHERSPELL_FONT_POINTS")
            .ok()
            .and_then(|p| p.trim().parse::<i32>().ok())
            .filter(|p| *p > 0)
    });
    if let (Some(points), Some(mut font)) = (points, window.get_font()) {
        font.set_point_size(points);
        window.set_font(&font);
    }
}

// A sizable dialog: wanted and minimum sizes in characters.
pub fn size(dialog: &Dialog, wanted: (i32, i32), minimum: (i32, i32)) {
    let (w, h) = (dialog.get_char_width(), dialog.get_char_height());
    dialog.set_min_size(within(dialog, Size::new(w * minimum.0, h * minimum.1)));
    dialog.set_size(within(dialog, Size::new(w * wanted.0, h * wanted.1)));
    dialog.centre();
}

// A fixed dialog closed up on its content, at least `width` characters
// wide.
pub fn fit(dialog: &Dialog, width: i32) {
    dialog.fit();
    let best = dialog.get_size();
    let size = Size::new(best.width.max(dialog.get_char_width() * width), best.height);
    dialog.set_min_size(within(dialog, size));
    dialog.set_size(within(dialog, size));
    dialog.centre();
}

fn within(window: &Dialog, size: Size) -> Size {
    match Display::from_window(window) {
        Some(display) => {
            let area = display.client_area();
            Size::new(size.width.min(area.width), size.height.min(area.height))
        }
        None => size,
    }
}

// Buttons in their order, right-aligned, spaced as 0.1's were.
pub fn button_row(buttons: &[&Button]) -> BoxSizer {
    let row = BoxSizer::builder(Orientation::Horizontal).build();
    row.add_stretch_spacer(1);
    for (i, button) in buttons.iter().enumerate() {
        let gap = if i + 1 < buttons.len() { 6 } else { 0 };
        row.add(*button, 0, SizerFlag::Right, gap);
    }
    row
}

// Ends a modal dialog, once. On the Mac, Return presses the default
// button and also reaches a control's own Enter handler, so a dialog was
// asked to end twice, which wxWidgets stops on with an assertion. Ending
// hides it at once, so a hidden dialog is already ending.
pub fn end(dialog: &Dialog, code: i32) {
    if dialog.is_shown() {
        dialog.end_modal(code);
    }
}

// A label's text wrapped to the dialog's content width, so a long place
// name wraps rather than widening the dialog.
pub fn wrap(text: &StaticText, chars: i32) {
    text.wrap(text.get_char_width() * chars);
}
