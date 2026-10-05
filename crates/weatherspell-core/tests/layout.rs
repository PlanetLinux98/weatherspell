// SectionLayoutTests from the C# app, ported, plus the UTF-16 offsets the
// text controls count in.

use jiff::{SignedDuration, Timestamp};
use weatherspell_core::alerts::{AlertSeverity, WeatherAlert};
use weatherspell_core::layout::{RewritePlan, SectionLayout, SelectionHold, units};
use weatherspell_core::writer::Section;

fn frost() -> WeatherAlert {
    WeatherAlert {
        id: "ec:1".to_string(),
        event: "Frost advisory".to_string(),
        severity: AlertSeverity::Moderate,
        issued: "2026-09-12T22:35:00-02:30".parse().unwrap(),
        onset: None,
        ends: None,
        source: "Environment Canada".to_string(),
        sender: "Environment Canada".to_string(),
        area: "Gander and vicinity".to_string(),
        level: None,
        description: "Areas of frost are expected.".to_string(),
        instruction: None,
        url: Some("https://weather.gc.ca/".to_string()),
        expires: None,
    }
}

fn section(heading: &str, paragraphs: &[&str]) -> Section {
    Section::new(heading, paragraphs.iter().map(|p| p.to_string()).collect())
}

fn sections(alert: bool, rest_of_today: bool) -> Vec<Section> {
    let mut list = vec![if alert {
        let mut s = section(
            "Alerts",
            &[
                "Frost advisory from Environment Canada, in effect until 6:30 am tomorrow. Press Enter for details.",
            ],
        );
        s.alerts = Some(vec![Some(frost())]);
        s
    } else {
        section("Alerts", &["No alerts in effect."])
    }];
    list.push(section(
        "Right now",
        &["As of 2:30 pm, it's 21 degrees.", "Humidity 52 percent."],
    ));
    if rest_of_today {
        list.push(section("Rest of today", &["This evening: clear."]));
    }
    list.push(section(
        "Saturday, September 12",
        &["Saturday: sunny.", "Saturday night: clear."],
    ));
    list.push(section("Sources", &["Open-Meteo."]));
    list
}

// An offset into the text; all ASCII here, so bytes and UTF-16 units agree.
fn index_of(layout: &SectionLayout, words: &str) -> usize {
    layout.text.find(words).unwrap()
}

#[test]
fn text_has_a_steady_rhythm_and_headings_are_located() {
    let layout = SectionLayout::build(&sections(false, true));

    assert!(layout.text.starts_with("Alerts\n\nNo alerts in effect.\n\n\nRight now\n\nAs of 2:30 pm, it's 21 degrees.\n\nHumidity 52 percent.\n\n\nRest of today"));
    let headings: Vec<&str> = layout.headings.iter().map(|(h, _)| h.as_str()).collect();
    assert_eq!(
        headings,
        [
            "Alerts",
            "Right now",
            "Rest of today",
            "Saturday, September 12",
            "Sources"
        ]
    );
    assert_eq!(layout.headings[0].1, 0);
    let at = layout.headings[1].1;
    assert_eq!(&layout.text[at..at + 9], "Right now");
    assert!(layout.alert_ranges.is_empty());
    assert_eq!(layout.length, layout.text.len());
}

#[test]
fn alert_lines_are_found_by_offset() {
    let layout = SectionLayout::build(&sections(true, true));
    assert_eq!(layout.alert_ranges.len(), 1);
    let (start, end, alert) = &layout.alert_ranges[0];

    assert_eq!(*alert, frost());
    assert_eq!(layout.alert_at(*start), Some(&frost()));
    assert_eq!(layout.alert_at(*end), Some(&frost()));
    assert_eq!(layout.alert_at(end + 1), None);
    assert_eq!(layout.alert_at(0), None);
}

#[test]
fn a_line_with_no_alert_of_its_own_is_skipped() {
    let mut s = section(
        "Alerts",
        &[
            "Alerts couldn't be checked this time; showing the alerts from 2 hours ago.",
            "Frost advisory from Environment Canada. Press Enter for details.",
        ],
    );
    s.alerts = Some(vec![None, Some(frost())]);
    let layout = SectionLayout::build(&[s]);
    assert_eq!(layout.alert_ranges.len(), 1);
    let (start, _, alert) = &layout.alert_ranges[0];

    assert_eq!(*alert, frost());
    assert_eq!(&layout.text[*start..start + 14], "Frost advisory");
    assert_eq!(layout.alert_at(index_of(&layout, "checked")), None);
}

