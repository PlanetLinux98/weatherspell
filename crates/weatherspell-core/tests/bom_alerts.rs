// Alerts for Australia from the Bureau of Meteorology: its outlines
// (zipped shapefiles), its CAP files, which warnings reach a location, and
// what is kept between checks and sessions. The Bureau's terms do not let
// its files be passed on, so nothing here is the Bureau's: the outlines
// are squares made below, and the CAP files are written in the Bureau's
// form with words of our own.

mod common;

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::sync::Mutex;
use std::time::Duration;

use common::TempDir;
use jiff::Timestamp;
use weatherspell_core::alerts::{self, AlertSeverity, WeatherAlert};
use weatherspell_core::bom;
use weatherspell_core::cap;
use weatherspell_core::fetch::{Fetch, FetchError, ForecastService};
use weatherspell_core::location::Location;

// --- Files in the Bureau's forms

// A square's code, name, parent, south edge, west edge and size in degrees.
type Square<'a> = (&'a str, &'a str, &'a str, f64, f64, f64);

fn shp(squares: &[Square]) -> Vec<u8> {
    let mut records = Vec::new();
    for (n, &(_, _, _, south, west, size)) in squares.iter().enumerate() {
        let ring = [
            (west, south),
            (west, south + size),
            (west + size, south + size),
            (west + size, south),
            (west, south),
        ];
        let mut content = Vec::new();
        content.extend(5i32.to_le_bytes());
        for v in [west, south, west + size, south + size] {
            content.extend(v.to_le_bytes());
        }
        content.extend(1i32.to_le_bytes());
        content.extend((ring.len() as i32).to_le_bytes());
        content.extend(0i32.to_le_bytes());
        for (x, y) in ring {
            content.extend(x.to_le_bytes());
            content.extend(y.to_le_bytes());
        }
        records.extend((n as i32 + 1).to_be_bytes());
        records.extend((content.len() as i32 / 2).to_be_bytes());
        records.extend(content);
    }
    let mut file = vec![0u8; 100];
    file[..4].copy_from_slice(&9994i32.to_be_bytes());
    file[24..28].copy_from_slice(&((100 + records.len()) as i32 / 2).to_be_bytes());
    file[28..32].copy_from_slice(&1000i32.to_le_bytes());
    file[32..36].copy_from_slice(&5i32.to_le_bytes());
    file.extend(records);
    file
}

fn dbf(product: &str, squares: &[Square]) -> Vec<u8> {
    let fields = if product == "IDM00016" {
        [("AAC", 12), ("ATWS_Zones", 40), ("TWZ", 12)]
    } else {
        [("AAC", 12), ("DIST_NAME", 40), ("AAC_PARENT", 12)]
    };
    let record_length = 1 + fields.iter().map(|(_, w)| w).sum::<usize>();
    let mut file = vec![0u8; 32];
    file[0] = 3;
    file[4..8].copy_from_slice(&(squares.len() as u32).to_le_bytes());
    file[8..10].copy_from_slice(&((32 + 32 * fields.len() + 1) as u16).to_le_bytes());
    file[10..12].copy_from_slice(&(record_length as u16).to_le_bytes());
    for (name, width) in fields {
        let mut descriptor = [0u8; 32];
        descriptor[..name.len()].copy_from_slice(name.as_bytes());
        descriptor[11] = b'C';
        descriptor[16] = width as u8;
        file.extend(descriptor);
    }
    file.push(0x0D);
    for &(code, name, parent, ..) in squares {
        file.push(b' ');
        for (value, (_, width)) in [code, name, parent].into_iter().zip(fields) {
            file.extend(format!("{value:<width$}").as_bytes());
        }
    }
    file.push(0x1A);
    file
}

