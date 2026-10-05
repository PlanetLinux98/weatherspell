// Sample forecasts in the C# app's shape (SectionLayout.cs): a heading
// line, a blank line, paragraphs separated by blank lines. The words are
// made up; what matters is the rhythm, the headings, and an Alerts section
// that grows on a refresh so the caret has something to survive.

pub const PLACES: [&str; 3] = [
    "Peterborough, Ontario, Canada",
    "Buffalo, New York, United States",
    "Paris, \u{ce}le-de-France, France",
];

pub struct Layout {
    pub text: String,
    // Heading text and its offset in UTF-16 units, as wx counts positions
    // in a RichEdit ("\n" is one).
    pub headings: Vec<(String, i64)>,
}

impl Layout {
    fn build(sections: &[(&str, Vec<String>)]) -> Layout {
        let mut text = String::new();
        let mut headings = Vec::new();
        for (heading, paragraphs) in sections {
            if !text.is_empty() {
                text.push('\n');
            }
            headings.push((heading.to_string(), units(&text)));
            text.push_str(heading);
            text.push_str("\n\n");
            for paragraph in paragraphs {
                text.push_str(paragraph);
                text.push_str("\n\n");
            }
        }
        Layout { text, headings }
    }

    pub fn fetching(place: &str) -> Layout {
        Layout {
            text: format!("Fetching the forecast for {}...", short(place)),
            headings: Vec::new(),
        }
    }

    // The heading at or before the offset, if any.
    pub fn section_at(&self, offset: i64) -> Option<usize> {
        self.headings.iter().rposition(|(_, at)| *at <= offset)
    }

    // SectionLayout.MapCaret: the same distance into the section with the
    // same heading; the heading itself when the section got too short.
    pub fn map_caret(&self, from: &Layout, caret: i64) -> i64 {
        let length = units(&self.text);
        let Some(i) = from.section_at(caret) else {
            return caret.clamp(0, length);
        };
        if self.headings.is_empty() {
            return caret.clamp(0, length);
        }
        let j = self
            .headings
            .iter()
            .position(|(heading, _)| *heading == from.headings[i].0)
            .unwrap_or(i.min(self.headings.len() - 1));
        let start = self.headings[j].1;
        let end = self.headings.get(j + 1).map_or(length, |(_, at)| *at);
        let distance = caret - from.headings[i].1;
        if distance < end - start {
            start + distance
        } else {
            start
        }
    }
}

pub fn units(text: &str) -> i64 {
    text.encode_utf16().count() as i64
}

pub fn short(place: &str) -> &str {
    place.split(',').next().unwrap_or(place)
}

// The alert names that "forecast ready" mentions, as the C# app does.
pub fn alerts_in_effect(place: usize, refresh: u32) -> Vec<&'static str> {
    match place {
        0 if refresh > 0 => vec!["special weather statement", "frost advisory"],
        0 => vec!["special weather statement"],
        1 => vec!["lake effect snow warning"],
        _ => Vec::new(),
    }
}

