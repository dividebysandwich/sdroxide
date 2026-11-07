//! Maidenhead grid math, great-circle geometry, and a coarse world land
//! mask for the QSO map. Pure + serde-free; shared native and wasm.

/// Parse a 4- or 6-character Maidenhead locator to (lat, lon) in degrees at
/// the center of the square/subsquare.
pub fn grid_to_latlon(grid: &str) -> Option<(f64, f64)> {
    let g: Vec<u8> = grid.trim().bytes().collect();
    if g.len() < 4 {
        return None;
    }
    let field_lon = (g[0].to_ascii_uppercase() as i32) - b'A' as i32;
    let field_lat = (g[1].to_ascii_uppercase() as i32) - b'A' as i32;
    let sq_lon = (g[2] as i32) - b'0' as i32;
    let sq_lat = (g[3] as i32) - b'0' as i32;
    if !(0..18).contains(&field_lon)
        || !(0..18).contains(&field_lat)
        || !(0..10).contains(&sq_lon)
        || !(0..10).contains(&sq_lat)
    {
        return None;
    }
    let mut lon = field_lon as f64 * 20.0 - 180.0 + sq_lon as f64 * 2.0;
    let mut lat = field_lat as f64 * 10.0 - 90.0 + sq_lat as f64;
    if g.len() >= 6 && g[4].is_ascii_alphabetic() && g[5].is_ascii_alphabetic() {
        let sub_lon = (g[4].to_ascii_uppercase() as i32) - b'A' as i32;
        let sub_lat = (g[5].to_ascii_uppercase() as i32) - b'A' as i32;
        lon += sub_lon as f64 * (2.0 / 24.0) + (2.0 / 24.0) / 2.0;
        lat += sub_lat as f64 * (1.0 / 24.0) + (1.0 / 24.0) / 2.0;
    } else {
        lon += 1.0; // center of the 2° square
        lat += 0.5; // center of the 1° square
    }
    Some((lat, lon))
}

/// The 4-character Maidenhead square a locator falls in, uppercased.
///
/// The one truncation every "worked grid" tally shares: a 6- or 8-character
/// locator and its 4-character square are the same award slot, so a caller that
/// truncates differently — or not at all — would count one place twice. `None`
/// for a string too short to be a locator or not opening with a letter, which
/// is the shape check; the rest of the characters are trusted, as the awards
/// tally does.
pub fn grid4(grid: &str) -> Option<String> {
    // `get`, not an index: a locator mistyped with a multi-byte character in
    // its first four bytes is not a square, and must not panic.
    let g = grid.trim().get(..4)?;
    g.as_bytes()[0].is_ascii_alphabetic().then(|| g.to_ascii_uppercase())
}

/// The 6-character Maidenhead locator a position falls in.
///
/// The inverse of [`grid_to_latlon`], and the reason it exists: the public
/// receiver directories publish a position two different ways — a KiwiSDR
/// states a locator, a SpyServer states latitude and longitude — and a station
/// reporting what it heard through somebody else's antenna has to name where
/// that antenna is in the one form PSK Reporter and WSPRnet accept.
///
/// Six characters rather than four because that is what the sources carry and
/// what both networks take; a caller wanting four can truncate. Positions
/// outside the sphere are folded into it rather than refused: a latitude of
/// exactly +90 belongs in the last square, not in the one after it.
pub fn latlon_to_grid(lat: f64, lon: f64) -> String {
    // Shifted into the 0..360 / 0..180 corner the locator system counts from,
    // then clamped a hair inside so that the poles and the antimeridian land in
    // the last square instead of one past its end.
    let lon = (lon + 180.0).clamp(0.0, 360.0 - 1e-9);
    let lat = (lat + 90.0).clamp(0.0, 180.0 - 1e-9);
    let field_lon = (lon / 20.0) as usize;
    let field_lat = (lat / 10.0) as usize;
    let sq_lon = ((lon % 20.0) / 2.0) as usize;
    let sq_lat = (lat % 10.0) as usize;
    let sub_lon = ((lon % 2.0) / (2.0 / 24.0)) as usize;
    let sub_lat = ((lat % 1.0) / (1.0 / 24.0)) as usize;
    let mut s = String::with_capacity(6);
    s.push((b'A' + field_lon as u8) as char);
    s.push((b'A' + field_lat as u8) as char);
    s.push((b'0' + sq_lon as u8) as char);
    s.push((b'0' + sq_lat as u8) as char);
    s.push((b'a' + sub_lon.min(23) as u8) as char);
    s.push((b'a' + sub_lat.min(23) as u8) as char);
    s
}

const EARTH_R_KM: f64 = 6371.0;

/// Great-circle distance in km between two grids.
pub fn grid_distance_km(a: &str, b: &str) -> Option<f64> {
    let (lat1, lon1) = grid_to_latlon(a)?;
    let (lat2, lon2) = grid_to_latlon(b)?;
    Some(distance_km((lat1, lon1), (lat2, lon2)))
}

