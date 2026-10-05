// The port's forecast text for a place, fetched live, as the app will show
// it: a quick look at the real services before there is a window.
//
//     cargo run -p weatherspell-core --features net --example forecast -- "Peterborough, Canada" [--imperial]
//
// The first place the geocoder finds whose full name contains the words
// after the comma is used; this PC's time zone, and the 12-hour clock.

use jiff::Timestamp;
use jiff::tz::TimeZone;
use weatherspell_core::clock::TimeFormat;
use weatherspell_core::fetch::{Fetch, ForecastService, check_alerts};
use weatherspell_core::net::UreqFetch;
use weatherspell_core::open_meteo;
use weatherspell_core::units::{self, UnitSystem};
use weatherspell_core::writer::{self, WriterOptions};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let region = if args.iter().any(|a| a == "--imperial") {
        UnitSystem::Imperial
    } else {
        UnitSystem::Metric
    };
    let query: Vec<&str> = args
        .iter()
        .filter(|a| !a.starts_with("--"))
        .map(String::as_str)
        .collect();
    let query = if query.is_empty() {
        "Peterborough".to_string()
    } else {
        query.join(" ")
    };

    let http = UreqFetch::new("Weatherspell/0.0.0-example");
    let (name, filter) = query.split_once(',').unwrap_or((&query, ""));
    let found = http
        .get(&open_meteo::search_url(name), None)
        .expect("the geocoder answers");
    let places = open_meteo::parse_search(&found).expect("the geocoder's answer reads");
    let Some(place) = places
        .into_iter()
        .find(|p| p.full_name().contains(filter.trim()))
    else {
        eprintln!("Nothing found for {query}.");
        std::process::exit(1);
    };
    let units = units::for_location(&place, region);
    eprintln!("{} ({:?})", place.full_name(), units);

    let now = Timestamp::now();
    let forecast = match ForecastService::new().forecast(&http, &place, units, 7, now) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("The forecast could not be fetched: {e}.");
            std::process::exit(1);
        }
    };
    let options = WriterOptions {
        now,
        pc_zone: TimeZone::system(),
        time_format: TimeFormat::new("h:mm tt", "AM", "PM"),
        refresh_problem: None,
    };
    let alerts = check_alerts(&http, &place, now);
    for section in writer::write(&forecast, &options, Some(&alerts)) {
        println!("{}\n", section.heading);
        for paragraph in section.paragraphs {
            println!("{paragraph}\n");
        }
        println!();
    }
}
