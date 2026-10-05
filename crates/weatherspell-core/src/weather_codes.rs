// WMO weather interpretation codes as Open-Meteo reports them, turned into
// phrases that sit mid-sentence ("it's 21 degrees and mostly clear").

pub fn describe(code: i32) -> &'static str {
    match code {
        0 => "clear",
        1 => "mostly clear",
        2 => "partly cloudy",
        3 => "overcast",
        45 => "foggy",
        48 => "freezing fog",
        51 => "light drizzle",
        53 => "drizzle",
        55 => "heavy drizzle",
        56 => "light freezing drizzle",
        57 => "freezing drizzle",
        61 => "light rain",
        63 => "rain",
        65 => "heavy rain",
        66 => "light freezing rain",
        67 => "freezing rain",
        71 => "light snow",
        73 => "snow",
        75 => "heavy snow",
        77 => "snow grains",
        80 => "light rain showers",
        81 => "rain showers",
        82 => "heavy rain showers",
        85 => "light snow showers",
        86 => "heavy snow showers",
        95 => "thunderstorms",
        96 => "thunderstorms with hail",
        99 => "thunderstorms with heavy hail",
        _ => "unknown conditions",
    }
}

// Capitalized for the start of a sentence ("Partly cloudy.").
pub fn describe_sentence(code: i32) -> String {
    let s = describe(code);
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

pub fn is_precipitation(code: i32) -> bool {
    code >= 50
}

pub fn is_snow(code: i32) -> bool {
    (71..=77).contains(&code) || code == 85 || code == 86
}

pub fn is_thunder(code: i32) -> bool {
    code >= 95
}

// The phrases that are nouns ("drizzle", "freezing fog", "thunderstorms"),
// which follow "with" rather than "and" in "it's 13 degrees with drizzle".
pub fn is_noun(code: i32) -> bool {
    is_precipitation(code) || code == 48
}

// The same weather at different strengths ("light snow", "snow", "heavy
// snow") shares a kind, so a heavier hour reads "heavier at times". Codes
// of no kind are their own, as the C# gave each its number.
pub fn kind(code: i32) -> i32 {
    match code {
        51 | 53 | 55 => 51,
        56 | 57 => 56,
        61 | 63 | 65 => 61,
        66 | 67 => 66,
        71 | 73 | 75 => 71,
        80..=82 => 80,
        85 | 86 => 85,
        95 | 96 | 99 => 95,
        _ => code,
    }
}

// Higher means more worth mentioning: a thunderstorm in one hour matters
// more than three hours of overcast.
pub fn severity(code: i32) -> i32 {
    match code {
        95.. => 9,
        66 | 67 => 8,
        56 | 57 => 8,
        65 | 82 | 75 | 86 => 7,
        63 | 81 | 73 => 6,
        61 | 80 | 71 | 85 | 77 => 5,
        55 => 4,
        51 | 53 => 3,
        45 | 48 => 2,
        3 => 1,
        _ => 0,
    }
}

// The word for what falls: chance of rain / snow / precipitation.
pub fn precipitation_word(code: i32) -> &'static str {
    if is_snow(code) {
        "snow"
    } else if is_precipitation(code) {
        "rain"
    } else {
        "precipitation"
    }
}
