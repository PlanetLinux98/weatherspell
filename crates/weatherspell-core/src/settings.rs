// What persists between runs: settings.json in the folder the app passes
// in (%APPDATA%\Weatherspell on Windows), the same file 0.1 reads and
// writes (AppSettings.cs, SettingsStore.cs), so moving between the two
// keeps the saved locations, nicknames, notify switches and seen alerts.
// Reading forgives what 0.1's does: a member the file lacks takes its
// default, a null where a list belongs is an empty list, and odd values
// are put right by normalize. A file that cannot be read at all is kept
// as settings.json.bad, since the next save replaces it and the
// locations in it would otherwise be lost for good (#22).

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::alerts::AlertSeverity;
use crate::files;
use crate::location::Location;

pub const FILE_NAME: &str = "settings.json";

// A day: a longer interval is a typo.
pub const MAX_MINUTES: u32 = 24 * 60;
pub const DEFAULT_FORECAST_MINUTES: u32 = 30;
pub const DEFAULT_ALERT_MINUTES: u32 = 10;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub version: i32,
    #[serde(deserialize_with = "present")]
    pub locations: Vec<SavedLocation>,
    #[serde(deserialize_with = "whole")]
    pub last_location: usize,
    // Minutes between automatic refreshes of the forecast on screen and
    // between checks of every saved location's alerts. Any positive value
    // up to a day is honoured; the Settings dialog offers a short list.
    #[serde(deserialize_with = "whole")]
    pub forecast_refresh_minutes: u32,
    #[serde(deserialize_with = "whole")]
    pub alert_check_minutes: u32,
    pub alert_announcements: Announcements,
    // Where the main window was when it last closed; None until then.
    pub window: Option<SavedWindow>,
}

impl Default for AppSettings {
    fn default() -> AppSettings {
        AppSettings {
            version: 1,
            locations: Vec::new(),
            last_location: 0,
            forecast_refresh_minutes: DEFAULT_FORECAST_MINUTES,
            alert_check_minutes: DEFAULT_ALERT_MINUTES,
            alert_announcements: Announcements::All,
            window: None,
        }
    }
}

impl AppSettings {
    pub fn normalize(&mut self) {
        self.locations.retain(|l| !l.name.trim().is_empty());
        if self.last_location >= self.locations.len() {
            self.last_location = 0;
        }
        if !(1..=MAX_MINUTES).contains(&self.forecast_refresh_minutes) {
            self.forecast_refresh_minutes = DEFAULT_FORECAST_MINUTES;
        }
        if !(1..=MAX_MINUTES).contains(&self.alert_check_minutes) {
            self.alert_check_minutes = DEFAULT_ALERT_MINUTES;
        }
        if self.window.is_some_and(|w| w.width <= 0 || w.height <= 0) {
            self.window = None;
        }
    }

    // The saved entry for the same place.
    pub fn index_of(&self, place: &Location) -> Option<usize> {
        self.locations
            .iter()
            .position(|s| place.is_same_place(&s.to_location()))
    }

    pub fn announces(&self, severity: AlertSeverity) -> bool {
        match self.alert_announcements {
            Announcements::All => true,
            Announcements::Severe => severity >= AlertSeverity::Severe,
            Announcements::Off => false,
        }
    }
}

// Which new alerts are spoken: all, severe and extreme only, or none.
// In the file as 0.1 writes it ("all", "severe", "off"); anything else
// reads as all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Announcements {
    #[default]
    All,
    Severe,
    Off,
}

impl Announcements {
    fn name(self) -> &'static str {
        match self {
            Announcements::All => "all",
            Announcements::Severe => "severe",
            Announcements::Off => "off",
        }
    }
}