// A zip archive, the .shp stored and the .dbf deflated, as either comes.
fn zip(product: &str, squares: &[Square]) -> Vec<u8> {
    let mut deflated =
        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    deflated.write_all(&dbf(product, squares)).unwrap();
    let entries = [
        (
            format!("{product}.shp"),
            0u16,
            shp(squares),
            shp(squares).len(),
        ),
        (
            format!("{product}.dbf"),
            8u16,
            deflated.finish().unwrap(),
            dbf(product, squares).len(),
        ),
    ];
    let mut file = Vec::new();
    let mut central = Vec::new();
    for (name, method, data, size) in &entries {
        let offset = file.len() as u32;
        file.extend(b"PK\x03\x04");
        file.extend(20u16.to_le_bytes());
        file.extend(0u16.to_le_bytes());
        file.extend(method.to_le_bytes());
        file.extend([0u8; 8]);
        file.extend((data.len() as u32).to_le_bytes());
        file.extend((*size as u32).to_le_bytes());
        file.extend((name.len() as u16).to_le_bytes());
        file.extend(0u16.to_le_bytes());
        file.extend(name.as_bytes());
        file.extend(data);
        central.extend(b"PK\x01\x02");
        central.extend(20u16.to_le_bytes());
        central.extend(20u16.to_le_bytes());
        central.extend(0u16.to_le_bytes());
        central.extend(method.to_le_bytes());
        central.extend([0u8; 8]);
        central.extend((data.len() as u32).to_le_bytes());
        central.extend((*size as u32).to_le_bytes());
        central.extend((name.len() as u16).to_le_bytes());
        central.extend([0u8; 12]);
        central.extend(offset.to_le_bytes());
        central.extend(name.as_bytes());
    }
    let at = file.len() as u32;
    file.extend(&central);
    file.extend(b"PK\x05\x06");
    file.extend([0u8; 4]);
    file.extend((entries.len() as u16).to_le_bytes());
    file.extend((entries.len() as u16).to_le_bytes());
    file.extend((central.len() as u32).to_le_bytes());
    file.extend(at.to_le_bytes());
    file.extend(0u16.to_le_bytes());
    file
}

// A Testland location at 29.5 south, 150.5 east: in a forecast district
// and a small catchment, which lies in a larger one.
const LAT: f64 = -29.5;
const LON: f64 = 150.5;

fn layers() -> Vec<(&'static str, Vec<Square<'static>>)> {
    // Every layer holds something, here a square far away.
    let far = |code| (code, "Far Away", "", -40.0, 120.0, 1.0);
    vec![
        (
            "IDM00001",
            vec![
                ("NSW_PW901", "Testland", "", -30.0, 150.0, 1.0),
                ("NSW_PW902", "South Testland", "", -31.0, 150.0, 1.0),
            ],
        ),
        ("IDM00014", vec![far("NSW_ME901")]),
        ("IDM00007", vec![far("NSW_FW901")]),
        (
            "IDM00017",
            vec![
                ("QLD_RC900", "Test River", "", -31.0, 149.0, 3.0),
                (
                    "QLD_RC901",
                    "Test River at Testville",
                    "QLD_RC900",
                    -29.6,
                    150.4,
                    0.2,
                ),
            ],
        ),
        ("IDM00020", vec![far("NSW_FL901")]),
        (
            "IDM00003",
            vec![("NSW_MW901", "Testland Coast", "", -30.0, 151.0, 1.0)],
        ),
        // The coast is the zone's west edge, at 151 east.
        (
            "IDM00016",
            vec![(
                "NSW_TW901",
                "Testland Coast",
                "NSW_TSW901",
                -30.0,
                151.0,
                1.0,
            )],
        ),
    ]
}

fn layer_line(product: &str, size: usize) -> String {
    format!("-rw-rw-r--    1 4466     1050      {size:>7} Feb 15  2024 {product}.zip")
}

fn warning_line(name: &str, time: &str) -> String {
    format!("-rw-rw-r--    1 1050     1502         4294 Oct 26 {time} {name}")
}

const THUNDERSTORM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<alert xmlns="urn:oasis:names:tc:emergency:cap:1.2">
  <identifier>AusBoM-IDN99001-2026-10-26T02:00:00+00:00</identifier>
  <sender>CAP.Message@bom.gov.au</sender>
  <sent>2026-10-26T02:00:00+00:00</sent>
  <status>Actual</status>
  <msgType>Alert</msgType>
  <scope>Public</scope>
  <info>
    <language>en-AU</language>
    <category>Met</category>
    <event>Thunderstorm</event>
    <urgency>Unknown</urgency>
    <severity>Unknown</severity>
    <certainty>Unknown</certainty>
    <expires>2026-10-26T07:00:00+00:00</expires>
    <senderName>Australian Government Bureau of Meteorology</senderName>
    <headline>Severe Thunderstorm Warning for Testland and South Testland forecast districts</headline>
    <description>
      Storms with large hail are likely in the next few hours, with 30-50 mm of rain in an hour.
    </description>
    <instruction>Test advice:
