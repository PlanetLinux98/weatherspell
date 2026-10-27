// The Air quality section: which index a place reads, the words for each
// band, the line about the hours ahead, pollen by MeteoSwiss's classes,
// and Environment Canada's AQHI and Open-Meteo's responses as captured.

mod common;

use common::fixture;
use jiff::Timestamp;
use jiff::civil::{DateTime, date};
use weatherspell_core::air::{
    self, AirHour, AirQuality, Community, Fetched, OpenMeteoAir, PollenHour, Pollutant, Scale,
};
use weatherspell_core::location::Location;

fn place(country: &str) -> Location {
    Location::new("Somewhere", None, Some(country), 0.0, 0.0)
}

#[test]
fn a_place_reads_the_scale_of_its_region() {
    assert_eq!(air::open_meteo_scale(&place("Germany")), Scale::European);
    assert_eq!(
        air::open_meteo_scale(&place("United Kingdom")),
        Scale::European
    );
    assert_eq!(air::open_meteo_scale(&place("Ireland")), Scale::European);
    assert_eq!(air::open_meteo_scale(&place("Andorra")), Scale::European);
    assert_eq!(air::open_meteo_scale(&place("Moldova")), Scale::European);
    assert_eq!(air::open_meteo_scale(&place("United States")), Scale::Us);
    assert_eq!(air::open_meteo_scale(&place("Canada")), Scale::Us);
    assert_eq!(air::open_meteo_scale(&place("Japan")), Scale::Us);
    assert_eq!(
        air::open_meteo_scale(&Location::at_point(44.5, -78.5)),
        Scale::Us
    );
}

#[test]
fn each_scale_has_its_own_bands() {
    let us = |n: f64| Scale::Us.band(n).1;
    assert_eq!(us(0.0), "good");
    assert_eq!(us(50.0), "good");
    assert_eq!(us(50.4), "good");
    assert_eq!(us(51.0), "moderate");
    assert_eq!(us(101.0), "unhealthy for sensitive groups");
    assert_eq!(us(151.0), "unhealthy");
    assert_eq!(us(201.0), "very unhealthy");
    assert_eq!(us(301.0), "hazardous");
    let eu = |n: f64| Scale::European.band(n).1;
    assert_eq!(eu(20.0), "good");
    assert_eq!(eu(21.0), "fair");
    assert_eq!(eu(41.0), "moderate");
    assert_eq!(eu(61.0), "poor");
    assert_eq!(eu(81.0), "very poor");
    assert_eq!(eu(101.0), "extremely poor");
    let aqhi = |n: f64| Scale::Aqhi.band(n).1;
    assert_eq!(aqhi(0.2), "low risk");
    assert_eq!(aqhi(3.4), "low risk");
    assert_eq!(aqhi(3.5), "moderate risk");
    assert_eq!(aqhi(7.0), "high risk");
    assert_eq!(aqhi(10.4), "high risk");
    assert_eq!(aqhi(11.0), "very high risk");
}

fn now_local() -> DateTime {
    date(2026, 10, 27).at(9, 20, 0, 0)
}

fn hour(h: i8, index: f64, main: Option<Pollutant>) -> AirHour {
    let d = if h < 24 {
        date(2026, 10, 27).at(h, 0, 0, 0)
    } else {
        date(2026, 10, 28).at(h - 24, 0, 0, 0)
    };
    AirHour {
        local_time: d,
        index,
        main,
    }
}

fn quality(scale: Scale, hours: Vec<AirHour>) -> AirQuality {
    AirQuality {
        scale,
        community: None,
        observed: None,
        hours,
        pollen: Vec::new(),
        sources: Vec::new(),
        problem: None,
    }
}

fn lines(air: &AirQuality) -> Vec<String> {
    air::section(air, now_local())
        .map(|s| {
            assert_eq!(s.heading, "Air quality");
            s.paragraphs
        })
        .unwrap_or_default()
}

#[test]
fn the_hour_in_progress_is_worded_with_its_main_pollutant_when_not_good() {
    let good = quality(
        Scale::Us,
        vec![hour(9, 32.0, Some(Pollutant::Ozone)), hour(10, 34.0, None)],
    );
    assert_eq!(
        lines(&good),
        ["Air quality is good, 32 on the US Air Quality Index."]
    );
    let moderate = quality(
        Scale::European,
        vec![hour(9, 54.4, Some(Pollutant::NitrogenDioxide))],
    );
    assert_eq!(
        lines(&moderate),
        [
            "Air quality is moderate, 54 on the European Air Quality Index, mostly from nitrogen dioxide."
        ]
    );
}

