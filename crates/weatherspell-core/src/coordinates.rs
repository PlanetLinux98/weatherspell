// Coordinates typed or pasted into the Add Location search field, read in
// as many of the ways people write and paste them as can be told apart
// from a place name or postal code: decimals with signs or compass letters
// (English, French and Spanish), degrees with minutes and seconds, degrees
// and decimal minutes as GPS units write them, decimal commas, either order
// when letters or labels say which is which, and the map links people
// copy. Anything that is not wholly coordinates gives None and goes to the
// place search, so a ZIP code, "SW1A 1AA" or a Japanese "060-0001" is never
// mistaken for a point.

use std::sync::LazyLock;

use regex::Regex;

use crate::units::trimmed;

// A point, or, when the text was plainly meant as coordinates but cannot
// be one, the reason to tell the user.
#[derive(Clone, Debug, PartialEq)]
pub struct CoordinateReading {
    pub latitude: f64,
    pub longitude: f64,
    pub problem: Option<String>,
}

impl CoordinateReading {
    fn point(latitude: f64, longitude: f64) -> CoordinateReading {
        CoordinateReading {
            latitude,
            longitude,
            problem: None,
        }
    }

    fn fail(problem: String) -> CoordinateReading {
        CoordinateReading {
            latitude: 0.0,
            longitude: 0.0,
            problem: Some(problem),
        }
    }
}

const EXAMPLE: &str = "44.54, -78.54";

pub fn read(text: &str) -> Option<CoordinateReading> {
    let s = normalize(text);
    if s.is_empty() {
        return None;
    }
    if is_link(&s) {
        return Some(read_link(&s));
    }

    let tokens = tokenize(&decimal_commas(&s))?;
    // Only text that can only be coordinates earns an explanation when it
    // fails. Whole numbers with nothing else count only when a comma or
    // semicolon parts them: "114 55" and "110 00" are Swedish and Czech
    // postal codes.
    let evident = tokens.iter().any(|t| {
        matches!(
            t.kind,
            Kind::Hemisphere | Kind::Label | Kind::Degrees | Kind::Minutes | Kind::Seconds
        ) || (t.kind == Kind::Number && t.has_decimals)
    });
    if !evident && indexes(&tokens, Kind::Separator).len() != 1 {
        return None;
    }
    let fail = |problem: String| evident.then(|| CoordinateReading::fail(problem));
    let unreadable = || {
        fail(format!(
            "Couldn't read those coordinates. Type a latitude and then a longitude, such as {EXAMPLE}."
        ))
    };

    let Some((first_tokens, second_tokens)) = split(&tokens) else {
        return unreadable();
    };
    let (Some(first), Some(second)) = (component(&first_tokens), component(&second_tokens)) else {
        return unreadable();
    };
    if let Some(problem) = first.problem.or(second.problem.clone()) {
        return fail(problem);
    }

    let (lat, lon) = match (first.axis, second.axis) {
        (Some(a), Some(b)) if a == b => {
            let both = if a == Axis::Latitude {
                "latitudes"
            } else {
                "longitudes"
            };
            return fail(format!(
                "Couldn't read those coordinates: both are {both}. Type a latitude and then a longitude, such as {EXAMPLE}."
            ));
        }
        (Some(a), _) => {
            if a == Axis::Latitude {
                (first.value, second.value)
            } else {
                (second.value, first.value)
            }
        }
        (None, Some(b)) => {
            if b == Axis::Longitude {
                (first.value, second.value)
            } else {
                (second.value, first.value)
            }
        }
        // Longitude first, as GeoJSON and some GIS tools write it: the
        // first number cannot be a latitude and the second can.
        (None, None)
            if first.value.abs() > 90.0
                && first.value.abs() <= 180.0
                && second.value.abs() <= 90.0 =>
        {
            (second.value, first.value)
        }
        (None, None) => (first.value, second.value),
    };

    if lat.abs() > 90.0 {
        return fail(latitude_problem(lat));
    }
    if lon.abs() > 180.0 {
        return fail(longitude_problem(lon));
    }
    Some(CoordinateReading::point(lat, lon))
}

fn latitude_problem(value: f64) -> String {
    format!(
        "Couldn't use {} as a latitude: it must be from 90 south to 90 north.",
        trimmed(value, 6)
    )
}

fn longitude_problem(value: f64) -> String {
    format!(
        "Couldn't use {} as a longitude: it must be from 180 west to 180 east.",
        trimmed(value, 6)
    )
}

