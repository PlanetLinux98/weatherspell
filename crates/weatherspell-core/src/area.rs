// Whether a location is in an alert's area, by outline: the outlines CAP
// alerts carry ("lat,lon lat,lon ...", the first point repeated last) and
// the tables of named areas this app keeps, MeteoAlarm's regions
// (meteoalarm.rs) and the Bureau of Meteorology's warning areas (bom.rs).
// A margin lets a location just outside an outline count as inside it, for
// outlines this app has simplified: hearing a neighbouring region's warning
// is better than missing one's own. A service's own outline is taken as
// drawn.

const KM_PER_DEGREE_LAT: f64 = 110.57;
const KM_PER_DEGREE_LON_AT_EQUATOR: f64 = 111.32;

// How far simplifying may move a boundary, in degrees, before rounding to
// three decimals: together at most about 0.6 km, which the tables' margin
// allows for.
pub const TOLERANCE: f64 = 0.005;

// One polygon: an outer ring and any holes, tested together by the
// even-odd rule. Points are (latitude, longitude).
#[derive(Clone, Debug, PartialEq)]
pub struct Polygon {
    rings: Vec<Vec<(f64, f64)>>,
    south: f64,
    north: f64,
    west: f64,
    east: f64,
}

impl Polygon {
    pub fn new(rings: Vec<Vec<(f64, f64)>>) -> Option<Polygon> {
        let rings: Vec<Vec<(f64, f64)>> = rings.into_iter().filter(|r| !r.is_empty()).collect();
        let points = || rings.iter().flatten();
        let first = *points().next()?;
        let (mut south, mut north, mut west, mut east) = (first.0, first.0, first.1, first.1);
        for &(lat, lon) in points() {
            south = south.min(lat);
            north = north.max(lat);
            west = west.min(lon);
            east = east.max(lon);
        }
        Some(Polygon {
            rings,
            south,
            north,
            west,
            east,
        })
    }

    // CAP's form: space-separated "lat,lon" pairs.
    pub fn parse(text: &str) -> Option<Polygon> {
        let mut ring = Vec::new();
        for pair in text.split_whitespace() {
            let (lat, lon) = pair.split_once(',')?;
            let lat: f64 = lat.trim().parse().ok()?;
            let lon: f64 = lon.trim().parse().ok()?;
            if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
                return None;
            }
            ring.push((lat, lon));
        }
        if ring.len() < 3 {
            return None;
        }
        Polygon::new(vec![ring])
    }

    pub fn rings(&self) -> &[Vec<(f64, f64)>] {
        &self.rings
    }

    pub fn covers(&self, latitude: f64, longitude: f64, margin_km: f64) -> bool {
        let margin_lat = margin_km / KM_PER_DEGREE_LAT;
        let margin_lon = margin_km / km_per_degree_lon(latitude);
        if latitude < self.south - margin_lat
            || latitude > self.north + margin_lat
            || longitude < self.west - margin_lon
            || longitude > self.east + margin_lon
        {
            return false;
        }
        self.contains(latitude, longitude)
            || (margin_km > 0.0 && self.distance_km(latitude, longitude) <= margin_km)
    }

    fn contains(&self, latitude: f64, longitude: f64) -> bool {
        let mut inside = false;
        for ring in &self.rings {
            let mut previous = *ring.last().expect("rings are never empty");
            for &point in ring {
                let ((lat_a, lon_a), (lat_b, lon_b)) = (previous, point);
                if (lat_a > latitude) != (lat_b > latitude) {
                    let crossing = lon_a + (latitude - lat_a) / (lat_b - lat_a) * (lon_b - lon_a);
                    if longitude < crossing {
                        inside = !inside;
                    }
                }
                previous = point;
            }
        }
        inside
    }

    // Distance to the nearest edge, flat-earth around the point, which is
    // close enough at a kilometre's scale.
    fn distance_km(&self, latitude: f64, longitude: f64) -> f64 {
        let x_scale = km_per_degree_lon(latitude);
        let project = |(lat, lon): (f64, f64)| {
            (
                (lon - longitude) * x_scale,
                (lat - latitude) * KM_PER_DEGREE_LAT,
            )
        };
        let mut nearest = f64::INFINITY;
        for ring in &self.rings {
            let mut previous = project(*ring.last().expect("rings are never empty"));
            for &point in ring {
                let current = project(point);
                nearest = nearest.min(segment_distance(previous, current));
                previous = current;
            }
        }
        nearest
    }
}

// A named area in a table. Parent is the larger area that holds it, where
// the table says (a river's catchment within its basin), so a warning for
// the larger one reaches it too.
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    pub code: String,
    pub name: String,
    pub parent: Option<String>,
    polygons: Vec<Polygon>,
}

