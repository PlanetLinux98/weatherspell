// Air quality, after Rest of today: the index the place's own authority
// publishes where this app can get it (Environment Canada's Air Quality
// Health Index near one of its communities), otherwise Open-Meteo's, from
// the Copernicus Atmosphere Monitoring Service's model, in the scale its
// region reads: the European Air Quality Index in Europe, the US Air
// Quality Index everywhere else (Elliott, 2026-10-27). Pollen is added
// wherever Open-Meteo has any (Europe, in season); elsewhere it gives none
// and the line is left out. Only the addresses, the parsing and the
// wording live here; fetch.rs does the asking.
//
// An AQHI estimated from Open-Meteo's concentrations for Canadian places
// away from a community was tried and left out: against Environment
// Canada's own readings at all 134 communities it caught 6 of 113
// moderate-risk hours and named 277 that were not (drafts, 2026-10-27).

use jiff::civil::DateTime;
use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};

use crate::clock;
use crate::location::Location;
use crate::official::OfficialError;
use crate::writer::Section;
use crate::{environment_canada as ec, meteoalarm, round_away};

pub const HEADING: &str = "Air quality";

const OPEN_METEO_ENDPOINT: &str = "https://air-quality-api.open-meteo.com/v1/air-quality";
pub const OPEN_METEO_NOTE: &str = "Open-Meteo (open-meteo.com), licensed CC BY 4.0, from the Copernicus Atmosphere Monitoring Service";

const GEOMET: &str = "https://api.weather.gc.ca/collections";
pub const STATIONS_URL: &str =
    "https://api.weather.gc.ca/collections/aqhi-stations/items?f=json&limit=500";
// Communities are cities and their parts ("Toronto Downtown"); further
// than this, another community's air is not this place's.
pub const MAX_COMMUNITY_DISTANCE_KM: f64 = 40.0;
// An observation older than this is not "right now"; the forecast for the
// hour stands in.
const OBSERVATION_FRESH: SignedDuration = SignedDuration::from_hours(3);
// How far ahead the forecast line looks.
const AHEAD: SignedDuration = SignedDuration::from_hours(12);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Scale {
    Us,
    European,
    Aqhi,
}

