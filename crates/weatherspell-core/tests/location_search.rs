// CoordinatesTests.cs and PostalCodesTests.cs (with LocationSearchTests),
// ported, and the search's own behaviour with the network down.

mod common;

use std::collections::HashMap;
use std::time::Duration;

use common::fixture;
use weatherspell_core::coordinates;
use weatherspell_core::fetch::{Fetch, FetchError};
use weatherspell_core::location::Location;
use weatherspell_core::nominatim;
use weatherspell_core::open_meteo;
use weatherspell_core::postal_codes::{self, COUNTRIES};
use weatherspell_core::search::LocationSearch;

fn four(v: f64) -> f64 {
    (v * 10_000.0).round() / 10_000.0
}

#[test]
fn reads_a_point() {
    let cases: &[(&str, f64, f64)] = &[
        // Decimals, signed, with every separator people use.
        ("44.54, -78.54", 44.54, -78.54),
        ("44.54,-78.54", 44.54, -78.54),
        ("44.54 -78.54", 44.54, -78.54),
        ("44.54;-78.54", 44.54, -78.54),
        ("44.54 / -78.54", 44.54, -78.54),
        ("  (44.54, -78.54)  ", 44.54, -78.54),
        ("[44.54, -78.54]", 44.54, -78.54),
        ("+44.54 -78.54", 44.54, -78.54),
        ("44.54\t-78.54", 44.54, -78.54),
        ("-33.87 151.21", -33.87, 151.21),
        ("44, -78", 44.0, -78.0),
        ("0, 0", 0.0, 0.0),
        // Minus signs and dashes from word processors and web pages.
        ("44.54, \u{2212}78.54", 44.54, -78.54),
        ("44.54, \u{2013}78.54", 44.54, -78.54),
        // Compass letters and words, before or after, in either order.
        ("44.54 N 78.54 W", 44.54, -78.54),
        ("44.54N 78.54W", 44.54, -78.54),
        ("44.54N78.54W", 44.54, -78.54),
        ("N44.54 W78.54", 44.54, -78.54),
        ("N 44.54, W 78.54", 44.54, -78.54),
        ("44.54 north, 78.54 west", 44.54, -78.54),
        ("44.54 North 78.54 West", 44.54, -78.54),
        ("44.54\u{00B0} N, 78.54\u{00B0} W", 44.54, -78.54),
        ("44.54\u{00B0}N 78.54\u{00B0}W", 44.54, -78.54),
        ("78.54 W 44.54 N", 44.54, -78.54),
        ("W 78.54, N 44.54", 44.54, -78.54),
        ("-78.54 W, 44.54 N", 44.54, -78.54),
        ("33.87 S 151.21 E", -33.87, 151.21),
        ("33.87 south, 151.21 east", -33.87, 151.21),
        ("45.5 N 73.57 O", 45.5, -73.57),
        ("45.5 nord, 73.57 ouest", 45.5, -73.57),
        ("40.4 norte, 3.7 oeste", 40.4, -3.7),
        // Degrees, minutes and seconds.
        ("44\u{00B0}32'24\"N 78\u{00B0}32'24\"W", 44.54, -78.54),
        (
            "44\u{00B0}32\u{2032}24\u{2033}N 78\u{00B0}32\u{2032}24\u{2033}W",
            44.54,
            -78.54,
        ),
        (
            "44\u{00B0} 32\u{2019} 24\u{201D} N, 78\u{00B0} 32\u{2019} 24\u{201D} W",
            44.54,
            -78.54,
        ),
        ("44\u{00BA}32'24''N 78\u{00BA}32'24''W", 44.54, -78.54),
        (
            "44\u{00B0} 32' 24\" N, 78\u{00B0} 32' 24\" W",
            44.54,
            -78.54,
        ),
        ("44 32 24 N 78 32 24 W", 44.54, -78.54),
        ("N 44 32 24 W 78 32 24", 44.54, -78.54),
        ("44:32:24 N 78:32:24 W", 44.54, -78.54),
        ("44d 32m 24sec N, 78d 32m 24sec W", 44.54, -78.54),
        (
            "44 degrees 32 minutes 24 seconds north, 78 degrees 32 minutes 24 seconds west",
            44.54,
            -78.54,
        ),
        ("44 32 24, -78 32 24", 44.54, -78.54),
        ("44\u{00B0}32'24\" -78\u{00B0}32'24\"", 44.54, -78.54),
        ("33\u{00B0}52'12\"S 151\u{00B0}12'36\"E", -33.87, 151.21),
        // Degrees and decimal minutes, as GPS units and geocaching write them.
        ("N 44\u{00B0} 32.400 W 078\u{00B0} 32.400", 44.54, -78.54),
        ("N44 32.4 W78 32.4", 44.54, -78.54),
        ("44 32.4 N 78 32.4 W", 44.54, -78.54),
        ("44 32.4, -78 32.4", 44.54, -78.54),
        ("44\u{00B0} 32.4' -78\u{00B0} 32.4'", 44.54, -78.54),
        // Decimal commas, as a French or German Windows writes numbers.
        ("44,54 -78,54", 44.54, -78.54),
        ("44,54; -78,54", 44.54, -78.54),
        ("44,54, -78,54", 44.54, -78.54),
        ("44,54,-78,54", 44.54, -78.54),
        ("45,5 N 73,57 O", 45.5, -73.57),
        // Labels, in either order.
        ("lat 44.54 lon -78.54", 44.54, -78.54),
        ("Latitude: 44.54, Longitude: -78.54", 44.54, -78.54),
        ("lon -78.54 lat 44.54", 44.54, -78.54),
        ("lat=44.54&lng=-78.54", 44.54, -78.54),
        // Longitude first (GeoJSON), when the first number cannot be a
        // latitude; "-78.54, 44.54" is a real point in Antarctica, so it
        // stays as typed.
        ("-93.27, 44.98", 44.98, -93.27),
        ("-78.54, 44.54", -78.54, 44.54),
        // Map links: the pin where there is one, else the view's centre.
        (
            "https://www.google.com/maps/place/Bobcaygeon/@44.54,-78.54,15z",
            44.54,
            -78.54,
        ),
        (
            "https://www.google.com/maps/place/X/@44.6,-78.6,15z/data=!3m1!4b1!4m6!3m5!1s0x0:0x0!8m2!3d44.54!4d-78.54",
            44.54,
            -78.54,
        ),
        ("https://maps.google.com/?q=44.54,-78.54", 44.54, -78.54),
        (
            "https://www.google.com/maps/search/?api=1&query=44.54%2C-78.54",
            44.54,
            -78.54,
        ),
        ("maps.google.com/?q=44.54,-78.54", 44.54, -78.54),
        (
            "https://www.openstreetmap.org/?mlat=44.54&mlon=-78.54#map=15/44.60/-78.60",
            44.54,
            -78.54,
        ),
        (
            "https://www.openstreetmap.org/#map=15/44.54/-78.54",
            44.54,
            -78.54,
        ),
        ("geo:44.54,-78.54", 44.54, -78.54),
        ("geo:44.54,-78.54;u=35", 44.54, -78.54),
        (
            "https://maps.apple.com/?ll=44.54,-78.54&q=Dropped%20Pin",
            44.54,
            -78.54,
        ),
        (
            "https://www.bing.com/maps?cp=44.54~-78.54&lvl=15",
            44.54,
            -78.54,
        ),
    ];
    for &(text, latitude, longitude) in cases {
        let reading = coordinates::read(text).unwrap_or_else(|| panic!("no reading for {text:?}"));
        assert_eq!(reading.problem, None, "{text:?}");
        assert_eq!(
            (four(reading.latitude), four(reading.longitude)),
            (latitude, longitude),
            "{text:?}"
        );
    }
}

