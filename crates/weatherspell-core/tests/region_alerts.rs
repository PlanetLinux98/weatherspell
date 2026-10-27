// Alerts for Hong Kong (the Observatory), the Philippines (PAGASA) and
// South Africa (SAWS). PAGASA's and SAWS's CAP files are real ones from
// October 2026 (Typhoon Rosal; KwaZulu-Natal and Gauteng's storms), with
// the feeds that list them written here, so each test sees only what was
// listed at its moment. The Observatory had nothing in force when these
// were written, so its answers follow the form its documentation gives.

mod common;

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use common::fixture;
use jiff::Timestamp;
use weatherspell_core::alerts::{AlertSeverity, WeatherAlert};
use weatherspell_core::fetch::{Fetch, FetchError, ForecastService};
use weatherspell_core::location::Location;
use weatherspell_core::open_meteo;
use weatherspell_core::{hko, pagasa, saws};

#[derive(Default)]
struct Fake {
    answers: HashMap<String, String>,
    asked: Mutex<Vec<String>>,
}

impl Fake {
    fn with(mut self, url: &str, body: String) -> Fake {
        self.answers.insert(url.to_string(), body);
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

fn lines(alerts: &[WeatherAlert]) -> Vec<(&str, AlertSeverity)> {
    alerts
        .iter()
        .map(|a| (a.event.as_str(), a.severity))
        .collect()
}

// --- Hong Kong

const SUMMARY: &str = r#"{
  "WRAIN": {"name": "Rainstorm Warning Signal", "code": "WRAINA", "actionCode": "ISSUE",
            "issueTime": "2026-10-26T08:15:00+08:00", "updateTime": "2026-10-26T08:15:00+08:00"},
  "WTCSGNL": {"name": "Tropical Cyclone Warning Signal", "code": "TC8NE", "type": "TC8NE",
              "actionCode": "ISSUE", "issueTime": "2026-10-26T06:40:00+08:00",
              "updateTime": "2026-10-26T06:40:00+08:00"},
  "WTS": {"name": "Thunderstorm Warning", "code": "WTS", "actionCode": "EXTEND",
          "issueTime": "2026-10-26T07:00:00+08:00", "expireTime": "2026-10-26T11:00:00+08:00",
          "updateTime": "2026-10-26T08:00:00+08:00"},
  "WFIRE": {"name": "Fire Danger Warning", "code": "WFIREY", "actionCode": "CANCEL",
            "issueTime": "2026-10-25T06:45:00+08:00", "updateTime": "2026-10-26T06:00:00+08:00"}
}"#;

const DETAILS: &str = r#"{"details": [
  {"contents": ["The Amber Rainstorm Warning Signal was issued at 8:15 a.m.",
                "Heavy rain has fallen or is expected to fall generally over Hong Kong."],
   "warningStatementCode": "WRAIN", "subtype": "WRAINA", "updateTime": "2026-10-26T08:15:00+08:00"},
  {"contents": ["The No. 8 Northeast Gale or Storm Signal is in force."],
   "warningStatementCode": "WTCSGNL", "subtype": "TC8NE", "updateTime": "2026-10-26T06:40:00+08:00"},
  {"contents": ["Thunderstorms are expected to occur over Hong Kong."],
   "warningStatementCode": "WTS", "updateTime": "2026-10-26T08:00:00+08:00"}
]}"#;

#[test]
fn hong_kong_hears_what_the_observatory_has_in_force() {
    let fake = Fake::default()
        .with(hko::SUMMARY_URL, SUMMARY.into())
        .with(hko::DETAILS_URL, DETAILS.into());
    let kowloon = place("Kowloon", "Hong Kong", 22.3167, 114.1833);
    let report = ForecastService::new().alerts(&fake, &kowloon, at("2026-10-26T00:30:00Z"));
    assert_eq!(
        report.attribution.as_deref(),
        Some("Hong Kong Observatory (hko.gov.hk)")
    );
    // The cancelled fire danger warning is gone; the rest most severe
    // first, a warning before a signal.
    use AlertSeverity::*;
    assert_eq!(
        lines(&report.alerts),
        [
            ("Number 8 northeast gale or storm signal", Severe),
            ("Thunderstorm warning", Moderate),
            ("Amber rainstorm warning signal", Moderate),
        ]
    );
    let rain = &report.alerts[2];
    assert_eq!(rain.id, "hk:WRAINA/1792973700");
    assert_eq!(rain.source, "the Hong Kong Observatory");
    assert_eq!(rain.area, "Hong Kong");
    assert_eq!(
        rain.description,
        "The Amber Rainstorm Warning Signal was issued at 8:15 a.m.\n\n\
         Heavy rain has fallen or is expected to fall generally over Hong Kong."
    );
    assert_eq!(report.alerts[1].ends, Some(at("2026-10-26T03:00:00Z")));
    assert_eq!(report.alerts[0].ends, None);

    // Nothing in force is an empty object.
    let quiet = Fake::default()
        .with(hko::SUMMARY_URL, "{}".into())
        .with(hko::DETAILS_URL, "{}".into());
    let report = ForecastService::new().alerts(&quiet, &kowloon, at("2026-10-26T00:30:00Z"));
    assert!(report.checked());
    assert!(report.alerts.is_empty());
}

