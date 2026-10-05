// The asking, answered from captured responses: which services are asked
// for which places, in what order, and what a reader is told when one of
// them fails. NwsClient's fallback tests and ForecastService's behaviour in
// 0.1, ported, with Environment Canada's walk back through the hours.

mod common;

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use common::fixture;
use jiff::Timestamp;
use weatherspell_core::environment_canada as ec;
use weatherspell_core::fetch::{
    Fetch, FetchError, ForecastService, PAGE_TIMEOUT, ServiceError, check_alerts,
};
use weatherspell_core::location::Location;
use weatherspell_core::units::UnitSystem;
use weatherspell_core::{nws, open_meteo};

// Answers each address it knows; anything else is a 404, as a server
// would say for a folder that is not there yet.
#[derive(Default)]
struct Fake {
    answers: HashMap<String, Result<String, FetchError>>,
    asked: Mutex<Vec<(String, Option<Duration>)>>,
}

impl Fake {
    fn answer(mut self, url: &str, body: Result<String, FetchError>) -> Fake {
        self.answers.insert(url.to_string(), body);
        self
    }

    fn with(self, url: &str, fixture_name: &str) -> Fake {
        self.answer(url, Ok(fixture(fixture_name)))
    }

    fn asked(&self) -> Vec<String> {
        self.asked
            .lock()
            .unwrap()
            .iter()
            .map(|(url, _)| url.clone())
            .collect()
    }

    fn times_asked(&self, url: &str) -> usize {
        self.asked().iter().filter(|u| *u == url).count()
    }
}

impl Fetch for Fake {
    fn get(&self, url: &str, timeout: Option<Duration>) -> Result<String, FetchError> {
        self.asked.lock().unwrap().push((url.to_string(), timeout));
        self.answers
            .get(url)
            .cloned()
            .unwrap_or_else(|| Err(status(404, "Not Found", url)))
    }
}

fn host(url: &str) -> String {
    url.split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap()
        .to_string()
}

fn status(code: u16, reason: &str, url: &str) -> FetchError {
    FetchError::Status {
        code,
        reason: reason.to_string(),
        host: host(url),
    }
}

fn albany() -> Location {
    Location::new(
        "Albany",
        Some("New York"),
        Some("United States"),
        42.65,
        -73.75,
    )
}

fn peterborough() -> Location {
    Location::new(
        "Peterborough",
        Some("Ontario"),
        Some("Canada"),
        44.3104,
        -78.2396,
    )
}

fn at(stamp: &str) -> Timestamp {
    stamp.parse().unwrap()
}

// Open-Meteo for the place, and the NWS's grid, station and observation.
fn nws_fake(location: &Location, units: UnitSystem) -> Fake {
    Fake::default()
        .with(
            &open_meteo::forecast_url(location, units, 7),
            "open-meteo-toronto.json",
        )
        .with(&nws::points_url(location), "nws-points-albany.json")
        .with(
            "https://api.weather.gov/gridpoints/ALY/72,63/stations",
            "nws-stations-albany.json",
        )
        .with(&nws::observation_url("KALB"), "nws-observation-kalb.json")
}

const API_FORECAST: &str = "https://api.weather.gov/gridpoints/ALY/72,63/forecast";

#[test]
fn the_page_text_is_used_and_the_api_is_not_asked() {
    let place = albany();
    let http = nws_fake(&place, UnitSystem::Imperial).with(
        &nws::page_url(&place, UnitSystem::Imperial),
        "nws-page-buckley.json",
    );
    let f = ForecastService::new()
        .forecast(
            &http,
            &place,
            UnitSystem::Imperial,
            7,
            at("2026-09-12T07:15:00Z"),
        )
        .unwrap();

    assert_eq!(f.periods[0].name, "Tonight");
    assert!(f.periods[0].text.ends_with("Calm wind."));
    assert_eq!(http.times_asked(API_FORECAST), 0);
    // The page gets the shorter wait.
    let page = http
        .asked
        .lock()
        .unwrap()
        .iter()
        .find(|(u, _)| u.contains("MapClick"))
        .cloned()
        .unwrap();
    assert_eq!(page.1, Some(PAGE_TIMEOUT));
    assert_eq!(
        f.sources[0],
        "Forecast text and current conditions: National Weather Service (weather.gov), forecast for Albany, NY, observed at Albany International Airport."
    );
    assert_eq!(
        f.current.station.as_deref(),
        Some("Albany International Airport")
    );
}

