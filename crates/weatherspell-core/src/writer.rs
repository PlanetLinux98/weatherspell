// Turns a Forecast into sections of sentences meant to be listened to:
// words instead of symbols, whole degrees, one idea per sentence.

use jiff::civil::{Date, DateTime};
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};

use crate::clock::{self, Clock, TimeFormat};
use crate::forecast::{DayForecast, Forecast, HourPoint, OfficialPeriod};
use crate::units::{self, UnitSystem};
use crate::{alerts, compass, round_away, round_even, weather_codes as codes};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    pub heading: String,
    pub paragraphs: Vec<String>,
}

impl Section {
    pub fn new(heading: &str, paragraphs: Vec<String>) -> Section {
        Section {
            heading: heading.to_string(),
            paragraphs,
        }
    }
}

// refresh_problem is why the text on screen was not replaced by the last
// refresh ("couldn't reach the weather service"), or None when it was.
#[derive(Clone, Debug)]
pub struct WriterOptions {
    pub now: Timestamp,
    pub pc_zone: TimeZone,
    pub time_format: TimeFormat,
    pub refresh_problem: Option<String>,
}

pub fn write(f: &Forecast, o: &WriterOptions) -> Vec<Section> {
    let clock = Clock::new(f.utc_offset, o.pc_zone.clone(), o.time_format.clone());
    let now_local = f.utc_offset.to_datetime(o.now);

    let mut sections = vec![
        Section::new(
            alerts::HEADING,
            vec![alerts::NOT_AVAILABLE_LINE.to_string()],
        ),
        right_now(f, o, &clock, now_local),
    ];

    let (today, days) = if !f.periods.is_empty() {
        (official_today(f, now_local), official_days(f, now_local))
    } else {
        let days = f
            .days
            .iter()
            .filter(|d| d.date > now_local.date())
            .map(|day| {
                let following = f
                    .days
                    .iter()
                    .find(|d| Some(d.date) == day.date.tomorrow().ok());
                Section {
                    heading: clock::day_heading(day.date),
                    paragraphs: day_and_night(day, following, f),
                }
            })
            .collect();
        (rest_of_today(f, now_local), days)
    };

    // The sun and UV are today's, so they close Rest of today rather than
    // wait in a section of their own after the coming days.
    let mut rest = today.clone();
    rest.extend(sun_and_uv(f, now_local, &clock, &today));
    if !rest.is_empty() {
        sections.push(Section::new("Rest of today", rest));
    }
    sections.extend(days);
    sections.push(Section::new("Sources", f.sources.clone()));
    sections
}

// A weather service's periods as written, under this app's headings:
// today's under "Rest of today", the rest one section per day, each
// paragraph led by the service's own period name. A night period is dated
// by the day it follows, so before dawn yesterday's is still today's news.
fn official_today(f: &Forecast, now_local: DateTime) -> Vec<String> {
    let today = now_local.date();
    let yesterday = today.yesterday().ok();
    f.periods
        .iter()
        .filter(|p| {
            p.date == today
                || (Some(p.date) == yesterday && is_night(&p.name) && now_local.hour() < 6)
        })
        .map(paragraph)
        .collect()
}

fn official_days(f: &Forecast, now_local: DateTime) -> Vec<Section> {
    let mut groups: Vec<(Date, Vec<String>)> = Vec::new();
    for p in f.periods.iter().filter(|p| p.date > now_local.date()) {
        match groups.iter_mut().find(|(date, _)| *date == p.date) {
            Some((_, paragraphs)) => paragraphs.push(paragraph(p)),
            None => groups.push((p.date, vec![paragraph(p)])),
        }
    }
    groups
        .into_iter()
        .map(|(date, paragraphs)| Section {
            heading: clock::day_heading(date),
            paragraphs,
        })
        .collect()
}

fn paragraph(p: &OfficialPeriod) -> String {
    format!("{}: {}", p.name, p.text)
}

fn is_night(period_name: &str) -> bool {
    period_name == "Tonight"
        || period_name == "Overnight"
        || period_name.to_lowercase().ends_with(" night")
}

