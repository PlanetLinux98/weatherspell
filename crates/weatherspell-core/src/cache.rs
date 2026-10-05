// One file per saved location in the cache folder beside settings.json:
// the last forecast and alerts fetched for it, so a launch or a switch
// that cannot reach the weather service still has something to read.
// They are 0.1's files (ForecastCache.cs, CacheFile.cs), named and shaped
// the same, so each app reads the other's. Overwritten on every
// successful fetch and never appended to, so the cache is the size of the
// saved locations and no more; files for locations no longer saved are
// pruned at startup. Read only after a fetch has failed: showing it first
// and swapping in the live text would replace the words under a reader
// (see NOTES.md).

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use jiff::tz::Offset;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::alerts::{AlertReport, WeatherAlert};
use crate::files;
use crate::forecast::{CurrentConditions, DayForecast, Forecast, HourPoint, OfficialPeriod};
use crate::location::Location;
use crate::units::UnitSystem;

pub const FOLDER_NAME: &str = "cache";

const VERSION: i32 = 1;

// The last forecast and alerts read from a location's cache file.
#[derive(Clone, Debug, PartialEq)]
pub struct CachedForecast {
    pub forecast: Forecast,
    pub alerts: Option<AlertReport>,
}

pub struct ForecastCache {
    folder: PathBuf,
}

// A file's two halves are kept as they were read, so saving a forecast
// leaves the alerts beside it untouched, as 0.1 does.
#[derive(Serialize, Deserialize)]
struct CacheFile {
    version: i32,
    #[serde(default)]
    forecast: Option<Value>,
    // The last alert check that succeeded, or the not-available report for
    // a region no source covers; never a failed check.
    #[serde(default)]
    alerts: Option<Value>,
}

// The location is not stored: the file is keyed by coordinates and read
// back for the saved location that asked.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForecastData {
    #[serde(with = "crate::iso::stamp")]
    fetched_at: Timestamp,
    utc_offset_seconds: i32,
    units: Option<String>,
    current: CurrentConditions,
    hours: Vec<HourPoint>,
    days: Vec<DayForecast>,
    periods: Vec<OfficialPeriod>,
    source_name: String,
    sources: Vec<String>,
}

impl ForecastData {
    fn from(f: &Forecast) -> ForecastData {
        ForecastData {
            fetched_at: f.fetched_at,
            utc_offset_seconds: f.utc_offset.seconds(),
            units: Some(
                match f.units {
                    UnitSystem::Imperial => "imperial",
                    UnitSystem::Metric => "metric",
                }
                .to_string(),
            ),
            current: f.current.clone(),
            hours: f.hours.clone(),
            days: f.days.clone(),
            periods: f.periods.clone(),
            source_name: f.source_name.clone(),
            sources: f.sources.clone(),
        }
    }

