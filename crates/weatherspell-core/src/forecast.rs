// Source-neutral forecast: what the prose is written from. Values are
// already in the location's unit system; times are the location's local
// wall-clock time unless they are Timestamps. Periods is empty when there
// is no official text and the sentences are generated from the hours.
//
// The serde names are the cache file's (cache.rs), the same as 0.1's
// (CacheFile.cs): a field renamed here keeps its old name in an
// attribute, or neither app can read the other's cache.

use jiff::Timestamp;
use jiff::civil::{Date, DateTime};
use jiff::tz::Offset;
use serde::{Deserialize, Serialize};

use crate::air::AirQuality;
use crate::location::Location;
use crate::units::UnitSystem;

// time_zone is the location's own zone as Open-Meteo names it
// ("America/Toronto"), for the hours a service dates in UTC; utc_offset is
// the offset when fetched. official_hours is a weather service's own hour
// by hour, set only alongside its periods, so the hours never disagree
// with the text above them. air is None when air quality is turned off.
#[derive(Clone, Debug, PartialEq)]
pub struct Forecast {
    pub location: Location,
    pub fetched_at: Timestamp,
    pub utc_offset: Offset,
    pub time_zone: Option<String>,
    pub units: UnitSystem,
    pub current: CurrentConditions,
    pub hours: Vec<HourPoint>,
    pub days: Vec<DayForecast>,
    pub periods: Vec<OfficialPeriod>,
    pub official_hours: Option<OfficialHours>,
    pub air: Option<AirQuality>,
    pub source_name: String,
    pub sources: Vec<String>,
}

impl Forecast {
    // The location's zone, for turning a service's timestamp into the
    // location's wall-clock time across a change to or from daylight
    // time; the fixed offset when the zone is unknown.
    pub fn zone(&self) -> jiff::tz::TimeZone {
        self.time_zone
            .as_deref()
            .and_then(|name| jiff::tz::TimeZone::get(name).ok())
            .unwrap_or_else(|| jiff::tz::TimeZone::fixed(self.utc_offset))
    }

    pub fn wall_time(&self, t: Timestamp) -> DateTime {
        self.zone().to_datetime(t)
    }
}

// Station and description are set when a weather service's observation is
// shown: the service's own condition words ("Mist", "Patchy Fog") rather
// than a code; dew point and visibility likewise when observed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentConditions {
    #[serde(with = "crate::iso::wall")]
    pub local_time: DateTime,
    pub temperature: f64,
    pub feels_like: f64,
    pub humidity: i32,
    pub weather_code: i32,
    pub is_day: bool,
    pub wind_speed: f64,
    pub wind_direction: i32,
    pub wind_gusts: f64,
    pub precipitation: f64,
    pub cloud_cover: i32,
    pub pressure_hpa: f64,
    pub station: Option<String>,
    pub description: Option<String>,
    pub dew_point: Option<f64>,
    pub visibility_metres: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HourPoint {
    #[serde(with = "crate::iso::wall")]
    pub local_time: DateTime,
    pub temperature: f64,
    pub precipitation_probability: Option<i32>,
    pub precipitation: f64,
    pub weather_code: i32,
    pub wind_speed: f64,
    pub wind_direction: i32,
    pub wind_gusts: f64,
    pub visibility_metres: Option<f64>,
    pub dew_point: Option<f64>,
    pub humidity: Option<i32>,
    pub is_day: bool,
    // Not in 0.1's files, which it ignores.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feels_like: Option<f64>,
}

// A weather service's own hour: its condition words as written ("Chance
// of showers", "Mostly Sunny"), in the location's units (Environment
// Canada is metric, as Canada's text is; the NWS is asked in the
// location's units, as its text is). wind_direction is None when the
// service calls it variable.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficialHour {
    #[serde(with = "crate::iso::wall")]
    pub local_time: DateTime,
    pub condition: String,
    pub temperature: f64,
    pub feels_like: Option<f64>,
    pub chance: Option<i32>,
    pub wind_speed: f64,
    pub wind_direction: Option<i32>,
    pub wind_gusts: Option<f64>,
}

// The service's hours and its name for the Sources line; problem is why
// they are missing when its text was had but its hours were not.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficialHours {
    pub attribution: String,
    pub hours: Vec<OfficialHour>,
    pub problem: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayForecast {
    #[serde(with = "crate::iso::date")]
    pub date: Date,
    pub weather_code: i32,
    pub high: f64,
    pub low: f64,
    pub feels_like_high: f64,
    pub feels_like_low: f64,
    #[serde(default, with = "crate::iso::wall_opt")]
    pub sunrise: Option<DateTime>,
    #[serde(default, with = "crate::iso::wall_opt")]
    pub sunset: Option<DateTime>,
    pub daylight_seconds: Option<f64>,
    pub uv_index_max: Option<f64>,
    pub precipitation_sum: f64,
    pub precipitation_probability_max: Option<i32>,
    pub snowfall_sum: f64,
    pub wind_speed_max: f64,
    pub wind_gusts_max: f64,
    pub wind_direction_dominant: i32,
}

// One period of a weather service's written forecast, as the service
// named it ("Tonight", "Saturday", "Saturday night") and worded it. Date
// is the local date the period belongs to: a night belongs to the day it
// follows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OfficialPeriod {
    pub name: String,
    #[serde(with = "crate::iso::date")]
    pub date: Date,
    pub text: String,
}
