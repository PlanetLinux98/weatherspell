// Writes data/meteoalarm-regions.txt, the outlines of MeteoAlarm's EMMA_ID
// warning regions that meteoalarm.rs embeds, from the GeoJSON file
// MeteoAlarm publishes in its documents repository
// (gitlab.com/meteoalarm-pm-group/documents, MeteoAlarm_Geocodes_*.json):
//
//   cargo run -p weatherspell-core --example meteoalarm_regions -- MeteoAlarm_Geocodes_2026_07_31.json
//
// The 33 MB file shrinks to about a megabyte: each ring is simplified as
// area::simplified does, which moves a boundary by at most about 0.6 km;
// meteoalarm::TABLE_MARGIN_KM allows for that.

use std::fmt::Write as _;

use serde_json::Value;
use weatherspell_core::area::{self, Polygon, Region};

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("give the path of MeteoAlarm's geocodes file");
    let json = std::fs::read_to_string(&path).expect("the geocodes file can be read");
    let doc: Value = serde_json::from_str(&json).expect("the geocodes file is JSON");
    let source = std::path::Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();

    let mut regions: Vec<Region> = Vec::new();
    for feature in doc["features"].as_array().expect("a feature collection") {
        let properties = &feature["properties"];
        let (Some(code), Some(name)) = (properties["code"].as_str(), properties["name"].as_str())
        else {
            continue;
        };
        if properties["type"].as_str() != Some("EMMA_ID") {
            continue;
        }
        let geometry = &feature["geometry"];
        let polygons: Vec<&Value> = match geometry["type"].as_str() {
            Some("Polygon") => vec![&geometry["coordinates"]],
            Some("MultiPolygon") => geometry["coordinates"]
                .as_array()
                .map(|a| a.iter().collect())
                .unwrap_or_default(),
            _ => continue,
        };
        let polygons: Vec<Polygon> = polygons
            .into_iter()
            .filter_map(|polygon| {
                Polygon::new(
                    polygon
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(simplified_ring)
                        .collect(),
                )
            })
            .collect();
        if !polygons.is_empty() {
            regions.push(Region::new(code, name.trim(), None, polygons));
        }
    }
    regions.sort_by(|a, b| (&a.code, &a.name).cmp(&(&b.code, &b.name)));

    let mut text = String::new();
    writeln!(
        text,
        "# MeteoAlarm's EMMA_ID warning regions, from {source} (MeteoAlarm,"
    )
    .unwrap();
    writeln!(
        text,
        "# EUMETNET), simplified. Written by examples/meteoalarm_regions.rs; do not edit."
    )
    .unwrap();
    writeln!(
        text,
        "# Code, name, then polygons split by |, rings by ;, points \"lat,lon\" by spaces."
    )
    .unwrap();
    for region in &regions {
        writeln!(text, "{}", area::table_line(region)).unwrap();
    }
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/meteoalarm-regions.txt");
    std::fs::write(&out, &text).expect("the regions file can be written");
    println!(
        "{} regions, {} bytes, written to {}",
        regions.len(),
        text.len(),
        out.display()
    );
}

// GeoJSON's [lon, lat] pairs, simplified.
fn simplified_ring(ring: &Value) -> Option<Vec<(f64, f64)>> {
    let points: Vec<(f64, f64)> = ring
        .as_array()?
        .iter()
        .filter_map(|p| Some((p.get(1)?.as_f64()?, p.get(0)?.as_f64()?)))
        .collect();
    area::simplified(&points)
}