#[test]
fn leaves_names_and_postal_codes_to_the_search() {
    for text in [
        "Peterborough",
        "New York",
        "St. Catharines",
        "Paris 75004",
        "Route 66",
        "K9J 7B8",
        "SW1A 1AA",
        "M1",
        "N1",
        "M1 1AE",
        "E1 6AN",
        "D02 X285",
        "1011 AB",
        "2000",
        "90210",
        "12345-6789",
        "060-0001",
        "01310-100",
        "114 55",
        "110 00",
        "12345, 6789",
        "44",
        "44.54",
        "44.54 N",
        "",
        "   ",
    ] {
        assert_eq!(coordinates::read(text), None, "{text:?}");
    }
}

#[test]
fn says_why_coordinates_cannot_be_used() {
    for (text, problem) in [
        (
            "95.5 N, 78.54 W",
            "Couldn't use 95.5 as a latitude: it must be from 90 south to 90 north.",
        ),
        (
            "100.5, 200.5",
            "Couldn't use 100.5 as a latitude: it must be from 90 south to 90 north.",
        ),
        (
            "44.54, 200.5",
            "Couldn't use 200.5 as a longitude: it must be from 180 west to 180 east.",
        ),
        (
            "44 65 N 78 32 W",
            "Couldn't read those coordinates: minutes and seconds must be under 60.",
        ),
        (
            "44.54 N 78.54 N",
            "Couldn't read those coordinates: both are latitudes. Type a latitude and then a longitude, such as 44.54, -78.54.",
        ),
        (
            "44.54 E 78.54 W",
            "Couldn't read those coordinates: both are longitudes. Type a latitude and then a longitude, such as 44.54, -78.54.",
        ),
        (
            "44.54 -78.54 12.3",
            "Couldn't read those coordinates. Type a latitude and then a longitude, such as 44.54, -78.54.",
        ),
        (
            "-44.54 N, 78.54 W",
            "Couldn't read those coordinates. Type a latitude and then a longitude, such as 44.54, -78.54.",
        ),
        (
            "44.54, -78.54, 12.3",
            "Couldn't read those coordinates. Type a latitude and then a longitude, such as 44.54, -78.54.",
        ),
        (
            "https://maps.app.goo.gl/AbCdEf123",
            "That link has no coordinates in it. Copy the coordinates themselves, or search for the place by name.",
        ),
        (
            "https://www.google.com/maps/@95.5,-78.54,15z",
            "Couldn't use 95.5 as a latitude: it must be from 90 south to 90 north.",
        ),
    ] {
        let reading = coordinates::read(text).unwrap_or_else(|| panic!("no reading for {text:?}"));
        assert_eq!(reading.problem.as_deref(), Some(problem), "{text:?}");
    }
}

