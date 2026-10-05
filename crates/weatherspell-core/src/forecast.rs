// Source-neutral forecast: what the prose is written from. Values are
// already in the location's unit system; times are the location's local
// wall-clock time unless they are Timestamps. Periods is empty when there
// is no official text and the sentences are generated from the hours.

use jiff::Timestamp;
use jiff::civil::{Date, DateTime};
use jiff::tz::Offset;

use crate::location::Location;
use crate::units::UnitSystem;

#[derive(Clone, Debug)]
pub struct Forecast {
    pub location: Location,
    pub fetched_at: Timestamp,
    pub utc_offset: Offset,
    pub units: UnitSystem,
    pub current: CurrentConditions,
    pub hours: Vec<HourPoint>,
    pub days: Vec<DayForecast>,
    pub periods: Vec<OfficialPeriod>,
    pub source_name: String,
    pub sources: Vec<String>,
}

// Station and description are set when a weather service's observation is
// shown: the service's own condition words ("Mist", "Patchy Fog") rather
// than a code; dew point and visibility likewise when observed.
#[derive(Clone, Debug)]
pub struct CurrentConditions {
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

#[derive(Clone, Debug)]
pub struct HourPoint {
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
}

#[derive(Clone, Debug)]
pub struct DayForecast {
    pub date: Date,
    pub weather_code: i32,
    pub high: f64,
    pub low: f64,
    pub feels_like_high: f64,
    pub feels_like_low: f64,
    pub sunrise: Option<DateTime>,
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
#[derive(Clone, Debug)]
pub struct OfficialPeriod {
    pub name: String,
    pub date: Date,
    pub text: String,
}
