// Hour by Hour (Ctrl+H; Cmd+Shift+H on the Mac): every hour from the one
// in progress to the end of the forecast, one line each, under a heading
// per day, so Ctrl+PageDown moves a day at a time and arrowing past
// midnight runs on into the next. A weather service's own hours are used
// where its text is the one above them (Environment Canada's first day,
// the NWS's week), so the two never disagree; Open-Meteo's elsewhere and
// after (Elliott, 2026-10-27). The lines are worded like the parts of the
// day: "3:00 pm: overcast, 23 degrees, 20 percent chance of rain, light
// wind." No blank line between hours, so each arrow is the next hour.

use jiff::civil::{Date, DateTime};
use jiff::tz::TimeZone;

use crate::clock;
use crate::forecast::{Forecast, HourPoint, OfficialHour};
use crate::layout::{SectionLayout, units as utf16};
use crate::units::{self, UnitSystem};
use crate::writer::{self, WriterOptions};
use crate::{compass, open_meteo, round_away, weather_codes as codes};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HourlyText {
    pub layout: SectionLayout,
    // Each hour and where its line starts.
    pub hours: Vec<(DateTime, usize)>,
    // Whether lines come before the first hour (the forecast's age, this
    // PC's clock).
    pub lead: bool,
}

impl HourlyText {
    // Where the caret goes for an hour asked for: that hour's line, or the
    // first after it; the last for one beyond the forecast. The hour in
    // progress (or one already past) opens at the top when lines come
    // before it, so they are not skipped unheard.
    pub fn opening(&self, at: Option<DateTime>) -> usize {
        let Some((first_hour, first)) = self.hours.first() else {
            return 0;
        };
        if at.is_none_or(|at| at <= *first_hour) {
            return if self.lead { 0 } else { *first };
        }
        self.hours
            .iter()
            .find(|(t, _)| Some(*t) >= at)
            .or(self.hours.last())
            .map_or(*first, |(_, offset)| *offset)
    }
}

const BREAK: &str = "\n";

enum Line<'a> {
    Official(&'a OfficialHour),
    Generated(&'a HourPoint),
}

impl Line<'_> {
    fn time(&self) -> DateTime {
        match self {
            Line::Official(h) => h.local_time,
            Line::Generated(h) => h.local_time,
        }
    }
}

pub fn write(f: &Forecast, o: &WriterOptions) -> HourlyText {
    let zone = f.zone();
    let now_local = zone.to_datetime(o.now);
    let start = now_local.date().at(now_local.hour(), 0, 0, 0);

    // The service's hours count only while its text is the one shown.
    let official: Vec<&OfficialHour> = match &f.official_hours {
        Some(o) if !f.periods.is_empty() => {
            o.hours.iter().filter(|h| h.local_time >= start).collect()
        }
        _ => Vec::new(),
    };
    let mut lines: Vec<Line> = official.iter().map(|h| Line::Official(h)).collect();
    for h in f.hours.iter().filter(|h| h.local_time >= start) {
        if !official.iter().any(|o| o.local_time == h.local_time) {
            lines.push(Line::Generated(h));
        }
    }
    lines.sort_by_key(Line::time);

    let mut text = HourlyText::default();
    let mut lead = Vec::new();
    if let Some(line) = writer::freshness(
        o.now.duration_since(f.fetched_at),
        o.refresh_problem.as_deref(),
    ) {
        lead.push(line);
    }
    if let Some(line) = zone_note(f, &zone, o, &lines) {
        lead.push(line);
    }
    text.lead = !lead.is_empty();

    if lines.is_empty() {
        let mut paragraphs = lead.clone();
        paragraphs.push(
            "There are no hours left in this forecast. Press F5 in the main window to update it."
                .to_string(),
        );
        section(&mut text, "Hour by hour", &paragraphs);
    }
    let mut day: Option<Date> = None;
    for line in &lines {
        let t = line.time();
        if day != Some(t.date()) {
            if day.is_some() {
                text.append(BREAK);
            }
            day = Some(t.date());
            heading(&mut text, &clock::day_heading(t.date()));
            for paragraph in lead.drain(..) {
                text.append(&paragraph);
                text.append(BREAK);
                text.append(BREAK);
            }
        }
        let words = match line {
            Line::Official(h) => official_line(h, f.units),
            Line::Generated(h) => generated_line(h, f.units),
        };
        text.hours.push((t, text.layout.length));
        text.append(&format!("{}: {words}", o.time_format.format(t)));
        text.append(BREAK);
    }
    if !lines.is_empty() {
        text.append(BREAK);
    }
    let sources = sources(f, o, now_local, &lines);
    section(&mut text, "Sources", &sources);
    text
}

