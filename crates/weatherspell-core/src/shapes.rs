// Reading the Bureau of Meteorology's outline files (bom.rs): a zip
// archive holding an ESRI shapefile, whose .shp has a polygon for each
// area and whose .dbf has a row of attributes for each, in the same order.
// Only what those files use is read: stored or deflated zip entries,
// polygon shapes (null shapes kept in their place), and text fields.

use std::collections::HashMap;
use std::io::Read;

use flate2::read::DeflateDecoder;

// Rings as (latitude, longitude): the shapefile's x is longitude.
pub type Rings = Vec<Vec<(f64, f64)>>;

fn u16_at(bytes: &[u8], at: usize) -> Option<usize> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?) as usize)
}

fn u32_at(bytes: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?) as usize)
}

fn i32_at(bytes: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn f64_at(bytes: &[u8], at: usize) -> Option<f64> {
    Some(f64::from_le_bytes(bytes.get(at..at + 8)?.try_into().ok()?))
}

// The file of that name in a zip archive, found through the central
// directory; names match whatever their case ("IDM00014.CPG").
pub fn unzip(archive: &[u8], name: &str) -> Result<Vec<u8>, String> {
    let bad = || "the outline archive could not be read".to_string();
    // The end record is the last thing in the file, before a comment of
    // at most 64 KB.
    let end = (0..archive.len().saturating_sub(21))
        .rev()
        .take(65_536 + 22)
        .find(|&i| archive[i..].starts_with(b"PK\x05\x06"))
        .ok_or_else(bad)?;
    let entries = u16_at(archive, end + 10).ok_or_else(bad)?;
    let mut at = u32_at(archive, end + 16).ok_or_else(bad)?;
    for _ in 0..entries {
        if !archive
            .get(at..)
            .is_some_and(|r| r.starts_with(b"PK\x01\x02"))
        {
            return Err(bad());
        }
        let method = u16_at(archive, at + 10).ok_or_else(bad)?;
        let packed = u32_at(archive, at + 20).ok_or_else(bad)?;
        let size = u32_at(archive, at + 24).ok_or_else(bad)?;
        let name_length = u16_at(archive, at + 28).ok_or_else(bad)?;
        let extra = u16_at(archive, at + 30).ok_or_else(bad)?;
        let comment = u16_at(archive, at + 32).ok_or_else(bad)?;
        let local = u32_at(archive, at + 42).ok_or_else(bad)?;
        let entry_name = archive
            .get(at + 46..at + 46 + name_length)
            .ok_or_else(bad)?;
        at += 46 + name_length + extra + comment;
        if !String::from_utf8_lossy(entry_name).eq_ignore_ascii_case(name) {
            continue;
        }
        if !archive
            .get(local..)
            .is_some_and(|r| r.starts_with(b"PK\x03\x04"))
        {
            return Err(bad());
        }
        let start = local
            + 30
            + u16_at(archive, local + 26).ok_or_else(bad)?
            + u16_at(archive, local + 28).ok_or_else(bad)?;
        let data = archive.get(start..start + packed).ok_or_else(bad)?;
        return match method {
            0 => Ok(data.to_vec()),
            8 => {
                let mut out = Vec::with_capacity(size);
                DeflateDecoder::new(data)
                    .read_to_end(&mut out)
                    .map_err(|_| bad())?;
                Ok(out)
            }
            _ => Err(bad()),
        };
    }
    Err(format!("the outline archive has no {name}"))
}

// Each record's polygon, in the file's order; a null shape is an empty
// list, so the rows of the .dbf still line up. Polygons with heights or
// measures (types 15 and 25) start the same way and are read as plain.
pub fn polygons(shp: &[u8]) -> Result<Vec<Rings>, String> {
    let bad = || "the outline file could not be read".to_string();
    let mut shapes = Vec::new();
    let mut at = 100;
    while at + 8 <= shp.len() {
        let words = u32::from_be_bytes(shp[at + 4..at + 8].try_into().map_err(|_| bad())?);
        let content = shp
            .get(at + 8..at + 8 + words as usize * 2)
            .ok_or_else(bad)?;
        at += 8 + words as usize * 2;
        match i32_at(content, 0).ok_or_else(bad)? {
            0 => shapes.push(Vec::new()),
            5 | 15 | 25 => {
                let parts = i32_at(content, 36).ok_or_else(bad)?.max(0) as usize;
                let count = i32_at(content, 40).ok_or_else(bad)?.max(0) as usize;
                let first_point = 44 + 4 * parts;
                let point = |i: usize| -> Option<(f64, f64)> {
                    let x = f64_at(content, first_point + 16 * i)?;
                    let y = f64_at(content, first_point + 16 * i + 8)?;
                    Some((y, x))
                };
                let mut starts = Vec::with_capacity(parts);
                for p in 0..parts {
                    starts.push(i32_at(content, 44 + 4 * p).ok_or_else(bad)?.max(0) as usize);
                }
                let mut rings = Vec::with_capacity(parts);
                for (p, &start) in starts.iter().enumerate() {
                    let end = starts.get(p + 1).copied().unwrap_or(count).min(count);
                    let ring: Option<Vec<(f64, f64)>> = (start..end).map(point).collect();
                    rings.push(ring.ok_or_else(bad)?);
                }
                shapes.push(rings);
            }
            _ => return Err("the outline file holds shapes other than polygons".to_string()),
        }
    }
    Ok(shapes)
}

// Each row's fields by name, text trimmed, in the file's order; a deleted
// row is empty, so the rows still line up with the shapes. Text is UTF-8
// where it reads as such, else Latin-1, which dBase files often are.
pub fn rows(dbf: &[u8]) -> Result<Vec<HashMap<String, String>>, String> {
    let bad = || "the outline attributes could not be read".to_string();
    let count = u32_at(dbf, 4).ok_or_else(bad)?;
    let header = u16_at(dbf, 8).ok_or_else(bad)?;
    let length = u16_at(dbf, 10).ok_or_else(bad)?;
    let mut fields = Vec::new();
    let mut at = 32;
    let mut offset = 1;
    while *dbf.get(at).ok_or_else(bad)? != 0x0D {
        let descriptor = dbf.get(at..at + 32).ok_or_else(bad)?;
        let name = String::from_utf8_lossy(&descriptor[..11])
            .trim_end_matches('\0')
            .to_string();
        let width = descriptor[16] as usize;
        fields.push((name, offset, width));
        offset += width;
        at += 32;
    }
    let mut rows = Vec::with_capacity(count);
    for i in 0..count {
        let record = dbf
            .get(header + i * length..header + (i + 1) * length)
            .ok_or_else(bad)?;
        let mut row = HashMap::new();
        if record[0] != b'*' {
            for (name, offset, width) in &fields {
                let raw = record.get(*offset..offset + width).ok_or_else(bad)?;
                row.insert(
                    name.clone(),
                    text(raw).trim_matches([' ', '\0']).to_string(),
                );
            }
        }
        rows.push(row);
    }
    Ok(rows)
}

fn text(raw: &[u8]) -> String {
    match std::str::from_utf8(raw) {
        Ok(s) => s.to_string(),
        Err(_) => raw.iter().map(|&b| b as char).collect(),
    }
}