* Keep clear of creeks.
* Stay indoors.
</instruction>
    <web>http://www.bom.gov.au/australia/warnings</web>
    <area>
      <areaDesc>New South Wales: Testland and South Testland</areaDesc>
      <geocode><valueName>AMOC-AreaCode</valueName><value>NSW_PW901</value></geocode>
      <geocode><valueName>AMOC-AreaCode</valueName><value>NSW_PW902</value></geocode>
    </area>
  </info>
</alert>
"#;

const MARINE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<alert xmlns="urn:oasis:names:tc:emergency:cap:1.2">
  <identifier>AusBoM-IDN99002-2026-10-26T01:00:00+00:00</identifier>
  <sent>2026-10-26T01:00:00+00:00</sent>
  <status>Actual</status>
  <msgType>Alert</msgType>
  <info>
    <event>Wind</event>
    <severity>Moderate</severity>
    <expires>2026-10-26T13:00:00+00:00</expires>
    <headline>Strong Wind Warning for the Testland Coast</headline>
    <description>Strong winds at sea.</description>
    <area>
      <areaDesc>New South Wales: Testland Coast</areaDesc>
      <geocode><valueName>AMOC-AreaCode</valueName><value>NSW_MW901</value></geocode>
    </area>
  </info>
</alert>
"#;

fn flood(severity: &str, kind: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<alert xmlns="urn:oasis:names:tc:emergency:cap:1.2">
  <identifier>AusBoM-IDQ99003-2026-10-26T01:30:00+00:00</identifier>
  <sent>2026-10-26T01:30:00+00:00</sent>
  <status>Actual</status>
  <msgType>Update</msgType>
  <info>
    <event>Riverine Flood</event>
    <severity>{severity}</severity>
    <expires>2026-10-27T04:30:00+00:00</expires>
    <headline>{kind} Flood Warning for the Test River</headline>
    <description>The Test River at Testville is at 4.15 m and falling.</description>
    <area>
      <areaDesc>Queensland: Test River</areaDesc>
      <geocode><valueName>AMOC-AreaCode</valueName><value>QLD_RC900</value></geocode>
    </area>
  </info>
  <info>
    <event>Riverine Flood</event>
    <severity>Minor</severity>
    <expires>2026-10-27T04:30:00+00:00</expires>
    <headline>Cancellation of Flood Warning for Testville Creek</headline>
    <description>Flooding has eased.</description>
    <area>
      <areaDesc>Queensland: Testville Creek</areaDesc>
      <geocode><valueName>AMOC-AreaCode</valueName><value>QLD_RC901</value></geocode>
    </area>
  </info>
  <info>
    <event>Riverine Flood</event>
    <severity>Unknown</severity>
    <expires>2026-10-25T04:30:00+00:00</expires>
    <headline>Flood Warning for Old Creek</headline>
    <description>Yesterday's warning.</description>
    <area>
      <areaDesc>Queensland: Old Creek</areaDesc>
      <geocode><valueName>AMOC-AreaCode</valueName><value>QLD_RC901</value></geocode>
    </area>
  </info>
</alert>
"#
    )
}

// The national warning centre's, naming the zone by its second code.
const TSUNAMI: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<alert xmlns="urn:oasis:names:tc:emergency:cap:1.2">
  <identifier>AusBoM-IDY68028-2026-10-26T02:30:00+00:00</identifier>
  <sent>2026-10-26T02:30:00+00:00</sent>
  <status>Actual</status>
  <msgType>Alert</msgType>
  <info>
    <event>Tsunami</event>
    <severity>Unknown</severity>
    <expires>2026-10-26T06:30:00+00:00</expires>
    <headline>Tsunami Warning for the Testland Coast</headline>
    <description>A test wave is expected along the coast.</description>
    <area>
      <areaDesc>New South Wales: Testland Coast</areaDesc>
      <geocode><valueName>AMOC-AreaCode</valueName><value>NSW_TSW901</value></geocode>
    </area>
  </info>
</alert>
"#;

