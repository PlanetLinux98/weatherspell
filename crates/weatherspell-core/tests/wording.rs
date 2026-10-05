// WordingTests.cs, ported (the station names come with the official text,
// and the PC-time lines with the app).

use jiff::SignedDuration;
use weatherspell_core::location::Location;
use weatherspell_core::units::{self, UnitSystem};
use weatherspell_core::{clock, compass, weather_codes};

#[test]
fn compass_points_from_degrees() {
    for (degrees, expected) in [
        (0.0, "north"),
        (11.0, "north"),
        (12.0, "north-northeast"),
        (45.0, "northeast"),
        (142.0, "southeast"),
        (225.0, "southwest"),
        (359.0, "north"),
        (-90.0, "west"),
    ] {
        assert_eq!(compass::from_degrees(degrees), expected, "{degrees}");
    }
}

#[test]
fn weather_codes_become_phrases() {
    for (code, expected) in [
        (0, "clear"),
        (2, "partly cloudy"),
        (45, "foggy"),
        (63, "rain"),
        (75, "heavy snow"),
        (80, "light rain showers"),
        (95, "thunderstorms"),
        (99, "thunderstorms with heavy hail"),
        (42, "unknown conditions"),
    ] {
        assert_eq!(weather_codes::describe(code), expected, "{code}");
    }
}

#[test]
fn precipitation_word_follows_the_code_family() {
    assert_eq!(weather_codes::precipitation_word(73), "snow");
    assert_eq!(weather_codes::precipitation_word(85), "snow");
    assert_eq!(weather_codes::precipitation_word(61), "rain");
    assert_eq!(weather_codes::precipitation_word(2), "precipitation");
}

#[test]
fn degrees_are_whole_and_say_minus() {
    for (value, expected) in [
        (21.4, "21 degrees"),
        (21.5, "22 degrees"),
        (-0.4, "0 degrees"),
        (-4.6, "minus 5 degrees"),
        (1.2, "1 degree"),
        (-0.8, "minus 1 degree"),
    ] {
        assert_eq!(units::degrees(value), expected, "{value}");
    }
}

#[test]
fn quantities_are_worded_per_unit_system() {
    use UnitSystem::{Imperial, Metric};
    assert_eq!(units::speed(12.3, Metric), "12 kilometres an hour");
    assert_eq!(units::speed(7.6, Imperial), "8 miles an hour");
    assert_eq!(units::distance(24100.0, Metric), "24 kilometres");
    assert_eq!(units::distance(800.0, Metric), "800 metres");
    assert_eq!(units::distance(800.0, Imperial), "under a mile");
    assert_eq!(units::distance(1200.0, Metric), "1 kilometre");
    assert_eq!(units::distance(1700.0, Imperial), "1 mile");
    assert_eq!(units::pressure(1016.8, Metric), "1017 hectopascals");
    assert_eq!(units::pressure(1016.8, Imperial), "30.03 inches of mercury");
    assert_eq!(
        units::precipitation_amount(0.4, Metric),
        "less than a millimetre"
    );
    assert_eq!(units::precipitation_amount(1.2, Metric), "1 millimetre");
    assert_eq!(units::precipitation_amount(8.4, Metric), "8 millimetres");
    assert_eq!(units::precipitation_amount(0.33, Imperial), "0.3 inches");
    assert_eq!(units::speed(1.2, Metric), "1 kilometre an hour");
    assert_eq!(units::centimetres(1.4), "1 centimetre");
    assert_eq!(units::centimetres(3.6), "4 centimetres");
}

#[test]
fn heavier_hours_of_the_same_weather_are_not_named_twice() {
    use weather_codes::{is_noun, kind};
    assert_eq!(kind(71), kind(73));
    assert_eq!(kind(80), kind(81));
    assert_ne!(kind(51), kind(80));
    assert_ne!(kind(61), kind(66));
    assert!(is_noun(53));
    assert!(is_noun(48));
    assert!(!is_noun(45));
    assert!(!is_noun(3));
}

// One system per location's whole text: Environment Canada writes only
// metric, the NWS writes either, so only Canada overrides the region.
#[test]
fn units_follow_the_location_s_service_or_else_the_region() {
    use UnitSystem::{Imperial, Metric};
    for (country, region, expected) in [
        (Some("Canada"), Imperial, Metric),
        (Some("Canada"), Metric, Metric),
        (Some("United States"), Imperial, Imperial),
        (Some("United States"), Metric, Metric),
        (Some("France"), Imperial, Imperial),
        (None, Metric, Metric),
    ] {
        let location = Location::new("Somewhere", None, country, 45.0, -75.0);
        assert_eq!(
            units::for_location(&location, region),
            expected,
            "{country:?}"
        );
    }
}

#[test]
fn ages_and_durations_read_naturally() {
    let secs = SignedDuration::from_secs;
    let mins = |m: f64| SignedDuration::from_secs_f64(m * 60.0);
    let hours = |h: f64| SignedDuration::from_secs_f64(h * 3600.0);
    assert_eq!(clock::age(secs(20)), "just now");
    assert_eq!(clock::age(secs(90)), "1 minute ago");
    assert_eq!(clock::age(mins(5.4)), "5 minutes ago");
    assert_eq!(clock::age(hours(2.9)), "2 hours ago");
    assert_eq!(clock::age(hours(30.0)), "1 day ago");
    assert_eq!(clock::age_after_from(secs(20)), "less than a minute ago");
    assert_eq!(clock::age_after_from(hours(2.9)), "2 hours ago");
    assert_eq!(clock::duration(45720.0), "12 hours and 42 minutes");
    assert_eq!(clock::duration(32400.0), "9 hours");
    assert_eq!(clock::duration(3660.0), "1 hour and 1 minute");
    assert_eq!(clock::duration(2700.0), "45 minutes");
}
