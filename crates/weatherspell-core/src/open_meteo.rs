// Forecast and geocoding from Open-Meteo (no key; non-commercial fair use;
// CC BY 4.0). Values are requested in the location's units so nothing is
// converted here; times come back in the location's own zone. Only the
// requests' addresses and the parsing live here: the app does the fetching.

use std::fmt;

use jiff::Timestamp;
use jiff::civil::{Date, DateTime};
use jiff::tz::Offset;
use serde::Deserialize;

use crate::forecast::{CurrentConditions, DayForecast, Forecast, HourPoint};
use crate::location::{Location, NWS_TERRITORIES};
use crate::round_even;
use crate::units::{self, UnitSystem};

pub const SOURCE_NAME: &str = "Open-Meteo";
pub const SOURCE_NOTE: &str = "Open-Meteo (open-meteo.com), licensed CC BY 4.0";

const FORECAST_ENDPOINT: &str = "https://api.open-meteo.com/v1/forecast";
const GEOCODING_ENDPOINT: &str = "https://geocoding-api.open-meteo.com/v1/search";

const CURRENT_FIELDS: &str = "temperature_2m,relative_humidity_2m,apparent_temperature,is_day,precipitation,weather_code,cloud_cover,wind_speed_10m,wind_direction_10m,wind_gusts_10m,pressure_msl";
const HOURLY_FIELDS: &str = "temperature_2m,precipitation_probability,precipitation,weather_code,wind_speed_10m,wind_direction_10m,wind_gusts_10m,visibility,dew_point_2m,relative_humidity_2m,is_day";
const DAILY_FIELDS: &str = "weather_code,temperature_2m_max,temperature_2m_min,apparent_temperature_max,apparent_temperature_min,sunrise,sunset,daylight_duration,uv_index_max,precipitation_sum,precipitation_probability_max,snowfall_sum,wind_speed_10m_max,wind_gusts_10m_max,wind_direction_10m_dominant";

#[derive(Debug)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

impl From<serde_json::Error> for ParseError {
    fn from(e: serde_json::Error) -> ParseError {
        ParseError(format!("Open-Meteo response could not be read: {e}"))
    }
}

pub fn forecast_url(location: &Location, units: UnitSystem, days: u32) -> String {
    format!(
        "{FORECAST_ENDPOINT}?latitude={}&longitude={}&current={CURRENT_FIELDS}&hourly={HOURLY_FIELDS}&daily={DAILY_FIELDS}&forecast_days={days}&timezone=auto&temperature_unit={}&wind_speed_unit={}&precipitation_unit={}",
        coordinate(location.latitude),
        coordinate(location.longitude),
        units::temperature_parameter(units),
        units::wind_parameter(units),
        units::precipitation_parameter(units),
    )
}

pub fn search_url(query: &str) -> String {
    format!(
        "{GEOCODING_ENDPOINT}?name={}&count=20&language=en&format=json",
        escape(query.trim())
    )
}

// .NET's "0.####": at most four decimals, trailing zeros dropped.
pub(crate) fn coordinate(value: f64) -> String {
    units::trimmed(value, 4)
}

