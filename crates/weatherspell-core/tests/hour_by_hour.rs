// Hour by Hour: which hours are shown and from where, how each reads, the
// headings Ctrl+PageDown moves between, the line it opens at, the note
// about this PC's clock (daylight time ending on one side only), and the
// hour each line of the main text opens it at.

mod common;

use common::{fixture, offset};
use jiff::Timestamp;
use jiff::civil::{DateTime, date};
use jiff::tz::TimeZone;
use weatherspell_core::clock::TimeFormat;
use weatherspell_core::environment_canada as ec;
use weatherspell_core::forecast::{Forecast, OfficialPeriod};
use weatherspell_core::hourly::{self, HourlyText};
use weatherspell_core::layout::SectionLayout;
use weatherspell_core::location::Location;
use weatherspell_core::nws;
use weatherspell_core::official::{self, OfficialForecast};
use weatherspell_core::open_meteo;
use weatherspell_core::units::UnitSystem;
use weatherspell_core::writer::{self, WriterOptions};

fn eastern(local: DateTime) -> Timestamp {
    offset(-4).to_timestamp(local).unwrap()
}

fn peterborough() -> Location {
    Location::new(
        "Peterborough",
        Some("Ontario"),
        Some("Canada"),
        44.3,
        -78.33,
    )
}

fn options(now: Timestamp, pc_zone: TimeZone) -> WriterOptions {
    WriterOptions {
        now,
        pc_zone,
        time_format: TimeFormat::new("h:mm tt", "AM", "PM"),
        refresh_problem: None,
    }
}

fn toronto_zone() -> TimeZone {
    TimeZone::get("America/Toronto").unwrap()
}

// Open-Meteo's week for Toronto (11 to 17 September 2026), fetched at
// 11:15 pm on the 11th.
fn base(location: Location) -> Forecast {
    open_meteo::parse(
        &fixture("open-meteo-toronto.json"),
        &location,
        UnitSystem::Metric,
        eastern(date(2026, 9, 11).at(23, 15, 0, 0)),
    )
    .unwrap()
}

// Environment Canada's page over it: its text, and 24 hours from midnight
// on the 12th.
fn with_ec() -> Forecast {
    official::compose(
        &base(peterborough()),
        &ec::parse(&fixture("ec-citypage-peterborough.xml")).unwrap(),
    )
}

fn lines(text: &HourlyText) -> Vec<&str> {
    text.layout.text.lines().collect()
}

fn line_at(text: &HourlyText, offset: usize) -> &str {
    text.layout.text[offset..].lines().next().unwrap()
}

#[test]
fn the_hours_run_from_the_one_in_progress_under_a_heading_per_day() {
    let f = base(Location::new("Toronto", None, None, 43.65, -79.38));
    let now = eastern(date(2026, 9, 11).at(22, 40, 0, 0));
    let text = hourly::write(&f, &options(now, toronto_zone()));
    let all = lines(&text);

    assert_eq!(all[0], "Friday, September 11");
    assert_eq!(all[1], "");
    assert!(all[2].starts_with("10:00 pm: "), "{}", all[2]);
    assert!(all[3].starts_with("11:00 pm: "), "{}", all[3]);
    // A blank line, then the next day.
    assert_eq!(all[4], "");
    assert_eq!(all[5], "Saturday, September 12");
    assert!(all[7].starts_with("12:00 am: "));
    // Every hour to the end of the week, one line each.
    assert_eq!(text.hours.len(), 2 + 6 * 24);
    assert_eq!(
        text.hours.last().unwrap().0,
        date(2026, 9, 17).at(23, 0, 0, 0)
    );
    let headings: Vec<&str> = text
        .layout
        .headings
        .iter()
        .map(|(h, _)| h.as_str())
        .collect();
    assert_eq!(
        headings,
        [
            "Friday, September 11",
            "Saturday, September 12",
            "Sunday, September 13",
            "Monday, September 14",
            "Tuesday, September 15",
            "Wednesday, September 16",
            "Thursday, September 17",
            "Sources"
        ]
    );
    for (heading, at) in &text.layout.headings {
        assert!(text.layout.text[*at..].starts_with(heading.as_str()));
    }
    for (_, at) in &text.hours {
        assert!(line_at(&text, *at).ends_with('.'));
    }
    assert_eq!(
        text.layout.text.rsplit("Sources\n\n").next().unwrap(),
        "Hours: Open-Meteo (open-meteo.com), licensed CC BY 4.0.\n\n"
    );
    // Offsets count UTF-16 units, as the layout's always have.
    assert_eq!(text.layout.length, text.layout.text.encode_utf16().count());
}

