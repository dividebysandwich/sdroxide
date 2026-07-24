//! Equirectangular land/ocean bitmap for the QSO world map.
//! Source: Natural Earth 1:50m land polygons (public domain), rasterized to an
//! equirectangular grid. Row-major, MSB-first, 1 bit per cell.
//! 2160x1080 cells (1/6 deg); x = lon -180..180, y = lat +90..-90.

pub const MASK_W: usize = 2160;
pub const MASK_H: usize = 1080;

/// Packed land bitmap (`MASK_W * MASK_H / 8` bytes), embedded at compile time.
pub static LAND_BITS: &[u8] = include_bytes!("worldmask.bin");

#[inline]
pub fn land_at(col: usize, row: usize) -> bool {
    if col >= MASK_W || row >= MASK_H {
        return false;
    }
    let idx = row * MASK_W + col;
    (LAND_BITS[idx / 8] >> (7 - (idx % 8))) & 1 == 1
}

/// True if the (lon, lat) point in degrees is over land.
pub fn is_land(lon: f64, lat: f64) -> bool {
    let col = (((lon + 180.0) / 360.0) * MASK_W as f64) as isize;
    let row = (((90.0 - lat) / 180.0) * MASK_H as f64) as isize;
    if col < 0 || row < 0 {
        return false;
    }
    land_at(col as usize, row as usize)
}
