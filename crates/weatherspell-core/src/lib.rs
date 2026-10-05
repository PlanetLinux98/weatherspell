// Weatherspell's logic, ported from the C# app (src/Weatherspell) piece by
// piece, with no user interface and no system calls: what it needs from
// the system (the unit system, this PC's time zone and time format, the
// folder for settings and the cache) is passed in. The C# app's tests and
// captured responses are the specification; see tests/reference.rs for
// how the two are compared.

pub mod alerts;
pub mod cache;
pub mod clock;
pub mod compass;
pub mod coordinates;
pub mod environment_canada;
pub mod fetch;
mod files;
pub mod forecast;
mod iso;
pub mod layout;
pub mod location;
pub mod location_editor;
#[cfg(feature = "net")]
pub mod net;
pub mod nominatim;
pub mod nws;
pub mod official;
pub mod open_meteo;
pub mod placement;
pub mod postal_codes;
pub mod search;
pub mod settings;
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
