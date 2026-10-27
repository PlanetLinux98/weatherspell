// Alerts from the CAP sources: MeteoAlarm for Europe and MetService for
// New Zealand, from feeds captured on 2026-10-25 (Storms Austen and Boelo
// in Europe), and what the two share: outlines, the update chain, and
// text made readable aloud.

mod common;

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use common::fixture;
use jiff::Timestamp;
use weatherspell_core::alerts::{self, AlertSeverity, WeatherAlert};
use weatherspell_core::area::Polygon;
use weatherspell_core::cap;
use weatherspell_core::fetch::{Fetch, FetchError, ForecastService};
use weatherspell_core::location::Location;
use weatherspell_core::{meteoalarm, metservice};

#[derive(Default)]
struct Fake {
    answers: HashMap<String, String>,
    asked: Mutex<Vec<String>>,
}

impl Fake {
    fn with(mut self, url: &str, fixture_name: &str) -> Fake {
        self.answers.insert(url.to_string(), fixture(fixture_name));
        self
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().unwrap().clone()
    }
}

impl Fetch for Fake {
    fn get(&self, url: &str, _timeout: Option<Duration>) -> Result<String, FetchError> {
        self.asked.lock().unwrap().push(url.to_string());
        self.answers.get(url).cloned().ok_or(FetchError::Status {
            code: 404,
            reason: "Not Found".into(),
            host: "example".into(),
        })
    }
}

fn at(stamp: &str) -> Timestamp {
    stamp.parse().unwrap()
}

fn place(name: &str, country: &str, latitude: f64, longitude: f64) -> Location {
    Location::new(name, None, Some(country), latitude, longitude)
}

fn feed_for(country_name: &str) -> String {
    meteoalarm::feed_url(meteoalarm::country_of(&place("x", country_name, 0.0, 0.0)).unwrap())
}

fn events(alerts: &[WeatherAlert]) -> Vec<&str> {
    alerts.iter().map(|a| a.event.as_str()).collect()
}

const MORNING: &str = "2026-10-25T07:00:00Z";

// --- Outlines

#[test]
fn an_outline_holds_what_is_inside_it_and_a_margin_reaches_just_past_it() {
    // A square about 11 km on a side.
    let square = Polygon::parse("50.0,10.0 50.1,10.0 50.1,10.1 50.0,10.1 50.0,10.0").unwrap();
    assert!(square.covers(50.05, 10.05, 0.0));
    assert!(!square.covers(50.05, 10.105, 0.0));
    // 0.005 degrees of longitude at 50 north is about 0.36 km.
    assert!(square.covers(50.05, 10.105, 0.75));
    assert!(!square.covers(50.05, 10.12, 0.75));
    assert!(!square.covers(51.0, 10.05, 0.75));
}

#[test]
fn a_hole_is_outside_its_polygon() {
    let with_hole = Polygon::new(vec![
        vec![(0.0, 0.0), (0.0, 4.0), (4.0, 4.0), (4.0, 0.0)],
        vec![(1.0, 1.0), (1.0, 3.0), (3.0, 3.0), (3.0, 1.0)],
    ])
    .unwrap();
    assert!(with_hole.covers(0.5, 0.5, 0.0));
    assert!(!with_hole.covers(2.0, 2.0, 0.0));
}

#[test]
fn an_outline_that_is_not_one_is_refused() {
    assert!(Polygon::parse("50.0,10.0 50.1,10.0").is_none());
    assert!(Polygon::parse("50.0,10.0 north,10.0 50.1,10.1").is_none());
    assert!(Polygon::parse("95.0,10.0 50.1,10.0 50.1,10.1").is_none());
}

#[test]
fn the_regions_table_places_a_location_in_its_region() {
    let regions = |lat, lon| -> Vec<String> {
        meteoalarm::regions_at(lat, lon)
            .iter()
            .map(|r| format!("{} {}", r.code, r.name))
            .collect()
    };
    assert!(regions(52.3759, 9.7320).contains(&"DE169 Region Hannover".to_string()));
    assert!(regions(53.3498, -6.2603).contains(&"IE012 County Dublin".to_string()));
    assert!(regions(51.2194, 4.4025).contains(&"BE002 Antwerpen".to_string()));
    // On the shore of Dublin Bay, but not at sea: the sea areas take no
    // margin.
    assert!(
        regions(53.3498, -6.2603)
            .iter()
            .all(|r| !r.starts_with("EI8"))
    );
}