impl Region {
    pub fn new(code: &str, name: &str, parent: Option<&str>, polygons: Vec<Polygon>) -> Region {
        Region {
            code: code.to_string(),
            name: name.to_string(),
            parent: parent.map(str::to_string),
            polygons,
        }
    }

    pub fn covers(&self, latitude: f64, longitude: f64, margin_km: f64) -> bool {
        self.polygons
            .iter()
            .any(|p| p.covers(latitude, longitude, margin_km))
    }
}

// A table's text: a region a line, its code, name and polygons split by
// tabs, then its parent's code where it has one; polygons split by "|",
// rings by ";", points "lat,lon" by spaces. Lines starting with "#" are
// comments.
pub fn parse_table(text: &str) -> Vec<Region> {
    text.lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .filter_map(|line| {
            let mut fields = line.trim_end().split('\t');
            let (code, name, outlines) = (fields.next()?, fields.next()?, fields.next()?);
            let parent = fields.next().filter(|p| !p.is_empty());
            let polygons = outlines
                .split('|')
                .filter_map(|polygon| {
                    let rings = polygon
                        .split(';')
                        .map(|ring| {
                            ring.split(' ')
                                .filter_map(|point| {
                                    let (lat, lon) = point.split_once(',')?;
                                    Some((lat.parse().ok()?, lon.parse().ok()?))
                                })
                                .collect()
                        })
                        .collect();
                    Polygon::new(rings)
                })
                .collect();
            Some(Region::new(code, name, parent, polygons))
        })
        .collect()
}

pub fn table_line(region: &Region) -> String {
    let polygons: Vec<String> = region
        .polygons
        .iter()
        .map(|p| {
            p.rings
                .iter()
                .map(|ring| {
                    ring.iter()
                        .map(|&(lat, lon)| format!("{},{}", decimals(lat), decimals(lon)))
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect::<Vec<_>>()
                .join(";")
        })
        .collect();
    let mut line = format!("{}\t{}\t{}", region.code, region.name, polygons.join("|"));
    if let Some(parent) = &region.parent {
        line.push('\t');
        line.push_str(parent);
    }
    line
}

// A ring as a table keeps it: simplified (Douglas-Peucker, TOLERANCE),
// rounded to three decimals, without repeated points or the closing one.
// A ring that comes down to two points is kept, so a small island still
// counts within the margin; one that comes down to one is not.
pub fn simplified(ring: &[(f64, f64)]) -> Option<Vec<(f64, f64)>> {
    let mut kept: Vec<(f64, f64)> = Vec::new();
    for point in douglas_peucker(ring) {
        let rounded = (round3(point.0), round3(point.1));
        if kept.last() != Some(&rounded) {
            kept.push(rounded);
        }
    }
    if kept.len() > 1 && kept.first() == kept.last() {
        kept.pop();
    }
    (kept.len() >= 2).then_some(kept)
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

fn decimals(v: f64) -> String {
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

// Without recursion: a coastline can run to a hundred thousand points.
fn douglas_peucker(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut spans = vec![(0, points.len() - 1)];
    while let Some((first, last)) = spans.pop() {
        let mut farthest = (0.0, 0);
        for (i, p) in points.iter().enumerate().take(last).skip(first + 1) {
            let d = line_distance(*p, points[first], points[last]);
            if d > farthest.0 {
                farthest = (d, i);
            }
        }
        if farthest.0 > TOLERANCE {
            keep[farthest.1] = true;
            spans.push((first, farthest.1));
            spans.push((farthest.1, last));
        }
    }
    points
        .iter()
        .zip(keep)
        .filter_map(|(p, k)| k.then_some(*p))
        .collect()
}

// From p to the line through a and b, or to a when they are the same
// point (a closed ring's ends).
fn line_distance(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx.hypot(dy);
    if length == 0.0 {
        return (p.0 - a.0).hypot(p.1 - a.1);
    }
    ((p.0 - a.0) * dy - (p.1 - a.1) * dx).abs() / length
}

fn km_per_degree_lon(latitude: f64) -> f64 {
    (KM_PER_DEGREE_LON_AT_EQUATOR * latitude.to_radians().cos()).max(0.001)
}

// From the origin to the segment a-b.
fn segment_distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx * dx + dy * dy;
    let t = if length == 0.0 {
        0.0
    } else {
        (-(a.0 * dx + a.1 * dy) / length).clamp(0.0, 1.0)
    };
    (a.0 + t * dx).hypot(a.1 + t * dy)
}
