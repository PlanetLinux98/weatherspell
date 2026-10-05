// AlertTests.cs, ported: the two services' alerts, their order and
// tracking, and their words. (The forecast text leading with the alerts is
// in official_writer.rs, beside the base forecast it uses.)

mod common;

use common::fixture;
use jiff::civil::{DateTime, date};
use jiff::tz::{Offset, TimeZone};
use jiff::{SignedDuration, Timestamp};
use weatherspell_core::alerts::{self, AlertKind, AlertReport, AlertSeverity, WeatherAlert};
use weatherspell_core::clock::{Clock, TimeFormat};
use weatherspell_core::environment_canada as ec;
use weatherspell_core::nws;

fn at(local: DateTime, offset_minutes: i32) -> Timestamp {
    Offset::from_seconds(offset_minutes * 60)
        .unwrap()
        .to_timestamp(local)
        .unwrap()
}

fn utc(stamp: &str) -> Timestamp {
    stamp.parse().unwrap()
}

// --- NwsAlertParsingTests

#[test]
fn the_zone_and_county_copies_of_one_product_become_one_alert() {
    let alerts = nws::parse_alerts(&fixture("nws-alerts-spokane.json")).unwrap();
    assert_eq!(alerts.len(), 1);
    let a = &alerts[0];
    assert_eq!(a.id, "nws:KOTX.FF.A.0001.2026");
    assert_eq!(a.event, "Flash flood watch");
    assert_eq!(a.severity, AlertSeverity::Severe);
    assert_eq!(a.kind(), AlertKind::Watch);
    assert_eq!(a.issued, at(date(2026, 9, 12).at(19, 41, 0, 0), -7 * 60));
    assert_eq!(a.onset, Some(at(date(2026, 9, 13).at(2, 0, 0, 0), -7 * 60)));
    assert_eq!(a.ends, Some(at(date(2026, 9, 13).at(16, 0, 0, 0), -7 * 60)));
    assert_eq!(a.source, "the National Weather Service");
    assert_eq!(a.sender, "NWS Spokane WA");
    assert_eq!(a.area, "Spokane Area");
    assert_eq!(a.level, None);
    assert!(a.description.starts_with("* WHAT...Flash flooding"));
    assert!(
        a.instruction
            .as_deref()
            .unwrap()
            .starts_with("You should monitor later forecasts")
    );
    assert_eq!(
        a.url.as_deref(),
        Some("https://forecast.weather.gov/wwamap/wwatxtget.php?cwa=OTX&wwa=flash%20flood%20watch")
    );
}

#[test]
fn the_event_end_is_preferred_to_the_message_expiry() {
    // The statement is reissued at 7 am but the hazard lasts until 11 pm.
    let alerts = nws::parse_alerts(&fixture("nws-alerts-muskegon.json")).unwrap();
    assert_eq!(alerts.len(), 1);
    let a = &alerts[0];
    assert_eq!(a.event, "Beach hazards statement");
    assert_eq!(a.kind(), AlertKind::Statement);
    assert_eq!(a.severity, AlertSeverity::Moderate);
    assert_eq!(a.ends, Some(at(date(2026, 9, 13).at(23, 0, 0, 0), -4 * 60)));
}

#[test]
fn a_message_expiry_is_never_taken_for_the_end() {
    // Hilo on 2026-09-25: the tropical products give no end, only when the
    // message runs out (the next update); the flood watch gives both.
    let all = nws::parse_alerts(&fixture("nws-alerts-hilo.json")).unwrap();
    let one = |event: &str| {
        let found: Vec<&WeatherAlert> = all.iter().filter(|a| a.event == event).collect();
        assert_eq!(found.len(), 1, "{event}");
        found[0].clone()
    };
    let hurricane = one("Hurricane watch");
    let flood = one("Flood watch");

    assert_eq!(hurricane.ends, None);
    assert_eq!(
        hurricane.expires,
        Some(at(date(2026, 9, 25).at(23, 15, 0, 0), -10 * 60))
    );
    assert_eq!(
        flood.ends,
        Some(at(date(2026, 9, 26).at(18, 0, 0, 0), -10 * 60))
    );
    assert_eq!(
        flood.expires,
        Some(at(date(2026, 9, 25).at(18, 45, 0, 0), -10 * 60))
    );

    // Ordered by its next update, the watch with no end still comes before
    // the flood watch that ends tomorrow.
    let ordered: Vec<String> = alerts::order(all.clone())
        .into_iter()
        .map(|a| a.event)
        .collect();
    let position = |e: &str| ordered.iter().position(|x| x == e).unwrap();
    assert!(position("Hurricane watch") < position("Flood watch"));
}