#[test]
fn the_hours_ahead_are_mentioned_when_they_change_band() {
    let rising = quality(
        Scale::Us,
        vec![
            hour(9, 81.0, Some(Pollutant::FineParticles)),
            hour(14, 104.0, Some(Pollutant::FineParticles)),
            hour(18, 109.0, Some(Pollutant::FineParticles)),
            // Beyond twelve hours: not counted.
            hour(23, 160.0, Some(Pollutant::FineParticles)),
        ],
    );
    assert_eq!(
        lines(&rising),
        [
            "Air quality is moderate, 81 on the US Air Quality Index, mostly from fine particles.",
            "Forecast to reach unhealthy for sensitive groups this evening, up to 109."
        ]
    );
    let easing = quality(
        Scale::European,
        vec![
            hour(9, 66.0, Some(Pollutant::Ozone)),
            hour(13, 50.0, None),
            hour(21, 35.0, None),
            // Beyond twelve hours.
            hour(26, 10.0, None),
        ],
    );
    assert_eq!(lines(&easing)[1], "Forecast to ease to fair tonight.");
    let steady = quality(Scale::Us, vec![hour(9, 20.0, None), hour(15, 45.0, None)]);
    assert_eq!(lines(&steady).len(), 1);
}

#[test]
fn the_aqhi_names_its_community_and_tops_out_above_10() {
    let mut air = quality(Scale::Aqhi, vec![hour(9, 2.0, None), hour(12, 11.0, None)]);
    air.community = Some("Peterborough".to_string());
    assert_eq!(
        lines(&air),
        [
            "Air quality is low risk, 2 on the Air Quality Health Index for Peterborough.",
            "Forecast to reach very high risk this afternoon, up to above 10."
        ]
    );
    // The latest reading stands for now while it is fresh.
    air.observed = Some(AirHour {
        local_time: date(2026, 10, 27).at(8, 0, 0, 0),
        index: 4.4,
        main: None,
    });
    assert_eq!(
        lines(&air)[0],
        "Air quality is moderate risk, 4 on the Air Quality Health Index for Peterborough."
    );
    air.observed.as_mut().unwrap().local_time = date(2026, 10, 27).at(5, 0, 0, 0);
    assert!(lines(&air)[0].contains(", 2 on the"));
}

#[test]
fn a_forecast_that_starts_soon_stands_for_now_and_one_that_has_not_started_says_so() {
    // Environment Canada's forecast can start an hour or two ahead.
    let soon = quality(Scale::Aqhi, vec![hour(11, 3.0, None)]);
    assert_eq!(
        lines(&soon),
        ["Air quality is low risk, 3 on the Air Quality Health Index."]
    );
    let later = quality(Scale::Aqhi, vec![hour(15, 3.0, None)]);
    assert_eq!(
        lines(&later),
        ["There is no Air Quality Health Index for this hour yet."]
    );
    // An old forecast's hours, all past: nothing about now, and no
    // section at all without pollen.
    let past = quality(Scale::Us, vec![hour(3, 40.0, None), hour(6, 45.0, None)]);
    assert!(air::section(&past, now_local()).is_none());
}

#[test]
fn a_failed_check_is_said_and_nothing_at_all_says_nothing() {
    let mut failed = quality(Scale::Us, Vec::new());
    failed.problem = Some("air-quality-api.open-meteo.com could not be reached".to_string());
    assert_eq!(
        lines(&failed),
        [
            "Air quality couldn't be checked this time (air-quality-api.open-meteo.com could not be reached). Press F5 to try again."
        ]
    );
    assert!(air::section(&quality(Scale::Us, Vec::new()), now_local()).is_none());
}

fn pollen(values: [Option<f64>; 6]) -> Vec<PollenHour> {
    (0..24)
        .map(|h| PollenHour {
            local_time: date(2026, 10, 27).at(h, 0, 0, 0),
            alder: values[0],
            birch: values[1],
            grass: values[2],
            mugwort: values[3],
            olive: values[4],
            ragweed: values[5],
        })
        .collect()
}

fn pollen_line(values: [Option<f64>; 6]) -> String {
    let mut air = quality(Scale::European, vec![hour(9, 15.0, None)]);
    air.pollen = pollen(values);
    lines(&air).pop().unwrap()
}