fn right_now(f: &Forecast, o: &WriterOptions, clock: &Clock, now_local: DateTime) -> Section {
    let c = &f.current;
    let mut s = String::new();
    if let Some(station) = &c.station {
        // The station's own words for the sky ("mist", "patchy fog"), then
        // this app's numbers; a comma when the words have their own "and"
        // ("mostly cloudy and windy, 13 degrees").
        s += &format!(
            "As of {}, {station} reports ",
            clock.time(c.local_time, now_local)
        );
        if let Some(description) = &c.description {
            let sky = spoken_condition(description);
            s += &if sky.contains(" and ") {
                format!("{sky}, ")
            } else {
                format!("{sky} and ")
            };
        }
        s += &units::degrees(c.temperature);
    } else {
        // "and clear", but "with drizzle": the precipitation and fog phrases
        // are nouns (#21).
        let joiner = if codes::is_noun(c.weather_code) {
            "with"
        } else {
            "and"
        };
        s += &format!(
            "As of {}, it's {} {joiner} {}",
            clock.time(c.local_time, now_local),
            units::degrees(c.temperature),
            codes::describe(c.weather_code)
        );
    }
    if round_away(c.feels_like) != round_away(c.temperature) {
        s += &format!(", feeling like {}", units::degrees_bare(c.feels_like));
    }
    s += ". ";
    s += &wind_sentence(c.wind_speed, c.wind_direction, c.wind_gusts, f.units);

    let mut paragraphs = Vec::new();
    if let Some(line) = freshness(
        o.now.duration_since(f.fetched_at),
        o.refresh_problem.as_deref(),
    ) {
        paragraphs.push(line);
    }
    paragraphs.push(s);
    paragraphs.push(measurements(f, now_local));
    Section::new("Right now", paragraphs)
}

// The finer measurements get a line of their own after the sky, the
// temperature and the wind, so a listener who wants only those can move on
// after one line. They were once a Details section near the end, far from
// the conditions they belong to (#5).
fn measurements(f: &Forecast, now_local: DateTime) -> String {
    let c = &f.current;
    // Dew point and visibility are hourly-only; take the hour nearest now.
    let nearest = first_min_by(&f.hours, |h| h.local_time.duration_since(now_local).abs());
    let mut pieces = vec![format!("Humidity {} percent", c.humidity)];
    if let Some(dew) = c.dew_point.or(nearest.and_then(|h| h.dew_point)) {
        pieces.push(format!("dew point {}", units::degrees(dew)));
    }
    if c.pressure_hpa > 0.0 {
        pieces.push(format!(
            "pressure {}",
            units::pressure(c.pressure_hpa, f.units)
        ));
    }
    if let Some(vis) = c
        .visibility_metres
        .or(nearest.and_then(|h| h.visibility_metres))
    {
        pieces.push(format!("visibility {}", units::distance(vis, f.units)));
    }
    pieces.push(format!("cloud cover {} percent", c.cloud_cover));
    pieces.join(", ") + "."
}

// Fresh text says nothing about its age (the exact time is in the status
// bar). From 30 minutes, or as soon as a refresh has failed and left older
// text on screen, the first line under Right now says how old it is and,
// when there is one, the reason.
pub const STALE_AFTER: SignedDuration = SignedDuration::from_mins(30);

pub fn freshness(age: SignedDuration, problem: Option<&str>) -> Option<String> {
    if problem.is_none() && age < STALE_AFTER {
        return None;
    }
    let when = clock::age_after_from(age);
    Some(match problem {
        None => format!("Showing the forecast from {when}."),
        Some(problem) => format!("Showing the forecast from {when}; {problem}."),
    })
}

// "Fog/Mist" is how the NWS writes fog or mist; mid-sentence and spoken,
// the words are lowercase and the slash is a word.
fn spoken_condition(description: &str) -> String {
    description.trim().to_lowercase().replace('/', " or ")
}

fn wind_sentence(speed: f64, direction: i32, gusts: f64, units: UnitSystem) -> String {
    if speed < 2.0 {
        return "The air is calm.".to_string();
    }
    let mut s = format!(
        "Wind from the {} at {}",
        compass::from_degrees(direction as f64),
        units::speed(speed, units)
    );
    if gusts - speed >= gust_margin(units) {
        s += &format!(", gusting to {}", units::speed_bare(gusts));
    }
    s + "."
}

// Gusts get a mention when they add something a listener would feel.
fn gust_margin(units: UnitSystem) -> f64 {
    if units == UnitSystem::Metric {
        10.0
    } else {
        6.0
    }
}

fn breezy_threshold(units: UnitSystem) -> f64 {
    if units == UnitSystem::Metric {
        10.0
    } else {
        6.0
    }
}

