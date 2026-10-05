// The port's forecast text for a place, fetched live, as the app will show
// it: a quick look at the real services before there is a window.
//
//     cargo run -p weatherspell-core --features net --example forecast -- "Peterborough, Canada" [--imperial]
//
// A place name, postal code or coordinates, as Add Location takes them;
// for a name, the first place found whose full name contains the words
// after the comma. This PC's time zone, and the 12-hour clock.

use jiff::Timestamp;
use jiff::tz::TimeZone;
use weatherspell_core::clock::TimeFormat;
use weatherspell_core::coordinates::{self, CoordinateReading};
use weatherspell_core::fetch::{ForecastService, check_alerts};
use weatherspell_core::location::Location;
use weatherspell_core::net::UreqFetch;
use weatherspell_core::search::LocationSearch;
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
    let search = LocationSearch::new();
    // As Add Location does: coordinates are named, anything else searched
    // (the postal code table and the geocoder).
    let place = match coordinates::read(&query) {
        Some(CoordinateReading {
            problem: Some(problem),
            ..
        }) => {
            eprintln!("{problem}");
            std::process::exit(1);
        }
        Some(point) => search
            .name_point(&http, point.latitude, point.longitude)
            .expect("Nominatim answers")
            .unwrap_or_else(|| Location::at_point(point.latitude, point.longitude)),
        None => {
            let (name, filter) = query.split_once(',').unwrap_or((&query, ""));
            let places = search.search(&http, name).expect("the search answers");
            let Some(place) = places
                .into_iter()
                .find(|p| p.full_name().contains(filter.trim()))
            else {
                eprintln!("Nothing found for {query}.");
                std::process::exit(1);
            };
            place
        }
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