pub fn distance_km((lat1, lon1): (f64, f64), (lat2, lon2): (f64, f64)) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dphi = (lat2 - lat1).to_radians();
    let dlmb = (lon2 - lon1).to_radians();
    let a = (dphi / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dlmb / 2.0).sin().powi(2);
    2.0 * EARTH_R_KM * a.sqrt().asin()
}

/// Initial great-circle bearing in degrees (0 = north) from a to b.
pub fn grid_bearing(a: &str, b: &str) -> Option<f64> {
    Some(bearing_deg(grid_to_latlon(a)?, grid_to_latlon(b)?))
}

/// Initial great-circle bearing in degrees (0 = north) from a to b — the
/// lat/lon form of [`grid_bearing`], for callers that have already placed both
/// ends.
pub fn bearing_deg((lat1, lon1): (f64, f64), (lat2, lon2): (f64, f64)) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dl = (lon2 - lon1).to_radians();
    let y = dl.sin() * p2.cos();
    let x = p1.cos() * p2.sin() - p1.sin() * p2.cos() * dl.cos();
    (y.atan2(x).to_degrees() + 360.0) % 360.0
}

/// Points sampled along the great-circle path a→b as (lat, lon), inclusive.
pub fn great_circle_points(
    (lat1, lon1): (f64, f64),
    (lat2, lon2): (f64, f64),
    n: usize,
) -> Vec<(f64, f64)> {
    let (p1, l1) = (lat1.to_radians(), lon1.to_radians());
    let (p2, l2) = (lat2.to_radians(), lon2.to_radians());
    let d = {
        let dphi = p2 - p1;
        let dl = l2 - l1;
        let a = (dphi / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
        2.0 * a.sqrt().asin()
    };
    if d < 1e-9 {
        return vec![(lat1, lon1)];
    }
    (0..=n)
        .map(|i| {
            let f = i as f64 / n as f64;
            let a = ((1.0 - f) * d).sin() / d.sin();
            let b = (f * d).sin() / d.sin();
            let x = a * p1.cos() * l1.cos() + b * p2.cos() * l2.cos();
            let y = a * p1.cos() * l1.sin() + b * p2.cos() * l2.sin();
            let z = a * p1.sin() + b * p2.sin();
            let lat = z.atan2((x * x + y * y).sqrt()).to_degrees();
            let lon = y.atan2(x).to_degrees();
            (lat, lon)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_parsing() {
        // FN42 (New England) ≈ 42.5N, 71W.
        let (lat, lon) = grid_to_latlon("FN42").unwrap();
        assert!((lat - 42.5).abs() < 0.6, "{lat}");
        assert!((lon - -71.0).abs() < 1.1, "{lon}");
        // JO53 (northern Germany, ~Hamburg/Rostock) ≈ 53.5N, 11E.
        let (lat, lon) = grid_to_latlon("JO53").unwrap();
        assert!((lat - 53.5).abs() < 0.6);
        assert!((lon - 11.0).abs() < 1.1, "{lon}");
        assert!(grid_to_latlon("XX").is_none());
    }

    /// The inverse has to land back in the square it started from, and the
    /// edges of the world are where an off-by-one shows: +90/+180 belong in the
    /// last square, not one past its end.
    #[test]
    fn grid_from_latlon_round_trips() {
        for g in ["FN42ma", "JO53gk", "JN88ec", "DO30db", "AA00aa", "RR99xx"] {
            let (lat, lon) = grid_to_latlon(g).unwrap();
            assert_eq!(latlon_to_grid(lat, lon), g.to_string(), "{g}");
        }
        // The corners of the sphere, folded in rather than refused.
        assert_eq!(latlon_to_grid(-90.0, -180.0), "AA00aa");
        assert_eq!(latlon_to_grid(90.0, 180.0), "RR99xx");
        // And a plain position: Vienna is JN88.
        assert!(latlon_to_grid(48.2, 16.37).starts_with("JN88"), "{}", latlon_to_grid(48.2, 16.37));
    }

    #[test]
    fn grid4_truncates_a_longer_locator_to_its_square() {
        // A 6- or 8-character locator and its 4-character square are the same
        // award slot, so the tracker has to fold them together or one place is
        // counted twice.
        assert_eq!(grid4("JO53gk"), Some("JO53".to_string()));
        assert_eq!(grid4("JO53gk99"), Some("JO53".to_string()));
        assert_eq!(grid4(" jo53 "), Some("JO53".to_string()));
        assert_eq!(grid4("JO53"), Some("JO53".to_string()));
        // Too short, or not a locator at all.
        assert_eq!(grid4("JO5"), None);
        assert_eq!(grid4("1234"), None);
        assert_eq!(grid4(""), None);
        // The awards tally's own rule: only the leading letter is checked, so
        // the two helpers agree about what a grid is.
        assert_eq!(grid4("JO5x"), Some("JO5X".to_string()));
        // A multi-byte character inside the first four bytes is not a square.
        assert_eq!(grid4("JOxé"), None);
        assert_eq!(grid4("éé"), None);
    }

    #[test]
    fn distance_reasonable() {
        // FN42 (Boston) to JO53 (Hamburg) ≈ 6000 km.
        let d = grid_distance_km("FN42", "JO53").unwrap();
        assert!((5500.0..6600.0).contains(&d), "{d}");
    }
}
