//! Callsign, grid and command packing — the primitives JS8's frame layouts are
//! built from.
//!
//! Everything here is transcribed from JS8Call's `varicode.cpp` and checked
//! against vectors emitted by *linking* that same file
//! (`tools/gen_js8_varicode_vectors.sh`), so the expected values in the tests
//! come from the reference implementation rather than from a second reading of
//! it. That matters more here than anywhere else in the mode: none of this
//! encoding is derivable from first principles. It carries per-country
//! workarounds, a reserved-value block for group names, and a grid encoding
//! that round-trips through degrees.

/// Callsign and grid alphabet, `varicode.cpp:44`. Index 0 is `'0'`; note that
/// space sits at 36, which is what makes short callsigns paddable.
const ALPHANUMERIC: &[u8; 39] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ /@";

/// Size of the ordinary callsign space: six positions with 37/36/10/27/27/27
/// possibilities. Everything at or above this is a reserved name.
pub const NBASECALL: u32 = 37 * 36 * 10 * 27 * 27 * 27;

/// Maidenhead grids that fit in the ordinary range.
const NBASEGRID: u16 = 180 * 180;

/// Returned by [`pack_grid`] when there is no grid to send.
pub const NO_GRID: u16 = (1 << 15) - 1;

/// Where a compound frame's 16-bit field stops meaning a grid and starts
/// carrying a directed command, `varicode.cpp:211` (`nusergrid`). Ten values
/// above the last real grid; everything from here up to [`NO_GRID`] is a
/// command and its number.
pub const NUSERGRID: u16 = NBASEGRID + 10;

/// Reserved callsign values: groups, and the marker for a callsign whose
/// compound form has not been heard yet. `varicode.cpp:214`.
///
/// Order is load-bearing — each entry's index sets its packed value — so append
/// only, and only in step with upstream.
pub const BASECALLS: &[&str] = &[
    "<....>",
    "@ALLCALL",
    "@JS8NET",
    "@DX/NA",
    "@DX/SA",
    "@DX/EU",
    "@DX/AS",
    "@DX/AF",
    "@DX/OC",
    "@DX/AN",
    "@REGION/1",
    "@REGION/2",
    "@REGION/3",
    "@GROUP/0",
    "@GROUP/1",
    "@GROUP/2",
    "@GROUP/3",
    "@GROUP/4",
    "@GROUP/5",
    "@GROUP/6",
    "@GROUP/7",
    "@GROUP/8",
    "@GROUP/9",
    "@COMMAND",
    "@CONTROL",
    "@NET",
    "@NTS",
    // `varicode.cpp:259-280`. These were missing until the group auto-response
    // work, and it mattered: a frame from JS8Call addressed to @EMCOMM or
    // @ARES unpacked to "no such callsign" and was dropped, so the busiest
    // groups on the band could not be heard at all.
    "@RESERVE/0",
    "@RESERVE/1",
    "@RESERVE/2",
    "@RESERVE/3",
    "@RESERVE/4",
    "@APRSIS",
    "@RAGCHEW",
    "@JS8",
    "@EMCOMM",
    "@ARES",
    "@MARS",
    "@AMRRON",
    "@RACES",
    "@RAYNET",
    "@RADAR",
    "@SKYWARN",
    "@CQ",
    "@HB",
    "@QSO",
    "@QSOPARTY",
    "@CONTEST",
    "@FIELDDAY",
    "@SOTA",
    "@IOTA",
    "@POTA",
    "@QRP",
    "@QRO",
];

fn idx(c: u8) -> Option<u32> {
    ALPHANUMERIC.iter().position(|&a| a == c).map(|p| p as u32)
}

