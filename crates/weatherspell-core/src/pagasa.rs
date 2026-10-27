// Alerts for the Philippines from PAGASA's public CAP feed (CC BY 4.0, as
// the feed says): an Atom list of its recent messages, each a CAP file
// that never changes once issued, so each is fetched once (fetch.rs).
// Three kinds: general flood advisories, with each province's outline;
// tropical cyclone alerts, with the outline of the Philippine Area of
// Responsibility; and tropical cyclone wind signals, which name their
// provinces but carry no outlines. For those the app carries the
// provinces' outlines (data/philippine-provinces.txt, simplified from
// geoBoundaries' copy of NAMRIA and PSA's boundaries, CC BY 3.0 IGO) and
// matches by name.
//
// A newer message names the ones it replaces; those, an all clear
// ("Final") and an ended one are left out. A bulletin's wind signals come
// as a file per signal level, and the next bulletin's name none of the
// last's, so for each storm only its latest bulletin's signals count: a
// province lowered from signal 2 to 1 hears 1, not both.
//
// PAGASA rates signals 1 and 2 Minor and Moderate; the signal sets the
// severity, 3 Severe and 4 and 5 Extreme, so that "severe and extreme
// only" hears signal 3 and up. Its other messages keep its own severity.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock};

use jiff::{SignedDuration, Timestamp};

use crate::alerts::{AlertSeverity, WeatherAlert};
use crate::area::{self, Region};
use crate::cap::{self, CapAlert, CapArea, CapInfo};
use crate::location::Location;
use crate::meteoalarm::fold;
use crate::official::{self, OfficialError};

const HOST: &str = "publicalert.pagasa.dost.gov.ph";
pub const FEED_URL: &str = "https://publicalert.pagasa.dost.gov.ph/feeds/";
pub const ALERTS_ATTRIBUTION: &str = "PAGASA (pagasa.dost.gov.ph), licensed CC BY 4.0";
const SOURCE_NAME: &str = "PAGASA";
const SITE: &str = "https://www.pagasa.dost.gov.ph";
const SIGNAL: &str = "Public Storm Warning Signal Level";

// As for the other simplified tables.
const TABLE_MARGIN_KM: f64 = 0.75;

// A bulletin's files are sent together; the leeway allows for one sent a
// little apart.
const BULLETIN: SignedDuration = SignedDuration::from_mins(10);

static PROVINCES: LazyLock<Vec<Region>> =
    LazyLock::new(|| area::parse_table(include_str!("../data/philippine-provinces.txt")));

pub fn covers(location: &Location) -> bool {
    location.country.as_deref() == Some("Philippines")
}

// The CAP files the feed lists, newest first, as it orders them.
pub fn parse_feed(xml: &str) -> Result<Vec<String>, OfficialError> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| {
        OfficialError::Unreadable(format!("PAGASA's alert list could not be read: {e}."))
    })?;
    let mut links = Vec::new();
    for link in doc
        .descendants()
        .filter(|n| n.has_tag_name("link"))
        .filter(|n| n.attribute("type") == Some("application/cap+xml"))
        .filter_map(|n| n.attribute("href"))
    {
        let link = link.trim().replacen("http://", "https://", 1);
        // Some entries name the server by its address, which its
        // certificate does not cover; the same files are under its name.
        let link = match link
            .strip_prefix("https://")
            .and_then(|r| r.split_once('/'))
        {
            Some((host, path)) if host.chars().all(|c| c.is_ascii_digit() || c == '.') => {
                format!("https://{HOST}/{path}")
            }
            _ => link,
        };
        if link.starts_with("https://") && !links.contains(&link) {
            links.push(link);
        }
    }
    Ok(links)
}

// The province's name as the table and PAGASA's messages both reduce to:
// "Lanao Del Sur" and "Lanao del Sur" are one. The names PAGASA has used
// besides the table's own are folded onto it.
fn province_key(name: &str) -> String {
    let key = fold(name);
    let key = key.strip_prefix("provinceof").unwrap_or(&key);
    match key {
        "ncr" | "nationalcapitalregion" => "metromanila",
        "compostelavalley" => "davaodeoro",
        "maguindanaodelnorte" | "maguindanaodelsur" => "maguindanao",
        "northcotabato" => "cotabato",
        "westernsamar" => "samar",
        other => other,
    }
    .to_string()
}

pub fn provinces_at(latitude: f64, longitude: f64) -> Vec<&'static Region> {
    PROVINCES
        .iter()
        .filter(|p| p.covers(latitude, longitude, TABLE_MARGIN_KM))
        .collect()
}

// The provinces a wind signal names: "Cagayan, Ilocos Norte, Apayao", or
// a part of one ("the northern portion of Cagayan (Santa Ana)"), which
// counts as the whole province.
fn named_provinces(area: &CapArea) -> Vec<String> {
    area.description
        .split([',', ';'])
        .flat_map(|part| part.split(" and "))
        .map(|part| {
            let part = part.split('(').next().unwrap_or(part).trim();
            let lower = part.to_lowercase();
            match lower.find(" of ") {
                Some(at) if lower[..at].contains("portion") || lower[..at].contains("rest") => {
                    &part[at + 4..]
                }
                _ => part,
            }
            .to_string()
        })
        .map(|part| province_key(&part))
        .filter(|key| !key.is_empty())
        .collect()
}

