// Postal codes for the countries Open-Meteo's geocoder does not index. Its
// postcode search is dependable for the US, France, Spain, the Netherlands
// and Belgium, patchy elsewhere in Europe and absent for Canada, the UK,
// Australia, New Zealand and Ireland (probed 2026-09-12), so the exe
// carries the GeoNames table for those five: one entry per code, named
// after its most populous place. tools\Update-PostalCodes.ps1 regenerates
// it (into the C# app's folder, where both apps read it until the switch)
// and explains the choices. GeoNames has only the first part of Canadian
// and Irish codes and the UK outward code, so a full code resolves to its
// area.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use crate::location::Location;

pub const SOURCE_NOTE: &str = "GeoNames (geonames.org), licensed CC BY 4.0";

// Countries whose codes come from the table; the geocoder's answers for
// them are dropped when the table has any, so each country has one source.
pub const COUNTRIES: [&str; 5] = [
    "Canada",
    "United Kingdom",
    "Australia",
    "New Zealand",
    "Ireland",
];

pub const TABLE_TEXT: &str = include_str!("../../../src/Weatherspell/Weather/PostalCodes.tsv");

static TABLE: LazyLock<HashMap<String, Vec<Location>>> = LazyLock::new(|| {
    let mut table: HashMap<String, Vec<Location>> = HashMap::new();
    for line in TABLE_TEXT.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 6 {
            continue;
        }
        let (Ok(latitude), Ok(longitude)) = (f[4].parse(), f[5].parse()) else {
            continue;
        };
        let region = (!f[2].is_empty()).then_some(f[2]);
        table
            .entry(f[0].to_string())
            .or_default()
            .push(Location::new(f[1], region, Some(f[3]), latitude, longitude));
    }
    table
});

// A UK code with the space left out: outward code, then digit and two
// letters. Canadian (letter digit letter digit letter digit) and Irish
// (letter, two digits, four more) codes with the space left out: the first
// three count.
static UK_FULL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Z]{1,2}\d[A-Z\d]?\d[A-Z]{2}$").unwrap());
static CANADIAN_OR_IRISH_FULL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:[A-Z]\d[A-Z]\d[A-Z]\d|[A-Z]\d\d[A-Z\d]{4})$").unwrap());
static CODE_LIKE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Z\d]{2,8}$").unwrap());

// Every place the query names as a postal code, in table order (country,
// then code); empty when it names none. A code can exist in several
// countries (2000 is Sydney, and 1010 both Sydney and Auckland), and all
// are listed with their country, as the geocoder does for its own.
pub fn find(query: &str) -> Vec<Location> {
    keys(query)
        .iter()
        .filter_map(|key| TABLE.get(key))
        .flatten()
        .cloned()
        .collect()
}

// The table keys a query could mean: the whole thing, the part before a
// space ("K9J 7B8", "SW1A 1AA", "D02 X285"), and the same cuts when the
// space was left out.
pub fn keys(query: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let text = query.trim().to_uppercase();
    let whole: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    add(&mut keys, &whole);
    if let Some(space) = text.find(' ').filter(|&i| i > 0) {
        add(&mut keys, &text[..space]);
    }
    if UK_FULL.is_match(&whole) {
        add(&mut keys, &whole[..whole.len() - 3]);
    }
    if CANADIAN_OR_IRISH_FULL.is_match(&whole) {
        add(&mut keys, &whole[..3]);
    }
    keys
}

fn add(keys: &mut Vec<String>, key: &str) {
    if CODE_LIKE.is_match(key) && !keys.iter().any(|k| k == key) {
        keys.push(key.to_string());
    }
}

// Table entries first (the query was their code exactly), then the
// geocoder's, minus its answers for countries the table covers: a code the
// table knows in Canada must not also surface the geocoder's guess.
pub fn merge(table: Vec<Location>, geocoder: Vec<Location>) -> Vec<Location> {
    if table.is_empty() {
        return geocoder;
    }
    let mut merged = table;
    merged.extend(
        geocoder
            .into_iter()
            .filter(|l| l.country.as_deref().is_none_or(|c| !COUNTRIES.contains(&c))),
    );
    merged
}
