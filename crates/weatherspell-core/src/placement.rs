// The main window opens where it was closed, if that is still somewhere a
// user can reach it (WindowPlacement.cs in 0.1). Screens come and go (a
// laptop off its dock, a remote session at another resolution), so the
// saved place is checked against the working areas there are now rather
// than trusted. The app asks the system for the working areas, the
// caption height and its font's average character; the arithmetic is
// here.

use crate::settings::SavedWindow;

// Screen pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn new(left: i32, top: i32, width: i32, height: i32) -> Rect {
        Rect {
            left,
            top,
            width,
            height,
        }
    }

    pub fn right(&self) -> i32 {
        self.left + self.width
    }

    pub fn bottom(&self) -> i32 {
        self.top + self.height
    }

    // The overlap, or an empty rectangle.
    fn intersect(&self, other: &Rect) -> Rect {
        let left = self.left.max(other.left);
        let right = self.right().min(other.right());
        let top = self.top.max(other.top);
        let bottom = self.bottom().min(other.bottom());
        if right >= left && bottom >= top {
            Rect::new(left, top, right - left, bottom - top)
        } else {
            Rect::default()
        }
    }
}

// A font's average character in pixels, which is what every window is
// sized by (7 by 15 for Segoe UI 9 pt at 96 DPI; not in proportion).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharSize {
    pub width: f64,
    pub height: f64,
}

// Enough of the title bar to drag, in pixels.
const MINIMUM_GRIP: i32 = 100;

pub fn save(normal_bounds: Rect, maximized: bool, char_size: CharSize) -> SavedWindow {
    let two_places = |v: f64| (v * 100.0).round_ties_even() / 100.0;
    SavedWindow {
        left: normal_bounds.left,
        top: normal_bounds.top,
        width: normal_bounds.width,
        height: normal_bounds.height,
        maximized,
        char_width: two_places(char_size.width),
        char_height: two_places(char_size.height),
    }
}

// The normal bounds to open with, or None to open centred as on a first
// run: when no screen shows enough of the title bar to take hold of. The
// size follows a change of display scale or Text size since it was saved,
// as the window's text does (across and down separately: a font does not
// grow in proportion), and the window is kept on the screen it lands on,
// give or take the invisible resize border Windows counts in a window's
// bounds (a snapped window reaches past the edge).
pub fn restore(
    saved: &SavedWindow,
    char_size: CharSize,
    working_areas: &[Rect],
    caption_height: i32,
) -> Option<Rect> {
    if saved.width <= 0 || saved.height <= 0 {
        return None;
    }
    let width = follow(saved.width, saved.char_width, char_size.width);
    let height = follow(saved.height, saved.char_height, char_size.height);

    let title_bar = Rect::new(saved.left, saved.top, width, caption_height);
    let mut reachable: Option<(Rect, i32)> = None;
    for area in working_areas {
        let shown = area.intersect(&title_bar);
        let visible = shown.width * shown.height;
        if shown.width >= MINIMUM_GRIP.min(width)
            && shown.height >= caption_height / 2
            && reachable.is_none_or(|(_, best)| visible > best)
        {
            reachable = Some((*area, visible));
        }
    }
    let (screen, _) = reachable?;

    // The invisible border is on the sides and the bottom (the top is
    // title bar), about half a caption high at any scale.
    let border = caption_height / 2;
    let (left, fitted_width) = fit(
        saved.left,
        width,
        screen.left,
        screen.right(),
        border,
        border,
    );
    let (top, fitted_height) = fit(saved.top, height, screen.top, screen.bottom(), 0, border);
    Some(Rect::new(left, top, fitted_width, fitted_height))
}

// A file from before the character size was kept, or edited by hand, has
// none: the size is taken as it is.
fn follow(length: i32, saved_char: f64, current_char: f64) -> i32 {
    if saved_char > 0.0 && current_char > 0.0 {
        crate::round_even(f64::from(length) * current_char / saved_char)
    } else {
        length
    }
}

// A span within [low, high], give or take the border, stays as it is; one
// that overhangs further is moved wholly inside, shortened to fit.
fn fit(
    start: i32,
    length: i32,
    low: i32,
    high: i32,
    border_low: i32,
    border_high: i32,
) -> (i32, i32) {
    if start >= low - border_low && start + length <= high + border_high {
        return (start, length);
    }
    let length = length.min(high - low);
    (low.max(start.min(high - length)), length)
}
