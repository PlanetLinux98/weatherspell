// The US National Weather Service: api.weather.gov (no key, but a
// User-Agent naming the app and a contact) and the data behind the
// weather.gov forecast page. points/{lat},{lon} resolves a location to a
// forecast grid and its nearby stations, and answers 404 outside the US,
// which is how coverage is decided; the page's data (or failing that, the
// grid's forecast) carries the written periods, the nearest station the
// latest observation. The addresses and the parsing live here; fetch.rs
// does the asking.

use jiff::Timestamp;
use jiff::civil::Date;
use jiff::fmt::temporal::Pieces;
use serde::Deserialize;

use crate::forecast::OfficialPeriod;
use crate::location::Location;
use crate::official::{self, Observation, OfficialError};
use crate::units::UnitSystem;

pub const SOURCE_NAME: &str = "National Weather Service";
pub const ENDPOINT: &str = "https://api.weather.gov";

// The points lookup is keyed by the coordinates as the C# app wrote them.
pub fn points_url(location: &Location) -> String {
    format!("{ENDPOINT}/points/{}", point_key(location))
}

pub fn point_key(location: &Location) -> String {
    format!(
        "{},{}",
        coordinate(location.latitude),
        coordinate(location.longitude)
    )
}

fn coordinate(value: f64) -> String {
    crate::open_meteo::coordinate(value)
}

// The grid's forecast text is written in US units unless asked for SI
// ("High near 16. Southwest wind 7 to 13 km/h."), which keeps a metric
// reader's whole text in one system.
pub fn forecast_url(grid_forecast_url: &str, units: UnitSystem) -> String {
    if units == UnitSystem::Metric {
        format!("{grid_forecast_url}?units=si")
    } else {
        grid_forecast_url.to_string()
    }
}

// The forecast page's data, in SI when asked as the API is.
pub fn page_url(location: &Location, units: UnitSystem) -> String {
    let url = format!(
        "https://forecast.weather.gov/MapClick.php?lat={}&lon={}&FcstType=json",
        coordinate(location.latitude),
        coordinate(location.longitude)
    );
    if units == UnitSystem::Metric {
        url + "&unit=1"
    } else {
        url
    }
}

