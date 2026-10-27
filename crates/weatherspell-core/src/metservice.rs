// Alerts for New Zealand from MetService's public CAP feed (CC BY 4.0, as
// the feed itself says): an RSS list of the watches and warnings in
// effect, each linking to its CAP file, which carries the area's outline.
// A CAP file never changes once issued, so each is fetched once and kept
// (fetch.rs).
//
// MetService's system is colour-coded, like Environment Canada's since
// 2019: yellow watches, orange and red warnings, the colour in the
// warning's name ("Orange Heavy Rain Warning"). Its CAP headline puts the
// colour last ("Heavy Rain Warning - Orange"), so the line reads it first,
// "Orange heavy rain warning", as Canada's do; a watch keeps its name. The
// feed rates an orange warning Moderate and a watch Minor; the colour and
// kind set the severity instead, as for Environment Canada, so that
// "severe and extreme only" means the same in every country.

use std::sync::LazyLock;

use jiff::Timestamp;
use regex::Regex;

use crate::alerts::WeatherAlert;
use crate::cap::{self, CapAlert};
use crate::environment_canada as ec;
use crate::location::Location;
use crate::official::{self, OfficialError};

pub const FEED_URL: &str = "https://alerts.metservice.com/cap/rss";
pub const ALERTS_ATTRIBUTION: &str = "MetService (metservice.com), licensed CC BY 4.0";
const SOURCE_NAME: &str = "MetService";
const WARNINGS_PAGE: &str = "https://www.metservice.com/warnings/home";

pub fn covers(location: &Location) -> bool {
    location.country.as_deref() == Some("New Zealand")
}

// The CAP files the feed lists, in its order.
pub fn parse_feed(xml: &str) -> Result<Vec<String>, OfficialError> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| {
        OfficialError::Unreadable(format!("MetService's alert list could not be read: {e}."))
    })?;
    Ok(doc
        .descendants()
        .filter(|n| n.has_tag_name("item"))
        .filter_map(|item| item.children().find(|c| c.has_tag_name("link")))
        .filter_map(|link| link.text())
        .map(|link| link.trim().to_string())
        .filter(|link| link.starts_with("https://"))
        .collect())
}

pub fn alert_for(alert: &CapAlert, location: &Location, now: Timestamp) -> Option<WeatherAlert> {
    if !alert.is_live() {
        return None;
    }
    let info = alert.english()?;
    if info.is_all_clear() || info.has_ended(now) {
        return None;
    }
    let areas = info.areas_covering(location.latitude, location.longitude);
    if areas.is_empty() {
        return None;
    }
    let colour = info.parameter("ColourCode");
    let (event, kind) = name(&info.headline, &info.event, colour);
    let issued = alert.sent.or(info.effective).or(info.onset)?;
    let mut area_names: Vec<&str> = Vec::new();
    for area in areas {
        let name = area.description.trim();
        if !name.is_empty() && !area_names.contains(&name) {
            area_names.push(name);
        }
    }
    Some(WeatherAlert {
        id: format!("nz:{}", alert.identifier),
        severity: ec::alert_severity(colour, Some(&kind)),
        event,
        issued,
        onset: info.onset.or(info.effective),
        ends: info.expires,
        source: SOURCE_NAME.to_string(),
        sender: SOURCE_NAME.to_string(),
        area: area_names.join("; "),
        level: None,
        description: labelled_paragraphs(&cap::readable(&info.description)),
        instruction: Some(cap::readable(
            info.instruction
                .trim()
                .strip_prefix("Action:")
                .unwrap_or(&info.instruction)
                .trim(),
        ))
        .filter(|i| !i.is_empty()),
        url: Some(
            if info.web.starts_with("http") {
                info.web.trim()
            } else {
                WARNINGS_PAGE
            }
            .to_string(),
        ),
        expires: info.expires,
        references: Some(alert.references.iter().map(|r| format!("nz:{r}")).collect()),
        notice: None,
    })
}

// ("Orange heavy rain warning", "warning") from "Heavy Rain Warning -
// Orange"; ("Strong wind watch", "watch") from "Strong Wind Watch".
fn name(headline: &str, event: &str, colour: Option<&str>) -> (String, String) {
    let headline = headline.trim();
    let base = if headline.is_empty() { event } else { headline };
    let (base, named_colour) = match base.rsplit_once(" - ") {
        Some((name, colour)) if is_colour(colour) => (name.trim(), Some(colour.trim())),
        _ => (base, None),
    };
    let kind = base.rsplit(' ').next().unwrap_or_default().to_lowercase();
    let colour = named_colour.or(colour.filter(|_| kind == "warning"));
    let name = match colour {
        Some(colour) if !base.to_lowercase().starts_with(&colour.to_lowercase()) => {
            format!("{} {}", colour.trim(), base)
        }
        _ => base.to_string(),
    };
    (official::sentence_case(&name), kind)
}

fn is_colour(word: &str) -> bool {
    matches!(
        word.trim().to_lowercase().as_str(),
        "yellow" | "orange" | "red"
    )
}

// MetService runs its labelled parts together ("... possible. Impact:
// Streams and rivers may rise rapidly. Chance of upgrading to a Red
// Warning: Low."); each becomes a paragraph of its own.
fn labelled_paragraphs(text: &str) -> String {
    static LABEL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\s+(Impact:|Chance of upgrading|Changes:|Next update:)").unwrap()
    });
    LABEL.replace_all(text.trim(), "\n\n$1").into_owned()
}