#[test]
fn says_a_point_in_words() {
    for (latitude, longitude, words) in [
        (44.54, -78.54, "44.54 north, 78.54 west"),
        (-33.8688, 151.2093, "33.8688 south, 151.2093 east"),
        (44.543219, -78.5, "44.5432 north, 78.5 west"),
        (0.0, 0.0, "0 north, 0 east"),
        (-0.00001, 0.00001, "0 north, 0 east"),
    ] {
        assert_eq!(coordinates::words(latitude, longitude), words);
    }
}

fn reverse(name: &str, latitude: f64, longitude: f64) -> Option<Location> {
    nominatim::parse_reverse(&fixture(name), latitude, longitude).unwrap()
}

#[test]
fn names_a_point_after_the_smallest_settlement_and_keeps_the_point_typed() {
    let place = reverse("nominatim-bobcaygeon.json", 44.54, -78.54).unwrap();

    // The town, not the amalgamated city of Kawartha Lakes around it, and
    // the cottage's point, not the town's.
    assert_eq!(place.name, "Bobcaygeon");
    assert_eq!(place.region.as_deref(), Some("Ontario"));
    assert_eq!(place.country.as_deref(), Some("Canada"));
    assert_eq!((place.latitude, place.longitude), (44.54, -78.54));
    assert_eq!(
        place.near_text(),
        "Near Bobcaygeon, Ontario, Canada (44.54 north, 78.54 west)"
    );
}

#[test]
fn prefers_a_village_to_the_us_town_it_is_in() {
    let place = reverse("nominatim-fort-hunter.json", 42.75, -73.95).unwrap();
    assert_eq!(place.full_name(), "Fort Hunter, New York, United States");
    assert!(place.is_nws_covered());
}

#[test]
fn calls_a_us_territory_by_its_name_as_the_geocoder_does() {
    // The city, not the Viejo San Juan quarter Nominatim matched (#19).
    let place = reverse("nominatim-san-juan.json", 18.4655, -66.1057).unwrap();
    assert_eq!(place.full_name(), "San Juan, Puerto Rico");
    assert!(place.is_nws_covered());
}

#[test]
fn names_a_point_with_no_settlement_after_its_area() {
    assert_eq!(
        reverse("nominatim-kitikmeot.json", 70.0, -95.0)
            .unwrap()
            .full_name(),
        "Kitikmeot Region, Nunavut, Canada"
    );
}

#[test]
fn a_point_nothing_names_goes_by_its_coordinates() {
    assert_eq!(reverse("nominatim-atlantic.json", 35.0, -40.0), None);
    let point = Location::at_point(35.0, -40.0);
    assert_eq!(point.full_name(), "35 north, 40 west");
    assert_eq!(point.country, None);
}

