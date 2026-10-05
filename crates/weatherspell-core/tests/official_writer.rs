// OfficialForecastWriterTests.cs, ported: the official text laid over a
// base forecast, composition first, then the reading it produces.

mod common;

use common::{fixture, offset};
use jiff::civil::{DateTime, date};
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};
use weatherspell_core::clock::TimeFormat;
use weatherspell_core::environment_canada as ec;
use weatherspell_core::forecast::{CurrentConditions, DayForecast, Forecast, HourPoint};
use weatherspell_core::location::Location;
use weatherspell_core::nws;
use weatherspell_core::official::{self, OfficialForecast};
use weatherspell_core::units::UnitSystem;
use weatherspell_core::writer::{self, Section, WriterOptions};

fn eastern(local: DateTime) -> Timestamp {
    offset(-4).to_timestamp(local).unwrap()
}

// Friday 11 September 2026, 11:59 pm local, just after the fixture's
// 11:57 pm observation.
fn now() -> Timestamp {
    eastern(date(2026, 9, 11).at(23, 59, 0, 0))
}

fn options_at(now: Timestamp) -> WriterOptions {
    WriterOptions {
        now,
        pc_zone: TimeZone::fixed(offset(-4)),
        time_format: TimeFormat::new("h:mm tt", "AM", "PM"),
        refresh_problem: None,
    }
}

fn options() -> WriterOptions {
    options_at(now())
}

#[allow(clippy::too_many_arguments)]
fn day(
    d: jiff::civil::Date,
    high: f64,
    low: f64,
    feels_high: f64,
    rise: (i8, i8),
    set: (i8, i8),
    daylight: f64,
    uv: f64,
    prob: i32,
    wind: f64,
    gusts: f64,
    direction: i32,
) -> DayForecast {
    DayForecast {
        date: d,
        weather_code: 1,
        high,
        low,
        feels_like_high: feels_high,
        feels_like_low: low,
        sunrise: Some(d.at(rise.0, rise.1, 0, 0)),
        sunset: Some(d.at(set.0, set.1, 0, 0)),
        daylight_seconds: Some(daylight),
        uv_index_max: Some(uv),
        precipitation_sum: 0.0,
        precipitation_probability_max: Some(prob),
        snowfall_sum: 0.0,
        wind_speed_max: wind,
        wind_gusts_max: gusts,
        wind_direction_dominant: direction,
    }
}

fn base_with(units: UnitSystem) -> Forecast {
    let mut peterborough = Location::new(
        "Peterborough",
        Some("Ontario"),
        Some("Canada"),
        44.30,
        -78.33,
    );
    peterborough.time_zone_id = Some("America/Toronto".to_string());
    let current = CurrentConditions {
        local_time: date(2026, 9, 11).at(23, 45, 0, 0),
        temperature: 12.4,
        feels_like: 11.0,
        humidity: 88,
        weather_code: 45,
        is_day: false,
        wind_speed: 6.0,
        wind_direction: 200,
        wind_gusts: 6.0,
        precipitation: 0.0,
        cloud_cover: 20,
        pressure_hpa: 1017.2,
        station: None,
        description: None,
        dew_point: None,
        visibility_metres: None,
    };
    let start = date(2026, 9, 11).at(0, 0, 0, 0);
    let hours = (0..48)
        .map(|h| HourPoint {
            local_time: start + SignedDuration::from_hours(h),
            temperature: 12.0,
            precipitation_probability: Some(0),
            precipitation: 0.0,
            weather_code: 1,
            wind_speed: 5.0,
            wind_direction: 200,
            wind_gusts: 8.0,
            visibility_metres: Some(20000.0),
            dew_point: Some(10.5),
            humidity: Some(85),
            is_day: h % 24 > 6 && h % 24 < 20,
        })
        .collect();
    let days = vec![
        day(
            date(2026, 9, 11),
            21.0,
            7.0,
            21.0,
            (6, 47),
            (19, 32),
            45900.0,
            5.0,
            0,
            12.0,
            20.0,
            200,
        ),
        day(
            date(2026, 9, 12),
            26.0,
            18.0,
            31.0,
            (6, 48),
            (19, 30),
            45720.0,
            6.2,
            40,
            20.0,
            35.0,
            180,
        ),
    ];
    Forecast {
        location: peterborough,
        fetched_at: now() - SignedDuration::from_mins(1),
        utc_offset: offset(-4),
        units,
        current,
        hours,
        days,
        periods: Vec::new(),
        source_name: "Open-Meteo".to_string(),
        sources: vec![
            "Forecast and current conditions: Open-Meteo (open-meteo.com), licensed CC BY 4.0."
                .to_string(),
        ],
    }
}

