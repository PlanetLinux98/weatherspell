// Alerts for Europe from MeteoAlarm (EUMETNET), which gathers the national
// services' warnings for 38 countries, the UK and Ireland included: one
// keyless feed per country, CAP fields as JSON, every warning with an
// English block. Its terms are CC BY 4.0 plus: name the national service,
// give the time of issue, link to meteoalarm.org, publish its disclaimer,
// and stay within ten minutes of meteoalarm.org (drafts/alerts-plan.md).
//
// A feed holds the whole country, expired warnings and "no warning"
// (green) entries included, so most of the work is choosing: live messages
// that nothing later in the feed replaces, not all-clear, not ended, at
// least yellow, in an area that holds the location. Areas are given three
// ways: an outline in the warning (the UK, Switzerland, Norway...), a
// MeteoAlarm region code (EMMA_ID, most countries), whose outlines this
// app carries simplified (data/meteoalarm-regions.txt), or a NUTS or FIPS
// code, matched by the area's name or, for Ireland's counties, a table.
//
// The services name their warnings in a dozen styles ("gale-force gusts",
// "Wind Level 2", "Moderate rain warning" for a yellow one, "no text"), so
// an alert is named the way MeteoAlarm and the national colour systems
// name them, "Orange wind warning", from its level and hazard type; the
// service's own headline and text follow in the details, unchanged in
// substance. The UK's Met Office says amber for orange. The flood
// agencies' warnings name a place and keep their own names.

use std::collections::HashSet;
use std::sync::LazyLock;

use jiff::Timestamp;
use serde_json::Value;

use crate::alerts::{AlertSeverity, WeatherAlert};
use crate::area::{self, Polygon, Region};
use crate::cap::{self, CapAlert, CapArea, CapInfo};
use crate::location::Location;
use crate::official::OfficialError;

pub const SITE: &str = "https://meteoalarm.org";
const FEEDS: &str = "https://feeds.meteoalarm.org/api/v1/warnings/feeds-";

// Section 5 of MeteoAlarm's terms, word for word.
pub const DISCLAIMER: &str = "Time delays between this website and the www.meteoalarm.org website are possible. For the most up-to-date awareness information as published by the participating National Meteorological and Hydrological Services, please refer to www.meteoalarm.org.";

// The regions table moves a boundary by up to about 0.6 km
// (examples/meteoalarm_regions.rs).
pub const TABLE_MARGIN_KM: f64 = 0.75;

// The terms' "never longer than ten minutes": European locations are
// checked at least this often whatever the alert interval setting says.
pub const MAX_CHECK_MINUTES: u32 = 10;

// The largest feeds take their time: Switzerland's 11.7 MB took 23
// seconds on the Windows machine's connection (2026-10-25), past the
// client's usual 20.
pub const FEED_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(90);

pub struct Country {
    pub feed: &'static str,
    names: &'static [&'static str],
    // The national service, as said after "from".
    pub service: &'static str,
    amber: bool,
}

