// The Common Alerting Protocol (CAP 1.2), in which most alerting services
// outside North America publish: MeteoAlarm's feeds carry its fields as
// JSON (meteoalarm.rs); MetService, the Bureau of Meteorology, PAGASA and
// SAWS as XML (metservice.rs, bom.rs, pagasa.rs, saws.rs). Only what the
// app reads is kept. Each source decides which messages and areas apply
// and how an alert is named; what they share is here: the English block,
// the chain of updates, and text made readable aloud.

use std::sync::LazyLock;

use jiff::Timestamp;
use regex::Regex;

use crate::area::Polygon;
use crate::official::OfficialError;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CapAlert {
    pub identifier: String,
    pub sent: Option<Timestamp>,
    pub status: String,
    pub msg_type: String,
    // The identifiers of the messages this one updates or cancels.
    pub references: Vec<String>,
    pub infos: Vec<CapInfo>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CapInfo {
    pub language: String,
    pub event: String,
    pub headline: String,
    pub description: String,
    pub instruction: String,
    pub sender_name: String,
    pub severity: String,
    pub response_types: Vec<String>,
    pub effective: Option<Timestamp>,
    pub onset: Option<Timestamp>,
    pub expires: Option<Timestamp>,
    pub web: String,
    pub parameters: Vec<(String, String)>,
    pub areas: Vec<CapArea>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CapArea {
    pub description: String,
    pub polygons: Vec<Polygon>,
    // (valueName, value): ("EMMA_ID", "DE169").
    pub geocodes: Vec<(String, String)>,
}

impl CapAlert {
    // Neither a test, an exercise nor a cancellation.
    pub fn is_live(&self) -> bool {
        self.status.eq_ignore_ascii_case("Actual") && !self.msg_type.eq_ignore_ascii_case("Cancel")
    }

    // The English block ("en", "en-GB", "en-AU"); CAP's default language is
    // English, so a block that names none counts.
    pub fn english(&self) -> Option<&CapInfo> {
        self.infos.iter().find(|i| {
            let language = i.language.trim().to_ascii_lowercase();
            language.is_empty() || language == "en" || language.starts_with("en-")
        })
    }
}

impl CapInfo {
    pub fn parameter(&self, name: &str) -> Option<&str> {
        self.parameters
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.trim())
            .filter(|v| !v.is_empty())
    }

    // An update saying the hazard has passed: not an alert in effect.
    pub fn is_all_clear(&self) -> bool {
        self.response_types
            .iter()
            .any(|r| r.eq_ignore_ascii_case("AllClear"))
    }

    pub fn has_ended(&self, now: Timestamp) -> bool {
        self.expires.is_some_and(|e| e <= now)
    }

    // The areas whose own outlines hold the point, taken as drawn.
    pub fn areas_covering(&self, latitude: f64, longitude: f64) -> Vec<&CapArea> {
        self.areas
            .iter()
            .filter(|a| {
                a.polygons
                    .iter()
                    .any(|p| p.covers(latitude, longitude, 0.0))
            })
            .collect()
    }
}

// CAP's references: space-separated "sender,identifier,sent" triples.
pub fn references(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter_map(|triple| triple.split(',').nth(1))
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn time(text: &str) -> Option<Timestamp> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.parse().ok()).flatten()
}

fn child<'a, 'i>(node: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
    node.children().find(|c| c.tag_name().name() == name)
}

fn children<'a, 'i>(
    node: roxmltree::Node<'a, 'i>,
    name: &'a str,
) -> impl Iterator<Item = roxmltree::Node<'a, 'i>> + 'a {
    node.children().filter(move |c| c.tag_name().name() == name)
}

fn text_of(node: roxmltree::Node, name: &str) -> String {
    child(node, name)
        .and_then(|c| c.text())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn pairs(node: roxmltree::Node, name: &str) -> Vec<(String, String)> {
    children(node, name)
        .map(|p| (text_of(p, "valueName"), text_of(p, "value")))
        .collect()
}

pub fn parse_xml(xml: &str) -> Result<CapAlert, OfficialError> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| OfficialError::Unreadable(format!("An alert could not be read: {e}.")))?;
    let root = doc.root_element();
    if root.tag_name().name() != "alert" {
        return Err(OfficialError::Unreadable(
            "An alert could not be read: it is not a CAP alert.".into(),
        ));
    }
    Ok(CapAlert {
        identifier: text_of(root, "identifier"),
        sent: time(&text_of(root, "sent")),
        status: text_of(root, "status"),
        msg_type: text_of(root, "msgType"),
        references: references(&text_of(root, "references")),
        infos: children(root, "info")
            .map(|info| CapInfo {
                language: text_of(info, "language"),
                event: text_of(info, "event"),
                headline: text_of(info, "headline"),
                description: text_of(info, "description"),
                instruction: text_of(info, "instruction"),
                sender_name: text_of(info, "senderName"),
                severity: text_of(info, "severity"),
                response_types: children(info, "responseType")
                    .filter_map(|r| r.text())
                    .map(|r| r.trim().to_string())
                    .collect(),
                effective: time(&text_of(info, "effective")),
                onset: time(&text_of(info, "onset")),
                expires: time(&text_of(info, "expires")),
                web: text_of(info, "web"),
                parameters: pairs(info, "parameter"),
                areas: children(info, "area")
                    .map(|area| CapArea {
                        description: text_of(area, "areaDesc"),
                        polygons: children(area, "polygon")
                            .filter_map(|p| p.text())
                            .filter_map(Polygon::parse)
                            .collect(),
                        geocodes: pairs(area, "geocode"),
                    })
                    .collect(),
            })
            .collect(),
    })
}