#[test]
fn an_update_keeps_the_identity_of_the_event_it_continues() {
    let alerts = nws::parse_alerts(&fixture("nws-alerts-pawhuska.json")).unwrap();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].id, "nws:KTSA.HT.Y.0058.2026");
    assert_eq!(alerts[0].event, "Heat advisory");
    assert_eq!(alerts[0].sender, "NWS Tulsa OK");
}

#[test]
fn an_nws_point_with_nothing_in_effect_gives_an_empty_list() {
    assert!(
        nws::parse_alerts(&fixture("nws-alerts-albany.json"))
            .unwrap()
            .is_empty()
    );
}

// --- EnvironmentCanadaAlertTests

fn ec_now() -> Timestamp {
    utc("2026-09-13T04:00:00Z")
}

#[test]
fn an_alert_at_a_point_carries_its_text_level_and_times() {
    let url = "https://weather.gc.ca/en/location/index.html?coords=48.96,-54.61";
    let alerts = ec::parse_alerts(&fixture("ec-alerts-gander.json"), ec_now(), Some(url)).unwrap();
    assert_eq!(alerts.len(), 1);
    let a = &alerts[0];
    assert_eq!(a.id, "ec:64919237566271632202609120507");
    assert_eq!(a.event, "Frost advisory");
    assert_eq!(a.kind(), AlertKind::Advisory);
    // Yellow outranks what an advisory would be on its own.
    assert_eq!(a.severity, AlertSeverity::Moderate);
    assert_eq!(a.issued, utc("2026-09-13T01:05:46.284Z"));
    assert_eq!(a.onset, Some(utc("2026-09-13T00:55:00Z")));
    // The event's end, not the message's expiry an hour later.
    assert_eq!(a.ends, Some(utc("2026-09-13T09:00:00Z")));
    assert_eq!(a.source, "Environment Canada");
    assert_eq!(a.sender, "Environment Canada");
    assert_eq!(a.area, "Gander and vicinity");
    assert_eq!(
        a.level.as_deref(),
        Some("Yellow level, moderate impact, high confidence")
    );
    assert!(a.description.starts_with("Areas of frost are expected."));
    assert_eq!(a.instruction, None);
    assert_eq!(a.url.as_deref(), Some(url));
}

#[test]
fn continued_warnings_are_kept_and_told_apart() {
    let alerts = ec::parse_alerts(&fixture("ec-alerts-warnings.json"), ec_now(), None).unwrap();
    assert_eq!(alerts.len(), 2);
    assert!(alerts.iter().all(|a| a.event == "Storm surge warning"));
    // A yellow warning is still a warning: Severe, not Moderate.
    assert!(alerts.iter().all(|a| a.severity == AlertSeverity::Severe));
    assert_ne!(alerts[0].id, alerts[1].id);
    assert_eq!(alerts[1].ends, Some(utc("2026-09-15T18:00:00Z")));
}

#[test]
fn a_stale_alert_is_dropped() {
    assert!(
        ec::parse_alerts(
            &fixture("ec-alerts-gander.json"),
            utc("2026-09-14T00:00:00Z"),
            None
        )
        .unwrap()
        .is_empty()
    );
}