// A rewrite keeps the reader on the same words: the same distance into
// the section with the same heading, however much the text above grew.
#[test]
fn the_caret_follows_its_heading_across_a_rewrite() {
    let before = SectionLayout::build(&sections(false, true));
    let after = SectionLayout::build(&sections(true, true));
    let humidity = index_of(&before, "Humidity");

    let mapped = after.map_caret(&before, humidity);

    assert_eq!(&after.text[mapped..mapped + 8], "Humidity");
    assert_ne!(mapped, humidity);
}

#[test]
fn a_vanished_section_maps_to_the_one_at_its_position_and_a_short_one_to_its_heading() {
    let before = SectionLayout::build(&sections(false, true));
    let after = SectionLayout::build(&sections(false, false));
    let evening = index_of(&before, "This evening");

    // Rest of today is gone; the caret lands the same distance into the
    // section now third, Saturday.
    let mapped = after.map_caret(&before, evening);
    assert_eq!(after.section_at(mapped), Some(2));
    assert_eq!(evening - before.headings[2].1, mapped - after.headings[2].1);

    // Same heading, but the section is now too short for the distance: the heading itself.
    let shorter = SectionLayout::build(&[section("Right now", &["Short."])]);
    let deep = index_of(&before, "Humidity");
    assert_eq!(shorter.map_caret(&before, deep), shorter.headings[0].1);
}

#[test]
fn the_same_words_are_never_set_again_and_a_selection_holds_off_an_automatic_rewrite() {
    let shown = SectionLayout::build(&sections(false, true));
    let same = SectionLayout::build(&sections(false, true));
    let changed = SectionLayout::build(&sections(true, true));
    let plan = SectionLayout::plan;

    // An alert check that finds nothing new, even while text is selected.
    assert_eq!(plan(&shown, &same, true, true), RewritePlan::KeepText);
    assert_eq!(plan(&shown, &same, false, false), RewritePlan::KeepText);
    // New words from a timer or a poll wait for the selection; F5 does not.
    assert_eq!(plan(&shown, &changed, true, true), RewritePlan::Wait);
    assert_eq!(plan(&shown, &changed, true, false), RewritePlan::Replace);
    assert_eq!(plan(&shown, &changed, false, true), RewritePlan::Replace);
}

#[test]
fn a_selection_holds_a_rewrite_off_for_five_minutes_at_most() {
    let mut hold = SelectionHold::default();
    let t0: Timestamp = "2026-09-27T20:00:00Z".parse().unwrap();
    let at = |minutes: i64| t0 + SignedDuration::from_mins(minutes);

    assert!(!hold.holds(false, t0));
    assert!(hold.holds(true, t0));
    assert!(hold.holds(true, at(4)));
    // Left behind: the rewrite goes ahead on the next tick.
    assert!(!hold.holds(true, at(5)));

    // Once the text is set, a new selection gets the full time again.
    hold.release();
    assert!(hold.holds(true, at(6)));
    assert!(hold.holds(true, at(10)));
    // A moment with nothing selected starts it over too.
    assert!(!hold.holds(false, at(10)));
    assert!(hold.holds(true, at(14)));
}

#[test]
fn an_empty_layout_keeps_the_caret_in_range() {
    let before = SectionLayout::build(&sections(false, true));
    let empty = SectionLayout::default();
    assert_eq!(empty.map_caret(&before, 40), 0);
    assert!(empty.headings.is_empty());
}

// Offsets are the text controls' own: UTF-16 units, so a place name
// beyond ASCII (or beyond the Basic Multilingual Plane) does not shift
// the headings after it.
#[test]
fn offsets_count_utf16_units() {
    let layout = SectionLayout::build(&[
        section("Right now", &["\u{CE}le-de-France \u{1F327} rain."]),
        section("Sources", &["Open-Meteo."]),
    ]);
    let before: String = layout.text.split("Sources").next().unwrap().to_string();

    assert_eq!(layout.headings[1].1, units(&before));
    assert_eq!(layout.headings[1].1, before.chars().count() + 1);
    assert_eq!(layout.length, units(&layout.text));
}
