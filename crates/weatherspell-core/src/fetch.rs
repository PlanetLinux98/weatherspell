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
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use jiff::Timestamp;
use jiff::tz::Offset;

use crate::alerts::{self, AlertReport, WeatherAlert};
use crate::area::Region;
use crate::cap::{self, CapAlert};
use crate::environment_canada::{self as ec, Site};
use crate::forecast::{Forecast, OfficialPeriod};
use crate::location::Location;
use crate::official::{self, OfficialError, OfficialForecast};
use crate::units::UnitSystem;
use crate::{bom, files, hko, meteoalarm, metservice, nws, open_meteo, pagasa, saws};

pub trait Fetch: Sync {
    // The body of a successful answer. The timeout, when given, replaces
    // the client's own.
    fn get(&self, url: &str, timeout: Option<Duration>) -> Result<String, FetchError>;

    // The body as it came, for a file that is not text (a zip archive).
    fn get_bytes(&self, url: &str, timeout: Option<Duration>) -> Result<Vec<u8>, FetchError> {
        self.get(url, timeout).map(String::into_bytes)
    }
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

// An alert feed's answer, kept briefly: a MeteoAlarm feed holds a whole
// country, so one fetch serves every saved location there in a round of
// checks. A failure is kept too, so an unreachable service costs one wait
// a round, not one per location.
type Recent<T> = Option<(Timestamp, Result<Arc<T>, String>)>;
const FEED_REUSE: jiff::SignedDuration = jiff::SignedDuration::from_secs(60);

// Holds what is fetched once per session: the NWS grids, Environment
// Canada's site list, MetService's, SAWS's and PAGASA's CAP files (they
// never change once issued) and the Bureau of Meteorology's outlines; the
// Bureau's CAP files until its listing shows them changed; and the alert
// feeds, briefly.
// Given a cache folder, the service keeps the Bureau's outlines there
// between sessions, so they are downloaded once rather than every launch.
#[derive(Default)]
pub struct ForecastService {
    grids: Mutex<HashMap<String, Grid>>,
    sites: Mutex<Option<Vec<Site>>>,
    europe: Mutex<HashMap<String, Recent<Vec<CapAlert>>>>,
    new_zealand: Mutex<Recent<Vec<String>>>,
    cap_files: Mutex<HashMap<String, Arc<CapAlert>>>,
    hong_kong: Mutex<Recent<Vec<WeatherAlert>>>,
    south_africa: Mutex<Recent<Vec<saws::Listed>>>,
    south_africa_files: Mutex<HashMap<String, Arc<CapAlert>>>,
    philippines: Mutex<Recent<Vec<String>>>,
    philippines_files: Mutex<HashMap<String, Arc<CapAlert>>>,
    folder: Option<PathBuf>,
    australia: Mutex<Recent<bom::Areas>>,
    australia_listing: Mutex<Recent<Vec<bom::Listed>>>,
    australia_files: Mutex<HashMap<String, (String, Arc<CapAlert>)>>,
}

impl ForecastService {
    pub fn new() -> ForecastService {
        ForecastService::default()
    }