/// Pack a callsign into 28 bits, or `None` if it has no representation.
///
/// Reserved names ([`BASECALLS`]) take fixed values above [`NBASECALL`].
/// Ordinary callsigns are normalised into a six-character field matching
/// `[0-9A-Z ][0-9A-Z][0-9][A-Z ][A-Z ][A-Z ]` — the shape essentially every
/// amateur callsign has once padded — then encoded positionally.
pub fn pack_callsign(call: &str) -> Option<u32> {
    let call = call.trim().to_ascii_uppercase();
    if let Some(i) = BASECALLS.iter().position(|&b| b == call) {
        return Some(NBASECALL + 1 + i as u32);
    }

    let mut c = call.as_str().to_string();
    // `/P` is carried as a flag by the frame layer, not in these 28 bits.
    if let Some(stripped) = c.strip_suffix("/P") {
        c = stripped.to_string();
    }
    // Two national prefixes do not fit the pattern and upstream rewrites them
    // into forms that do; the inverse runs in `unpack_callsign`.
    if let Some(rest) = c.strip_prefix("3DA0") {
        c = format!("3D0{rest}"); // Eswatini (Swaziland)
    }
    if let Some(rest) = c.strip_prefix("3X") {
        if rest.starts_with(|ch: char| ch.is_ascii_uppercase()) {
            c = format!("Q{rest}"); // Guinea
        }
    }

    if c.len() < 2 || c.len() > 6 {
        return None;
    }

    // Try each way of padding the callsign to six characters and keep the
    // first that matches the positional pattern.
    let mut candidates = vec![c.clone()];
    match c.len() {
        2 => candidates.push(format!(" {c}   ")),
        3 => {
            candidates.push(format!(" {c}  "));
            candidates.push(format!("{c}   "));
        }
        4 => {
            candidates.push(format!(" {c} "));
            candidates.push(format!("{c}  "));
        }
        5 => {
            candidates.push(format!(" {c}"));
            candidates.push(format!("{c} "));
        }
        _ => {}
    }

    let matched = candidates.into_iter().rev().find(|p| matches_pattern(p))?;
    let b = matched.as_bytes();
    let mut packed = idx(b[0])?;
    packed = 36 * packed + idx(b[1])?;
    packed = 10 * packed + idx(b[2])?;
    packed = 27 * packed + idx(b[3])?.checked_sub(10)?;
    packed = 27 * packed + idx(b[4])?.checked_sub(10)?;
    packed = 27 * packed + idx(b[5])?.checked_sub(10)?;
    Some(packed)
}

/// `[0-9A-Z ][0-9A-Z][0-9][A-Z ][A-Z ][A-Z ]`, `varicode.cpp:43`.
fn matches_pattern(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 6 {
        return false;
    }
    let alnum_or_space = |c: u8| c.is_ascii_digit() || c.is_ascii_uppercase() || c == b' ';
    let alnum = |c: u8| c.is_ascii_digit() || c.is_ascii_uppercase();
    let alpha_or_space = |c: u8| c.is_ascii_uppercase() || c == b' ';
    alnum_or_space(b[0])
        && alnum(b[1])
        && b[2].is_ascii_digit()
        && alpha_or_space(b[3])
        && alpha_or_space(b[4])
        && alpha_or_space(b[5])
}

/// Recover a callsign from its 28-bit form. `portable` re-appends `/P`.
pub fn unpack_callsign(value: u32, portable: bool) -> Option<String> {
    if value > NBASECALL {
        let i = value.checked_sub(NBASECALL + 1)? as usize;
        return BASECALLS.get(i).map(|s| (*s).to_string());
    }

    let mut v = value;
    let mut word = [0u8; 6];
    for slot in (3..6).rev() {
        word[slot] = ALPHANUMERIC[(v % 27 + 10) as usize];
        v /= 27;
    }
    word[2] = ALPHANUMERIC[(v % 10) as usize];
    v /= 10;
    word[1] = ALPHANUMERIC[(v % 36) as usize];
    v /= 36;
    word[0] = *ALPHANUMERIC.get(v as usize)?;

    let mut call = String::from_utf8(word.to_vec()).ok()?;
    if let Some(rest) = call.strip_prefix("3D0") {
        call = format!("3DA0{rest}");
    }
    if let Some(rest) = call.strip_prefix('Q') {
        if rest.starts_with(|c: char| c.is_ascii_uppercase()) {
            call = format!("3X{rest}");
        }
    }
    let call = call.trim().to_string();
    Some(if portable { format!("{call}/P") } else { call })
}

/// Pack a four-character Maidenhead locator into 15 bits.
///
/// Upstream routes this through degrees rather than encoding the characters
/// directly (`varicode.cpp:1129`), and the round trip is lossy in the last
/// place, so it is reproduced here rather than shortened.
pub fn pack_grid(grid: &str) -> u16 {
    let g = grid.trim();
    if g.len() < 4 {
        return NO_GRID;
    }
    let (dlong, dlat) = grid_to_deg(&g[..4]);
    let ilong = dlong as i32;
    // Bias *before* truncating, as upstream does (`int ilat = pair.second + 90`).
    // Truncating first rounds the wrong way for southern latitudes and shifts
    // every grid below the equator by one square.
    let ilat = (dlat + 90.0) as i32;
    (((ilong + 180) / 2) * 180 + ilat) as u16
}

