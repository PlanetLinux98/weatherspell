// SettingsStoreTests, LocationEditorTests and the settings half of
// WindowPlacementTests from the C# app, ported.

mod common;

use common::TempDir;
use weatherspell_core::alerts::AlertSeverity;
use weatherspell_core::location::Location;
use weatherspell_core::location_editor::LocationEditor;
use weatherspell_core::settings::{
    Announcements, AppSettings, SavedLocation, SavedWindow, SettingsStore, choices,
};

fn place(
    name: &str,
    region: Option<&str>,
    country: &str,
    latitude: f64,
    longitude: f64,
    zone: Option<&str>,
) -> Location {
    let mut l = Location::new(name, region, Some(country), latitude, longitude);
    l.time_zone_id = zone.map(str::to_string);
    l
}

fn store(dir: &TempDir) -> SettingsStore {
    SettingsStore::new(dir.join("nested").join("settings.json"))
}

fn write(store: &SettingsStore, contents: &[u8]) {
    std::fs::create_dir_all(store.path().parent().unwrap()).unwrap();
    std::fs::write(store.path(), contents).unwrap();
}

#[test]
fn a_place_already_saved_is_found_by_its_point() {
    let mut settings = AppSettings::default();
    settings.locations.push(SavedLocation::from_location(&place(
        "Peterborough",
        Some("England"),
        "United Kingdom",
        52.57364,
        -0.24777,
        Some("Europe/London"),
    )));
    settings.locations.push(SavedLocation::from_location(&place(
        "Peterborough",
        Some("Ontario"),
        "Canada",
        44.30012,
        -78.31623,
        Some("America/Toronto"),
    )));

    let same = place(
        "Peterborough",
        Some("Ontario"),
        "Canada",
        44.300121,
        -78.316229,
        None,
    );
    let other = place(
        "Peterborough",
        Some("Ontario"),
        "Canada",
        44.3104,
        -78.2396,
        None,
    );
    assert_eq!(settings.index_of(&same), Some(1));
    assert_eq!(settings.index_of(&other), None);
}

#[test]
fn missing_file_gives_defaults_without_a_problem() {
    let dir = TempDir::new();
    let (settings, problem) = store(&dir).load();

    assert!(settings.locations.is_empty());
    assert_eq!(settings.last_location, 0);
    assert_eq!(problem, None);
}

#[test]
fn round_trips_locations_and_creates_the_folder() {
    let dir = TempDir::new();
    let mut settings = AppSettings::default();
    let mut home = place(
        "Peterborough",
        Some("Ontario"),
        "Canada",
        44.3,
        -78.32,
        Some("America/Toronto"),
    );
    home.nickname = Some("Home".to_string());
    settings.locations.push(SavedLocation::from_location(&home));
    settings.locations.push(SavedLocation::from_location(&place(
        "Reykjavik",
        None,
        "Iceland",
        64.15,
        -21.94,
        Some("Atlantic/Reykjavik"),
    )));
    settings.last_location = 1;

    store(&dir).save(&mut settings).unwrap();
    let (loaded, problem) = store(&dir).load();

    assert_eq!(problem, None);
    assert_eq!(loaded.locations.len(), 2);
    assert_eq!(loaded.locations[0].to_location().display_name(), "Home");
    assert_eq!(
        loaded.locations[0].to_location().full_name(),
        "Peterborough, Ontario, Canada"
    );
    assert_eq!(
        loaded.locations[1].to_location().full_name(),
        "Reykjavik, Iceland"
    );
    assert_eq!(loaded.last_location, 1);
    assert!(loaded.locations[0].notify_alerts);
    assert!(!dir.join("nested").join("settings.json.tmp").exists());
    assert_eq!(loaded, settings);
}

#[test]
fn corrupt_file_falls_back_to_defaults_and_reports_it() {
    let dir = TempDir::new();
    let store = store(&dir);
    write(&store, b"{ this is not json");

    let (mut settings, problem) = store.load();

    assert!(settings.locations.is_empty());
    let problem = problem.unwrap();
    assert!(!problem.error.is_empty());
    assert!(problem.message.starts_with("Weatherspell couldn't read its settings, so it has started without your saved locations. The file it couldn't read was kept as settings.json.bad in "), "{}", problem.message);
    // Kept, so the save that follows cannot lose what it held (#22).
    let bad = std::fs::read_to_string(store.bad_copy_path()).unwrap();
    assert_eq!(bad, "{ this is not json");
    store.save(&mut settings).unwrap();
    let bad = std::fs::read_to_string(store.bad_copy_path()).unwrap();
    assert_eq!(bad, "{ this is not json");
}