// Names as Open-Meteo's geocoder and Nominatim give them in English, with
// the other spellings either has used.
const COUNTRIES: &[Country] = &[
    country(
        "andorra",
        &["Andorra"],
        "the National Meteorological Service of Andorra",
    ),
    country("austria", &["Austria"], "GeoSphere Austria"),
    country(
        "belgium",
        &["Belgium"],
        "the Royal Meteorological Institute of Belgium",
    ),
    country(
        "bosnia-herzegovina",
        &["Bosnia and Herzegovina"],
        "the Federal Hydrometeorological Institute of Bosnia and Herzegovina",
    ),
    country(
        "bulgaria",
        &["Bulgaria"],
        "the National Institute of Meteorology and Hydrology",
    ),
    country(
        "croatia",
        &["Croatia"],
        "the Croatian Meteorological and Hydrological Service",
    ),
    country(
        "cyprus",
        &["Cyprus"],
        "the Cyprus Department of Meteorology",
    ),
    country(
        "czechia",
        &["Czechia", "Czech Republic"],
        "the Czech Hydrometeorological Institute",
    ),
    country(
        "denmark",
        &["Denmark"],
        "the Danish Meteorological Institute",
    ),
    country("estonia", &["Estonia"], "the Estonian Environment Agency"),
    country(
        "finland",
        &["Finland"],
        "the Finnish Meteorological Institute",
    ),
    country("france", &["France"], "Météo-France"),
    country("germany", &["Germany"], "Deutscher Wetterdienst"),
    country(
        "greece",
        &["Greece"],
        "the Hellenic National Meteorological Service",
    ),
    country("hungary", &["Hungary"], "HungaroMet"),
    country(
        "iceland",
        &["Iceland"],
        "the Icelandic Meteorological Office",
    ),
    country("ireland", &["Ireland"], "Met Éireann"),
    country("israel", &["Israel"], "the Israel Meteorological Service"),
    country(
        "italy",
        &["Italy"],
        "the Italian Air Force Meteorological Service",
    ),
    country(
        "latvia",
        &["Latvia"],
        "the Latvian Environment, Geology and Meteorology Centre",
    ),
    country(
        "lithuania",
        &["Lithuania"],
        "the Lithuanian Hydrometeorological Service",
    ),
    country("luxembourg", &["Luxembourg"], "MeteoLux"),
    country("malta", &["Malta"], "the Malta Meteorological Office"),
    country(
        "moldova",
        &["Moldova", "Republic of Moldova"],
        "the Meteorology and Environmental Monitoring Authority of Moldova",
    ),
    country(
        "montenegro",
        &["Montenegro"],
        "the Institute of Hydrometeorology and Seismology of Montenegro",
    ),
    country(
        "netherlands",
        &[
            "Netherlands",
            "The Netherlands",
            "Kingdom of the Netherlands",
        ],
        "the Royal Netherlands Meteorological Institute",
    ),
    country(
        "republic-of-north-macedonia",
        &["North Macedonia", "Macedonia"],
        "the Hydrometeorological Service of North Macedonia",
    ),
    country(
        "norway",
        &["Norway"],
        "the Norwegian Meteorological Institute",
    ),
    country(
        "poland",
        &["Poland"],
        "the Institute of Meteorology and Water Management",
    ),
    country(
        "portugal",
        &["Portugal"],
        "the Portuguese Institute for Sea and Atmosphere",
    ),
    country(
        "romania",
        &["Romania"],
        "the National Meteorological Administration",
    ),
    country(
        "serbia",
        &["Serbia"],
        "the Republic Hydrometeorological Service of Serbia",
    ),
    country(
        "slovakia",
        &["Slovakia"],
        "the Slovak Hydrometeorological Institute",
    ),
    country(
        "slovenia",
        &["Slovenia"],
        "the Slovenian Environment Agency",
    ),
    country("spain", &["Spain"], "the State Meteorological Agency"),
    country(
        "sweden",
        &["Sweden"],
        "the Swedish Meteorological and Hydrological Institute",
    ),
    country("switzerland", &["Switzerland"], "MeteoSwiss"),
    country(
        "ukraine",
        &["Ukraine"],
        "the Ukrainian Hydrometeorological Center",
    ),
    Country {
        feed: "united-kingdom",
        names: &["United Kingdom"],
        service: "the Met Office",
        amber: true,
    },
];

const fn country(
    feed: &'static str,
    names: &'static [&'static str],
    service: &'static str,
) -> Country {
    Country {
        feed,
        names,
        service,
        amber: false,
    }
}

// The other services that issue through MeteoAlarm, by a word of their
// sender name, with whether they are a flood agency naming a place.
const OTHER_SERVICES: &[(&str, &str, bool)] = &[
    ("Environment Agency", "the Environment Agency", true),
    ("Natural Resources Wales", "Natural Resources Wales", true),
    (
        "Scottish Environment Protection Agency",
        "the Scottish Environment Protection Agency",
        true,
    ),
    (
        "Water Resources and Energy Directorate",
        "the Norwegian Water Resources and Energy Directorate",
        false,
    ),
    (
        "Srpska",
        "the Republic Hydrometeorological Service of Republika Srpska",
        false,
    ),
];

pub fn country_of(location: &Location) -> Option<&'static Country> {
    let name = location.country.as_deref()?.trim();
    COUNTRIES
        .iter()
        .find(|c| c.names.iter().any(|n| n.eq_ignore_ascii_case(name)))
}

pub fn feed_url(country: &Country) -> String {
    format!("{FEEDS}{}", country.feed)
}

// "Met Éireann through MeteoAlarm (meteoalarm.org), licensed CC BY 4.0",
// naming whichever services the alerts shown came from.
pub fn attribution(country: &Country, alerts: &[WeatherAlert]) -> String {
    let mut services: Vec<&str> = Vec::new();
    for a in alerts {
        if !services.contains(&a.source.as_str()) {
            services.push(&a.source);
        }
    }
    if services.is_empty() {
        services.push(country.service);
    }
    let named = match services.as_slice() {
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
        [] => unreachable!(),
    };
    format!("{named} through MeteoAlarm (meteoalarm.org), licensed CC BY 4.0")
}

