// The Mac's View > Bigger, Smaller and Actual Size. Windows has a Text
// size setting that wx's controls follow; the Mac has none they follow,
// so the app offers its own steps, as Safari and TextEdit do. Each step is
// a share of the system's size: two below it, up to three times it (much
// more and the main window's 68 characters outgrow a laptop's screen).
// Not remembered: every launch starts at Actual Size (Elliott, 2026-10-07).

const PERCENTS: [i32; 10] = [75, 90, 100, 110, 125, 150, 175, 200, 250, 300];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSize(usize);

impl TextSize {
    pub const ACTUAL: TextSize = TextSize(2);

    pub fn bigger(self) -> TextSize {
        TextSize((self.0 + 1).min(PERCENTS.len() - 1))
    }

    pub fn smaller(self) -> TextSize {
        TextSize(self.0.saturating_sub(1))
    }

    pub fn percent(self) -> i32 {
        PERCENTS[self.0]
    }

    // Whole points, halves rounded up: 13 points (the Mac's) at 150
    // percent is 20.
    pub fn points(self, actual: i32) -> i32 {
        (actual * self.percent() + 50) / 100
    }
}

impl Default for TextSize {
    fn default() -> Self {
        TextSize::ACTUAL
    }
}
