// Environment Canada's city page XML on the MSC Datamart: the forecast
// text per period and the current conditions for a site. Files land in
// dd.weather.gc.ca/today/citypage_weather/{PROV}/{HH}/ by UTC hour of
// emission, hourly at a minimum, and only today's tree is kept, so the
// latest file is found by listing the current hour and walking back
// (fetch.rs). The site list gives each of about 850 sites' coordinates; a
// location gets the nearest site's page.

use jiff::Timestamp;
use jiff::civil::{Date, DateTime, Weekday};

use crate::alerts::{AlertSeverity, WeatherAlert};
use crate::forecast::OfficialPeriod;
use crate::official::{self, Observation, OfficialError, OfficialForecast};

pub const SOURCE_NAME: &str = "Environment Canada";
// Its licence asks for the full name in the credit ("Data Source:
// Environment and Climate Change Canada"); speech and the status bar keep
// the name people say.
pub const FULL_NAME: &str = "Environment and Climate Change Canada";
pub const BASE: &str = "https://dd.weather.gc.ca/today/citypage_weather";
pub const SITE_LIST_URL: &str =
    "https://dd.weather.gc.ca/today/citypage_weather/docs/site_list_en.csv";

// A site further than this is not this location's forecast.
pub const MAX_SITE_DISTANCE_KM: f64 = 200.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Site {
    pub code: String,
    pub name: String,
    pub province: String,
    pub latitude: f64,
    pub longitude: f64,
}

// Two header lines ("Site Names" then the column names), then
// s0000629,Peterborough,ON,44.30N,78.33W per line.
pub fn parse_site_list(csv: &str) -> Result<Vec<Site>, OfficialError> {
    let mut sites = Vec::new();
    for raw in csv.split('\n') {
        let f: Vec<&str> = raw.trim_end_matches('\r').split(',').collect();
        if f.len() < 5 || !f[0].starts_with('s') || f[0].chars().count() != 8 {
            continue;
        }
        let (Some(latitude), Some(longitude)) =
            (coordinate(f[3], 'N', 'S'), coordinate(f[4], 'E', 'W'))
        else {
            continue;
        };
        sites.push(Site {
            code: f[0].to_string(),
            name: f[1].trim().to_string(),
            province: f[2].trim().to_uppercase(),
            latitude,
            longitude,
        });
    }
    if sites.is_empty() {
        return Err(OfficialError::Unreadable(
            "The Environment Canada site list is empty.".into(),
        ));
    }
    Ok(sites)
}

fn coordinate(text: &str, positive: char, negative: char) -> Option<f64> {
    let t = text.trim();
    let hemisphere = t.chars().last()?.to_ascii_uppercase();
    if t.chars().count() < 2 || (hemisphere != positive && hemisphere != negative) {
        return None;
    }
    let value: f64 = t[..t.len() - 1].parse().ok()?;
    Some(if hemisphere == negative {
        -value
    } else {
        value
    })
}

pub fn nearest(sites: &[Site], latitude: f64, longitude: f64) -> Option<(&Site, f64)> {
    let mut best: Option<(&Site, f64)> = None;
    for site in sites {
        let d = distance_km(latitude, longitude, site.latitude, site.longitude);
        if best.is_none_or(|(_, b)| d < b) {
            best = Some((site, d));
        }
    }
    best
}

// Haversine; the site list's two decimals make anything finer pointless.
pub fn distance_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const R: f64 = 6371.0;
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
    2.0 * R * a.sqrt().asin()
}

pub fn hour_folder(province: &str, hour: i8) -> String {
    format!("{BASE}/{province}/{hour:02}/")
}

// File names start with the emission time, so the greatest name is the
// newest: 20260912T035744.540Z_MSC_CitypageWeather_s0000629_en.xml.
pub fn latest_file(listing_html: &str, site_code: &str) -> Option<String> {
    let suffix = format!("Z_MSC_CitypageWeather_{site_code}_en.xml");
    let mut latest: Option<String> = None;
    for piece in listing_html.split("href=\"").skip(1) {
        let Some(name) = piece.split('"').next() else {
            continue;
        };
        if is_city_page_name(name, &suffix) && latest.as_deref().is_none_or(|l| name > l) {
            latest = Some(name.to_string());
        }
    }
    latest
}

