// The forecast text box's contents, built from the writer's sections
// (SectionLayout.cs in 0.1): heading line, blank line, paragraphs
// separated by blank lines, blank line, a steady rhythm for line-by-line
// reading. Records where each heading starts (Ctrl+PageDown/PageUp) and
// which characters are an alert line (Enter opens it), and maps a caret
// across a rewrite so a refresh the user did not ask for leaves them on
// the same words. Lines break with "\n" and offsets count UTF-16 units,
// as 0.1's did; the app converts them to its text control's positions (a
// plain EDIT control counts each break as two).

use jiff::{SignedDuration, Timestamp};

use crate::alerts::WeatherAlert;
use crate::writer::Section;

const BREAK: &str = "\n";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SectionLayout {
    pub text: String,
    // The text's length in UTF-16 units.
    pub length: usize,
    pub headings: Vec<(String, usize)>,
    // Start and end of each alert line, and its alert.
    pub alert_ranges: Vec<(usize, usize, WeatherAlert)>,
}

// What a rewrite does with the text box. KeepText: take the new layout
// (its alert lines may point at newer details) and leave the text box
// alone. Wait: leave both until later.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RewritePlan {
    KeepText,
    Wait,
    Replace,
}

pub fn units(text: &str) -> usize {
    text.encode_utf16().count()
}

impl SectionLayout {
    pub fn build(sections: &[Section]) -> SectionLayout {
        let mut layout = SectionLayout::default();
        for section in sections {
            if !layout.text.is_empty() {
                layout.append(BREAK);
            }
            layout
                .headings
                .push((section.heading.clone(), layout.length));
            layout.append(&section.heading);
            layout.append(BREAK);
            layout.append(BREAK);
            for (i, paragraph) in section.paragraphs.iter().enumerate() {
                let alert = section
                    .alerts
                    .as_ref()
                    .and_then(|a| a.get(i))
                    .and_then(Option::as_ref);
                if let Some(alert) = alert {
                    let start = layout.length;
                    layout
                        .alert_ranges
                        .push((start, start + units(paragraph), alert.clone()));
                }
                layout.append(paragraph);
                layout.append(BREAK);
                layout.append(BREAK);
            }
        }
        layout
    }

    fn append(&mut self, s: &str) {
        self.text.push_str(s);
        self.length += units(s);
    }

    pub fn alert_at(&self, offset: usize) -> Option<&WeatherAlert> {
        self.alert_ranges
            .iter()
            .find(|(start, end, _)| (*start..=*end).contains(&offset))
            .map(|(_, _, alert)| alert)
    }

    // The heading at or before the offset; None before the first.
    pub fn section_at(&self, offset: usize) -> Option<usize> {
        self.headings
            .iter()
            .take_while(|(_, at)| *at <= offset)
            .count()
            .checked_sub(1)
    }

    // The same words are never set again, whatever else changed (an alert
    // check's time, an alert's details behind the same line): setting
    // them drops a selection, which NVDA announces as "unselected". A
    // rewrite the user did not ask for waits while text is selected; the
    // clock's tick applies it once the selection is gone, or has held it
    // for SelectionHold::LIMIT.
    pub fn plan(
        shown: &SectionLayout,
        next: &SectionLayout,
        selecting: bool,
        automatic: bool,
    ) -> RewritePlan {
        if next.text == shown.text {
            RewritePlan::KeepText
        } else if automatic && selecting {
            RewritePlan::Wait
        } else {
            RewritePlan::Replace
        }
    }

    // Where a caret in an earlier layout belongs in this one: the same
    // distance into the section with the same heading; the section at the
    // same position when that heading has gone (a day rolled over); the
    // heading itself when the section is now too short for the distance.
    pub fn map_caret(&self, from: &SectionLayout, caret: usize) -> usize {
        if self.headings.is_empty() {
            return caret.min(self.length);
        }
        let Some(i) = from.section_at(caret) else {
            return caret.min(self.length);
        };
        let (heading, offset) = &from.headings[i];
        let j = self
            .headings
            .iter()
            .position(|(h, _)| h == heading)
            .unwrap_or(i.min(self.headings.len() - 1));
        let start = self.headings[j].1;
        let end = self.headings.get(j + 1).map_or(self.length, |(_, at)| *at);
        let distance = caret - offset;
        if distance < end - start {
            start + distance
        } else {
            start
        }
    }
}

// How long a selection keeps a waiting rewrite off the screen: time enough
// to finish a copy, but bounded, since a selection left behind by accident
// would otherwise keep the text (its age line too) from ever updating
// while the status bar reports a fresh fetch. Five minutes is well inside
// the shortest forecast interval, so a held rewrite lands before the next.
#[derive(Clone, Debug, Default)]
pub struct SelectionHold {
    since: Option<Timestamp>,
}

impl SelectionHold {
    pub const LIMIT: SignedDuration = SignedDuration::from_mins(5);

    // Asked while a rewrite is waiting; the first ask with text selected
    // starts the clock.
    pub fn holds(&mut self, selecting: bool, now: Timestamp) -> bool {
        if !selecting {
            self.since = None;
            return false;
        }
        let since = *self.since.get_or_insert(now);
        now.duration_since(since) < SelectionHold::LIMIT
    }

    pub fn release(&mut self) {
        self.since = None;
    }
}