// "44.54 north, 78.54 west": how a point is said in results and used as the
// name of a point nothing nearby names. Four places is about ten metres,
// finer than any forecast.
pub fn words(latitude: f64, longitude: f64) -> String {
    format!(
        "{}, {}",
        word(latitude, "north", "south"),
        word(longitude, "east", "west")
    )
}

fn word(value: f64, positive: &str, negative: &str) -> String {
    let rounded = (value * 10_000.0).round_ties_even() / 10_000.0;
    format!(
        "{} {}",
        trimmed(rounded.abs(), 4),
        if rounded < 0.0 { negative } else { positive }
    )
}

// Typographic marks as they arrive from web pages and word processors fold
// into the plain ones: primes and curly quotes into minutes and seconds,
// the masculine ordinal and ring into degrees, and the minus sign and
// dashes into a hyphen-minus.
fn normalize(text: &str) -> String {
    let mut out = String::new();
    for c in text.trim().chars() {
        match c {
            '\u{2212}' | '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}'
            | '\u{FE63}' | '\u{FF0D}' => out.push('-'),
            '\u{2032}' | '\u{2018}' | '\u{2019}' | '\u{00B4}' | '`' => out.push('\''),
            '\u{2033}' | '\u{201C}' | '\u{201D}' => out.push('"'),
            '\u{00BA}' | '\u{02DA}' => out.push('\u{00B0}'),
            '\u{00A0}' | '\u{202F}' | '\t' => out.push(' '),
            _ => out.extend(c.to_lowercase()),
        }
    }
    out.replace("''", "\"").trim().to_string()
}

// Decimal commas ("44,54 -78,54", as a French or German Windows writes
// them) are commas between digits in text with no decimal point, and only
// when something else separates the two numbers; "44,78" stays a
// separator.
fn decimal_commas(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let digit = |i: usize| chars.get(i).is_some_and(char::is_ascii_digit);
    let between_digits = |i: usize| i > 0 && digit(i - 1) && digit(i + 1);
    let commas: Vec<usize> = (0..chars.len()).filter(|&i| chars[i] == ',').collect();
    if s.contains('.') || !commas.iter().any(|&i| between_digits(i)) {
        return s.to_string();
    }
    let other_separator = s.contains(';')
        || chars.iter().any(|c| c.is_whitespace())
        || commas
            .iter()
            .any(|&i| !(i > 0 && digit(i - 1)) || !digit(i + 1));
    if !other_separator {
        return s.to_string();
    }
    chars
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            if c == ',' && between_digits(i) {
                '.'
            } else {
                c
            }
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Number,
    Sign,
    Degrees,
    Minutes,
    Seconds,
    Hemisphere,
    Label,
    Separator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    Latitude,
    Longitude,
}

#[derive(Clone, Copy, Debug)]
struct Token {
    kind: Kind,
    value: f64,
    has_decimals: bool,
    negative: bool,
    axis: Option<Axis>,
}

impl Token {
    fn of(kind: Kind) -> Token {
        Token {
            kind,
            value: 0.0,
            has_decimals: false,
            negative: false,
            axis: None,
        }
    }

    fn hemisphere(axis: Axis, negative: bool) -> Token {
        Token {
            negative,
            axis: Some(axis),
            ..Token::of(Kind::Hemisphere)
        }
    }

    fn label(axis: Axis) -> Token {
        Token {
            axis: Some(axis),
            ..Token::of(Kind::Label)
        }
    }
}

fn vocabulary(word: &str) -> Option<Token> {
    use Axis::{Latitude, Longitude};
    Some(match word {
        "n" | "north" | "nord" | "norte" => Token::hemisphere(Latitude, false),
        "s" | "south" | "sud" | "sur" => Token::hemisphere(Latitude, true),
        "e" | "east" | "est" | "este" => Token::hemisphere(Longitude, false),
        "w" | "west" | "o" | "ouest" | "oeste" => Token::hemisphere(Longitude, true),
        "lat" | "latitude" => Token::label(Latitude),
        "lon" | "long" | "lng" | "longitude" => Token::label(Longitude),
        // "s" is south, so seconds are only ever spelt out or marked.
        "d" | "deg" | "degree" | "degrees" => Token::of(Kind::Degrees),
        "m" | "min" | "mins" | "minute" | "minutes" => Token::of(Kind::Minutes),
        "sec" | "secs" | "second" | "seconds" => Token::of(Kind::Seconds),
        _ => return None,
    })
}

