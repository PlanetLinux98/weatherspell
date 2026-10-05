// Names for coordinates typed into Add Location, from OpenStreetMap's
// Nominatim (no key; data ODbL, credited in About). Open-Meteo's geocoder
// only goes from names to coordinates. The usage policy allows lookups the
// user asks for, with a User-Agent naming the app and at most one request
// a second, which fetch::name_point holds to even if Enter is pressed
// repeatedly.

use serde::Deserialize;

use crate::coordinates;
use crate::location::{Location, NWS_TERRITORIES};
use crate::official::OfficialError;
use crate::units::trimmed;

pub const SOURCE_NOTE: &str =
    "OpenStreetMap (openstreetmap.org), \u{00A9} OpenStreetMap contributors, licensed ODbL";

const ENDPOINT: &str = "https://nominatim.openstreetmap.org/reverse";

// Zoom 14 finds the neighbourhood; its address still names the settlement,
// which is what the name is taken from.
pub fn reverse_url(latitude: f64, longitude: f64) -> String {
    format!(
        "{ENDPOINT}?format=jsonv2&addressdetails=1&zoom=14&accept-language=en&lat={}&lon={}",
        trimmed(latitude, 6),
        trimmed(longitude, 6)
    )
}

// reverse?format=jsonv2&addressdetails=1: the matched object and the
// address of where it is; {"error": "..."} when nothing is there.
#[derive(Deserialize)]
struct ReverseResponse {
    error: Option<serde_json::Value>,
    name: Option<String>,
    address: Option<Address>,
}

#[derive(Deserialize)]
struct Address {
    hamlet: Option<String>,
    village: Option<String>,
    town: Option<String>,
    city: Option<String>,
    municipality: Option<String>,
    county: Option<String>,
    state_district: Option<String>,
    state: Option<String>,
    #[serde(rename = "ISO3166-2-lvl4")]
    iso3166_level4: Option<String>,
    country: Option<String>,
}

// The place keeps the coordinates asked about, not those of the object
// Nominatim matched: a cottage is not the town centre. None when nothing
// there has an address at all ("Unable to geocode": open sea).
pub fn parse_reverse(
    json: &str,
    latitude: f64,
    longitude: f64,
) -> Result<Option<Location>, OfficialError> {
    let response: ReverseResponse = serde_json::from_str(json)?;
    if response.error.is_some() {
        return Ok(None);
    }
    let Some(a) = response.address else {
        return Ok(None);
    };

    // The smallest settlement first: a village inside a US town, a town
    // inside an amalgamated city ("Bobcaygeon", not "Kawartha Lakes"). Then
    // whatever area the point is in, for the countryside and water.
    let name = first(&[
        &a.hamlet,
        &a.village,
        &a.town,
        &a.city,
        &a.municipality,
        &a.county,
        &a.state_district,
        &response.name,
    ])
    .unwrap_or_else(|| coordinates::words(latitude, longitude));
    let mut region = first(&[&a.state]);
    let mut country = first(&[&a.country]);

    // Nominatim puts the US territories in the United States; the geocoder,
    // and so the rest of the app, calls them by name (#19).
    if let Some(code) = a
        .iso3166_level4
        .as_deref()
        .and_then(|c| c.strip_prefix("US-"))
        && let Some((_, territory)) = NWS_TERRITORIES.iter().find(|(c, _)| *c == code)
    {
        country = Some(territory.to_string());
        region = None;
    }
    Ok(Some(Location {
        name,
        region,
        country,
        latitude,
        longitude,
        time_zone_id: None,
        nickname: None,
        population: None,
    }))
}

fn first(values: &[&Option<String>]) -> Option<String> {
    values
        .iter()
        .filter_map(|v| v.as_deref().map(str::trim))
        .find(|v| !v.is_empty())
        .map(str::to_string)
}