impl Scale {
    pub fn name(self) -> &'static str {
        match self {
            Scale::Us => "US Air Quality Index",
            Scale::European => "European Air Quality Index",
            Scale::Aqhi => "Air Quality Health Index",
        }
    }

    // The band's words and its rank, 0 for the best. Bands are read from
    // the index as shown: whole numbers, and the AQHI at least 1.
    pub fn band(self, index: f64) -> (usize, &'static str) {
        let n = self.shown(index);
        match self {
            Scale::Us => match n {
                ..=50 => (0, "good"),
                51..=100 => (1, "moderate"),
                101..=150 => (2, "unhealthy for sensitive groups"),
                151..=200 => (3, "unhealthy"),
                201..=300 => (4, "very unhealthy"),
                _ => (5, "hazardous"),
            },
            // Open-Meteo's bands for the EEA's index.
            Scale::European => match n {
                ..=20 => (0, "good"),
                21..=40 => (1, "fair"),
                41..=60 => (2, "moderate"),
                61..=80 => (3, "poor"),
                81..=100 => (4, "very poor"),
                _ => (5, "extremely poor"),
            },
            Scale::Aqhi => match n {
                ..=3 => (0, "low risk"),
                4..=6 => (1, "moderate risk"),
                7..=10 => (2, "high risk"),
                _ => (3, "very high risk"),
            },
        }
    }

    fn shown(self, index: f64) -> i32 {
        let n = round_away(index);
        if self == Scale::Aqhi {
            n.max(1)
        } else {
            n.max(0)
        }
    }

    // "32", or for the AQHI past its top, "above 10" as Environment Canada
    // shows "10+".
    fn number(self, index: f64) -> String {
        let n = self.shown(index);
        if self == Scale::Aqhi && n > 10 {
            "above 10".to_string()
        } else {
            n.to_string()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Pollutant {
    FineParticles,
    CoarseParticles,
    Ozone,
    NitrogenDioxide,
    SulphurDioxide,
    CarbonMonoxide,
}

impl Pollutant {
    pub fn words(self) -> &'static str {
        match self {
            Pollutant::FineParticles => "fine particles",
            Pollutant::CoarseParticles => "coarse particles",
            Pollutant::Ozone => "ozone",
            Pollutant::NitrogenDioxide => "nitrogen dioxide",
            Pollutant::SulphurDioxide => "sulphur dioxide",
            Pollutant::CarbonMonoxide => "carbon monoxide",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AirHour {
    #[serde(with = "crate::iso::wall")]
    pub local_time: DateTime,
    pub index: f64,
    pub main: Option<Pollutant>,
}

// Grains per cubic metre, None where the model has none.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PollenHour {
    #[serde(with = "crate::iso::wall")]
    pub local_time: DateTime,
    pub alder: Option<f64>,
    pub birch: Option<f64>,
    pub grass: Option<f64>,
    pub mugwort: Option<f64>,
    pub olive: Option<f64>,
    pub ragweed: Option<f64>,
}

// observed is Environment Canada's latest reading; hours the forecast
// index by hour (EC's own forecast for the AQHI). problem, with no index
// at all, is why the section says it could not be checked.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AirQuality {
    pub scale: Scale,
    pub community: Option<String>,
    pub observed: Option<AirHour>,
    pub hours: Vec<AirHour>,
    pub pollen: Vec<PollenHour>,
    pub sources: Vec<String>,
    pub problem: Option<String>,
}

// The scale a place reads when its own authority's index cannot be had.
pub fn open_meteo_scale(location: &Location) -> Scale {
    let country = location.country.as_deref().unwrap_or_default().trim();
    let european = meteoalarm::country_of(location).is_some()
        || OTHER_EUROPEAN
            .iter()
            .any(|c| c.eq_ignore_ascii_case(country));
    if european { Scale::European } else { Scale::Us }
}

// European countries MeteoAlarm has no feed for, as the geocoder names
// them.
const OTHER_EUROPEAN: [&str; 14] = [
    "Albania",
    "Andorra",
    "Belarus",
    "Kosovo",
    "Liechtenstein",
    "Monaco",
    "San Marino",
    "Vatican City",
    "Faroe Islands",
    "Gibraltar",
    "Guernsey",
    "Jersey",
    "Isle of Man",
    "Moldova",
];

const POLLEN_FIELDS: &str =
    "alder_pollen,birch_pollen,grass_pollen,mugwort_pollen,olive_pollen,ragweed_pollen";
const US_FIELDS: &str = "us_aqi,us_aqi_pm2_5,us_aqi_pm10,us_aqi_nitrogen_dioxide,us_aqi_ozone,us_aqi_sulphur_dioxide,us_aqi_carbon_monoxide";
const EUROPEAN_FIELDS: &str = "european_aqi,european_aqi_pm2_5,european_aqi_pm10,european_aqi_nitrogen_dioxide,european_aqi_ozone,european_aqi_sulphur_dioxide";

// Today from midnight (pollen is classed by the day's mean) through
// tomorrow, so the next twelve hours are there late in the evening.
pub fn open_meteo_url(location: &Location, scale: Scale) -> String {
    let fields = if scale == Scale::European {
        EUROPEAN_FIELDS
    } else {
        US_FIELDS
    };
    format!(
        "{OPEN_METEO_ENDPOINT}?latitude={}&longitude={}&hourly={fields},{POLLEN_FIELDS}&forecast_days=2&timezone=auto",
        crate::open_meteo::coordinate(location.latitude),
        crate::open_meteo::coordinate(location.longitude),
    )
}

#[derive(Deserialize)]
struct OpenMeteoResponse {
    hourly: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OpenMeteoAir {
    pub hours: Vec<AirHour>,
    pub pollen: Vec<PollenHour>,
}

pub fn parse_open_meteo(json: &str, scale: Scale) -> Result<OpenMeteoAir, OfficialError> {
    let response: OpenMeteoResponse = serde_json::from_str(json)?;
    let hourly = response
        .hourly
        .ok_or_else(|| OfficialError::Unreadable("Air quality response has no hours.".into()))?;
    let column = |name: &str| -> Vec<Option<f64>> {
        hourly
            .get(name)
            .and_then(|v| v.as_array())
            .map(|values| values.iter().map(serde_json::Value::as_f64).collect())
            .unwrap_or_default()
    };
    let times: Vec<DateTime> = hourly
        .get("time")
        .and_then(|v| v.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|v| v.as_str()?.parse::<DateTime>().ok())
                .collect()
        })
        .unwrap_or_default();
    let at = |values: &[Option<f64>], i: usize| values.get(i).copied().flatten();

    let (index_name, parts): (&str, &[(&str, Pollutant)]) = if scale == Scale::European {
        (
            "european_aqi",
            &[
                ("european_aqi_pm2_5", Pollutant::FineParticles),
                ("european_aqi_pm10", Pollutant::CoarseParticles),
                ("european_aqi_ozone", Pollutant::Ozone),
                ("european_aqi_nitrogen_dioxide", Pollutant::NitrogenDioxide),
                ("european_aqi_sulphur_dioxide", Pollutant::SulphurDioxide),
            ],
        )
    } else {
        (
            "us_aqi",
            &[
                ("us_aqi_pm2_5", Pollutant::FineParticles),
                ("us_aqi_pm10", Pollutant::CoarseParticles),
                ("us_aqi_ozone", Pollutant::Ozone),
                ("us_aqi_nitrogen_dioxide", Pollutant::NitrogenDioxide),
                ("us_aqi_sulphur_dioxide", Pollutant::SulphurDioxide),
                ("us_aqi_carbon_monoxide", Pollutant::CarbonMonoxide),
            ],
        )
    };
    let index = column(index_name);
    let part_columns: Vec<(Vec<Option<f64>>, Pollutant)> = parts
        .iter()
        .map(|(name, pollutant)| (column(name), *pollutant))
        .collect();
    let pollen: Vec<Vec<Option<f64>>> = [
        "alder_pollen",
        "birch_pollen",
        "grass_pollen",
        "mugwort_pollen",
        "olive_pollen",
        "ragweed_pollen",
    ]
    .iter()
    .map(|name| column(name))
    .collect();

    let mut air = OpenMeteoAir::default();
    for (i, t) in times.iter().enumerate() {
        if let Some(value) = at(&index, i) {
            // The pollutant whose own index is the overall one; the first
            // listed wins a tie.
            let mut main: Option<(f64, Pollutant)> = None;
            for (values, pollutant) in &part_columns {
                if let Some(v) = at(values, i)
                    && main.is_none_or(|(best, _)| v > best)
                {
                    main = Some((v, *pollutant));
                }
            }
            air.hours.push(AirHour {
                local_time: *t,
                index: value,
                main: main.map(|(_, p)| p),
            });
        }
        let p = |k: usize| at(&pollen[k], i);
        if (0..6).any(|k| p(k).is_some()) {
            air.pollen.push(PollenHour {
                local_time: *t,
                alder: p(0),
                birch: p(1),
                grass: p(2),
                mugwort: p(3),
                olive: p(4),
                ragweed: p(5),
            });
        }
    }
    Ok(air)
}

// Environment Canada's AQHI communities.
#[derive(Clone, Debug, PartialEq)]
pub struct Community {
    pub id: String,
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Deserialize)]
struct Features {
    features: Option<Vec<Feature>>,
}

