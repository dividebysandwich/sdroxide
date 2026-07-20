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
    let (lat1, lon1) = grid_to_latlon(a)?;
    let (lat2, lon2) = grid_to_latlon(b)?;
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dl = (lon2 - lon1).to_radians();
    let y = dl.sin() * p2.cos();
    let x = p1.cos() * p2.sin() - p1.sin() * p2.cos() * dl.cos();
    Some((y.atan2(x).to_degrees() + 360.0) % 360.0)
}

/// Points sampled along the great-circle path a→b as (lat, lon), inclusive.
pub fn great_circle_points((lat1, lon1): (f64, f64), (lat2, lon2): (f64, f64), n: usize) -> Vec<(f64, f64)> {
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

/// True if the (lon, lat) point in degrees is over land, from the baked
/// NASA Blue Marble land/ocean bitmap ([`crate::worldmask`]).
pub fn is_land(lon: f64, lat: f64) -> bool {
    crate::worldmask::is_land(lon, lat)
}

/// Direct access to the land bitmap cell (col, row) for map rendering.
pub fn land_cell(col: usize, row: usize) -> bool {
    crate::worldmask::land_at(col, row)
}

/// Dimensions of the land bitmap (columns, rows).
pub fn land_mask_dims() -> (usize, usize) {
    (crate::worldmask::MASK_W, crate::worldmask::MASK_H)
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

    #[test]
    fn distance_reasonable() {
        // FN42 (Boston) to JO53 (Hamburg) ≈ 6000 km.
        let d = grid_distance_km("FN42", "JO53").unwrap();
        assert!((5500.0..6600.0).contains(&d), "{d}");
    }

    #[test]
    fn land_mask_hits_continents() {
        assert!(is_land(-100.0, 40.0)); // USA
        assert!(is_land(10.0, 50.0)); // Europe
        assert!(!is_land(-140.0, 0.0)); // mid-Pacific
        assert!(!is_land(-30.0, 30.0)); // mid-Atlantic
    }
}