#[test]
fn an_ec_point_with_nothing_in_effect_gives_an_empty_list() {
    assert!(
        ec::parse_alerts(&fixture("ec-alerts-peterborough.json"), ec_now(), None)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn severity_is_the_colour_level_with_the_type_as_a_floor() {
    use AlertSeverity::*;
    for (colour, kind, expected) in [
        (Some("red"), Some("advisory"), Extreme),
        (Some("orange"), Some("watch"), Severe),
        (Some("yellow"), Some("warning"), Severe),
        (Some("yellow"), Some("statement"), Moderate),
        (None, Some("warning"), Severe),
        (None, Some("watch"), Moderate),
        (None, Some("advisory"), Minor),
        (Some(""), Some("statement"), Minor),
        (None, None, Unknown),
    ] {
        assert_eq!(
            ec::alert_severity(colour, kind),
            expected,
            "{colour:?} {kind:?}"
        );
    }
}

// --- AlertOrderingAndTrackingTests

fn eastern(local: DateTime) -> Timestamp {
    at(local, -4 * 60)
}

fn simple(id: &str, event: &str, severity: AlertSeverity, ends_hour: i8) -> WeatherAlert {
    WeatherAlert {
        id: id.to_string(),
        event: event.to_string(),
        severity,
        issued: eastern(date(2026, 9, 11).at(12, 0, 0, 0)),
        onset: None,
        ends: Some(eastern(date(2026, 9, 11).at(ends_hour, 0, 0, 0))),
        source: "Environment Canada".to_string(),
        sender: "Environment Canada".to_string(),
        area: "Somewhere".to_string(),
        level: None,
        description: "Text.".to_string(),
        instruction: None,
        url: None,
        expires: None,
    }
}

fn ids(alerts: &[WeatherAlert]) -> Vec<&str> {
    alerts.iter().map(|a| a.id.as_str()).collect()
}

#[test]
fn most_severe_first_then_warnings_before_watches_then_soonest_to_end() {
    let ordered = alerts::order(vec![
        simple("a", "Frost advisory", AlertSeverity::Minor, 8),
        simple("b", "Severe thunderstorm watch", AlertSeverity::Severe, 21),
        simple("c", "Rainfall warning", AlertSeverity::Severe, 23),
        simple("d", "Tornado warning", AlertSeverity::Extreme, 15),
        simple(
            "e",
            "Severe thunderstorm warning",
            AlertSeverity::Severe,
            18,
        ),
    ]);
    assert_eq!(ids(&ordered), ["d", "e", "c", "b", "a"]);
}

#[test]
fn only_unseen_alerts_are_new_and_the_seen_list_follows_what_is_in_effect() {
    let mut seen = Vec::new();
    let a = simple("a", "Rainfall warning", AlertSeverity::Severe, 18);
    let b = simple("b", "Frost advisory", AlertSeverity::Minor, 8);
    let c = simple("c", "Fog advisory", AlertSeverity::Minor, 9);

    assert_eq!(
        ids(&alerts::track(&mut seen, &[a.clone(), b.clone()])),
        ["a", "b"]
    );
    assert_eq!(seen, ["a", "b"]);

    assert_eq!(
        ids(&alerts::track(&mut seen, &[b.clone(), c.clone()])),
        ["c"]
    );
    assert_eq!(seen, ["b", "c"]);

    assert!(alerts::track(&mut seen, &[]).is_empty());
    assert!(seen.is_empty());

    // Issued afresh after ending: new again.
    assert_eq!(ids(&alerts::track(&mut seen, &[b])), ["b"]);
}

const EC_ATTRIBUTION: &str = "Environment Canada (weather.gc.ca)";

fn report(
    alerts: Vec<WeatherAlert>,
    problem: Option<&str>,
    checked_at: Option<Timestamp>,
) -> AlertReport {
    AlertReport {
        alerts,
        attribution: Some(EC_ATTRIBUTION.to_string()),
        problem: problem.map(str::to_string),
        checked_at,
    }
}

#[test]
fn a_failed_check_carries_the_last_known_alerts_and_their_time() {
    let a = simple("a", "Rainfall warning", AlertSeverity::Severe, 18);
    let checked_at = eastern(date(2026, 9, 11).at(12, 0, 0, 0));
    let earlier = report(vec![a.clone()], None, Some(checked_at));
    let failed = report(
        vec![],
        Some("503 Service Unavailable from api.weather.gc.ca"),
        None,
    );

    let dated = failed.or_last_known(Some(&earlier));
    assert_eq!(dated.alerts, [a]);
    assert_eq!(dated.checked_at, Some(checked_at));
    assert_eq!(dated.problem, failed.problem);
    assert!(!dated.checked());

    // Carried on again through a second outage, still dated by the check
    // that succeeded.
    assert_eq!(
        failed.or_last_known(Some(&dated)).checked_at,
        Some(checked_at)
    );

    // A check that succeeded, a region no source covers, and nothing known
    // are all left as they are.
    assert_eq!(earlier.or_last_known(Some(&dated)), earlier);
    assert_eq!(
        AlertReport::not_available().or_last_known(Some(&earlier)),
        AlertReport::not_available()
    );
    assert_eq!(failed.or_last_known(None), failed);
    assert_eq!(failed.or_last_known(Some(&failed)), failed);
    assert_eq!(
        failed.or_last_known(Some(&AlertReport::not_available())),
        failed
    );
}

// --- AlertWriterTests

// 2:45 pm on Friday 11 September 2026, location time.
fn now_local() -> DateTime {
    date(2026, 9, 11).at(14, 45, 0, 0)
}

fn clock_in(location_minutes: i32, pc_minutes: i32) -> Clock {
    Clock::new(
        Offset::from_seconds(location_minutes * 60).unwrap(),
        TimeZone::fixed(Offset::from_seconds(pc_minutes * 60).unwrap()),
        TimeFormat::new("h:mm tt", "AM", "PM"),
    )
}

fn eastern_clock() -> Clock {
    clock_in(-4 * 60, -4 * 60)
}

fn alert(event: &str, ends: Option<Timestamp>, severity: AlertSeverity) -> WeatherAlert {
    WeatherAlert {
        id: format!("id-{event}"),
        event: event.to_string(),
        severity,
        issued: eastern(date(2026, 9, 11).at(13, 10, 0, 0)),
        onset: None,
        ends,
        source: "Environment Canada".to_string(),
        sender: "Environment Canada".to_string(),
        area: "Peterborough City - Lakefield - Southern Peterborough County".to_string(),
        level: None,
        description: "Text.".to_string(),
        instruction: None,
        url: None,
        expires: None,
    }
}

fn severe(event: &str, ends: Option<Timestamp>) -> WeatherAlert {
    alert(event, ends, AlertSeverity::Severe)
}

#[test]
fn lines_name_the_event_when_it_ends_and_the_source() {
    let clock = eastern_clock();
    let line = |a: WeatherAlert| alerts::line(&a, &clock, now_local());

    assert_eq!(
        line(severe(
            "Severe thunderstorm warning",
            Some(eastern(date(2026, 9, 11).at(18, 0, 0, 0)))
        )),
        "Severe thunderstorm warning until 6:00 pm today, from Environment Canada. Press Enter for details."
    );
    assert_eq!(
        line(severe(
            "Frost advisory",
            Some(eastern(date(2026, 9, 12).at(6, 30, 0, 0)))
        )),
        "Frost advisory until 6:30 am tomorrow, from Environment Canada. Press Enter for details."
    );
    assert_eq!(
        line(severe(
            "Rainfall warning",
            Some(eastern(date(2026, 9, 13).at(6, 30, 0, 0)))
        )),
        "Rainfall warning until 6:30 am Sunday, from Environment Canada. Press Enter for details."
    );
    assert_eq!(
        line(severe(
            "Storm surge warning",
            Some(eastern(date(2026, 9, 20).at(6, 30, 0, 0)))
        )),
        "Storm surge warning until 6:30 am on September 20, from Environment Canada. Press Enter for details."
    );
    assert_eq!(
        line(severe("Special weather statement", None)),
        "Special weather statement from Environment Canada. Press Enter for details."
    );
}

#[test]
fn end_times_are_the_location_s_own_with_the_pc_time_in_brackets() {
    // The alert's own stamp is UTC, as Environment Canada sends them; the PC
    // is three hours behind the location.
    let a = severe("Rainfall warning", Some(utc("2026-09-12T02:00:00Z")));
    assert_eq!(
        alerts::line(&a, &clock_in(-4 * 60, -7 * 60), now_local()),
        "Rainfall warning until 10:00 pm today (7:00 pm today your time), from Environment Canada. Press Enter for details."
    );
}

#[test]
fn the_section_states_a_gap_a_failure_or_a_quiet_day_outright() {
    let clock = eastern_clock();

    let unsupported = alerts::section(Some(&AlertReport::not_available()), &clock, now_local());
    assert_eq!(unsupported.heading, "Alerts");
    assert_eq!(
        unsupported.paragraphs,
        ["Alerts are not available for this region."]
    );
    assert_eq!(unsupported.alerts, None);

    assert_eq!(
        alerts::section(None, &clock, now_local()).paragraphs,
        ["Alerts are not available for this region."]
    );

    let failed = alerts::section(
        Some(&report(
            vec![],
            Some("503 Service Unavailable from api.weather.gc.ca"),
            None,
        )),
        &clock,
        now_local(),
    );
    assert_eq!(
        failed.paragraphs,
        [
            "Alerts couldn't be checked this time (503 Service Unavailable from api.weather.gc.ca). Press F5 to try again."
        ]
    );

    assert_eq!(
        alerts::section(Some(&report(vec![], None, None)), &clock, now_local()).paragraphs,
        ["No alerts in effect."]
    );
}

#[test]
fn a_failed_check_reads_the_last_known_alerts_dated_and_without_those_that_have_ended() {
    let clock = eastern_clock();
    let over = severe(
        "Fog advisory",
        Some(eastern(date(2026, 9, 11).at(14, 0, 0, 0))),
    );
    let running = severe(
        "Rainfall warning",
        Some(eastern(date(2026, 9, 11).at(18, 0, 0, 0))),
    );
    let open = severe("Special weather statement", None);
    let checked_at = eastern(date(2026, 9, 11).at(12, 40, 0, 0));
    let failed = report(
        vec![],
        Some("503 Service Unavailable from api.weather.gc.ca"),
        None,
    );

    let earlier = report(
        vec![over.clone(), running.clone(), open.clone()],
        None,
        Some(checked_at),
    );
    let section = alerts::section(
        Some(&failed.or_last_known(Some(&earlier))),
        &clock,
        now_local(),
    );
    assert_eq!(
        section.paragraphs,
        [
            "Alerts couldn't be checked this time (503 Service Unavailable from api.weather.gc.ca); showing the alerts from 2 hours ago. Press F5 to try again.",
            "Rainfall warning until 6:00 pm today, from Environment Canada. Press Enter for details.",
            "Special weather statement from Environment Canada. Press Enter for details.",
        ]
    );
    // The note is not an alert line; the lines after it are.
    assert_eq!(section.alerts, Some(vec![None, Some(running), Some(open)]));

    let just_now = eastern(now_local()) - SignedDuration::from_secs(30);
    let quiet = alerts::section(
        Some(&failed.or_last_known(Some(&report(vec![over], None, Some(just_now))))),
        &clock,
        now_local(),
    );
    assert_eq!(
        quiet.paragraphs,
        [
            "Alerts couldn't be checked this time (503 Service Unavailable from api.weather.gc.ca); none were in effect when last checked, less than a minute ago. Press F5 to try again."
        ]
    );
    assert_eq!(quiet.alerts, None);
}

#[test]
fn the_section_keeps_each_line_s_alert_for_the_enter_key() {
    let a = alert(
        "Tornado warning",
        Some(eastern(date(2026, 9, 11).at(15, 15, 0, 0))),
        AlertSeverity::Extreme,
    );
    let b = severe(
        "Severe thunderstorm watch",
        Some(eastern(date(2026, 9, 11).at(21, 0, 0, 0))),
    );

    let section = alerts::section(
        Some(&report(vec![a.clone(), b.clone()], None, None)),
        &eastern_clock(),
        now_local(),
    );
    assert_eq!(section.paragraphs.len(), 2);
    assert!(section.paragraphs[0].starts_with("Tornado warning until 3:15 pm today"));
    assert_eq!(section.alerts, Some(vec![Some(a), Some(b)]));
}

#[test]
fn announcements_name_the_location_and_join_several_alerts() {
    let clock = eastern_clock();
    let a = alert(
        "Tornado warning",
        Some(eastern(date(2026, 9, 11).at(18, 15, 0, 0))),
        AlertSeverity::Extreme,
    );
    let b = severe(
        "Severe thunderstorm watch",
        Some(eastern(date(2026, 9, 11).at(21, 0, 0, 0))),
    );
    let c = severe("Special weather statement", None);

    assert_eq!(
        alerts::announcement(
            "Peterborough",
            std::slice::from_ref(&a),
            &clock,
            now_local()
        ),
        "Peterborough: tornado warning until 6:15 pm today."
    );
    assert_eq!(
        alerts::announcement("Peterborough", &[a.clone(), b.clone()], &clock, now_local()),
        "Peterborough: tornado warning until 6:15 pm today and severe thunderstorm watch until 9:00 pm today."
    );
    assert_eq!(
        alerts::announcement("Home", &[a, b, c], &clock, now_local()),
        "Home: tornado warning until 6:15 pm today, severe thunderstorm watch until 9:00 pm today and special weather statement."
    );
}

#[test]
fn a_launch_or_a_switch_names_each_kind_of_alert_in_effect_once() {
    let nws_report = |alerts: Vec<WeatherAlert>, problem: Option<&str>| AlertReport {
        alerts,
        attribution: Some("National Weather Service (weather.gov)".to_string()),
        problem: problem.map(str::to_string),
        checked_at: None,
    };
    let a = severe(
        "Frost advisory",
        Some(eastern(date(2026, 9, 12).at(9, 0, 0, 0))),
    );
    let b = severe("Special weather statement", None);
    let mut b2 = b.clone();
    b2.id = "id-second-area".to_string();

    assert_eq!(
        alerts::in_effect(&nws_report(vec![b.clone(), b2], None)).as_deref(),
        Some("special weather statement in effect")
    );
    assert_eq!(
        alerts::in_effect(&nws_report(vec![a.clone(), b], None)).as_deref(),
        Some("frost advisory and special weather statement in effect")
    );
    assert_eq!(alerts::in_effect(&nws_report(vec![], None)), None);
    assert_eq!(alerts::in_effect(&AlertReport::not_available()), None);
    // A failed check carrying the last known alerts: the section dates them,
    // and "in effect" would claim more than is known.
    let mut known = nws_report(vec![a], None);
    known.checked_at = Some(Timestamp::now());
    assert_eq!(
        alerts::in_effect(
            &nws_report(vec![], Some("503 Service Unavailable")).or_last_known(Some(&known))
        ),
        None
    );
}

#[test]
fn details_of_an_nws_alert_read_the_bullets_as_labelled_paragraphs() {
    let alert = nws::parse_alerts(&fixture("nws-alerts-spokane.json"))
        .unwrap()
        .remove(0);
    let clock = clock_in(-7 * 60, -7 * 60);

    let d = alerts::details(&alert, &clock, date(2026, 9, 12).at(20, 0, 0, 0));

    assert_eq!(
        d[0],
        "Flash flood watch from NWS Spokane WA, in effect from 2:00 am tomorrow until 4:00 pm tomorrow."
    );
    assert_eq!(d[1], "Area: Spokane Area.");
    assert_eq!(d[2], "Issued 7:41 pm today.");
    assert_eq!(
        d[3],
        "What: Flash flooding and debris flows caused by excessive rainfall are possible over the burn scar."
    );
    assert_eq!(
        d[4],
        "Where: A portion of Northeast Washington, including the following area and county, Spokane Area."
    );
    assert_eq!(d[5], "When: From 2 AM PDT Sunday through Sunday afternoon.");
    assert!(d[6].starts_with("Impacts: Moderate to heavy rainfall over the burn scar is expected to develop Sunday morning. Rainfall rates"));
    assert_eq!(d[7], "Additional details:");
    assert!(d[8].starts_with("National Weather Service Meteorologists are forecasting"));
    assert_eq!(
        d[9],
        "Some locations that may experience flash flooding include... Spokane and Nine Mile Falls."
    );
    assert_eq!(d[10], "http://www.weather.gov/safety/flood");
    assert_eq!(
        d[11],
        "Instructions: You should monitor later forecasts and be prepared to take action should Flash Flood Warnings be issued."
    );
    assert_eq!(d.len(), 12);
}

#[test]
fn an_alert_with_no_end_reads_without_until_and_its_details_say_when_the_message_expires() {
    let alert = nws::parse_alerts(&fixture("nws-alerts-hilo.json"))
        .unwrap()
        .into_iter()
        .find(|a| a.event == "Hurricane watch")
        .unwrap();
    let clock = clock_in(-10 * 60, -10 * 60);
    let now = date(2026, 9, 25).at(11, 30, 0, 0);

    assert_eq!(
        alerts::line(&alert, &clock, now),
        "Hurricane watch from the National Weather Service. Press Enter for details."
    );
    assert_eq!(
        alerts::announcement("Hilo", std::slice::from_ref(&alert), &clock, now),
        "Hilo: hurricane watch."
    );
    let d = alerts::details(&alert, &clock, now);
    assert_eq!(d[0], "Hurricane watch from NWS Honolulu HI.");
    assert_eq!(
        d[1],
        "No end time is given; this message expires at 11:15 pm today."
    );
}

#[test]
fn details_of_an_environment_canada_alert_carry_its_level_and_drop_the_rule() {
    let alert = ec::parse_alerts(
        &fixture("ec-alerts-gander.json"),
        utc("2026-09-13T01:30:00Z"),
        None,
    )
    .unwrap()
    .remove(0);
    let clock = clock_in(-150, -150);

    // 11 pm NDT on the 12th: the advisory was issued at 10:35 pm and ends at
    // 6:30 am tomorrow (09:00 UTC).
    let d = alerts::details(&alert, &clock, date(2026, 9, 12).at(23, 0, 0, 0));

    assert_eq!(
        d[0],
        "Frost advisory from Environment Canada, in effect until 6:30 am tomorrow."
    );
    assert_eq!(d[1], "Area: Gander and vicinity.");
    assert_eq!(d[2], "Issued 10:35 pm today.");
    assert_eq!(d[3], "Yellow level, moderate impact, high confidence.");
    assert_eq!(d[4], "Areas of frost are expected.");
    assert!(d[5].starts_with("Locations: Deer Lake - Humber Valley,"));
    assert_eq!(
        d[6],
        "Minimum temperatures: +5 to +1 (coolest in low-lying areas)"
    );
    assert_eq!(d[7], "Time span: early Sunday morning.");
    assert!(d[8].starts_with("Remarks: Patchy frost"));
    assert_eq!(d[9], "Damage to plants, trees, and crops is possible.");
    assert!(d[10].starts_with("Please continue to monitor alerts"));
    assert_eq!(d.len(), 11);
}

#[test]
fn paragraphs_join_wrapped_lines_and_speak_symbols() {
    assert_eq!(
        alerts::paragraphs_of(
            "* WHAT...Winds of 90%\nstrength.\r\n\r\nDense fog will persist.\n\n###\n\nVisibility near zero\nat times."
        ),
        [
            "What: Winds of 90 percent strength.",
            "Dense fog will persist.",
            "Visibility near zero at times."
        ]
    );
}