#[test]
fn the_api_text_stands_in_when_the_page_cannot_be_had_or_read() {
    let place = albany();
    let page_url = nws::page_url(&place, UnitSystem::Imperial);
    for page in [
        Err(status(503, "Service Unavailable", &page_url)),
        Ok("<html><title>Forecast Error</title></html>".to_string()),
        Ok("{\"time\":{\"startPeriodName\":[\"Tonight\"],\"startValidTime\":[]},\"data\":{\"text\":[\"Clear.\"]}}".to_string()),
        Ok("{\"operationalMode\":\"Production\"}".to_string()),
        Err(FetchError::TimedOut { host: host(&page_url) }),
    ] {
        let http = nws_fake(&place, UnitSystem::Imperial).answer(&page_url, page.clone()).with(API_FORECAST, "nws-forecast-albany.json");
        let f = ForecastService::new().forecast(&http, &place, UnitSystem::Imperial, 7, at("2026-09-12T07:15:00Z")).unwrap();

        assert_eq!(f.periods[0].name, "Overnight", "{page:?}");
        assert!(f.periods[0].text.ends_with("Wind around 0 miles per hour."));
    }
}

#[test]
fn a_metric_reader_asks_both_for_si_text() {
    let place = albany();
    let http = nws_fake(&place, UnitSystem::Metric).with(
        &format!("{API_FORECAST}?units=si"),
        "nws-forecast-albany.json",
    );
    ForecastService::new()
        .forecast(
            &http,
            &place,
            UnitSystem::Metric,
            7,
            at("2026-09-12T07:15:00Z"),
        )
        .unwrap();

    let asked = http.asked();
    assert!(
        asked
            .iter()
            .any(|u| u.contains("MapClick") && u.ends_with("&unit=1"))
    );
    assert!(asked.contains(&format!("{API_FORECAST}?units=si")));
}

#[test]
fn a_station_that_is_down_costs_only_the_observation() {
    let place = albany();
    let http = nws_fake(&place, UnitSystem::Imperial)
        .answer(
            &nws::observation_url("KALB"),
            Err(status(
                500,
                "Internal Server Error",
                "https://api.weather.gov/",
            )),
        )
        .with(
            &nws::page_url(&place, UnitSystem::Imperial),
            "nws-page-buckley.json",
        );
    let f = ForecastService::new()
        .forecast(
            &http,
            &place,
            UnitSystem::Imperial,
            7,
            at("2026-09-12T07:15:00Z"),
        )
        .unwrap();

    assert_eq!(f.current.station, None);
    assert_eq!(
        f.sources,
        [
            "Forecast text: National Weather Service (weather.gov), forecast for Albany, NY.",
            "Current conditions, hourly data, sun and UV: Open-Meteo (open-meteo.com), licensed CC BY 4.0."
        ]
    );
}

#[test]
fn the_grid_is_looked_up_once_per_session() {
    let place = albany();
    let http = nws_fake(&place, UnitSystem::Imperial).with(
        &nws::page_url(&place, UnitSystem::Imperial),
        "nws-page-buckley.json",
    );
    let service = ForecastService::new();
    for _ in 0..2 {
        service
            .forecast(
                &http,
                &place,
                UnitSystem::Imperial,
                7,
                at("2026-09-12T07:15:00Z"),
            )
            .unwrap();
    }
    assert_eq!(http.times_asked(&nws::points_url(&place)), 1);
    assert_eq!(
        http.times_asked("https://api.weather.gov/gridpoints/ALY/72,63/stations"),
        1
    );
    assert_eq!(http.times_asked(&nws::observation_url("KALB")), 2);
}

#[test]
fn a_point_with_no_forecast_grid_is_said_as_a_fact() {
    // American Samoa's points name no grid (#19).
    let place = Location::new(
        "Pago Pago",
        Some("Eastern District"),
        Some("American Samoa"),
        -14.2781,
        -170.7025,
    );
    let http = Fake::default()
        .with(
            &open_meteo::forecast_url(&place, UnitSystem::Imperial, 7),
            "open-meteo-toronto.json",
        )
        .answer(
            &nws::points_url(&place),
            Ok("{\"properties\":{\"forecast\":null,\"observationStations\":null}}".to_string()),
        );
    let f = ForecastService::new()
        .forecast(
            &http,
            &place,
            UnitSystem::Imperial,
            7,
            at("2026-09-12T07:15:00Z"),
        )
        .unwrap();

    assert_eq!(
        f.sources[0],
        "National Weather Service has no forecast text for this area, so these sentences are written from Open-Meteo data."
    );
    assert!(f.periods.is_empty());
}

