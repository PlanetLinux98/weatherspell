// Alerts for Australia from the Bureau of Meteorology. Its website refuses
// programs and points them to its anonymous FTP, where each warning
// product in effect is a CAP-AU file ("IDQ20790.cap.xml", Queensland's
// flood warnings for the Mary River) and each info block in it a warning
// for areas of its own. The Bureau's default terms let a person download
// its material for their own use but not pass it on: each copy of the app
// fetches for its own user, and nothing of the Bureau's is built into the
// app or its tests (NOTES.md).
//
// The warnings name their areas only by the Bureau's codes ("NSW_PW005",
// the Metropolitan forecast district). The outlines are in its spatial
// files, a zipped shapefile per kind of area on the same FTP: the app
// downloads them the first time an Australian location is checked, keeps
// them simplified in the cache folder, and downloads a layer again only
// when the Bureau's listing shows it changed (fetch.rs). A warning applies
// where one of its codes is an area holding the location, or the larger
// area holding that one. The location's areas also say which states'
// files to read, since a catchment can cross a border. Tsunami warnings
// are the national warning centre's, for zones drawn at sea, which reach
// a few kilometres inland (LAYERS).
//
// The Bureau's warnings are not colour-coded: each reads as its headline
// names it, "Severe thunderstorm warning", "Minor flood warning". Its CAP
// severity, where it gives one, ranks the warning (Minor to Major for
// floods); elsewhere the kind does, so that a severe thunderstorm warning
// counts as severe. The messages' expiry is when the next issue is due,
// not when the hazard ends, so it is never read as "until".

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use jiff::Timestamp;

use crate::alerts::{AlertSeverity, WeatherAlert};
use crate::area::{self, Polygon, Region};
use crate::cap::{self, CapAlert, CapInfo};
use crate::location::Location;
use crate::official::{self, OfficialError};
use crate::shapes;

// The acknowledgement the Bureau asks for where its material is not
// licensed CC BY, as its warnings under the default terms are not;
// "Commonwealth of Australia" is the form for CC BY material.
pub const ALERTS_ATTRIBUTION: &str = "\u{a9} Bureau of Meteorology (bom.gov.au)";
const SOURCE_NAME: &str = "the Bureau of Meteorology";
const WARNINGS_PAGE: &str = "https://www.bom.gov.au/australia/warnings/";

const WARNINGS_FOLDER: &str = "ftp://ftp.bom.gov.au/anon/gen/fwo/";
pub const WARNINGS_URL: &str = "ftp://ftp.bom.gov.au/anon/gen/fwo/*.cap.xml";
const SPATIAL_FOLDER: &str = "ftp://ftp.bom.gov.au/anon/home/adfd/spatial/";
pub const LAYERS_URL: &str = "ftp://ftp.bom.gov.au/anon/home/adfd/spatial/IDM000*.zip";

// The cache folder's subfolder for the outlines, a file per layer.
pub const FOLDER_NAME: &str = "bom-areas";

// The largest layer is 14 MB.
pub const LAYER_TIMEOUT: Duration = Duration::from_secs(120);

// As for MeteoAlarm's table: the outlines are simplified here.
const TABLE_MARGIN_KM: f64 = 0.75;

pub struct Layer {
    pub product: &'static str,
    margin_km: f64,
    // The fields with an area's name and the code of the larger area
    // holding it.
    name_field: &'static str,
    parent_field: &'static str,
}

const fn layer(product: &'static str, margin_km: f64) -> Layer {
    Layer {
        product,
        margin_km,
        name_field: "DIST_NAME",
        parent_field: "AAC_PARENT",
    }
}