// The feed's warnings, each with its English block only.
pub fn parse_feed(json: &str) -> Result<Vec<CapAlert>, OfficialError> {
    let doc: Value = serde_json::from_str(json).map_err(|e| {
        OfficialError::Unreadable(format!("MeteoAlarm's alerts could not be read: {e}."))
    })?;
    let warnings = doc["warnings"].as_array().ok_or_else(|| {
        OfficialError::Unreadable("MeteoAlarm's alerts could not be read: no warnings list.".into())
    })?;
    Ok(warnings
        .iter()
        .map(|w| cap_alert(&w["alert"]))
        .filter(|a| !a.infos.is_empty())
        .collect())
}

fn text(v: &Value) -> String {
    v.as_str().unwrap_or_default().trim().to_string()
}

// A field given either as a string or a list of strings.
fn texts(v: &Value) -> Vec<String> {
    match v {
        Value::String(s) => vec![s.trim().to_string()],
        Value::Array(list) => list
            .iter()
            .filter_map(Value::as_str)
            .map(|s| s.trim().to_string())
            .collect(),
        _ => Vec::new(),
    }
}

fn pairs(v: &Value) -> Vec<(String, String)> {
    v.as_array()
        .into_iter()
        .flatten()
        .map(|p| (text(&p["valueName"]), text(&p["value"])))
        .collect()
}

fn cap_alert(a: &Value) -> CapAlert {
    let english = a["info"].as_array().into_iter().flatten().find(|i| {
        let language = text(&i["language"]).to_ascii_lowercase();
        language.is_empty() || language == "en" || language.starts_with("en-")
    });
    CapAlert {
        identifier: text(&a["identifier"]),
        sent: cap::time(&text(&a["sent"])),
        status: text(&a["status"]),
        msg_type: text(&a["msgType"]),
        references: cap::references(&text(&a["references"])),
        infos: english
            .map(|i| CapInfo {
                language: text(&i["language"]),
                event: text(&i["event"]),
                headline: text(&i["headline"]),
                description: text(&i["description"]),
                instruction: text(&i["instruction"]),
                sender_name: text(&i["senderName"]),
                severity: text(&i["severity"]),
                response_types: texts(&i["responseType"]),
                effective: cap::time(&text(&i["effective"])),
                onset: cap::time(&text(&i["onset"])),
                expires: cap::time(&text(&i["expires"])),
                web: text(&i["web"]),
                parameters: pairs(&i["parameter"]),
                areas: i["area"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|area| CapArea {
                        description: text(&area["areaDesc"]),
                        polygons: texts(&area["polygon"])
                            .iter()
                            .filter_map(|p| Polygon::parse(p))
                            .collect(),
                        geocodes: pairs(&area["geocode"]),
                    })
                    .collect(),
            })
            .into_iter()
            .collect(),
    }
}

// The alerts in effect at the location, in no particular order.
pub fn alerts_for(
    feed: &[CapAlert],
    country: &Country,
    location: &Location,
    now: Timestamp,
) -> Vec<WeatherAlert> {
    let regions = regions_at(location.latitude, location.longitude);
    // A message that a later one updates or cancels is replaced by it.
    let replaced: HashSet<&str> = feed
        .iter()
        .flat_map(|a| a.references.iter().map(String::as_str))
        .collect();
    feed.iter()
        .filter(|a| a.is_live() && !replaced.contains(a.identifier.as_str()))
        .filter_map(|a| {
            let info = a.english()?;
            if info.is_all_clear() || info.has_ended(now) {
                return None;
            }
            let level = level(info)?;
            let areas = areas_at(info, location, &regions);
            if areas.is_empty() {
                return None;
            }
            weather_alert(a, info, level, &areas, country)
        })
        .collect()
}

// MeteoAlarm's awareness level, "2; yellow; Moderate" to "4; red;
// Extreme"; green (1) means no warning. A message without one is placed
// by its CAP severity.
fn level(info: &CapInfo) -> Option<u8> {
    let level = match info.parameter("awareness_level") {
        Some(value) => value.split(';').next()?.trim().parse().ok()?,
        None => match info.severity.to_ascii_lowercase().as_str() {
            "extreme" => 4,
            "severe" => 3,
            "moderate" => 2,
            _ => 1,
        },
    };
    (2..=4).contains(&level).then_some(level)
}