impl HourlyText {
    fn append(&mut self, s: &str) {
        self.layout.text.push_str(s);
        self.layout.length += utf16(s);
    }
}

fn heading(text: &mut HourlyText, heading: &str) {
    text.layout
        .headings
        .push((heading.to_string(), text.layout.length));
    text.append(heading);
    text.append(BREAK);
    text.append(BREAK);
}

// A section as the main text lays one out: paragraphs between blank lines.
fn section(text: &mut HourlyText, title: &str, paragraphs: &[String]) {
    heading(text, title);
    for paragraph in paragraphs {
        text.append(paragraph);
        text.append(BREAK);
        text.append(BREAK);
    }
}

// The lines are the location's own times. Where this PC's clock differs,
// one line says by how much before the first hour, rather than a second
// time on every line; a change to or from daylight time on one side only,
// within the hours shown, is named with the day it happens. Each day is
// compared at noon (the first day from its first hour), so the few hours
// between two places' own changes on the same night are not named.
fn zone_note(f: &Forecast, zone: &TimeZone, o: &WriterOptions, lines: &[Line]) -> Option<String> {
    let first = lines.first()?.time();
    let mut spans: Vec<(Date, i32)> = Vec::new();
    let mut dates: Vec<Date> = lines.iter().map(|l| l.time().date()).collect();
    dates.dedup();
    for date in dates {
        let t = if date == first.date() {
            first
        } else {
            date.at(12, 0, 0, 0)
        };
        let Ok(stamp) = zone.to_ambiguous_timestamp(t).compatible() else {
            continue;
        };
        let difference = zone.to_offset(stamp).seconds() - o.pc_zone.to_offset(stamp).seconds();
        if spans.last().is_none_or(|(_, d)| *d != difference) {
            spans.push((date, difference));
        }
    }
    if spans.iter().all(|(_, d)| *d == 0) {
        return None;
    }
    let name = f
        .location
        .nickname
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or(&f.location.name);
    let mut s = format!("Times are local to {name}, {}", apart(spans[0].1, true));
    for (date, difference) in &spans[1..] {
        s += &format!(
            " until {}, then {}",
            clock::day_word(*date, first.date()),
            apart(*difference, false)
        );
    }
    Some(s + ".")
}

// "4 hours ahead of yours", "3 hours behind yours", later "5 hours ahead",
// "the same as yours".
fn apart(seconds: i32, first: bool) -> String {
    if seconds == 0 {
        return "the same as yours".to_string();
    }
    let amount = clock::duration(f64::from(seconds.abs()));
    match (seconds > 0, first) {
        (true, true) => format!("{amount} ahead of yours"),
        (false, true) => format!("{amount} behind yours"),
        (true, false) => format!("{amount} ahead"),
        (false, false) => format!("{amount} behind"),
    }
}

fn sources(f: &Forecast, o: &WriterOptions, now_local: DateTime, lines: &[Line]) -> Vec<String> {
    let open_meteo_line = format!("Hours: {}.", open_meteo::SOURCE_NOTE);
    let Some(official) = f.official_hours.as_ref().filter(|_| !f.periods.is_empty()) else {
        return vec![open_meteo_line];
    };
    let used: Vec<DateTime> = lines
        .iter()
        .filter(|l| matches!(l, Line::Official(_)))
        .map(Line::time)
        .collect();
    let Some(last) = used.last() else {
        let name = f.source_name.split(" and ").next().unwrap_or_default();
        return match &official.problem {
            Some(problem) => vec![
                format!(
                    "{name}'s hour by hour could not be fetched this time ({}), so these hours are written from Open-Meteo data.",
                    problem.trim_end_matches(['.', ' '])
                ),
                open_meteo_line,
            ],
            None => vec![open_meteo_line],
        };
    };
    if used.len() == lines.len() {
        return vec![format!("Hours: {}.", official.attribution)];
    }
    vec![
        format!(
            "Hours up to {} {}: {}.",
            o.time_format.format(*last),
            clock::day_word(last.date(), now_local.date()),
            official.attribution
        ),
        format!("Other hours: {}.", open_meteo::SOURCE_NOTE),
    ]
}

// Mentioned from 15 percent, in tens, as the parts of the day are.
const MINIMUM_MENTIONED_CHANCE: i32 = 15;

fn tens(chance: i32) -> i32 {
    round_away(chance as f64 / 10.0) * 10
}