// \d{8}T\d{6}(\.\d+)? before the suffix.
fn is_city_page_name(name: &str, suffix: &str) -> bool {
    let Some(stamp) = name.strip_suffix(suffix) else {
        return false;
    };
    let (main, fraction) = match stamp.split_once('.') {
        Some((main, fraction)) => (main, Some(fraction)),
        None => (stamp, None),
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    main.is_ascii()
        && main.len() == 15
        && digits(&main[..8])
        && &main[8..9] == "T"
        && digits(&main[9..])
        && fraction.is_none_or(digits)
}

fn child<'a, 'i>(node: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
    node.children().find(|n| n.has_tag_name(name))
}

fn children<'a, 'i>(
    node: roxmltree::Node<'a, 'i>,
    name: &'a str,
) -> impl Iterator<Item = roxmltree::Node<'a, 'i>> {
    node.children().filter(move |n| n.has_tag_name(name))
}

// XElement.Value: all the text within, entities decoded.
fn value(node: roxmltree::Node) -> String {
    node.descendants()
        .filter(|n| n.is_text())
        .filter_map(|n| n.text())
        .collect()
}

pub fn parse(xml: &str) -> Result<OfficialForecast, OfficialError> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| OfficialError::Unreadable(format!("City page could not be read: {e}")))?;
    let root = doc.root_element();
    let location = child(root, "location");
    let site_name = location
        .and_then(|l| child(l, "name"))
        .map(|n| value(n).trim().to_string());
    let region = location
        .and_then(|l| child(l, "region"))
        .map(|n| value(n).trim().to_string());
    let mut attribution = format!("{FULL_NAME} (weather.gc.ca)");
    if let Some(region) = region.filter(|r| !r.is_empty()) {
        attribution += &format!(", forecast for {region}");
    } else if let Some(site) = site_name.filter(|s| !s.is_empty()) {
        attribution += &format!(", forecast for {site}");
    }
    Ok(OfficialForecast {
        source_name: SOURCE_NAME.to_string(),
        attribution,
        periods: parse_periods(root)?,
        observation: parse_observation(root),
    })
}

fn parse_periods(root: roxmltree::Node) -> Result<Vec<OfficialPeriod>, OfficialError> {
    // Some sites (Alert, Eureka) only ever report observations.
    let no_text = || OfficialError::NoForecastText("City page has no forecast text.".into());
    let group = child(root, "forecastGroup").ok_or_else(no_text)?;
    let issued = local_date(group, "forecastIssue").ok_or_else(no_text)?;

    // Periods are named, not dated: "Tonight" belongs to the issue date, a
    // weekday name to the first such day from there on, and the names run
    // forward, so each one moves the cursor.
    let mut list = Vec::new();
    let mut cursor = issued;
    for forecast in children(group, "forecast") {
        let name = child(forecast, "period")
            .and_then(|p| p.attribute("textForecastName"))
            .map(str::trim)
            .unwrap_or_default();
        let text = child(forecast, "textSummary").map(value);
        let Some(text) = text.filter(|t| !name.is_empty() && !t.trim().is_empty()) else {
            continue;
        };
        let mut date = cursor;
        if name != "Today"
            && name != "Tonight"
            && let Some(weekday) = weekday(name)
        {
            while date.weekday() != weekday {
                date = date
                    .tomorrow()
                    .map_err(|e| OfficialError::Unreadable(e.to_string()))?;
            }
            cursor = date;
        }
        list.push(OfficialPeriod {
            name: name.to_string(),
            date,
            text: official::spoken(&text),
        });
    }
    if list.is_empty() {
        return Err(OfficialError::Unreadable(
            "City page forecast has no periods.".into(),
        ));
    }
    Ok(list)
}

fn weekday(period_name: &str) -> Option<Weekday> {
    let first = period_name.split(' ').next()?.to_lowercase();
    Some(match first.as_str() {
        "monday" => Weekday::Monday,
        "tuesday" => Weekday::Tuesday,
        "wednesday" => Weekday::Wednesday,
        "thursday" => Weekday::Thursday,
        "friday" => Weekday::Friday,
        "saturday" => Weekday::Saturday,
        "sunday" => Weekday::Sunday,
        _ => return None,
    })
}

// The local (non-UTC) dateTime element of the given name, as a date.
fn local_date(parent: roxmltree::Node, name: &str) -> Option<Date> {
    for dt in children(parent, "dateTime") {
        if dt.attribute("name") != Some(name) || dt.attribute("zone") == Some("UTC") {
            continue;
        }
        let number = |tag: &str| child(dt, tag).and_then(|n| value(n).trim().parse::<i32>().ok());
        if let (Some(y), Some(m), Some(d)) = (number("year"), number("month"), number("day"))
            && let Ok(day) = Date::new(y as i16, m as i8, d as i8)
        {
            return Some(day);
        }
    }
    None
}