    fn to(self, location: &Location) -> Option<Forecast> {
        // Beyond 14 hours no place's offset reaches; 0.1 refuses one too,
        // since its writer would fail on it later (#22).
        if self.utc_offset_seconds.abs() > 14 * 3600 {
            return None;
        }
        Some(Forecast {
            location: location.clone(),
            fetched_at: self.fetched_at,
            utc_offset: Offset::from_seconds(self.utc_offset_seconds).ok()?,
            units: match self.units.as_deref() {
                Some("imperial") => UnitSystem::Imperial,
                _ => UnitSystem::Metric,
            },
            current: self.current,
            hours: self.hours,
            days: self.days,
            periods: self.periods,
            source_name: self.source_name,
            sources: self.sources,
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AlertsData {
    attribution: Option<String>,
    #[serde(default, with = "crate::iso::stamp_opt")]
    checked_at: Option<Timestamp>,
    alerts: Option<Vec<WeatherAlert>>,
}

impl AlertsData {
    fn from(r: &AlertReport) -> AlertsData {
        AlertsData {
            attribution: r.attribution.clone(),
            checked_at: r.checked_at,
            alerts: Some(r.alerts.clone()),
        }
    }

    fn to(self) -> Option<AlertReport> {
        match self.attribution {
            None => Some(AlertReport::not_available()),
            Some(attribution) => Some(AlertReport {
                alerts: self.alerts?,
                attribution: Some(attribution),
                problem: None,
                checked_at: self.checked_at,
            }),
        }
    }
}

impl ForecastCache {
    pub fn new(folder: impl Into<PathBuf>) -> ForecastCache {
        ForecastCache {
            folder: folder.into(),
        }
    }

    // In the app's folder, beside settings.json, as 0.1 keeps it.
    pub fn in_folder(app_folder: &Path) -> ForecastCache {
        ForecastCache::new(app_folder.join(FOLDER_NAME))
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    // Coordinates, not the name: a nickname or a rename keeps the file, and
    // nothing from a geocoder's name has to be made safe for a filename.
    pub fn file_name(l: &Location) -> String {
        format!("{}_{}.json", f4(l.latitude), f4(l.longitude))
    }

    fn path_for(&self, l: &Location) -> PathBuf {
        self.folder.join(ForecastCache::file_name(l))
    }

    // None when there is no file or it cannot be read, whatever is wrong
    // with it: the next successful fetch replaces a bad one, so nothing is
    // reported, and the cache is only ever a convenience.
    pub fn load(&self, location: &Location) -> Option<CachedForecast> {
        let file = read(&self.path_for(location))?;
        let forecast: ForecastData = serde_json::from_value(file.forecast?).ok()?;
        let alerts = match file.alerts {
            Some(v) => Some(serde_json::from_value::<AlertsData>(v).ok()?.to()?),
            None => None,
        };
        Some(CachedForecast {
            forecast: forecast.to(location)?,
            alerts,
        })
    }

    // alerts is None when this fetch's check failed: the file keeps the
    // alerts from the last check that succeeded, still dated by it.
    pub fn save(
        &self,
        location: &Location,
        forecast: &Forecast,
        alerts: Option<&AlertReport>,
    ) -> io::Result<()> {
        let path = self.path_for(location);
        let mut file = read(&path).unwrap_or(CacheFile {
            version: VERSION,
            forecast: None,
            alerts: None,
        });
        file.forecast = Some(serde_json::to_value(ForecastData::from(forecast))?);
        if let Some(alerts) = alerts {
            file.alerts = Some(serde_json::to_value(AlertsData::from(alerts))?);
        }
        write(&path, &file)
    }

    // The alert poll's result for a location that has a file; one without
    // a forecast would have nothing to show, so none is started here.
    pub fn save_alerts(&self, location: &Location, alerts: &AlertReport) -> io::Result<()> {
        let path = self.path_for(location);
        let Some(mut file) = read(&path).filter(|f| f.forecast.is_some()) else {
            return Ok(());
        };
        file.alerts = Some(serde_json::to_value(AlertsData::from(alerts))?);
        write(&path, &file)
    }

    // Every .json file but those of the locations to keep; anything else
    // in the folder is not the cache's.
    pub fn prune<'a>(&self, keep: impl IntoIterator<Item = &'a Location>) -> io::Result<()> {
        let names: HashSet<String> = keep
            .into_iter()
            .map(|l| ForecastCache::file_name(l).to_lowercase())
            .collect();
        let entries = match fs::read_dir(&self.folder) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            entries => entries?,
        };
        for entry in entries {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.ends_with(".json") && entry.file_type()?.is_file() && !names.contains(&name) {
                fs::remove_file(entry.path())?;
            }
        }
        Ok(())
    }
}

fn read(path: &Path) -> Option<CacheFile> {
    let text = files::read_text(path).ok()?;
    serde_json::from_str::<CacheFile>(&text)
        .ok()
        .filter(|f| f.version == VERSION)
}

fn write(path: &Path, file: &CacheFile) -> io::Result<()> {
    files::replace(path, serde_json::to_string(file)?.as_bytes())
}

// 0.1 names its files with .NET Framework's "F4", which takes the value
// to 15 significant digits first and then rounds half away from zero:
// 43.70005 is "43.7001" there, though the double is a shade under it,
// and a value that rounds to zero has no minus sign. The port names the
// file the same, or it would miss 0.1's cache and prune it.
fn f4(value: f64) -> String {
    const DECIMALS: i32 = 4;
    let scientific = format!("{:.14e}", value.abs());
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let digits: u128 = mantissa.replace('.', "").parse().unwrap_or(0);
    // digits is the value times 10^(14 - exponent); the result wants it
    // times 10^DECIMALS.
    let shift = exponent.parse::<i32>().unwrap_or(0) - 14 + DECIMALS;
    let scaled = if shift >= 0 {
        digits * 10u128.pow(shift as u32)
    } else if -shift > 16 {
        0
    } else {
        let drop = 10u128.pow(-shift as u32);
        let first_dropped = digits / (drop / 10) % 10;
        digits / drop + u128::from(first_dropped >= 5)
    };
    let scale = 10u128.pow(DECIMALS as u32);
    let sign = if value < 0.0 && scaled != 0 { "-" } else { "" };
    format!(
        "{sign}{}.{:0width$}",
        scaled / scale,
        scaled % scale,
        width = DECIMALS as usize
    )
}

#[cfg(test)]
mod tests {
    use super::f4;

    #[test]
    fn file_names_round_as_dotnet_framework_does() {
        assert_eq!(f4(44.3), "44.3000");
        assert_eq!(f4(-78.33), "-78.3300");
        assert_eq!(f4(43.70005), "43.7001");
        assert_eq!(f4(-12.34565), "-12.3457");
        assert_eq!(f4(-0.00004), "0.0000");
        assert_eq!(f4(-0.00005), "-0.0001");
        assert_eq!(f4(0.0), "0.0000");
        assert_eq!(f4(-180.0), "-180.0000");
        assert_eq!(f4(1e-20), "0.0000");
    }
}