#[test]
fn hong_kong_is_named_by_the_search_and_known_by_its_point_without_a_name() {
    let json = r#"{"results": [{"name": "Kowloon", "latitude": 22.31667, "longitude": 114.18333,
        "country_code": "HK", "timezone": "Asia/Hong_Kong"},
        {"name": "Hong Kong", "latitude": 22.27832, "longitude": 114.17469, "country_code": "HK"}]}"#;
    let found = open_meteo::parse_search(json).unwrap();
    assert_eq!(found[0].full_name(), "Kowloon, Hong Kong");
    assert_eq!(found[1].full_name(), "Hong Kong");
    assert!(hko::covers(&found[0]));

    // Saved before the search named it.
    let mut saved = found[0].clone();
    saved.country = None;
    assert!(hko::covers(&saved));
    assert!(!hko::covers(&Location::new(
        "Shenzhen",
        None,
        Some("China"),
        22.54,
        114.06
    )));
    assert!(!hko::covers(&Location::new(
        "Nowhere", None, None, 10.0, 10.0
    )));
}

// --- The Philippines

fn pagasa_feed(entries: &[&str]) -> String {
    let mut feed = String::from(
        "<?xml version='1.0' encoding='UTF-8'?>\n<feed xmlns=\"http://www.w3.org/2005/Atom\">\n",
    );
    for entry in entries {
        feed.push_str(&format!(
            "<entry><link type=\"application/cap+xml\" href=\"{entry}\"/></entry>\n"
        ));
    }
    feed.push_str("</feed>\n");
    feed
}

const SIGNAL_2_BATANES: &str =
    "https://publicalert.pagasa.dost.gov.ph/output/wcp/b6cb7676-9a0a-4072-8d5f-e554de03422d.cap";
const SIGNAL_1_CAGAYAN: &str =
    "https://publicalert.pagasa.dost.gov.ph/output/wcp/fdf153e2-fd16-42b0-a001-22a4d625aecb.cap";
const SIGNAL_1_BATANES: &str =
    "https://publicalert.pagasa.dost.gov.ph/output/wcp/a48da0d2-bee0-4c9f-a68b-e8709ff4b0a5.cap";
const FLOOD_MODERATE: &str =
    "https://publicalert.pagasa.dost.gov.ph/output/gfa/f1232c02-5d52-4786-90aa-46865ab991a4.cap";
const FLOOD_FINAL: &str =
    "https://publicalert.pagasa.dost.gov.ph/output/gfa/da3e8383-a066-49ed-99ef-8dd3fe93fb55.cap";

fn pagasa(entries: &[&str]) -> Fake {
    Fake::default()
        .with(pagasa::FEED_URL, pagasa_feed(entries))
        .with(
            SIGNAL_2_BATANES,
            fixture("pagasa-signal-2-batanes-1652.cap"),
        )
        .with(
            SIGNAL_1_CAGAYAN,
            fixture("pagasa-signal-1-cagayan-1652.cap"),
        )
        .with(
            SIGNAL_1_BATANES,
            fixture("pagasa-signal-1-batanes-2242.cap"),
        )
        .with(FLOOD_MODERATE, fixture("pagasa-flood-region7-moderate.cap"))
        .with(FLOOD_FINAL, fixture("pagasa-flood-region7-final.cap"))
}

fn basco() -> Location {
    place("Basco", "Philippines", 20.4487, 121.9702)
}

fn aparri() -> Location {
    place("Aparri", "Philippines", 18.357, 121.638)
}