const CANCELLED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<alert xmlns="urn:oasis:names:tc:emergency:cap:1.2">
  <identifier>AusBoM-IDV99004-2026-10-26T00:00:00+00:00</identifier>
  <sent>2026-10-26T00:00:00+00:00</sent>
  <status>Actual</status>
  <msgType>Cancel</msgType>
  <info>
    <event>Sheep Grazier Warning</event>
    <headline>Cancellation of Warning to Sheep Graziers for Testland</headline>
    <area>
      <geocode><valueName>AMOC-AreaCode</valueName><value>NSW_PW901</value></geocode>
    </area>
  </info>
</alert>
"#;

// --- A network that answers from these

#[derive(Default)]
struct Fake {
    answers: Mutex<HashMap<String, Vec<u8>>>,
    asked: Mutex<Vec<String>>,
}

impl Fake {
    fn bureau() -> Fake {
        let fake = Fake::default();
        let mut listing = Vec::new();
        for (product, squares) in layers() {
            let file = zip(product, &squares);
            listing.push(layer_line(product, file.len()));
            fake.set(
                &format!("ftp://ftp.bom.gov.au/anon/home/adfd/spatial/{product}.zip"),
                file,
            );
        }
        fake.set(bom::LAYERS_URL, listing.join("\n").into_bytes());
        fake.set_warnings("02:00");
        fake.set(&bom::warning_url("IDN99001.cap.xml"), THUNDERSTORM.into());
        fake.set(&bom::warning_url("IDN99002.cap.xml"), MARINE.into());
        fake.set(
            &bom::warning_url("IDQ99003.cap.xml"),
            flood("Minor", "Minor").into(),
        );
        fake.set(&bom::warning_url("IDV99004.cap.xml"), CANCELLED.into());
        fake.set(&bom::warning_url("IDY68028.cap.xml"), TSUNAMI.into());
        fake
    }

    // The warnings listing, the thunderstorm's file last changed at time.
    fn set_warnings(&self, time: &str) {
        let listing = [
            warning_line("IDN99001.cap.xml", time),
            warning_line("IDN99002.cap.xml", "01:00"),
            warning_line("IDQ99003.cap.xml", "01:30"),
            warning_line("IDV99004.cap.xml", "00:00"),
            warning_line("IDN99005.txt", "00:00"),
            warning_line("IDY68028.cap.xml", "02:30"),
        ];
        self.set(bom::WARNINGS_URL, listing.join("\n").into_bytes());
    }

    fn set(&self, url: &str, body: Vec<u8>) {
        self.answers.lock().unwrap().insert(url.to_string(), body);
    }

    fn asked(&self) -> Vec<String> {
        std::mem::take(&mut *self.asked.lock().unwrap())
    }
}

impl Fetch for Fake {
    fn get(&self, url: &str, timeout: Option<Duration>) -> Result<String, FetchError> {
        self.get_bytes(url, timeout)
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    }

    fn get_bytes(&self, url: &str, _timeout: Option<Duration>) -> Result<Vec<u8>, FetchError> {
        self.asked.lock().unwrap().push(url.to_string());
        self.answers
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .ok_or(FetchError::Unreachable {
                host: "ftp.bom.gov.au".into(),
                detail: "refused".into(),
            })
    }
}

fn at(stamp: &str) -> Timestamp {
    stamp.parse().unwrap()
}

const NOW: &str = "2026-10-26T03:00:00Z";

fn testville() -> Location {
    Location::new(
        "Testville",
        Some("New South Wales"),
        Some("Australia"),
        LAT,
        LON,
    )
}

fn events(alerts: &[WeatherAlert]) -> Vec<&str> {
    alerts.iter().map(|a| a.event.as_str()).collect()
}

fn layer_urls(asked: &[String]) -> Vec<&str> {
    asked
        .iter()
        .map(String::as_str)
        .filter(|u| u.ends_with(".zip") && !u.contains('*'))
        .collect()
}

// --- Outlines

#[test]
fn a_layer_is_read_from_the_bureaus_zipped_shapefile() {
    let (product, squares) = layers().remove(3);
    let layer = bom::LAYERS.iter().find(|l| l.product == product).unwrap();
    let regions = bom::read_layer(&zip(product, &squares), layer).unwrap();
    let codes: Vec<(&str, &str, Option<&str>)> = regions
        .iter()
        .map(|r| (r.code.as_str(), r.name.as_str(), r.parent.as_deref()))
        .collect();
    assert_eq!(
        codes,
        [
            ("QLD_RC900", "Test River", None),
            ("QLD_RC901", "Test River at Testville", Some("QLD_RC900"))
        ]
    );
    assert!(regions[1].covers(LAT, LON, 0.0));
    assert!(!regions[1].covers(-29.0, LON, 0.0));

    // As the cache folder keeps it, and back.
    let text = bom::layer_text(layer, "4349010 Jul 2", &regions);
    assert_eq!(
        bom::parse_layer_text(&text),
        Some(("4349010 Jul 2".to_string(), regions))
    );
    assert!(bom::read_layer(b"not a zip", layer).is_err());
}

