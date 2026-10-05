// ForecastWriterTests.cs, ported: the same hand-built forecast and the same
// sentences, verbatim, so a wording change is a deliberate one.

mod common;

use common::{fixture, offset, toronto};
use jiff::civil::{DateTime, date};
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};
use weatherspell_core::clock::TimeFormat;
use weatherspell_core::forecast::{CurrentConditions, DayForecast, Forecast, HourPoint};
use weatherspell_core::open_meteo;
use weatherspell_core::units::UnitSystem;
use weatherspell_core::writer::{self, Section, WriterOptions};

// 2:45 pm local on Thursday 11 September 2026, PC clock in the same zone.
fn now() -> Timestamp {
    at(date(2026, 9, 11).at(14, 45, 0, 0), -4)
}

fn at(local: DateTime, hours: i8) -> Timestamp {
    offset(hours).to_timestamp(local).unwrap()
}

fn options_in(pc_zone: TimeZone) -> WriterOptions {
    WriterOptions {
        now: now(),
        pc_zone,
        time_format: TimeFormat::new("h:mm tt", "AM", "PM"),
        refresh_problem: None,
    }
}

fn options() -> WriterOptions {
    options_in(TimeZone::fixed(offset(-4)))
}

fn minutes_zone(minutes: i32) -> TimeZone {
    TimeZone::fixed(jiff::tz::Offset::from_seconds(minutes * 60).unwrap())
}

#[allow(clippy::too_many_arguments)]
fn hour(
    t: DateTime,
    temp: f64,
    prob: i32,
    precip: f64,
    code: i32,
    wind: f64,
    gusts: f64,
    vis: f64,
    dew: f64,
    humidity: i32,
    is_day: bool,
) -> HourPoint {
    HourPoint {
        local_time: t,
        temperature: temp,
        precipitation_probability: Some(prob),
        precipitation: precip,
        weather_code: code,
        wind_speed: wind,
        wind_direction: 225,
        wind_gusts: gusts,
        visibility_metres: Some(vis),
        dew_point: Some(dew),
        humidity: Some(humidity),
        is_day,
    }
}

#[allow(clippy::too_many_arguments)]
fn day(
    d: jiff::civil::Date,
    code: i32,
    high: f64,
    low: f64,
    feels_high: f64,
    feels_low: f64,
    rise: (i8, i8),
    set: (i8, i8),
    daylight: f64,
    uv: f64,
    precip: f64,
    prob: i32,
    snow: f64,
    wind: f64,
    gusts: f64,
    direction: i32,
) -> DayForecast {
    DayForecast {
        date: d,
        weather_code: code,
        high,
        low,
        feels_like_high: feels_high,
        feels_like_low: feels_low,
        sunrise: Some(d.at(rise.0, rise.1, 0, 0)),
        sunset: Some(d.at(set.0, set.1, 0, 0)),
        daylight_seconds: Some(daylight),
        uv_index_max: Some(uv),
        precipitation_sum: precip,
        precipitation_probability_max: Some(prob),
        snowfall_sum: snow,
        wind_speed_max: wind,
        wind_gusts_max: gusts,
        wind_direction_dominant: direction,
    }
}