#[test]
fn pollen_is_classed_by_the_days_mean_highest_first() {
    assert_eq!(
        pollen_line([
            Some(0.0),
            Some(0.0),
            Some(19.9),
            Some(6.0),
            Some(0.0),
            Some(0.0)
        ]),
        "Pollen today: mugwort moderate, grass low."
    );
    assert_eq!(
        pollen_line([
            Some(0.0),
            Some(300.0),
            Some(20.0),
            None,
            Some(201.0),
            Some(40.0)
        ]),
        "Pollen today: birch very high, ragweed very high, olive high, grass moderate."
    );
    assert_eq!(
        pollen_line([
            Some(0.0),
            Some(0.9),
            Some(0.0),
            Some(0.0),
            Some(0.0),
            Some(0.0)
        ]),
        "Pollen today: none."
    );
    // The mean, not the peak: a morning burst over a quiet day.
    let mut air = quality(Scale::European, vec![hour(9, 15.0, None)]);
    air.pollen = pollen([Some(0.0); 6]);
    air.pollen[8].grass = Some(200.0);
    assert_eq!(lines(&air)[1], "Pollen today: grass low.");
    // Where the model has none, no line.
    let mut air = quality(Scale::Us, vec![hour(9, 15.0, None)]);
    air.pollen = Vec::new();
    assert_eq!(lines(&air).len(), 1);
}

#[test]
fn open_meteos_european_response_reads_with_its_pollen() {
    let air =
        air::parse_open_meteo(&fixture("open-meteo-air-madrid.json"), Scale::European).unwrap();
    assert_eq!(air.hours.len(), 48);
    assert_eq!(air.hours[0].local_time, date(2026, 10, 27).at(0, 0, 0, 0));
    for h in &air.hours {
        assert!(h.index >= 0.0);
        assert!(h.main.is_some());
    }
    assert_eq!(air.pollen.len(), 48);
    assert!(air.pollen.iter().all(|p| p.grass.is_some()));
}

#[test]
fn the_main_pollutant_is_the_one_whose_own_index_is_highest() {
    let json = r#"{"hourly":{"time":["2026-10-27T00:00","2026-10-27T01:00"],
        "us_aqi":[80,null],"us_aqi_pm2_5":[60,1],"us_aqi_pm10":[20,1],
        "us_aqi_ozone":[80,1],"us_aqi_nitrogen_dioxide":[10,1],
        "us_aqi_sulphur_dioxide":[null,1],"us_aqi_carbon_monoxide":[2,1],
        "grass_pollen":[null,null]}}"#;
    let air = air::parse_open_meteo(json, Scale::Us).unwrap();
    // An hour without the index is left out.
    assert_eq!(air.hours.len(), 1);
    assert_eq!(air.hours[0].main, Some(Pollutant::Ozone));
    assert!(air.pollen.is_empty());
    assert!(air::parse_open_meteo("{}", Scale::Us).is_err());
    assert!(air::parse_open_meteo("not json", Scale::Us).is_err());
}

#[test]
fn outside_europe_there_is_no_pollen() {
    let air = air::parse_open_meteo(&fixture("open-meteo-air-kathmandu.json"), Scale::Us).unwrap();
    assert!(!air.hours.is_empty());
    assert!(air.pollen.is_empty());
}