#[test]
fn a_generated_hour_reads_like_a_part_of_the_day() {
    let f = base(Location::new("Toronto", None, None, 43.65, -79.38));
    let text = hourly::write(
        &f,
        &options(eastern(date(2026, 9, 11).at(16, 5, 0, 0)), toronto_zone()),
    );
    for (_, at) in &text.hours {
        let line = line_at(&text, *at);
        // Time, sky, temperature, perhaps a chance, then the wind, in words.
        let (time, rest) = line.split_once(": ").unwrap();
        assert!(time.ends_with(" am") || time.ends_with(" pm"), "{line}");
        assert!(rest.contains(" degree"), "{line}");
        assert!(
            rest.contains("light wind") || rest.contains("wind from the "),
            "{line}"
        );
        assert!(!rest.contains("km/h") && !rest.contains('%'), "{line}");
        if let Some((before, _)) = rest.split_once(" percent chance of ") {
            let n: i32 = before.rsplit(", ").next().unwrap().parse().unwrap();
            assert!(n >= 20 && n % 10 == 0, "{line}");
        }
    }
}

#[test]
fn environment_canadas_hours_stand_where_it_gives_them() {
    let f = with_ec();
    let now = eastern(date(2026, 9, 11).at(23, 20, 0, 0));
    let text = hourly::write(&f, &options(now, toronto_zone()));
    let all = lines(&text);

    // The hour in progress is Open-Meteo's: the page starts at midnight.
    assert!(all[2].starts_with("11:00 pm: "));
    let midnight = line_at(&text, text.opening(Some(date(2026, 9, 12).at(0, 0, 0, 0))));
    assert_eq!(
        midnight,
        "12:00 am: clear, 10 degrees, wind from the southwest at 10 kilometres an hour."
    );
    // Its variable light wind, its humidex, then a chance in its own
    // words with the number folded in.
    let at = |h: i8| line_at(&text, text.opening(Some(date(2026, 9, 12).at(h, 0, 0, 0))));
    assert_eq!(at(3), "3:00 am: clear, 8 degrees, light wind.");
    assert_eq!(
        at(14),
        "2:00 pm: sunny, 26 degrees, feeling like 31, wind from the south at 20 kilometres an hour."
    );
    assert_eq!(
        at(21),
        "9:00 pm: 40 percent chance of showers, risk of thunderstorms, 19 degrees, wind from the south at 15 kilometres an hour."
    );
    // After its last hour (11 pm), Open-Meteo's again.
    let after = line_at(&text, text.opening(Some(date(2026, 9, 13).at(0, 0, 0, 0))));
    assert!(after.starts_with("12:00 am: "));
    assert!(
        !after.contains("chance of showers, risk"),
        "Open-Meteo words: {after}"
    );
    assert!(text.layout.text.ends_with(
        "Sources\n\nHours up to 11:00 pm tomorrow: Environment and Climate Change Canada (weather.gc.ca), forecast for Peterborough City - Lakefield - Southern Peterborough County.\n\nOther hours: Open-Meteo (open-meteo.com), licensed CC BY 4.0.\n\n"
    ));
}

#[test]
fn a_services_hours_are_left_out_when_its_text_is_not_the_one_shown() {
    let mut f = with_ec();
    // As when a cached forecast or a failed fetch leaves Open-Meteo's text.
    f.periods.clear();
    let text = hourly::write(
        &f,
        &options(eastern(date(2026, 9, 11).at(23, 20, 0, 0)), toronto_zone()),
    );
    assert!(!text.layout.text.contains("feeling like 31"));
    assert!(
        text.layout
            .text
            .ends_with("Sources\n\nHours: Open-Meteo (open-meteo.com), licensed CC BY 4.0.\n\n")
    );
}

