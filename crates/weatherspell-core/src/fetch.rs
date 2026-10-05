// The asking: one forecast for a location, the Open-Meteo base with a
// national weather service's text and observation laid over it where one
// exists (Canada and the United States). Coverage is decided by the
// geocoded country and then by the service accepting the point; when the
// official fetch fails the location gets generated sentences for that
// refresh and the Sources line says why. Open-Meteo failing still fails
// the refresh.
//
// The requests go through Fetch, so tests answer them from captured
// responses; net::UreqFetch is the real one. Everything blocks: the app
// runs a refresh on a thread of its own, and the independent requests
// here run side by side on scoped threads, as 0.1 awaited them together.

use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;
use std::time::Duration;

use jiff::Timestamp;
use jiff::tz::Offset;

use crate::alerts::{self, AlertReport};
use crate::environment_canada::{self as ec, Site};
use crate::forecast::{Forecast, OfficialPeriod};
use crate::location::Location;
use crate::official::{self, OfficialError, OfficialForecast};
use crate::units::UnitSystem;
use crate::{nws, open_meteo};

pub trait Fetch: Sync {
    // The body of a successful answer. The timeout, when given, is shorter
    // than the client's own.
    fn get(&self, url: &str, timeout: Option<Duration>) -> Result<String, FetchError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FetchError {
    // "404 Not Found from dd.weather.gc.ca", as 0.1 worded it.
    Status {
        code: u16,
        reason: String,
        host: String,
    },
    TimedOut {
        host: String,
    },
    // The developer's WEATHERSPELL_OFFLINE: every request fails at once.
    Offline {
        host: String,
    },
    // No answer at all: no network, a name that did not resolve, a refused
    // or broken connection.
    Unreachable {
        host: String,
        detail: String,
    },
}

impl FetchError {
    pub fn host(&self) -> &str {
        match self {
            FetchError::Status { host, .. }
            | FetchError::TimedOut { host }
            | FetchError::Offline { host }
            | FetchError::Unreachable { host, .. } => host,
        }
    }
}

// The words a reader hears in brackets after "could not be fetched this
// time". 0.1 gave .NET's own message for a failed connection ("Unable to
// connect to the remote server"); these name the host instead.
impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::Status { code, reason, host } if reason.is_empty() => {
                write!(f, "{code} from {host}")
            }
            FetchError::Status { code, reason, host } => write!(f, "{code} {reason} from {host}"),
            FetchError::TimedOut { .. } => f.write_str("it took too long to answer"),
            FetchError::Offline { host } => {
                write!(f, "{host} was not contacted (WEATHERSPELL_OFFLINE is set)")
            }
            FetchError::Unreachable { host, .. } => write!(f, "{host} could not be reached"),
        }
    }
}

impl std::error::Error for FetchError {}

