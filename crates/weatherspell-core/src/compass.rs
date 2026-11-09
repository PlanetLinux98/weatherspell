const POINTS: [&str; 16] = [
    "north",
    "north-northeast",
    "northeast",
    "east-northeast",
    "east",
    "east-southeast",
    "southeast",
    "south-southeast",
    "south",
    "south-southwest",
    "southwest",
    "west-southwest",
    "west",
    "west-northwest",
    "northwest",
    "north-northwest",
];

// Sixteen points is the most anyone hears in a forecast; eight would lose
// "north-northeast", which coastal listeners do use.
pub fn from_degrees(degrees: f64) -> &'static str {
    let normalized = ((degrees % 360.0) + 360.0) % 360.0;
    let index = crate::round_away(normalized / 22.5) % 16;
    POINTS[index as usize]
}

// A direction as the services abbreviate it ("SW", "NNE"), in degrees;
// None for anything else, such as Environment Canada's "VR" (variable).
pub fn from_abbreviation(text: &str) -> Option<f64> {
    const ABBREVIATIONS: [&str; 16] = [
        "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW",
        "NW", "NNW",
    ];
    ABBREVIATIONS
        .iter()
        .position(|a| a.eq_ignore_ascii_case(text.trim()))
        .map(|i| i as f64 * 22.5)
}