#[test]
fn a_layers_version_is_its_size_and_day_whatever_its_age() {
    assert_eq!(
        bom::layer_version("-rw-r--r--    1 1050     1050     14099518 Aug 19 04:08 IDM00007.zip"),
        "14099518 Aug 19"
    );
    assert_eq!(
        bom::layer_version("-rw-r--r--    1 1050     1050     14099518 Aug 19  2026 IDM00007.zip"),
        "14099518 Aug 19"
    );
}

// --- Which warnings reach a location

#[test]
fn a_location_hears_the_warnings_for_its_areas_and_the_larger_ones() {
    let fake = Fake::bureau();
    let report = ForecastService::new().alerts(&fake, &testville(), at(NOW));
    assert_eq!(report.problem, None);
    assert_eq!(
        report.attribution.as_deref(),
        Some("\u{a9} Bureau of Meteorology (bom.gov.au)")
    );
    // The marine warning is at sea, and the tsunami warning's zone is too
    // far away; the flood file's cancellation and its ended warning are
    // dropped, and Victoria's file and the tsunami's are not even fetched.
    assert_eq!(
        events(&report.alerts),
        ["Severe thunderstorm warning", "Minor flood warning"]
    );
    let asked = fake.asked();
    assert!(asked.iter().all(|u| !u.contains("IDV99004")));
    assert!(asked.iter().all(|u| !u.contains("IDN99005")));
    assert!(asked.iter().all(|u| !u.contains("IDY68028")));

    let storm = &report.alerts[0];
    assert_eq!(storm.id, "au:IDN99001/NSW_PW901");
    // The Bureau rates it Unknown; the kind ranks it.
    assert_eq!(storm.severity, AlertSeverity::Severe);
    assert_eq!(storm.source, "the Bureau of Meteorology");
    assert_eq!(storm.sender, "the Bureau of Meteorology");
    assert_eq!(storm.ends, None);
    assert_eq!(storm.expires, Some(at("2026-10-26T07:00:00Z")));
    assert_eq!(storm.area, "New South Wales: Testland and South Testland");
    assert_eq!(
        storm.description,
        "Severe Thunderstorm Warning for Testland and South Testland forecast districts\n\n\
         Storms with large hail are likely in the next few hours, with 30 to 50 millimetres of rain in an hour."
    );
    assert_eq!(
        storm.instruction.as_deref(),
        Some("Test advice:\n- Keep clear of creeks.\n- Stay indoors.")
    );
    assert_eq!(
        storm.url.as_deref(),
        Some("https://www.bom.gov.au/australia/warnings")
    );

    let flood = &report.alerts[1];
    assert_eq!(flood.id, "au:IDQ99003/QLD_RC900");
    assert_eq!(flood.severity, AlertSeverity::Minor);
    assert_eq!(
        flood.description,
        "Minor Flood Warning for the Test River\n\nThe Test River at Testville is at 4.15 metres and falling."
    );
}

#[test]
fn a_location_in_no_area_has_no_alerts_and_one_the_ftp_refuses_is_told_why() {
    let fake = Fake::bureau();
    let service = ForecastService::new();
    let offshore = Location::new("Nowhere", None, Some("Australia"), -20.0, 100.0);
    let report = service.alerts(&fake, &offshore, at(NOW));
    assert!(report.checked());
    assert!(report.alerts.is_empty());

    let report = ForecastService::new().alerts(&Fake::default(), &testville(), at(NOW));
    assert!(report.is_available());
    assert!(!report.checked());
    assert_eq!(
        report.problem.as_deref(),
        Some("ftp.bom.gov.au could not be reached")
    );
}