fn generated_line(h: &HourPoint, units: UnitSystem) -> String {
    let mut pieces = vec![
        codes::describe(h.weather_code).to_string(),
        units::degrees(h.temperature),
    ];
    if let Some(feels) = feels_like(h.temperature, h.feels_like) {
        pieces.push(feels);
    }
    let chance = h.precipitation_probability.unwrap_or(0);
    if chance >= MINIMUM_MENTIONED_CHANCE {
        let word = if codes::is_precipitation(h.weather_code) {
            codes::precipitation_word(h.weather_code)
        } else {
            word_by_temperature(h.temperature, units)
        };
        pieces.push(format!("{} percent chance of {word}", tens(chance)));
    }
    pieces.push(wind(
        h.wind_speed,
        Some(h.wind_direction),
        Some(h.wind_gusts),
        units,
    ));
    pieces.join(", ") + "."
}

// The service's own words for the hour, with its chance folded in where
// they already speak of one: "Chance Rain Showers" at 48 percent reads "50
// percent chance of rain showers", "Showers" at 60 "showers, 60 percent
// chance".
fn official_line(h: &OfficialHour, units: UnitSystem) -> String {
    // Environment Canada joins phrases with a full stop ("Chance of
    // showers. Risk of thunderstorms"); mid-line they take a comma.
    let mut condition = crate::official::spoken(&h.condition)
        .to_lowercase()
        .replace('/', " or ")
        .replace(". ", ", ")
        .trim_end_matches('.')
        .to_string();
    // The NWS writes "Chance Rain Showers"; spoken, it is "chance of".
    for prefix in ["slight chance ", "chance "] {
        if let Some(rest) = condition.strip_prefix(prefix)
            && !rest.starts_with("of ")
        {
            condition = format!("{prefix}of {rest}");
            break;
        }
    }
    let chance = h.chance.unwrap_or(0);
    let mut pieces = Vec::new();
    let mut chance_said = false;
    if chance >= MINIMUM_MENTIONED_CHANCE {
        let n = tens(chance);
        let rest = ["slight chance of ", "chance of "]
            .iter()
            .find_map(|prefix| condition.strip_prefix(prefix))
            .or_else(|| condition.strip_suffix(" likely"));
        if let Some(rest) = rest {
            pieces.push(format!("{n} percent chance of {rest}"));
            chance_said = true;
        }
    }
    if !chance_said && !condition.is_empty() {
        pieces.push(condition.clone());
    }
    pieces.push(units::degrees(h.temperature));
    if let Some(feels) = feels_like(h.temperature, h.feels_like) {
        pieces.push(feels);
    }
    if !chance_said && chance >= MINIMUM_MENTIONED_CHANCE {
        let n = tens(chance);
        // After the temperature, as in a generated line.
        pieces.push(if names_precipitation(&condition) {
            format!("{n} percent chance")
        } else {
            format!(
                "{n} percent chance of {}",
                word_by_temperature(h.temperature, units)
            )
        });
    }
    pieces.push(wind(h.wind_speed, h.wind_direction, h.wind_gusts, units));
    pieces.join(", ") + "."
}

fn names_precipitation(condition: &str) -> bool {
    [
        "rain",
        "shower",
        "snow",
        "drizzle",
        "flurr",
        "sleet",
        "pellet",
        "freezing",
        "thunder",
        "hail",
        "precipitation",
        "storm",
        "squall",
        "wintry",
    ]
    .iter()
    .any(|w| condition.contains(w))
}

// A feels-like temperature 3 degrees or more from the air's, as the
// coming days mention one.
fn feels_like(temperature: f64, feels: Option<f64>) -> Option<String> {
    let feels = feels?;
    ((round_away(feels) - round_away(temperature)).abs() >= 3)
        .then(|| format!("feeling like {}", units::degrees_bare(feels)))
}

fn word_by_temperature(temperature: f64, units: UnitSystem) -> &'static str {
    let (rain_above, snow_below) = if units == UnitSystem::Metric {
        (3.0, -1.0)
    } else {
        (37.0, 30.0)
    };
    if temperature > rain_above {
        "rain"
    } else if temperature < snow_below {
        "snow"
    } else {
        "precipitation"
    }
}

// "light wind" below a breeze, as the parts of the day say; otherwise the
// wind and, when they add something a listener would feel, its gusts.
fn wind(speed: f64, direction: Option<i32>, gusts: Option<f64>, units: UnitSystem) -> String {
    let (breezy, gust_margin) = if units == UnitSystem::Metric {
        (10.0, 10.0)
    } else {
        (6.0, 6.0)
    };
    if speed < breezy {
        return "light wind".to_string();
    }
    let mut s = match direction {
        Some(d) => format!(
            "wind from the {} at {}",
            compass::from_degrees(d as f64),
            units::speed(speed, units)
        ),
        None => format!("variable wind at {}", units::speed(speed, units)),
    };
    if let Some(gusts) = gusts
        && gusts - speed >= gust_margin
    {
        s += &format!(", gusting to {}", units::speed_bare(gusts));
    }
    s
}
