// ForecastCacheTests from the C# app, ported. The sample is an official
// forecast laid over the base (periods, a station with its own words, dew
// point and visibility), all of which a file must keep.

mod common;

use common::{TempDir, fixture};
use jiff::Timestamp;
use jiff::civil::date;
use jiff::tz::{Offset, TimeZone};
use weatherspell_core::alerts::{AlertReport, AlertSeverity, WeatherAlert};
use weatherspell_core::cache::ForecastCache;
use weatherspell_core::clock::TimeFormat;
use weatherspell_core::environment_canada;
use weatherspell_core::forecast::Forecast;
use weatherspell_core::layout::SectionLayout;
use weatherspell_core::location::Location;
use weatherspell_core::official::compose;
use weatherspell_core::open_meteo;
use weatherspell_core::units::UnitSystem;
use weatherspell_core::writer::{self, WriterOptions};

fn peterborough() -> Location {
    let mut l = Location::new(
        "Peterborough",
        Some("Ontario"),
        Some("Canada"),
        44.30,
        -78.33,
    );
    l.time_zone_id = Some("America/Toronto".to_string());
    l
}

fn eastern(text: &str) -> Timestamp {
    format!("{text}-04:00").parse().unwrap()
}

fn sample() -> Forecast {
    let base = open_meteo::parse(
        &fixture("open-meteo-toronto.json"),
        &peterborough(),
        UnitSystem::Metric,
        eastern("2026-09-11T23:58:00"),
    )
    .unwrap();
    compose(
        &base,
        &environment_canada::parse(&fixture("ec-citypage-peterborough.xml")).unwrap(),
    )
}

fn alert(id: &str, event: &str, ends: Option<Timestamp>) -> WeatherAlert {
    WeatherAlert {
        id: id.to_string(),
        event: event.to_string(),
        severity: AlertSeverity::Severe,
        issued: eastern("2026-09-11T13:10:00"),
        onset: Some(eastern("2026-09-11T14:00:00")),
        ends,
        source: "Environment Canada".to_string(),
        sender: "Environment Canada".to_string(),
        area: "Peterborough City - Lakefield - Southern Peterborough County".to_string(),
        level: Some("warning".to_string()),
        description: "Text.\r\n\r\nMore \"text\".".to_string(),
        instruction: Some("Take care.".to_string()),
        url: Some("https://weather.gc.ca/".to_string()),
        expires: None,
    }
}

fn report(alerts: Vec<WeatherAlert>) -> AlertReport {
    AlertReport {
        alerts,
        attribution: Some("Environment Canada (weather.gc.ca)".to_string()),
        problem: None,
        checked_at: Some(eastern("2026-09-11T23:58:00")),
    }
}

fn cache(dir: &TempDir) -> ForecastCache {
    ForecastCache::new(dir.join("cache"))
}

fn file_of(dir: &TempDir) -> std::path::PathBuf {
    dir.join("cache")
        .join(ForecastCache::file_name(&peterborough()))
}

#[test]
fn files_are_named_by_coordinates() {
    let mut home = peterborough();
    assert_eq!(ForecastCache::file_name(&home), "44.3000_-78.3300.json");
    home.nickname = Some("Home".to_string());
    assert_eq!(ForecastCache::file_name(&home), "44.3000_-78.3300.json");
}

#[test]
fn a_forecast_and_its_alerts_come_back_as_they_went_in() {
    let dir = TempDir::new();
    let f = sample();
    assert!(!f.periods.is_empty() && f.current.station.is_some());
    let alerts = report(vec![
        alert(
            "a",
            "Rainfall warning",
            Some(eastern("2026-09-12T06:30:00")),
        ),
        alert("b", "Special weather statement", None),
    ]);

    cache(&dir)
        .save(&peterborough(), &f, Some(&alerts))
        .unwrap();
    let loaded = cache(&dir).load(&peterborough()).unwrap();

    assert_eq!(loaded.forecast, f);
    assert_eq!(loaded.alerts, Some(alerts));
    assert!(!dir.join("cache").join("44.3000_-78.3300.json.tmp").exists());
}

#[test]
fn the_text_written_from_the_cache_is_the_text_written_from_the_fetch() {
    let dir = TempDir::new();
    let f = sample();
    cache(&dir)
        .save(&peterborough(), &f, Some(&AlertReport::not_available()))
        .unwrap();
    let loaded = cache(&dir).load(&peterborough()).unwrap();

    let options = WriterOptions {
        now: eastern("2026-09-12T01:00:00"),
        pc_zone: TimeZone::fixed(Offset::constant(-4)),
        time_format: TimeFormat::new("h:mm tt", "AM", "PM"),
        refresh_problem: None,
    };
    let text = |f: &Forecast, a: Option<&AlertReport>| {
        SectionLayout::build(&writer::write(f, &options, a)).text
    };
    assert_eq!(
        text(&loaded.forecast, loaded.alerts.as_ref()),
        text(&f, Some(&AlertReport::not_available()))
    );
    assert_eq!(loaded.alerts, Some(AlertReport::not_available()));
}