#[test]
fn a_tsunami_warning_reaches_three_kilometres_inland() {
    // At 29.5 south a degree of longitude is about 97 km: the beach is
    // 1.9 km from the zone, the hill 3.9 km.
    let beach = Location::new(
        "Testbeach",
        Some("New South Wales"),
        Some("Australia"),
        LAT,
        150.98,
    );
    let hill = Location::new(
        "Testhill",
        Some("New South Wales"),
        Some("Australia"),
        LAT,
        150.96,
    );
    let (product, squares) = layers().remove(6);
    let layer = bom::LAYERS.iter().find(|l| l.product == product).unwrap();
    let regions = bom::read_layer(&zip(product, &squares), layer).unwrap();
    assert_eq!(regions[0].name, "Testland Coast");
    let areas = bom::Areas::new(vec![bom::KeptLayer::new(layer, "1 Jan 1", regions)]);
    // Either of the zone's codes names it.
    let codes = areas.codes_at(beach.latitude, beach.longitude);
    assert_eq!(
        codes,
        HashSet::from(["NSW_TW901".to_string(), "NSW_TSW901".to_string()])
    );
    assert!(areas.codes_at(hill.latitude, hill.longitude).is_empty());
    // The national warning centre's files, not the state office's.
    assert!(bom::concerns("IDY68028.cap.xml", &codes));
    assert!(!bom::concerns("IDN99001.cap.xml", &codes));

    let fake = Fake::bureau();
    let service = ForecastService::new();
    let report = service.alerts(&fake, &beach, at(NOW));
    assert_eq!(report.problem, None);
    let tsunami = report
        .alerts
        .iter()
        .find(|a| a.event == "Tsunami warning")
        .expect("the beach hears the tsunami warning");
    assert_eq!(tsunami.id, "au:IDY68028/NSW_TSW901");
    // The Bureau rates it Unknown; the kind ranks it.
    assert_eq!(tsunami.severity, AlertSeverity::Severe);
    assert_eq!(tsunami.area, "New South Wales: Testland Coast");

    let report = service.alerts(&fake, &hill, at(NOW));
    assert_eq!(
        events(&report.alerts),
        ["Severe thunderstorm warning", "Minor flood warning"]
    );
}

#[test]
fn a_citys_warning_and_its_districts_name_their_areas_when_they_would_read_the_same() {
    let codes: HashSet<String> = ["NSW_PW901".to_string(), "NSW_ME901".to_string()].into();
    let district = cap::parse_xml(THUNDERSTORM).unwrap();
    let city = cap::parse_xml(
        &THUNDERSTORM
            .replace("IDN99001", "IDN99006")
            .replace(
                "Severe Thunderstorm Warning for Testland and South Testland forecast districts",
                "Severe Thunderstorm Warning for Greater Testville and Surrounds",
            )
            .replace("NSW_PW901", "NSW_ME901"),
    )
    .unwrap();
    let both = bom::alerts_for([&district, &city], &codes, at(NOW));
    assert_eq!(
        events(&both),
        [
            "Severe thunderstorm warning for Testland and South Testland forecast districts",
            "Severe thunderstorm warning for Greater Testville and Surrounds"
        ]
    );
    assert!(both.iter().all(|a| a.severity == AlertSeverity::Severe));
    // The line already says what the headline would.
    assert_eq!(
        both[1].description,
        "Storms with large hail are likely in the next few hours, with 30 to 50 millimetres of rain in an hour."
    );

    // Alone, a warning keeps the short name.
    let one = bom::alerts_for([&city], &codes, at(NOW));
    assert_eq!(events(&one), ["Severe thunderstorm warning"]);
}

#[test]
fn the_kind_ranks_a_warning_the_bureau_does_not() {
    let codes: HashSet<String> = ["NSW_PW901".to_string()].into();
    let warning =
        |headline: &str, severity: &str| {
            let xml = THUNDERSTORM
            .replace(
                "Severe Thunderstorm Warning for Testland and South Testland forecast districts",
                headline,
            )
            .replace("<severity>Unknown</severity>", &format!("<severity>{severity}</severity>"));
            let alert = cap::parse_xml(&xml).unwrap();
            let found = bom::alerts_for([&alert], &codes, at(NOW));
            (found[0].event.clone(), found[0].severity)
        };
    use AlertSeverity::*;
    assert_eq!(
        warning("Fire Weather Warning for Testland", "Unknown"),
        ("Fire weather warning".to_string(), Severe)
    );
    assert_eq!(
        warning("Extreme Heatwave Warning for Testland", "Unknown"),
        ("Extreme heatwave warning".to_string(), Extreme)
    );
    assert_eq!(
        warning("Flood Watch for Testland", "Unknown"),
        ("Flood watch".to_string(), Moderate)
    );
    assert_eq!(
        warning("Road Weather Alert for Testland", "Unknown"),
        ("Road weather alert".to_string(), Minor)
    );
    assert_eq!(
        warning("Major Flood Warning for the Test River", "Severe"),
        ("Major flood warning".to_string(), Severe)
    );
}