#[test]
fn reads_a_file_saved_with_a_byte_order_mark() {
    // Notepad and PowerShell's Set-Content both write UTF-8 with a BOM.
    let dir = TempDir::new();
    let store = store(&dir);
    write(&store, b"\xEF\xBB\xBF{\"version\":1,\"locations\":[{\"name\":\"Toronto\",\"latitude\":43.65,\"longitude\":-79.38}],\"lastLocation\":0}");

    let (settings, problem) = store.load();

    assert_eq!(problem, None);
    assert_eq!(settings.locations.len(), 1);
    assert_eq!(settings.locations[0].name, "Toronto");
}

#[test]
fn round_trips_alert_state_and_settings() {
    let dir = TempDir::new();
    let mut settings = AppSettings {
        forecast_refresh_minutes: 60,
        alert_check_minutes: 5,
        alert_announcements: Announcements::Severe,
        ..AppSettings::default()
    };
    let mut home = SavedLocation::from_location(&place(
        "Peterborough",
        Some("Ontario"),
        "Canada",
        44.3,
        -78.32,
        Some("America/Toronto"),
    ));
    home.seen_alert_ids
        .push("ec:64919237566271632202609120507".to_string());
    home.utc_offset_seconds = Some(-14400);
    home.notify_alerts = false;
    settings.locations.push(home);

    store(&dir).save(&mut settings).unwrap();
    let (loaded, _) = store(&dir).load();

    assert_eq!(loaded.forecast_refresh_minutes, 60);
    assert_eq!(loaded.alert_check_minutes, 5);
    assert_eq!(loaded.alert_announcements, Announcements::Severe);
    assert_eq!(
        loaded.locations[0].seen_alert_ids,
        ["ec:64919237566271632202609120507"]
    );
    assert_eq!(loaded.locations[0].utc_offset_seconds, Some(-14400));
    assert!(!loaded.locations[0].notify_alerts);
}

#[test]
fn older_files_and_odd_values_normalize_to_the_defaults() {
    let dir = TempDir::new();
    let store = store(&dir);
    write(&store, b"{\"version\":1,\"locations\":[{\"name\":\"X\",\"latitude\":1,\"longitude\":2}],\"lastLocation\":0,\"forecastRefreshMinutes\":5000,\"alertCheckMinutes\":0,\"alertAnnouncements\":\"loud\"}");

    let (settings, problem) = store.load();

    assert_eq!(problem, None);
    assert_eq!(settings.forecast_refresh_minutes, 30);
    assert_eq!(settings.alert_check_minutes, 10);
    assert_eq!(settings.alert_announcements, Announcements::All);
    assert!(settings.locations[0].seen_alert_ids.is_empty());
    assert_eq!(settings.locations[0].utc_offset_seconds, None);
    assert!(settings.locations[0].notify_alerts);
}

// What 0.1 forgave beyond its own tests: nulls where lists and names
// belong, and negative numbers.
#[test]
fn nulls_and_negatives_are_forgiven_as_0_1_forgives_them() {
    let dir = TempDir::new();
    let store = store(&dir);
    write(&store, b"{\"locations\":[null,{\"name\":null,\"latitude\":1,\"longitude\":2},{\"name\":\"Y\",\"latitude\":3,\"longitude\":4,\"seenAlertIds\":null,\"nickname\":null}],\"lastLocation\":-3,\"forecastRefreshMinutes\":-5,\"alertAnnouncements\":null,\"window\":null}");

    let (settings, problem) = store.load();

    assert_eq!(problem, None);
    assert_eq!(settings.locations.len(), 1);
    assert_eq!(settings.locations[0].name, "Y");
    assert!(settings.locations[0].seen_alert_ids.is_empty());
    assert_eq!(settings.last_location, 0);
    assert_eq!(settings.forecast_refresh_minutes, 30);
    assert_eq!(settings.alert_announcements, Announcements::All);
    assert_eq!(settings.window, None);

    write(&store, b"null");
    assert_eq!(store.load(), (AppSettings::default(), None));
}