/// Recover a four-character locator, or `None` for the "no grid" value and
/// anything else outside the ordinary range.
pub fn unpack_grid(value: u16) -> Option<String> {
    if value > NBASEGRID {
        return None;
    }
    let dlat = f32::from(value % 180) - 90.0;
    let dlong = f32::from(value / 180) * 2.0 - 180.0 + 2.0;
    Some(deg_to_grid(dlong, dlat)[..4].to_string())
}

fn grid_to_deg(grid: &str) -> (f32, f32) {
    // A four-character grid names a square; upstream takes its centre by
    // filling the sub-square with "mm".
    let g = format!("{}mm", grid.to_ascii_uppercase());
    let b = g.as_bytes();
    let nlong = 180 - 20 * i32::from(b[0] - b'A');
    let n20d = 2 * i32::from(b[2] - b'0');
    let xminlong = 5.0 * (f32::from(b[4] - b'a') + 0.5);
    let dlong = nlong as f32 - n20d as f32 - xminlong / 60.0;

    let nlat = -90 + 10 * i32::from(b[1] - b'A') + i32::from(b[3] - b'0');
    let xminlat = 2.5 * (f32::from(b[5] - b'a') + 0.5);
    (dlong, nlat as f32 + xminlat / 60.0)
}

fn deg_to_grid(mut dlong: f32, dlat: f32) -> String {
    if dlong < -180.0 {
        dlong += 360.0;
    }
    if dlong > 180.0 {
        dlong -= 360.0;
    }
    let mut grid = [0u8; 6];
    let nlong = (60.0 * (180.0 - dlong) / 5.0) as i32;
    let (n1, n2, n3) = (nlong / 240, (nlong - 240 * (nlong / 240)) / 24, nlong % 24);
    grid[0] = b'A' + n1 as u8;
    grid[2] = b'0' + n2 as u8;
    grid[4] = b'a' + n3 as u8;

    let nlat = (60.0 * (dlat + 90.0) / 2.5) as i32;
    let (m1, m2, m3) = (nlat / 240, (nlat - 240 * (nlat / 240)) / 24, nlat % 24);
    grid[1] = b'A' + m1 as u8;
    grid[3] = b'0' + m2 as u8;
    grid[5] = b'a' + m3 as u8;
    String::from_utf8_lossy(&grid).into_owned()
}

/// Pack a report or count into the 6-bit field directed frames carry.
/// Clamped to −30..=31 then biased, `varicode.cpp:1154`.
pub fn pack_num(num: i32) -> u8 {
    (num.clamp(-30, 31) + 31) as u8
}

/// Inverse of [`pack_num`].
pub fn unpack_num(packed: u8) -> i32 {
    i32::from(packed) - 31
}

/// Pack a compound callsign into 50 bits.
///
/// A different encoding from [`pack_callsign`] and used for a different job:
/// the 28-bit form handles the base callsign that fits the positional pattern,
/// while this one carries the full compound form (`W1AW/4`, `KN4CRD/QRP`) at
/// the cost of nearly twice the bits, which is why it only appears in frames
/// that have room for it.
///
/// The field is eleven characters: three, a `/` flag, three, a `/` flag, then
/// three. Slashes are recognised in place rather than encoded, so the
/// separators cost one bit each instead of six.
pub fn pack_alphanumeric50(value: &str) -> u64 {
    let cleaned: String =
        value.to_ascii_uppercase().chars().filter(|c| ALPHANUMERIC.contains(&(*c as u8))).collect();

    // Insert the padding that puts the slash flags at positions 3 and 7.
    let mut word: Vec<char> = cleaned.chars().collect();
    if word.len() > 3 && word[3] != '/' {
        word.insert(3, ' ');
    }
    if word.len() > 7 && word[7] != '/' {
        word.insert(7, ' ');
    }
    while word.len() < 11 {
        word.push(' ');
    }

    let d = |i: usize| ALPHANUMERIC.iter().position(|&a| a == word[i] as u8).unwrap_or(0) as u64;
    let slash = |i: usize| u64::from(word[i] == '/');

    // Place values, least significant first: three base-38 digits, a flag,
    // three more, a flag, then three more.
    let mut packed = d(10);
    packed += 38 * d(9);
    packed += 38 * 38 * d(8);
    packed += 38 * 38 * 38 * slash(7);
    packed += 38 * 38 * 38 * 2 * d(6);
    packed += 38 * 38 * 38 * 2 * 38 * d(5);
    packed += 38 * 38 * 38 * 2 * 38 * 38 * d(4);
    packed += 38 * 38 * 38 * 2 * 38 * 38 * 38 * slash(3);
    packed += 38 * 38 * 38 * 2 * 38 * 38 * 38 * 2 * d(2);
    packed += 38 * 38 * 38 * 2 * 38 * 38 * 38 * 2 * 38 * d(1);
    packed += 38 * 38 * 38 * 2 * 38 * 38 * 38 * 2 * 38 * 38 * d(0);
    packed
}

