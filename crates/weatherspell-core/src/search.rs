// What the Add Location dialog searches: the embedded postal code table and
// the Open-Meteo geocoder (place names everywhere, and the postal codes it
// does index), merged so each country has one source for its codes; and
// for typed coordinates, Nominatim's name for the place they fall in.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::fetch::{Fetch, ServiceError};
use crate::location::Location;
use crate::{nominatim, open_meteo, postal_codes};

#[derive(Default)]
pub struct LocationSearch {
    // When Nominatim was last asked: its policy allows one request a
    // second, held to even if Enter is pressed repeatedly.
    last_named: Mutex<Option<Instant>>,
}

impl LocationSearch {
    pub fn new() -> LocationSearch {
        LocationSearch::default()
    }

    pub fn search(&self, http: &dyn Fetch, query: &str) -> Result<Vec<Location>, ServiceError> {
        let table = postal_codes::find(query);
        let geocoder = http
            .get(&open_meteo::search_url(query), None)
            .map_err(ServiceError::Fetch)
            .and_then(|json| {
                open_meteo::parse_search(&json).map_err(|e| ServiceError::Unreadable(e.0))
            });
        match geocoder {
            Ok(found) => Ok(postal_codes::merge(table, found)),
            // The table answered, so a dead network is no reason to show an
            // error: the geocoder could only have added other countries.
            Err(_) if !table.is_empty() => Ok(table),
            Err(e) => Err(e),
        }
    }

    // None when nothing near the point has a name.
    pub fn name_point(
        &self,
        http: &dyn Fetch,
        latitude: f64,
        longitude: f64,
    ) -> Result<Option<Location>, ServiceError> {
        let mut last = self.last_named.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(at) = *last {
            std::thread::sleep(Duration::from_secs(1).saturating_sub(at.elapsed()));
        }
        let answer = http.get(&nominatim::reverse_url(latitude, longitude), None);
        *last = Some(Instant::now());
        nominatim::parse_reverse(&answer.map_err(ServiceError::Fetch)?, latitude, longitude)
            .map_err(|e| ServiceError::Unreadable(e.to_string()))
    }
}