#[test]
fn asks_for_the_point_in_invariant_numbers() {
    let url = nominatim::reverse_url(44.543219, -78.5);
    assert!(url.starts_with("https://nominatim.openstreetmap.org/reverse?"));
    assert!(url.ends_with("&lat=44.543219&lon=-78.5"));
}

// --- PostalCodesTests

#[test]
fn reads_the_part_of_a_code_the_table_keys_on() {
    for (query, key) in [
        ("K9J", "K9J"),
        ("k9j 7b8", "K9J"),
        ("K9J7B8", "K9J"),
        ("  K9J 7  ", "K9J"),
        ("M1", "M1"),
        ("m1 1ae", "M1"),
        ("M11AE", "M1"),
        ("SW1A 1AA", "SW1A"),
        ("sw1a1aa", "SW1A"),
        ("D02 X285", "D02"),
        ("D02X285", "D02"),
        ("2000", "2000"),
    ] {
        assert!(
            postal_codes::keys(query).iter().any(|k| k == key),
            "{query:?}"
        );
    }
}

#[test]
fn finds_nothing_for_place_names_and_codes_of_other_countries() {
    for query in ["Peterborough", "New York", "49620", "Dublin 2", ""] {
        assert!(postal_codes::find(query).is_empty(), "{query:?}");
    }
}

fn single(code: &str) -> Location {
    let found = postal_codes::find(code);
    assert_eq!(found.len(), 1, "{code}");
    found[0].clone()
}

#[test]
fn a_full_canadian_code_resolves_to_its_forward_sortation_area() {
    let place = single("K9J 7B8");
    // The municipality and the median of the area's full codes (#18).
    assert_eq!(place.name, "Peterborough");
    assert_eq!(place.region.as_deref(), Some("Ontario"));
    assert_eq!(place.country.as_deref(), Some("Canada"));
    assert_eq!(place.search_result_text(), "Peterborough, Ontario, Canada");
    assert_eq!(
        (four(place.latitude), four(place.longitude)),
        (44.2924, -78.3296)
    );
}

#[test]
fn canadian_areas_sit_where_they_are_under_a_plain_name() {
    for (code, latitude, longitude) in [
        ("T2P", 51.0447, -114.0719),
        ("L8N", 43.2557, -79.8711),
        ("M5V", 43.6426, -79.3871),
    ] {
        let place = single(code);
        assert!((place.latitude - latitude).abs() <= 0.03, "{code}");
        assert!((place.longitude - longitude).abs() <= 0.03, "{code}");
        assert!(!place.name.contains('('), "{code}");
    }
}

#[test]
fn a_code_shared_by_several_places_is_one_entry_named_after_the_largest() {
    // L9X covers Barrie and three townships; E17 lists a tube station before
    // Walthamstow; IP8 is thirteen Suffolk villages.
    assert_eq!(single("L9X").full_name(), "Barrie, Ontario, Canada");
    assert_eq!(
        single("E17").full_name(),
        "Walthamstow, England, United Kingdom"
    );
    assert_eq!(
        single("IP8").full_name(),
        "Bramford, England, United Kingdom"
    );
    assert_eq!(
        single("2000").full_name(),
        "Sydney, New South Wales, Australia"
    );
}

#[test]
fn a_code_used_in_two_countries_lists_both() {
    let names: Vec<String> = postal_codes::find("1010")
        .iter()
        .map(Location::full_name)
        .collect();
    assert_eq!(
        names,
        [
            "Sydney, New South Wales, Australia",
            "Auckland, New Zealand"
        ]
    );
}

#[test]
fn finds_codes_in_each_covered_country() {
    for (query, full_name) in [
        ("M1 1AE", "Manchester, England, United Kingdom"),
        ("EH1", "Edinburgh, Scotland, United Kingdom"),
        ("BT62", "Craigavon, Northern Ireland, United Kingdom"),
        ("3000", "Melbourne, Victoria, Australia"),
        ("6011", "Mount Victoria, Wellington, New Zealand"),
        ("D02 X285", "Dublin 2, Leinster, Ireland"),
        ("T12", "Cork city southside, Munster, Ireland"),
        ("H0H 0H0", "Reserved (Santa Claus), Quebec, Canada"),
    ] {
        assert!(
            postal_codes::find(query)
                .iter()
                .any(|p| p.full_name() == full_name),
            "{query:?}"
        );
    }
}

#[test]
fn territories_are_named_as_the_geocoder_names_them() {
    assert_eq!(
        single("X1A").region.as_deref(),
        Some("Northwest Territories")
    );
    assert_eq!(single("X0A").region.as_deref(), Some("Nunavut"));
}