// Environment Canada for Peterborough at 5:10 am UTC: the 05 folder is not
// there yet, the 04 one has nothing for the site, the 03 one has its page.
fn canada_fake(place: &Location) -> Fake {
    Fake::default()
        .with(
            &open_meteo::forecast_url(place, UnitSystem::Metric, 7),
            "open-meteo-toronto.json",
        )
        .with(ec::SITE_LIST_URL, "ec-site-list.csv")
        .answer(
            &ec::hour_folder("ON", 4),
            Ok("<html><body>nothing yet</body></html>".to_string()),
        )
        .with(&ec::hour_folder("ON", 3), "ec-hour-listing-on-03.html")
        .with(
            &format!(
                "{}20260912T035744.540Z_MSC_CitypageWeather_s0000629_en.xml",
                ec::hour_folder("ON", 3)
            ),
            "ec-citypage-peterborough.xml",
        )
}

#[test]
fn canada_walks_back_to_the_latest_hour_with_the_site_s_page() {
    let place = peterborough();
    let http = canada_fake(&place);
    let f = ForecastService::new()
        .forecast(
            &http,
            &place,
            UnitSystem::Metric,
            7,
            at("2026-09-12T05:10:00Z"),
        )
        .unwrap();

    assert_eq!(f.periods.len(), 12);
    assert_eq!(
        f.current.station.as_deref(),
        Some("Peterborough Municipal Airport")
    );
    let folders: Vec<String> = http
        .asked()
        .into_iter()
        .filter(|u| u.contains("/ON/"))
        .collect();
    assert_eq!(
        folders,
        [
            ec::hour_folder("ON", 5),
            ec::hour_folder("ON", 4),
            ec::hour_folder("ON", 3),
            format!(
                "{}20260912T035744.540Z_MSC_CitypageWeather_s0000629_en.xml",
                ec::hour_folder("ON", 3)
            ),
        ]
    );
}

#[test]
fn the_site_list_is_fetched_once_per_session() {
    let place = peterborough();
    let http = canada_fake(&place);
    let service = ForecastService::new();
    for _ in 0..2 {
        service
            .forecast(
                &http,
                &place,
                UnitSystem::Metric,
                7,
                at("2026-09-12T05:10:00Z"),
            )
            .unwrap();
    }
    assert_eq!(http.times_asked(ec::SITE_LIST_URL), 1);
}

#[test]
fn a_place_far_from_every_site_gets_generated_text_and_says_why() {
    let place = Location::new("Somewhere", None, Some("Canada"), 50.0, -150.0);
    let http = canada_fake(&place);
    let f = ForecastService::new()
        .forecast(
            &http,
            &place,
            UnitSystem::Metric,
            7,
            at("2026-09-12T05:10:00Z"),
        )
        .unwrap();

    assert!(f.periods.is_empty());
    assert_eq!(
        f.sources[0],
        "Environment Canada's forecast text could not be fetched this time (no Environment Canada forecast site within 200 kilometres), so these sentences are written from Open-Meteo data."
    );
}

#[test]
fn an_observation_only_site_is_said_as_a_fact() {
    let place = peterborough();
    let xml = fixture("ec-citypage-peterborough.xml");
    let start = xml.find("<forecastGroup>").unwrap();
    let end = xml.find("</forecastGroup>").unwrap() + "</forecastGroup>".len();
    let http = canada_fake(&place).answer(
        &format!(
            "{}20260912T035744.540Z_MSC_CitypageWeather_s0000629_en.xml",
            ec::hour_folder("ON", 3)
        ),
        Ok(format!("{}{}", &xml[..start], &xml[end..])),
    );
    let f = ForecastService::new()
        .forecast(
            &http,
            &place,
            UnitSystem::Metric,
            7,
            at("2026-09-12T05:10:00Z"),
        )
        .unwrap();

    assert_eq!(
        f.sources[0],
        "Environment Canada has no forecast text for this area, so these sentences are written from Open-Meteo data."
    );
}

#[test]
fn a_slow_service_is_said_to_have_taken_too_long() {
    let place = peterborough();
    let http = canada_fake(&place).answer(
        &ec::hour_folder("ON", 5),
        Err(FetchError::TimedOut {
            host: "dd.weather.gc.ca".into(),
        }),
    );
    let f = ForecastService::new()
        .forecast(
            &http,
            &place,
            UnitSystem::Metric,
            7,
            at("2026-09-12T05:10:00Z"),
        )
        .unwrap();

    assert_eq!(
        f.sources[0],
        "Environment Canada's forecast text could not be fetched this time (it took too long to answer), so these sentences are written from Open-Meteo data."
    );
}