// The layers the warnings' codes come from, as the Bureau's spatial data
// guide gives them.
pub const LAYERS: [Layer; 7] = [
    // Forecast districts: severe thunderstorm, severe weather, heatwave,
    // coastal hazard, tropical cyclone and agricultural warnings.
    layer("IDM00001", TABLE_MARGIN_KM),
    // Metropolitan areas, which the cities' thunderstorm warnings name.
    layer("IDM00014", TABLE_MARGIN_KM),
    // Fire weather districts.
    layer("IDM00007", TABLE_MARGIN_KM),
    // Flood warning catchments.
    layer("IDM00017", TABLE_MARGIN_KM),
    // Flood watch catchments.
    layer("IDM00020", TABLE_MARGIN_KM),
    // Coastal waters, for marine wind and hazardous surf warnings: at sea,
    // so taken as drawn, or every coastal town would hear the sea's.
    layer("IDM00003", 0.0),
    // Tsunami warning zones, also at sea along the coast, so the margin is
    // how far inland a warning reaches. No distance is published; the
    // Bureau tells people under a land threat to go at least 1 km inland,
    // and 3 km takes in the towns along the beaches as well, such as
    // Coffs Harbour, whose centre is 2.6 km from its zone. A zone has a
    // second code ("NSW_TSW008" beside "NSW_TW008"), kept as its parent so
    // that a warning naming either reaches it. Its AAC_PARENT, the state
    // with its waters ("NSW_FA001"), is left out: a product for the whole
    // state naming that would otherwise reach only its coast.
    Layer {
        product: "IDM00016",
        margin_km: 3.0,
        name_field: "ATWS_Zones",
        parent_field: "TWZ",
    },
];

pub fn covers(location: &Location) -> bool {
    location.country.as_deref() == Some("Australia")
}

pub fn warning_url(name: &str) -> String {
    format!("{WARNINGS_FOLDER}{name}")
}

pub fn layer_url(layer: &Layer) -> String {
    format!("{SPATIAL_FOLDER}{}.zip", layer.product)
}

// A file in a folder's listing, with the listing's line for it, which
// changes when the file does (its size and time).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    pub name: String,
    pub line: String,
}

// "-rw-rw-r--    1 1050     1502         4294 Oct 26 01:04 IDN20400.cap.xml"
pub fn parse_listing(text: &str) -> Vec<Listed> {
    text.lines()
        .map(str::trim)
        .filter_map(|line| {
            let name = line.split_whitespace().last()?;
            Some(Listed {
                name: name.to_string(),
                line: line.to_string(),
            })
        })
        .collect()
}

// A layer's size and day: its listed time turns into a year once it is
// six months old, which is no change to the file.
pub fn layer_version(line: &str) -> String {
    let fields: Vec<&str> = line.split_whitespace().collect();
    match fields.as_slice() {
        [_, _, _, _, size, month, day, _, _] => format!("{size} {month} {day}"),
        _ => line.to_string(),
    }
}

fn unreadable(why: impl std::fmt::Display) -> OfficialError {
    OfficialError::Unreadable(format!(
        "the Bureau of Meteorology's warning areas could not be read: {why}"
    ))
}

// A layer's areas from the Bureau's zip archive, simplified.
pub fn read_layer(archive: &[u8], layer: &Layer) -> Result<Vec<Region>, OfficialError> {
    let shp = shapes::unzip(archive, &format!("{}.shp", layer.product)).map_err(unreadable)?;
    let dbf = shapes::unzip(archive, &format!("{}.dbf", layer.product)).map_err(unreadable)?;
    let shapes = shapes::polygons(&shp).map_err(unreadable)?;
    let rows = shapes::rows(&dbf).map_err(unreadable)?;
    if shapes.len() != rows.len() {
        return Err(unreadable("its outlines and names do not match"));
    }
    fn field<'a>(row: &'a HashMap<String, String>, name: &str) -> Option<&'a str> {
        row.get(name).map(String::as_str).filter(|v| !v.is_empty())
    }
    Ok(shapes
        .iter()
        .zip(&rows)
        .filter_map(|(rings, row)| {
            let code = field(row, "AAC")?;
            let rings: Vec<Vec<(f64, f64)>> =
                rings.iter().filter_map(|r| area::simplified(r)).collect();
            let polygon = Polygon::new(rings)?;
            Some(Region::new(
                code,
                field(row, layer.name_field).unwrap_or_default(),
                field(row, layer.parent_field),
                vec![polygon],
            ))
        })
        .collect())
}

// A layer as the cache folder keeps it: the outlines table (area.rs) with
// the listing it came from.
pub fn layer_text(layer: &Layer, version: &str, regions: &[Region]) -> String {
    let mut text = format!(
        "# The Bureau of Meteorology's {} warning areas, simplified, for this\n\
         # app's own use. Downloaded, and replaced when the Bureau's changes.\n\
         # Listed as: {version}\n",
        layer.product
    );
    for region in regions {
        text.push_str(&area::table_line(region));
        text.push('\n');
    }
    text
}