// None when anything in the text is not part of a coordinate.
fn tokenize(s: &str) -> Option<Vec<Token>> {
    let chars: Vec<char> = s.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() || matches!(c, ':' | '=' | '&' | '(' | ')' | '[' | ']') {
            // Colons also stand between degrees, minutes and seconds
            // ("44:32:24"), where the numbers' order is enough.
            i += 1;
        } else if matches!(c, '-' | '+') && i > 0 && chars[i - 1].is_ascii_digit() {
            // A sign against a digit is a postal code's hyphen: "060-0001"
            // (Japan), "01310-100" (Brazil), ZIP+4.
            return None;
        } else if c.is_ascii_digit()
            || (c == '.' && chars.get(i + 1).is_some_and(char::is_ascii_digit))
        {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let mut decimals = false;
            if i < chars.len() && chars[i] == '.' {
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                    decimals = true;
                }
            }
            let digits: String = chars[start..i].iter().collect();
            let value = digits.parse::<f64>().ok()?;
            tokens.push(Token {
                value,
                has_decimals: decimals,
                ..Token::of(Kind::Number)
            });
        } else if matches!(c, '-' | '+') {
            tokens.push(Token {
                negative: c == '-',
                ..Token::of(Kind::Sign)
            });
            i += 1;
        } else if c == '\u{00B0}' {
            tokens.push(Token::of(Kind::Degrees));
            i += 1;
        } else if c == '\'' {
            tokens.push(Token::of(Kind::Minutes));
            i += 1;
        } else if c == '"' {
            tokens.push(Token::of(Kind::Seconds));
            i += 1;
        } else if matches!(c, ',' | ';' | '/') {
            tokens.push(Token::of(Kind::Separator));
            i += 1;
        } else if c.is_ascii_lowercase() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_lowercase() {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            tokens.push(vocabulary(&word)?);
        } else {
            return None;
        }
    }
    // One number is never a point: "N1" and "M1" are postal districts.
    (tokens.iter().filter(|t| t.kind == Kind::Number).count() >= 2).then_some(tokens)
}

fn indexes(t: &[Token], kind: Kind) -> Vec<usize> {
    (0..t.len()).filter(|&i| t[i].kind == kind).collect()
}

type Halves = (Vec<Token>, Vec<Token>);

fn at(t: &[Token], index: usize, skip: usize) -> Option<Halves> {
    (index > 0 && index + skip < t.len()).then(|| (t[..index].to_vec(), t[index + skip..].to_vec()))
}

// The two halves: at the one separator; else where the second label or
// compass letter begins; else at the second degree mark or the sign after
// the first; else plain numbers halved (2, 4 or 6 of them).
fn split(t: &[Token]) -> Option<Halves> {
    let separators = indexes(t, Kind::Separator);
    if separators.len() > 1 {
        return None;
    }
    if let [one] = separators[..] {
        return at(t, one, 1);
    }

    let labels = indexes(t, Kind::Label);
    if labels.len() == 2 {
        return if labels[0] == 0 {
            at(t, labels[1], 0)
        } else {
            None
        };
    }

    let hemis = indexes(t, Kind::Hemisphere);
    if hemis.len() == 2 {
        if hemis[0] == 0 {
            return at(t, hemis[1], 0);
        }
        if hemis[1] == t.len() - 1 {
            return at(t, hemis[0] + 1, 0);
        }
        return None;
    }
    if hemis.len() > 2 {
        return None;
    }

    let prefix_style = matches!(t[0].kind, Kind::Hemisphere | Kind::Label);
    let degrees = indexes(t, Kind::Degrees);
    let signs: Vec<usize> = indexes(t, Kind::Sign)
        .into_iter()
        .filter(|&i| i > 0)
        .collect();
    let second_start = if degrees.len() == 2 {
        degrees[1].checked_sub(1)
    } else if signs.len() == 1 {
        Some(signs[0] + 1)
    } else if signs.is_empty() {
        let numbers = indexes(t, Kind::Number);
        matches!(numbers.len(), 2 | 4 | 6).then(|| numbers[numbers.len() / 2])
    } else {
        None
    };
    let mut start = second_start?;
    if start == 0 || t.get(start).is_none_or(|token| token.kind != Kind::Number) {
        return None;
    }

    // Back over the sign, and over a compass letter or label that opens the
    // second half when the first half opened with one too.
    if t[start - 1].kind == Kind::Sign {
        start -= 1;
    }
    if start > 0 && prefix_style && matches!(t[start - 1].kind, Kind::Hemisphere | Kind::Label) {
        start -= 1;
    }
    if start > 0 { at(t, start, 0) } else { None }
}

struct Part {
    value: f64,
    axis: Option<Axis>,
    problem: Option<String>,
}