#[test]
fn open_meteo_failing_fails_the_refresh() {
    let place = peterborough();
    let url = open_meteo::forecast_url(&place, UnitSystem::Metric, 7);
    let http = canada_fake(&place).answer(&url, Err(status(503, "Service Unavailable", &url)));
    let result = ForecastService::new().forecast(
        &http,
        &place,
        UnitSystem::Metric,
        7,
        at("2026-09-12T05:10:00Z"),
    );

    assert_eq!(
        result.unwrap_err(),
        ServiceError::Fetch(status(503, "Service Unavailable", &url))
    );
    assert_eq!(
        status(503, "Service Unavailable", &url).to_string(),
        "503 Service Unavailable from api.open-meteo.com"
    );
}

#[test]
fn elsewhere_only_open_meteo_is_asked() {
    let place = Location::new(
        "Paris",
        Some("\u{ce}le-de-France"),
        Some("France"),
        48.8534,
        2.3488,
    );
    let url = open_meteo::forecast_url(&place, UnitSystem::Metric, 7);
    let http = Fake::default().with(&url, "open-meteo-toronto.json");
    let f = ForecastService::new()
        .forecast(
            &http,
            &place,
            UnitSystem::Metric,
            7,
            at("2026-09-12T05:10:00Z"),
        )
        .unwrap();

    assert_eq!(http.asked(), [url]);
    assert_eq!(
        f.sources,
        ["Forecast and current conditions: Open-Meteo (open-meteo.com), licensed CC BY 4.0."]
    );
}

// AlertService: the service that covers the country, most severe first,
// and a failure named rather than read as a quiet day.

#[test]
fn canada_s_alerts_come_from_environment_canada_with_the_location_page() {
    let place = Location::new(
        "Gander",
        Some("Newfoundland and Labrador"),
        Some("Canada"),
        48.9569,
        -54.6089,
    );
    let http = Fake::default().with(&ec::alerts_url(&place), "ec-alerts-gander.json");
    let report = check_alerts(&http, &place, at("2026-09-13T04:00:00Z"));

    assert!(report.checked());
    assert_eq!(report.checked_at, Some(at("2026-09-13T04:00:00Z")));
    assert_eq!(
        report.attribution.as_deref(),
        Some("Environment and Climate Change Canada (weather.gc.ca)")
    );
    assert_eq!(report.alerts.len(), 1);
    assert_eq!(
        report.alerts[0].url.as_deref(),
        Some("https://weather.gc.ca/en/location/index.html?coords=48.957,-54.609")
    );
    assert!(
        ec::alerts_url(&place)
            .ends_with("&skipGeometry=true&bbox=-54.6089,48.9569,-54.6089,48.9569")
    );
}

#[test]
fn us_alerts_come_from_the_nws_most_severe_first() {
    let place = Location::new(
        "Hilo",
        Some("Hawaii"),
        Some("United States"),
        19.7297,
        -155.09,
    );
    let http = Fake::default().with(&nws::alerts_url(&place), "nws-alerts-hilo.json");
    let report = check_alerts(&http, &place, at("2026-09-25T21:30:00Z"));

    assert!(report.checked());
    assert_eq!(
        report.attribution.as_deref(),
        Some("National Weather Service (weather.gov)")
    );
    let severities: Vec<_> = report.alerts.iter().map(|a| a.severity).collect();
    let mut sorted = severities.clone();
    sorted.sort_by(|a, b| b.cmp(a));
    assert_eq!(severities, sorted);
    assert_eq!(
        nws::alerts_url(&place),
        "https://api.weather.gov/alerts/active?point=19.7297,-155.09&status=actual"
    );
}

#[test]
fn a_failed_alert_check_names_the_problem() {
    let place = peterborough();
    let url = ec::alerts_url(&place);
    let unavailable = Fake::default().answer(&url, Err(status(503, "Service Unavailable", &url)));
    let report = check_alerts(&unavailable, &place, at("2026-09-13T04:00:00Z"));
    assert!(!report.checked());
    assert!(report.is_available());
    assert_eq!(report.checked_at, None);
    assert_eq!(
        report.problem.as_deref(),
        Some("503 Service Unavailable from api.weather.gc.ca")
    );

    let slow = Fake::default().answer(&url, Err(FetchError::TimedOut { host: host(&url) }));
    assert_eq!(
        check_alerts(&slow, &place, at("2026-09-13T04:00:00Z"))
            .problem
            .as_deref(),
        Some("the alert service took too long to answer")
    );
}

#[test]
fn elsewhere_alerts_are_not_available_and_nothing_is_asked() {
    let place = Location::new(
        "Paris",
        Some("\u{ce}le-de-France"),
        Some("France"),
        48.8534,
        2.3488,
    );
    let http = Fake::default();
    let report = check_alerts(&http, &place, at("2026-09-13T04:00:00Z"));
    assert_eq!(
        report,
        weatherspell_core::alerts::AlertReport::not_available()
    );
    assert!(http.asked().is_empty());
}
