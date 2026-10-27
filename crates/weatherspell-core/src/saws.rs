// Alerts for South Africa from the South African Weather Service's CAP
// feed (public domain, as the feed says): an RSS list of its warnings'
// CAP files, each with the outlines of the municipalities it covers and
// an info block per period. The list keeps every warning since May 2025,
// so only those published in the last few days are read, and of those
// only the newest file for each office and kind
// ("KZN_CAP_DamagingWinds_20261019_091747.xml"): a later file replaces
// an earlier one, though it names no references. A file never changes
// once published, so each is fetched once (fetch.rs). The feed's links say
// http, which does not answer; https does. Its server leaves out its
// intermediate certificate: Windows and the Mac fetch it themselves.
//
// SAWS's impact-based warnings are colour-coded, with a level from 1 to
// 10 for impact and likelihood together: 1 to 4 yellow, 5 to 8 orange, 9
// and 10 red. Its CAP rates every one Minor and heads it with the hazard
// alone ("Damaging Winds"), so the line reads as SAWS announces them,
// "Yellow level 2 warning for damaging winds", and the colour sets the
// severity, as for the other colour-coded services. Warning periods end
// when the message expires.

use jiff::{SignedDuration, Timestamp};

use crate::alerts::{AlertSeverity, WeatherAlert};
use crate::cap::{self, CapAlert, CapArea};
use crate::location::Location;
use crate::official::OfficialError;

pub const FEED_URL: &str = "https://caps.weathersa.co.za/Home/RssFeed";
pub const ALERTS_ATTRIBUTION: &str = "South African Weather Service (weathersa.co.za)";
const SOURCE_NAME: &str = "the South African Weather Service";
const SITE: &str = "https://www.weathersa.co.za";

// Its warnings run a day or two.
const RECENT: SignedDuration = SignedDuration::from_hours(72);

pub fn covers(location: &Location) -> bool {
    location.country.as_deref() == Some("South Africa")
}

// A CAP file the feed lists, with its office and kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    pub url: String,
    pub group: String,
    published: Timestamp,
}

pub fn parse_feed(xml: &str, now: Timestamp) -> Result<Vec<Listed>, OfficialError> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| {
        OfficialError::Unreadable(format!(
            "the South African Weather Service's alert list could not be read: {e}."
        ))
    })?;
    let mut newest: Vec<Listed> = Vec::new();
    for item in doc.descendants().filter(|n| n.has_tag_name("item")) {
        let field = |name: &str| {
            item.children()
                .find(|c| c.has_tag_name(name))
                .and_then(|c| c.text())
                .map(str::trim)
        };
        let (Some(link), Some(published)) = (field("link"), field("pubDate")) else {
            continue;
        };
        let Ok(published) = jiff::fmt::rfc2822::parse(published).map(|z| z.timestamp()) else {
            continue;
        };
        if now.duration_since(published) > RECENT {
            continue;
        }
        let url = match link.strip_prefix("http://") {
            Some(rest) => format!("https://{rest}"),
            None => link.to_string(),
        };
        // "KZN_CAP_DamagingWinds_20261019_091747.xml": the office and the
        // kind, without the date and time.
        let file = url.rsplit('/').next().unwrap_or_default();
        let parts: Vec<&str> = file.trim_end_matches(".xml").split('_').collect();
        let group = parts[..parts.len().saturating_sub(2).max(1)].join("_");
        let listed = Listed {
            url,
            group,
            published,
        };
        match newest.iter_mut().find(|l| l.group == listed.group) {
            Some(had) if (listed.published, &listed.url) > (had.published, &had.url) => {
                *had = listed;
            }
            Some(_) => {}
            None => newest.push(listed),
        }
    }
    Ok(newest)
}

pub fn alerts_for(
    alert: &CapAlert,
    listed: &Listed,
    location: &Location,
    now: Timestamp,
) -> Vec<WeatherAlert> {
    if !alert.is_live() {
        return Vec::new();
    }
    let mut alerts: Vec<WeatherAlert> = Vec::new();
    for info in &alert.infos {
        if info.is_all_clear() || info.has_ended(now) {
            continue;
        }
        let areas: Vec<&CapArea> = info.areas_covering(location.latitude, location.longitude);
        let Some(first) = areas.first() else {
            continue;
        };
        let Some(issued) = alert.sent.or(info.effective).or(info.onset) else {
            continue;
        };
        let level: Option<u32> = info.parameter("alertType").and_then(|t| {
            t.rsplit(' ')
                .next()
                .and_then(|n| n.parse().ok())
                .filter(|n| (1..=10).contains(n))
        });
        let (colour, severity) = match level {
            Some(1..=4) => ("Yellow", AlertSeverity::Moderate),
            Some(5..=8) => ("Orange", AlertSeverity::Severe),
            Some(_) => ("Red", AlertSeverity::Extreme),
            None => ("", AlertSeverity::Moderate),
        };
        let hazard = Some(info.event.trim())
            .filter(|e| !e.is_empty())
            .unwrap_or(info.headline.trim())
            .to_lowercase();
        let event = match level {
            Some(n) => format!("{colour} level {n} warning for {hazard}"),
            None => crate::official::sentence_case(&format!("warning for {hazard}")),
        };
        // One id for a warning through the files that replace it, so it is
        // heard again only if raised (alerts::track).
        let base = format!("za:{}/{}", listed.group, first.description.trim());
        let mut id = base.clone();
        let mut n = 1;
        while alerts.iter().any(|a| a.id == id) {
            n += 1;
            id = format!("{base}/{n}");
        }
        let names: Vec<&str> = areas.iter().map(|a| a.description.trim()).collect();
        let web = info.web.trim();
        alerts.push(WeatherAlert {
            id,
            event,
            severity,
            issued,
            onset: info.onset,
            ends: info.expires,
            source: SOURCE_NAME.to_string(),
            sender: SOURCE_NAME.to_string(),
            area: names.join("; "),
            level: None,
            description: cap::readable(&items(&info.description)),
            instruction: Some(cap::readable(&items(&info.instruction))).filter(|i| !i.is_empty()),
            url: Some(
                if web.starts_with("https://") {
                    web
                } else {
                    SITE
                }
                .to_string(),
            ),
            expires: info.expires,
            references: Some(Vec::new()),
            notice: None,
        });
    }
    alerts
}

// SAWS lists impacts and advice a line each, some ending in a comma:
// items, so each is read as one.
fn items(text: &str) -> String {
    text.lines()
        .map(|line| line.trim().trim_end_matches(',').trim())
        .filter(|line| !line.is_empty())
        .map(|line| format!("- {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}