fn spokane() -> Location {
    let mut l = Location::new(
        "Spokane",
        Some("Washington"),
        Some("United States"),
        47.66,
        -117.43,
    );
    l.time_zone_id = Some("America/Los_Angeles".to_string());
    l
}

// The NWS's hours for Spokane (27 October to 1 November 2026, through the
// end of daylight time), over a base whose own hours are long past.
fn with_nws() -> Forecast {
    let mut f = base(spokane());
    f.units = UnitSystem::Imperial;
    f.fetched_at = "2026-10-27T07:15:00Z".parse().unwrap();
    f.time_zone = Some("America/Los_Angeles".to_string());
    let official = OfficialForecast {
        source_name: nws::SOURCE_NAME.to_string(),
        attribution: "National Weather Service (weather.gov), forecast for Spokane, WA".to_string(),
        periods: vec![OfficialPeriod {
            name: "Tonight".to_string(),
            date: date(2026, 10, 27),
            text: "Showers.".to_string(),
        }],
        observation: None,
        hours: nws::parse_hourly(&fixture("nws-hourly-spokane.json")).unwrap(),
        hours_problem: None,
    };
    official::compose(&f, &official)
}

fn pacific_now() -> Timestamp {
    "2026-10-27T07:20:00Z".parse().unwrap()
}

#[test]
fn the_nws_words_are_spoken_with_their_chance() {
    let f = with_nws();
    let text = hourly::write(&f, &options(pacific_now(), toronto_zone()));
    let first = line_at(&text, text.hours[0].1);
    assert_eq!(
        first,
        "12:00 am: 50 percent chance of rain showers, 44 degrees, light wind."
    );
    let all = &text.layout.text;
    assert!(
        all.contains(": 20 percent chance of rain showers, "),
        "{all}"
    );
    assert!(!all.contains("chance rain"));
    assert!(!all.contains(" mph"));
    assert!(all.contains(" miles an hour") || all.contains("light wind"));
}

#[test]
fn the_hour_the_clocks_go_back_is_read_once() {
    let f = with_nws();
    let text = hourly::write(&f, &options(pacific_now(), toronto_zone()));
    let november = text
        .hours
        .iter()
        .filter(|(t, _)| t.date() == date(2026, 11, 1))
        .map(|(t, _)| t.hour())
        .collect::<Vec<_>>();
    assert_eq!(&november[..4], [0, 1, 2, 3]);
    // Wall-clock times on both sides of the change.
    assert!(
        text.hours
            .iter()
            .any(|(t, _)| *t == date(2026, 11, 1).at(9, 0, 0, 0))
    );
}

#[test]
fn a_clock_that_differs_is_said_once_with_the_day_it_changes() {
    let f = with_nws();
    let text = hourly::write(&f, &options(pacific_now(), toronto_zone()));
    let note = lines(&text)[2];
    // Toronto and Spokane both leave daylight time on 1 November, three
    // hours apart: the hours between are not a change worth a word.
    assert_eq!(note, "Times are local to Spokane, 3 hours behind yours.");
    // The hour in progress opens at the top, so the note is heard; a later
    // hour opens on its line.
    assert_eq!(text.opening(None), 0);
    assert_eq!(text.opening(Some(date(2026, 10, 27).at(0, 0, 0, 0))), 0);
    assert!(
        line_at(&text, text.opening(Some(date(2026, 10, 27).at(3, 0, 0, 0))))
            .starts_with("3:00 am: ")
    );
    assert_eq!(lines(&text)[3], "");
    assert!(lines(&text)[4].starts_with("12:00 am: "));

    // Berlin's clocks went back a week before this PC's.
    let mut berlin = with_nws();
    berlin.location = Location::new("Berlin", None, Some("Germany"), 52.52, 13.4);
    berlin.location.nickname = Some("Office".to_string());
    berlin.time_zone = Some("Europe/Berlin".to_string());
    let text = hourly::write(&berlin, &options(pacific_now(), toronto_zone()));
    assert_eq!(
        lines(&text)[2],
        "Times are local to Office, 5 hours ahead of yours until Sunday, then 6 hours ahead."
    );

    // A PC in the same zone hears nothing.
    let text = hourly::write(
        &f,
        &options(pacific_now(), TimeZone::get("America/Los_Angeles").unwrap()),
    );
    assert!(lines(&text)[2].starts_with("12:00 am: "));

    // Kathmandu's quarter hour.
    let text = hourly::write(
        &f,
        &options(pacific_now(), TimeZone::get("Asia/Kathmandu").unwrap()),
    );
    assert_eq!(
        lines(&text)[2],
        "Times are local to Spokane, 12 hours and 45 minutes behind yours until Sunday, then 13 hours and 45 minutes behind."
    );
}