fn signal_level(info: &CapInfo) -> Option<u32> {
    info.parameter(SIGNAL).and_then(|v| v.parse().ok())
}

// "Tropical Cyclone Warning : Typhoon Rosal (CHAMPI) Signal #1": the storm.
fn storm(info: &CapInfo) -> String {
    let headline = info.headline.trim();
    let after = headline.split_once(':').map_or(headline, |(_, s)| s);
    after
        .split(" Signal")
        .next()
        .unwrap_or(after)
        .trim()
        .to_string()
}

pub fn alerts_for(
    alerts: &[Arc<CapAlert>],
    location: &Location,
    now: Timestamp,
) -> Vec<WeatherAlert> {
    let replaced: HashSet<&str> = alerts
        .iter()
        .flat_map(|a| a.references.iter())
        .map(String::as_str)
        .collect();
    let mut latest: HashMap<String, Timestamp> = HashMap::new();
    for a in alerts {
        if let (Some(info), Some(sent)) = (a.english(), a.sent)
            && signal_level(info).is_some()
        {
            let at = latest.entry(storm(info)).or_insert(sent);
            *at = (*at).max(sent);
        }
    }
    let here: HashSet<String> = provinces_at(location.latitude, location.longitude)
        .iter()
        .map(|p| province_key(&p.name))
        .collect();
    let mut found = Vec::new();
    for a in alerts {
        if !a.is_live() || replaced.contains(a.identifier.as_str()) {
            continue;
        }
        let Some(info) = a.english() else {
            continue;
        };
        if info.is_all_clear() || info.has_ended(now) {
            continue;
        }
        let signal = signal_level(info);
        if signal.is_some()
            && let (Some(sent), Some(newest)) = (a.sent, latest.get(&storm(info)))
            && newest.duration_since(sent) > BULLETIN
        {
            continue;
        }
        let areas: Vec<&CapArea> = info
            .areas
            .iter()
            .filter(|area| {
                if area.polygons.is_empty() {
                    named_provinces(area).iter().any(|p| here.contains(p))
                } else {
                    area.polygons
                        .iter()
                        .any(|p| p.covers(location.latitude, location.longitude, 0.0))
                }
            })
            .collect();
        if areas.is_empty() {
            continue;
        }
        if let Some(alert) = weather_alert(a, info, signal, &areas) {
            found.push(alert);
        }
    }
    found
}

fn weather_alert(
    a: &CapAlert,
    info: &CapInfo,
    signal: Option<u32>,
    areas: &[&CapArea],
) -> Option<WeatherAlert> {
    let issued = a.sent.or(info.effective)?;
    let (event, severity) = match signal {
        Some(n) => (
            format!("Tropical cyclone wind signal number {n}"),
            match n {
                1 => AlertSeverity::Minor,
                2 => AlertSeverity::Moderate,
                3 => AlertSeverity::Severe,
                _ => AlertSeverity::Extreme,
            },
        ),
        None => (
            official::sentence_case(&info.event),
            match info.severity.trim().to_ascii_lowercase().as_str() {
                "extreme" => AlertSeverity::Extreme,
                "severe" => AlertSeverity::Severe,
                "moderate" => AlertSeverity::Moderate,
                "minor" => AlertSeverity::Minor,
                _ => AlertSeverity::Unknown,
            },
        ),
    };
    let mut description = Vec::new();
    let headline = info.headline.trim();
    if !headline.is_empty() && !headline.eq_ignore_ascii_case(&event) {
        description.push(cap::readable(headline));
    }
    if !info.description.trim().is_empty() {
        description.push(cap::readable(info.description.trim()));
    }
    let mut names: Vec<&str> = Vec::new();
    for name in areas.iter().map(|a| a.description.trim()) {
        if !name.is_empty() && !names.contains(&name) {
            names.push(name);
        }
    }
    let web = info.web.trim();
    let url = match web.strip_prefix("http://") {
        Some(rest) => format!("https://{rest}"),
        None if web.starts_with("https://") => web.to_string(),
        None => SITE.to_string(),
    };
    // A storm's signal keeps one id from bulletin to bulletin, so it is
    // heard again only when raised (alerts::track); the others follow
    // their messages' references.
    let (id, references) = match signal {
        Some(_) => (format!("ph:{}/signal", storm(info)), Vec::new()),
        None => (
            format!("ph:{}", a.identifier),
            a.references.iter().map(|r| format!("ph:{r}")).collect(),
        ),
    };
    Some(WeatherAlert {
        id,
        event,
        severity,
        issued,
        onset: info.onset,
        ends: None,
        source: SOURCE_NAME.to_string(),
        sender: SOURCE_NAME.to_string(),
        area: names.join("; "),
        level: None,
        description: description.join("\n\n"),
        instruction: Some(cap::readable(info.instruction.trim())).filter(|i| !i.is_empty()),
        url: Some(url),
        expires: info.expires,
        references: Some(references),
        notice: None,
    })
}
