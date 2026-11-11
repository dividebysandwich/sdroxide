//! Station catalogues beyond the broadcast schedule, for LABELS BY S/N: the
//! whole EiBi schedule (utility stations, fax, DRM, the rest the waterfall's
//! broadcast list leaves out) and Nobuo Aoki's shortwave list (NDXC), which
//! the user imports by hand.
//!
//! Aoki's list is never fetched: its pages carry no licence for reuse, ask not
//! to be crawled, and change the file's name and place every season. The user
//! downloads it from <http://www1.s2.starcat.ne.jp/ndxc/> and imports the text
//! file under Settings → UI; it then lives in [`dir`] as `aoki.txt`.
// Why: HDZ 2026-11-11: "die Datenschutzerklärung braucht bei Katalogen ein
//      aktives Vorgehen"; Aoki "per Import, ohne Download".

/// Where the catalogues live, `None` in the browser build.
#[cfg(not(target_arch = "wasm32"))]
pub fn dir() -> Option<std::path::PathBuf> {
    sdroxide_config::config_dir().ok().map(|d| d.join("catalogs"))
}
#[cfg(target_arch = "wasm32")]
pub fn dir() -> Option<std::path::PathBuf> {
    None
}

/// How many imports this process has made: the app reads Aoki's list anew
/// when it changes, so an import shows within the minute without a restart.
static IMPORTS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// See [`IMPORTS`].
pub fn imports() -> u64 {
    IMPORTS.load(std::sync::atomic::Ordering::Relaxed)
}

/// Whether Aoki's list has been imported (the file is there).
pub fn aoki_imported() -> bool {
    dir().is_some_and(|d| d.join("aoki.txt").is_file())
}

/// Import Aoki's list from the text file the user picked: parsed first, so a
/// wrong file is refused rather than stored, then written to [`dir`] as
/// `aoki.txt`. The number of lines read, or why it was refused.
pub fn import_aoki(bytes: &[u8]) -> Result<usize, String> {
    let n = parse_aoki(bytes).len();
    if n == 0 {
        return Err("not Aoki's list: no FRE/STATION header or no line with a time".into());
    }
    let dir = dir().ok_or("no configuration directory")?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join("aoki.txt");
    std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    IMPORTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    Ok(n)
}

/// One line of Aoki's list in the schedule's own shape.
#[derive(Debug, Clone)]
pub struct AokiEntry {
    pub st: sdroxide_types::BroadcastStation,
    /// Marked `x` in the list: the transmission has ended or moved.
    pub off: bool,
}

/// Aoki's list, `aoki.txt` in [`dir`], or why there is none.
pub fn load_aoki() -> Result<Vec<AokiEntry>, String> {
    let dir = dir().ok_or("no configuration directory")?;
    let path = dir.join("aoki.txt");
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let list = parse_aoki(&bytes);
    if list.is_empty() {
        return Err(format!("{}: no entries", path.display()));
    }
    Ok(list)
}

/// Parse Aoki's fixed-width list (`xzXNN.txt`).
///
/// The column starts are read off the header line (`FRE STATION UTC Su-W-Sa
/// Language Pow Azi Location ADM L/L Remarks`). The widths count bytes, and
/// the file is Latin-1 with the odd UTF-8 name, so each field is cut as bytes
/// and decoded on its own. A few lines are shifted by a column in the source;
/// the time (`HHMM-HHMM`) is looked for near its column and the fields after
/// it follow its shift. A line with no time is skipped. Days count from
/// Sunday = 1 there and are turned into the schedules' Monday = 1.
pub fn parse_aoki(bytes: &[u8]) -> Vec<AokiEntry> {
    let lines: Vec<&[u8]> = bytes.split(|b| *b == b'\n').collect();
    let Some(header) = lines
        .iter()
        .map(|l| sdroxide_types::broadcast::decode_latin1(l))
        .find(|l| l.starts_with("FRE"))
    else {
        return Vec::new();
    };
    let names = [
        "STATION", "UTC", "Su-W-Sa", "Language", "Pow", "Azi", "Location", "ADM", "L/L", "Remarks",
    ];
    let Some(pos) = names.iter().map(|n| header.find(n)).collect::<Option<Vec<usize>>>() else {
        return Vec::new();
    };
    let field = |l: &[u8], i: usize, shift: isize| -> String {
        let a = (pos[i] as isize + shift).max(0) as usize;
        let b = pos.get(i + 1).map_or(l.len(), |p| (*p as isize + shift).max(0) as usize);
        let raw = l.get(a.min(l.len())..b.min(l.len())).unwrap_or(&[]);
        match std::str::from_utf8(raw) {
            Ok(s) => s.trim().to_string(),
            Err(_) => sdroxide_types::broadcast::decode_latin1(raw).trim().to_string(),
        }
    };
    let is_time = |w: &[u8]| {
        w.len() == 9 && w[4] == b'-' && w[..4].iter().chain(&w[5..]).all(u8::is_ascii_digit)
    };
    let mut out = Vec::new();
    for l in lines {
        let l = l.strip_suffix(b"\r").unwrap_or(l);
        if l.len() < pos[1] + 9 {
            continue;
        }
        let Ok(khz) = std::str::from_utf8(&l[..5]).unwrap_or("").trim().parse::<f64>() else {
            continue;
        };
        let at = pos[1];
        let Some(shift) = (-3isize..=3).find(|d| {
            let a = (at as isize + d) as usize;
            l.get(a..a + 9).is_some_and(is_time)
        }) else {
            continue;
        };
        let t = &l[(at as isize + shift) as usize..(at as isize + shift) as usize + 9];
        let hhmm = |b: &[u8]| std::str::from_utf8(b).ok().and_then(|s| s.parse::<u16>().ok());
        let (start, end) = (hhmm(&t[..4]), hhmm(&t[5..]));
        let days_raw = field(l, 2, shift);
        let mut days: Vec<char> = days_raw
            .chars()
            .filter_map(|c| c.to_digit(10))
            .filter(|d| (1..=7).contains(d))
            .map(|d| char::from_digit(if d == 1 { 7 } else { d - 1 }, 10).unwrap_or('7'))
            .collect();
        days.sort_unstable();
        days.dedup();
        let days: String = if days.len() == 7 { String::new() } else { days.into_iter().collect() };
        let name = field(l, 0, 0);
        let (lat, lon) = parse_aoki_latlon(&field(l, 8, shift)).unzip();
        let remark = field(l, 9, shift);
        let season = remark
            .split_whitespace()
            .find(|w| {
                w.len() == 3
                    && (w.starts_with('a') || w.starts_with('b'))
                    && w[1..].chars().all(|c| c.is_ascii_digit())
            })
            .map(|w| w[..1].to_ascii_uppercase());
        out.push(AokiEntry {
            st: sdroxide_types::BroadcastStation {
                mode: name.contains("DRM").then(|| "DRM".to_string()),
                name,
                freq_khz: khz,
                site: field(l, 6, shift),
                country: field(l, 7, shift),
                lat,
                lon,
                power_kw: field(l, 4, shift).parse().ok(),
                lang: field(l, 3, shift),
                target: String::new(),
                start_utc: start,
                end_utc: end,
                days,
                season,
            },
            off: l.get(5) == Some(&b'x'),
        });
    }
    out
}