#[test]
fn a_region_the_data_leaves_blank_is_left_out_of_the_name() {
    assert_eq!(single("JE4").full_name(), "Jersey, United Kingdom");
}

#[test]
fn the_embedded_table_parses_completely_and_plausibly() {
    // The exe's embedded copy, read the way the app reads it, so an accented
    // name or a regenerated file with a bad row shows up here.
    let mut rows = 0;
    let mut countries = std::collections::BTreeSet::new();
    for line in postal_codes::TABLE_TEXT.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        rows += 1;
        let code = &line[..line.find('\t').unwrap()];
        let found = postal_codes::find(code);
        assert!(!found.is_empty(), "{code}");
        for p in found {
            countries.insert(p.country.clone().unwrap());
            assert!(!p.name.trim().is_empty());
            assert!(COUNTRIES.contains(&p.country.as_deref().unwrap()));
            if code == "H0H" {
                continue; // Santa Claus, at the North Pole
            }
            assert!((-55.0..=84.0).contains(&p.latitude), "{code}");
            assert!((-142.0..=180.0).contains(&p.longitude), "{code}");
        }
    }
    assert!((9000..=12000).contains(&rows));
    let mut expected: Vec<&str> = COUNTRIES.to_vec();
    expected.sort();
    assert_eq!(
        countries.iter().map(String::as_str).collect::<Vec<_>>(),
        expected
    );
    assert_eq!(single("E8T").name, "Lam\u{e8}que");
}

// --- LocationSearchTests

fn at(name: &str, region: Option<&str>, country: Option<&str>) -> Location {
    Location::new(name, region, country, 0.0, 0.0)
}

#[test]
fn table_entries_come_first_and_replace_the_geocoder_for_their_countries() {
    let merged = postal_codes::merge(
        vec![at("Sydney", Some("New South Wales"), Some("Australia"))],
        vec![
            at("Antwerpen", Some("Flanders"), Some("Belgium")),
            at("Somewhere", Some("Queensland"), Some("Australia")),
            at("Elsewhere", None, None),
        ],
    );
    let names: Vec<&str> = merged.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["Sydney", "Antwerpen", "Elsewhere"]);
}

#[test]
fn without_table_entries_the_geocoder_answer_is_untouched() {
    // A place-name search must still find places in covered countries.
    let geocoder = vec![at("Peterborough", Some("Ontario"), Some("Canada"))];
    assert_eq!(postal_codes::merge(Vec::new(), geocoder.clone()), geocoder);
}

// --- The search itself

struct Answers(HashMap<String, Result<String, FetchError>>);

impl Fetch for Answers {
    fn get(&self, url: &str, _timeout: Option<Duration>) -> Result<String, FetchError> {
        self.0.get(url).cloned().unwrap_or_else(|| {
            Err(FetchError::Unreachable {
                host: "nowhere".into(),
                detail: "no network".into(),
            })
        })
    }
}

#[test]
fn a_postal_code_is_found_with_the_network_down() {
    let offline = Answers(HashMap::new());
    let search = LocationSearch::new();

    // The table answers on its own; a place name has nothing to fall back on.
    let found = search.search(&offline, "K9J 7B8").unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].full_name(), "Peterborough, Ontario, Canada");
    assert!(search.search(&offline, "Peterborough").is_err());
}

#[test]
fn a_name_is_searched_with_the_geocoder_and_merged() {
    let online = Answers(HashMap::from([(
        open_meteo::search_url("Peterborough"),
        Ok(fixture("geocoding-peterborough.json")),
    )]));
    let found = LocationSearch::new()
        .search(&online, "Peterborough")
        .unwrap();
    assert!(
        found
            .iter()
            .any(|l| l.full_name() == "Peterborough, Ontario, Canada")
    );
}

#[test]
fn a_typed_point_is_named_and_a_second_lookup_waits_its_turn() {
    let http = Answers(HashMap::from([(
        nominatim::reverse_url(44.54, -78.54),
        Ok(fixture("nominatim-bobcaygeon.json")),
    )]));
    let search = LocationSearch::new();
    let started = std::time::Instant::now();
    let first = search.name_point(&http, 44.54, -78.54).unwrap().unwrap();
    let second = search.name_point(&http, 44.54, -78.54).unwrap().unwrap();
    assert_eq!(first.name, "Bobcaygeon");
    assert_eq!(second, first);
    // Nominatim's policy: at most one request a second.
    assert!(started.elapsed() >= Duration::from_millis(950));
}