// --- What is kept

#[test]
fn outlines_are_kept_in_the_cache_folder_and_downloaded_again_only_when_changed() {
    let folder = TempDir::new();
    let fake = Fake::bureau();
    ForecastService::with_cache_folder(folder.0.clone()).alerts(&fake, &testville(), at(NOW));
    assert_eq!(layer_urls(&fake.asked()).len(), 7);
    assert!(folder.join("bom-areas/IDM00001.txt").exists());

    // The next session reads them, after a look at the listing.
    let report =
        ForecastService::with_cache_folder(folder.0.clone()).alerts(&fake, &testville(), at(NOW));
    assert_eq!(report.alerts.len(), 2);
    let asked = fake.asked();
    assert!(asked.iter().any(|u| u == bom::LAYERS_URL));
    assert!(layer_urls(&asked).is_empty());

    // One the Bureau has changed is fetched again, and only that one.
    let (product, squares) = layers().remove(3);
    let mut listing: Vec<String> = layers()
        .iter()
        .map(|(p, s)| layer_line(p, zip(p, s).len()))
        .collect();
    listing[3] = layer_line(product, 1);
    fake.set(bom::LAYERS_URL, listing.join("\n").into_bytes());
    fake.set(
        &format!("ftp://ftp.bom.gov.au/anon/home/adfd/spatial/{product}.zip"),
        zip(product, &squares),
    );
    ForecastService::with_cache_folder(folder.0.clone()).alerts(&fake, &testville(), at(NOW));
    assert_eq!(
        layer_urls(&fake.asked()),
        ["ftp://ftp.bom.gov.au/anon/home/adfd/spatial/IDM00017.zip"]
    );

    // Without the listing, what is kept serves.
    fake.answers.lock().unwrap().remove(bom::LAYERS_URL);
    let report =
        ForecastService::with_cache_folder(folder.0.clone()).alerts(&fake, &testville(), at(NOW));
    assert_eq!(report.problem, None);
    assert_eq!(report.alerts.len(), 2);
}

#[test]
fn a_warning_file_is_fetched_again_only_when_its_listing_changes() {
    let fake = Fake::bureau();
    let service = ForecastService::new();
    service.alerts(&fake, &testville(), at(NOW));
    fake.asked();

    // Within a minute the listing itself is reused.
    service.alerts(&fake, &testville(), at("2026-10-26T03:00:30Z"));
    assert!(fake.asked().is_empty());

    fake.set_warnings("02:45");
    service.alerts(&fake, &testville(), at("2026-10-26T03:05:00Z"));
    assert_eq!(
        fake.asked(),
        [
            bom::WARNINGS_URL.to_string(),
            bom::warning_url("IDN99001.cap.xml")
        ]
    );
}

#[test]
fn a_warning_keeps_its_id_through_updates_and_is_heard_again_only_when_raised() {
    let codes: HashSet<String> = ["QLD_RC900".to_string()].into();
    let issue = |severity: &str, kind: &str| -> Vec<WeatherAlert> {
        bom::alerts_for(
            [&cap::parse_xml(&flood(severity, kind)).unwrap()],
            &codes,
            at(NOW),
        )
    };
    let (minor, moderate) = (issue("Minor", "Minor"), issue("Moderate", "Moderate"));
    assert_eq!(minor[0].id, moderate[0].id);

    let mut seen = Vec::new();
    assert_eq!(
        events(&alerts::track(&mut seen, &minor)),
        ["Minor flood warning"]
    );
    assert!(alerts::track(&mut seen, &minor).is_empty());
    assert_eq!(
        events(&alerts::track(&mut seen, &moderate)),
        ["Moderate flood warning"]
    );
    assert!(alerts::track(&mut seen, &minor).is_empty());
}