#[test]
fn the_announcement_setting_is_a_severity_threshold() {
    let cases = [
        (Announcements::All, AlertSeverity::Minor, true),
        (Announcements::Severe, AlertSeverity::Moderate, false),
        (Announcements::Severe, AlertSeverity::Severe, true),
        (Announcements::Severe, AlertSeverity::Extreme, true),
        (Announcements::Off, AlertSeverity::Extreme, false),
    ];
    for (setting, severity, spoken) in cases {
        let settings = AppSettings {
            alert_announcements: setting,
            ..AppSettings::default()
        };
        assert_eq!(
            settings.announces(severity),
            spoken,
            "{setting:?} {severity:?}"
        );
    }
}

#[test]
fn a_file_from_before_the_refresh_interval_existed_gets_the_default() {
    let dir = TempDir::new();
    let store = store(&dir);
    write(&store, b"{\"version\":1,\"locations\":[],\"lastLocation\":0,\"alertCheckMinutes\":10,\"alertAnnouncements\":\"all\"}");

    assert_eq!(store.load().0.forecast_refresh_minutes, 30);
}

// The dialog's lists: the presets, plus a hand-edited value in its place
// so opening the dialog never silently changes it.
#[test]
fn interval_choices_keep_an_unlisted_value() {
    assert_eq!(
        choices::minutes(&choices::FORECAST_MINUTES, 30),
        [15, 30, 60, 120]
    );
    assert_eq!(
        choices::minutes(&choices::FORECAST_MINUTES, 45),
        [15, 30, 45, 60, 120]
    );
    assert_eq!(
        choices::minutes(&choices::ALERT_MINUTES, 240),
        [5, 10, 15, 30, 240]
    );
    assert_eq!(choices::minutes_label(15), "15 minutes");
    assert_eq!(choices::minutes_label(60), "1 hour");
    assert_eq!(choices::minutes_label(90), "1 hour and 30 minutes");
    assert_eq!(choices::minutes_label(120), "2 hours");
    assert_eq!(
        choices::announcement_label(Announcements::Severe),
        "Severe and extreme only"
    );
}

#[test]
fn out_of_range_last_location_is_clamped() {
    let dir = TempDir::new();
    let store = store(&dir);
    write(&store, b"{\"version\":1,\"locations\":[{\"name\":\"X\",\"latitude\":1,\"longitude\":2}],\"lastLocation\":7}");

    let (settings, _) = store.load();

    assert_eq!(settings.locations.len(), 1);
    assert_eq!(settings.last_location, 0);
}

fn saved_window(left: i32, top: i32, maximized: bool) -> SavedWindow {
    SavedWindow {
        left,
        top,
        width: 1080,
        height: 840,
        maximized,
        char_width: 10.0,
        char_height: 25.0,
    }
}

#[test]
fn the_window_round_trips_through_settings_and_a_bad_one_is_dropped() {
    let dir = TempDir::new();
    let store = SettingsStore::in_folder(&dir.0);
    assert_eq!(store.load().0.window, None);

    let mut settings = AppSettings {
        window: Some(saved_window(200, 100, true)),
        ..AppSettings::default()
    };
    store.save(&mut settings).unwrap();
    assert_eq!(store.load().0.window, Some(saved_window(200, 100, true)));

    write(
        &store,
        b"{\"window\":{\"left\":5,\"top\":5,\"width\":0,\"height\":400}}",
    );
    let (settings, problem) = store.load();
    assert_eq!(settings.window, None);
    assert_eq!(problem, None);
}

// The Manage Locations dialog's editor.

fn peterborough() -> Location {
    place(
        "Peterborough",
        Some("Ontario"),
        "Canada",
        44.30012,
        -78.31623,
        Some("America/Toronto"),
    )
}

fn albany() -> Location {
    place(
        "Albany",
        Some("New York"),
        "United States",
        42.65258,
        -73.75623,
        Some("America/New_York"),
    )
}

fn paris() -> Location {
    place(
        "Paris",
        Some("Ile-de-France"),
        "France",
        48.85341,
        2.3488,
        Some("Europe/Paris"),
    )
}

fn saved(places: &[Location]) -> Vec<SavedLocation> {
    places.iter().map(SavedLocation::from_location).collect()
}