#[test]
fn wall_clock_times_survive_a_dst_change_on_the_pc() {
    // 2:30 am on a spring-forward day does not exist in most zones.
    let dir = TempDir::new();
    let mut f = sample();
    f.current.local_time = date(2026, 3, 8).at(2, 30, 0, 0);
    cache(&dir).save(&peterborough(), &f, None).unwrap();

    assert_eq!(
        cache(&dir).load(&peterborough()).unwrap().forecast.current,
        f.current
    );
}

#[test]
fn a_damaged_file_is_no_cache_whatever_the_damage() {
    let dir = TempDir::new();
    cache(&dir)
        .save(&peterborough(), &sample(), Some(&report(Vec::new())))
        .unwrap();
    let path = file_of(&dir);
    let good = std::fs::read_to_string(&path).unwrap();

    // A null in a list and an offset out of range once raised the .NET
    // error dialog from the refresh that read them (#22).
    assert!(good.contains("\"hours\":["));
    std::fs::write(&path, good.replace("\"hours\":[", "\"hours\":[null,")).unwrap();
    assert_eq!(cache(&dir).load(&peterborough()), None);
    let offset = regex::Regex::new("\"utcOffsetSeconds\":-?\\d+").unwrap();
    assert!(offset.is_match(&good));
    std::fs::write(
        &path,
        offset
            .replace(&good, "\"utcOffsetSeconds\":999999")
            .as_ref(),
    )
    .unwrap();
    assert_eq!(cache(&dir).load(&peterborough()), None);
}

#[test]
fn the_location_asked_for_is_the_one_read_back() {
    let dir = TempDir::new();
    cache(&dir).save(&peterborough(), &sample(), None).unwrap();
    let mut home = peterborough();
    home.nickname = Some("Home".to_string());

    assert_eq!(cache(&dir).load(&home).unwrap().forecast.location, home);
}

#[test]
fn a_failed_check_keeps_the_alerts_from_the_last_one_that_succeeded() {
    let dir = TempDir::new();
    let earlier = report(vec![alert("a", "Rainfall warning", None)]);
    cache(&dir)
        .save(&peterborough(), &sample(), Some(&earlier))
        .unwrap();

    cache(&dir).save(&peterborough(), &sample(), None).unwrap();
    let loaded = cache(&dir).load(&peterborough()).unwrap().alerts.unwrap();

    assert_eq!(loaded.checked_at, earlier.checked_at);
    assert_eq!(
        loaded
            .alerts
            .iter()
            .map(|a| a.id.as_str())
            .collect::<Vec<_>>(),
        ["a"]
    );
}

#[test]
fn the_alert_poll_updates_a_file_that_exists_and_starts_none() {
    let dir = TempDir::new();
    let mut later = report(vec![alert("b", "Frost advisory", None)]);
    later.checked_at = Some(eastern("2026-09-12T00:30:00"));

    cache(&dir).save_alerts(&peterborough(), &later).unwrap();
    assert_eq!(cache(&dir).load(&peterborough()), None);
    assert!(!dir.join("cache").exists());

    cache(&dir)
        .save(&peterborough(), &sample(), Some(&report(Vec::new())))
        .unwrap();
    cache(&dir).save_alerts(&peterborough(), &later).unwrap();
    let loaded = cache(&dir).load(&peterborough()).unwrap().alerts.unwrap();

    assert_eq!(loaded.checked_at, later.checked_at);
    assert_eq!(
        loaded
            .alerts
            .iter()
            .map(|a| a.id.as_str())
            .collect::<Vec<_>>(),
        ["b"]
    );
}

#[test]
fn missing_unreadable_and_foreign_files_read_as_no_cache() {
    let dir = TempDir::new();
    assert_eq!(cache(&dir).load(&peterborough()), None);

    std::fs::create_dir_all(dir.join("cache")).unwrap();
    let path = file_of(&dir);
    std::fs::write(&path, "{ this is not json").unwrap();
    assert_eq!(cache(&dir).load(&peterborough()), None);

    std::fs::write(
        &path,
        "{\"version\":1,\"forecast\":{\"fetchedAt\":\"2026-09-11T23:58:00.0000000-04:00\"}}",
    )
    .unwrap();
    assert_eq!(cache(&dir).load(&peterborough()), None);

    std::fs::write(&path, "{\"version\":99}").unwrap();
    assert_eq!(cache(&dir).load(&peterborough()), None);

    // A bad file is simply replaced by the next fetch.
    cache(&dir).save(&peterborough(), &sample(), None).unwrap();
    assert!(cache(&dir).load(&peterborough()).is_some());
}

#[test]
fn pruning_keeps_the_saved_locations_and_drops_the_rest() {
    let dir = TempDir::new();
    let toronto = common::toronto();
    cache(&dir).prune([&peterborough()]).unwrap();
    cache(&dir).save(&peterborough(), &sample(), None).unwrap();
    cache(&dir).save(&toronto, &sample(), None).unwrap();
    std::fs::write(dir.join("cache").join("notes.txt"), "not ours").unwrap();

    let mut home = peterborough();
    home.nickname = Some("Home".to_string());
    cache(&dir).prune([&home]).unwrap();

    assert!(cache(&dir).load(&peterborough()).is_some());
    assert_eq!(cache(&dir).load(&toronto), None);
    assert!(dir.join("cache").join("notes.txt").exists());
}
