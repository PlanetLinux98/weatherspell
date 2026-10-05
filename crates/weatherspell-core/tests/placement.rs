// WindowPlacementTests from the C# app, ported. A 150 percent display:
// 1920 x 1080 less a 72-pixel taskbar, with a second screen to its right;
// the caption as Windows gives it, and Segoe UI 9 pt's average character
// as WinForms measured it there (7 by 15 at 96 DPI; not in proportion).

use weatherspell_core::placement::{CharSize, Rect, restore, save};
use weatherspell_core::settings::SavedWindow;

const MAIN: Rect = Rect {
    left: 0,
    top: 0,
    width: 1920,
    height: 1008,
};
const RIGHT: Rect = Rect {
    left: 1920,
    top: 0,
    width: 1920,
    height: 1008,
};
const CAPTION: i32 = 34;
const CHAR_150: CharSize = CharSize {
    width: 10.0,
    height: 25.0,
};
const CHAR_100: CharSize = CharSize {
    width: 7.0,
    height: 15.0,
};

fn saved(left: i32, top: i32, width: i32, height: i32) -> SavedWindow {
    SavedWindow {
        left,
        top,
        width,
        height,
        maximized: false,
        char_width: 10.0,
        char_height: 25.0,
    }
}

fn at(left: i32, top: i32) -> SavedWindow {
    saved(left, top, 1080, 840)
}

#[test]
fn a_window_on_a_current_screen_comes_back_where_it_was() {
    assert_eq!(
        restore(&at(200, 100), CHAR_150, &[MAIN], CAPTION),
        Some(Rect::new(200, 100, 1080, 840))
    );
    assert_eq!(
        restore(&at(2300, 50), CHAR_150, &[MAIN, RIGHT], CAPTION),
        Some(Rect::new(2300, 50, 1080, 840))
    );
}

#[test]
fn a_window_on_a_screen_that_has_gone_opens_centred() {
    assert_eq!(restore(&at(2300, 50), CHAR_150, &[MAIN], CAPTION), None);
    // Only the last few pixels of the title bar still on the screen.
    assert_eq!(restore(&at(1880, 50), CHAR_150, &[MAIN], CAPTION), None);
    // Title bar above the top of the screen.
    assert_eq!(restore(&at(200, -200), CHAR_150, &[MAIN], CAPTION), None);
}

#[test]
fn the_size_follows_a_change_of_display_scale_or_text_size() {
    assert_eq!(
        restore(&at(200, 100), CHAR_100, &[MAIN], CAPTION),
        Some(Rect::new(200, 100, 756, 504))
    );
    // No character size recorded: taken as it is.
    let no_char_size = SavedWindow {
        char_width: 0.0,
        char_height: 0.0,
        ..at(200, 100)
    };
    assert_eq!(
        restore(&no_char_size, CHAR_100, &[MAIN], CAPTION),
        Some(Rect::new(200, 100, 1080, 840))
    );
}

#[test]
fn a_window_hanging_off_its_screen_is_brought_back_onto_it() {
    // Saved on a larger display: shrunk to the screen and moved in.
    assert_eq!(
        restore(&saved(100, 60, 2400, 1300), CHAR_150, &[MAIN], CAPTION),
        Some(Rect::new(0, 0, 1920, 1008))
    );
    // Its bottom below the taskbar.
    assert_eq!(
        restore(&at(600, 500), CHAR_150, &[MAIN], CAPTION),
        Some(Rect::new(600, 168, 1080, 840))
    );
}

#[test]
fn a_snapped_window_keeps_its_reach_past_the_edge() {
    // Snapped left at 150 percent: the invisible resize border takes the
    // bounds 11 pixels past the left edge and the bottom.
    assert_eq!(
        restore(&saved(-11, 0, 982, 1019), CHAR_150, &[MAIN], CAPTION),
        Some(Rect::new(-11, 0, 982, 1019))
    );
}

#[test]
fn save_records_the_bounds_state_and_character_size() {
    let saved = save(
        Rect::new(200, 100, 1080, 840),
        true,
        CharSize {
            width: 9.916667,
            height: 25.0,
        },
    );

    assert_eq!(
        saved,
        SavedWindow {
            maximized: true,
            char_width: 9.92,
            ..at(200, 100)
        }
    );
}