// Why a forecast or a search could not be had at all: the service did not
// answer, or answered with something that could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServiceError {
    Fetch(FetchError),
    Unreadable(String),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceError::Fetch(e) => e.fmt(f),
            ServiceError::Unreadable(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for ServiceError {}

// An official service's failure: a request that failed, or an answer that
// could not be used.
#[derive(Debug)]
enum Problem {
    Fetch(FetchError),
    Official(OfficialError),
}

impl From<FetchError> for Problem {
    fn from(e: FetchError) -> Problem {
        Problem::Fetch(e)
    }
}

impl From<OfficialError> for Problem {
    fn from(e: OfficialError) -> Problem {
        Problem::Official(e)
    }
}

// The NWS grid and station for a point: the points lookup never changes,
// and the NWS asks that it not be repeated needlessly.
#[derive(Clone, Debug)]
struct Grid {
    forecast_url: String,
    attribution: String,
    station_id: Option<String>,
    station_name: Option<String>,
}

// The weather.gov page's data gets a shorter wait than the client's own,
// so a hung request costs seconds rather than the whole timeout before the
// API's text stands in.
pub const PAGE_TIMEOUT: Duration = Duration::from_secs(10);

// Holds what is fetched once per session: the NWS grids and Environment
// Canada's site list.
#[derive(Default)]
pub struct ForecastService {
    grids: Mutex<HashMap<String, Grid>>,
    sites: Mutex<Option<Vec<Site>>>,
}

impl ForecastService {
    pub fn new() -> ForecastService {
        ForecastService::default()
    }

    pub fn forecast(
        &self,
        http: &dyn Fetch,
        location: &Location,
        units: UnitSystem,
        days: u32,
        now: Timestamp,
    ) -> Result<Forecast, ServiceError> {
        let (base, official) = std::thread::scope(|s| {
            let official = s.spawn(|| self.official(http, location, units, now));
            let base = http
                .get(&open_meteo::forecast_url(location, units, days), None)
                .map_err(ServiceError::Fetch)
                .and_then(|json| {
                    open_meteo::parse(&json, location, units, now)
                        .map_err(|e| ServiceError::Unreadable(e.0))
                });
            (
                base,
                official.join().expect("the official fetch does not panic"),
            )
        });
        let base = base?;
        Ok(match official {
            Ok(Some(o)) => official::compose(&base, &o),
            Ok(None) => base,
            Err(problem) => official::with_problem(base, Some(problem)),
        })
    }

    // The official forecast, None where no service covers the place, or
    // the Sources line's explanation of why it is missing.
    fn official(
        &self,
        http: &dyn Fetch,
        location: &Location,
        units: UnitSystem,
        now: Timestamp,
    ) -> Result<Option<OfficialForecast>, String> {
        let (name, result) = if location.country.as_deref() == Some("Canada") {
            (
                ec::SOURCE_NAME,
                self.environment_canada(http, location, now),
            )
        } else if location.is_nws_covered() {
            (nws::SOURCE_NAME, self.nws(http, location, units))
        } else {
            return Ok(None);
        };
        result.map(Some).map_err(|problem| match problem {
            Problem::Official(OfficialError::NoForecastText(_)) => official::no_text_problem(name),
            Problem::Official(OfficialError::Unreadable(why)) => {
                official::fetch_problem(name, &why)
            }
            Problem::Fetch(e) => official::fetch_problem(name, &e.to_string()),
        })
    }

    fn nws(
        &self,
        http: &dyn Fetch,
        location: &Location,
        units: UnitSystem,
    ) -> Result<OfficialForecast, Problem> {
        let grid = self.grid(http, location)?;
        let (periods, observation) = std::thread::scope(|s| {
            let observation = s.spawn(|| {
                // A station that is down should not cost the forecast text:
                // the base's current conditions stand in, and the Sources
                // line says so.
                let id = grid.station_id.as_deref()?;
                http.get(&nws::observation_url(id), None).ok()
            });
            let periods = nws_periods(
                http,
                &nws::page_url(location, units),
                &nws::forecast_url(&grid.forecast_url, units),
            );
            (
                periods,
                observation
                    .join()
                    .expect("the observation fetch does not panic"),
            )
        });
        let observation = match observation {
            Some(json) => Some(nws::parse_observation(&json, grid.station_name.as_deref())?),
            None => None,
        };
        Ok(OfficialForecast {
            source_name: nws::SOURCE_NAME.to_string(),
            attribution: grid.attribution,
            periods: periods?,
            observation,
        })
    }

    fn grid(&self, http: &dyn Fetch, location: &Location) -> Result<Grid, Problem> {
        let key = nws::point_key(location);
        let mut grids = self.grids.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(grid) = grids.get(&key) {
            return Ok(grid.clone());
        }
        let points = nws::parse_points(&http.get(&nws::points_url(location), None)?)?;
        let station = nws::parse_stations(&http.get(&points.stations_url, None)?)?;
        let (station_id, station_name) =
            station.map_or((None, None), |(id, name)| (Some(id), name));
        let grid = Grid {
            forecast_url: points.forecast_url,
            attribution: points.attribution,
            station_id,
            station_name,
        };
        grids.insert(key, grid.clone());
        Ok(grid)
    }

    fn environment_canada(
        &self,
        http: &dyn Fetch,
        location: &Location,
        now: Timestamp,
    ) -> Result<OfficialForecast, Problem> {
        let site = {
            let mut sites = self.sites.lock().unwrap_or_else(|e| e.into_inner());
            if sites.is_none() {
                *sites = Some(ec::parse_site_list(&http.get(ec::SITE_LIST_URL, None)?)?);
            }
            let list = sites.as_deref().unwrap_or_default();
            let (site, distance) = ec::nearest(list, location.latitude, location.longitude)
                .ok_or_else(|| OfficialError::Unreadable("No Environment Canada sites.".into()))?;
            if distance > ec::MAX_SITE_DISTANCE_KM {
                return Err(OfficialError::Unreadable(format!(
                    "no Environment Canada forecast site within {} kilometres",
                    ec::MAX_SITE_DISTANCE_KM
                ))
                .into());
            }
            site.clone()
        };
        let xml = latest_city_page(http, &site, now)?;
        Ok(ec::parse(&xml)?)
    }
}

// The text the weather.gov forecast page shows, and the API's own when the
// page's data cannot be had or read (NOTES.md).
fn nws_periods(
    http: &dyn Fetch,
    page_url: &str,
    api_url: &str,
) -> Result<Vec<OfficialPeriod>, Problem> {
    let page = http.get(page_url, Some(PAGE_TIMEOUT));
    if let Ok(periods) = page
        .map_err(Problem::from)
        .and_then(|json| Ok(nws::parse_page_periods(&json)?))
    {
        return Ok(periods);
    }
    Ok(nws::parse_periods(&http.get(api_url, None)?)?)
}

// Files land in an hour's folder by UTC hour of emission and only today's
// tree is kept, so the latest is found by listing the current hour and
// walking back; an hour's folder that is not there yet is skipped.
fn latest_city_page(http: &dyn Fetch, site: &Site, now: Timestamp) -> Result<String, Problem> {
    let hour = Offset::UTC.to_datetime(now).hour();
    for h in (0..=hour).rev() {
        let folder = ec::hour_folder(&site.province, h);
        let listing = match http.get(&folder, None) {
            Ok(listing) => listing,
            Err(FetchError::TimedOut { host }) => return Err(FetchError::TimedOut { host }.into()),
            Err(_) => continue,
        };
        if let Some(file) = ec::latest_file(&listing, &site.code) {
            return Ok(http.get(&format!("{folder}{file}"), None)?);
        }
    }
    Err(OfficialError::Unreadable(format!(
        "no Environment Canada page for {} published yet today",
        site.name
    ))
    .into())
}

// Alerts in effect for a location, from the service that covers its
// country, most severe first. A location outside every covered region
// gets a report that is not available, which the text states outright; a
// source that cannot be reached gets a report with the problem named,
// never an empty list that would read as a quiet day.
pub fn check_alerts(http: &dyn Fetch, location: &Location, now: Timestamp) -> AlertReport {
    let (attribution, result) = if location.country.as_deref() == Some("Canada") {
        let alerts = http
            .get(&ec::alerts_url(location), None)
            .map_err(Problem::from)
            .and_then(|json| {
                Ok(ec::parse_alerts(
                    &json,
                    now,
                    Some(&ec::location_url(location)),
                )?)
            });
        (ec::ALERTS_ATTRIBUTION, alerts)
    } else if location.is_nws_covered() {
        let alerts = http
            .get(&nws::alerts_url(location), None)
            .map_err(Problem::from)
            .and_then(|json| Ok(nws::parse_alerts(&json)?));
        (nws::ALERTS_ATTRIBUTION, alerts)
    } else {
        return AlertReport::not_available();
    };
    match result {
        Ok(alerts) => AlertReport {
            alerts: alerts::order(alerts),
            attribution: Some(attribution.to_string()),
            problem: None,
            checked_at: Some(now),
        },
        Err(problem) => AlertReport {
            alerts: Vec::new(),
            attribution: Some(attribution.to_string()),
            problem: Some(match problem {
                Problem::Fetch(FetchError::TimedOut { .. }) => {
                    "the alert service took too long to answer".to_string()
                }
                Problem::Fetch(e) => e.to_string(),
                Problem::Official(e) => e.to_string().trim_end_matches(['.', ' ']).to_string(),
            }),
            checked_at: None,
        },
    }
}