fn rest_of_today(f: &Forecast, now_local: DateTime) -> Vec<String> {
    let today = now_local.date();
    let tomorrow = today.tomorrow().expect("a date within range");
    let five = today.at(5, 0, 0, 0);
    let noon = today.at(12, 0, 0, 0);
    let five_pm = today.at(17, 0, 0, 0);
    let nine_pm = today.at(21, 0, 0, 0);
    let five_tomorrow = tomorrow.at(5, 0, 0, 0);

    // The hour in progress counts: its point is the nearest thing to "now".
    let hour_floor = floor(now_local);
    let mut parts: Vec<(&str, DateTime, DateTime)> = Vec::new();
    if now_local < five {
        parts.push(("Early this morning", hour_floor, five));
    }
    if now_local < noon {
        parts.push(("This morning", hour_floor.max(five), noon));
    }
    if now_local < five_pm {
        parts.push(("This afternoon", hour_floor.max(noon), five_pm));
    }
    if now_local < nine_pm {
        parts.push(("This evening", hour_floor.max(five_pm), nine_pm));
    }
    parts.push(("Overnight", hour_floor.max(nine_pm), five_tomorrow));

    let mut paragraphs = Vec::new();
    let today_day = f.days.iter().find(|d| d.date == today);
    let overnight = between(&f.hours, hour_floor.max(nine_pm), five_tomorrow);
    if let Some(day) = today_day
        && !overnight.is_empty()
    {
        paragraphs.push(format!(
            "Today's high {}, tonight's low {}.",
            units::degrees_bare(day.high),
            units::degrees_bare(min_of(&overnight, |h| h.temperature))
        ));
    }

    for (label, start, end) in parts {
        let hours = between(&f.hours, start, end);
        if hours.is_empty() {
            continue;
        }
        paragraphs.push(part_sentence(label, &hours, f.units));
    }
    paragraphs
}

fn part_sentence(label: &str, hours: &[&HourPoint], units: UnitSystem) -> String {
    let mut pieces = vec![condition_phrase(hours)];

    let first = hours[0].temperature;
    let last = hours[hours.len() - 1].temperature;
    if (round_away(last) - round_away(first)).abs() <= 2 {
        let average = hours.iter().map(|h| h.temperature).sum::<f64>() / hours.len() as f64;
        pieces.push(format!("around {}", units::degrees(average)));
    } else if last < first {
        pieces.push(format!(
            "cooling from {} to {}",
            units::degrees_bare(first),
            units::degrees(last)
        ));
    } else {
        pieces.push(format!(
            "warming from {} to {}",
            units::degrees_bare(first),
            units::degrees(last)
        ));
    }

    let chance = max_chance(hours);
    if chance >= MINIMUM_MENTIONED_CHANCE {
        pieces.push(chance_phrase(hours, chance, units));
    }

    let wind = max_of(hours, |h| h.wind_speed);
    pieces.push(if wind >= breezy_threshold(units) {
        format!("wind up to {}", units::speed(wind, units))
    } else {
        "light wind".to_string()
    });

    format!("{label}: {}.", pieces.join(", "))
}

// The most frequent condition carries the sentence; a rarer but more
// serious one (a shower in an otherwise cloudy afternoon) is added "at
// times" rather than allowed to define the whole period.
fn condition_phrase(hours: &[&HourPoint]) -> String {
    // Codes in order of first appearance, with their counts; ties go to
    // the earlier, as LINQ's stable ordering did.
    let mut groups: Vec<(i32, usize)> = Vec::new();
    for h in hours {
        match groups.iter_mut().find(|(code, _)| *code == h.weather_code) {
            Some((_, count)) => *count += 1,
            None => groups.push((h.weather_code, 1)),
        }
    }
    let common = first_max_by(&groups, |(code, count)| (*count, codes::severity(*code)))
        .unwrap()
        .0;
    let worst = worst_code(hours);
    if worst != common
        && codes::severity(worst) > codes::severity(common)
        && codes::is_precipitation(worst)
    {
        // The same weather harder is "heavier", not named twice ("light snow
        // with snow at times", #21); hail is the one thing a stronger
        // thunderstorm adds.
        if codes::kind(worst) == codes::kind(common) {
            return if common == 95 {
                "thunderstorms, with hail at times".to_string()
            } else {
                format!("{}, heavier at times", codes::describe(common))
            };
        }
        return format!(
            "{} with {} at times",
            codes::describe(common),
            codes::describe(worst)
        );
    }
    codes::describe(common).to_string()
}