    pub fn with_cache_folder(folder: PathBuf) -> ForecastService {
        ForecastService {
            folder: Some(folder),
            ..ForecastService::default()
        }
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

// Alerts in effect for a location, with a service of its own, for a
// one-off check (the example, tests); the app keeps one ForecastService.
pub fn check_alerts(http: &dyn Fetch, location: &Location, now: Timestamp) -> AlertReport {
    ForecastService::new().alerts(http, location, now)
}

fn problem_text(problem: Problem) -> String {
    match problem {
        Problem::Fetch(FetchError::TimedOut { .. }) => {
            "the alert service took too long to answer".to_string()
        }
        Problem::Fetch(e) => e.to_string(),
        Problem::Official(e) => e.to_string().trim_end_matches(['.', ' ']).to_string(),
    }
}

impl ForecastService {
    // Alerts in effect for a location, from the service that covers its
    // country, most severe first. A location outside every covered region
    // gets a report that is not available, which the text states outright;
    // a source that cannot be reached gets a report with the problem named,
    // never an empty list that would read as a quiet day.
    pub fn alerts(&self, http: &dyn Fetch, location: &Location, now: Timestamp) -> AlertReport {
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
                })
                .map_err(problem_text);
            (ec::ALERTS_ATTRIBUTION.to_string(), alerts)
        } else if location.is_nws_covered() {
            let alerts = http
                .get(&nws::alerts_url(location), None)
                .map_err(Problem::from)
                .and_then(|json| Ok(nws::parse_alerts(&json)?))
                .map_err(problem_text);
            (nws::ALERTS_ATTRIBUTION.to_string(), alerts)
        } else if let Some(country) = meteoalarm::country_of(location) {
            let alerts = self
                .europe_feed(http, country, now)
                .map(|feed| meteoalarm::alerts_for(&feed, country, location, now));
            let shown = alerts.as_deref().unwrap_or_default();
            (meteoalarm::attribution(country, shown), alerts)
        } else if metservice::covers(location) {
            (
                metservice::ALERTS_ATTRIBUTION.to_string(),
                self.new_zealand(http, location, now),
            )
        } else if bom::covers(location) {
            (
                bom::ALERTS_ATTRIBUTION.to_string(),
                self.australia(http, location, now),
            )
        } else if hko::covers(location) {
            (
                hko::ALERTS_ATTRIBUTION.to_string(),
                self.hong_kong(http, now).map(|a| a.to_vec()),
            )
        } else if saws::covers(location) {
            (
                saws::ALERTS_ATTRIBUTION.to_string(),
                self.south_africa(http, location, now),
            )
        } else if pagasa::covers(location) {
            (
                pagasa::ALERTS_ATTRIBUTION.to_string(),
                self.philippines(http, location, now),
            )
        } else {
            return AlertReport::not_available();
        };
        match result {
            Ok(alerts) => AlertReport {
                alerts: alerts::order(alerts),
                attribution: Some(attribution),
                problem: None,
                checked_at: Some(now),
            },
            Err(problem) => AlertReport {
                alerts: Vec::new(),
                attribution: Some(attribution),
                problem: Some(problem),
                checked_at: None,
            },
        }
    }

    fn europe_feed(
        &self,
        http: &dyn Fetch,
        country: &meteoalarm::Country,
        now: Timestamp,
    ) -> Result<Arc<Vec<CapAlert>>, String> {
        let url = meteoalarm::feed_url(country);
        let mut feeds = self.europe.lock().unwrap_or_else(|e| e.into_inner());
        recent(feeds.entry(url.clone()).or_default(), now, || {
            http.get(&url, Some(meteoalarm::FEED_TIMEOUT))
                .map_err(Problem::from)
                .and_then(|json| Ok(meteoalarm::parse_feed(&json)?))
        })
    }

    // The feed lists the CAP files in effect; each is fetched the first
    // time it is listed, and dropped once it no longer is.
    fn new_zealand(
        &self,
        http: &dyn Fetch,
        location: &Location,
        now: Timestamp,
    ) -> Result<Vec<WeatherAlert>, String> {
        let links = {
            let mut list = self.new_zealand.lock().unwrap_or_else(|e| e.into_inner());
            recent(&mut list, now, || {
                http.get(metservice::FEED_URL, None)
                    .map_err(Problem::from)
                    .and_then(|xml| Ok(metservice::parse_feed(&xml)?))
            })?
        };
        Ok(cap_files(http, &self.cap_files, &links)?
            .iter()
            .filter_map(|alert| metservice::alert_for(alert, location, now))
            .collect())
    }

    // Territory-wide, so one answer serves every Hong Kong location in a
    // round of checks.
    fn hong_kong(
        &self,
        http: &dyn Fetch,
        now: Timestamp,
    ) -> Result<Arc<Vec<WeatherAlert>>, String> {
        let mut slot = self.hong_kong.lock().unwrap_or_else(|e| e.into_inner());
        recent(&mut slot, now, || {
            let summary = http.get(hko::SUMMARY_URL, None)?;
            let details = http.get(hko::DETAILS_URL, None)?;
            Ok(hko::parse(&summary, &details)?)
        })
    }

    fn south_africa(
        &self,
        http: &dyn Fetch,
        location: &Location,
        now: Timestamp,
    ) -> Result<Vec<WeatherAlert>, String> {
        let listed = {
            let mut slot = self.south_africa.lock().unwrap_or_else(|e| e.into_inner());
            recent(&mut slot, now, || {
                Ok(saws::parse_feed(&http.get(saws::FEED_URL, None)?, now)?)
            })?
        };
        let links: Vec<String> = listed.iter().map(|l| l.url.clone()).collect();
        let files = cap_files(http, &self.south_africa_files, &links)?;
        Ok(listed
            .iter()
            .zip(files)
            .flat_map(|(listed, alert)| saws::alerts_for(&alert, listed, location, now))
            .collect())
    }