#[test]
fn environment_canadas_communities_reading_and_forecast_are_read() {
    let communities = air::parse_communities(&fixture("ec-aqhi-stations.json")).unwrap();
    assert!(communities.len() >= 4);
    let peterborough = Location::new("Peterborough", None, Some("Canada"), 44.3, -78.32);
    let (nearest, distance) = air::nearest_community(&communities, &peterborough).unwrap();
    assert_eq!(nearest.name, "Peterborough");
    assert!(distance < 5.0);
    // Lakefield, 15 km north, is still Peterborough's air; Bancroft, 90 km,
    // is not.
    let lakefield = Location::new("Lakefield", None, Some("Canada"), 44.42, -78.27);
    assert_eq!(
        air::nearest_community(&communities, &lakefield)
            .unwrap()
            .0
            .name,
        "Peterborough"
    );
    let bancroft = Location::new("Bancroft", None, Some("Canada"), 45.06, -77.85);
    assert!(air::nearest_community(&communities, &bancroft).is_none());

    let (at, value) = air::parse_observation(&fixture("ec-aqhi-observation-peterborough.json"))
        .unwrap()
        .unwrap();
    assert_eq!(at, "2026-10-27T07:00:00Z".parse::<Timestamp>().unwrap());
    assert!((value - 1.32).abs() < 1e-9);

    // The newest publication only, in order, one value an hour; its
    // summary by period ("Tonight", "Tomorrow"), which has no hour, is
    // left out.
    let forecasts = air::parse_forecasts(&fixture("ec-aqhi-forecasts-peterborough.json")).unwrap();
    assert_eq!(forecasts.len(), 37);
    assert_eq!(
        forecasts[0].0,
        "2026-10-26T21:00:00Z".parse::<Timestamp>().unwrap()
    );
    assert!(forecasts.windows(2).all(|w| w[0].0 < w[1].0));
    assert!(
        air::parse_forecasts(r#"{"features":[]}"#)
            .unwrap()
            .is_empty()
    );
    assert!(
        air::parse_observation(r#"{"features":[]}"#)
            .unwrap()
            .is_none()
    );
    assert!(air::parse_communities(r#"{"features":[]}"#).is_err());
}

fn community() -> Community {
    Community {
        id: "FDGEJ".to_string(),
        name: "Peterborough".to_string(),
        latitude: 44.3,
        longitude: -78.33,
    }
}

fn open_meteo() -> OpenMeteoAir {
    OpenMeteoAir {
        hours: vec![hour(9, 40.0, Some(Pollutant::Ozone))],
        pollen: Vec::new(),
    }
}

fn wall(t: Timestamp) -> DateTime {
    jiff::tz::TimeZone::get("America/Toronto")
        .unwrap()
        .to_datetime(t)
}

#[test]
fn environment_canadas_index_is_used_near_a_community() {
    let fetched = Fetched {
        community: Some(community()),
        observed: Some(("2026-10-27T12:00:00Z".parse().unwrap(), 2.6)),
        forecasts: vec![("2026-10-27T13:00:00Z".parse().unwrap(), 3.0)],
        aqhi_problem: None,
        open_meteo: Some(Ok(open_meteo())),
        open_meteo_scale: Some(Scale::Us),
    };
    let air = air::compose(fetched, wall);
    assert_eq!(air.scale, Scale::Aqhi);
    assert_eq!(air.community.as_deref(), Some("Peterborough"));
    // On the location's clock: 8 am Toronto.
    assert_eq!(
        air.observed.as_ref().unwrap().local_time,
        date(2026, 10, 27).at(8, 0, 0, 0)
    );
    assert_eq!(
        air.sources,
        [
            "Air quality: Environment and Climate Change Canada's Air Quality Health Index for Peterborough (weather.gc.ca)."
        ]
    );
}

#[test]
fn without_environment_canadas_index_the_us_index_stands_in_and_says_why() {
    let fetched = Fetched {
        community: Some(community()),
        aqhi_problem: Some("503 Service Unavailable from api.weather.gc.ca".to_string()),
        open_meteo: Some(Ok(open_meteo())),
        open_meteo_scale: Some(Scale::Us),
        ..Fetched::default()
    };
    let air = air::compose(fetched, wall);
    assert_eq!(air.scale, Scale::Us);
    assert_eq!(air.community, None);
    assert_eq!(
        air.sources,
        [
            "Environment Canada's Air Quality Health Index for Peterborough could not be fetched this time (503 Service Unavailable from api.weather.gc.ca), so this is the US Air Quality Index.",
            "Air quality: Open-Meteo (open-meteo.com), licensed CC BY 4.0, from the Copernicus Atmosphere Monitoring Service."
        ]
    );

    // A community with nothing published says so the same way.
    let fetched = Fetched {
        community: Some(community()),
        open_meteo: Some(Ok(open_meteo())),
        open_meteo_scale: Some(Scale::Us),
        ..Fetched::default()
    };
    assert!(air::compose(fetched, wall).sources[0].contains("(it had no reading or forecast)"));

    // Away from any community: Open-Meteo's, without comment.
    let fetched = Fetched {
        open_meteo: Some(Ok(open_meteo())),
        open_meteo_scale: Some(Scale::Us),
        ..Fetched::default()
    };
    let air = air::compose(fetched, wall);
    assert_eq!(air.sources.len(), 1);
}

#[test]
fn a_failed_request_leaves_the_problem_for_the_section() {
    let fetched = Fetched {
        open_meteo: Some(Err(
            "air-quality-api.open-meteo.com could not be reached".to_string()
        )),
        open_meteo_scale: Some(Scale::European),
        ..Fetched::default()
    };
    let air = air::compose(fetched, wall);
    assert!(air.sources.is_empty());
    assert_eq!(
        air.problem.as_deref(),
        Some("air-quality-api.open-meteo.com could not be reached")
    );
}

#[test]
fn pollen_is_credited_beside_environment_canadas_index() {
    let mut om = open_meteo();
    om.pollen = pollen([Some(5.0); 6]);
    let fetched = Fetched {
        community: Some(community()),
        forecasts: vec![("2026-10-27T13:00:00Z".parse().unwrap(), 3.0)],
        open_meteo: Some(Ok(om)),
        open_meteo_scale: Some(Scale::Us),
        ..Fetched::default()
    };
    let air = air::compose(fetched, wall);
    assert_eq!(air.sources.len(), 2);
    assert!(air.sources[1].starts_with("Pollen: Open-Meteo"));
}
