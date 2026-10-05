// OfficialSourcesTests.cs, ported (the page-then-API fallback is tested
// with the fetching, in fetch.rs), and WordingTests.cs's station names.

mod common;

use common::fixture;
use jiff::Timestamp;
use jiff::civil::date;
use weatherspell_core::environment_canada as ec;
use weatherspell_core::forecast::OfficialPeriod;
use weatherspell_core::location::Location;
use weatherspell_core::nws;
use weatherspell_core::official::{self, OfficialError};
use weatherspell_core::units::UnitSystem;

fn period(name: &str, d: jiff::civil::Date, text: &str) -> OfficialPeriod {
    OfficialPeriod {
        name: name.to_string(),
        date: d,
        text: text.to_string(),
    }
}

fn assert_period(actual: &OfficialPeriod, expected: OfficialPeriod) {
    assert_eq!(
        (&actual.name, actual.date, &actual.text),
        (&expected.name, expected.date, &expected.text)
    );
}

#[test]
fn points_give_the_grid_forecast_the_station_list_and_a_named_area() {
    let p = nws::parse_points(&fixture("nws-points-albany.json")).unwrap();
    assert_eq!(
        p.forecast_url,
        "https://api.weather.gov/gridpoints/ALY/72,63/forecast"
    );
    assert_eq!(
        p.stations_url,
        "https://api.weather.gov/gridpoints/ALY/72,63/stations"
    );
    assert_eq!(
        p.attribution,
        "National Weather Service (weather.gov), forecast for Albany, NY"
    );
}

#[test]
fn the_nearest_station_is_the_first_listed() {
    let station = nws::parse_stations(&fixture("nws-stations-albany.json")).unwrap();
    assert_eq!(
        station,
        Some((
            "KALB".to_string(),
            Some("Albany International Airport".to_string())
        ))
    );
}

#[test]
fn periods_keep_the_official_words_dated_by_their_local_start() {
    let periods = nws::parse_periods(&fixture("nws-forecast-albany.json")).unwrap();
    assert_eq!(periods.len(), 14);
    assert_period(
        &periods[0],
        period(
            "Overnight",
            date(2026, 9, 12),
            "Patchy fog after 3am. Mostly clear, with a low around 48. Wind around 0 miles per hour.",
        ),
    );
    assert_eq!(periods[1].name, "Saturday");
    // "Saturday Night" as the NWS cases it becomes "Saturday night"; the
    // night is dated by the day it follows; symbols become words.
    assert_eq!(periods[2].name, "Saturday night");
    assert_eq!(periods[2].date, date(2026, 9, 12));
    assert!(
        periods[2]
            .text
            .contains("gusts as high as 21 miles per hour. Chance of precipitation is 90 percent.")
    );
    assert_eq!(periods[13].date, date(2026, 9, 18));
}

#[test]
fn the_grid_forecast_is_asked_for_si_text_for_a_metric_reader() {
    let url = "https://api.weather.gov/gridpoints/OTX/144,86/forecast";
    assert_eq!(
        nws::forecast_url(url, UnitSystem::Metric),
        format!("{url}?units=si")
    );
    assert_eq!(nws::forecast_url(url, UnitSystem::Imperial), url);
}

#[test]
fn the_page_text_is_the_weather_gov_forecast_page_word_for_word() {
    let periods = nws::parse_page_periods(&fixture("nws-page-buckley.json")).unwrap();

    // Where the API said "East wind around 0 mph" and "Northeast wind 0 to
    // 5 mph"; the page's double spaces close up.
    assert_eq!(periods.len(), 14);
    assert_period(
        &periods[0],
        period(
            "Tonight",
            date(2026, 9, 26),
            "Patchy fog after 2am. Otherwise, mostly clear, with a low around 42. Calm wind.",
        ),
    );
    assert_eq!(
        periods[1].text,
        "Patchy fog before 10am. Otherwise, sunny, with a high near 72. Calm wind becoming north around 5 miles per hour in the afternoon."
    );
    assert_eq!(periods[2].name, "Sunday night");
    assert_eq!(periods[2].date, date(2026, 9, 27));
    assert_eq!(
        periods[6].text,
        "A 50 percent chance of showers after 2am. Mostly cloudy, with a low around 56."
    );
    assert_period(
        &periods[13],
        period(
            "Saturday",
            date(2026, 10, 3),
            "Mostly sunny, with a high near 66.",
        ),
    );
}