// The forecast for a place after `refresh` refreshes: each one moves the
// observation on, and Peterborough gains an alert, which pushes every
// later section down.
pub fn forecast(place: usize, refresh: u32) -> Layout {
    let minutes = 42 + 5 * refresh;
    let time = format!("{}:{:02} pm", 3 + minutes / 60, minutes % 60);
    let temperature = 12 + (refresh % 3) as i32;
    match place {
        0 => {
            let mut alerts = vec![
                "Special weather statement in effect until 6:00 am tomorrow, from Environment and Climate Change Canada. Press Enter for details.".to_string(),
            ];
            if refresh > 0 {
                alerts.insert(
                    0,
                    "Frost advisory in effect from 11:00 pm tonight until 9:00 am tomorrow, from Environment and Climate Change Canada. Press Enter for details.".to_string(),
                );
            }
            Layout::build(&[
                ("Alerts", alerts),
                ("Right now", vec![
                    format!("Mostly cloudy, {temperature} degrees, wind from the southwest at 15 kilometres an hour. Observed at Peterborough Airport at {time}."),
                    "Humidity 64 percent, dew point 5 degrees, pressure 101.4 kilopascals and rising, visibility 24 kilometres.".to_string(),
                ]),
                ("Rest of today", vec![
                    "This evening and tonight: Cloudy with 40 percent chance of showers. Wind southwest 20 kilometres an hour becoming light this evening. Low 4.".to_string(),
                    "The sun sets at 6:48 pm and rises at 7:15 am tomorrow.".to_string(),
                ]),
                ("Monday", vec![
                    "A mix of sun and cloud. Wind northwest 20 kilometres an hour gusting to 40. High 11. UV index 3 or moderate.".to_string(),
                ]),
                ("Monday night", vec!["Clear. Low minus 2, with frost.".to_string()]),
                ("Tuesday", vec!["Sunny. High 13.".to_string()]),
                ("Tuesday night", vec!["Cloudy periods. Low plus 3.".to_string()]),
                ("Wednesday", vec!["Cloudy with 60 percent chance of showers. High 14.".to_string()]),
                ("Sources", vec![
                    "Forecast and observations from Environment and Climate Change Canada; hourly data, sun and UV from Open-Meteo.".to_string(),
                ]),
            ])
        }
        1 => Layout::build(&[
            ("Alerts", vec![
                "Lake effect snow warning until 7:00 pm Monday, from the National Weather Service. Press Enter for details.".to_string(),
            ]),
            ("Right now", vec![
                format!("Snow, {} degrees, wind from the west at 30 kilometres an hour. Observed at Buffalo Niagara International Airport at {time}.", temperature - 12),
                "Humidity 88 percent, dew point minus 2 degrees, pressure 100.9 kilopascals and falling, visibility 1 kilometre.".to_string(),
            ]),
            ("Rest of today", vec![
                "Tonight: Snow. Low around minus 3. West wind 25 to 35 kilometres an hour, with gusts as high as 60. New snow accumulation of 15 to 25 centimetres possible.".to_string(),
            ]),
            ("Monday", vec!["Snow showers, mainly before 2pm. High near 1. Chance of precipitation is 90 percent.".to_string()]),
            ("Monday night", vec!["A chance of snow showers. Mostly cloudy, with a low around minus 4.".to_string()]),
            ("Sources", vec![
                "Forecast and observations from the National Weather Service; hourly data, sun and UV from Open-Meteo.".to_string(),
            ]),
        ]),
        _ => Layout::build(&[
            ("Alerts", vec!["Alerts are not available for this region.".to_string()]),
            ("Right now", vec![
                format!("Clear, {} degrees, wind from the northeast at 10 kilometres an hour. At 9:{:02} pm in Paris ({time} here).", temperature + 3, minutes % 60),
            ]),
            ("Rest of today", vec!["Overnight: clear, falling to 9 degrees.".to_string()]),
            ("Monday", vec!["Monday: sunny, high 21 degrees, light wind.".to_string()]),
            ("Monday night", vec!["Monday night: clear, low 10 degrees.".to_string()]),
            ("Sources", vec!["Forecast from Open-Meteo.".to_string()]),
        ]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_alert_keeps_the_caret_on_the_same_words() {
        let before = forecast(0, 0);
        let after = forecast(0, 1);
        let monday = before
            .headings
            .iter()
            .find(|(h, _)| h == "Monday")
            .unwrap()
            .1;
        let caret = monday + 10;
        let mapped = after.map_caret(&before, caret);
        let after_monday = after
            .headings
            .iter()
            .find(|(h, _)| h == "Monday")
            .unwrap()
            .1;
        assert_eq!(mapped, after_monday + 10);
        assert!(after_monday > monday);
    }

    #[test]
    fn offsets_count_utf16_units() {
        let paris = forecast(2, 0);
        for (heading, at) in &paris.headings {
            let prefix: Vec<u16> = paris.text.encode_utf16().take(*at as usize).collect();
            let rest = &paris.text[String::from_utf16(&prefix).unwrap().len()..];
            assert!(rest.starts_with(heading.as_str()));
        }
    }
}