fn names(editor: &LocationEditor) -> Vec<&str> {
    editor
        .entries()
        .iter()
        .map(|e| e.place.name.as_str())
        .collect()
}

#[test]
fn moves_stop_at_either_end() {
    let mut editor = LocationEditor::new(&saved(&[peterborough(), albany(), paris()]));

    assert!(editor.move_by(2, -1));
    assert_eq!(names(&editor), ["Peterborough", "Paris", "Albany"]);
    assert!(!editor.move_by(0, -1));
    assert!(!editor.move_by(2, 1));
    assert_eq!(names(&editor), ["Peterborough", "Paris", "Albany"]);
}

#[test]
fn a_place_already_in_the_list_is_not_added_twice() {
    let mut editor = LocationEditor::new(&saved(&[peterborough(), albany()]));

    let mut again = albany();
    again.latitude = 42.652581;
    assert_eq!(editor.add(again), (1, false));
    assert_eq!(editor.add(paris()), (2, true));
    assert!(editor.entries()[2].notify_alerts);
}

#[test]
fn the_list_names_a_nickname_with_the_full_name_and_says_when_alerts_are_not_spoken() {
    let mut editor = LocationEditor::new(&saved(&[peterborough()]));

    assert_eq!(
        editor.entries()[0].to_string(),
        "Peterborough, Ontario, Canada"
    );
    editor.rename(0, "  Home ");
    assert_eq!(
        editor.entries()[0].to_string(),
        "Home (Peterborough, Ontario, Canada)"
    );
    assert_eq!(editor.entries()[0].display_name(), "Home");
    editor.set_notify(0, false);
    assert_eq!(
        editor.entries()[0].to_string(),
        "Home (Peterborough, Ontario, Canada); no alert notifications"
    );
    editor.rename(0, " ");
    assert_eq!(editor.entries()[0].nickname, None);
    assert_eq!(
        editor.entries()[0].display_name(),
        "Peterborough, Ontario, Canada"
    );
}

// Seen alert ids and the UTC offset live on the saved location, so they
// must survive a move and a rename, including ids an alert check added
// while the dialog was open; nothing is written before commit.
#[test]
fn commit_keeps_the_saved_state_and_touches_nothing_before() {
    let mut saved = saved(&[peterborough(), albany()]);
    saved[1].seen_alert_ids.push("alert-1".to_string());
    let mut editor = LocationEditor::new(&saved);

    editor.move_by(1, -1);
    editor.rename(0, "Work");
    editor.set_notify(0, false);
    editor.remove(1);
    editor.add(paris());
    saved[1].seen_alert_ids.push("alert-2".to_string());
    assert_eq!(saved[1].nickname, None);
    assert!(saved[1].notify_alerts);

    let committed = editor.commit(&saved);

    assert_eq!(committed.len(), 2);
    assert_eq!(committed[0].name, "Albany");
    assert_eq!(committed[0].nickname.as_deref(), Some("Work"));
    assert!(!committed[0].notify_alerts);
    assert_eq!(committed[0].seen_alert_ids, ["alert-1", "alert-2"]);
    assert_eq!(committed[1].name, "Paris");
    assert!(committed[1].notify_alerts);
}

#[test]
fn the_location_on_screen_is_followed_or_replaced_by_the_one_in_its_place() {
    let saved = saved(&[peterborough(), albany(), paris()]);

    // Moved: still shown, at its new place.
    let mut editor = LocationEditor::new(&saved);
    editor.move_by(1, -1);
    assert_eq!(editor.shown(Some(1)), Some(0));
    // Removed: the one now where it stood.
    let mut editor = LocationEditor::new(&saved);
    editor.remove(1);
    assert_eq!(editor.shown(Some(1)), Some(1));
    // Removed from the end: the new last one.
    let mut editor = LocationEditor::new(&saved);
    editor.remove(2);
    assert_eq!(editor.shown(Some(2)), Some(1));
    // None left.
    let mut editor = LocationEditor::new(&saved[..1]);
    editor.remove(0);
    assert_eq!(editor.shown(Some(0)), None);
    // Nothing was on screen: the first.
    let editor = LocationEditor::new(&saved[..1]);
    assert_eq!(editor.shown(None), Some(0));
}
