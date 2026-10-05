// Shared by the test files. The captured responses are the C# tests' own,
// read in place, so both apps are tested against the same data.

#![allow(dead_code)]

use jiff::tz::Offset;
use weatherspell_core::location::Location;

pub fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/Weatherspell.Tests/Fixtures")
        .join(name)
}

pub fn fixture(name: &str) -> String {
    std::fs::read_to_string(fixture_path(name)).unwrap_or_else(|e| panic!("fixture {name}: {e}"))
}

pub fn offset(hours: i8) -> Offset {
    Offset::constant(hours)
}

pub fn toronto() -> Location {
    let mut l = Location::new("Toronto", Some("Ontario"), Some("Canada"), 43.65, -79.38);
    l.time_zone_id = Some("America/Toronto".to_string());
    l
}