fn worst_code(hours: &[&HourPoint]) -> i32 {
    first_max_by(hours, |h| codes::severity(h.weather_code))
        .unwrap()
        .weather_code
}

// Twelve-hour periods, as the official services write them: "Saturday"
// from sunrise to sunset, "Saturday night" until the next sunrise. Built
// from the hourly data; the daily aggregates are the fallback when the
// hours run out near the end of the horizon.
fn day_and_night(day: &DayForecast, following: Option<&DayForecast>, f: &Forecast) -> Vec<String> {
    let day_start = floor(day.sunrise.unwrap_or(day.date.at(6, 0, 0, 0)));
    let day_end = floor(day.sunset.unwrap_or(day.date.at(18, 0, 0, 0)));
    let next_morning = day
        .date
        .tomorrow()
        .expect("a date within range")
        .at(6, 0, 0, 0);
    let night_end = floor(following.and_then(|d| d.sunrise).unwrap_or(next_morning));
    let day_hours = between(&f.hours, day_start, day_end);
    let night_hours = between(&f.hours, day_end, night_end);

    if day_hours.len() < 3 {
        return vec![day_paragraph(day, f.units)];
    }

    let weekday = clock::weekday_name(day.date);
    let mut paragraphs = vec![format!(
        "{weekday}: {}.",
        day_sentence(&day_hours, day, f.units)
    )];
    if night_hours.len() >= 3 {
        paragraphs.push(format!(
            "{weekday} night: {}.",
            night_sentence(&night_hours, f.units)
        ));
    }
    paragraphs
}

fn day_sentence(hours: &[&HourPoint], day: &DayForecast, units: UnitSystem) -> String {
    let mut pieces = vec![condition_phrase(hours)];
    let high = max_of(hours, |h| h.temperature);
    pieces.push(format!("high {}", units::degrees_bare(high)));
    if (round_away(day.feels_like_high) - round_away(high)).abs() >= 3 {
        pieces.push(format!(
            "feeling like {} at the warmest",
            units::degrees_bare(day.feels_like_high)
        ));
    }

    let chance = max_chance(hours);
    if chance >= MINIMUM_MENTIONED_CHANCE {
        let mut piece = chance_phrase(hours, chance, units) + &timing(hours, chance);
        if day.precipitation_sum >= minimum_mentioned_amount(units) {
            piece += &format!(
                ", about {}",
                units::precipitation_amount(day.precipitation_sum, units)
            );
        }
        pieces.push(piece);
    }
    if let Some(snow) = snowfall(day.snowfall_sum, units) {
        pieces.push(format!("snowfall around {snow}"));
    }
    pieces.push(wind_phrase(hours, units));
    pieces.join(", ")
}

fn night_sentence(hours: &[&HourPoint], units: UnitSystem) -> String {
    let mut pieces = vec![
        condition_phrase(hours),
        format!(
            "low {}",
            units::degrees_bare(min_of(hours, |h| h.temperature))
        ),
    ];
    let chance = max_chance(hours);
    if chance >= MINIMUM_MENTIONED_CHANCE {
        pieces.push(chance_phrase(hours, chance, units));
    }
    pieces.push(wind_phrase(hours, units));
    pieces.join(", ")
}

// "30 percent chance of rain": chances rounded to tens, as forecasts say
// them, and the word chosen from the hour's conditions or, failing a
// precipitation code, from how cold the period is.
fn chance_phrase(hours: &[&HourPoint], chance: i32, units: UnitSystem) -> String {
    let worst = worst_code(hours);
    let word = if codes::is_precipitation(worst) {
        codes::precipitation_word(worst)
    } else {
        precipitation_word_by_temperature(hours, units)
    };
    format!("{} percent chance of {word}", tens(chance))
}

fn tens(chance: i32) -> i32 {
    round_away(chance as f64 / 10.0) * 10
}

fn precipitation_word_by_temperature(hours: &[&HourPoint], units: UnitSystem) -> &'static str {
    let coldest = min_of(hours, |h| h.temperature);
    let warmest = max_of(hours, |h| h.temperature);
    let (rain_above, snow_below) = if units == UnitSystem::Metric {
        (3.0, -1.0)
    } else {
        (37.0, 30.0)
    };
    if coldest > rain_above {
        "rain"
    } else if warmest < snow_below {
        "snow"
    } else {
        "precipitation"
    }
}