#[test]
fn a_wind_signal_reaches_the_provinces_it_names() {
    // The 4:52 pm bulletin of 23 October: signal 2 for Batanes, signal 1
    // for Cagayan, Ilocos Norte and Apayao.
    let fake = pagasa(&[SIGNAL_2_BATANES, SIGNAL_1_CAGAYAN]);
    let service = ForecastService::new();
    let now = at("2026-10-23T12:00:00Z");
    let report = service.alerts(&fake, &basco(), now);
    assert_eq!(
        report.attribution.as_deref(),
        Some("PAGASA (pagasa.dost.gov.ph), licensed CC BY 4.0")
    );
    assert_eq!(
        lines(&report.alerts),
        [(
            "Tropical cyclone wind signal number 2",
            AlertSeverity::Moderate
        )]
    );
    let signal = &report.alerts[0];
    assert_eq!(signal.id, "ph:Typhoon Rosal (CHAMPI)/signal");
    assert_eq!(signal.area, "Batanes");
    assert_eq!(signal.source, "PAGASA");
    assert_eq!(signal.ends, None);
    assert!(
        signal
            .description
            .starts_with("Tropical Cyclone Warning : Typhoon Rosal (CHAMPI) Signal #2")
    );
    assert_eq!(
        lines(&service.alerts(&fake, &aparri(), now).alerts),
        [(
            "Tropical cyclone wind signal number 1",
            AlertSeverity::Minor
        )]
    );
    // Manila is under none of them.
    let manila = place("Manila", "Philippines", 14.6042, 120.9822);
    assert!(service.alerts(&fake, &manila, now).alerts.is_empty());
}

#[test]
fn only_a_storms_latest_bulletin_counts() {
    // At 10:42 pm Batanes was lowered to signal 1 and Cagayan's lifted;
    // the earlier bulletin's files had not yet expired.
    let fake = pagasa(&[SIGNAL_1_BATANES, SIGNAL_2_BATANES, SIGNAL_1_CAGAYAN]);
    let service = ForecastService::new();
    let now = at("2026-10-23T15:00:00Z");
    assert_eq!(
        lines(&service.alerts(&fake, &basco(), now).alerts),
        [(
            "Tropical cyclone wind signal number 1",
            AlertSeverity::Minor
        )]
    );
    assert!(service.alerts(&fake, &aparri(), now).alerts.is_empty());
}

#[test]
fn a_flood_advisory_is_placed_by_its_outlines_and_ends_with_its_final_message() {
    let cebu = place("Cebu City", "Philippines", 10.3167, 123.8907);
    let report = ForecastService::new().alerts(
        &pagasa(&[FLOOD_MODERATE]),
        &cebu,
        at("2026-10-25T12:00:00Z"),
    );
    assert_eq!(
        lines(&report.alerts),
        [("General flood advisory (moderate)", AlertSeverity::Moderate)]
    );
    assert_eq!(report.alerts[0].area, "Cebu");
    assert!(
        report.alerts[0]
            .url
            .as_deref()
            .unwrap()
            .starts_with("https://")
    );

    // The final message replaces it, and is an all clear itself; the feed
    // named its server by address, which is moved to its name.
    let by_address = FLOOD_FINAL.replace("publicalert.pagasa.dost.gov.ph", "121.58.193.10");
    let fake = pagasa(&[&by_address, FLOOD_MODERATE]);
    let report = ForecastService::new().alerts(&fake, &cebu, at("2026-10-25T21:25:00Z"));
    assert!(report.checked());
    assert!(report.alerts.is_empty());
    assert!(fake.asked().contains(&FLOOD_FINAL.to_string()));
}

#[test]
fn each_pagasa_file_is_fetched_once() {
    let fake = pagasa(&[SIGNAL_2_BATANES, SIGNAL_1_CAGAYAN]);
    let service = ForecastService::new();
    service.alerts(&fake, &basco(), at("2026-10-23T12:00:00Z"));
    service.alerts(&fake, &aparri(), at("2026-10-23T12:05:00Z"));
    let files = fake.asked().iter().filter(|u| u.ends_with(".cap")).count();
    assert_eq!(files, 2);
}

#[test]
fn provinces_are_found_by_their_outlines() {
    let names = |lat, lon| -> Vec<String> {
        pagasa::provinces_at(lat, lon)
            .iter()
            .map(|p| p.name.clone())
            .collect()
    };
    assert_eq!(names(20.4487, 121.9702), ["Batanes"]);
    assert_eq!(names(14.6042, 120.9822), ["Metro Manila"]);
    assert_eq!(names(7.6, 126.0), ["Davao de Oro"]);
}

// --- South Africa

const SAWS_FILES: &str = "https://www.weathersa.co.za/Documents/Warnings/";

fn saws_feed(items: &[(&str, &str)]) -> String {
    let mut feed = String::from(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<rss version=\"2.0\"><channel>\n",
    );
    for (file, published) in items {
        feed.push_str(&format!(
            "<item><link>http://www.weathersa.co.za/Documents/Warnings/{file}</link>\
             <title> Warning</title><pubDate>{published}</pubDate></item>\n"
        ));
    }
    feed.push_str("</channel></rss>\n");
    feed
}