fn parse_observation(root: roxmltree::Node) -> Option<Observation> {
    let c = child(root, "currentConditions")?;
    if !c.children().any(|n| n.is_element()) {
        return None;
    }
    let station = child(c, "station")
        .map(|n| value(n).trim().to_string())
        .filter(|s| !s.is_empty())?;
    let stamp = children(c, "dateTime")
        .find(|d| d.attribute("zone") == Some("UTC"))
        .and_then(|d| child(d, "timeStamp"))
        .map(value)
        .filter(|s| !s.is_empty())?;
    let time = utc_stamp(&stamp)?;

    let wind = child(c, "wind");
    let number =
        |node: Option<roxmltree::Node>| node.and_then(|n| value(n).trim().parse::<f64>().ok());
    let condition = child(c, "condition")
        .map(|n| value(n).trim().to_string())
        .filter(|s| !s.is_empty());
    Some(Observation {
        station: official::station_name(&station),
        time,
        description: condition,
        temperature_c: number(child(c, "temperature")),
        feels_like_c: number(child(c, "humidex")).or(number(child(c, "windChill"))),
        humidity: number(child(c, "relativeHumidity")).map(crate::round_even),
        wind_kmh: number(wind.and_then(|w| child(w, "speed"))),
        wind_direction: number(wind.and_then(|w| child(w, "bearing"))).map(crate::round_even),
        wind_gust_kmh: number(wind.and_then(|w| child(w, "gust"))),
        pressure_hpa: number(child(c, "pressure")).map(|kpa| kpa * 10.0),
        dew_point_c: number(child(c, "dewpoint")),
        visibility_metres: number(child(c, "visibility")).map(|km| km * 1000.0),
    })
}

// yyyyMMddHHmmss, UTC.
fn utc_stamp(stamp: &str) -> Option<Timestamp> {
    if stamp.len() != 14 || !stamp.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let n = |range: std::ops::Range<usize>| stamp[range].parse::<i16>().ok();
    let local = DateTime::new(
        n(0..4)?,
        n(4..6)? as i8,
        n(6..8)? as i8,
        n(8..10)? as i8,
        n(10..12)? as i8,
        n(12..14)? as i8,
        0,
    )
    .ok()?;
    jiff::tz::Offset::UTC.to_timestamp(local).ok()
}

// Environment Canada's alerts in effect at a point, from the weather-alerts
// collection of MSC GeoMet-OGC-API: one request gives each alert with its
// full English text, colour level, area and times, which the city page's
// warnings block (a headline and a link) and the CAP files on the datamart
// (one file per message, findable only by walking every office's hourly
// folder) could not do between them. The feature id is "{alert}_{area}":
// the alert part stays the same through EC's updates of one alert, so it
// is the identity. Times are UTC ISO 8601; the French fields are not read.
pub const ALERTS_ATTRIBUTION: &str = "Environment and Climate Change Canada (weather.gc.ca)";
const ALERTS_ENDPOINT: &str = "https://api.weather.gc.ca/collections/weather-alerts/items";

// A point-sized bbox: the polygons are forecast regions, and the geometry
// itself (megabytes for a province-wide alert) is not needed.
pub fn alerts_url(location: &crate::location::Location) -> String {
    let lat = crate::open_meteo::coordinate(location.latitude);
    let lon = crate::open_meteo::coordinate(location.longitude);
    format!("{ALERTS_ENDPOINT}?f=json&limit=100&skipGeometry=true&bbox={lon},{lat},{lon},{lat}")
}

// The location page on weather.gc.ca lists the same alerts with links to
// each one's full report.
pub fn location_url(location: &crate::location::Location) -> String {
    format!(
        "https://weather.gc.ca/en/location/index.html?coords={},{}",
        three_decimals(location.latitude),
        three_decimals(location.longitude)
    )
}

// .NET's "0.###".
fn three_decimals(value: f64) -> String {
    crate::units::trimmed(value, 3)
}

#[derive(serde::Deserialize)]
struct AlertsResponse {
    features: Option<Vec<AlertFeature>>,
}

#[derive(serde::Deserialize)]
struct AlertFeature {
    id: Option<String>,
    properties: Option<Alert>,
}

#[derive(serde::Deserialize)]
struct Alert {
    alert_type: Option<String>,
    alert_name_en: Option<String>,
    alert_text_en: Option<String>,
    publication_datetime: Option<String>,
    expiration_datetime: Option<String>,
    validity_datetime: Option<String>,
    event_end_datetime: Option<String>,
    risk_colour_en: Option<String>,
    impact_en: Option<String>,
    confidence_en: Option<String>,
    feature_name_en: Option<String>,
    status_en: Option<String>,
}

