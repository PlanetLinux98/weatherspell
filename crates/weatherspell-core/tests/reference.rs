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
use weatherspell_core::alerts::{self, AlertReport};
use weatherspell_core::clock::{Clock, TimeFormat};
use weatherspell_core::location::Location;
use weatherspell_core::official::{OfficialForecast, compose};
use weatherspell_core::units::UnitSystem;
use weatherspell_core::writer::{self, Section, WriterOptions};
use weatherspell_core::{environment_canada, nws, open_meteo};

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
    #[serde(default)]
    official: Option<Official>,
    #[serde(default)]
    alerts: Option<AlertsCase>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AlertsCase {
    kind: String,
    fixture: String,
    problem: Option<String>,
    last_checked: Option<String>,
}

// A service's alerts from a captured response, as the alert check would
// have reported them, or a failed check carrying an earlier one's alerts.
fn alert_report(a: &AlertsCase, now: Timestamp) -> AlertReport {
    let json = fixture(&a.fixture);
    let parsed_at: Timestamp = a
        .last_checked
        .as_deref()
        .map_or(now, |t| t.parse().unwrap());
    let (list, attribution) = if a.kind == "ec" {
        (
            environment_canada::parse_alerts(&json, parsed_at, None).unwrap(),
            environment_canada::ALERTS_ATTRIBUTION,
        )
    } else {
        (nws::parse_alerts(&json).unwrap(), nws::ALERTS_ATTRIBUTION)
    };
    let report = |alerts, problem: Option<&String>, checked_at| AlertReport {
        alerts,
        attribution: Some(attribution.to_string()),
        problem: problem.cloned(),
        checked_at,
    };
    let ordered = alerts::order(list);
    match &a.problem {
        None => report(ordered, None, Some(now)),
        Some(problem) => {
            let failed = report(Vec::new(), Some(problem), None);
            match a.last_checked {
                None => failed,
                Some(_) => failed.or_last_known(Some(&report(ordered, None, Some(parsed_at)))),
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Official {
    kind: String,
    city_page: Option<String>,
    points: Option<String>,
    stations: Option<String>,
    forecast: Option<String>,
    page: Option<String>,
    observation: Option<String>,
}

// A national weather service's text and observation, from captured
// responses, as the forecast service would lay them over the base.
fn official(o: &Official) -> OfficialForecast {
    let read = |name: &Option<String>| fixture(name.as_deref().unwrap());
    if o.kind == "ec" {
        return environment_canada::parse(&read(&o.city_page)).unwrap();
    }
    let points = nws::parse_points(&read(&o.points)).unwrap();
    let station = nws::parse_stations(&read(&o.stations)).unwrap();
    let periods = match &o.page {
        Some(_) => nws::parse_page_periods(&read(&o.page)),
        None => nws::parse_periods(&read(&o.forecast)),
    }
    .unwrap();
    let station_name = station.as_ref().and_then(|(_, name)| name.as_deref());
    let observation = o
        .observation
        .as_ref()
        .map(|_| nws::parse_observation(&read(&o.observation), station_name).unwrap());
    OfficialForecast {
        source_name: nws::SOURCE_NAME.to_string(),
        attribution: points.attribution,
        periods,
        observation,
    }
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

// The coordinate reader against 0.1's over variants of its tests' inputs
// (coordinates-input.json, made once by a script and kept): the same point
// to six decimals, the same problem, or the same "none" for text left to
// the place search.
#[test]
fn the_port_reads_coordinates_as_0_1_did() {
    let inputs: Vec<String> = serde_json::from_str(
        &std::fs::read_to_string(reference_dir().join("coordinates-input.json")).unwrap(),
    )
    .unwrap();
    let theirs = std::fs::read_to_string(reference_dir().join("coordinates-0.1.txt"))
        .unwrap()
        .replace("\r\n", "\n");
    let mut differences = Vec::new();
    for ((i, input), expected) in inputs.iter().enumerate().zip(theirs.lines()) {
        let reading = match weatherspell_core::coordinates::read(input) {
            None => "none".to_string(),
            // Adding zero drops a negative zero's sign ("-0, 0"), which .NET
            // does not write either; the app never shows or sends it.
            Some(r) => r
                .problem
                .unwrap_or_else(|| format!("{:.6} {:.6}", r.latitude + 0.0, r.longitude + 0.0)),
        };
        let ours = format!("{i}: {reading}");
        if ours != expected {
            differences.push(format!("{input:?}\n  0.1:  {expected}\n  port: {ours}"));
        }
    }
    assert_eq!(theirs.lines().count(), inputs.len());
    assert!(
        differences.is_empty(),
        "{} of {} readings differ:\n{}",
        differences.len(),
        inputs.len(),
        differences.join("\n")
    );
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
        let mut forecast =
            open_meteo::parse(&fixture(&c.fixture), &location, units, fetched).unwrap();
        if let Some(o) = &c.official {
            forecast = compose(&forecast, &official(o));
        }
        let options = WriterOptions {
            now: c.now.parse().unwrap(),
            pc_zone: TimeZone::fixed(Offset::from_seconds(c.pc_offset_minutes * 60).unwrap()),
            time_format: TimeFormat::new(&c.time_pattern, &c.am, &c.pm),
            refresh_problem: c.refresh_problem.clone(),
        };
        let report = c.alerts.as_ref().map(|a| alert_report(a, options.now));
        let mut ours = layout(&writer::write(&forecast, &options, report.as_ref()));
        if let Some(report) = &report {
            // What a reader hears about the alerts beyond the text: the
            // details dialog, the announcement of new ones, and what a
            // switch says.
            let clock = Clock::new(
                forecast.utc_offset,
                options.pc_zone.clone(),
                options.time_format.clone(),
            );
            let now_local = forecast.utc_offset.to_datetime(options.now);
            for alert in &report.alerts {
                ours += &format!("Details: {}\n\n", alert.event);
                for paragraph in alerts::details(alert, &clock, now_local) {
                    ours += &format!("{paragraph}\n\n");
                }
            }
            if !report.alerts.is_empty() {
                ours += &format!(
                    "Announcement: {}\n\n",
                    alerts::announcement(&c.place.name, &report.alerts, &clock, now_local)
                );
            }
            ours += &format!(
                "In effect: {}\n\n",
                alerts::in_effect(report).as_deref().unwrap_or("nothing")
            );
        }
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