#[test]
fn the_page_is_asked_for_si_text_for_a_metric_reader() {
    let buckley = Location::new(
        "Buckley",
        Some("Michigan"),
        Some("United States"),
        44.5045,
        -85.677,
    );
    assert_eq!(
        nws::page_url(&buckley, UnitSystem::Imperial),
        "https://forecast.weather.gov/MapClick.php?lat=44.5045&lon=-85.677&FcstType=json"
    );
    assert_eq!(
        nws::page_url(&buckley, UnitSystem::Metric),
        "https://forecast.weather.gov/MapClick.php?lat=44.5045&lon=-85.677&FcstType=json&unit=1"
    );

    let periods = nws::parse_page_periods(&fixture("nws-page-buckley-si.json")).unwrap();
    assert_eq!(
        periods[1].text,
        "Patchy fog before 10am. Otherwise, sunny, with a high near 22. Calm wind becoming north 5 to 10 kilometres per hour in the afternoon."
    );
}

#[test]
fn unreadable_page_data_is_an_error_the_api_can_stand_in_for() {
    for page in [
        "<html><title>Forecast Error</title></html>",
        "{\"time\":{\"startPeriodName\":[\"Tonight\"],\"startValidTime\":[]},\"data\":{\"text\":[\"Clear.\"]}}",
        "{\"operationalMode\":\"Production\"}",
    ] {
        assert!(
            matches!(
                nws::parse_page_periods(page),
                Err(OfficialError::Unreadable(_))
            ),
            "{page}"
        );
    }
}

#[test]
fn an_observation_reads_si_values_and_leaves_unreported_ones_none() {
    let o = nws::parse_observation(
        &fixture("nws-observation-kalb.json"),
        Some("Albany International Airport"),
    )
    .unwrap();

    assert_eq!(o.station, "Albany International Airport");
    assert_eq!(o.time, "2026-09-12T07:05:00Z".parse::<Timestamp>().unwrap());
    assert_eq!(o.description.as_deref(), Some("Patchy Fog"));
    assert_eq!(o.temperature_c, Some(11.0));
    assert_eq!(o.feels_like_c, None);
    assert_eq!(o.humidity, None);
    assert_eq!(o.wind_kmh, Some(0.0));
    assert_eq!(o.wind_direction, Some(0));
    assert_eq!(o.wind_gust_kmh, None);
    assert_eq!(o.pressure_hpa, None);
    assert_eq!(o.dew_point_c, None);
    assert!((o.visibility_metres.unwrap() - 16093.44).abs() < 0.01);
}

#[test]
fn the_site_list_parses_and_finds_the_nearest_site() {
    let sites = ec::parse_site_list(&fixture("ec-site-list.csv")).unwrap();

    assert!((800..=1000).contains(&sites.len()));
    let (site, distance) = ec::nearest(&sites, 44.3104, -78.2396).unwrap();
    assert_eq!(
        *site,
        ec::Site {
            code: "s0000629".to_string(),
            name: "Peterborough".to_string(),
            province: "ON".to_string(),
            latitude: 44.30,
            longitude: -78.33,
        }
    );
    assert!((5.0..=10.0).contains(&distance));
}

#[test]
fn the_newest_file_for_a_site_is_picked_from_an_hour_listing() {
    let listing = fixture("ec-hour-listing-on-03.html");
    assert_eq!(
        ec::latest_file(&listing, "s0000629").as_deref(),
        Some("20260912T035744.540Z_MSC_CitypageWeather_s0000629_en.xml")
    );
    assert_eq!(ec::latest_file(&listing, "s9999999"), None);
}

#[test]
fn periods_are_dated_from_the_issue_day_by_their_names() {
    let official = ec::parse(&fixture("ec-citypage-peterborough.xml")).unwrap();

    assert_eq!(official.source_name, "Environment Canada");
    assert_eq!(
        official.attribution,
        "Environment and Climate Change Canada (weather.gc.ca), forecast for Peterborough City - Lakefield - Southern Peterborough County"
    );
    // Issued Friday the 11th at 3:30 pm: Tonight is the 11th, the named days
    // follow, nights belong to their day.
    assert_eq!(official.periods.len(), 12);
    assert_period(
        &official.periods[0],
        period(
            "Tonight",
            date(2026, 9, 11),
            "Clear. Fog patches developing near midnight. Low 7.",
        ),
    );
    assert_period(
        &official.periods[1],
        period(
            "Saturday",
            date(2026, 9, 12),
            "Sunny. Fog patches dissipating early in the morning. Wind becoming south 20 kilometres per hour in the afternoon. High 26. Humidex 31. UV index 6 or high.",
        ),
    );
    assert_eq!(
        (official.periods[2].name.as_str(), official.periods[2].date),
        ("Saturday night", date(2026, 9, 12))
    );
    assert_eq!(
        (
            official.periods[11].name.as_str(),
            official.periods[11].date
        ),
        ("Thursday", date(2026, 9, 17))
    );
}

