// Time words. Forecast times are the location's own; when this PC's clock
// differs, the PC time follows in brackets so a faraway sunrise still
// makes sense to the reader.
//
// Day and month names are English, as the app is: the C# app took them
// from the region's culture, so a French region put "samedi" into English
// sentences. The time itself follows the PC's own format (TimeFormat).

use jiff::civil::{Date, DateTime, Weekday};
use jiff::tz::{Offset, TimeZone};
use jiff::{SignedDuration, Timestamp};

// How this PC writes a time: Windows' short time pattern ("h:mm tt") and
// its designators ("AM", or "a.m." in English (Canada)), which the app
// reads from the system. The letters are .NET's and Windows' own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeFormat {
    pub pattern: String,
    pub am: String,
    pub pm: String,
}

impl TimeFormat {
    pub fn new(pattern: &str, am: &str, pm: &str) -> TimeFormat {
        TimeFormat {
            pattern: pattern.to_string(),
            am: am.to_string(),
            pm: pm.to_string(),
        }
    }

    // "2:45 pm": the designator lowercased so it reads as a word, not
    // initials.
    pub fn format(&self, t: DateTime) -> String {
        let mut s = self.format_raw(t);
        for designator in [&self.am, &self.pm] {
            if !designator.is_empty() {
                s = s.replace(designator.as_str(), &designator.to_lowercase());
            }
        }
        s
    }

    // This PC's clock with the day when it is not today ("10:43 pm
    // yesterday", "10:43 pm on September 15"): a cached forecast days old
    // was spoken as "from 10:43 pm" (#17).
    pub fn format_on_day(&self, t: DateTime, today: Date) -> String {
        let time = self.format(t);
        if t.date() == today {
            time
        } else {
            format!("{time} {}", day_word(t.date(), today))
        }
    }

    fn format_raw(&self, t: DateTime) -> String {
        let hour = t.hour() as i32;
        let twelve = if hour % 12 == 0 { 12 } else { hour % 12 };
        let designator = if hour < 12 { &self.am } else { &self.pm };
        let chars: Vec<char> = self.pattern.chars().collect();
        let mut out = String::new();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == '\'' || c == '"' {
                // A quoted literal, up to the matching quote.
                i += 1;
                while i < chars.len() && chars[i] != c {
                    out.push(chars[i]);
                    i += 1;
                }
                i += 1;
                continue;
            }
            if c == '\\' && i + 1 < chars.len() {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            let mut run = 1;
            while i + run < chars.len() && chars[i + run] == c {
                run += 1;
            }
            match c {
                'h' => out.push_str(&pad(twelve, run)),
                'H' => out.push_str(&pad(hour, run)),
                'm' => out.push_str(&pad(t.minute() as i32, run)),
                's' => out.push_str(&pad(t.second() as i32, run)),
                't' if run == 1 => out.extend(designator.chars().next()),
                't' => out.push_str(designator),
                _ => (0..run).for_each(|_| out.push(c)),
            }
            i += run;
        }
        out
    }
}

fn pad(value: i32, run: usize) -> String {
    if run >= 2 {
        format!("{value:02}")
    } else {
        value.to_string()
    }
}

pub struct Clock {
    location_offset: Offset,
    pc_zone: TimeZone,
    format: TimeFormat,
}

impl Clock {
    pub fn new(location_offset: Offset, pc_zone: TimeZone, format: TimeFormat) -> Clock {
        Clock {
            location_offset,
            pc_zone,
            format,
        }
    }

    // "6:40 am (9:40 am your time)". The bracket names the reader's own
    // day ("today", "tomorrow", "Saturday", as alert times do) unless both
    // times fall on the reader's today: it used to compare with the
    // location's date, so a Kathmandu sunrise due that evening read "8:08
    // pm yesterday your time" (#21).
    pub fn time(&self, local: DateTime, now_local: DateTime) -> String {
        let text = self.format.format(local);
        let utc = self.to_utc(local);
        let pc_offset = self.pc_zone.to_offset(utc);
        if pc_offset == self.location_offset {
            return text;
        }
        let pc_local = pc_offset.to_datetime(utc);
        let now_utc = self.to_utc(now_local);
        let pc_today = self.pc_zone.to_offset(now_utc).to_datetime(now_utc).date();
        let day = if pc_local.date() == pc_today && local.date() == pc_today {
            String::new()
        } else {
            format!(" {}", day_word(pc_local.date(), pc_today))
        };
        format!("{text} ({}{day} your time)", self.format.format(pc_local))
    }