fn base() -> Forecast {
    base_with(UnitSystem::Metric)
}

fn official() -> OfficialForecast {
    ec::parse(&fixture("ec-citypage-peterborough.xml")).unwrap()
}

fn find(sections: &[Section], heading: &str) -> Section {
    let found: Vec<&Section> = sections.iter().filter(|s| s.heading == heading).collect();
    assert_eq!(found.len(), 1, "one section headed {heading}");
    found[0].clone()
}

fn headings(sections: &[Section]) -> Vec<&str> {
    sections.iter().map(|s| s.heading.as_str()).collect()
}

#[test]
fn composition_lays_the_observation_and_periods_over_the_base() {
    let f = official::compose(&base(), &official());

    assert_eq!(f.source_name, "Environment Canada and Open-Meteo");
    assert_eq!(f.periods.len(), 12);
    let c = &f.current;
    assert_eq!(c.station.as_deref(), Some("Peterborough Municipal Airport"));
    assert_eq!(c.description.as_deref(), Some("Mist"));
    assert_eq!(c.local_time, date(2026, 9, 11).at(23, 57, 0, 0));
    assert_eq!(c.temperature, 9.2);
    assert_eq!(c.feels_like, 9.2); // no humidex or wind chill reported: never the model's apparent temperature
    assert_eq!(c.humidity, 100);
    assert_eq!(c.wind_speed, 4.0);
    assert_eq!(c.wind_gusts, 4.0);
    assert_eq!(c.wind_direction, 260);
    assert!((c.pressure_hpa - 1018.0).abs() < 1e-3);
    assert_eq!(c.dew_point, Some(9.2));
    assert_eq!(c.visibility_metres, Some(6400.0));
    assert_eq!(c.cloud_cover, 20); // the base's; stations do not report it
    assert_eq!(
        f.sources,
        [
            "Forecast text and current conditions: Environment and Climate Change Canada (weather.gc.ca), forecast for Peterborough City - Lakefield - Southern Peterborough County, observed at Peterborough Municipal Airport.",
            "Hourly data, sun and UV: Open-Meteo (open-meteo.com), licensed CC BY 4.0.",
        ]
    );
}

#[test]
fn observed_values_are_converted_to_imperial_units() {
    let c = official::compose(&base_with(UnitSystem::Imperial), &official()).current;
    assert!((c.temperature - 48.56).abs() < 0.005);
    assert!((c.wind_speed - 2.49).abs() < 0.005);
    assert!((c.dew_point.unwrap() - 48.56).abs() < 0.005);
}

#[test]
fn without_an_observation_the_base_conditions_stay_and_the_sources_say_so() {
    let mut no_observation = official();
    no_observation.observation = None;
    let f = official::compose(&base(), &no_observation);

    assert_eq!(f.current.station, None);
    assert_eq!(f.current.temperature, 12.4);
    assert_eq!(
        f.sources,
        [
            "Forecast text: Environment and Climate Change Canada (weather.gc.ca), forecast for Peterborough City - Lakefield - Southern Peterborough County.",
            "Current conditions, hourly data, sun and UV: Open-Meteo (open-meteo.com), licensed CC BY 4.0.",
        ]
    );
}