// Mentioned from 15 percent, which rounds to "20 percent"; below that a
// chance is noise.
const MINIMUM_MENTIONED_CHANCE: i32 = 15;

fn max_chance(hours: &[&HourPoint]) -> i32 {
    hours
        .iter()
        .map(|h| h.precipitation_probability.unwrap_or(0))
        .max()
        .unwrap_or(0)
}

// "in the afternoon" when the likely hours all sit in one part of the day;
// nothing when the chance is spread across the day.
fn timing(hours: &[&HourPoint], chance: i32) -> String {
    let mut likely: Vec<&str> = Vec::new();
    for h in hours
        .iter()
        .filter(|h| h.precipitation_probability.unwrap_or(0) >= chance - 10)
    {
        let part = part_of_day(h.local_time.hour());
        if !likely.contains(&part) {
            likely.push(part);
        }
    }
    if likely.len() == 1 {
        format!(" in the {}", likely[0])
    } else {
        String::new()
    }
}

fn part_of_day(hour: i8) -> &'static str {
    if hour < 12 {
        "morning"
    } else if hour < 17 {
        "afternoon"
    } else {
        "evening"
    }
}

// Direction taken at the strongest hour, so "from the southwest up to 25"
// describes one moment rather than an average of shifting winds.
fn wind_phrase(hours: &[&HourPoint], units: UnitSystem) -> String {
    let strongest = first_max_by(hours, |h| h.wind_speed).unwrap();
    if strongest.wind_speed < breezy_threshold(units) {
        return "light wind".to_string();
    }
    let mut s = format!(
        "wind from the {} up to {}",
        compass::from_degrees(strongest.wind_direction as f64),
        units::speed(strongest.wind_speed, units)
    );
    let gust = max_of(hours, |h| h.wind_gusts);
    if gust - strongest.wind_speed >= gust_margin(units) {
        s += &format!(", gusting to {}", units::speed_bare(gust));
    }
    s
}

fn day_paragraph(d: &DayForecast, units: UnitSystem) -> String {
    let mut s = format!(
        "{}. High {}, low {}.",
        codes::describe_sentence(d.weather_code),
        units::degrees_bare(d.high),
        units::degrees_bare(d.low)
    );
    if (round_away(d.feels_like_high) - round_away(d.high)).abs() >= 3 {
        s += &format!(
            " Feeling like {} at the warmest.",
            units::degrees_bare(d.feels_like_high)
        );
    }
    let chance = d.precipitation_probability_max.unwrap_or(0);
    if chance >= MINIMUM_MENTIONED_CHANCE {
        s += &format!(
            " {} percent chance of {}",
            tens(chance),
            codes::precipitation_word(d.weather_code)
        );
        if d.precipitation_sum >= minimum_mentioned_amount(units) {
            s += &format!(
                ", about {}",
                units::precipitation_amount(d.precipitation_sum, units)
            );
        }
        s += ".";
    }
    if let Some(snow) = snowfall(d.snowfall_sum, units) {
        s += &format!(" Snowfall around {snow}.");
    }
    s += &format!(
        " Wind from the {} up to {}",
        compass::from_degrees(d.wind_direction_dominant as f64),
        units::speed(d.wind_speed_max, units)
    );
    if d.wind_gusts_max - d.wind_speed_max >= gust_margin(units) {
        s += &format!(", gusting to {}", units::speed_bare(d.wind_gusts_max));
    }
    s + "."
}

fn snowfall(sum: f64, units: UnitSystem) -> Option<String> {
    match units {
        UnitSystem::Metric if sum >= 1.0 => Some(units::centimetres(sum)),
        UnitSystem::Imperial if sum >= 0.5 => Some(format!("{} inches", units::one_decimal(sum))),
        _ => None,
    }
}

fn minimum_mentioned_amount(units: UnitSystem) -> f64 {
    if units == UnitSystem::Metric {
        1.0
    } else {
        0.05
    }
}