// --- The text made readable

#[test]
fn html_becomes_paragraphs_and_items() {
    let text = cap::readable(
        "<p>Intense rain associated with Storm Austen.</p><p>&nbsp;</p><p>Prepare for impacts:</p><ul><li>Flooding expected</li><li>Very difficult travelling conditions</li></ul>",
    );
    assert_eq!(
        alerts::paragraphs_of(&text),
        [
            "Intense rain associated with Storm Austen.",
            "Prepare for impacts:",
            "Flooding expected",
            "Very difficult travelling conditions",
        ]
    );
}

#[test]
fn units_written_as_symbols_become_words() {
    assert_eq!(
        cap::readable("Heavy rain 40-80mm/24h."),
        "Heavy rain 40 to 80 millimetres in 24 hours."
    );
    assert_eq!(
        cap::readable("Height range: > 1500 m; Precipitation amounts: 5-10 cm"),
        "Height range: above 1500 metres; Precipitation amounts: 5 to 10 centimetres"
    );
    assert_eq!(
        cap::readable("Minimum temperature: > -1 °C"),
        "Minimum temperature: above minus 1 degrees"
    );
    assert_eq!(
        cap::readable("gusts between 15 and 20 m/s, rates of 15 to 25 mm/h"),
        "gusts between 15 and 20 metres per second, rates of 15 to 25 millimetres per hour"
    );
    // Dates and times are left alone.
    assert_eq!(
        cap::readable("Gale, yellow level, 2026-10-20T06:00:00+00:00"),
        "Gale, yellow level, 2026-10-20T06:00:00+00:00"
    );
}

#[test]
fn references_name_the_messages_replaced() {
    assert_eq!(
        cap::references(
            "forecasts@met.ie,2.49.0.1.372.0.1,2026-10-20T10:42:17+01:00 forecasts@met.ie,2.49.0.1.372.0.2,2026-10-21T10:42:17+01:00"
        ),
        ["2.49.0.1.372.0.1", "2.49.0.1.372.0.2"]
    );
    assert!(cap::references("").is_empty());
}

// --- MeteoAlarm

#[test]
fn a_location_s_country_picks_its_feed() {
    assert_eq!(
        feed_for("United Kingdom"),
        "https://feeds.meteoalarm.org/api/v1/warnings/feeds-united-kingdom"
    );
    assert_eq!(
        feed_for("Czech Republic"),
        "https://feeds.meteoalarm.org/api/v1/warnings/feeds-czechia"
    );
    assert_eq!(
        feed_for("North Macedonia"),
        "https://feeds.meteoalarm.org/api/v1/warnings/feeds-republic-of-north-macedonia"
    );
    assert!(meteoalarm::country_of(&place("Kathmandu", "Nepal", 27.7, 85.3)).is_none());
}

#[test]
fn the_uk_s_warnings_are_read_by_their_outlines_and_orange_is_amber() {
    let edinburgh = place("Edinburgh", "United Kingdom", 55.9533, -3.1883);
    let http = Fake::default().with(
        &feed_for("United Kingdom"),
        "meteoalarm-united-kingdom.json",
    );
    let report = ForecastService::new().alerts(&http, &edinburgh, at(MORNING));

    assert!(report.checked());
    assert_eq!(
        events(&report.alerts),
        [
            "Amber wind warning",
            "Yellow wind warning",
            "Yellow wind warning"
        ]
    );
    let amber = &report.alerts[0];
    assert_eq!(amber.severity, AlertSeverity::Severe);
    assert_eq!(amber.source, "the Met Office");
    assert_eq!(amber.ends, Some(at("2026-10-25T21:00:00Z")));
    assert_eq!(
        amber.url.as_deref(),
        Some("https://www.metoffice.gov.uk/weather/warnings-and-advice/uk-warnings")
    );
    assert_eq!(amber.notice.as_deref(), Some(meteoalarm::DISCLAIMER));
    assert!(amber.id.starts_with("eu:"));
    assert_eq!(
        report.attribution.as_deref(),
        Some("the Met Office through MeteoAlarm (meteoalarm.org), licensed CC BY 4.0")
    );
}

