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
