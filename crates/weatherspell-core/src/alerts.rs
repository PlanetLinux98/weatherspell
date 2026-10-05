// Weather alerts, source-neutral: what the NWS and Environment Canada
// parsers produce (nws.rs, environment_canada.rs), how they are ordered and
// tracked, and the words for them: the Alerts section, the spoken
// announcement of new alerts, and the text of the details dialog, one line
// per alert, event first, in the location's own time, with the service's
// own words for the details.

use jiff::Timestamp;
use jiff::civil::DateTime;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::clock::{self, Clock};
use crate::official;
use crate::writer::Section;

pub const HEADING: &str = "Alerts";
pub const NOT_AVAILABLE_LINE: &str = "Alerts are not available for this region.";
pub const NONE_LINE: &str = "No alerts in effect.";

// The services' own scales, aligned: the NWS says Extreme, Severe,
// Moderate, Minor, Unknown; Environment Canada's colour levels and alert
// types map onto the same words (environment_canada::alert_severity). The
// announcement threshold setting and the ordering both read this.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    Unknown,
    Minor,
    Moderate,
    Severe,
    Extreme,
}

// In a cache file by name, as 0.1 writes it; a name neither app knows is
// read as Unknown, as 0.1's Enum.TryParse does.
impl Serialize for AlertSeverity {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{self:?}"))
    }
}

impl<'de> Deserialize<'de> for AlertSeverity {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<AlertSeverity, D::Error> {
        Ok(match Option::<String>::deserialize(d)?.as_deref() {
            Some("Minor") => AlertSeverity::Minor,
            Some("Moderate") => AlertSeverity::Moderate,
            Some("Severe") => AlertSeverity::Severe,
            Some("Extreme") => AlertSeverity::Extreme,
            _ => AlertSeverity::Unknown,
        })
    }
}

// From the last word of the event name, which both services use
// consistently ("Flash flood watch", "special weather statement"); an
// ordering tie-break within one severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertKind {
    Warning,
    Watch,
    Advisory,
    Statement,
    Other,
}

// One alert in effect for a location. The id is stable across a service's
// updates of the same alert, so each alert is announced once (each parser
// says how). Source is the phrase that follows "from" in a sentence
// ("Environment Canada", "the National Weather Service"); sender is the
// issuing office as the service names it. Ends is when the hazard is
// expected to end, and only that: expires is when this message runs out,
// which for a hurricane watch is when the next update is due, not when the
// watch ends, so it is never read as "until" (#20).
//
// The serde names are the cache file's, shared with 0.1 (see forecast.rs).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeatherAlert {
    pub id: String,
    pub event: String,
    pub severity: AlertSeverity,
    #[serde(with = "crate::iso::stamp")]
    pub issued: Timestamp,
    #[serde(default, with = "crate::iso::stamp_opt")]
    pub onset: Option<Timestamp>,
    #[serde(default, with = "crate::iso::stamp_opt")]
    pub ends: Option<Timestamp>,
    pub source: String,
    pub sender: String,
    pub area: String,
    pub level: Option<String>,
    pub description: String,
    pub instruction: Option<String>,
    pub url: Option<String>,
    #[serde(default, with = "crate::iso::stamp_opt")]
    pub expires: Option<Timestamp>,
}

impl WeatherAlert {
    pub fn kind(&self) -> AlertKind {
        match self
            .event
            .rsplit(' ')
            .next()
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            "warning" => AlertKind::Warning,
            "watch" => AlertKind::Watch,
            "advisory" => AlertKind::Advisory,
            "statement" => AlertKind::Statement,
            _ => AlertKind::Other,
        }
    }
}

// What one check for a location produced. Attribution is None where no
// source covers the region; problem is set when the source could not be
// reached or read, in which case alerts is empty and must not be read as
// "none in effect", unless checked_at is also set: then the alerts are the
// last ones known, from a check at that time, kept so an outage does not
// hide a warning that may still be in effect (or_last_known).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AlertReport {
    pub alerts: Vec<WeatherAlert>,
    pub attribution: Option<String>,
    pub problem: Option<String>,
    pub checked_at: Option<Timestamp>,
}

impl AlertReport {
    pub fn not_available() -> AlertReport {
        AlertReport::default()
    }

    pub fn is_available(&self) -> bool {
        self.attribution.is_some()
    }

    pub fn checked(&self) -> bool {
        self.attribution.is_some() && self.problem.is_none()
    }

    // This check's failure carrying an earlier report's alerts, dated by
    // that report: what is shown while the service is unreachable. A check
    // that succeeded, a region no source covers, and an earlier report that
    // knew nothing all give this report unchanged. The writer drops alerts
    // whose end has passed when it reads a dated report.
    pub fn or_last_known(&self, earlier: Option<&AlertReport>) -> AlertReport {
        match earlier {
            Some(e) if !self.checked() && self.is_available() && e.checked_at.is_some() => {
                AlertReport {
                    alerts: e.alerts.clone(),
                    checked_at: e.checked_at,
                    ..self.clone()
                }
            }
            _ => self.clone(),
        }
    }
}