pub fn observation_url(station_id: &str) -> String {
    format!(
        "{ENDPOINT}/stations/{}/observations/latest",
        crate::open_meteo::escape(station_id)
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Points {
    pub forecast_url: String,
    pub stations_url: String,
    pub attribution: String,
}

#[derive(Deserialize)]
struct PointsResponse {
    properties: Option<PointsProperties>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PointsProperties {
    forecast: Option<String>,
    observation_stations: Option<String>,
    relative_location: Option<RelativeLocation>,
}

#[derive(Deserialize)]
struct RelativeLocation {
    properties: Option<RelativeLocationProperties>,
}

#[derive(Deserialize)]
struct RelativeLocationProperties {
    city: Option<String>,
    state: Option<String>,
}

fn present(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|v| !v.is_empty())
}

pub fn parse_points(json: &str) -> Result<Points, OfficialError> {
    let response: PointsResponse = serde_json::from_str(json)?;
    let p = response.properties.ok_or_else(|| {
        OfficialError::Unreadable("NWS points response has no properties.".into())
    })?;
    let (Some(forecast), Some(stations)) = (present(&p.forecast), present(&p.observation_stations))
    else {
        // American Samoa's points have alerts but no forecast grid (#19).
        return Err(OfficialError::NoForecastText(
            "NWS points response names no forecast grid.".into(),
        ));
    };
    let mut attribution = format!("{SOURCE_NAME} (weather.gov)");
    if let Some(near) = p.relative_location.and_then(|r| r.properties)
        && let Some(city) = present(&near.city)
    {
        attribution += &match present(&near.state) {
            Some(state) => format!(", forecast for {city}, {state}"),
            None => format!(", forecast for {city}"),
        };
    }
    Ok(Points {
        forecast_url: forecast.to_string(),
        stations_url: stations.to_string(),
        attribution,
    })
}

#[derive(Deserialize)]
struct StationsResponse {
    features: Option<Vec<StationFeature>>,
}

#[derive(Deserialize)]
struct StationFeature {
    properties: Option<Station>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Station {
    station_identifier: Option<String>,
    name: Option<String>,
}

// The list is nearest first; the first station with an identifier is used.
pub fn parse_stations(json: &str) -> Result<Option<(String, Option<String>)>, OfficialError> {
    let response: StationsResponse = serde_json::from_str(json)?;
    for feature in response.features.unwrap_or_default() {
        if let Some(station) = feature.properties
            && let Some(id) = present(&station.station_identifier)
        {
            return Ok(Some((id.to_string(), station.name)));
        }
    }
    Ok(None)
}

#[derive(Deserialize)]
struct ForecastResponse {
    properties: Option<ForecastProperties>,
}

#[derive(Deserialize)]
struct ForecastProperties {
    periods: Option<Vec<Period>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Period {
    name: Option<String>,
    start_time: Option<String>,
    detailed_forecast: Option<String>,
}

fn blank(value: &Option<String>) -> bool {
    value.as_deref().is_none_or(|v| v.trim().is_empty())
}

// The offset in a start time is the location's own, so its date is the
// local date; a night period starts in the evening of its day.
fn local_date(stamp: &str) -> Result<Date, OfficialError> {
    Pieces::parse(stamp)
        .map(|p| p.date())
        .map_err(|_| OfficialError::Unreadable(format!("NWS time \"{stamp}\" could not be read.")))
}

pub fn parse_periods(json: &str) -> Result<Vec<OfficialPeriod>, OfficialError> {
    let response: ForecastResponse = serde_json::from_str(json)?;
    let p = response
        .properties
        .ok_or_else(|| OfficialError::Unreadable("NWS forecast has no properties.".into()))?;
    let mut list = Vec::new();
    for period in p.periods.unwrap_or_default() {
        if blank(&period.name)
            || blank(&period.detailed_forecast)
            || period.start_time.as_deref().is_none_or(str::is_empty)
        {
            continue;
        }
        list.push(OfficialPeriod {
            name: period_name(period.name.as_deref().unwrap_or_default()),
            date: local_date(period.start_time.as_deref().unwrap_or_default())?,
            text: official::spoken(period.detailed_forecast.as_deref().unwrap_or_default()),
        });
    }
    if list.is_empty() {
        return Err(OfficialError::Unreadable(
            "NWS forecast has no periods.".into(),
        ));
    }
    Ok(list)
}

// forecast.weather.gov/MapClick.php?FcstType=json: the data behind the
// weather.gov forecast page, not part of the API. Periods are parallel
// arrays, every value a string.
#[derive(Deserialize)]
struct PageResponse {
    time: Option<PageTime>,
    data: Option<PageData>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageTime {
    start_period_name: Option<Vec<Option<String>>>,
    start_valid_time: Option<Vec<Option<String>>>,
}

#[derive(Deserialize)]
struct PageData {
    text: Option<Vec<Option<String>>>,
}

// The text the weather.gov forecast page shows. The API's own is written
// from the same forecast by a different sentence generator, which reads
// worse and at times says something else (NOTES.md), so it only stands in
// when the page's data cannot be had or read.
pub fn parse_page_periods(json: &str) -> Result<Vec<OfficialPeriod>, OfficialError> {
    let page: PageResponse = serde_json::from_str(json)?;
    let misaligned = || {
        OfficialError::Unreadable(
            "Forecast page data has no periods, or periods that do not line up.".into(),
        )
    };
    let time = page.time.ok_or_else(misaligned)?;
    let (Some(names), Some(starts), Some(texts)) = (
        time.start_period_name,
        time.start_valid_time,
        page.data.and_then(|d| d.text),
    ) else {
        return Err(misaligned());
    };
    if names.len() != starts.len() || names.len() != texts.len() {
        return Err(misaligned());
    }
    let mut list = Vec::new();
    for ((name, start), text) in names.iter().zip(&starts).zip(&texts) {
        if blank(name) || blank(text) || start.as_deref().is_none_or(str::is_empty) {
            continue;
        }
        list.push(OfficialPeriod {
            name: period_name(name.as_deref().unwrap_or_default()),
            date: local_date(start.as_deref().unwrap_or_default())?,
            text: official::spoken(text.as_deref().unwrap_or_default()),
        });
    }
    if list.is_empty() {
        return Err(OfficialError::Unreadable(
            "Forecast page data has no periods.".into(),
        ));
    }
    Ok(list)
}

// "Saturday Night" and "This Afternoon" as Environment Canada and this
// app's own headings case them.
fn period_name(name: &str) -> String {
    let mut n = name.trim().to_string();
    if let Some(day) = n.strip_suffix(" Night") {
        n = format!("{day} night");
    }
    if n == "This Afternoon" || n == "Late Afternoon" {
        n = format!("{}afternoon", &n[..n.len() - 9]);
    }
    n
}

// Measured values are objects with a nullable "value" (null with a
// qualityControl flag when not reported).
#[derive(Deserialize)]
struct Value {
    value: Option<f64>,
}

#[derive(Deserialize)]
struct ObservationResponse {
    properties: Option<ObservationProperties>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ObservationProperties {
    station_name: Option<String>,
    timestamp: Option<String>,
    text_description: Option<String>,
    temperature: Option<Value>,
    dewpoint: Option<Value>,
    wind_direction: Option<Value>,
    wind_speed: Option<Value>,
    wind_gust: Option<Value>,
    barometric_pressure: Option<Value>,
    sea_level_pressure: Option<Value>,
    visibility: Option<Value>,
    relative_humidity: Option<Value>,
    wind_chill: Option<Value>,
    heat_index: Option<Value>,
}

fn value(v: &Option<Value>) -> Option<f64> {
    v.as_ref().and_then(|v| v.value)
}

pub fn parse_observation(
    json: &str,
    station_name: Option<&str>,
) -> Result<Observation, OfficialError> {
    let response: ObservationResponse = serde_json::from_str(json)?;
    let o = response
        .properties
        .ok_or_else(|| OfficialError::Unreadable("NWS observation has no properties.".into()))?;
    let stamp = o
        .timestamp
        .as_deref()
        .filter(|t| !t.is_empty())
        .ok_or_else(|| OfficialError::Unreadable("NWS observation has no timestamp.".into()))?;
    let time: Timestamp = stamp.parse().map_err(|_| {
        OfficialError::Unreadable(format!(
            "NWS observation time \"{stamp}\" could not be read."
        ))
    })?;
    let pressure_pa = value(&o.sea_level_pressure).or(value(&o.barometric_pressure));
    Ok(Observation {
        station: official::station_name(
            station_name
                .or(o.station_name.as_deref())
                .unwrap_or("the nearest station"),
        ),
        time,
        description: o
            .text_description
            .as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty())
            .map(str::to_string),
        temperature_c: value(&o.temperature),
        feels_like_c: value(&o.heat_index).or(value(&o.wind_chill)),
        humidity: value(&o.relative_humidity).map(crate::round_even),
        wind_kmh: value(&o.wind_speed),
        wind_direction: value(&o.wind_direction).map(crate::round_even),
        wind_gust_kmh: value(&o.wind_gust),
        pressure_hpa: pressure_pa.map(|pa| pa / 100.0),
        dew_point_c: value(&o.dewpoint),
        visibility_metres: value(&o.visibility),
    })
}