#[test]
fn current_conditions_come_from_the_named_station_in_si() {
    let o = ec::parse(&fixture("ec-citypage-peterborough.xml"))
        .unwrap()
        .observation
        .unwrap();

    assert_eq!(o.station, "Peterborough Municipal Airport");
    assert_eq!(o.time, "2026-09-12T03:57:00Z".parse::<Timestamp>().unwrap());
    assert_eq!(o.description.as_deref(), Some("Mist"));
    assert_eq!(o.temperature_c, Some(9.2));
    assert_eq!(o.feels_like_c, None);
    assert_eq!(o.humidity, Some(100));
    assert_eq!(o.wind_kmh, Some(4.0));
    assert_eq!(o.wind_direction, Some(260));
    assert_eq!(o.wind_gust_kmh, None);
    assert!((o.pressure_hpa.unwrap() - 1018.0).abs() < 1e-3);
    assert_eq!(o.dew_point_c, Some(9.2));
    assert!((o.visibility_metres.unwrap() - 6400.0).abs() < 1e-3);
}

fn without(xml: &str, element: &str, replacement: &str) -> String {
    let start = xml.find(&format!("<{element}>")).unwrap();
    let close = format!("</{element}>");
    let end = xml.find(&close).unwrap() + close.len();
    format!("{}{replacement}{}", &xml[..start], &xml[end..])
}

#[test]
fn a_page_without_current_conditions_has_no_observation() {
    let xml = fixture("ec-citypage-peterborough.xml");
    let stripped = without(&xml, "currentConditions", "<currentConditions/>");
    assert_eq!(ec::parse(&stripped).unwrap().observation, None);
}

#[test]
fn an_observation_only_site_is_told_apart_from_a_failure() {
    // Alert and Eureka publish an empty forecast group: a fact about the
    // site, not something another try would fix.
    let xml = fixture("ec-citypage-peterborough.xml");
    for page in [
        without(&xml, "forecastGroup", "<forecastGroup/>"),
        without(&xml, "forecastGroup", ""),
    ] {
        assert!(matches!(
            ec::parse(&page),
            Err(OfficialError::NoForecastText(_))
        ));
    }
}

#[test]
fn symbols_become_the_words_the_service_says_aloud() {
    for (written, spoken) in [
        (
            "Chance of precipitation is 90%.",
            "Chance of precipitation is 90 percent.",
        ),
        (
            "Wind becoming south 20 km/h in the afternoon.",
            "Wind becoming south 20 kilometres per hour in the afternoon.",
        ),
        (
            "South wind 7 to 10 mph, with gusts as high as 21 mph.",
            "South wind 7 to 10 miles per hour, with gusts as high as 21 miles per hour.",
        ),
        ("  Sunny. High 26.  ", "Sunny. High 26."),
    ] {
        assert_eq!(official::spoken(written), spoken);
    }
}

#[test]
fn station_names_read_as_words() {
    for (name, expected) in [
        ("Gander Int'l Airport", "Gander International Airport"),
        (
            "VISITORS CENTER AT FURNACE CREEK DEATH VALLEY",
            "Visitors Center at Furnace Creek Death Valley",
        ),
        (
            "Spokane, Spokane International Airport",
            "Spokane, Spokane International Airport",
        ),
        ("Burlington Lift Bridge", "Burlington Lift Bridge"),
    ] {
        assert_eq!(official::station_name(name), expected);
    }
}

#[test]
fn the_failure_lines_name_the_service_and_the_reason() {
    assert_eq!(
        official::fetch_problem("Environment Canada", "404 Not Found from dd.weather.gc.ca."),
        "Environment Canada's forecast text could not be fetched this time (404 Not Found from dd.weather.gc.ca), so these sentences are written from Open-Meteo data."
    );
    assert_eq!(
        official::no_text_problem("National Weather Service"),
        "National Weather Service has no forecast text for this area, so these sentences are written from Open-Meteo data."
    );
}