#[derive(Deserialize)]
struct Feature {
    geometry: Option<Point>,
    properties: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Deserialize)]
struct Point {
    coordinates: Option<Vec<f64>>,
}

fn text(p: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<String> {
    p.get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

pub fn parse_communities(json: &str) -> Result<Vec<Community>, OfficialError> {
    let list: Features = serde_json::from_str(json)?;
    let communities: Vec<Community> = list
        .features
        .unwrap_or_default()
        .into_iter()
        .filter_map(|f| {
            let p = f.properties?;
            let point = f.geometry?.coordinates?;
            Some(Community {
                id: text(&p, "location_id")?,
                name: text(&p, "location_name_en")?,
                longitude: *point.first()?,
                latitude: *point.get(1)?,
            })
        })
        .collect();
    if communities.is_empty() {
        return Err(OfficialError::Unreadable(
            "Environment Canada's list of AQHI communities is empty.".into(),
        ));
    }
    Ok(communities)
}

pub fn nearest_community<'a>(
    communities: &'a [Community],
    location: &Location,
) -> Option<(&'a Community, f64)> {
    communities
        .iter()
        .map(|c| {
            (
                c,
                ec::distance_km(
                    location.latitude,
                    location.longitude,
                    c.latitude,
                    c.longitude,
                ),
            )
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .filter(|(_, distance)| *distance <= MAX_COMMUNITY_DISTANCE_KM)
}

pub fn observation_url(community: &Community) -> String {
    format!(
        "{GEOMET}/aqhi-observations-realtime/items?f=json&location_id={}&latest=true",
        crate::open_meteo::escape(&community.id)
    )
}

// Newest publication first; one publication is about fifty hours.
pub fn forecasts_url(community: &Community) -> String {
    format!(
        "{GEOMET}/aqhi-forecasts-realtime/items?f=json&location_id={}&sortby=-publication_datetime&limit=150",
        crate::open_meteo::escape(&community.id)
    )
}

fn stamp(p: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<Timestamp> {
    text(p, key)?.parse().ok()
}

// The latest reading, if the community has one.
pub fn parse_observation(json: &str) -> Result<Option<(Timestamp, f64)>, OfficialError> {
    let list: Features = serde_json::from_str(json)?;
    Ok(list
        .features
        .unwrap_or_default()
        .into_iter()
        .filter_map(|f| {
            let p = f.properties?;
            Some((stamp(&p, "observation_datetime")?, p.get("aqhi")?.as_f64()?))
        })
        .max_by_key(|(t, _)| *t))
}

// The newest publication's hours, in order.
pub fn parse_forecasts(json: &str) -> Result<Vec<(Timestamp, f64)>, OfficialError> {
    let list: Features = serde_json::from_str(json)?;
    let items: Vec<(Timestamp, Timestamp, f64)> = list
        .features
        .unwrap_or_default()
        .into_iter()
        .filter_map(|f| {
            let p = f.properties?;
            Some((
                stamp(&p, "publication_datetime")?,
                stamp(&p, "forecast_datetime")?,
                p.get("aqhi")?.as_f64()?,
            ))
        })
        .collect();
    let Some(newest) = items.iter().map(|(published, _, _)| *published).max() else {
        return Ok(Vec::new());
    };
    let mut hours: Vec<(Timestamp, f64)> = items
        .into_iter()
        .filter(|(published, _, _)| *published == newest)
        .map(|(_, at, aqhi)| (at, aqhi))
        .collect();
    hours.sort_by_key(|(at, _)| *at);
    hours.dedup_by_key(|(at, _)| *at);
    Ok(hours)
}

// What the asking came back with, before it is put on the location's
// clock (compose).
#[derive(Clone, Debug, Default)]
pub struct Fetched {
    pub community: Option<Community>,
    pub observed: Option<(Timestamp, f64)>,
    pub forecasts: Vec<(Timestamp, f64)>,
    // Why Environment Canada's index could not be had near a community.
    pub aqhi_problem: Option<String>,
    pub open_meteo: Option<Result<OpenMeteoAir, String>>,
    pub open_meteo_scale: Option<Scale>,
}

// wall turns a service's timestamp into the location's own clock time.
pub fn compose(fetched: Fetched, wall: impl Fn(Timestamp) -> DateTime) -> AirQuality {
    let pollen = match &fetched.open_meteo {
        Some(Ok(air)) => air.pollen.clone(),
        _ => Vec::new(),
    };
    let has_pollen = pollen_reported(&pollen);
    let open_meteo_scale = fetched.open_meteo_scale.unwrap_or(Scale::Us);

    // Environment Canada's index, when it has anything for this hour or
    // the hours ahead.
    if let Some(community) = &fetched.community
        && fetched.aqhi_problem.is_none()
        && (fetched.observed.is_some() || !fetched.forecasts.is_empty())
    {
        let mut sources = vec![format!(
            "Air quality: {}'s Air Quality Health Index for {} (weather.gc.ca).",
            ec::FULL_NAME,
            community.name
        )];
        if has_pollen {
            sources.push(format!("Pollen: {OPEN_METEO_NOTE}."));
        }
        return AirQuality {
            scale: Scale::Aqhi,
            community: Some(community.name.clone()),
            observed: fetched.observed.map(|(t, index)| AirHour {
                local_time: wall(t),
                index,
                main: None,
            }),
            hours: fetched
                .forecasts
                .iter()
                .map(|(t, index)| AirHour {
                    local_time: wall(*t),
                    index: *index,
                    main: None,
                })
                .collect(),
            pollen,
            sources,
            problem: None,
        };
    }

    let mut sources = Vec::new();
    if let Some(community) = &fetched.community {
        let why = fetched
            .aqhi_problem
            .as_deref()
            .unwrap_or("it had no reading or forecast")
            .trim_end_matches(['.', ' ']);
        sources.push(format!(
            "Environment Canada's Air Quality Health Index for {} could not be fetched this time ({why}), so this is the {}.",
            community.name,
            open_meteo_scale.name()
        ));
    }
    match fetched.open_meteo {
        Some(Ok(air)) => {
            sources.push(format!(
                "{}: {OPEN_METEO_NOTE}.",
                if has_pollen {
                    "Air quality and pollen"
                } else {
                    "Air quality"
                }
            ));
            AirQuality {
                scale: open_meteo_scale,
                community: None,
                observed: None,
                hours: air.hours,
                pollen: air.pollen,
                sources,
                problem: None,
            }
        }
        Some(Err(problem)) => AirQuality {
            scale: open_meteo_scale,
            community: None,
            observed: None,
            hours: Vec::new(),
            pollen: Vec::new(),
            sources: Vec::new(),
            problem: Some(problem),
        },
        None => AirQuality {
            scale: open_meteo_scale,
            community: None,
            observed: None,
            hours: Vec::new(),
            pollen: Vec::new(),
            sources: Vec::new(),
            problem: Some("no air quality service was asked".to_string()),
        },
    }
}

// The section, or None when there is nothing to say (no hours left in the
// data and no problem to report).
pub fn section(air: &AirQuality, now_local: DateTime) -> Option<Section> {
    let hour = floor(now_local);
    let now = air
        .observed
        .as_ref()
        .filter(|o| now_local.duration_since(o.local_time) <= OBSERVATION_FRESH)
        .or_else(|| air.hours.iter().find(|h| h.local_time == hour))
        // The forecast's first hour when this one is not in it (EC's
        // forecast can start an hour or two ahead).
        .or_else(|| {
            air.hours.iter().find(|h| {
                h.local_time > hour && h.local_time.duration_since(hour) <= OBSERVATION_FRESH
            })
        });
    let mut paragraphs = Vec::new();
    match now {
        Some(now) => {
            paragraphs.push(now_line(air, now));
            if let Some(line) = ahead_line(air, now, now_local) {
                paragraphs.push(line);
            }
        }
        None => {
            if let Some(problem) = &air.problem {
                let problem = problem.trim_end_matches(['.', ' ']);
                paragraphs.push(format!(
                    "Air quality couldn't be checked this time ({problem}). Press F5 to try again."
                ));
            } else if air.hours.iter().any(|h| h.local_time > hour) {
                paragraphs.push(format!(
                    "There is no {} for this hour yet.",
                    air.scale.name()
                ));
            }
            // Hours all past (an old forecast from the cache, whose age
            // Right now gives): nothing to say about now.
        }
    }
    if let Some(line) = pollen_line(&air.pollen, now_local) {
        paragraphs.push(line);
    }
    (!paragraphs.is_empty()).then(|| Section::new(HEADING, paragraphs))
}

// "Air quality is moderate, 68 on the US Air Quality Index, mostly from
// ozone."
fn now_line(air: &AirQuality, now: &AirHour) -> String {
    let (rank, words) = air.scale.band(now.index);
    let mut s = format!(
        "Air quality is {words}, {} on the {}",
        air.scale.number(now.index),
        air.scale.name()
    );
    if let Some(community) = &air.community {
        s += &format!(" for {community}");
    }
    if rank > 0
        && let Some(main) = now.main
    {
        s += &format!(", mostly from {}", main.words());
    }
    s + "."
}

// The next twelve hours, when they move to another band: up to the worst,
// or, when nothing is worse, down to the best.
fn ahead_line(air: &AirQuality, now: &AirHour, now_local: DateTime) -> Option<String> {
    let (rank, _) = air.scale.band(now.index);
    let end = now_local.checked_add(AHEAD).ok()?;
    let ahead: Vec<&AirHour> = air
        .hours
        .iter()
        .filter(|h| h.local_time > now.local_time.max(floor(now_local)) && h.local_time <= end)
        .collect();
    let worst = first_by(&ahead, |a, b| a.index > b.index)?;
    let (worst_rank, worst_words) = air.scale.band(worst.index);
    if worst_rank > rank {
        return Some(format!(
            "Forecast to reach {worst_words} {}, up to {}.",
            when(worst.local_time, now_local),
            air.scale.number(worst.index)
        ));
    }
    let best = first_by(&ahead, |a, b| a.index < b.index)?;
    let (best_rank, best_words) = air.scale.band(best.index);
    if best_rank < rank {
        // When it first gets there, not when it is lowest.
        let first = ahead
            .iter()
            .find(|h| air.scale.band(h.index).0 == best_rank)
            .unwrap_or(&best);
        return Some(format!(
            "Forecast to ease to {best_words} {}.",
            when(first.local_time, now_local)
        ));
    }
    None
}

// The first item that beats every later one.
fn first_by<'a>(
    items: &[&'a AirHour],
    better: impl Fn(&AirHour, &AirHour) -> bool,
) -> Option<&'a AirHour> {
    let mut best: Option<&AirHour> = None;
    for item in items {
        if best.is_none_or(|b| better(item, b)) {
            best = Some(item);
        }
    }
    best
}

// "this afternoon", "overnight", "tomorrow morning".
fn when(t: DateTime, now_local: DateTime) -> &'static str {
    let today = now_local.date();
    let hour = t.hour();
    if t.date() == today {
        match hour {
            ..5 => "early this morning",
            5..12 => "this morning",
            12..17 => "this afternoon",
            17..21 => "this evening",
            _ => "tonight",
        }
    } else if clock::days_between(today, t.date()) == 1 {
        match hour {
            ..5 => "overnight",
            5..12 => "tomorrow morning",
            12..17 => "tomorrow afternoon",
            17..21 => "tomorrow evening",
            _ => "tomorrow night",
        }
    } else {
        "later"
    }
}

// MeteoSwiss's classes for a day's mean grains per cubic metre (the lowest
// figure of low, moderate, high and very high), and for olive, which it
// does not class, the Spanish Aerobiology Network's (low, moderate, high).
const POLLEN_CLASSES: [(&str, [f64; 4]); 6] = [
    ("alder", [1.0, 11.0, 70.0, 250.0]),
    ("birch", [1.0, 11.0, 70.0, 300.0]),
    ("grass", [1.0, 20.0, 50.0, 150.0]),
    ("mugwort", [1.0, 6.0, 15.0, 50.0]),
    ("olive", [1.0, 51.0, 201.0, f64::INFINITY]),
    ("ragweed", [1.0, 6.0, 11.0, 40.0]),
];
const POLLEN_LEVELS: [&str; 4] = ["low", "moderate", "high", "very high"];

fn pollen_reported(pollen: &[PollenHour]) -> bool {
    !pollen.is_empty()
}

fn values(h: &PollenHour) -> [Option<f64>; 6] {
    [h.alder, h.birch, h.grass, h.mugwort, h.olive, h.ragweed]
}

// "Pollen today: grass moderate, birch low." from the day's mean, as the
// classes are meant to be read; "none" when nothing reaches low; nothing
// at all where the model has no pollen.
fn pollen_line(pollen: &[PollenHour], now_local: DateTime) -> Option<String> {
    let today: Vec<&PollenHour> = pollen
        .iter()
        .filter(|h| h.local_time.date() == now_local.date())
        .collect();
    if today.is_empty() {
        return None;
    }
    let mut levels: Vec<(usize, usize)> = Vec::new();
    for (k, (_, classes)) in POLLEN_CLASSES.iter().enumerate() {
        let day: Vec<f64> = today.iter().filter_map(|h| values(h)[k]).collect();
        if day.is_empty() {
            continue;
        }
        let mean = day.iter().sum::<f64>() / day.len() as f64;
        if let Some(level) = classes.iter().rposition(|lowest| mean >= *lowest) {
            levels.push((k, level));
        }
    }
    if levels.is_empty() {
        return Some("Pollen today: none.".to_string());
    }
    // Highest first; the table's order among equals.
    levels.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let parts: Vec<String> = levels
        .iter()
        .map(|(k, level)| format!("{} {}", POLLEN_CLASSES[*k].0, POLLEN_LEVELS[*level]))
        .collect();
    Some(format!("Pollen today: {}.", parts.join(", ")))
}

fn floor(t: DateTime) -> DateTime {
    t.date().at(t.hour(), 0, 0, 0)
}