    fn philippines(
        &self,
        http: &dyn Fetch,
        location: &Location,
        now: Timestamp,
    ) -> Result<Vec<WeatherAlert>, String> {
        let links = {
            let mut slot = self.philippines.lock().unwrap_or_else(|e| e.into_inner());
            recent(&mut slot, now, || {
                Ok(pagasa::parse_feed(&http.get(pagasa::FEED_URL, None)?)?)
            })?
        };
        let files = cap_files(http, &self.philippines_files, &links)?;
        Ok(pagasa::alerts_for(&files, location, now))
    }
}

// The CAP files a feed lists, in its order: those fetched before from the
// cache (a CAP file never changes once issued), the rest side by side;
// any no longer listed are dropped.
fn cap_files(
    http: &dyn Fetch,
    cache: &Mutex<HashMap<String, Arc<CapAlert>>>,
    links: &[String],
) -> Result<Vec<Arc<CapAlert>>, String> {
    let new: Vec<String> = {
        let mut files = cache.lock().unwrap_or_else(|e| e.into_inner());
        files.retain(|url, _| links.contains(url));
        links
            .iter()
            .filter(|l| !files.contains_key(*l))
            .cloned()
            .collect()
    };
    let answers = side_by_side(http, &new);
    let mut files = cache.lock().unwrap_or_else(|e| e.into_inner());
    for (url, answer) in new.into_iter().zip(answers) {
        let alert = answer
            .map_err(Problem::from)
            .and_then(|xml| Ok(cap::parse_xml(&xml)?))
            .map_err(problem_text)?;
        files.insert(url, Arc::new(alert));
    }
    Ok(links.iter().filter_map(|l| files.get(l).cloned()).collect())
}

impl ForecastService {
    // The listing of the Bureau's warning files is shared by a round of
    // checks, like a feed; a file is fetched again only when its line in
    // the listing changes, and only from the offices whose areas hold the
    // location. New files are fetched a few at a time: each costs the
    // FTP's round trips, which from overseas add up (Queensland can list
    // a dozen).
    fn australia(
        &self,
        http: &dyn Fetch,
        location: &Location,
        now: Timestamp,
    ) -> Result<Vec<WeatherAlert>, String> {
        let areas = self.australian_areas(http, now)?;
        let codes = areas.codes_at(location.latitude, location.longitude);
        if codes.is_empty() {
            return Ok(Vec::new());
        }
        let listing = {
            let mut slot = self
                .australia_listing
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            recent(&mut slot, now, || {
                Ok(bom::parse_listing(&http.get(bom::WARNINGS_URL, None)?))
            })?
        };
        let wanted: Vec<&bom::Listed> = listing
            .iter()
            .filter(|l| bom::concerns(&l.name, &codes))
            .collect();
        let new: Vec<&bom::Listed> = {
            let mut files = self
                .australia_files
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            files.retain(|name, _| listing.iter().any(|l| l.name == *name));
            wanted
                .iter()
                .copied()
                .filter(|l| files.get(&l.name).is_none_or(|(line, _)| *line != l.line))
                .collect()
        };
        let urls: Vec<String> = new.iter().map(|l| bom::warning_url(&l.name)).collect();
        let answers = side_by_side(http, &urls);
        let mut files = self
            .australia_files
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        for (listed, answer) in new.into_iter().zip(answers) {
            let alert = answer
                .map_err(Problem::from)
                .and_then(|xml| Ok(cap::parse_xml(&xml)?))
                .map_err(problem_text)?;
            files.insert(listed.name.clone(), (listed.line.clone(), Arc::new(alert)));
        }
        Ok(bom::alerts_for(
            wanted
                .iter()
                .filter_map(|l| files.get(&l.name))
                .map(|(_, alert)| alert.as_ref()),
            &codes,
            now,
        ))
    }

    // Once had, the outlines serve the whole session; a failure is kept
    // as briefly as a feed's, so the next round tries again.
    fn australian_areas(
        &self,
        http: &dyn Fetch,
        now: Timestamp,
    ) -> Result<Arc<bom::Areas>, String> {
        let mut slot = self.australia.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((at, answer)) = slot.as_ref()
            && (answer.is_ok() || now.duration_since(*at).abs() < FEED_REUSE)
        {
            return answer.clone();
        }
        let answer = self.load_areas(http).map(Arc::new).map_err(problem_text);
        *slot = Some((now, answer.clone()));
        answer
    }