/// True when `call` has to travel in the 50-bit compound form: it is a valid
/// compound callsign or custom group that the 28-bit form cannot carry.
///
/// Upstream's `isCompoundCallsign` (`varicode.cpp:1292`) with
/// `isValidCompoundCallsign` (`varicode.cpp:1232`). The checks are:
/// * Reserved names (`@ALLCALL`, `@JS8NET`…) and ordinary callsigns, `/P`
///   included, fit 28 bits and are not compound.
/// * At most nine characters once the slashes are removed.
/// * It must be a group (`@…`), contain a `/` whose prefix is not a reserved
///   name, or look like a callsign (a letter beside a digit). The last rule
///   stops a short word being sent as an address.
///
/// It also refuses anything the 50-bit field cannot represent exactly. Upstream
/// does this with its `compound_callsign_pattern` regex. Here, a value counts as
/// representable when it unpacks back to itself.
pub fn needs_compound(call: &str) -> bool {
    let call = call.trim().to_ascii_uppercase();
    if call.is_empty() || pack_callsign(&call).is_some() {
        return false;
    }
    if call.chars().filter(|&c| c != '/').count() > 9 {
        return false;
    }
    if !call.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'/' || c == b'@') {
        return false;
    }
    // `@` is only the leading character of a group: the 50-bit alphabet has
    // room for it in the first position and nowhere else.
    if call[1..].contains('@') {
        return false;
    }
    let shaped = if let Some(i) = call.find('/') {
        !BASECALLS.contains(&&call[..i])
    } else if call.starts_with('@') {
        call.len() > 1
    } else {
        call.len() > 2
            && call.as_bytes().windows(2).any(|w| {
                (w[0].is_ascii_digit() && w[1].is_ascii_uppercase())
                    || (w[0].is_ascii_uppercase() && w[1].is_ascii_digit())
            })
    };
    shaped && unpack_alphanumeric50(pack_alphanumeric50(&call)) == call
}