const KZN_STORMS: &str = "KZN_CAP_SevereThunderstorms_20261019_085422.xml";
const KZN_STORMS_BEFORE: &str = "KZN_CAP_SevereThunderstorms_20261018_084907.xml";
const KZN_WINDS: &str = "KZN_CAP_DamagingWinds_20261019_091747.xml";
const GAUTENG_WINDS: &str = "GP_CAP_DamagingWinds_20261019_095710.xml";

fn saws() -> Fake {
    let mut fake = Fake::default().with(
        saws::FEED_URL,
        saws_feed(&[
            (GAUTENG_WINDS, "Mon, 19 Oct 2026 09:56:48 +0200"),
            (KZN_WINDS, "Mon, 19 Oct 2026 09:15:49 +0200"),
            (KZN_WINDS, "Mon, 19 Oct 2026 09:15:49 +0200"),
            (KZN_STORMS, "Mon, 19 Oct 2026 08:53:16 +0200"),
            (KZN_STORMS_BEFORE, "Sun, 18 Oct 2026 08:47:24 +0200"),
            (
                "WC_CAP_DisruptiveSnow_20250509_084421.xml",
                "Fri, 09 May 2025 08:44:21 +0200",
            ),
        ]),
    );
    for file in [KZN_STORMS, KZN_STORMS_BEFORE, KZN_WINDS, GAUTENG_WINDS] {
        fake = fake.with(
            &format!("{SAWS_FILES}{file}"),
            fixture(&format!("saws-{file}")),
        );
    }
    fake
}

const SAWS_NOW: &str = "2026-10-19T11:00:00Z";

#[test]
fn a_saws_warning_reads_colour_and_level_first() {
    let fake = saws();
    let service = ForecastService::new();
    let durban = place("Durban", "South Africa", -29.8579, 31.0292);
    let report = service.alerts(&fake, &durban, at(SAWS_NOW));
    assert_eq!(
        report.attribution.as_deref(),
        Some("South African Weather Service (weathersa.co.za)")
    );
    assert_eq!(
        lines(&report.alerts),
        [
            (
                "Yellow level 2 warning for damaging winds",
                AlertSeverity::Moderate
            ),
            (
                "Yellow level 4 warning for severe thunderstorms",
                AlertSeverity::Moderate
            ),
        ]
    );
    let storms = &report.alerts[1];
    assert_eq!(
        storms.id,
        "za:KZN_CAP_SevereThunderstorms/Ethekwini / King Shaka Airport"
    );
    assert_eq!(storms.source, "the South African Weather Service");
    assert_eq!(storms.ends, Some(at("2026-10-19T21:59:59Z")));
    assert!(storms.area.contains("Ethekwini / King Shaka Airport"));
    assert!(storms.description.starts_with("- "));

    // Johannesburg, in Gauteng's file.
    let johannesburg = place("Johannesburg", "South Africa", -26.2023, 28.0436);
    assert_eq!(
        lines(&service.alerts(&fake, &johannesburg, at(SAWS_NOW)).alerts),
        [(
            "Yellow level 2 warning for damaging winds",
            AlertSeverity::Moderate
        )]
    );
}

#[test]
fn only_the_newest_file_of_a_kind_counts_and_old_ones_are_not_fetched() {
    let fake = saws();
    let pietermaritzburg = place("Pietermaritzburg", "South Africa", -29.6006, 30.3794);
    let report = ForecastService::new().alerts(&fake, &pietermaritzburg, at(SAWS_NOW));
    // The 18 October file also covered it, for this afternoon, at level 2.
    let storms: Vec<&WeatherAlert> = report
        .alerts
        .iter()
        .filter(|a| a.event.contains("thunderstorms"))
        .collect();
    assert_eq!(storms.len(), 1);
    assert_eq!(
        storms[0].event,
        "Yellow level 4 warning for severe thunderstorms"
    );
    let asked = fake.asked();
    assert!(asked.iter().all(|u| !u.contains("20261018")));
    assert!(asked.iter().all(|u| !u.contains("2025")));
    assert!(asked.iter().all(|u| !u.starts_with("http://")));
}

#[test]
fn saws_lines_of_impacts_read_as_items() {
    let fake = saws();
    let heidelberg = place("Heidelberg", "South Africa", -26.5042, 28.3597);
    let report = ForecastService::new().alerts(&fake, &heidelberg, at(SAWS_NOW));
    let winds = &report.alerts[0];
    assert!(winds.description.lines().all(|l| l.starts_with("- ")));
    assert!(winds.description.lines().all(|l| !l.ends_with(',')));
}