#[test]
fn it_opens_on_the_hour_asked_for_or_the_nearest_there_is() {
    let f = base(Location::new("Toronto", None, None, 43.65, -79.38));
    let text = hourly::write(
        &f,
        &options(eastern(date(2026, 9, 11).at(22, 40, 0, 0)), toronto_zone()),
    );
    let first = text.hours[0].1;
    assert_eq!(text.opening(None), first);
    // Already past: the hour in progress.
    assert_eq!(text.opening(Some(date(2026, 9, 11).at(6, 0, 0, 0))), first);
    assert!(
        line_at(&text, text.opening(Some(date(2026, 9, 13).at(19, 0, 0, 0))))
            .starts_with("7:00 pm: ")
    );
    // Half past: the next hour's line.
    assert!(
        line_at(&text, text.opening(Some(date(2026, 9, 13).at(6, 30, 0, 0))))
            .starts_with("7:00 am: ")
    );
    // Beyond the forecast: its last hour.
    assert_eq!(
        text.opening(Some(date(2026, 9, 30).at(0, 0, 0, 0))),
        text.hours.last().unwrap().1
    );
}

#[test]
fn an_old_forecast_says_so_and_one_with_no_hours_left_says_that() {
    let f = base(Location::new("Toronto", None, None, 43.65, -79.38));
    let mut o = options(eastern(date(2026, 9, 12).at(9, 0, 0, 0)), toronto_zone());
    o.refresh_problem = Some("couldn't reach the weather service".to_string());
    let text = hourly::write(&f, &o);
    assert_eq!(
        lines(&text)[2],
        "Showing the forecast from 9 hours ago; couldn't reach the weather service."
    );

    let text = hourly::write(
        &f,
        &options(eastern(date(2026, 9, 25).at(9, 0, 0, 0)), toronto_zone()),
    );
    assert!(text.hours.is_empty());
    assert_eq!(text.opening(None), 0);
    assert!(text.layout.text.starts_with("Hour by hour\n\nShowing the forecast from 13 days ago.\n\nThere are no hours left in this forecast. Press F5 in the main window to update it.\n\nSources\n\n"));
}

#[test]
fn a_service_whose_hours_could_not_be_had_is_named() {
    let mut f = with_nws();
    let hours = f.official_hours.as_mut().unwrap();
    hours.hours.clear();
    hours.problem = Some("503 Service Unavailable from api.weather.gov".to_string());
    f.source_name = "National Weather Service and Open-Meteo".to_string();
    let text = hourly::write(
        &f,
        &options(eastern(date(2026, 9, 11).at(22, 40, 0, 0)), toronto_zone()),
    );
    assert!(text.layout.text.ends_with("Sources\n\nNational Weather Service's hour by hour could not be fetched this time (503 Service Unavailable from api.weather.gov), so these hours are written from Open-Meteo data.\n\nHours: Open-Meteo (open-meteo.com), licensed CC BY 4.0.\n\n"), "{}", text.layout.text);
}

// The main text: each line that forecasts something opens Hour by Hour at
// the hour it starts.
fn main_layout(f: &Forecast, now: Timestamp) -> SectionLayout {
    SectionLayout::build(&writer::write(f, &options(now, toronto_zone()), None))
}

fn hour_of(layout: &SectionLayout, words: &str) -> Option<DateTime> {
    let at = layout
        .text
        .find(words)
        .unwrap_or_else(|| panic!("no \"{words}\" in {}", layout.text));
    layout.hour_at(layout.text[..at].encode_utf16().count())
}