// Severity first; within it warnings before watches before advisories and
// statements; then whichever ends soonest, or for one with no end (a
// hurricane watch) is next updated, so it does not sink below a flood
// watch.
pub fn order(mut alerts: Vec<WeatherAlert>) -> Vec<WeatherAlert> {
    alerts.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then(a.kind().cmp(&b.kind()))
            .then(
                a.ends
                    .or(a.expires)
                    .unwrap_or(Timestamp::MAX)
                    .cmp(&b.ends.or(b.expires).unwrap_or(Timestamp::MAX)),
            )
            .then(a.event.cmp(&b.event))
    });
    alerts
}

// Which of a location's alerts are new since the last check, given the ids
// seen before. The seen list is replaced with the current ids, so an alert
// that ends and is later issued afresh under a new id counts as new again,
// and the list never grows past what is in effect. Callers keep the list
// per saved location and persist it, so a relaunch during a long-running
// alert does not announce it a second time.
pub fn track(seen: &mut Vec<String>, current: &[WeatherAlert]) -> Vec<WeatherAlert> {
    let fresh = current
        .iter()
        .filter(|a| !seen.contains(&a.id))
        .cloned()
        .collect();
    seen.clear();
    for a in current {
        if !seen.contains(&a.id) {
            seen.push(a.id.clone());
        }
    }
    fresh
}

// None means no check was made; it reads the same as an uncovered region
// rather than as a quiet day.
pub fn section(report: Option<&AlertReport>, clock: &Clock, now_local: DateTime) -> Section {
    let Some(report) = report.filter(|r| r.is_available()) else {
        return Section::new(HEADING, vec![NOT_AVAILABLE_LINE.to_string()]);
    };
    if let Some(problem) = &report.problem {
        let Some(checked_at) = report.checked_at else {
            return Section::new(
                HEADING,
                vec![format!(
                    "Alerts couldn't be checked this time ({problem}). Press F5 to try again."
                )],
            );
        };
        // The last check's alerts, dated before any of them is read, and
        // without those whose end has passed: "until" would read as still in
        // effect. An alert with no end is kept.
        let age = clock::age_after_from(now_local.duration_since(clock.local(checked_at)));
        let alerts: Vec<&WeatherAlert> = report
            .alerts
            .iter()
            .filter(|a| a.ends.is_none_or(|ends| clock.local(ends) > now_local))
            .collect();
        if alerts.is_empty() {
            return Section::new(
                HEADING,
                vec![format!(
                    "Alerts couldn't be checked this time ({problem}); none were in effect when last checked, {age}. Press F5 to try again."
                )],
            );
        }
        let mut paragraphs = vec![format!(
            "Alerts couldn't be checked this time ({problem}); showing the alerts from {age}. Press F5 to try again."
        )];
        let mut lines = vec![None];
        for a in alerts {
            paragraphs.push(line(a, clock, now_local));
            lines.push(Some(a.clone()));
        }
        return Section {
            heading: HEADING.to_string(),
            paragraphs,
            alerts: Some(lines),
        };
    }
    if report.alerts.is_empty() {
        return Section::new(HEADING, vec![NONE_LINE.to_string()]);
    }
    Section {
        heading: HEADING.to_string(),
        paragraphs: report
            .alerts
            .iter()
            .map(|a| line(a, clock, now_local))
            .collect(),
        alerts: Some(report.alerts.iter().cloned().map(Some).collect()),
    }
}

pub fn line(a: &WeatherAlert, clock: &Clock, now_local: DateTime) -> String {
    match a.ends {
        Some(ends) => format!(
            "{} until {}, from {}. Press Enter for details.",
            a.event,
            clock.time_on_day(clock.local(ends), now_local),
            a.source
        ),
        None => format!("{} from {}. Press Enter for details.", a.event, a.source),
    }
}

// "Peterborough: severe thunderstorm warning until 6:00 pm today."
pub fn announcement(
    location: &str,
    alerts: &[WeatherAlert],
    clock: &Clock,
    now_local: DateTime,
) -> String {
    let parts: Vec<String> = alerts
        .iter()
        .map(|a| match a.ends {
            Some(ends) => format!(
                "{} until {}",
                lower_first(&a.event),
                clock.time_on_day(clock.local(ends), now_local)
            ),
            None => lower_first(&a.event),
        })
        .collect();
    format!("{location}: {}.", join_and(&parts))
}

