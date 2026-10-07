// Whole texts, case by case: tests/reference/cases.json lists captured
// responses and the situations to write them in (time of day, this PC's
// zone and time format, units, a failed refresh), and the .txt beside it
// is the text the app writes for each. They began as the C# app's own
// text, which the Rust port matched exactly; since then they are snapshots
// of the app's. After a deliberate change, run the tests with
// WEATHERSPELL_BLESS=1 to rewrite them, and read the diff before
// committing. Under files/ are settings and cache files 0.1 wrote, which
// the app must go on reading for anyone coming from 0.1, and the app's
// own, kept as snapshots so a change of format is never unnoticed.

mod common;

use common::{TempDir, fixture};
use jiff::Timestamp;
use jiff::tz::{Offset, TimeZone};
use serde::Deserialize;
use weatherspell_core::alerts::{self, AlertReport};
use weatherspell_core::cache::ForecastCache;
use weatherspell_core::clock::{Clock, TimeFormat};
use weatherspell_core::forecast::Forecast;
use weatherspell_core::layout::SectionLayout;
use weatherspell_core::location::Location;
use weatherspell_core::official::{OfficialForecast, compose};
use weatherspell_core::settings::{
    Announcements, AppSettings, SavedLocation, SavedWindow, SettingsStore,
};
use weatherspell_core::units::UnitSystem;
use weatherspell_core::writer::{self, WriterOptions};
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

fn reference_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/reference")
}

// A file under tests/reference. Git may have checked it out with Windows
// line breaks.
fn reference(name: &str) -> String {
    std::fs::read_to_string(reference_dir().join(name))
        .unwrap_or_else(|e| panic!("{name}: {e}"))
        .replace("\r\n", "\n")
}

// The first line where the app's text and the snapshot differ, if any.
fn difference(name: &str, ours: &str, theirs: &str) -> Option<String> {
    if ours == theirs {
        return None;
    }
    let (line, (a, b)) = ours
        .lines()
        .chain(std::iter::repeat(""))
        .zip(theirs.lines().chain(std::iter::repeat("")))
        .enumerate()
        .find(|(_, (a, b))| a != b)
        .unwrap();
    Some(format!(
        "{name}, line {}:\n  snapshot: {b}\n  now:      {a}",
        line + 1
    ))
}

fn blessing() -> bool {
    std::env::var_os("WEATHERSPELL_BLESS").is_some()
}

// A snapshot under tests/reference: compared, or written afresh when
// WEATHERSPELL_BLESS is set.
fn golden(name: &str, ours: &str) {
    if blessing() {
        std::fs::write(reference_dir().join(name), ours).unwrap();
    } else {
        assert!(
            difference(name, ours, &reference(name)).is_none(),
            "{}\nif that is on purpose, run the tests with WEATHERSPELL_BLESS=1 and read the diff",
            difference(name, ours, &reference(name)).unwrap()
        );
    }
}