fn hazard(info: &CapInfo) -> Option<&'static str> {
    let number: u8 = info
        .parameter("awareness_type")?
        .split(';')
        .next()?
        .trim()
        .parse()
        .ok()?;
    Some(match number {
        1 => "wind",
        2 => "snow and ice",
        3 => "thunderstorm",
        4 => "fog",
        5 => "high temperature",
        6 => "low temperature",
        7 => "coastal event",
        8 => "forest fire",
        9 => "avalanche",
        10 => "rain",
        12 => "flood",
        13 => "rain and flood",
        14 => "marine",
        15 => "drought",
        _ => return None,
    })
}

static REGIONS: LazyLock<Vec<Region>> =
    LazyLock::new(|| area::parse_table(include_str!("../data/meteoalarm-regions.txt")));

// Codes 801 and up are sea areas: a location on land is never in one,
// and a margin would put every coastal town under the sea's gale warnings.
fn is_sea(code: &str) -> bool {
    code.get(2..)
        .and_then(|n| n.parse::<u32>().ok())
        .is_some_and(|n| (801..=899).contains(&n))
}

pub fn regions_at(latitude: f64, longitude: f64) -> Vec<&'static Region> {
    REGIONS
        .iter()
        .filter(|r| {
            let margin = if is_sea(&r.code) {
                0.0
            } else {
                TABLE_MARGIN_KM
            };
            r.covers(latitude, longitude, margin)
        })
        .collect()
}

// Ireland's land warnings name counties by their FIPS 10-4 codes; the
// regions table names them "County Dublin".
const IRISH_COUNTIES: &[(&str, &str)] = &[
    ("EI01", "Carlow"),
    ("EI02", "Cavan"),
    ("EI03", "Clare"),
    ("EI04", "Cork"),
    ("EI06", "Donegal"),
    ("EI07", "Dublin"),
    ("EI10", "Galway"),
    ("EI11", "Kerry"),
    ("EI12", "Kildare"),
    ("EI13", "Kilkenny"),
    ("EI14", "Leitrim"),
    ("EI15", "Laois"),
    ("EI16", "Limerick"),
    ("EI18", "Longford"),
    ("EI19", "Louth"),
    ("EI20", "Mayo"),
    ("EI21", "Meath"),
    ("EI22", "Monaghan"),
    ("EI23", "Offaly"),
    ("EI24", "Roscommon"),
    ("EI25", "Sligo"),
    ("EI26", "Tipperary"),
    ("EI27", "Waterford"),
    ("EI29", "Westmeath"),
    ("EI30", "Wexford"),
    ("EI31", "Wicklow"),
];

// NUTS areas are matched by name; where a service's English name differs
// from the table's, the table's (both folded).
const NAME_ALIASES: &[(&str, &str)] = &[
    ("antwerp", "antwerpen"),
    ("eastflanders", "oostvlaanderen"),
    ("westflanders", "westvlaanderen"),
    ("flemishbrabant", "brabant"),
    ("walloonbrabant", "brabant"),
    ("brussels", "brabant"),
    ("brusselscapitalregion", "brabant"),
    ("targovishte", "targovoshte"),
    ("bucharest", "municipiulbucuresti"),
    ("bucuresti", "municipiulbucuresti"),
    ("budapest", "centralhungary"),
    ("pest", "centralhungary"),
];

// The areas of a warning that hold the location: by the warning's own
// outline where it has one, else by region code, else by name.
fn areas_at<'a>(info: &'a CapInfo, location: &Location, regions: &[&Region]) -> Vec<&'a CapArea> {
    let (lat, lon) = (location.latitude, location.longitude);
    let codes: HashSet<&str> = regions.iter().map(|r| r.code.as_str()).collect();
    let names: HashSet<String> = regions.iter().map(|r| fold(&r.name)).collect();
    info.areas
        .iter()
        .filter(|area| {
            if !area.polygons.is_empty() {
                return area.polygons.iter().any(|p| p.covers(lat, lon, 0.0));
            }
            let emma: Vec<&str> = area
                .geocodes
                .iter()
                .filter(|(kind, _)| kind.eq_ignore_ascii_case("EMMA_ID"))
                .map(|(_, code)| code.as_str())
                .collect();
            if !emma.is_empty() {
                return emma.iter().any(|code| codes.contains(code));
            }
            let fips: Vec<&str> = area
                .geocodes
                .iter()
                .filter(|(kind, _)| kind.eq_ignore_ascii_case("FIPS"))
                .map(|(_, code)| code.as_str())
                .collect();
            if !fips.is_empty() {
                return fips.iter().any(|code| {
                    IRISH_COUNTIES
                        .iter()
                        .find(|(c, _)| c == code)
                        .is_some_and(|(_, county)| {
                            names.contains(&fold(&format!("County {county}")))
                        })
                });
            }
            !area.geocodes.is_empty() && {
                let name = fold(&area.description);
                let name = NAME_ALIASES
                    .iter()
                    .find(|(from, _)| *from == name)
                    .map_or(name.clone(), |(_, to)| to.to_string());
                names.contains(&name)
            }
        })
        .collect()
}

