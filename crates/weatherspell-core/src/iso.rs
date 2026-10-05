// Times in a cache file as 0.1 writes them (Iso in CacheFile.cs), so each
// app reads the other's files. A location's wall-clock time is
// "2026-09-11T23:45:00", no offset and no fraction: it never passes
// through this PC's zone, which would move it across a DST change. An
// instant is .NET's round-trip form, seven fractional digits and an
// offset; the port writes it in UTC ("+00:00"), which 0.1 reads as the
// same instant, and reads any offset back.

use jiff::Timestamp;
use jiff::civil::DateTime;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serializer};

const WALL: &str = "%Y-%m-%dT%H:%M:%S";

fn write_wall(t: DateTime) -> String {
    t.strftime(WALL).to_string()
}

fn read_wall<E: Error>(s: &str) -> Result<DateTime, E> {
    DateTime::strptime(WALL, s).map_err(E::custom)
}

fn write_stamp(t: Timestamp) -> String {
    format!(
        "{}.{:07}+00:00",
        t.strftime(WALL),
        t.subsec_nanosecond() / 100
    )
}

fn read_stamp<E: Error>(s: &str) -> Result<Timestamp, E> {
    s.parse().map_err(E::custom)
}

pub(crate) mod wall {
    use super::*;

    pub fn serialize<S: Serializer>(t: &DateTime, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&write_wall(*t))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<DateTime, D::Error> {
        read_wall(&String::deserialize(d)?)
    }
}

pub(crate) mod wall_opt {
    use super::*;

    pub fn serialize<S: Serializer>(t: &Option<DateTime>, s: S) -> Result<S::Ok, S::Error> {
        match t {
            Some(t) => s.serialize_str(&write_wall(*t)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<DateTime>, D::Error> {
        Option::<String>::deserialize(d)?
            .map(|s| read_wall(&s))
            .transpose()
    }
}

// A day's date, which 0.1 keeps as a DateTime at midnight.
pub(crate) mod date {
    use super::*;
    use jiff::civil::Date;

    pub fn serialize<S: Serializer>(d: &Date, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&write_wall(d.to_datetime(jiff::civil::Time::midnight())))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Date, D::Error> {
        Ok(read_wall::<D::Error>(&String::deserialize(d)?)?.date())
    }
}

pub(crate) mod stamp {
    use super::*;

    pub fn serialize<S: Serializer>(t: &Timestamp, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&write_stamp(*t))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Timestamp, D::Error> {
        read_stamp(&String::deserialize(d)?)
    }
}

pub(crate) mod stamp_opt {
    use super::*;

    pub fn serialize<S: Serializer>(t: &Option<Timestamp>, s: S) -> Result<S::Ok, S::Error> {
        match t {
            Some(t) => s.serialize_str(&write_stamp(*t)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Timestamp>, D::Error> {
        Option::<String>::deserialize(d)?
            .map(|s| read_stamp(&s))
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instants_are_written_as_dotnet_round_trip_text_in_utc() {
        let t: Timestamp = "2026-09-11T23:58:00.1234567-04:00".parse().unwrap();
        assert_eq!(write_stamp(t), "2026-09-12T03:58:00.1234567+00:00");
        assert_eq!(read_stamp::<serde_json::Error>(&write_stamp(t)).unwrap(), t);
    }

    #[test]
    fn wall_times_have_no_offset_or_fraction() {
        let t = jiff::civil::date(2026, 3, 8).at(2, 30, 0, 0);
        assert_eq!(write_wall(t), "2026-03-08T02:30:00");
        assert_eq!(
            read_wall::<serde_json::Error>("2026-03-08T02:30:00").unwrap(),
            t
        );
        assert!(read_wall::<serde_json::Error>("2026-03-08T02:30:00-05:00").is_err());
    }
}