// The UV index is the day's highest, so it is left out once the sun has
// set, and where the text above already gives it: Environment Canada ends
// a daytime period with its own "UV index 6 or high."
fn sun_and_uv(
    f: &Forecast,
    now_local: DateTime,
    clock: &Clock,
    today_text: &[String],
) -> Vec<String> {
    let Some(today) = f.days.iter().find(|d| d.date == now_local.date()) else {
        return Vec::new();
    };
    let mut paragraphs = Vec::new();
    // Polar day and night: Open-Meteo gives midnight for both sunrise and
    // sunset, which read "rose at 12:00 am and sets at 12:00 am".
    let polar_day = today.daylight_seconds.is_some_and(|d| d >= FULL_DAY);
    let polar_night = today.daylight_seconds.is_some_and(|d| d <= NO_DAY);
    let tomorrow = f
        .days
        .iter()
        .find(|d| Some(d.date) == today.date.tomorrow().ok());
    if polar_day {
        paragraphs.push("The sun does not set today.".to_string());
    } else if polar_night {
        paragraphs.push("The sun does not rise today.".to_string());
    } else if let (Some(spent), Some(next)) = (
        today.sunset,
        tomorrow.filter(|t| !is_polar(t)).and_then(|t| t.sunrise),
    ) && now_local >= spent
    {
        // After sunset the day's own times are spent; the next sunrise is
        // what a listener is waiting for. Its daylight is tomorrow's.
        paragraphs.push(format!(
            "The sun set at {} and rises at {}.",
            clock.time(spent, now_local),
            clock.time_on_day(next, now_local)
        ));
    } else if let (Some(rise), Some(set)) = (today.sunrise, today.sunset) {
        let rises = if now_local < rise { "rises" } else { "rose" };
        let sets = if now_local < set { "sets" } else { "set" };
        let mut s = format!(
            "The sun {rises} at {} and {sets} at {}",
            clock.time(rise, now_local),
            clock.time(set, now_local)
        );
        if let Some(daylight) = today.daylight_seconds {
            s += &format!(", {} of daylight", clock::duration(daylight));
        }
        paragraphs.push(s + ".");
    }

    let sun_up =
        polar_day || (!polar_night && !today.sunset.is_some_and(|sunset| now_local >= sunset));
    let given_above = today_text
        .iter()
        .any(|p| p.to_lowercase().contains("uv index"));
    if let Some(uv) = today.uv_index_max
        && sun_up
        && !given_above
    {
        let level = round_even(uv);
        paragraphs.push(format!("UV index {level}, {}.", uv_band(level)));
    }
    paragraphs
}

const FULL_DAY: f64 = 86400.0 - 60.0;
const NO_DAY: f64 = 60.0;

fn is_polar(d: &DayForecast) -> bool {
    d.daylight_seconds
        .is_some_and(|s| s >= FULL_DAY || s <= NO_DAY)
}

fn uv_band(uv: i32) -> &'static str {
    match uv {
        ..3 => "low",
        3..6 => "moderate",
        6..8 => "high",
        8..11 => "very high",
        _ => "extreme",
    }
}

fn floor(t: DateTime) -> DateTime {
    t.date().at(t.hour(), 0, 0, 0)
}

fn between(hours: &[HourPoint], start: DateTime, end: DateTime) -> Vec<&HourPoint> {
    hours
        .iter()
        .filter(|h| h.local_time >= start && h.local_time < end)
        .collect()
}

fn max_of(hours: &[&HourPoint], value: impl Fn(&HourPoint) -> f64) -> f64 {
    hours
        .iter()
        .map(|h| value(h))
        .fold(f64::NEG_INFINITY, f64::max)
}

fn min_of(hours: &[&HourPoint], value: impl Fn(&HourPoint) -> f64) -> f64 {
    hours.iter().map(|h| value(h)).fold(f64::INFINITY, f64::min)
}

// The first of the greatest, as LINQ's OrderByDescending(...).First()
// gives; Iterator::max_by_key gives the last.
fn first_max_by<T, K: PartialOrd>(items: &[T], key: impl Fn(&T) -> K) -> Option<&T> {
    let mut best: Option<(&T, K)> = None;
    for item in items {
        let k = key(item);
        if best.as_ref().is_none_or(|(_, b)| k > *b) {
            best = Some((item, k));
        }
    }
    best.map(|(item, _)| item)
}

fn first_min_by<T, K: PartialOrd>(items: &[T], key: impl Fn(&T) -> K) -> Option<&T> {
    let mut best: Option<(&T, K)> = None;
    for item in items {
        let k = key(item);
        if best.as_ref().is_none_or(|(_, b)| k < *b) {
            best = Some((item, k));
        }
    }
    best.map(|(item, _)| item)
}