#[test]
fn the_reading_keeps_its_order_with_official_periods_under_the_day_headings() {
    let sections = writer::write(&official::compose(&base(), &official()), &options(), None);

    assert_eq!(
        headings(&sections),
        [
            "Alerts",
            "Right now",
            "Rest of today",
            "Saturday, September 12",
            "Sunday, September 13",
            "Monday, September 14",
            "Tuesday, September 15",
            "Wednesday, September 16",
            "Thursday, September 17",
            "Sources"
        ]
    );
    assert_eq!(
        find(&sections, "Rest of today").paragraphs,
        [
            "Tonight: Clear. Fog patches developing near midnight. Low 7.",
            "The sun set at 7:32 pm and rises at 6:48 am tomorrow."
        ]
    );
    let saturday = find(&sections, "Saturday, September 12");
    assert_eq!(
        saturday.paragraphs[0],
        "Saturday: Sunny. Fog patches dissipating early in the morning. Wind becoming south 20 kilometres per hour in the afternoon. High 26. Humidex 31. UV index 6 or high."
    );
    assert_eq!(
        saturday.paragraphs[1],
        "Saturday night: Increasing cloudiness early in the evening. 40 percent chance of showers late in the evening and overnight with risk of thunderstorms. Low 18."
    );
    assert_eq!(
        find(&sections, "Thursday, September 17").paragraphs,
        ["Thursday: Cloudy. High 21."]
    );
}

#[test]
fn right_now_names_the_station_and_uses_its_words() {
    let section = find(
        &writer::write(&official::compose(&base(), &official()), &options(), None),
        "Right now",
    );
    assert_eq!(
        section.paragraphs[0],
        "As of 11:57 pm, Peterborough Municipal Airport reports mist and 9 degrees. Wind from the west at 4 kilometres an hour."
    );
}

#[test]
fn measurements_prefer_observed_dew_point_and_visibility() {
    let section = find(
        &writer::write(&official::compose(&base(), &official()), &options(), None),
        "Right now",
    );
    assert_eq!(
        section.paragraphs[1],
        "Humidity 100 percent, dew point 9 degrees, pressure 1018 hectopascals, visibility 6 kilometres, cloud cover 20 percent."
    );
}

#[test]
fn before_dawn_last_nights_period_is_still_todays() {
    // 1:30 am Saturday: "Tonight" (Friday's) is in progress and leads the
    // day, with Saturday's own periods after it. Saturday's text gives the
    // UV index, so the sun line is not followed by another.
    let at = options_at(eastern(date(2026, 9, 12).at(1, 30, 0, 0)));
    let sections = writer::write(&official::compose(&base(), &official()), &at, None);

    let today = find(&sections, "Rest of today").paragraphs;
    assert_eq!(today.len(), 4);
    assert!(today[0].starts_with("Tonight: "));
    assert!(today[1].starts_with("Saturday: "));
    assert!(today[1].ends_with("UV index 6 or high."));
    assert!(today[2].starts_with("Saturday night: "));
    assert_eq!(
        today[3],
        "The sun rises at 6:48 am and sets at 7:30 pm, 12 hours and 42 minutes of daylight."
    );
    assert_eq!(sections[3].heading, "Sunday, September 13");
}

#[test]
fn a_failed_official_fetch_is_explained_ahead_of_the_sources() {
    let problem =
        official::fetch_problem("Environment Canada", "404 Not Found from dd.weather.gc.ca");
    let f = official::with_problem(base(), Some(problem));
    let section = find(&writer::write(&f, &options(), None), "Sources");

    assert_eq!(section.paragraphs.len(), 2);
    assert!(
        section.paragraphs[0]
            .starts_with("Environment Canada's forecast text could not be fetched")
    );
    assert!(section.paragraphs[1].starts_with("Forecast and current conditions: Open-Meteo"));
}