pub fn parse_alerts(
    json: &str,
    now: Timestamp,
    url: Option<&str>,
) -> Result<Vec<WeatherAlert>, OfficialError> {
    let response: AlertsResponse = serde_json::from_str(json)?;
    let mut list = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for feature in response.features.unwrap_or_default() {
        let Some(a) = feature.properties else {
            continue;
        };
        let (Some(name), Some(feature_id), Some(published)) = (
            a.alert_name_en.as_deref().filter(|n| !n.trim().is_empty()),
            feature.id.as_deref().filter(|i| !i.is_empty()),
            a.publication_datetime.as_deref().filter(|p| !p.is_empty()),
        ) else {
            continue;
        };
        // The collection is meant to hold only alerts in effect; an ended or
        // stale one is dropped in case it lingers.
        if a.status_en.as_deref() == Some("ended") {
            continue;
        }
        let expires = alert_time(&a.expiration_datetime)?;
        if expires.is_some_and(|e| e < now) {
            continue;
        }
        let id = format!("ec:{}", feature_id.split('_').next().unwrap_or(feature_id));
        if !seen.insert(id.clone()) {
            continue;
        }
        let issued = alert_time(&Some(published.to_string()))?.expect("checked present above");
        // "Yellow wind warning": the colour first, as EC's own "Yellow
        // Warning - Wind" puts it, but in a form that reads well in the
        // alert line and in "... in effect". Not in 0.1 (2026-10-06).
        let event = match a.risk_colour_en.as_deref().map(str::trim) {
            Some(colour) if !colour.is_empty() => format!("{colour} {name}"),
            _ => name.to_string(),
        };
        list.push(WeatherAlert {
            id,
            event: official::sentence_case(&event),
            severity: alert_severity(a.risk_colour_en.as_deref(), a.alert_type.as_deref()),
            issued,
            onset: alert_time(&a.validity_datetime)?,
            ends: alert_time(&a.event_end_datetime)?,
            source: SOURCE_NAME.to_string(),
            sender: SOURCE_NAME.to_string(),
            area: a
                .feature_name_en
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .to_string(),
            level: level(&a),
            description: a.alert_text_en.clone().unwrap_or_default(),
            instruction: None,
            url: url.map(str::to_string),
            expires,
        });
    }
    Ok(list)
}

// UTC when no offset is given, as DateTimeStyles.AssumeUniversal had it.
fn alert_time(text: &Option<String>) -> Result<Option<Timestamp>, OfficialError> {
    let Some(text) = text.as_deref().filter(|t| !t.is_empty()) else {
        return Ok(None);
    };
    if let Ok(t) = text.parse::<Timestamp>() {
        return Ok(Some(t));
    }
    text.parse::<DateTime>()
        .ok()
        .and_then(|local| jiff::tz::Offset::UTC.to_timestamp(local).ok())
        .map(Some)
        .ok_or_else(|| {
            OfficialError::Unreadable(format!(
                "Environment Canada alert time \"{text}\" could not be read."
            ))
        })
}

// EC's colour levels (yellow: be aware; orange: be prepared; red: take
// action) set the severity, and the alert type is a floor under them: a
// warning is at least Severe whatever its colour, so that the "severe and
// extreme only" announcement setting means the same on both sides of the
// border, where every NWS warning is Severe or Extreme.
pub fn alert_severity(colour: Option<&str>, kind: Option<&str>) -> AlertSeverity {
    let by_colour = match colour.map(str::to_lowercase).as_deref() {
        Some("red") => AlertSeverity::Extreme,
        Some("orange") => AlertSeverity::Severe,
        Some("yellow") => AlertSeverity::Moderate,
        _ => AlertSeverity::Unknown,
    };
    let by_type = match kind.map(str::to_lowercase).as_deref() {
        Some("warning") => AlertSeverity::Severe,
        Some("watch") => AlertSeverity::Moderate,
        Some("advisory") | Some("statement") => AlertSeverity::Minor,
        _ => AlertSeverity::Unknown,
    };
    by_colour.max(by_type)
}

// "Yellow level, moderate impact, high confidence": EC's tiered ranking as
// the public alert pages state it, when the alert carries one.
fn level(a: &Alert) -> Option<String> {
    let blank = |v: &Option<String>| v.as_deref().is_none_or(|s| s.trim().is_empty());
    if blank(&a.risk_colour_en) {
        return None;
    }
    let mut s = format!(
        "{} level",
        official::sentence_case(a.risk_colour_en.as_deref().unwrap_or_default())
    );
    if !blank(&a.impact_en) {
        s += &format!(
            ", {} impact",
            a.impact_en
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_lowercase()
        );
    }
    if !blank(&a.confidence_en) {
        s += &format!(
            ", {} confidence",
            a.confidence_en
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_lowercase()
        );
    }
    Some(s)
}