// One half: [label] [letter] [sign] degrees [mark] [minutes [mark] [seconds [mark]]] [letter].
fn component(t: &[Token]) -> Option<Part> {
    let mut i = 0;
    let mut next = |kind: Kind| -> Option<Token> {
        let token = t.get(i).filter(|token| token.kind == kind).copied();
        if token.is_some() {
            i += 1;
        }
        token
    };

    let mut axis = next(Kind::Label).and_then(|label| label.axis);
    let prefix = next(Kind::Hemisphere);
    let mut negative = next(Kind::Sign).is_some_and(|sign| sign.negative);
    let degrees = next(Kind::Number)?;
    next(Kind::Degrees);
    let minutes = next(Kind::Number);
    if minutes.is_some() {
        next(Kind::Minutes);
    }
    let seconds = if minutes.is_some() {
        next(Kind::Number)
    } else {
        None
    };
    if seconds.is_some() {
        next(Kind::Seconds);
    }
    let suffix = if prefix.is_none() {
        next(Kind::Hemisphere)
    } else {
        None
    };
    if i != t.len() {
        return None;
    }

    if let Some(hemi) = prefix.or(suffix) {
        if axis.is_some() && axis != hemi.axis {
            return None;
        }
        // "-78.54 W" says west twice; "-44 N" contradicts itself.
        if negative && !hemi.negative {
            return None;
        }
        axis = hemi.axis;
        negative = hemi.negative;
    }

    let mut value = degrees.value;
    if let Some(minutes) = minutes {
        // Only the last number may have decimals: "44.5 32" is not a reading.
        if degrees.has_decimals || (seconds.is_some() && minutes.has_decimals) {
            return None;
        }
        let seconds_value = seconds.map_or(0.0, |s| s.value);
        if minutes.value >= 60.0 || seconds_value >= 60.0 {
            return Some(Part {
                value: 0.0,
                axis,
                problem: Some(
                    "Couldn't read those coordinates: minutes and seconds must be under 60."
                        .to_string(),
                ),
            });
        }
        value += minutes.value / 60.0 + seconds_value / 3600.0;
    }
    Some(Part {
        value: if negative { -value } else { value },
        axis,
        problem: None,
    })
}

// Map links: the dropped pin where the link has one, else the view's
// centre. Short links (maps.app.goo.gl) carry no coordinates at all.
fn is_link(s: &str) -> bool {
    static DOMAIN_PATH: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^[a-z0-9-]+(\.[a-z0-9-]+)*\.[a-z]{2,}/").unwrap());
    s.starts_with("geo:") || s.contains("://") || s.starts_with("www.") || DOMAIN_PATH.is_match(s)
}

static LINK_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    const N: &str = r"([-+]?\d+(?:\.\d+)?)";
    [
        format!(r"^geo:\s*{N}\s*,\s*{N}"),
        format!(r"!3d{N}!4d{N}"),
        format!(r"[?&](?:q|query|ll|sll|daddr|destination|center|coordinate|where1)=[\s+]*(?:loc:)?[\s+]*{N}[\s+]*,[\s+]*{N}"),
        format!(r"[?&]mlat={N}&mlon={N}"),
        format!(r"[?&]lat={N}&(?:lon|lng)={N}"),
        format!(r"#map=\d+(?:\.\d+)?/{N}/{N}"),
        format!(r"[?&]cp={N}~{N}"),
        format!(r"@{N},{N}"),
    ]
    .iter()
    .map(|p| Regex::new(p).unwrap())
    .collect()
});

fn read_link(s: &str) -> CoordinateReading {
    let url = unescape(s);
    for pattern in LINK_PATTERNS.iter() {
        let Some(m) = pattern.captures(&url) else {
            continue;
        };
        let (Ok(lat), Ok(lon)) = (m[1].parse::<f64>(), m[2].parse::<f64>()) else {
            continue;
        };
        if lat.abs() > 90.0 {
            return CoordinateReading::fail(latitude_problem(lat));
        }
        if lon.abs() > 180.0 {
            return CoordinateReading::fail(longitude_problem(lon));
        }
        return CoordinateReading::point(lat, lon);
    }
    CoordinateReading::fail(
        "That link has no coordinates in it. Copy the coordinates themselves, or search for the place by name.".to_string(),
    )
}

// Uri.UnescapeDataString: %XX sequences decoded as UTF-8; anything that
// does not decode is left as it was.
fn unescape(s: &str) -> String {
    let hex = |b: u8| (b as char).to_digit(16).map(|d| d as u8);
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(high), Some(low)) = (hex(bytes[i + 1]), hex(bytes[i + 2]))
        {
            out.push(high * 16 + low);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}