    // The layers kept in the cache folder, each downloaded afresh where it
    // is missing or the Bureau's listing shows it changed, side by side.
    // Without the listing, what is kept serves; a layer that cannot be had
    // fails the check rather than leave its warnings unmatched.
    fn load_areas(&self, http: &dyn Fetch) -> Result<bom::Areas, Problem> {
        let folder = self.folder.as_ref().map(|f| f.join(bom::FOLDER_NAME));
        let path = |layer: &bom::Layer| {
            folder
                .as_ref()
                .map(|f| f.join(format!("{}.txt", layer.product)))
        };
        let mut kept: Vec<Option<(String, Vec<Region>)>> = bom::LAYERS
            .iter()
            .map(|layer| bom::parse_layer_text(&files::read_text(&path(layer)?).ok()?))
            .collect();
        let listing = match http.get(bom::LAYERS_URL, None) {
            Ok(text) => Some(bom::parse_listing(&text)),
            Err(_) if kept.iter().all(Option::is_some) => None,
            Err(e) => return Err(e.into()),
        };
        let wanted: Vec<(usize, String)> = bom::LAYERS
            .iter()
            .enumerate()
            .filter_map(|(i, layer)| {
                let name = format!("{}.zip", layer.product);
                let version = listing
                    .iter()
                    .flatten()
                    .find(|f| f.name == name)
                    .map(|f| bom::layer_version(&f.line))
                    .unwrap_or_default();
                let stale = match &kept[i] {
                    None => true,
                    Some((had, _)) => !version.is_empty() && *had != version,
                };
                stale.then_some((i, version))
            })
            .collect();
        let fetched: Vec<(usize, String, Result<Vec<Region>, Problem>)> = std::thread::scope(|s| {
            let downloads: Vec<_> = wanted
                .into_iter()
                .map(|(i, version)| {
                    s.spawn(move || {
                        let layer = &bom::LAYERS[i];
                        let regions = http
                            .get_bytes(&bom::layer_url(layer), Some(bom::LAYER_TIMEOUT))
                            .map_err(Problem::from)
                            .and_then(|zip| Ok(bom::read_layer(&zip, layer)?));
                        (i, version, regions)
                    })
                })
                .collect();
            downloads
                .into_iter()
                .map(|d| d.join().expect("an outline download does not panic"))
                .collect()
        });
        for (i, version, regions) in fetched {
            let layer = &bom::LAYERS[i];
            match regions {
                Ok(regions) => {
                    // A folder that cannot be written costs a download next
                    // session, nothing more.
                    if let Some(path) = path(layer) {
                        let _ = files::replace(
                            &path,
                            bom::layer_text(layer, &version, &regions).as_bytes(),
                        );
                    }
                    kept[i] = Some((version, regions));
                }
                Err(_) if kept[i].is_some() => {}
                Err(problem) => return Err(problem),
            }
        }
        Ok(bom::Areas::new(
            bom::LAYERS
                .iter()
                .zip(kept)
                .filter_map(|(layer, kept)| {
                    let (version, regions) = kept?;
                    Some(bom::KeptLayer::new(layer, &version, regions))
                })
                .collect(),
        ))
    }
}

// Up to four requests at once, the answers in the order asked.
fn side_by_side(http: &dyn Fetch, urls: &[String]) -> Vec<Result<String, FetchError>> {
    let next = AtomicUsize::new(0);
    let answers = Mutex::new(vec![None; urls.len()]);
    std::thread::scope(|s| {
        for _ in 0..urls.len().min(4) {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(url) = urls.get(i) else {
                        break;
                    };
                    let answer = http.get(url, None);
                    answers.lock().unwrap_or_else(|e| e.into_inner())[i] = Some(answer);
                }
            });
        }
    });
    answers
        .into_inner()
        .unwrap_or_else(|e| e.into_inner())
        .into_iter()
        .map(|a| a.expect("every request was made"))
        .collect()
}

// What a feed answered within FEED_REUSE of now, else a fresh answer.
fn recent<T>(
    slot: &mut Recent<T>,
    now: Timestamp,
    fetch: impl FnOnce() -> Result<T, Problem>,
) -> Result<Arc<T>, String> {
    if let Some((at, answer)) = slot.as_ref()
        && now.duration_since(*at).abs() < FEED_REUSE
    {
        return answer.clone();
    }
    let answer = fetch().map(Arc::new).map_err(problem_text);
    *slot = Some((now, answer.clone()));
    answer
}
