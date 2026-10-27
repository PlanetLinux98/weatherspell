// Writes data/philippine-provinces.txt, the outlines of the Philippines'
// provinces that pagasa.rs embeds for PAGASA's wind signals, which name
// provinces but carry no outlines. From geoBoundaries' simplified copy of
// the boundaries NAMRIA and PSA publish through OCHA (CC BY 3.0 IGO):
//
//   cargo run -p weatherspell-core --example philippine_provinces -- geoBoundaries-PHL-ADM2_simplified.geojson
//
// (github.com/wmgeolab/geoBoundaries, releaseData/gbOpen/PHL/ADM2.) Each
// ring is simplified as area::simplified does. Two provinces take the
// names PAGASA uses: Metro Manila's four districts, and Compostela Valley,
// renamed Davao de Oro in 2019.

use std::fmt::Write as _;

use serde_json::Value;
use weatherspell_core::area::{self, Polygon, Region};

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("give the path of geoBoundaries' PHL ADM2 file");
    let json = std::fs::read_to_string(&path).expect("the boundaries file can be read");
    let doc: Value = serde_json::from_str(&json).expect("the boundaries file is JSON");

    let mut regions: Vec<Region> = Vec::new();
    for feature in doc["features"].as_array().expect("a feature collection") {
        let Some(code) = feature["properties"]["shapeName"].as_str() else {
            continue;
        };
        let name = if code.starts_with("NCR") {
            "Metro Manila"
        } else if code == "Compostela Valley" {
            "Davao de Oro"
        } else {
            code
        };
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
            regions.push(Region::new(code, name, None, polygons));
        }
    }
    regions.sort_by(|a, b| a.code.cmp(&b.code));

    let mut text = String::new();
    writeln!(
        text,
        "# The Philippines' provinces, from geoBoundaries (PHL ADM2, gbOpen), whose source is"
    )
    .unwrap();
    writeln!(
        text,
        "# NAMRIA, PSA and OCHA Philippines; licensed CC BY 3.0 IGO. Simplified. Written by"
    )
    .unwrap();
    writeln!(
        text,
        "# examples/philippine_provinces.rs; do not edit. Code, name, then polygons split by |,"
    )
    .unwrap();
    writeln!(text, "# rings by ;, points \"lat,lon\" by spaces.").unwrap();
    for region in &regions {
        writeln!(text, "{}", area::table_line(region)).unwrap();
    }
    let out =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/philippine-provinces.txt");
    std::fs::write(&out, &text).expect("the provinces file can be written");
    println!(
        "{} provinces, {} bytes, written to {}",
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