// The coordinate reader over variants of its tests' inputs
// (coordinates-input.json, made once by a script and kept): the point to
// six decimals, the problem, or "none" for text left to the place search.
// The expected readings began as 0.1's own.
#[test]
fn coordinates_read_as_their_snapshot() {
    let inputs: Vec<String> = serde_json::from_str(
        &std::fs::read_to_string(reference_dir().join("coordinates-input.json")).unwrap(),
    )
    .unwrap();
    let readings: Vec<String> = inputs
        .iter()
        .enumerate()
        .map(|(i, input)| {
            let reading = match weatherspell_core::coordinates::read(input) {
                None => "none".to_string(),
                // Adding zero drops a negative zero's sign ("-0, 0"), which
                // .NET did not write either; the app never shows or sends it.
                Some(r) => r
                    .problem
                    .unwrap_or_else(|| format!("{:.6} {:.6}", r.latitude + 0.0, r.longitude + 0.0)),
            };
            format!("{i}: {reading}")
        })
        .collect();
    if blessing() {
        golden("coordinates-expected.txt", &(readings.join("\n") + "\n"));
        return;
    }
    let theirs = reference("coordinates-expected.txt");
    let mut differences = Vec::new();
    for ((input, ours), expected) in inputs.iter().zip(&readings).zip(theirs.lines()) {
        if ours != expected {
            differences.push(format!(
                "{input:?}\n  snapshot: {expected}\n  now:      {ours}"
            ));
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

fn cases() -> Vec<Case> {
    serde_json::from_str(&reference("cases.json")).unwrap()
}

fn location(c: &Case) -> Location {
    Location::new(
        &c.place.name,
        c.place.region.as_deref(),
        c.place.country.as_deref(),
        c.place.latitude,
        c.place.longitude,
    )
}

fn options(c: &Case) -> WriterOptions {
    WriterOptions {
        now: c.now.parse().unwrap(),
        pc_zone: TimeZone::fixed(Offset::from_seconds(c.pc_offset_minutes * 60).unwrap()),
        time_format: TimeFormat::new(&c.time_pattern, &c.am, &c.pm),
        refresh_problem: c.refresh_problem.clone(),
    }
}

// The case's forecast and alerts, from its captured responses.
fn forecast(c: &Case) -> (Forecast, Option<AlertReport>) {
    let units = if c.units == "imperial" {
        UnitSystem::Imperial
    } else {
        UnitSystem::Metric
    };
    let fetched: Timestamp = c.fetched.parse().unwrap();
    let mut forecast =
        open_meteo::parse(&fixture(&c.fixture), &location(c), units, fetched).unwrap();
    if let Some(o) = &c.official {
        forecast = compose(&forecast, &official(o));
    }
    let report = c
        .alerts
        .as_ref()
        .map(|a| alert_report(a, c.now.parse().unwrap()));
    (forecast, report)
}

// The case's text, and for a case with alerts what a reader hears about
// them beyond it: the details dialog, the announcement of new ones, and
// what a switch says.
fn text(c: &Case, forecast: &Forecast, report: Option<&AlertReport>) -> String {
    let options = options(c);
    let mut ours = SectionLayout::build(&writer::write(forecast, &options, report)).text;
    if let Some(report) = report {
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
    ours
}

#[test]
fn each_case_writes_its_snapshot() {
    let cases = cases();
    let differences: Vec<String> = cases
        .iter()
        .filter_map(|c| {
            let (forecast, report) = forecast(c);
            let ours = text(c, &forecast, report.as_ref());
            let name = format!("{}.txt", c.name);
            if blessing() {
                golden(&name, &ours);
                return None;
            }
            difference(&c.name, &ours, &reference(&name))
        })
        .collect();
    assert!(
        differences.is_empty(),
        "{} of {} cases differ:\n{}\nif that is on purpose, run the tests with WEATHERSPELL_BLESS=1 and read the diff",
        differences.len(),
        cases.len(),
        differences.join("\n")
    );
}

// Environment Canada's alerts as 0.1 named them, without the colour the
// app puts first ("Yellow frost advisory" was "Frost advisory").
fn names_as_0_1_wrote_them(report: &AlertReport) -> AlertReport {
    let mut report = report.clone();
    for alert in &mut report.alerts {
        if alert.source == environment_canada::SOURCE_NAME && alert.level.is_some() {
            let name = alert.event.split_once(' ').map_or("", |(_, rest)| rest);
            alert.event = weatherspell_core::official::sentence_case(name);
        }
    }
    report
}

// Cases whose forecast and alerts were cached by 0.1: the app must read
// the same records back. The app's own file for each is a snapshot; a
// change to it may be one that 0.1 cannot read, which would cost a tester
// going back to 0.1 only that location's cached forecast.
const CACHE_CASES: [&str; 3] = ["ec-peterborough-evening", "alerts-spokane", "alerts-gander"];

#[test]
fn the_app_reads_0_1s_cache_files_and_keeps_its_own_format() {
    let cases = cases();
    for name in CACHE_CASES {
        let c = cases.iter().find(|c| c.name == name).unwrap();
        let place = location(c);
        let (forecast, report) = forecast(c);
        let dir = TempDir::new();

        let theirs = ForecastCache::new(dir.join("0.1"));
        std::fs::create_dir_all(theirs.folder()).unwrap();
        std::fs::copy(
            reference_dir().join(format!("files/cache-{name}-0.1.json")),
            theirs.folder().join(ForecastCache::file_name(&place)),
        )
        .unwrap();
        let cached = theirs
            .load(&place)
            .unwrap_or_else(|| panic!("{name}: the app could not read 0.1's file"));
        assert_eq!(cached.forecast, forecast, "{name}");
        // 0.1's file holds its own alert names until the next check; the
        // records read are otherwise the case's own, so its text follows.
        assert_eq!(
            cached.alerts,
            report.as_ref().map(names_as_0_1_wrote_them),
            "{name}"
        );

        let ours = ForecastCache::new(dir.join("app"));
        ours.save(&place, &forecast, report.as_ref()).unwrap();
        let file = ours.folder().join(ForecastCache::file_name(&place));
        golden(
            &format!("files/cache-{name}-app.json"),
            &std::fs::read_to_string(file).unwrap(),
        );
    }
}

// The settings in 0.1's files: a nickname, seen alerts, a muted location,
// a name beyond ASCII, a point named by its coordinates, and a window.
fn sample_settings() -> AppSettings {
    let mut home = Location::new(
        "Peterborough",
        Some("Ontario"),
        Some("Canada"),
        44.30012,
        -78.31623,
    );
    home.time_zone_id = Some("America/Toronto".to_string());
    home.nickname = Some("Home".to_string());
    let mut home = SavedLocation::from_location(&home);
    home.seen_alert_ids = vec![
        "ec:64919237566271632202609120507".to_string(),
        "nws:KALB.FL.W.0012".to_string(),
    ];
    home.utc_offset_seconds = Some(-14400);
    let mut paris = Location::new(
        "Paris",
        Some("\u{CE}le-de-France"),
        Some("France"),
        48.85341,
        2.3488,
    );
    paris.time_zone_id = Some("Europe/Paris".to_string());
    let mut paris = SavedLocation::from_location(&paris);
    paris.notify_alerts = false;
    paris.utc_offset_seconds = Some(7200);
    let point = SavedLocation::from_location(&Location::new(
        "44.54 north, 78.54 west",
        None,
        None,
        44.54,
        -78.54,
    ));
    AppSettings {
        locations: vec![home, paris, point],
        last_location: 1,
        forecast_refresh_minutes: 60,
        alert_check_minutes: 5,
        alert_announcements: Announcements::Severe,
        window: Some(SavedWindow {
            left: -11,
            top: 0,
            width: 982,
            height: 1019,
            maximized: true,
            char_width: 9.92,
            char_height: 25.0,
        }),
        ..AppSettings::default()
    }
}

// settings-0.1.json is what 0.1 saved; settings-resaved-by-0.1.json is the
// port's file as 0.1 read it and saved it again (both kept from when the C#
// app could still be run against the port). The app's own is a snapshot.
#[test]
fn the_app_reads_0_1s_settings_and_keeps_its_own_format() {
    let dir = TempDir::new();
    std::fs::create_dir_all(&dir.0).unwrap();
    // Read from a copy: a store keeps a file it cannot read beside it.
    let read = |name: &str| {
        let copy = dir.join(name);
        std::fs::copy(reference_dir().join("files").join(name), &copy).unwrap();
        SettingsStore::new(copy).load()
    };

    assert_eq!(read("settings-0.1.json"), (sample_settings(), None));

    assert_eq!(
        read("settings-resaved-by-0.1.json"),
        (sample_settings(), None)
    );

    let ours = SettingsStore::in_folder(&dir.join("app"));
    ours.save(&mut sample_settings()).unwrap();
    golden(
        "files/settings-app.json",
        &std::fs::read_to_string(ours.path()).unwrap(),
    );
}

// The file a location's cache is kept in, as 0.1 named it, for points whose
// rounding to four decimals is a close call: a name that differs would
// lose that location's cached forecast to anyone coming from 0.1.
#[test]
fn cache_files_are_named_as_0_1_names_them() {
    let theirs = reference("files/cache-names-0.1.txt");
    let mut differences = Vec::new();
    for line in theirs.lines() {
        let parts: Vec<&str> = line.split(' ').collect();
        let point = Location::new(
            "x",
            None,
            None,
            parts[0].parse().unwrap(),
            parts[1].parse().unwrap(),
        );
        let ours = ForecastCache::file_name(&point);
        if ours != parts[2] {
            differences.push(format!("{line}: app {ours}"));
        }
    }
    assert!(theirs.lines().count() > 2000);
    assert!(
        differences.is_empty(),
        "{} names differ:\n{}",
        differences.len(),
        differences.join("\n")
    );
}