// Lower case letters only, accents dropped: "Caraș-Severin" and
// "CARAS-SEVERIN" are both "carasseverin".
pub(crate) fn fold(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        let plain = match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => "a",
            'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => "c",
            'ď' | 'đ' | 'ð' => "d",
            'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => "e",
            'ĝ' | 'ğ' | 'ġ' | 'ģ' => "g",
            'ĥ' | 'ħ' => "h",
            'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' => "i",
            'ĵ' => "j",
            'ķ' => "k",
            'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => "l",
            'ñ' | 'ń' | 'ņ' | 'ň' => "n",
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => "o",
            'ŕ' | 'ŗ' | 'ř' => "r",
            'ś' | 'ŝ' | 'ş' | 'š' | 'ș' => "s",
            'ţ' | 'ť' | 'ŧ' | 'ț' => "t",
            'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => "u",
            'ŵ' => "w",
            'ý' | 'ÿ' | 'ŷ' => "y",
            'ź' | 'ż' | 'ž' => "z",
            'ß' => "ss",
            'æ' => "ae",
            'œ' => "oe",
            'þ' => "th",
            c if c.is_ascii_alphanumeric() => {
                out.push(c);
                continue;
            }
            _ => continue,
        };
        out.push_str(plain);
    }
    out
}

fn weather_alert(
    a: &CapAlert,
    info: &CapInfo,
    level: u8,
    areas: &[&CapArea],
    country: &Country,
) -> Option<WeatherAlert> {
    let issued = a.sent.or(info.effective).or(info.onset)?;
    let other = OTHER_SERVICES
        .iter()
        .find(|(word, _, _)| info.sender_name.contains(word));
    let source = other.map_or(country.service, |(_, name, _)| name);
    let names_a_place = other.is_some_and(|(_, _, floods)| *floods);
    let event = if names_a_place && !info.event.is_empty() {
        flood_agency_event(&info.event)
    } else {
        let colour = match level {
            2 => "Yellow",
            3 if country.amber => "Amber",
            3 => "Orange",
            _ => "Red",
        };
        match hazard(info) {
            Some(hazard) => format!("{colour} {hazard} warning"),
            None => format!("{colour} warning"),
        }
    };
    let mut description = Vec::new();
    let headline = info.headline.trim();
    if !headline.is_empty()
        && !headline.eq_ignore_ascii_case(&event)
        && !info.description.trim_start().starts_with(headline)
    {
        description.push(cap::readable(headline));
    }
    if !info.description.trim().is_empty() {
        description.push(cap::readable(&info.description));
    }
    // The Met Office lists its regions split by " | ", and its region
    // names hold commas ("SW Scotland, Lothian Borders").
    let mut area_names: Vec<&str> = Vec::new();
    for name in areas
        .iter()
        .flat_map(|area| area.description.split(" | "))
        .map(str::trim)
    {
        if !name.is_empty() && !area_names.contains(&name) {
            area_names.push(name);
        }
    }
    let web = info.web.trim();
    Some(WeatherAlert {
        id: format!("eu:{}", a.identifier),
        event,
        severity: match level {
            2 => AlertSeverity::Moderate,
            3 => AlertSeverity::Severe,
            _ => AlertSeverity::Extreme,
        },
        issued,
        onset: info.onset.or(info.effective),
        ends: info.expires,
        source: source.to_string(),
        sender: source.to_string(),
        area: area_names.join("; "),
        level: None,
        description: description.join("\n\n"),
        instruction: Some(cap::readable(&info.instruction)).filter(|i| !i.trim().is_empty()),
        url: Some(if web.starts_with("http") { web } else { SITE }.to_string()),
        expires: info.expires,
        references: Some(a.references.iter().map(|r| format!("eu:{r}")).collect()),
        notice: Some(DISCLAIMER.to_string()),
    })
}

// "Flood Warning: River Exe at Winsford" as "Flood warning: River Exe at
// Winsford": the kind in sentence case, the place as written.
fn flood_agency_event(event: &str) -> String {
    match event.split_once(':') {
        Some((kind, place)) => format!("{}:{place}", crate::official::sentence_case(kind)),
        None => event.trim().to_string(),
    }
}
