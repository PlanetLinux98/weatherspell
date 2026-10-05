// A place the user can ask about. Name, region and country come from the
// geocoder and are kept verbatim; the nickname is the user's own label.

#[derive(Clone, Debug, PartialEq)]
pub struct Location {
    pub name: String,
    pub region: Option<String>,
    pub country: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub time_zone_id: Option<String>,
    pub nickname: Option<String>,
    pub population: Option<i64>,
}

// Where the National Weather Service writes the forecast and the alerts:
// the states and the territories it has offices for (San Juan, Guam, Pago
// Pago). Open-Meteo's geocoder gives the territories only a country code,
// so the search names them from this list (#19).
pub const NWS_TERRITORIES: [(&str, &str); 5] = [
    ("PR", "Puerto Rico"),
    ("VI", "United States Virgin Islands"),
    ("GU", "Guam"),
    ("MP", "Northern Mariana Islands"),
    ("AS", "American Samoa"),
];

fn present(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|v| !v.trim().is_empty())
}

impl Location {
    pub fn new(
        name: &str,
        region: Option<&str>,
        country: Option<&str>,
        latitude: f64,
        longitude: f64,
    ) -> Location {
        Location {
            name: name.to_string(),
            region: region.map(str::to_string),
            country: country.map(str::to_string),
            latitude,
            longitude,
            time_zone_id: None,
            nickname: None,
            population: None,
        }
    }

    // "Peterborough, Ontario, Canada": what the Location box and headings
    // show when there is no nickname.
    pub fn full_name(&self) -> String {
        let mut parts = vec![self.name.as_str()];
        if let Some(region) = present(&self.region).filter(|r| *r != self.name) {
            parts.push(region);
        }
        if let Some(country) = present(&self.country) {
            parts.push(country);
        }
        parts.join(", ")
    }

    pub fn display_name(&self) -> String {
        match present(&self.nickname) {
            Some(nickname) => nickname.to_string(),
            None => self.full_name(),
        }
    }

    // The same place, to the fourth decimal as its cache file is keyed,
    // whatever it is called: a renamed location is still the one on screen.
    pub fn is_same_place(&self, other: &Location) -> bool {
        fn four(v: f64) -> f64 {
            (v * 10_000.0).round_ties_even() / 10_000.0
        }
        four(self.latitude) == four(other.latitude) && four(self.longitude) == four(other.longitude)
    }

    pub fn is_nws_covered(&self) -> bool {
        match self.country.as_deref() {
            Some("United States") => true,
            Some(country) => NWS_TERRITORIES.iter().any(|(_, name)| *name == country),
            None => false,
        }
    }

    // Result line for typed coordinates: the name is the nearest place's,
    // the point is the one typed, so both are said.
    pub fn near_text(&self) -> String {
        format!(
            "Near {} ({})",
            self.full_name(),
            crate::coordinates::words(self.latitude, self.longitude)
        )
    }

    // A point nothing nearby names (open sea) goes by its coordinates.
    pub fn at_point(latitude: f64, longitude: f64) -> Location {
        Location::new(
            &crate::coordinates::words(latitude, longitude),
            None,
            None,
            latitude,
            longitude,
        )
    }

    // Search-result line: the population tells namesakes apart. Thousands
    // are grouped with commas: the app is in English, whatever the
    // region's own separator (the C# app used the region's).
    pub fn search_result_text(&self) -> String {
        match self.population {
            Some(p) if p > 0 => format!("{} (population {})", self.full_name(), grouped(p)),
            _ => self.full_name(),
        }
    }
}

fn grouped(n: i64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn populations_are_grouped_in_thousands() {
        assert_eq!(grouped(84_000), "84,000");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(1_234_567), "1,234,567");
    }
}