/// Inverse of [`pack_alphanumeric50`].
///
/// Note the leading position divides by 39, not 38 — upstream's asymmetry, and
/// correct: the first character may be any of the 39 alphabet entries while the
/// rest are bounded at 38.
pub fn unpack_alphanumeric50(mut packed: u64) -> String {
    let mut word = [' '; 11];
    let take = |packed: &mut u64, base: u64| {
        let v = (*packed % base) as usize;
        *packed /= base;
        ALPHANUMERIC[v] as char
    };
    word[10] = take(&mut packed, 38);
    word[9] = take(&mut packed, 38);
    word[8] = take(&mut packed, 38);
    word[7] = if packed % 2 == 1 { '/' } else { ' ' };
    packed /= 2;
    word[6] = take(&mut packed, 38);
    word[5] = take(&mut packed, 38);
    word[4] = take(&mut packed, 38);
    word[3] = if packed % 2 == 1 { '/' } else { ' ' };
    packed /= 2;
    word[2] = take(&mut packed, 38);
    word[1] = take(&mut packed, 38);
    word[0] = take(&mut packed, 39);
    word.iter().filter(|c| **c != ' ').collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_what_28_bits_cannot_carry_needs_the_compound_form() {
        // Custom groups and slashed calls do.
        for call in ["@MAGNET", "@TEST/1", "W1AW/4", "KN4CRD/QRP", "VE/KN4CRD"] {
            assert!(needs_compound(call), "{call}");
        }
        // Ordinary calls, /P and the reserved groups (old and new) fit a
        // directed frame and must keep using one: that is what JS8Call sends.
        for call in ["KN4CRD", "KN4CRD/P", "@ALLCALL", "@EMCOMM", "@AMRRON", "@QRO"] {
            assert!(!needs_compound(call), "{call}");
        }
        // Not addresses at all: too long, a stray @, a bare word.
        for call in ["@WAYTOOLONGNAME", "KN@CRD", "", "HELLO"] {
            assert!(!needs_compound(call), "{call:?}");
        }
    }

    #[test]
    fn callsigns_round_trip() {
        for call in ["KN4CRD", "N0JDS", "VK3ABC", "W1AW", "9A1A", "OH2BH", "K7A"] {
            let packed = pack_callsign(call).unwrap_or_else(|| panic!("{call} did not pack"));
            assert_eq!(unpack_callsign(packed, false).as_deref(), Some(call), "{call}");
        }
    }

    #[test]
    fn the_national_prefix_workarounds_survive_the_round_trip() {
        // Eswatini and Guinea do not fit the positional pattern, so upstream
        // rewrites them on the way in and back on the way out. Getting only one
        // half right would corrupt exactly two countries' callsigns.
        for call in ["3DA0XX", "3XY1AB"] {
            let packed = pack_callsign(call).unwrap_or_else(|| panic!("{call} did not pack"));
            assert_eq!(unpack_callsign(packed, false).as_deref(), Some(call), "{call}");
        }
    }

    #[test]
    fn group_names_take_reserved_values_above_the_callsign_space() {
        for name in BASECALLS {
            let packed = pack_callsign(name).unwrap_or_else(|| panic!("{name} did not pack"));
            assert!(packed > NBASECALL, "{name} collided with the callsign space");
            assert_eq!(unpack_callsign(packed, false).as_deref(), Some(*name));
        }
    }

    #[test]
    fn portable_suffixes_are_carried_by_the_flag_not_the_bits() {
        let packed = pack_callsign("KN4CRD/P").expect("packs");
        assert_eq!(packed, pack_callsign("KN4CRD").expect("packs"));
        assert_eq!(unpack_callsign(packed, true).as_deref(), Some("KN4CRD/P"));
    }

    #[test]
    fn a_callsign_that_cannot_be_represented_is_refused() {
        assert!(pack_callsign("A").is_none());
        assert!(pack_callsign("TOOLONGCALL").is_none());
    }

    #[test]
    fn grids_round_trip() {
        for grid in ["EM73", "QF22", "IO91", "FN42", "JJ00", "JO31", "PM95", "BP51"] {
            let packed = pack_grid(grid);
            assert_eq!(unpack_grid(packed).as_deref(), Some(grid), "{grid}");
        }
    }

    #[test]
    fn an_absent_grid_has_its_own_value() {
        assert_eq!(pack_grid(""), NO_GRID);
        assert_eq!(pack_grid("EM"), NO_GRID);
        assert!(unpack_grid(NO_GRID).is_none());
    }

    #[test]
    fn compound_callsigns_round_trip_through_fifty_bits() {
        // The 50-bit form exists precisely to carry what the 28-bit one cannot.
        for call in ["W1AW/4", "KN4CRD/QRP", "VK3ABC/P", "G0ABC", "DL1ABC/MM", "N0JDS"] {
            let packed = pack_alphanumeric50(call);
            assert_eq!(unpack_alphanumeric50(packed), call, "{call}");
            assert!(packed < (1 << 50), "{call} overflowed the field");
        }
    }

    #[test]
    fn the_fifty_bit_form_places_slashes_as_flags_not_characters() {
        // Slashes at positions 3 and 7 cost one bit each; anywhere else they
        // would have to be spelled out and the callsign would not fit.
        let a = pack_alphanumeric50("W1AW/4");
        let b = pack_alphanumeric50("W1AW4");
        assert_ne!(a, b, "the slash must survive packing");
        assert_eq!(unpack_alphanumeric50(a), "W1AW/4");
    }

    #[test]
    fn reports_round_trip_across_the_range_they_clamp_to() {
        for n in -30..=31 {
            assert_eq!(unpack_num(pack_num(n)), n);
        }
        // And out-of-range values clamp rather than wrapping into a wrong report.
        assert_eq!(unpack_num(pack_num(-99)), -30);
        assert_eq!(unpack_num(pack_num(99)), 31);
    }
}