pub fn parse_layer_text(text: &str) -> Option<(String, Vec<Region>)> {
    let version = text
        .lines()
        .take_while(|line| line.starts_with('#'))
        .find_map(|line| line.strip_prefix("# Listed as: "))?;
    let regions = area::parse_table(text);
    (!regions.is_empty()).then(|| (version.trim().to_string(), regions))
}

// The layers in hand, with what they were downloaded from.
pub struct Areas {
    pub layers: Vec<KeptLayer>,
    parents: HashMap<String, String>,
}

pub struct KeptLayer {
    pub product: &'static str,
    pub version: String,
    regions: Vec<Region>,
}

impl KeptLayer {
    pub fn new(layer: &Layer, version: &str, regions: Vec<Region>) -> KeptLayer {
        KeptLayer {
            product: layer.product,
            version: version.to_string(),
            regions,
        }
    }
}

impl Areas {
    pub fn new(layers: Vec<KeptLayer>) -> Areas {
        let parents = layers
            .iter()
            .flat_map(|l| &l.regions)
            .filter_map(|r| Some((r.code.clone(), r.parent.clone()?)))
            .collect();
        Areas { layers, parents }
    }

    // The codes of the areas holding the point, and of the larger areas
    // holding those.
    pub fn codes_at(&self, latitude: f64, longitude: f64) -> HashSet<String> {
        let mut codes = HashSet::new();
        for kept in &self.layers {
            let margin = LAYERS
                .iter()
                .find(|l| l.product == kept.product)
                .map_or(TABLE_MARGIN_KM, |l| l.margin_km);
            for region in &kept.regions {
                if region.covers(latitude, longitude, margin) {
                    let mut code = Some(region.code.clone());
                    while let Some(c) = code {
                        if !codes.insert(c.clone()) {
                            break;
                        }
                        code = self.parents.get(&c).cloned();
                    }
                }
            }
        }
        codes
    }
}

// Each office's products start with its state's letters: "IDN" for New
// South Wales and the ACT, as the Bureau's catalogue lists them. Tsunami
// warnings are the national warning centre's, a product for each state
// and territory ("IDY68028" for New South Wales, "IDY68034" for Lord Howe
// Island).
fn office(code: &str) -> Option<&'static str> {
    let (state, kind) = code.split_once('_')?;
    if kind.starts_with("TW") || kind.starts_with("TSW") {
        return Some("IDY68");
    }
    Some(match state {
        "NSW" | "ACT" => "IDN",
        "VIC" => "IDV",
        "QLD" => "IDQ",
        "SA" => "IDS",
        "WA" => "IDW",
        "TAS" => "IDT",
        "NT" => "IDD",
        _ => return None,
    })
}

// Whether a listed warning file is from an office whose areas hold the
// location.
pub fn concerns(name: &str, codes: &HashSet<String>) -> bool {
    name.ends_with(".cap.xml")
        && codes
            .iter()
            .filter_map(|c| office(c))
            .any(|prefix| name.starts_with(prefix))
}

// The warnings in the CAP files that apply where the codes are.
pub fn alerts_for<'a>(
    files: impl IntoIterator<Item = &'a CapAlert>,
    codes: &HashSet<String>,
    now: Timestamp,
) -> Vec<WeatherAlert> {
    let mut found: Vec<(&CapAlert, &CapInfo, String)> = Vec::new();
    for alert in files.into_iter().filter(|a| a.is_live()) {
        // "AusBoM-IDQ20790-2026-10-26T01:34:18+00:00": the product stays
        // the same through its updates.
        let product = alert
            .identifier
            .split('-')
            .nth(1)
            .unwrap_or(&alert.identifier);
        for info in &alert.infos {
            if info.is_all_clear()
                || info.has_ended(now)
                || info
                    .headline
                    .trim_start()
                    .to_lowercase()
                    .starts_with("cancellation")
            {
                continue;
            }
            let mut held: Vec<&str> = info
                .areas
                .iter()
                .flat_map(|a| &a.geocodes)
                .filter(|(name, _)| name.eq_ignore_ascii_case("AMOC-AreaCode"))
                .map(|(_, code)| code.trim())
                .filter(|code| codes.contains(*code))
                .collect();
            held.sort_unstable();
            let Some(first) = held.first() else {
                continue;
            };
            // One id for a warning through its updates, so it is heard
            // once, and again only if an update raises it (alerts::track).
            // Two of a product's warnings can name the same area
            // (tomorrow's and today's).
            let base = format!("au:{product}/{first}");
            let mut id = base.clone();
            let mut n = 1;
            while found.iter().any(|(_, _, f)| *f == id) {
                n += 1;
                id = format!("{base}/{n}");
            }
            found.push((alert, info, id));
        }
    }
    // A city's warning and its district's can both hold a location, and
    // would read the same, "Severe thunderstorm warning": those two keep
    // the areas their headlines name.
    let names: Vec<String> = found.iter().map(|(_, info, _)| name(info)).collect();
    found
        .into_iter()
        .zip(&names)
        .filter_map(|((alert, info, id), name)| {
            let twins = names.iter().filter(|n| *n == name).count() > 1;
            weather_alert(alert, info, id, name, twins)
        })
        .collect()
}

