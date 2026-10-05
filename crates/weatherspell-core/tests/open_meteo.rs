// OpenMeteoParsingTests.cs, ported.

mod common;

use common::{fixture, offset, toronto};
use jiff::Timestamp;
use jiff::civil::date;
use weatherspell_core::open_meteo;
use weatherspell_core::units::UnitSystem;

#[test]
fn parses_a_real_forecast_response() {
    let fetched: Timestamp = "2026-09-11T20:31:00Z".parse().unwrap();
    let f = open_meteo::parse(
        &fixture("open-meteo-toronto.json"),
        &toronto(),
        UnitSystem::Metric,
        fetched,
    )
    .unwrap();

    assert_eq!(f.utc_offset, offset(-4));
    assert_eq!(f.current.local_time, date(2026, 9, 11).at(16, 30, 0, 0));
    assert!((f.current.temperature - 21.1).abs() < 1e-3);
    assert_eq!(f.current.weather_code, 0);
    assert!(f.current.is_day);
    assert!((0..=100).contains(&f.current.humidity));
    assert!((900.0..=1100.0).contains(&f.current.pressure_hpa));

    assert_eq!(f.hours.len(), 168);
    assert_eq!(f.hours[0].local_time, date(2026, 9, 11).at(0, 0, 0, 0));
    assert!(f.hours[12].dew_point.is_some());
    assert!(f.hours[12].visibility_metres.is_some());

    assert_eq!(f.days.len(), 7);
    assert_eq!(f.days[0].date, date(2026, 9, 11));
    assert_eq!(f.days[0].sunrise, Some(date(2026, 9, 11).at(6, 52, 0, 0)));
    assert!(f.days[0].sunset.is_some());
    assert!(f.days[0].daylight_seconds.is_some());
    assert!((f.days[0].uv_index_max.unwrap() - 5.75).abs() < 1e-3);
    assert_eq!(f.fetched_at, fetched);
    assert_eq!(f.source_name, open_meteo::SOURCE_NAME);
}

#[test]
fn skips_hours_the_model_has_not_filled() {
    // A trailing null temperature marks the end of the model's horizon.
    let json = r#"
        {"utc_offset_seconds":0,
         "current":{"time":"2026-01-01T12:00","temperature_2m":1.0,"weather_code":3},
         "hourly":{"time":["2026-01-01T12:00","2026-01-01T13:00"],"temperature_2m":[1.0,null],"weather_code":[3,null]},
         "daily":{"time":["2026-01-01"],"temperature_2m_max":[2.0],"temperature_2m_min":[0.0]}}
        "#;
    let f = open_meteo::parse(json, &toronto(), UnitSystem::Metric, Timestamp::now()).unwrap();

    assert_eq!(f.hours.len(), 1);
    assert_eq!(f.days.len(), 1);
    assert_eq!(f.days[0].sunrise, None);
    assert_eq!(f.current.feels_like, 1.0); // falls back to the temperature
}

#[test]
fn builds_the_forecast_url_with_units_and_local_time() {
    let url = open_meteo::forecast_url(&toronto(), UnitSystem::Imperial, 7);

    assert!(
        url.starts_with("https://api.open-meteo.com/v1/forecast?latitude=43.65&longitude=-79.38&")
    );
    assert!(url.contains("&timezone=auto"));
    assert!(url.contains("&temperature_unit=fahrenheit"));
    assert!(url.contains("&wind_speed_unit=mph"));
    assert!(url.contains("&precipitation_unit=inch"));
    assert!(url.contains("&forecast_days=7"));
}

#[test]
fn parses_geocoding_results_with_region_country_and_population() {
    let results = open_meteo::parse_search(&fixture("geocoding-peterborough.json")).unwrap();

    assert!(results.len() >= 3);
    // The geocoder knows a city and a township of that name in Ontario.
    let ontario = results
        .iter()
        .find(|r| r.name == "Peterborough" && r.region.as_deref() == Some("Ontario"))
        .unwrap();
    assert_eq!(ontario.country.as_deref(), Some("Canada"));
    assert_eq!(ontario.time_zone_id.as_deref(), Some("America/Toronto"));
    assert_eq!(ontario.full_name(), "Peterborough, Ontario, Canada");
    assert!(
        ontario
            .search_result_text()
            .starts_with("Peterborough, Ontario, Canada (population ")
    );
}

#[test]
fn a_territory_the_geocoder_gives_no_country_is_named_and_covered_by_the_nws() {
    // As the geocoder answered on 2026-09-25: a country code and no country.
    let json = r#"
        {"results":[
          {"id":4568127,"name":"San Juan","latitude":18.46633,"longitude":-66.10572,"country_code":"PR","timezone":"America/Puerto_Rico","population":418140,"admin1":"San Juan"},
          {"id":5881576,"name":"Pago Pago","latitude":-14.27806,"longitude":-170.7025,"country_code":"AS","timezone":"Pacific/Pago_Pago","admin1":"Eastern District"},
          {"id":2729907,"name":"Longyearbyen","latitude":78.22334,"longitude":15.64689,"country_code":"SJ","timezone":"Arctic/Longyearbyen"}
        ]}
        "#;
    let results = open_meteo::parse_search(json).unwrap();

    assert_eq!(results[0].full_name(), "San Juan, Puerto Rico");
    assert!(results[0].is_nws_covered());
    assert_eq!(
        results[1].full_name(),
        "Pago Pago, Eastern District, American Samoa"
    );
    assert!(results[1].is_nws_covered());
    // Elsewhere a missing country stays missing.
    assert_eq!(results[2].country, None);
    assert!(!results[2].is_nws_covered());
}

#[test]
fn empty_geocoding_response_yields_no_results() {
    assert!(
        open_meteo::parse_search("{\"generationtime_ms\":0.5}")
            .unwrap()
            .is_empty()
    );
}