// A service's text made readable aloud, before the paragraph pass every
// alert gets (alerts::paragraphs_of): HTML (Met Eireann sends paragraphs
// and bulleted lists) becomes paragraphs and "- " items, and units written
// as symbols become words, as "km/h" and "%" already do. Nothing else is
// reworded: MeteoAlarm's terms ask that the text not be changed in
// substance.
pub fn readable(text: &str) -> String {
    units(&bullets(&plain(text)))
}

// "* Keep clear of creeks", as the Bureau of Meteorology lists its advice,
// read as the list items made from HTML are, not as "star".
fn bullets(text: &str) -> String {
    static STAR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^([ \t]*)\* ").unwrap());
    STAR.replace_all(text, "$1- ").into_owned()
}

static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<(/?)([a-z0-9]+)[^>]*>").unwrap());

fn plain(text: &str) -> String {
    if !TAG.is_match(text) {
        return text.to_string();
    }
    let text = TAG.replace_all(text, |c: &regex::Captures| {
        let closing = !c[1].is_empty();
        match (c[2].to_ascii_lowercase().as_str(), closing) {
            ("li", false) => "\n- ",
            ("p" | "ul" | "ol" | "div" | "h1" | "h2" | "h3" | "h4", _) => "\n\n",
            ("br", _) => "\n",
            _ => "",
        }
    });
    entities(&text)
}

fn entities(text: &str) -> String {
    static ENTITY: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"&(#[0-9]+|#x[0-9a-fA-F]+|[a-zA-Z]+);").unwrap());
    ENTITY
        .replace_all(text, |c: &regex::Captures| {
            let name = &c[1];
            let code = if let Some(hex) = name.strip_prefix("#x").or(name.strip_prefix("#X")) {
                u32::from_str_radix(hex, 16).ok()
            } else if let Some(decimal) = name.strip_prefix('#') {
                decimal.parse().ok()
            } else {
                match name {
                    "nbsp" => Some(' ' as u32),
                    "amp" => Some('&' as u32),
                    "lt" => Some('<' as u32),
                    "gt" => Some('>' as u32),
                    "quot" => Some('"' as u32),
                    "apos" => Some('\'' as u32),
                    _ => None,
                }
            };
            match code.and_then(char::from_u32) {
                Some('\u{a0}') => " ".to_string(),
                Some(ch) => ch.to_string(),
                None => c[0].to_string(),
            }
        })
        .into_owned()
}

// "40-80mm/24h", "5-10 cm", "> -1 °C", "gusts of 15 to 20 m/s", in the
// words official::spoken uses for "km/h". A range is read as one only
// before a unit, so dates ("2026-10-25") are left alone.
fn units(text: &str) -> String {
    static RULES: LazyLock<Vec<(Regex, &str)>> = LazyLock::new(|| {
        [
            (
                r"(\d)\s*-\s*(\d+)(\s*(?:mm|cm|m/s|km/h|m\b|°|%))",
                "$1 to $2$3",
            ),
            (r"(^|[\s(])-(\d)", "${1}minus $2"),
            (r"(^|[\s(;:])>\s*(\S)", "${1}above $2"),
            (r"(^|[\s(;:])<\s*(\S)", "${1}below $2"),
            (r"(\d)\s*mm\s*/\s*(\d+)\s*h\b", "$1 millimetres in $2 hours"),
            (r"(\d)\s*mm\s*/\s*h\b", "$1 millimetres per hour"),
            (r"(\d)\s*mm\b", "$1 millimetres"),
            (r"(\d)\s*cm\b", "$1 centimetres"),
            (r"(\d)\s*m/s\b", "$1 metres per second"),
            (r"(\d)\s*m\b", "$1 metres"),
            (r"(\d)\s*°\s*C\b", "$1 degrees"),
        ]
        .into_iter()
        .map(|(pattern, words)| (Regex::new(pattern).unwrap(), words))
        .collect()
    });
    let mut text = text.to_string();
    for (pattern, words) in RULES.iter() {
        text = pattern.replace_all(&text, *words).into_owned();
    }
    text
}