#[test]
fn a_flood_agency_s_warning_keeps_its_own_name_and_place() {
    let new_brighton = place("New Brighton", "United Kingdom", 53.44154, -3.04493);
    let http = Fake::default().with(
        &feed_for("United Kingdom"),
        "meteoalarm-united-kingdom.json",
    );
    let report = ForecastService::new().alerts(&http, &new_brighton, at(MORNING));

    let flood: Vec<&WeatherAlert> = report
        .alerts
        .iter()
        .filter(|a| a.source == "the Environment Agency")
        .collect();
    assert_eq!(flood.len(), 1);
    assert_eq!(flood[0].event, "Flood warning: Irish Sea at New Brighton");
    assert_eq!(flood[0].severity, AlertSeverity::Severe);
    assert!(
        flood[0]
            .description
            .starts_with("Flooding is expected: Irish Sea at New Brighton")
    );
    assert!(
        report
            .attribution
            .as_deref()
            .unwrap()
            .contains("the Environment Agency")
    );
}

#[test]
fn ireland_s_counties_are_found_by_their_codes_and_the_sea_s_warnings_left_out() {
    let dublin = place("Dublin", "Ireland", 53.3498, -6.2603);
    let http = Fake::default().with(&feed_for("Ireland"), "meteoalarm-ireland.json");
    let report = ForecastService::new().alerts(&http, &dublin, at(MORNING));

    // The all-coasts gale warning is at sea; the red gale has ended.
    assert_eq!(events(&report.alerts), ["Yellow wind warning"]);
    let a = &report.alerts[0];
    assert_eq!(a.source, "Met Éireann");
    assert_eq!(
        a.area,
        "Cavan, Dublin, Longford, Louth, Meath, Monaghan, Westmeath"
    );
    assert_eq!(a.ends, Some(at("2026-10-25T12:00:00Z")));
    let paragraphs = alerts::paragraphs_of(&a.description);
    assert_eq!(
        paragraphs[0],
        "Status Yellow - Wind warning for Cavan, Dublin, Longford, Louth, Meath, Monaghan, Westmeath"
    );
    assert_eq!(
        paragraphs[1],
        "Storm Boelo: Very strong southwesterly winds will veer westerly with severe gusts."
    );
    assert!(paragraphs.contains(&"Debris, loose objects displaced".to_string()));
    assert_eq!(a.instruction, None);
    assert_eq!(a.url.as_deref(), Some("https://www.met.ie/warnings"));
}

#[test]
fn a_german_region_s_warnings_come_by_region_code_and_ended_ones_are_dropped() {
    let hannover = place("Hannover", "Germany", 52.3759, 9.7320);
    let http = Fake::default().with(&feed_for("Germany"), "meteoalarm-germany.json");
    let report = ForecastService::new().alerts(&http, &hannover, at(MORNING));

    assert_eq!(
        events(&report.alerts),
        ["Orange wind warning", "Yellow wind warning"]
    );
    assert_eq!(report.alerts[0].source, "Deutscher Wetterdienst");
    // The service's own words follow in the details.
    assert!(
        report.alerts[0]
            .description
            .to_lowercase()
            .contains("gale-force gusts")
    );
}

#[test]
fn a_nuts_area_is_matched_by_name_and_green_is_no_warning() {
    let antwerp = place("Antwerp", "Belgium", 51.2194, 4.4025);
    let http = Fake::default().with(&feed_for("Belgium"), "meteoalarm-belgium.json");
    let report = ForecastService::new().alerts(&http, &antwerp, at(MORNING));

    assert_eq!(events(&report.alerts), ["Yellow coastal event warning"]);
    assert_eq!(
        report.alerts[0].source,
        "the Royal Meteorological Institute of Belgium"
    );
    assert_eq!(report.alerts[0].area, "Antwerp");
}