fn weather_alert(
    alert: &CapAlert,
    info: &CapInfo,
    id: String,
    name: &str,
    with_area: bool,
) -> Option<WeatherAlert> {
    let issued = alert.sent.or(info.effective)?;
    let event = match info.headline.trim().split_once(" for ") {
        // The area as the Bureau writes it, capitals and all, since its
        // place names can't be told from its title case.
        Some((_, area)) if with_area => format!("{name} for {}", area.trim()),
        _ => name.to_string(),
    };
    let mut description = Vec::new();
    let headline = info.headline.trim();
    if !headline.is_empty() && !headline.eq_ignore_ascii_case(&event) {
        description.push(cap::readable(headline));
    }
    if !info.description.trim().is_empty() {
        description.push(cap::readable(info.description.trim()));
    }
    let mut areas: Vec<&str> = Vec::new();
    for area in info.areas.iter().map(|a| a.description.trim()) {
        if !area.is_empty() && !areas.contains(&area) {
            areas.push(area);
        }
    }
    let web = info.web.trim();
    let url = match web.strip_prefix("http://") {
        Some(rest) => format!("https://{rest}"),
        None if web.starts_with("https://") => web.to_string(),
        None => WARNINGS_PAGE.to_string(),
    };
    Some(WeatherAlert {
        id,
        severity: severity(info, name),
        event,
        issued,
        onset: info.onset,
        ends: None,
        // Its CAP names no office, and "from Australian Government Bureau
        // of Meteorology" reads badly.
        source: SOURCE_NAME.to_string(),
        sender: SOURCE_NAME.to_string(),
        area: areas.join("; "),
        level: None,
        description: description.join("\n\n"),
        instruction: Some(cap::readable(info.instruction.trim())).filter(|i| !i.is_empty()),
        url: Some(url),
        expires: info.expires,
        // Followed by its id rather than by a chain (above); an empty list
        // still has its rank kept.
        references: Some(Vec::new()),
        notice: None,
    })
}

// "Severe Thunderstorm Warning for Northern Rivers, ... forecast districts"
// as "Severe thunderstorm warning": the headline names the warning before
// the areas.
fn name(info: &CapInfo) -> String {
    let headline = info.headline.trim();
    let name = headline
        .split_once(" for ")
        .map_or(headline, |(n, _)| n)
        .trim();
    if name.is_empty() {
        official::sentence_case(&format!("{} warning", info.event.trim()))
    } else {
        official::sentence_case(name)
    }
}

const SEVERE_KINDS: [&str; 5] = [
    "severe",
    "fire weather",
    "tropical cyclone",
    "heatwave",
    "tsunami",
];

fn severity(info: &CapInfo, name: &str) -> AlertSeverity {
    match info.severity.trim().to_ascii_lowercase().as_str() {
        "extreme" => return AlertSeverity::Extreme,
        "severe" => return AlertSeverity::Severe,
        "moderate" => return AlertSeverity::Moderate,
        "minor" => return AlertSeverity::Minor,
        _ => {}
    }
    let name = name.to_lowercase();
    if name.contains("extreme heatwave") {
        AlertSeverity::Extreme
    } else if SEVERE_KINDS.iter().any(|kind| name.contains(kind)) {
        AlertSeverity::Severe
    } else if name.ends_with("warning") || name.ends_with("watch") {
        AlertSeverity::Moderate
    } else {
        AlertSeverity::Minor
    }
}
