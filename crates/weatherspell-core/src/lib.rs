// Weatherspell's logic, ported from the C# app (src/Weatherspell) piece by
// piece, with no user interface and no system calls: what it needs from
// the system (the unit system, this PC's time zone and time format) is
// passed in. The C# app's tests and captured responses are the
// specification; see tests/reference.rs for how the two are compared.

pub mod alerts;
pub mod clock;
pub mod compass;
pub mod forecast;
pub mod location;
pub mod open_meteo;
pub mod units;
pub mod weather_codes;
pub mod writer;

// C#'s Math.Round without a mode rounds halves to even; with
// MidpointRounding.AwayFromZero it matches Rust's f64::round. Each call
// site says which, as the C# did, so the text stays the same.
pub(crate) fn round_even(value: f64) -> i32 {
    value.round_ties_even() as i32
}

pub(crate) fn round_away(value: f64) -> i32 {
    value.round() as i32
}
