// What a national weather service adds on top of the Open-Meteo base: its
// written forecast periods and its station observation, laid over the base
// by compose(). Observed values are SI (Celsius, kilometres an hour,
// hectopascals, metres) whatever the location's units; compose converts
// them once.

use std::fmt;

use jiff::Timestamp;

use crate::forecast::{CurrentConditions, Forecast, OfficialPeriod};
use crate::open_meteo;
use crate::units;

#[derive(Clone, Debug)]
pub struct OfficialForecast {
    pub source_name: String,
    pub attribution: String,
    pub periods: Vec<OfficialPeriod>,
    pub observation: Option<Observation>,
}

// Nones are values the station did not report this time (the NWS marks
// them as such; Environment Canada leaves the element empty).
#[derive(Clone, Debug, PartialEq)]
pub struct Observation {
    pub station: String,
    pub time: Timestamp,
    pub description: Option<String>,
    pub temperature_c: Option<f64>,
    pub feels_like_c: Option<f64>,
    pub humidity: Option<i32>,
    pub wind_kmh: Option<f64>,
    pub wind_direction: Option<i32>,
    pub wind_gust_kmh: Option<f64>,
    pub pressure_hpa: Option<f64>,
    pub dew_point_c: Option<f64>,
    pub visibility_metres: Option<f64>,
}

// Why a service's response could not be used. NoForecastText is a fact
// about the place, not a failure another try would fix: Environment
// Canada's observation-only Arctic sites (Alert, Eureka), an NWS point
// with no forecast grid (American Samoa), so it is worded as one (#21).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OfficialError {
    NoForecastText(String),
    Unreadable(String),
}