fn sample_with(units: UnitSystem, current_code: i32) -> Forecast {
    let current = CurrentConditions {
        local_time: date(2026, 9, 11).at(14, 30, 0, 0),
        temperature: 21.4,
        feels_like: 19.6,
        humidity: 52,
        weather_code: current_code,
        is_day: true,
        wind_speed: 12.0,
        wind_direction: 225,
        wind_gusts: 30.0,
        precipitation: 0.0,
        cloud_cover: 40,
        pressure_hpa: 1016.8,
        station: None,
        description: None,
        dew_point: None,
        visibility_metres: None,
    };

    let mut hours = Vec::new();
    // Today (the 11th): warms to 24.5 at 15:00, then cools a degree an
    // hour; one shower hour at 20:00 inside a partly cloudy evening.
    for h in 0..24 {
        let t = date(2026, 9, 11).at(h, 0, 0, 0);
        let temp = if h < 15 {
            20.0 + h as f64 * 0.3
        } else {
            24.5 - (h - 15) as f64
        };
        let code = if h == 20 { 80 } else { 2 };
        let prob = if h == 20 {
            60
        } else if h == 19 || h == 21 {
            30
        } else {
            0
        };
        let precip = if h == 20 { 0.6 } else { 0.0 };
        hours.push(hour(
            t,
            temp,
            prob,
            precip,
            code,
            8.0 + (h % 5) as f64 * 3.0,
            20.0,
            24100.0,
            11.2,
            55,
            h > 6 && h < 20,
        ));
    }
    // Saturday the 12th: a cool clear night, partly cloudy day with showers
    // at 13:00 and 14:00, clearing evening. Sunday has no hourly data, so
    // it falls back to the daily aggregates.
    let saturday = [
        13.0, 12.5, 12.0, 11.5, 11.5, 11.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 21.5, 22.5, 23.0,
        23.2, 22.8, 22.0, 21.0, 19.5, 18.0, 17.0, 16.0, 15.0,
    ];
    for h in 0..24 {
        let t = date(2026, 9, 12).at(h, 0, 0, 0);
        let code = if h <= 6 || h >= 19 {
            1
        } else if h == 13 || h == 14 {
            80
        } else {
            2
        };
        let prob = match h {
            13 => 55,
            14 => 60,
            12 | 15 => 30,
            _ => 0,
        };
        let wind = 10.0 + (h % 6) as f64 * 3.0;
        let precip = if h == 13 || h == 14 { 2.0 } else { 0.0 };
        hours.push(hour(
            t,
            saturday[h as usize],
            prob,
            precip,
            code,
            wind,
            wind * 1.8,
            20000.0,
            10.0,
            60,
            h > 6 && h < 20,
        ));
    }

    let days = vec![
        day(
            date(2026, 9, 11),
            2,
            24.6,
            14.2,
            25.0,
            13.0,
            (6, 52),
            (19, 34),
            45720.0,
            5.75,
            1.2,
            60,
            0.0,
            18.0,
            32.0,
            225,
        ),
        day(
            date(2026, 9, 12),
            61,
            23.2,
            11.0,
            24.0,
            11.0,
            (6, 53),
            (19, 32),
            45600.0,
            3.1,
            8.4,
            80,
            0.0,
            22.0,
            41.0,
            45,
        ),
        day(
            date(2026, 9, 13),
            0,
            22.0,
            9.5,
            21.0,
            8.0,
            (6, 54),
            (19, 30),
            45480.0,
            6.4,
            0.0,
            0,
            0.0,
            9.0,
            14.0,
            315,
        ),
    ];

    Forecast {
        location: toronto(),
        fetched_at: now() - SignedDuration::from_mins(3),
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

fn sample() -> Forecast {
    sample_with(UnitSystem::Metric, 1)
}

fn find(sections: &[Section], heading: &str) -> Section {
    let found: Vec<&Section> = sections.iter().filter(|s| s.heading == heading).collect();
    assert_eq!(found.len(), 1, "one section headed {heading}");
    found[0].clone()
}

fn write(f: &Forecast) -> Vec<Section> {
    writer::write(f, &options(), None)
}

#[test]
fn sections_come_in_the_agreed_order() {
    let headings: Vec<String> = write(&sample()).into_iter().map(|s| s.heading).collect();
    assert_eq!(
        headings,
        [
            "Alerts",
            "Right now",
            "Rest of today",
            "Saturday, September 12",
            "Sunday, September 13",
            "Sources"
        ]
    );
}

#[test]
fn right_now_reads_as_a_sentence_in_words() {
    let section = find(&write(&sample()), "Right now");
    assert_eq!(
        section.paragraphs[0],
        "As of 2:30 pm, it's 21 degrees and mostly clear, feeling like 20. Wind from the southwest at 12 kilometres an hour, gusting to 30."
    );
    assert_eq!(
        section.paragraphs[1],
        "Humidity 52 percent, dew point 11 degrees, pressure 1017 hectopascals, visibility 24 kilometres, cloud cover 40 percent."
    );
    assert_eq!(section.paragraphs.len(), 2);
}

// Fresh text carries no age; from 30 minutes, or after a failed refresh,
// the first line under Right now says how old it is and why.
#[test]
fn stale_or_unrefreshed_text_is_dated_on_its_first_line() {
    let mut stale = sample();
    stale.fetched_at = now() - SignedDuration::from_mins(45);
    let section = find(&write(&stale), "Right now");
    assert_eq!(
        section.paragraphs[0],
        "Showing the forecast from 45 minutes ago."
    );
    assert!(section.paragraphs[1].starts_with("As of 2:30 pm, "));

    let mut failing = options();
    failing.refresh_problem = Some("couldn't reach the weather service".to_string());
    let failed = find(&writer::write(&sample(), &failing, None), "Right now");
    assert_eq!(
        failed.paragraphs[0],
        "Showing the forecast from 3 minutes ago; couldn't reach the weather service."
    );

    assert_eq!(writer::freshness(SignedDuration::from_mins(29), None), None);
    assert_eq!(
        writer::freshness(SignedDuration::from_mins(75), None).as_deref(),
        Some("Showing the forecast from 1 hour ago.")
    );
    assert_eq!(
        writer::freshness(
            SignedDuration::from_secs(20),
            Some("couldn't reach the weather service")
        )
        .as_deref(),
        Some(
            "Showing the forecast from less than a minute ago; couldn't reach the weather service."
        )
    );
}

#[test]
fn feels_like_is_omitted_when_it_rounds_to_the_temperature() {
    let mut f = sample();
    f.current.feels_like = 21.2;
    assert!(
        find(&write(&f), "Right now").paragraphs[0]
            .starts_with("As of 2:30 pm, it's 21 degrees and mostly clear. ")
    );
}

#[test]
fn calm_air_and_minor_gusts_are_worded_simply() {
    let mut calm = sample();
    calm.current.wind_speed = 1.0;
    calm.current.wind_gusts = 3.0;
    assert!(find(&write(&calm), "Right now").paragraphs[0].contains("The air is calm."));

    let mut steady = sample();
    steady.current.wind_gusts = 15.0;
    assert!(
        find(&write(&steady), "Right now").paragraphs[0]
            .ends_with("Wind from the southwest at 12 kilometres an hour.")
    );
}

#[test]
fn rest_of_today_covers_the_remaining_parts_of_the_day_then_the_sun_and_uv() {
    let p = find(&write(&sample()), "Rest of today").paragraphs;
    assert_eq!(p[0], "Today's high 25, tonight's low 12.");
    assert_eq!(
        p[1],
        "This afternoon: partly cloudy, around 24 degrees, wind up to 20 kilometres an hour."
    );
    assert_eq!(
        p[2],
        "This evening: partly cloudy with light rain showers at times, cooling from 23 to 20 degrees, 60 percent chance of rain, wind up to 20 kilometres an hour."
    );
    assert_eq!(
        p[3],
        "Overnight: mostly clear, cooling from 19 to 12 degrees, 30 percent chance of rain, wind up to 22 kilometres an hour."
    );
    assert_eq!(
        p[4],
        "The sun rose at 6:52 am and sets at 7:34 pm, 12 hours and 42 minutes of daylight."
    );
    assert_eq!(p[5], "UV index 6, high.");
    assert_eq!(p.len(), 6);
}

#[test]
fn coming_days_split_into_day_and_night_from_the_hourly_data() {
    let p = find(&write(&sample()), "Saturday, September 12").paragraphs;
    assert_eq!(
        p[0],
        "Saturday: partly cloudy with light rain showers at times, high 23, 60 percent chance of rain in the afternoon, about 8 millimetres, wind from the southwest up to 25 kilometres an hour, gusting to 45."
    );
    assert_eq!(
        p[1],
        "Saturday night: mostly clear, low 15, wind from the southwest up to 25 kilometres an hour, gusting to 45."
    );
    assert_eq!(p.len(), 2);
}

#[test]
fn a_day_without_hourly_data_falls_back_to_one_paragraph() {
    let p = find(&write(&sample()), "Sunday, September 13").paragraphs;
    assert_eq!(
        p,
        ["Clear. High 22, low 10. Wind from the northwest up to 9 kilometres an hour."]
    );
}

fn saturday_night_changed(f: &Forecast, change: impl Fn(&mut HourPoint)) -> Forecast {
    let mut f = f.clone();
    for h in f
        .hours
        .iter_mut()
        .filter(|h| h.local_time.date() == date(2026, 9, 12) && h.local_time.hour() >= 19)
    {
        change(h);
    }
    f
}

#[test]
fn chances_round_to_tens_and_pick_rain_or_snow_by_temperature() {
    let cold = saturday_night_changed(&sample(), |h| {
        h.temperature = -4.0;
        h.precipitation_probability = Some(26);
        h.weather_code = 3;
    });
    assert_eq!(
        find(&write(&cold), "Saturday, September 12").paragraphs[1],
        "Saturday night: overcast, low minus 4, 30 percent chance of snow, wind from the southwest up to 25 kilometres an hour, gusting to 45."
    );

    let marginal = saturday_night_changed(&sample(), |h| {
        h.temperature = 1.0;
        h.precipitation_probability = Some(14);
        h.weather_code = 3;
    });
    assert!(!find(&write(&marginal), "Saturday, September 12").paragraphs[1].contains("percent"));

    let marginal_wet =
        saturday_night_changed(&marginal, |h| h.precipitation_probability = Some(44));
    assert!(
        find(&write(&marginal_wet), "Saturday, September 12").paragraphs[1]
            .contains("40 percent chance of precipitation")
    );
}

#[test]
fn precipitation_timing_is_omitted_when_the_chance_spans_the_day() {
    let mut f = sample();
    for h in f.hours.iter_mut().filter(|h| {
        h.local_time.date() == date(2026, 9, 12) && (7..=18).contains(&h.local_time.hour())
    }) {
        h.precipitation_probability = Some(50);
        h.weather_code = 61;
    }
    assert!(
        find(&write(&f), "Saturday, September 12").paragraphs[0].starts_with(
            "Saturday: light rain, high 23, 50 percent chance of rain, about 8 millimetres, "
        )
    );
}

// After sunset the next sunrise is the news, and the UV index, the day's
// highest, is nothing to act on.
#[test]
fn after_sunset_the_next_sunrise_is_given_and_the_uv_index_left_out() {
    let mut evening = options();
    evening.now = at(date(2026, 9, 11).at(20, 15, 0, 0), -4);
    let p = find(&writer::write(&sample(), &evening, None), "Rest of today").paragraphs;
    assert_eq!(
        p.last().unwrap(),
        "The sun set at 7:34 pm and rises at 6:53 am tomorrow."
    );
    assert!(!p.iter().any(|p| p.starts_with("UV index")));

    let mut away = options_in(minutes_zone(-7 * 60));
    away.now = evening.now;
    let p = find(&writer::write(&sample(), &away, None), "Rest of today").paragraphs;
    assert_eq!(
        p.last().unwrap(),
        "The sun set at 7:34 pm (4:34 pm your time) and rises at 6:53 am tomorrow (3:53 am tomorrow your time)."
    );

    // Without the next day's sunrise, the day's own times as before.
    let mut last_day = sample();
    last_day.days.truncate(1);
    let p = find(&writer::write(&last_day, &evening, None), "Rest of today").paragraphs;
    assert_eq!(
        p.last().unwrap(),
        "The sun rose at 6:52 am and set at 7:34 pm, 12 hours and 42 minutes of daylight."
    );
}

#[test]
fn sources_close_the_reading() {
    let sections = write(&sample());
    let last = sections.last().unwrap();
    assert_eq!(last.heading, "Sources");
    assert_eq!(
        last.paragraphs[0],
        "Forecast and current conditions: Open-Meteo (open-meteo.com), licensed CC BY 4.0."
    );
}

#[test]
fn times_carry_the_pc_time_in_brackets_when_zones_differ() {
    let sections = writer::write(&sample(), &options_in(minutes_zone(-7 * 60)), None);
    assert!(
        find(&sections, "Right now").paragraphs[0]
            .starts_with("As of 2:30 pm (11:30 am your time), ")
    );
    assert!(
        find(&sections, "Rest of today").paragraphs.contains(
            &"The sun rose at 6:52 am (3:52 am your time) and sets at 7:34 pm (4:34 pm your time), 12 hours and 42 minutes of daylight."
                .to_string()
        )
    );
}

#[test]
fn the_bracket_names_the_reader_s_own_day_when_it_is_not_all_today() {
    // A PC in Kathmandu's zone (UTC+5:45) reading Toronto at 2:45 pm on the
    // 11th is at 12:30 am on the 12th: Toronto's sunrise was the reader's
    // yesterday afternoon, and its afternoon and sunset fall in the
    // reader's today, which the bracket now says.
    let sections = writer::write(&sample(), &options_in(minutes_zone(5 * 60 + 45)), None);
    assert!(
        find(&sections, "Right now").paragraphs[0]
            .starts_with("As of 2:30 pm (12:15 am today your time), ")
    );
    assert!(
        find(&sections, "Rest of today").paragraphs.contains(
            &"The sun rose at 6:52 am (4:37 pm yesterday your time) and sets at 7:34 pm (5:19 am today your time), 12 hours and 42 minutes of daylight."
                .to_string()
        )
    );

    // A PC behind the location, as in Honolulu: Toronto's late evening is
    // the reader's afternoon, all on their today, so no day is named;
    // Toronto's early tomorrow is still the reader's today.
    let clock = weatherspell_core::clock::Clock::new(
        offset(-4),
        minutes_zone(-10 * 60),
        TimeFormat::new("h:mm tt", "AM", "PM"),
    );
    let now = date(2026, 9, 11).at(14, 45, 0, 0);
    assert_eq!(
        clock.time(date(2026, 9, 11).at(23, 30, 0, 0), now),
        "11:30 pm (5:30 pm your time)"
    );
    assert_eq!(
        clock.time(date(2026, 9, 12).at(1, 0, 0, 0), now),
        "1:00 am (7:00 pm today your time)"
    );
}

#[test]
fn precipitation_right_now_reads_with_rather_than_and() {
    let sections = write(&sample_with(UnitSystem::Metric, 53));
    assert!(
        find(&sections, "Right now").paragraphs[0]
            .starts_with("As of 2:30 pm, it's 21 degrees with drizzle, feeling like 20.")
    );
    let foggy = write(&sample_with(UnitSystem::Metric, 45));
    assert!(
        find(&foggy, "Right now").paragraphs[0]
            .starts_with("As of 2:30 pm, it's 21 degrees and foggy")
    );
}

#[test]
fn a_heavier_hour_of_the_same_weather_reads_heavier_at_times() {
    let mut f = sample();
    // Saturday's daytime hours all light snow but one hour of snow.
    for h in f.hours.iter_mut().filter(|h| {
        h.local_time.date() == date(2026, 9, 12) && (7..19).contains(&h.local_time.hour())
    }) {
        h.weather_code = if h.local_time.hour() == 13 { 73 } else { 71 };
    }
    assert!(
        find(&write(&f), "Saturday, September 12").paragraphs[0]
            .starts_with("Saturday: light snow, heavier at times, high 23")
    );
}

#[test]
fn polar_day_and_night_are_said_outright() {
    let f = sample();
    let mut midsummer = f.clone();
    midsummer.days[0].sunrise = Some(date(2026, 9, 11).at(0, 0, 0, 0));
    midsummer.days[0].sunset = Some(date(2026, 9, 12).at(0, 0, 0, 0));
    midsummer.days[0].daylight_seconds = Some(86400.0);
    let p = find(&write(&midsummer), "Rest of today").paragraphs;
    assert_eq!(
        p[p.len() - 2..],
        ["The sun does not set today.", "UV index 6, high."]
    );

    let mut midwinter = f.clone();
    midwinter.days[0].sunrise = Some(date(2026, 9, 11).at(0, 0, 0, 0));
    midwinter.days[0].sunset = Some(date(2026, 9, 11).at(0, 0, 0, 0));
    midwinter.days[0].daylight_seconds = Some(0.0);
    let p = find(&write(&midwinter), "Rest of today").paragraphs;
    assert_eq!(p.last().unwrap(), "The sun does not rise today.");
}

#[test]
fn imperial_units_are_worded_in_miles_and_inches() {
    let sections = write(&sample_with(UnitSystem::Imperial, 1));
    let right_now = find(&sections, "Right now");
    assert!(
        right_now.paragraphs[0]
            .contains("Wind from the southwest at 12 miles an hour, gusting to 30.")
    );
    assert!(
        right_now.paragraphs[1].contains("pressure 30.03 inches of mercury, visibility 15 miles")
    );
}

#[test]
fn negative_temperatures_say_minus() {
    let mut f = sample();
    f.current.temperature = -4.6;
    f.current.feels_like = -11.2;
    assert!(find(&write(&f), "Right now").paragraphs[0].starts_with(
        "As of 2:30 pm, it's minus 5 degrees and mostly clear, feeling like minus 11."
    ));
}

#[test]
fn writes_a_real_response_without_error() {
    let f = open_meteo::parse(
        &fixture("open-meteo-toronto.json"),
        &toronto(),
        UnitSystem::Metric,
        now() - SignedDuration::from_mins(1),
    )
    .unwrap();
    let sections = write(&f);
    assert_eq!(sections[0].heading, "Alerts");
    assert_eq!(sections[1].heading, "Right now");
    assert_eq!(sections.last().unwrap().heading, "Sources");
    assert!(sections.iter().all(|s| !s.paragraphs.is_empty()));
    assert!(
        sections
            .iter()
            .any(|s| s.heading == "Saturday, September 12")
    );
}
