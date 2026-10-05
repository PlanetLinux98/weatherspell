// Unit choice and the words for each quantity. Sentences say "kilometres
// an hour", never "km/h": words read the same in every screen reader.

use crate::location::Location;
use crate::{round_away, round_even};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitSystem {
    Metric,
    Imperial,
}

// One system for a location's whole text, so this app's numbers never
// disagree with the official sentences beside them. Environment Canada
// writes metric only, so Canada is metric whatever the region; the NWS
// writes either (units=si), so the US follows the region like everywhere
// else. There is no units setting (see NOTES.md). The region's own system
// comes from the app, which asks the system.
pub fn for_location(location: &Location, region_default: UnitSystem) -> UnitSystem {
    if location.country.as_deref() == Some("Canada") {
        UnitSystem::Metric
    } else {
        region_default
    }
}

// Open-Meteo request parameters, so values arrive already converted.
pub fn temperature_parameter(u: UnitSystem) -> &'static str {
    if u == UnitSystem::Metric {
        "celsius"
    } else {
        "fahrenheit"
    }
}

pub fn wind_parameter(u: UnitSystem) -> &'static str {
    if u == UnitSystem::Metric {
        "kmh"
    } else {
        "mph"
    }
}

pub fn precipitation_parameter(u: UnitSystem) -> &'static str {
    if u == UnitSystem::Metric {
        "mm"
    } else {
        "inch"
    }
}

// Observed values arrive in SI whatever the user chose; Open-Meteo's are
// requested in the user's units, so only official sources convert.
pub fn from_celsius(celsius: f64, u: UnitSystem) -> f64 {
    if u == UnitSystem::Metric {
        celsius
    } else {
        celsius * 9.0 / 5.0 + 32.0
    }
}

pub fn from_kmh(kmh: f64, u: UnitSystem) -> f64 {
    if u == UnitSystem::Metric {
        kmh
    } else {
        kmh / 1.609344
    }
}

pub fn degrees(value: f64) -> String {
    let rounded = round_away(value);
    // "minus 5 degrees" rather than a hyphen the synth may skip.
    let unit = if rounded.abs() == 1 {
        "degree"
    } else {
        "degrees"
    };
    if rounded < 0 {
        format!("minus {} {unit}", -rounded)
    } else {
        format!("{rounded} {unit}")
    }
}

// Snowfall, always metric's centimetres (imperial snowfall is inches,
// written by the caller).
pub fn centimetres(value: f64) -> String {
    let rounded = round_away(value);
    if rounded == 1 {
        "1 centimetre".to_string()
    } else {
        format!("{rounded} centimetres")
    }
}

// Bare number for "high 26, low 15": the unit word was said once already.
pub fn degrees_bare(value: f64) -> String {
    let rounded = round_away(value);
    if rounded < 0 {
        format!("minus {}", -rounded)
    } else {
        rounded.to_string()
    }
}

pub fn speed(value: f64, u: UnitSystem) -> String {
    let rounded = round_away(value);
    match (u, rounded) {
        (UnitSystem::Metric, 1) => "1 kilometre an hour".to_string(),
        (UnitSystem::Metric, _) => format!("{rounded} kilometres an hour"),
        (UnitSystem::Imperial, 1) => "1 mile an hour".to_string(),
        (UnitSystem::Imperial, _) => format!("{rounded} miles an hour"),
    }
}

pub fn speed_bare(value: f64) -> String {
    round_away(value).to_string()
}

pub fn distance(metres: f64, u: UnitSystem) -> String {
    if u == UnitSystem::Metric {
        if metres < 1000.0 {
            return format!("{} metres", round_even(metres));
        }
        let km = round_even(metres / 1000.0);
        return if km == 1 {
            "1 kilometre".to_string()
        } else {
            format!("{km} kilometres")
        };
    }
    if metres < 1609.344 {
        return "under a mile".to_string();
    }
    let miles = round_even(metres / 1609.344);
    if miles == 1 {
        "1 mile".to_string()
    } else {
        format!("{miles} miles")
    }
}

pub fn pressure(hpa: f64, u: UnitSystem) -> String {
    if u == UnitSystem::Metric {
        format!("{} hectopascals", round_even(hpa))
    } else {
        format!("{} inches of mercury", fixed(hpa * 0.0295299830714, 2))
    }
}

pub fn precipitation_amount(amount: f64, u: UnitSystem) -> String {
    if u == UnitSystem::Metric {
        let mm = amount.round_ties_even();
        if mm < 1.0 {
            return "less than a millimetre".to_string();
        }
        return if mm == 1.0 {
            "1 millimetre".to_string()
        } else {
            format!("{} millimetres", mm as i32)
        };
    }
    let inches = (amount * 10.0).round_ties_even() / 10.0;
    if inches < 0.1 {
        return "less than a tenth of an inch".to_string();
    }
    format!(
        "{} {}",
        one_decimal(inches),
        if inches == 1.0 { "inch" } else { "inches" }
    )
}

// .NET's "0.00": exactly two decimals, halves away from zero.
pub(crate) fn fixed(value: f64, decimals: i32) -> String {
    let scale = 10f64.powi(decimals);
    format!("{:.*}", decimals as usize, (value * scale).round() / scale)
}

// .NET's "0.#": at most one decimal, none when it would be zero.
pub(crate) fn one_decimal(value: f64) -> String {
    let text = fixed(value, 1);
    match text.strip_suffix(".0") {
        Some(whole) => whole.to_string(),
        None => text,
    }
}
