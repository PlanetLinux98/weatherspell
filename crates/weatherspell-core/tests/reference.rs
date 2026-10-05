// The port against 0.1, case by case: tests/reference/cases.json lists
// captured responses and the situations to write them in (time of day,
// this PC's zone and time format, units, a failed refresh), and the .txt
// beside it is what the C# app wrote for each, made by tools/ReferenceText.
// A difference here is either a bug in the port or a change Elliott has
// agreed to, in which case the reference text is edited to match and the
// change is recorded.

mod common;

use common::fixture;
use jiff::Timestamp;
use jiff::tz::{Offset, TimeZone};
use serde::Deserialize;
use weatherspell_core::clock::TimeFormat;
use weatherspell_core::location::Location;
use weatherspell_core::open_meteo;
use weatherspell_core::units::UnitSystem;
use weatherspell_core::writer::{self, Section, WriterOptions};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Case {
    name: String,
    fixture: String,
    place: Place,
    units: String,
    fetched: String,
    now: String,
    pc_offset_minutes: i32,
    time_pattern: String,
    am: String,
    pm: String,
    refresh_problem: Option<String>,
}

#[derive(Deserialize)]
struct Place {
    name: String,
    region: Option<String>,
    country: Option<String>,
    latitude: f64,
    longitude: f64,
}

// SectionLayout.Build's text: heading, blank line, paragraphs each followed
// by a blank line, and a further line break between sections.
fn layout(sections: &[Section]) -> String {
    let mut text = String::new();
    for section in sections {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&section.heading);
        text.push_str("\n\n");
        for paragraph in &section.paragraphs {
            text.push_str(paragraph);
            text.push_str("\n\n");
        }
    }
    text
}

fn reference_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/reference")
}

#[test]
fn the_port_writes_what_0_1_wrote() {
    let cases: Vec<Case> =
        serde_json::from_str(&std::fs::read_to_string(reference_dir().join("cases.json")).unwrap())
            .unwrap();
    let mut differences = Vec::new();
    for c in &cases {
        let location = Location::new(
            &c.place.name,
            c.place.region.as_deref(),
            c.place.country.as_deref(),
            c.place.latitude,
            c.place.longitude,
        );
        let units = if c.units == "imperial" {
            UnitSystem::Imperial
        } else {
            UnitSystem::Metric
        };
        let fetched: Timestamp = c.fetched.parse().unwrap();
        let forecast = open_meteo::parse(&fixture(&c.fixture), &location, units, fetched).unwrap();
        let options = WriterOptions {
            now: c.now.parse().unwrap(),
            pc_zone: TimeZone::fixed(Offset::from_seconds(c.pc_offset_minutes * 60).unwrap()),
            time_format: TimeFormat::new(&c.time_pattern, &c.am, &c.pm),
            refresh_problem: c.refresh_problem.clone(),
        };
        let ours = layout(&writer::write(&forecast, &options));
        // Git may have checked the reference out with Windows line breaks.
        let theirs = std::fs::read_to_string(reference_dir().join(format!("{}.txt", c.name)))
            .unwrap()
            .replace("\r\n", "\n");
        if ours != theirs {
            let (line, (a, b)) = ours
                .lines()
                .chain(std::iter::repeat(""))
                .zip(theirs.lines().chain(std::iter::repeat("")))
                .enumerate()
                .find(|(_, (a, b))| a != b)
                .unwrap();
            differences.push(format!(
                "{}, line {}:\n  0.1:  {b}\n  port: {a}",
                c.name,
                line + 1
            ));
        }
    }
    assert!(
        differences.is_empty(),
        "{} of {} cases differ:\n{}",
        differences.len(),
        cases.len(),
        differences.join("\n")
    );
}