#[test]
fn one_country_feed_serves_every_location_in_a_round() {
    let http = Fake::default().with(
        &feed_for("United Kingdom"),
        "meteoalarm-united-kingdom.json",
    );
    let service = ForecastService::new();
    let now = at(MORNING);
    service.alerts(
        &http,
        &place("Edinburgh", "United Kingdom", 55.9533, -3.1883),
        now,
    );
    service.alerts(
        &http,
        &place("New Brighton", "United Kingdom", 53.44154, -3.04493),
        at("2026-10-25T07:00:30Z"),
    );
    assert_eq!(http.asked().len(), 1);
    service.alerts(
        &http,
        &place("Edinburgh", "United Kingdom", 55.9533, -3.1883),
        at("2026-10-25T07:10:00Z"),
    );
    assert_eq!(http.asked().len(), 2);
}

#[test]
fn an_unreachable_feed_is_a_problem_not_a_quiet_day() {
    let report = ForecastService::new().alerts(
        &Fake::default(),
        &place("Dublin", "Ireland", 53.3498, -6.2603),
        at(MORNING),
    );
    assert!(!report.checked());
    assert!(report.is_available());
    assert_eq!(
        report.problem.as_deref(),
        Some("404 Not Found from example")
    );
    assert_eq!(
        report.attribution.as_deref(),
        Some("Met Éireann through MeteoAlarm (meteoalarm.org), licensed CC BY 4.0")
    );
}

// Built by hand: a warning, an update of it, an all-clear and a
// cancellation, as the feeds carry them.
fn hand_made_feed() -> String {
    let warning = |id: &str, msg: &str, references: &str, level: &str, response: &str| {
        format!(
            r#"{{"alert":{{"identifier":"{id}","msgType":"{msg}","status":"Actual","sent":"2026-10-25T06:00:00+00:00","references":"{references}","info":[{{"language":"en-GB","event":"x","headline":"","description":"Wind.","expires":"2026-10-25T18:00:00+00:00","responseType":["{response}"],"parameter":[{{"valueName":"awareness_level","value":"{level}"}},{{"valueName":"awareness_type","value":"1; Wind"}}],"area":[{{"areaDesc":"Region Hannover","geocode":[{{"valueName":"EMMA_ID","value":"DE169"}}]}}]}}]}}}}"#
        )
    };
    format!(
        r#"{{"warnings":[{},{},{},{},{}]}}"#,
        warning("A", "Alert", "", "2; yellow; Moderate", "Prepare"),
        warning(
            "B",
            "Update",
            "dwd,A,2026-10-25T05:00:00+00:00",
            "3; orange; Severe",
            "Prepare"
        ),
        warning("C", "Update", "", "2; yellow; Moderate", "AllClear"),
        warning("D", "Alert", "", "2; yellow; Moderate", "Prepare"),
        warning(
            "E",
            "Cancel",
            "dwd,D,2026-10-25T05:00:00+00:00",
            "2; yellow; Moderate",
            "Prepare"
        ),
    )
}

#[test]
fn a_replaced_message_an_all_clear_and_a_cancelled_one_are_left_out() {
    let feed = meteoalarm::parse_feed(&hand_made_feed()).unwrap();
    let germany = meteoalarm::country_of(&place("x", "Germany", 0.0, 0.0)).unwrap();
    let alerts = meteoalarm::alerts_for(
        &feed,
        germany,
        &place("Hannover", "Germany", 52.3759, 9.7320),
        at(MORNING),
    );
    assert_eq!(
        alerts.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(),
        ["eu:B"]
    );
    assert_eq!(
        alerts[0].references.as_deref(),
        Some(&["eu:A".to_string()][..])
    );
}

// --- MetService

fn nz_feed() -> Fake {
    let links = metservice::parse_feed(&fixture("metservice-rss.xml")).unwrap();
    assert_eq!(links.len(), 3);
    let mut http = Fake::default().with(metservice::FEED_URL, "metservice-rss.xml");
    for (n, link) in links.iter().enumerate() {
        http = http.with(link, &format!("metservice-cap-{}.xml", n + 1));
    }
    http
}

const EVENING: &str = "2026-10-25T08:00:00Z";