// "frost advisory and special weather statement in effect": what a launch
// or a switch says after "forecast ready". The text opens with the Alerts
// section, but nothing reads it out unless the user does, and those alerts
// are marked seen, so this is where they are heard. Each kind once (a
// service may issue one per area); None when the check found none or
// failed, as the Alerts section then says why itself.
pub fn in_effect(report: &AlertReport) -> Option<String> {
    if !report.checked() || report.alerts.is_empty() {
        return None;
    }
    let mut kinds: Vec<String> = Vec::new();
    for a in &report.alerts {
        let kind = lower_first(&a.event);
        if !kinds
            .iter()
            .any(|k| k.to_lowercase() == kind.to_lowercase())
        {
            kinds.push(kind);
        }
    }
    Some(format!("{} in effect", join_and(&kinds)))
}

pub fn details(a: &WeatherAlert, clock: &Clock, now_local: DateTime) -> Vec<String> {
    let when = |t: Timestamp| clock.time_on_day(clock.local(t), now_local);
    let mut paragraphs = Vec::new();
    let timing = a.ends.map(|ends| match a.onset {
        Some(onset) if clock.local(onset) > now_local => {
            format!("from {} until {}", when(onset), when(ends))
        }
        _ => format!("until {}", when(ends)),
    });
    paragraphs.push(match timing {
        Some(timing) => format!("{} from {}, in effect {timing}.", a.event, a.sender),
        None => format!("{} from {}.", a.event, a.sender),
    });
    // No end given: say when this message runs out, as that, so that
    // "until" is never heard for a hurricane watch's next update (#20).
    if a.ends.is_none()
        && let Some(expires) = a.expires
    {
        paragraphs.push(format!(
            "No end time is given; this message expires at {}.",
            when(expires)
        ));
    }
    if !a.area.is_empty() {
        paragraphs.push(format!("Area: {}.", a.area));
    }
    paragraphs.push(format!("Issued {}.", when(a.issued)));
    if let Some(level) = &a.level {
        paragraphs.push(format!("{level}."));
    }
    paragraphs.extend(paragraphs_of(&a.description));
    if let Some(instruction) = &a.instruction {
        let mut instructions = paragraphs_of(instruction);
        if let Some(first) = instructions.first_mut() {
            *first = format!("Instructions: {first}");
            paragraphs.extend(instructions);
        }
    }
    paragraphs
}

// The service's text in paragraphs: blank lines separate them, single line
// breaks are the NWS's hard wrapping, "###" is EC's rule between the alert
// and its boilerplate, "- " starts one of the NWS's sub-items, and "*
// WHAT..." bullets become "What:" so a listener hears the label rather
// than "star" and "dot dot dot".
pub fn paragraphs_of(text: &str) -> Vec<String> {
    let text = text.replace("\r\n", "\n");
    let mut list = Vec::new();
    // Blocks: runs of lines between whitespace-only lines (\n\s*\n).
    let mut blocks: Vec<Vec<&str>> = vec![Vec::new()];
    for line in text.split('\n') {
        if line.trim().is_empty() {
            if !blocks.last().is_some_and(Vec::is_empty) {
                blocks.push(Vec::new());
            }
        } else {
            blocks
                .last_mut()
                .expect("there is always a block")
                .push(line);
        }
    }
    for block in blocks {
        // Items: a line starting "- " begins a new one (\n(?=- )).
        let mut items: Vec<Vec<&str>> = Vec::new();
        for line in block {
            if items.is_empty() || line.starts_with("- ") {
                items.push(vec![line]);
            } else {
                items.last_mut().expect("an item was started").push(line);
            }
        }
        for item in items {
            let joined = item.iter().map(|l| l.trim()).collect::<Vec<_>>().join(" ");
            let mut s = official::collapse_whitespace_runs(&joined);
            if let Some(rest) = s.strip_prefix("- ") {
                s = rest.to_string();
            }
            if s.is_empty() || s.chars().all(|c| c == '#') {
                continue;
            }
            list.push(official::spoken(&bullet_label(&s)));
        }
    }
    list
}

// Regex.Replace(s, @"^\*\s*([A-Z][A-Z ]*[A-Z])\.\.\.\s*", label + ": ").
fn bullet_label(s: &str) -> String {
    let Some(rest) = s.strip_prefix('*') else {
        return s.to_string();
    };
    let rest = rest.trim_start();
    let run = rest
        .find(|c: char| !(c.is_ascii_uppercase() || c == ' '))
        .unwrap_or(rest.len());
    let label = &rest[..run];
    let after = &rest[run..];
    let shaped = label.len() >= 2
        && label.starts_with(|c: char| c.is_ascii_uppercase())
        && label.ends_with(|c: char| c.is_ascii_uppercase());
    match after.strip_prefix("...") {
        Some(text) if shaped => {
            format!("{}: {}", official::sentence_case(label), text.trim_start())
        }
        _ => s.to_string(),
    }
}

fn lower_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn join_and(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}