#[test]
fn the_nws_words_read_the_same_way() {
    let official = OfficialForecast {
        source_name: "National Weather Service".to_string(),
        attribution: "National Weather Service (weather.gov), forecast for Albany, NY".to_string(),
        periods: nws::parse_periods(&fixture("nws-forecast-albany.json")).unwrap(),
        observation: None,
    };
    let at = options_at(eastern(date(2026, 9, 12).at(2, 30, 0, 0)));
    let sections = writer::write(&official::compose(&base(), &official), &at, None);

    assert_eq!(
        find(&sections, "Rest of today").paragraphs,
        [
            "Overnight: Patchy fog after 3am. Mostly clear, with a low around 48. Wind around 0 miles per hour.",
            "Saturday: Patchy fog before 9am. Mostly sunny, with a high near 77. South wind 0 to 12 miles per hour.",
            "Saturday night: A slight chance of rain showers between 8pm and 11pm, then showers and thunderstorms. Mostly cloudy, with a low around 60. South wind 7 to 10 miles per hour, with gusts as high as 21 miles per hour. Chance of precipitation is 90 percent. New rainfall amounts between a half and three quarters of an inch possible.",
            "The sun rises at 6:48 am and sets at 7:30 pm, 12 hours and 42 minutes of daylight.",
            "UV index 6, high.",
        ]
    );
    assert_eq!(sections[sections.len() - 2].heading, "Friday, September 18");
}

#[test]
fn open_meteo_alone_still_reads_as_before() {
    let sections = writer::write(&base(), &options(), None);
    assert_eq!(
        headings(&sections),
        [
            "Alerts",
            "Right now",
            "Rest of today",
            "Saturday, September 12",
            "Sources"
        ]
    );
    assert!(
        find(&sections, "Right now").paragraphs[0]
            .starts_with("As of 11:45 pm, it's 12 degrees and foggy")
    );
}

// From AlertWriterTests: the alerts lead the text and are credited last.
#[test]
fn the_forecast_text_leads_with_the_alerts_and_credits_their_source() {
    use weatherspell_core::alerts::{AlertReport, AlertSeverity, WeatherAlert};
    let a = WeatherAlert {
        id: "id-Rainfall warning".to_string(),
        event: "Rainfall warning".to_string(),
        severity: AlertSeverity::Severe,
        issued: eastern(date(2026, 9, 11).at(13, 10, 0, 0)),
        onset: None,
        ends: Some(eastern(date(2026, 9, 11).at(23, 0, 0, 0))),
        source: "Environment Canada".to_string(),
        sender: "Environment Canada".to_string(),
        area: "Peterborough City - Lakefield - Southern Peterborough County".to_string(),
        level: None,
        description: "Text.".to_string(),
        instruction: None,
        url: None,
        expires: None,
    };
    let report = AlertReport {
        alerts: vec![a.clone()],
        attribution: Some(ec::ALERTS_ATTRIBUTION.to_string()),
        problem: None,
        checked_at: None,
    };
    let at = options_at(eastern(date(2026, 9, 11).at(14, 45, 0, 0)));

    let sections = writer::write(&base(), &at, Some(&report));
    assert_eq!(sections[0].heading, "Alerts");
    assert_eq!(
        sections[0].paragraphs,
        [
            "Rainfall warning until 11:00 pm today, from Environment Canada. Press Enter for details."
        ]
    );
    assert_eq!(sections[0].alerts, Some(vec![Some(a)]));
    assert_eq!(
        sections.last().unwrap().paragraphs.last().unwrap(),
        "Alerts: Environment and Climate Change Canada (weather.gc.ca)."
    );

    let quiet = writer::write(&base(), &at, Some(&AlertReport::not_available()));
    assert_eq!(
        quiet[0].paragraphs,
        ["Alerts are not available for this region."]
    );
    assert!(
        !quiet
            .last()
            .unwrap()
            .paragraphs
            .iter()
            .any(|p| p.starts_with("Alerts:"))
    );
}