// Uri.EscapeDataString: everything but the unreserved characters.
pub(crate) fn escape(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[derive(Deserialize)]
struct ForecastResponse {
    #[serde(default)]
    utc_offset_seconds: i32,
    current: Option<CurrentBlock>,
    hourly: Option<HourlyBlock>,
    daily: Option<DailyBlock>,
}

#[derive(Deserialize)]
struct CurrentBlock {
    time: Option<String>,
    temperature_2m: Option<f64>,
    relative_humidity_2m: Option<f64>,
    apparent_temperature: Option<f64>,
    is_day: Option<f64>,
    precipitation: Option<f64>,
    weather_code: Option<f64>,
    cloud_cover: Option<f64>,
    wind_speed_10m: Option<f64>,
    wind_direction_10m: Option<f64>,
    wind_gusts_10m: Option<f64>,
    pressure_msl: Option<f64>,
}

type Column = Option<Vec<Option<f64>>>;

#[derive(Deserialize)]
struct HourlyBlock {
    time: Option<Vec<String>>,
    temperature_2m: Column,
    precipitation_probability: Column,
    precipitation: Column,
    weather_code: Column,
    wind_speed_10m: Column,
    wind_direction_10m: Column,
    wind_gusts_10m: Column,
    visibility: Column,
    dew_point_2m: Column,
    relative_humidity_2m: Column,
    is_day: Column,
}

#[derive(Deserialize)]
struct DailyBlock {
    time: Option<Vec<String>>,
    weather_code: Column,
    temperature_2m_max: Column,
    temperature_2m_min: Column,
    apparent_temperature_max: Column,
    apparent_temperature_min: Column,
    sunrise: Option<Vec<Option<String>>>,
    sunset: Option<Vec<Option<String>>>,
    daylight_duration: Column,
    uv_index_max: Column,
    precipitation_sum: Column,
    precipitation_probability_max: Column,
    snowfall_sum: Column,
    wind_speed_10m_max: Column,
    wind_gusts_10m_max: Column,
    wind_direction_10m_dominant: Column,
}

fn at(values: &Column, i: usize) -> Option<f64> {
    values.as_ref().and_then(|v| v.get(i).copied().flatten())
}

fn at_int(values: &Column, i: usize) -> Option<i32> {
    at(values, i).map(round_even)
}

fn time(value: &str) -> Result<DateTime, ParseError> {
    value
        .parse()
        .map_err(|_| ParseError(format!("Open-Meteo time \"{value}\" could not be read.")))
}

fn date(value: &str) -> Result<Date, ParseError> {
    value
        .parse()
        .map_err(|_| ParseError(format!("Open-Meteo date \"{value}\" could not be read.")))
}

fn optional_time(
    values: &Option<Vec<Option<String>>>,
    i: usize,
) -> Result<Option<DateTime>, ParseError> {
    match values
        .as_ref()
        .and_then(|v| v.get(i))
        .and_then(|v| v.as_deref())
    {
        Some(text) if !text.is_empty() => time(text).map(Some),
        _ => Ok(None),
    }
}

// Pure: fixtures feed this directly in tests.
pub fn parse(
    json: &str,
    location: &Location,
    units: UnitSystem,
    fetched_at: Timestamp,
) -> Result<Forecast, ParseError> {
    let r: ForecastResponse = serde_json::from_str(json)?;
    let c = r
        .current
        .ok_or_else(|| ParseError("Open-Meteo response has no current block.".into()))?;
    let h = r
        .hourly
        .ok_or_else(|| ParseError("Open-Meteo response has no hourly block.".into()))?;
    let d = r
        .daily
        .ok_or_else(|| ParseError("Open-Meteo response has no daily block.".into()))?;

    let current = CurrentConditions {
        local_time: time(
            c.time
                .as_deref()
                .ok_or_else(|| ParseError("Open-Meteo time missing.".into()))?,
        )?,
        temperature: c.temperature_2m.unwrap_or(0.0),
        feels_like: c.apparent_temperature.or(c.temperature_2m).unwrap_or(0.0),
        humidity: round_even(c.relative_humidity_2m.unwrap_or(0.0)),
        weather_code: c.weather_code.map_or(0, |v| v as i32),
        is_day: c.is_day.map_or(1, |v| v as i32) == 1,
        wind_speed: c.wind_speed_10m.unwrap_or(0.0),
        wind_direction: round_even(c.wind_direction_10m.unwrap_or(0.0)),
        wind_gusts: c.wind_gusts_10m.or(c.wind_speed_10m).unwrap_or(0.0),
        precipitation: c.precipitation.unwrap_or(0.0),
        cloud_cover: round_even(c.cloud_cover.unwrap_or(0.0)),
        pressure_hpa: c.pressure_msl.unwrap_or(0.0),
        station: None,
        description: None,
        dew_point: None,
        visibility_metres: None,
    };

    let mut hours = Vec::new();
    for (i, t) in h.time.iter().flatten().enumerate() {
        // A missing temperature marks the end of the model's horizon.
        let Some(temperature) = at(&h.temperature_2m, i) else {
            continue;
        };
        hours.push(HourPoint {
            local_time: time(t)?,
            temperature,
            precipitation_probability: at_int(&h.precipitation_probability, i),
            precipitation: at(&h.precipitation, i).unwrap_or(0.0),
            weather_code: at_int(&h.weather_code, i).unwrap_or(0),
            wind_speed: at(&h.wind_speed_10m, i).unwrap_or(0.0),
            wind_direction: at_int(&h.wind_direction_10m, i).unwrap_or(0),
            wind_gusts: at(&h.wind_gusts_10m, i)
                .or(at(&h.wind_speed_10m, i))
                .unwrap_or(0.0),
            visibility_metres: at(&h.visibility, i),
            dew_point: at(&h.dew_point_2m, i),
            humidity: at_int(&h.relative_humidity_2m, i),
            is_day: at_int(&h.is_day, i).unwrap_or(1) == 1,
        });
    }

    let mut days = Vec::new();
    for (i, day) in d.time.iter().flatten().enumerate() {
        let (Some(high), Some(low)) = (at(&d.temperature_2m_max, i), at(&d.temperature_2m_min, i))
        else {
            continue;
        };
        days.push(DayForecast {
            date: date(day)?,
            weather_code: at_int(&d.weather_code, i).unwrap_or(0),
            high,
            low,
            feels_like_high: at(&d.apparent_temperature_max, i).unwrap_or(high),
            feels_like_low: at(&d.apparent_temperature_min, i).unwrap_or(low),
            sunrise: optional_time(&d.sunrise, i)?,
            sunset: optional_time(&d.sunset, i)?,
            daylight_seconds: at(&d.daylight_duration, i),
            uv_index_max: at(&d.uv_index_max, i),
            precipitation_sum: at(&d.precipitation_sum, i).unwrap_or(0.0),
            precipitation_probability_max: at_int(&d.precipitation_probability_max, i),
            snowfall_sum: at(&d.snowfall_sum, i).unwrap_or(0.0),
            wind_speed_max: at(&d.wind_speed_10m_max, i).unwrap_or(0.0),
            wind_gusts_max: at(&d.wind_gusts_10m_max, i).unwrap_or(0.0),
            wind_direction_dominant: at_int(&d.wind_direction_10m_dominant, i).unwrap_or(0),
        });
    }

    let utc_offset = Offset::from_seconds(r.utc_offset_seconds).map_err(|_| {
        ParseError(format!(
            "Open-Meteo offset {} is out of range.",
            r.utc_offset_seconds
        ))
    })?;
    Ok(Forecast {
        location: location.clone(),
        fetched_at,
        utc_offset,
        units,
        current,
        hours,
        days,
        periods: Vec::new(),
        source_name: SOURCE_NAME.to_string(),
        sources: vec![format!("Forecast and current conditions: {SOURCE_NOTE}.")],
    })
}

#[derive(Deserialize)]
struct GeocodingResponse {
    results: Option<Vec<GeocodingResult>>,
}

#[derive(Deserialize)]
struct GeocodingResult {
    name: Option<String>,
    #[serde(default)]
    latitude: f64,
    #[serde(default)]
    longitude: f64,
    country: Option<String>,
    country_code: Option<String>,
    admin1: Option<String>,
    timezone: Option<String>,
    population: Option<i64>,
}

pub fn parse_search(json: &str) -> Result<Vec<Location>, ParseError> {
    let response: GeocodingResponse = serde_json::from_str(json)?;
    let mut list = Vec::new();
    for r in response.results.unwrap_or_default() {
        let Some(name) = r.name.filter(|n| !n.trim().is_empty()) else {
            continue;
        };
        let blank = r.country.as_deref().is_none_or(|c| c.trim().is_empty());
        let territory = r
            .country_code
            .as_deref()
            .and_then(|code| NWS_TERRITORIES.iter().find(|(c, _)| *c == code))
            .map(|(_, name)| name.to_string());
        let country = if blank && territory.is_some() {
            territory
        } else {
            r.country
        };
        list.push(Location {
            name,
            region: r.admin1,
            country,
            latitude: r.latitude,
            longitude: r.longitude,
            time_zone_id: r.timezone,
            nickname: None,
            population: r.population,
        });
    }
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_and_queries_are_written_as_dotnet_did() {
        assert_eq!(coordinate(43.65), "43.65");
        assert_eq!(coordinate(-79.38), "-79.38");
        assert_eq!(coordinate(44.123456), "44.1235");
        assert_eq!(coordinate(10.0), "10");
        assert_eq!(escape("Saint-Jérôme, QC"), "Saint-J%C3%A9r%C3%B4me%2C%20QC");
    }
}