impl fmt::Display for OfficialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OfficialError::NoForecastText(why) | OfficialError::Unreadable(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for OfficialError {}

impl From<serde_json::Error> for OfficialError {
    fn from(e: serde_json::Error) -> OfficialError {
        OfficialError::Unreadable(e.to_string())
    }
}

// The Sources line's first words when the official text could not be had,
// so a reader knows why the sentences are this app's own.
pub fn no_text_problem(source_name: &str) -> String {
    format!(
        "{source_name} has no forecast text for this area, so these sentences are written from Open-Meteo data."
    )
}

pub fn fetch_problem(source_name: &str, reason: &str) -> String {
    // A reason's own full stop would sit inside the brackets.
    let reason = reason.trim_end_matches(['.', ' ']);
    format!(
        "{source_name}'s forecast text could not be fetched this time ({reason}), so these sentences are written from Open-Meteo data."
    )
}

pub fn with_problem(mut f: Forecast, problem: Option<String>) -> Forecast {
    if let Some(problem) = problem {
        f.sources.insert(0, problem);
    }
    f
}

// The official periods and observation over the base; pure, so tests can
// lay a captured response over a captured base.
pub fn compose(f: &Forecast, official: &OfficialForecast) -> Forecast {
    let mut composed = f.clone();
    let mut sources = Vec::new();
    match &official.observation {
        Some(o) if o.temperature_c.is_some() => {
            composed.current = observed(f, o);
            sources.push(format!(
                "Forecast text and current conditions: {}, observed at {}.",
                official.attribution, o.station
            ));
            sources.push(format!(
                "Hourly data, sun and UV: {}.",
                open_meteo::SOURCE_NOTE
            ));
        }
        _ => {
            sources.push(format!("Forecast text: {}.", official.attribution));
            sources.push(format!(
                "Current conditions, hourly data, sun and UV: {}.",
                open_meteo::SOURCE_NOTE
            ));
        }
    }
    composed.periods = official.periods.clone();
    composed.source_name = format!("{} and {}", official.source_name, open_meteo::SOURCE_NAME);
    composed.sources = sources;
    composed
}

// The station's numbers in the location's units; what it did not report
// (a broken humidity sensor, no gust) keeps the base forecast's value, so
// the Right now sentence is always whole. Feels-like is only ever the
// station's own humidex, heat index or wind chill: pairing an observed
// temperature with a modelled apparent one would mislead.
fn observed(f: &Forecast, o: &Observation) -> CurrentConditions {
    let b = &f.current;
    let u = f.units;
    let temperature = units::from_celsius(o.temperature_c.unwrap_or_default(), u);
    let wind = o.wind_kmh.map_or(b.wind_speed, |w| units::from_kmh(w, u));
    CurrentConditions {
        local_time: f.utc_offset.to_datetime(o.time),
        temperature,
        feels_like: o
            .feels_like_c
            .map_or(temperature, |c| units::from_celsius(c, u)),
        humidity: o.humidity.unwrap_or(b.humidity),
        weather_code: b.weather_code,
        is_day: b.is_day,
        wind_speed: wind,
        wind_direction: o.wind_direction.unwrap_or(b.wind_direction),
        wind_gusts: o.wind_gust_kmh.map_or(wind, |g| units::from_kmh(g, u)),
        precipitation: b.precipitation,
        cloud_cover: b.cloud_cover,
        pressure_hpa: o.pressure_hpa.unwrap_or(b.pressure_hpa),
        station: Some(o.station.clone()),
        description: o.description.clone(),
        dew_point: o.dew_point_c.map(|c| units::from_celsius(c, u)),
        visibility_metres: o.visibility_metres,
    }
}

// Official sentences are shown as written, in the service's own units,
// with one typographic pass so they read aloud the way the service says
// them on the radio: symbols become words.
pub fn spoken(text: &str) -> String {
    // The weather.gov page's text has two spaces after a full stop.
    let s = collapse_whitespace_runs(text.trim());
    let s = percent_words(&s);
    let s = replace_word(&s, "km/h", "kilometres per hour");
    replace_word(&s, "mph", "miles per hour")
}

// .NET's \w, near enough: letters, digits and the underscore.
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

// Regex.Replace(s, @"\s{2,}", " "): a run of two or more becomes one
// space; a lone line break stays as it is.
fn collapse_whitespace_runs(s: &str) -> String {
    let mut out = String::new();
    let mut run = String::new();
    for c in s.chars() {
        if c.is_whitespace() {
            run.push(c);
            continue;
        }
        flush_run(&mut out, &mut run);
        out.push(c);
    }
    flush_run(&mut out, &mut run);
    out
}

fn flush_run(out: &mut String, run: &mut String) {
    if run.chars().count() >= 2 {
        out.push(' ');
    } else {
        out.push_str(run);
    }
    run.clear();
}

// Regex.Replace(s, @"(\d)\s*%", "$1 percent").
fn percent_words(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        out.push(chars[i]);
        if chars[i].is_ascii_digit() {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && chars[j] == '%' {
                out.push_str(" percent");
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

// Regex.Replace(s, @"\bWORD\b", replacement), for a word that starts and
// ends with word characters.
fn replace_word(s: &str, word: &str, replacement: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    let mut previous: Option<char> = None;
    while let Some(at) = rest.find(word) {
        let before = rest[..at].chars().last().or(previous);
        let after = rest[at + word.len()..].chars().next();
        out.push_str(&rest[..at]);
        if before.is_none_or(|c| !is_word(c)) && after.is_none_or(|c| !is_word(c)) {
            out.push_str(replacement);
        } else {
            out.push_str(word);
        }
        previous = word.chars().last();
        rest = &rest[at + word.len()..];
    }
    out.push_str(rest);
    out
}

// A station as the services name it, read as words: "Gander Int'l
// Airport" spelled out, and the NWS's all-capital names ("VISITORS CENTER
// AT FURNACE CREEK DEATH VALLEY") in title case (#21).
pub fn station_name(name: &str) -> String {
    let mut s = name.trim().to_string();
    if s == s.to_uppercase() && s.chars().any(char::is_alphabetic) {
        s = small_words_lowercase(&title_case(&s.to_lowercase()));
    }
    int_l(&s)
}

// TextInfo.ToTitleCase on lowercase text: a letter after anything that
// separates words is capitalized; letters, digits and apostrophes inside a
// word are not ("int'l" stays "Int'l", "4th" becomes "4Th" as .NET has it).
fn title_case(s: &str) -> String {
    let mut out = String::new();
    let mut in_word = false;
    for c in s.chars() {
        if in_word {
            if c.is_alphanumeric() || c == '\'' {
                out.push(c);
                continue;
            }
            in_word = false;
            out.push(c);
        } else if c.is_alphabetic() {
            out.extend(c.to_uppercase());
            in_word = true;
        } else {
            out.push(c);
        }
    }
    out
}

// Regex.Replace(s, @"(?<=\S )(At|Of|The|And|On|In)\b", lowercase): the
// little words, except as the first word.
fn small_words_lowercase(s: &str) -> String {
    const SMALL: [&str; 6] = ["At", "Of", "The", "And", "On", "In"];
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    'scan: while i < chars.len() {
        if i >= 2 && chars[i - 1] == ' ' && !chars[i - 2].is_whitespace() {
            for small in SMALL {
                let n = small.chars().count();
                let matches = chars[i..].iter().take(n).copied().eq(small.chars());
                let ends = chars.get(i + n).is_none_or(|c| !is_word(*c));
                if matches && ends {
                    out.push_str(&small.to_lowercase());
                    i += n;
                    continue 'scan;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

// Regex.Replace(s, @"\bInt'l(?=\s|$)", "International").
fn int_l(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    let mut previous: Option<char> = None;
    while let Some(at) = rest.find("Int'l") {
        let before = rest[..at].chars().last().or(previous);
        let after = rest[at + 5..].chars().next();
        out.push_str(&rest[..at]);
        if before.is_none_or(|c| !is_word(c)) && after.is_none_or(char::is_whitespace) {
            out.push_str("International");
        } else {
            out.push_str("Int'l");
        }
        previous = Some('l');
        rest = &rest[at + 5..];
    }
    out.push_str(rest);
    out
}

// "Flash Flood Watch" as the NWS titles it, or "frost advisory" as
// Environment Canada names it, becomes "Flash flood watch": the sentence
// case this app's own text uses.
pub fn sentence_case(name: &str) -> String {
    let s = name.trim();
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first
            .to_uppercase()
            .chain(chars.as_str().to_lowercase().chars())
            .collect(),
        None => String::new(),
    }
}