/// `361024N1394923E` → (36.17°, 139.82°); west and south negative.
fn parse_aoki_latlon(s: &str) -> Option<(f64, f64)> {
    let b = s.as_bytes();
    if b.len() != 15 {
        return None;
    }
    let num = |r: std::ops::Range<usize>| std::str::from_utf8(&b[r]).ok()?.parse::<f64>().ok();
    let lat = num(0..2)? + num(2..4)? / 60.0 + num(4..6)? / 3600.0;
    let lon = num(7..10)? + num(10..12)? / 60.0 + num(12..14)? / 3600.0;
    let lat = if b[6] == b'S' { -lat } else { lat };
    let lon = if b[14] == b'W' { -lon } else { lon };
    Some((lat, lon))
}

#[cfg(test)]
mod tests {
    use super::*;

    const AOKI: &[u8] = concat!(
        "B26 Shortwave Frequecy List  November 6 2026,  1400 UTC   Day 1 = Sunday\r\n",
        "FRE   STATION                         UTC       Su-W-Sa Language             Pow Azi Location                ADM L/L             Remarks\r\n",
        " 9560 PBS Xinjiang                    0800-0900 12.4567 Chinese              100  ND Urumqi                  TKS 440934N0865357E XJBS a26\r\n",
        " 9570 R.ROMANIA INT. DRM xHE E1E101   1800-1857 1234567 English               90 292 Tiganesti               ROU 444457N0260610E RRO b26\r\n",
        " 5800 WRMI Radio Rep?blica           0200-0300  3456   English              100 160 Okeechobee10            USA 272726N0805559W RMI a26\r\n",
        " 1485xAomori Hoso (RAB)               0000-2400 1234567 Japanese               1     Aomori-Hachinohe        J   403029N1412811E JOGO'26Feb.8-off\r\n",
    )
    .as_bytes();

    #[test]
    fn aokis_list_is_read_column_by_column() {
        let list = parse_aoki(AOKI);
        assert_eq!(list.len(), 4, "{list:#?}");
        let x = &list[0].st;
        assert_eq!(
            (x.freq_khz, x.name.as_str(), x.start_utc, x.end_utc),
            (9560.0, "PBS Xinjiang", Some(800), Some(900))
        );
        // "12.4567" counts from Sunday = 1, so the dot is Tuesday; counted
        // from Monday = 1 that is every day but 2.
        assert_eq!(x.days, "134567");
        assert_eq!(
            (x.lang.as_str(), x.power_kw, x.site.as_str(), x.country.as_str()),
            ("Chinese", Some(100.0), "Urumqi", "TKS")
        );
        let (lat, lon) = (x.lat.unwrap(), x.lon.unwrap());
        assert!((lat - 44.1594).abs() < 0.001 && (lon - 86.8992).abs() < 0.001, "{lat} {lon}");
        assert_eq!(x.season.as_deref(), Some("A"));
        assert_eq!(list[1].st.mode.as_deref(), Some("DRM"));
        assert_eq!(list[1].st.days, "", "every day is daily");
        // Shifted a column in the source: the fields after the time follow it.
        let w = &list[2].st;
        assert_eq!(
            (w.start_utc, w.lang.as_str(), w.site.as_str()),
            (Some(200), "English", "Okeechobee10")
        );
        assert!(w.lon.unwrap() < 0.0, "west is negative");
        assert!(list[3].off && !list[0].off);
    }

    #[test]
    fn a_file_that_is_not_aokis_list_reads_as_nothing() {
        assert!(parse_aoki(b"just some text\nwithout a header\n").is_empty());
        assert!(parse_aoki(b"").is_empty());
    }
}