impl Serialize for Announcements {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for Announcements {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Announcements, D::Error> {
        Ok(match Option::<String>::deserialize(d)?.as_deref() {
            Some("severe") => Announcements::Severe,
            Some("off") => Announcements::Off,
            _ => Announcements::All,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SavedLocation {
    #[serde(deserialize_with = "or_default")]
    pub name: String,
    pub region: Option<String>,
    pub country: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub time_zone_id: Option<String>,
    pub nickname: Option<String>,
    pub notify_alerts: bool,
    // Ids of the alerts in effect at the last check, so each is announced
    // once, across runs (alerts::track).
    #[serde(deserialize_with = "present")]
    pub seen_alert_ids: Vec<String>,
    // The location's UTC offset from its last forecast, so an alert for a
    // location not on screen can be announced in its own time.
    pub utc_offset_seconds: Option<i32>,
}

impl Default for SavedLocation {
    fn default() -> SavedLocation {
        SavedLocation {
            name: String::new(),
            region: None,
            country: None,
            latitude: 0.0,
            longitude: 0.0,
            time_zone_id: None,
            nickname: None,
            notify_alerts: true,
            seen_alert_ids: Vec::new(),
            utc_offset_seconds: None,
        }
    }
}

impl SavedLocation {
    pub fn from_location(l: &Location) -> SavedLocation {
        SavedLocation {
            name: l.name.clone(),
            region: l.region.clone(),
            country: l.country.clone(),
            latitude: l.latitude,
            longitude: l.longitude,
            time_zone_id: l.time_zone_id.clone(),
            nickname: l.nickname.clone(),
            ..SavedLocation::default()
        }
    }

    pub fn to_location(&self) -> Location {
        Location {
            name: self.name.clone(),
            region: self.region.clone(),
            country: self.country.clone(),
            latitude: self.latitude,
            longitude: self.longitude,
            time_zone_id: self.time_zone_id.clone(),
            nickname: self.nickname.clone(),
            population: None,
        }
    }
}

// The window's normal (not maximized) bounds in screen pixels, whether it
// was maximized, and the average character size of its font in pixels (7
// by 15 for Segoe UI 9 pt at 96 DPI), so a size saved before the display
// scale or Text size changed can follow it (placement.rs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SavedWindow {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub maximized: bool,
    pub char_width: f64,
    pub char_height: f64,
}

// A list with its nulls left out, and a null list as an empty one.
fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Vec<T>, D::Error> {
    Ok(Option::<Vec<Option<T>>>::deserialize(d)?
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .collect())
}

fn or_default<'de, D: Deserializer<'de>, T: Deserialize<'de> + Default>(
    d: D,
) -> Result<T, D::Error> {
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

// A count or an index; a negative one (hand-edited) reads as 0, which
// normalize then replaces with the default, as 0.1 does.
fn whole<'de, D: Deserializer<'de>, T: TryFrom<i64> + Default>(d: D) -> Result<T, D::Error> {
    Ok(Option::<i64>::deserialize(d)?
        .and_then(|v| T::try_from(v).ok())
        .unwrap_or_default())
}

pub struct SettingsStore {
    path: PathBuf,
}

// Why the settings were not read, so the app can say so once: the message
// in plain words, and the reader's own error beside it.
#[derive(Clone, Debug, PartialEq)]
pub struct LoadProblem {
    pub message: String,
    pub error: String,
}

impl SettingsStore {
    pub fn new(path: impl Into<PathBuf>) -> SettingsStore {
        SettingsStore { path: path.into() }
    }

    // settings.json in the app's folder, as 0.1 keeps it.
    pub fn in_folder(folder: &Path) -> SettingsStore {
        SettingsStore::new(folder.join(FILE_NAME))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn bad_copy_path(&self) -> PathBuf {
        files::with_suffix(&self.path, ".bad")
    }

    // Defaults when there is no file yet, without a problem; defaults and
    // the problem when there is one that cannot be read.
    pub fn load(&self) -> (AppSettings, Option<LoadProblem>) {
        let text = match files::read_text(&self.path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return (AppSettings::default(), None),
            Err(e) => return (AppSettings::default(), Some(self.problem(e.to_string()))),
        };
        match serde_json::from_str::<Option<AppSettings>>(&text) {
            Ok(settings) => {
                let mut settings = settings.unwrap_or_default();
                settings.normalize();
                (settings, None)
            }
            Err(e) => (AppSettings::default(), Some(self.problem(e.to_string()))),
        }
    }

    fn problem(&self, error: String) -> LoadProblem {
        let folder = self.path.parent().unwrap_or(Path::new("")).display();
        let start = "Weatherspell couldn't read its settings, so it has started without your saved locations.";
        let bad = self.bad_copy_path();
        let message = match std::fs::copy(&self.path, &bad) {
            Ok(_) => format!(
                "{start} The file it couldn't read was kept as {} in {folder}.",
                files::name(&bad)
            ),
            Err(_) => format!(
                "{start} The file is {} in {folder}.",
                files::name(&self.path)
            ),
        };
        LoadProblem { message, error }
    }

    // Normalized first, as it will be read back; written to a temporary
    // file and swapped in, so a crash mid-write never leaves half a file.
    pub fn save(&self, settings: &mut AppSettings) -> io::Result<()> {
        settings.normalize();
        let json = serde_json::to_string_pretty(settings)?;
        files::replace(&self.path, json.as_bytes())
    }
}

// What the Settings dialog offers for each field, kept out of the dialog
// so the lists and their wording are testable. The intervals are short
// lists in a choice control rather than a number field: the control
// screen readers read best, and nothing to mistype. A value outside the
// list (a hand-edited settings.json) is kept and shown as its own entry,
// in order, so opening the dialog never changes it.
pub mod choices {
    use super::Announcements;

    pub const FORECAST_MINUTES: [u32; 4] = [15, 30, 60, 120];
    pub const ALERT_MINUTES: [u32; 4] = [5, 10, 15, 30];

    pub const ANNOUNCEMENTS: [(Announcements, &str); 3] = [
        (Announcements::All, "All new alerts"),
        (Announcements::Severe, "Severe and extreme only"),
        (Announcements::Off, "Off"),
    ];

    pub fn minutes(presets: &[u32], current: u32) -> Vec<u32> {
        let mut list = presets.to_vec();
        if current >= 1 && !list.contains(&current) {
            list.push(current);
            list.sort_unstable();
        }
        list
    }

    // "15 minutes", "1 hour", "1 hour and 30 minutes", "2 hours".
    pub fn minutes_label(minutes: u32) -> String {
        crate::clock::duration(f64::from(minutes) * 60.0)
    }

    pub fn announcement_label(value: Announcements) -> &'static str {
        ANNOUNCEMENTS
            .iter()
            .find(|(v, _)| *v == value)
            .map_or(ANNOUNCEMENTS[0].1, |(_, label)| label)
    }
}