#[test]
fn each_line_of_the_generated_text_opens_at_its_own_hour() {
    let f = base(Location::new("Toronto", None, None, 43.65, -79.38));
    let now = eastern(date(2026, 9, 11).at(10, 40, 0, 0));
    let layout = main_layout(&f, now);
    let ten = Some(date(2026, 9, 11).at(10, 0, 0, 0));
    assert_eq!(hour_of(&layout, "Right now"), ten);
    assert_eq!(hour_of(&layout, "As of "), ten);
    assert_eq!(hour_of(&layout, "Rest of today"), ten);
    assert_eq!(hour_of(&layout, "This morning: "), ten);
    assert_eq!(
        hour_of(&layout, "This afternoon: "),
        Some(date(2026, 9, 11).at(12, 0, 0, 0))
    );
    assert_eq!(
        hour_of(&layout, "Overnight: "),
        Some(date(2026, 9, 11).at(21, 0, 0, 0))
    );
    assert_eq!(hour_of(&layout, "The sun "), ten);
    // A day from its sunrise (6:53 am), its night from sunset (7:33 pm).
    assert_eq!(
        hour_of(&layout, "Saturday, September 12"),
        Some(date(2026, 9, 12).at(6, 0, 0, 0))
    );
    assert_eq!(
        hour_of(&layout, "Saturday: "),
        Some(date(2026, 9, 12).at(6, 0, 0, 0))
    );
    assert_eq!(
        hour_of(&layout, "Saturday night: "),
        Some(date(2026, 9, 12).at(19, 0, 0, 0))
    );
    assert_eq!(hour_of(&layout, "Alerts"), None);
    assert_eq!(hour_of(&layout, "Sources"), None);
    assert_eq!(hour_of(&layout, "Forecast and current conditions"), None);

    // Ctrl+H on a blank line: the hour of the line above it in the same
    // section; never a line of another section's.
    let afternoon = layout.text.find("This afternoon: ").unwrap();
    let blank = afternoon + layout.text[afternoon..].find('\n').unwrap() + 1;
    assert_eq!(layout.hour_at(blank), None);
    assert_eq!(
        layout.hour_near(blank),
        Some(date(2026, 9, 11).at(12, 0, 0, 0))
    );
    let sources = layout.text.find("Forecast and current conditions").unwrap();
    assert_eq!(layout.hour_near(sources), None);
    assert_eq!(layout.hour_near(sources - 1), None);
    assert_eq!(layout.hour_near(0), None);
}

#[test]
fn each_period_of_a_services_text_opens_where_it_begins() {
    let f = with_ec();
    let now = eastern(date(2026, 9, 11).at(23, 59, 0, 0));
    let layout = main_layout(&f, now);
    assert_eq!(
        hour_of(&layout, "Tonight: "),
        Some(date(2026, 9, 11).at(19, 0, 0, 0))
    );
    assert_eq!(
        hour_of(&layout, "Saturday: "),
        Some(date(2026, 9, 12).at(6, 0, 0, 0))
    );
    assert_eq!(
        hour_of(&layout, "Saturday night: "),
        Some(date(2026, 9, 12).at(19, 0, 0, 0))
    );
    // A blank line opens nothing.
    let blank = layout.text.find("\n\n").unwrap() + 1;
    assert_eq!(layout.hour_at(blank), None);
}

#[test]
fn a_period_named_for_the_afternoon_or_overnight_opens_then() {
    let mut f = with_ec();
    f.periods = vec![
        OfficialPeriod {
            name: "This afternoon".to_string(),
            date: date(2026, 9, 11),
            text: "Sunny.".to_string(),
        },
        OfficialPeriod {
            name: "Overnight".to_string(),
            date: date(2026, 9, 12),
            text: "Clear.".to_string(),
        },
        OfficialPeriod {
            name: "Veterans Day".to_string(),
            date: date(2026, 9, 12),
            text: "Sunny.".to_string(),
        },
    ];
    let layout = main_layout(&f, eastern(date(2026, 9, 11).at(13, 10, 0, 0)));
    assert_eq!(
        hour_of(&layout, "This afternoon: "),
        Some(date(2026, 9, 11).at(12, 0, 0, 0))
    );
    assert_eq!(
        hour_of(&layout, "Overnight: "),
        Some(date(2026, 9, 12).at(0, 0, 0, 0))
    );
    assert_eq!(
        hour_of(&layout, "Veterans Day: "),
        Some(date(2026, 9, 12).at(6, 0, 0, 0))
    );
}