    // "4:00 pm today", "6:30 am tomorrow", "6:30 am Sunday", "6:30 am on
    // September 20": when an alert ends, relative to the location's own
    // day, and in brackets relative to this PC's day when the zones differ.
    pub fn time_on_day(&self, local: DateTime, now_local: DateTime) -> String {
        let text = format!(
            "{} {}",
            self.format.format(local),
            day_word(local.date(), now_local.date())
        );
        let utc = self.to_utc(local);
        let pc_offset = self.pc_zone.to_offset(utc);
        if pc_offset == self.location_offset {
            return text;
        }
        let pc_local = pc_offset.to_datetime(utc);
        // The offset at the time named, as the C# had it, also for "now".
        let pc_now = pc_offset.to_datetime(self.to_utc(now_local));
        format!(
            "{text} ({} {} your time)",
            self.format.format(pc_local),
            day_word(pc_local.date(), pc_now.date())
        )
    }

    // A service's timestamp as the location's own wall-clock time.
    pub fn local(&self, time: Timestamp) -> DateTime {
        self.location_offset.to_datetime(time)
    }

    fn to_utc(&self, local: DateTime) -> Timestamp {
        self.location_offset
            .to_timestamp(local)
            .expect("a forecast time within jiff's range")
    }
}

pub fn day_word(date: Date, today: Date) -> String {
    match days_between(today, date) {
        0 => "today".to_string(),
        1 => "tomorrow".to_string(),
        -1 => "yesterday".to_string(),
        2..=6 => weekday_name(date).to_string(),
        _ => format!("on {} {}", month_name(date), date.day()),
    }
}

pub fn days_between(from: Date, to: Date) -> i64 {
    from.until(to)
        .map(|span| span.get_days() as i64)
        .unwrap_or(0)
}

// "Saturday, September 12".
pub fn day_heading(date: Date) -> String {
    format!(
        "{}, {} {}",
        weekday_name(date),
        month_name(date),
        date.day()
    )
}

pub fn weekday_name(date: Date) -> &'static str {
    match date.weekday() {
        Weekday::Monday => "Monday",
        Weekday::Tuesday => "Tuesday",
        Weekday::Wednesday => "Wednesday",
        Weekday::Thursday => "Thursday",
        Weekday::Friday => "Friday",
        Weekday::Saturday => "Saturday",
        Weekday::Sunday => "Sunday",
    }
}

pub fn month_name(date: Date) -> &'static str {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    MONTHS[date.month() as usize - 1]
}

pub fn duration(seconds: f64) -> String {
    let total = crate::round_even(seconds / 60.0);
    let hours = total / 60;
    let minutes = total % 60;
    let h = if hours == 1 {
        "1 hour".to_string()
    } else {
        format!("{hours} hours")
    };
    let m = if minutes == 1 {
        "1 minute".to_string()
    } else {
        format!("{minutes} minutes")
    };
    if hours == 0 {
        m
    } else if minutes == 0 {
        h
    } else {
        format!("{h} and {m}")
    }
}

pub fn age(age: SignedDuration) -> String {
    let seconds = age.as_secs();
    if age < SignedDuration::from_mins(1) {
        return "just now".to_string();
    }
    if age < SignedDuration::from_hours(1) {
        let m = seconds / 60;
        return if m == 1 {
            "1 minute ago".to_string()
        } else {
            format!("{m} minutes ago")
        };
    }
    if age < SignedDuration::from_hours(24) {
        let h = seconds / 3600;
        return if h == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{h} hours ago")
        };
    }
    let d = seconds / 86400;
    if d == 1 {
        "1 day ago".to_string()
    } else {
        format!("{d} days ago")
    }
}

// After "from": "just now" would not follow it.
pub fn age_after_from(age_of: SignedDuration) -> String {
    if age_of < SignedDuration::from_mins(1) {
        "less than a minute ago".to_string()
    } else {
        age(age_of)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    #[test]
    fn patterns_follow_windows_letters() {
        let t = date(2026, 9, 11).at(14, 5, 0, 0);
        assert_eq!(TimeFormat::new("h:mm tt", "AM", "PM").format(t), "2:05 pm");
        assert_eq!(
            TimeFormat::new("h:mm tt", "a.m.", "p.m.").format(t),
            "2:05 p.m."
        );
        assert_eq!(TimeFormat::new("HH:mm", "", "").format(t), "14:05");
        assert_eq!(TimeFormat::new("H'h'mm", "", "").format(t), "14h05");
        assert_eq!(
            TimeFormat::new("hh:mm t", "AM", "PM").format(date(2026, 9, 11).at(0, 30, 0, 0)),
            "12:30 A"
        );
    }
}