#[test]
fn new_zealand_s_warnings_read_colour_first_and_set_severity_by_it() {
    let http = nz_feed();
    let wellington = place("Wellington", "New Zealand", -41.2866, 174.7756);
    let report = ForecastService::new().alerts(&http, &wellington, at(EVENING));

    assert!(report.checked());
    assert_eq!(events(&report.alerts), ["Orange strong wind warning"]);
    let a = &report.alerts[0];
    assert_eq!(a.severity, AlertSeverity::Severe);
    assert_eq!(a.source, "MetService");
    assert_eq!(a.area, "Wellington");
    assert_eq!(a.onset, Some(at("2026-10-26T06:00:00Z")));
    assert_eq!(a.ends, Some(at("2026-10-26T20:00:00Z")));
    assert_eq!(
        alerts::paragraphs_of(&a.description),
        [
            "Severe gale northwest winds gusting 120 kilometres per hour in exposed places.",
            "Impact: Damage to trees, powerlines, and unsecured structures possible. Driving may be difficult, especially for high-sided vehicles and motorcycles.",
            "Chance of upgrading to a Red Warning: Minimal.",
        ]
    );
    assert_eq!(
        a.instruction.as_deref(),
        Some(
            "Prepare your property by securing items that can be picked up by strong winds. Drive cautiously."
        )
    );
    assert_eq!(
        report.attribution.as_deref(),
        Some("MetService (metservice.com), licensed CC BY 4.0")
    );
}

#[test]
fn a_watch_keeps_its_name_and_is_moderate() {
    let http = nz_feed();
    let christchurch = place("Christchurch", "New Zealand", -43.5321, 172.6362);
    let report = ForecastService::new().alerts(&http, &christchurch, at(EVENING));

    assert_eq!(events(&report.alerts), ["Strong wind watch"]);
    assert_eq!(report.alerts[0].severity, AlertSeverity::Moderate);
}

#[test]
fn each_cap_file_is_fetched_once() {
    let http = nz_feed();
    let service = ForecastService::new();
    let wellington = place("Wellington", "New Zealand", -41.2866, 174.7756);
    service.alerts(&http, &wellington, at(EVENING));
    service.alerts(&http, &wellington, at("2026-10-25T08:10:00Z"));
    let asked = http.asked();
    assert_eq!(
        asked.iter().filter(|u| *u == metservice::FEED_URL).count(),
        2
    );
    assert_eq!(asked.len(), 2 + 3);
}

// --- The update chain

fn chained(id: &str, references: &[&str], severity: AlertSeverity, event: &str) -> WeatherAlert {
    WeatherAlert {
        id: id.to_string(),
        event: event.to_string(),
        severity,
        issued: at(MORNING),
        onset: None,
        ends: None,
        source: "MetService".into(),
        sender: "MetService".into(),
        area: String::new(),
        level: None,
        description: String::new(),
        instruction: None,
        url: None,
        expires: None,
        references: Some(references.iter().map(|r| r.to_string()).collect()),
        notice: None,
    }
}

#[test]
fn an_update_is_not_announced_again_unless_it_raises_the_alert() {
    let mut seen = Vec::new();
    let watch = chained("nz:1", &[], AlertSeverity::Moderate, "Strong wind watch");
    assert_eq!(
        alerts::track(&mut seen, std::slice::from_ref(&watch)).len(),
        1
    );
    assert_eq!(seen, ["nz:1@23"]);

    // Reissued with new times: the same watch.
    let again = chained(
        "nz:2",
        &["nz:1"],
        AlertSeverity::Moderate,
        "Strong wind watch",
    );
    assert!(alerts::track(&mut seen, std::slice::from_ref(&again)).is_empty());

    // Upgraded to an orange warning: heard.
    let warning = chained(
        "nz:3",
        &["nz:2"],
        AlertSeverity::Severe,
        "Orange strong wind warning",
    );
    assert_eq!(
        alerts::track(&mut seen, std::slice::from_ref(&warning)).len(),
        1
    );

    // Downgraded back to a watch: not.
    let down = chained(
        "nz:4",
        &["nz:3"],
        AlertSeverity::Moderate,
        "Strong wind watch",
    );
    assert!(alerts::track(&mut seen, std::slice::from_ref(&down)).is_empty());
}

#[test]
fn alerts_without_a_chain_are_tracked_by_id_as_before() {
    let mut seen = vec!["ec:1".to_string()];
    let mut a = chained("ec:1", &[], AlertSeverity::Severe, "Yellow wind warning");
    a.references = None;
    assert!(alerts::track(&mut seen, std::slice::from_ref(&a)).is_empty());
    assert_eq!(seen, ["ec:1"]);
}
