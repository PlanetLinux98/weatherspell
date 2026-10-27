// Alerts for Hong Kong from the Hong Kong Observatory's open data
// (DATA.GOV.HK): the warnings and signals in force (warnsum) and their
// texts (warningInfo). They hold for the whole territory, so there are no
// areas to match. The Observatory names a warning by its colour or number
// ("Amber Rainstorm Warning Signal", "No. 8 Northeast Gale or Storm
// Signal") and the summary gives only a code, so the lines take the names
// from the table below, with "No." read as "number". An upgrade (amber to
// red, signal 3 to 8) is a new code, so it is announced as new.

use jiff::Timestamp;
use serde_json::Value;

use crate::alerts::AlertSeverity::{self, Extreme, Minor, Moderate, Severe};
use crate::alerts::WeatherAlert;
use crate::location::Location;
use crate::official::{self, OfficialError};

pub const SUMMARY_URL: &str =
    "https://data.weather.gov.hk/weatherAPI/opendata/weather.php?dataType=warnsum&lang=en";
pub const DETAILS_URL: &str =
    "https://data.weather.gov.hk/weatherAPI/opendata/weather.php?dataType=warningInfo&lang=en";
pub const ALERTS_ATTRIBUTION: &str = "Hong Kong Observatory (hko.gov.hk)";
const SOURCE_NAME: &str = "the Hong Kong Observatory";
const WARNINGS_PAGE: &str = "https://www.hko.gov.hk/en/wxinfo/dailywx/wxwarntoday.htm";

// Open-Meteo's geocoder names no country for Hong Kong; the search now
// does (open_meteo.rs), and a place saved before that is known by where
// it is.
pub fn covers(location: &Location) -> bool {
    match location.country.as_deref() {
        Some(country) => country == "Hong Kong",
        None => {
            (22.13..=22.57).contains(&location.latitude)
                && (113.82..=114.45).contains(&location.longitude)
        }
    }
}

// The Observatory's warnings and signals by code, as its site names them.
const WARNINGS: &[(&str, &str, AlertSeverity)] = &[
    ("WFIREY", "Yellow fire danger warning", Moderate),
    ("WFIRER", "Red fire danger warning", Severe),
    ("WFROST", "Frost warning", Moderate),
    ("WHOT", "Very hot weather warning", Moderate),
    ("WCOLD", "Cold weather warning", Moderate),
    ("WMSGNL", "Strong monsoon signal", Moderate),
    ("WRAINA", "Amber rainstorm warning signal", Moderate),
    ("WRAINR", "Red rainstorm warning signal", Severe),
    ("WRAINB", "Black rainstorm warning signal", Extreme),
    (
        "WFNTSA",
        "Special announcement on flooding in the northern New Territories",
        Moderate,
    ),
    ("WL", "Landslip warning", Severe),
    ("TC1", "Standby signal number 1", Minor),
    ("TC3", "Strong wind signal number 3", Moderate),
    ("TC8NE", "Number 8 northeast gale or storm signal", Severe),
    ("TC8SE", "Number 8 southeast gale or storm signal", Severe),
    ("TC8NW", "Number 8 northwest gale or storm signal", Severe),
    ("TC8SW", "Number 8 southwest gale or storm signal", Severe),
    ("TC9", "Increasing gale or storm signal number 9", Extreme),
    ("TC10", "Hurricane signal number 10", Extreme),
    ("WTMW", "Tsunami warning", Severe),
    ("WTS", "Thunderstorm warning", Moderate),
];

fn unreadable(e: impl std::fmt::Display) -> OfficialError {
    OfficialError::Unreadable(format!(
        "the Hong Kong Observatory's warnings could not be read: {e}"
    ))
}

fn text<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn time(v: &Value, key: &str) -> Option<Timestamp> {
    text(v, key).and_then(|t| t.parse().ok())
}

// The summary is an object of warnings in force, keyed by kind
// ("WRAIN"); a cancelled one may stay listed a while with actionCode
// CANCEL, and a tropical cyclone signal's cancellation is code CANCEL.
pub fn parse(summary: &str, details: &str) -> Result<Vec<WeatherAlert>, OfficialError> {
    let summary: Value = serde_json::from_str(summary).map_err(unreadable)?;
    let details: Value = serde_json::from_str(details).map_err(unreadable)?;
    let texts = details
        .get("details")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let Some(warnings) = summary.as_object() else {
        return Err(unreadable("not an object"));
    };
    let mut alerts = Vec::new();
    for (kind, w) in warnings {
        let code = text(w, "code").unwrap_or(kind);
        if text(w, "actionCode").is_some_and(|a| a.eq_ignore_ascii_case("CANCEL"))
            || code.eq_ignore_ascii_case("CANCEL")
        {
            continue;
        }
        let Some(issued) = time(w, "issueTime").or_else(|| time(w, "updateTime")) else {
            continue;
        };
        let (event, severity) = match WARNINGS.iter().find(|(c, ..)| *c == code) {
            Some((_, name, severity)) => (name.to_string(), *severity),
            None => (
                official::sentence_case(text(w, "name").unwrap_or(kind)),
                Moderate,
            ),
        };
        // Its text: the entry for this kind, and for this sub-type where
        // the details give one (WRAIN's WRAINA).
        let contents: Vec<&str> = texts
            .iter()
            .filter(|d| text(d, "warningStatementCode") == Some(kind.as_str()))
            .filter(|d| text(d, "subtype").is_none_or(|s| s == code))
            .flat_map(|d| d.get("contents").and_then(Value::as_array))
            .flatten()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .collect();
        alerts.push(WeatherAlert {
            id: format!("hk:{code}/{}", issued.as_second()),
            event,
            severity,
            issued,
            onset: None,
            ends: time(w, "expireTime"),
            source: SOURCE_NAME.to_string(),
            sender: SOURCE_NAME.to_string(),
            area: "Hong Kong".to_string(),
            level: None,
            description: contents.join("\n\n"),
            instruction: None,
            url: Some(WARNINGS_PAGE.to_string()),
            expires: None,
            references: None,
            notice: None,
        });
    }
    Ok(alerts)
}
