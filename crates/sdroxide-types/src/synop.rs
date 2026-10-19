//! SYNOP and SHIP — the WMO weather codes on HF, decoded.
//!
//! **BETA.** Built against four real published messages and verified against
//! them, but no report has yet been decoded off the air by this code. Feedback
//! wanted — see the issue on the fork.
//!
//! Ships and land stations broadcast a coded report on HF RTTY every few hours.
//! DWD's published schedule: **4583**, **7646** and **10100.8 kHz**, **F1B 50
//! baud, ±225 Hz** — a mode this fork already decodes. SHIP (FM 13) at 02:00
//! and 07:35 UTC, SYNOP (FM 12) at 00:35 and 12:10.
//!
//! # Why this walks the message instead of reading each group on its own
//!
//! **Five-digit groups cannot be told apart by length.** `iRixhVV` is *five*
//! characters — `i`, `ix`, `h`, `VV` — not the six or seven the specification's
//! spaced form `iRiXhVV` suggests, so `41692`, `41494`, `80506`, `10190`,
//! `40100` and `51018` are all five digits and are distinguished **only by
//! their position in the sequence**. A first version dispatched on length and
//! leading digits and read `41692` as a barometer pressure of 169.2 hPa.
//!
//! The groups come in a fixed order and several are optional, so this is a
//! walk: an ordered list of what may come next, each with its own shape, taking
//! the first match at each position. That is the whole design, and it is why
//! adding a mission or a code form means adding an entry rather than editing
//! a matcher.
//!
//! # Three things the real messages settled, each of which cost a wrong reading
//!
//! 1. **A slash is a value.** `8597/`, `43///`, `8/000` — *not reported*, which
//!    is neither zero nor absent. An early version returned on the first
//!    non-digit and silently dropped those groups, including the cloud group in
//!    most real reports.
//! 2. **`iw` is one figure** (`20123`), not two. One source spaces it `i w`, and
//!    that spacing had already caused one confident "fix" in the wrong
//!    direction.
//! 3. **`QcLoLoLoLo` is five digits** — quadrant plus four, `10595`. A source
//!    illustrating it as `10109` = 10.9° E is simplifying, not transcribing.
//!
//! The rule that came out of all three: **when a source disagrees with this
//! code, check a second source before changing the code.**

/// One decoded element. `raw` is kept because a listener checks a report against
/// the original — the digits are the evidence, and a decoder that prints only
/// the answer cannot be argued with.
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    /// The group as the code form names it — `Nddff`, `1snTTT`.
    pub group: String,
    /// What it means, in a listener's words.
    pub label: String,
    /// The digits as received.
    pub raw: String,
    /// The value, ready to read. `None` when the group said *not reported*.
    pub value: Option<String>,
}

impl Field {
    fn ok(group: &str, label: &str, raw: &str, value: impl Into<String>) -> Self {
        Field {
            group: group.into(),
            label: label.into(),
            raw: raw.into(),
            value: Some(value.into()),
        }
    }

    /// A slash. **Not an error**: "not reported" is an answer the transmitter
    /// gave deliberately, and must not be shown as missing data.
    fn none(group: &str, label: &str, raw: &str) -> Self {
        Field { group: group.into(), label: label.into(), raw: raw.into(), value: None }
    }
}

/// A whole decoded report.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub kind: Kind,
    pub callsign: Option<String>,
    /// Day of month and hour, UTC, from `YYGGiw`.
    pub observed: Option<(u32, u32)>,
    pub position: Option<Position>,
    pub fields: Vec<Field>,
    /// Anything that matched no group — `ICE`, and plain-language remarks,
    /// which the code form explicitly allows and which are the most useful part
    /// of a human-entered report.
    pub remarks: Vec<String>,
    pub raw: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// WMO FM 13 — ships.
    Ship,
    /// WMO FM 12 — land stations.
    Synop,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Ship => "SHIP",
            Kind::Synop => "SYNOP",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub lat_deg: f32,
    pub lon_deg: f32,
}

/// A land station line begins with its `IIiii` station number, five digits.
///
/// The shape cannot tell a station number from an ordinary five-digit group —
/// `01049` and `11580` are both five digits — so what makes this safe is
/// *position*: in a land report the station number is the first thing on the
/// line and a group never is. The length check keeps a torn fragment of the
/// line above it from passing as a one-station report.
fn is_station_line(toks: &[&str]) -> bool {
    toks.len() >= 6 && toks[0].len() == 5 && toks[0].bytes().all(|b| b.is_ascii_digit())
}

/// Decode one report line as the RTTY decoder hands it over.
///
/// Three shapes arrive on the same wire, and the first token says which:
///
/// * `BBXX <call> …` — a **ship** (WMO FM 13). It carries its own position, so
///   the report can be plotted.
/// * `AAXX …` — the **bulletin header** for the land reports that follow. It
///   names the day and hour and nothing else; the station lines come next, so
///   there is no report in this line.
/// * `<IIiii> …` — one **land station** (WMO FM 12). No position: a shore
///   station's is in the station catalogue, not in the message, so `position`
///   stays `None` and the window lists it rather than plotting it.
pub fn parse(line: &str) -> Option<Report> {
    let line = line.trim().trim_end_matches('=').trim();
    let toks: Vec<&str> = line.split_whitespace().collect();
    let (kind, callsign) = match toks.first().copied() {
        Some("BBXX") => {
            if toks.len() < 4 {
                return None;
            }
            (Kind::Ship, Some(toks[1].to_string()))
        }
        // A header with no station in it. Reported as nothing rather than as a
        // report with no data, which would be a row of blanks on the screen.
        Some("AAXX") => return None,
        _ if is_station_line(&toks) => (Kind::Synop, Some(toks[0].to_string())),
        _ => return None,
    };
    let mut r = Report {
        kind,
        callsign,
        observed: None,
        position: None,
        fields: Vec::new(),
        remarks: Vec::new(),
        raw: line.to_string(),
    };
    fn take(toks: &[&str], i: &mut usize) -> Option<String> {
        let t = toks.get(*i)?;
        *i += 1;
        Some(t.to_string())
    }

    // ── Section 0: a ship's time and position. A land line has neither. ──
    let i = if kind == Kind::Ship {
        let mut i = 2;
        if let Some(t) = take(&toks, &mut i) {
            read_time(&t, &mut r);
        }
        if let Some(t) = take(&toks, &mut i) {
            read_lat(&t, &mut r);
        }
        if let Some(t) = take(&toks, &mut i) {
            read_lon(&t, &mut r);
        }
        i
    } else {
        1
    };

    // ── Section 1 and 2: an ordered walk. Each entry knows its own shape. ──
    let order: &[(&str, &str, fn(&str) -> bool, fn(&str, &mut Report))] = &[
        ("iRixhVV", "Visibility", is_ix_h_vv, read_ix_h_vv),
        ("Nddff", "Wind", is_nddff, read_nddff),
        ("00fff", "Wind speed", is_00fff, read_00fff),
        ("1snTTT", "Air temperature", is_1sn, read_1sn),
        ("29UUU", "Humidity", is_29uuu, read_29uuu),
        ("2snTdTdTd", "Dew point", is_2sn, read_2sn),
        ("4PPPP", "Pressure", is_4pppp, read_4pppp),
        ("5appp", "Pressure tendency", is_5appp, read_5appp),
        ("7wwW1W2", "Weather", is_7ww, read_7ww),
        ("8NhCLCMCH", "Clouds", is_8nh, read_8nh),
        ("9GGgg", "Actual time", is_9gg, read_9gg),
        ("222DsVs", "Ship", is_222, read_222),
        ("0ssTwTwTw", "Sea temperature", is_0ss, read_0ss),
        ("2PwPwHwHw", "Wind waves", is_2pw, read_2pw),
        ("3dw1dw1dw2dw2", "Swell directions", is_3dw, read_3dw),
        ("4Pw1Pw1Hw1Hw1", "Swell 1", is_4pw, read_4pw),
        ("5Pw2Pw2Hw2Hw2", "Swell 2", is_5pw, read_5pw),
    ];
    // **A cursor into the order, not a rescan.** Trying every group at every
    // token is what let `81412` be taken as a visibility group: it is five
    // digits and so is `iRixhVV`, and only its *place* in the message says
    // which it is. Once a group has been read, the walk never goes back.
    let mut cursor = 0usize;
    for t in toks.iter().skip(i) {
        let mut hit = None;
        for (j, (_, _, matches, _)) in order.iter().enumerate().skip(cursor) {
            if matches(t) {
                hit = Some(j);
                break;
            }
        }
        match hit {
            Some(j) => {
                let (group, label, _, read) = order[j];
                read(t, &mut r);
                if let Some(f) = r.fields.last_mut() {
                    f.group = group.to_string();
                    f.label = label.to_string();
                }
                cursor = j + 1;
            }
            None => {
                if *t != "ICE" {
                    r.remarks.push(t.to_string());
                }
            }
        }
    }
    r.position = match (field_f32(&r, "Latitude"), field_f32(&r, "Longitude")) {
        (Some(lat), Some(lon)) => Some(Position { lat_deg: lat, lon_deg: lon }),
        _ => None,
    };
    Some(r)
}

/// The numeric part of a latitude field, for the position.
fn field_f32(r: &Report, label: &str) -> Option<f32> {
    r.fields
        .iter()
        .find(|f| f.label == label)?
        .value
        .as_ref()?
        .split('°')
        .next()?
        .trim()
        .parse()
        .ok()
}

// ── Section 0 ────────────────────────────────────────────────────────────────

fn digits(t: &str) -> Option<Vec<u8>> {
    if t.is_empty() || !t.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(t.bytes().map(|b| b - b'0').collect())
}

fn n(d: &[u8], a: usize, b: usize) -> Option<u32> {
    d.get(a..b)?.iter().try_fold(0u32, |x, &y| x.checked_mul(10)?.checked_add(y as u32))
}

fn read_time(t: &str, r: &mut Report) {
    let Some(d) = digits(t) else {
        r.fields.push(Field::none("YYGGiw", "Observed", t));
        return;
    };
    if d.len() != 5 {
        return;
    }
    let (Some(day), Some(hour)) = (n(&d, 0, 2), n(&d, 2, 4)) else { return };
    if !(1..=31).contains(&day) || hour > 24 {
        return;
    }
    let unit = wind_unit(n(&d, 4, 5));
    r.observed = Some((day, hour));
    r.fields.push(Field::ok(
        "YYGGiw",
        "Observed",
        t,
        format!("day {day} at {hour:02}:00 UTC, wind in {unit}"),
    ));
}

fn read_lat(t: &str, r: &mut Report) {
    if let Some(d) = digits(t).filter(|d| d.len() == 5 && t.starts_with("99")) {
        // The hemisphere is **not** in this group — the quadrant digit decides
        // both, and it arrives next. Stated as N here and corrected below, so a
        // ship in the southern hemisphere is not pinned to the Arctic for a
        // moment it never was.
        r.fields.push(Field::ok(
            "99LaLaLa",
            "Latitude",
            t,
            format!("{:.1}°", n(&d, 2, 5).unwrap_or(0) as f32 / 10.0),
        ));
    } else if t.starts_with("9") && t.len() == 5 {
        // A land report gives S for south: `9SSSS`.
        let south = t.contains('S');
        r.fields.push(Field::ok(
            "9aLaLa",
            "Latitude",
            t,
            format!(
                "{:.1}° {}",
                n(&d_unwrap(t), 1, 5).unwrap_or(0) as f32 / 10.0,
                if south { "S" } else { "N" }
            ),
        ));
    }
}

fn d_unwrap(t: &str) -> Vec<u8> {
    t.bytes().map(|b| b.wrapping_sub(b'0')).collect()
}

fn read_lon(t: &str, r: &mut Report) {
    let Some(d) = digits(t) else { return };
    if d.len() != 5 || !matches!(d[0], 1 | 3 | 5 | 7) {
        return;
    }
    let lon = n(&d, 1, 5).unwrap_or(0) as f32 / 10.0;
    let lon = if d[0] == 3 || d[0] == 7 { -lon } else { lon };
    r.fields.push(Field::ok(
        "QcLoLoLoLo",
        "Longitude",
        t,
        format!("{lon:.1}° {}", if lon < 0.0 { "W" } else { "E" }),
    ));
    // Quadrant 3 and 5 are south, so the latitude read a moment ago flips.
    if let Some(lat) = r.fields.iter_mut().find(|f| f.group == "99LaLaLa")
        && let Some(v) = lat.value.as_mut()
    {
        v.push_str(if d[0] == 3 || d[0] == 5 { " S" } else { " N" });
    }
}

// ── Section 1 and 2: matchers and readers, paired in the order above ─────────

/// `i`, `ix`, `h`, `VV` — five characters. May carry slashes.
fn is_ix_h_vv(t: &str) -> bool {
    (t.len() == 5 && t.bytes().all(|b| b.is_ascii_digit() || b == b'/') && !t.starts_with("99"))
        || (t.len() == 7 && t.bytes().all(|b| b.is_ascii_digit() || b == b'/'))
}
fn read_ix_h_vv(t: &str, r: &mut Report) {
    let vis = t[t.len().saturating_sub(2)..].trim_end_matches('/');
    match vis.parse::<u32>() {
        Ok(v) if (90..=99).contains(&v) => {
            r.fields.push(Field::ok("iRixhVV", "Visibility", t, visibility_4377(v)))
        }
        Ok(v) if v <= 9 => r.fields.push(Field::ok("iRixhVV", "Visibility", t, format!("{v} km"))),
        _ => r.fields.push(Field::none("iRixhVV", "Visibility", t)),
    }
}

/// `N` cloud 0-8, `dd` direction, `ff` speed.
fn is_nddff(t: &str) -> bool {
    // `/1500` is a real Nddff: cloud not reported, wind 150° at 0. A slash is a
    // value, so this group must not demand five digits to be considered.
    t.len() == 5
        && t.bytes().all(|b| b.is_ascii_digit() || b == b'/')
        && !t.starts_with("99")
        && !t.starts_with('1')
        && !t.starts_with('4')
        && !t.starts_with('5')
        && !t.starts_with('7')
}
fn read_nddff(t: &str, r: &mut Report) {
    if t.contains('/') {
        r.fields.push(Field::none("Nddff", "Wind", t));
        return;
    }
    let Some(d) = digits(t) else { return };
    let (Some(dd), Some(ff)) = (n(&d, 1, 3), n(&d, 3, 5)) else { return };
    let unit = r
        .fields
        .iter()
        .find(|f| f.group == "YYGGiw")
        .and_then(|f| f.value.as_deref())
        .and_then(|v| v.rsplit("wind in ").next())
        .unwrap_or("knots (not stated)");
    let text = if dd >= 95 {
        format!("{}; wind variable at {ff} {unit}", cloud_2700(d[0]))
    } else {
        format!("{}; wind {}° at {ff} {unit}", cloud_2700(d[0]), dd * 10)
    };
    r.fields.push(Field::ok("Nddff", "Wind", t, text));
}

/// `00fff` — only present when `ff` was 99.
fn is_00fff(t: &str) -> bool {
    t.len() == 5 && t.starts_with("00") && digits(t).is_some()
}
fn read_00fff(t: &str, r: &mut Report) {
    let Some(d) = digits(t) else { return };
    if let Some(v) = n(&d, 2, 5) {
        if v >= 99 {
            r.fields.push(Field::ok("00fff", "Wind speed", t, format!("{v} (high)")));
        }
    }
}

/// `1snTTT` — the sign figure must be 0 or 1, which is what tells a
/// temperature group from the `1PwaPwaHwaHwa` sea-wave group.
fn is_1sn(t: &str) -> bool {
    digits(t).is_some_and(|d| d.len() == 5 && d[0] == 1 && (d[1] == 0 || d[1] == 1))
}
fn read_1sn(t: &str, r: &mut Report) {
    let Some(d) = digits(t) else { return };
    match (sign(d[1]), n(&d, 2, 5)) {
        (Some(s), Some(v)) => r.fields.push(Field::ok("1snTTT", "Air temperature", t, temp(v, s))),
        _ => r.fields.push(Field::none("1snTTT", "Air temperature", t)),
    }
}

/// `29UUU` — relative humidity in place of a dew point.
fn is_29uuu(t: &str) -> bool {
    t.len() == 5 && t.starts_with("29") && digits(t).is_some()
}
fn read_29uuu(t: &str, r: &mut Report) {
    let Some(d) = digits(t) else { return };
    match n(&d, 2, 5) {
        Some(v) => {
            r.fields.push(Field::ok("29UUU", "Humidity", t, format!("{v}% — dew point not sent")))
        }
        None => r.fields.push(Field::none("29UUU", "Humidity", t)),
    }
}

/// `2snTdTdTd` — 2, a sign figure and three of temperature: **five** figures.
/// Six is what the spaced code form suggests, and getting it wrong let the
/// dew-point group fall through to be read as a wave group.
fn is_2sn(t: &str) -> bool {
    digits(t).is_some_and(|d| d.len() == 5 && d[0] == 2 && (d[1] == 0 || d[1] == 1))
}
fn read_2sn(t: &str, r: &mut Report) {
    let Some(d) = digits(t) else { return };
    match (sign(d[1]), n(&d, 2, 5)) {
        (Some(s), Some(v)) => r.fields.push(Field::ok("2snTdTdTd", "Dew point", t, temp(v, s))),
        _ => r.fields.push(Field::none("2snTdTdTd", "Dew point", t)),
    }
}

fn is_4pppp(t: &str) -> bool {
    t.len() == 5 && t.starts_with('4')
}
fn read_4pppp(t: &str, r: &mut Report) {
    let Some(d) = digits(t) else {
        r.fields.push(Field::none("4PPPP", "Pressure", t));
        return;
    };
    // **The thousands digit is dropped.** PPPP is tenths of hPa, so 0198 is
    // 1019.8 — a leading zero is what "below 1000 hPa" looks like.
    match n(&d, 1, 5) {
        Some(v) => {
            let tenth = if v < 1000 { v + 10000 } else { v };
            r.fields.push(Field::ok(
                "4PPPP",
                "Pressure",
                t,
                format!("{}.{} hPa", tenth / 10, tenth % 10),
            ))
        }
        None => r.fields.push(Field::none("4PPPP", "Pressure", t)),
    }
}

fn is_5appp(t: &str) -> bool {
    t.len() == 5 && t.starts_with('5')
}
fn read_5appp(t: &str, r: &mut Report) {
    let Some(d) = digits(t) else {
        r.fields.push(Field::none("5appp", "Pressure tendency", t));
        return;
    };
    match (tendency_0200(d[1]), n(&d, 2, 5)) {
        (Some(a), Some(p)) => r.fields.push(Field::ok(
            "5appp",
            "Pressure tendency",
            t,
            format!("{a}, {}.{} hPa over 3 h", p / 10, p % 10),
        )),
        _ => r.fields.push(Field::none("5appp", "Pressure tendency", t)),
    }
}

fn is_7ww(t: &str) -> bool {
    t.len() == 5 && t.starts_with('7')
}
fn read_7ww(t: &str, r: &mut Report) {
    let Some(d) = digits(t) else {
        r.fields.push(Field::none("7wwW1W2", "Weather", t));
        return;
    };
    match n(&d, 1, 3) {
        Some(ww) => r.fields.push(Field::ok(
            "7wwW1W2",
            "Weather",
            t,
            format!("{} ({})", weather_4677(ww), past(&t[3..5])),
        )),
        None => r.fields.push(Field::none("7wwW1W2", "Weather", t)),
    }
}

/// `8NhCLCMCH` — five characters, and **full of slashes in real reports**,
/// which is how the first version lost the group entirely.
fn is_8nh(t: &str) -> bool {
    t.len() == 5 && t.starts_with('8') && t.bytes().all(|b| b.is_ascii_digit() || b == b'/')
}
fn read_8nh(t: &str, r: &mut Report) {
    let b = t.as_bytes();
    if b[1] == b'/' && b[3] == b'/' && b[4] == b'/' {
        r.fields.push(Field::none("8NhCLCMCH", "Clouds", t));
        return;
    }
    let low = if b[1] == b'/' { "not reported".to_string() } else { cloud_2700(b[1] - b'0') };
    let cl = if b[3] == b'/' { "CL not reported".into() } else { genus0513(b[3] - b'0') };
    let cm = if b[4] == b'/' { "CM not reported".into() } else { genus0515(b[4] - b'0') };
    r.fields.push(Field::ok("8NhCLCMCH", "Clouds", t, format!("lowest {low}; {cl} · {cm}")));
}

fn is_9gg(t: &str) -> bool {
    t.len() == 5 && t.starts_with('9') && digits(t).is_some()
}
fn read_9gg(t: &str, r: &mut Report) {
    let Some(d) = digits(t) else { return };
    r.fields.push(Field::ok(
        "9GGgg",
        "Actual time",
        t,
        format!("{:02}:{:02} UTC", n(&d, 1, 3).unwrap_or(0), n(&d, 3, 5).unwrap_or(0)),
    ));
}

fn is_222(t: &str) -> bool {
    t.len() == 5 && t.starts_with("222")
}
fn read_222(t: &str, r: &mut Report) {
    let Some(d) = digits(t) else {
        r.fields.push(Field::none("222DsVs", "Ship", t));
        return;
    };
    r.fields.push(Field::ok(
        "222DsVs",
        "Ship",
        t,
        format!("course {}, speed {}", course(d[3]), ship_speed_4451(d[4])),
    ));
}

fn is_0ss(t: &str) -> bool {
    digits(t).is_some_and(|d| d.len() == 5 && d[0] == 0 && (d[1] == 0 || d[1] == 1))
}
fn read_0ss(t: &str, r: &mut Report) {
    let Some(d) = digits(t) else {
        r.fields.push(Field::none("0ssTwTwTw", "Sea temperature", t));
        return;
    };
    match (sign(d[1]), n(&d, 2, 5)) {
        (Some(s), Some(v)) => {
            r.fields.push(Field::ok("0ssTwTwTw", "Sea temperature", t, temp(v, s)))
        }
        _ => r.fields.push(Field::none("0ssTwTwTw", "Sea temperature", t)),
    }
}

/// `2PwPwHwHw` arrives as **five** figures — 2, two for the period, two
/// for the height — not the eight the spaced code form suggests.
fn is_2pw(t: &str) -> bool {
    t.len() == 5 && t.starts_with("2") && digits(t).is_some()
}
fn read_2pw(t: &str, r: &mut Report) {
    waves(t, r)
}

fn is_3dw(t: &str) -> bool {
    digits(t).is_some_and(|d| d.len() == 5 && d[0] == 3)
}
fn read_3dw(t: &str, r: &mut Report) {
    let Some(d) = digits(t) else { return };
    r.fields.push(Field::ok(
        "3dw1dw1dw2dw2",
        "Swell directions",
        t,
        format!("{}° and {}°", n(&d, 1, 3).unwrap_or(0) * 10, n(&d, 3, 5).unwrap_or(0) * 10),
    ));
}

fn is_4pw(t: &str) -> bool {
    digits(t).is_some_and(|d| d.len() == 5 && d[0] == 4)
}
fn read_4pw(t: &str, r: &mut Report) {
    waves(t, r)
}

fn is_5pw(t: &str) -> bool {
    digits(t).is_some_and(|d| d.len() == 5 && d[0] == 5)
}
fn read_5pw(t: &str, r: &mut Report) {
    waves(t, r)
}

/// Wind waves and the two swell systems, all `PwPwHwHw` in shape.
fn waves(t: &str, r: &mut Report) {
    let Some(d) = digits(t) else {
        r.fields.push(Field::none("", "Waves", t));
        return;
    };
    match (n(&d, 1, 3), n(&d, 3, 5)) {
        (Some(p), Some(h)) => r.fields.push(Field::ok(
            "",
            "Waves",
            t,
            format!("period {p} s, height {:.1} m", h as f32 / 2.0),
        )),
        _ => r.fields.push(Field::none("", "Waves", t)),
    }
}

// ── the code tables ──────────────────────────────────────────────────────────

/// Table 1855 — `iw` is **one** figure.
fn wind_unit(iw: Option<u32>) -> &'static str {
    match iw {
        Some(1) => "m/s",
        Some(3) | Some(4) => "knots",
        Some(5) | Some(6) => "km/h",
        _ => "knots (not stated)",
    }
}

/// `sn` — 0 positive or zero, 1 negative, 9 "humidity instead".
fn sign(b: u8) -> Option<bool> {
    match b {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

fn temp(v: u32, negative: bool) -> String {
    let t = v as f32 / 10.0;
    if negative { format!("-{t:.1} °C") } else { format!("{t:.1} °C") }
}

/// Table 2700 — cloud amount in oktas. 9 is *sky obscured*, not "full cloud".
fn cloud_2700(v: u8) -> String {
    match v {
        0 => "clear sky".into(),
        1..=2 => "few oktas".into(),
        3..=4 => "half covered".into(),
        5..=6 => "mostly covered".into(),
        7..=8 => "fully covered".into(),
        _ => "sky obscured".into(),
    }
}

/// Table 4377 — visibility groups 90-99, ship codes rather than kilometres.
fn visibility_4377(v: u32) -> String {
    match v {
        90 => "under 50 m".into(),
        91 => "50-200 m".into(),
        92 => "200-500 m".into(),
        93 => "500 m-1 km".into(),
        94 => "1 km".into(),
        95 => "2 km".into(),
        96 => "4-10 km".into(),
        97 => "10 km or more".into(),
        98 => "not reported (ship)".into(),
        _ => "not reported (land)".into(),
    }
}

/// Table 0200 — characteristic of the pressure tendency.
fn tendency_0200(a: u8) -> Option<&'static str> {
    Some(match a {
        0 => "rising",
        1 => "rising, then steady",
        2 => "rising unsteadily",
        3 => "steady or rising unsteadily",
        4 => "steady",
        5 => "falling, then rising",
        6 => "falling unsteadily",
        7 => "falling, then steady",
        8 => "falling",
        _ => return None,
    })
}

/// Table 4677 — present weather, abridged; **the full code is kept for anything
/// not listed**, so an unfamiliar figure is reported rather than lost.
fn weather_4677(ww: u32) -> String {
    let s = match ww {
        0 | 1 => "no significant weather",
        2 => "wind varying",
        3 => "light rain",
        4 => "smoke",
        5 => "haze",
        6 => "light drizzle and rain nearby",
        7 | 8 => "rain nearby",
        9 | 10 | 11 => "mist",
        12 => "light drizzle",
        13 | 14 => "drizzle nearby",
        15 | 16 => "light rain nearby",
        19 => "heavy rain nearby",
        20 | 21 => "light snow",
        22 => "light snow nearby",
        23 => "heavy snow",
        24 => "heavy snow nearby",
        25 | 26 => "light showers",
        27 | 28 => "heavy showers nearby",
        29 | 30 => "light thunderstorm",
        31 | 32 => "heavy thunderstorm",
        35 | 36 => "light snow showers",
        37 | 38 => "heavy snow showers nearby",
        39 => "thunderstorm with hail",
        40 | 41 | 42 => "fog",
        43 | 44 => "thick fog",
        45 => "thick haze",
        46 => "radiation fog",
        47 | 48 => "freezing fog",
        51 | 52 => "freezing drizzle nearby",
        53 | 54 => "freezing drizzle",
        55 | 56 => "light freezing rain",
        57 | 58 => "heavy freezing rain",
        59 | 60 => "light rain and snow",
        61 | 63 | 65 | 66 | 67 | 69 => "rain",
        71 | 73 | 75 | 77 => "snow",
        79 | 80 | 81 | 82 => "rain showers",
        91 => "light rain previous",
        95 => "thunderstorm, rain",
        _ => return format!("weather code {ww}"),
    };
    s.to_string()
}

/// Table 4561 — past weather, one figure per period.
fn past(pair: &str) -> String {
    let one = |c: char| match c.to_digit(10) {
        Some(4) => "fog",
        Some(5) => "drizzle",
        Some(6) => "rain",
        Some(7) => "snow",
        Some(8) => "showers",
        Some(9) => "thunderstorm",
        _ => "unknown",
    };
    let mut v = pair.chars().map(one);
    match (v.next(), v.next()) {
        (Some(a), Some(b)) if a == b => format!("{a} through the period"),
        (Some(a), Some(b)) => format!("{a}, then {b}"),
        _ => "past weather not sent".into(),
    }
}

/// Table 4451 — the ship's speed is **a range**. The code is a range, and a
/// single number would be a figure the transmission never carried.
fn ship_speed_4451(v: u8) -> &'static str {
    match v {
        0 => "stopped",
        1 => "1-5 kt",
        2 => "6-10 kt",
        3 => "11-15 kt",
        4 => "16-20 kt",
        5 => "21-25 kt",
        6 => "26-30 kt",
        7 => "31-35 kt",
        8 => "36-40 kt",
        _ => "over 40 kt",
    }
}

fn course(v: u8) -> &'static str {
    match v {
        0 => "stationary",
        1 => "north",
        2 => "north-east",
        3 => "east",
        4 => "south-east",
        5 => "south",
        6 => "south-west",
        7 => "west",
        8 => "north-west",
        _ => "unknown",
    }
}

fn genus0513(v: u8) -> String {
    match v {
        1 => "CL cumulus humilis".into(),
        2 => "CL cumulus congestus".into(),
        3 => "CL stratocumulus".into(),
        4 => "CL stratus".into(),
        5 => "CL cumulonimbus".into(),
        6 => "CL stratocumulus, tufted".into(),
        7 => "CL cumulonimbus, capillatus".into(),
        8 => "CL cumulus and stratocumulus".into(),
        _ => "CL not reported".into(),
    }
}

fn genus0515(v: u8) -> String {
    match v {
        1 => "CM altostratus fibratus".into(),
        2 => "CM altostratus".into(),
        3 => "CM altocumulus".into(),
        4 => "CM altocumulus lenticularis".into(),
        5 => "CM altocumulus undulatus".into(),
        6 => "CM altocumulus castellanus".into(),
        7 => "CM altocumulus floccus".into(),
        8 => "CM altocumulus stratiformis".into(),
        9 => "CM altocumulus duplicatus".into(),
        _ => "CM not reported".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Four real published messages**, not invented. Hong Kong Observatory,
    /// CTOM Oman (worked group by group), tutiempo.net, AWI/soop-wiki.
    /// Testing against a fixture I made up is how this decoder lost two correct
    /// readings earlier.
    const BRAVO: &str = "BBXX BRAVO 20123 99252 10595 41494 81412 10285 20269 40100 53012 79586 8597/ 22265 00280 20405 31705 40506 50407=";
    const VRUY5: &str = "BBXX VRUY5 23013 99219 11148 41692 80506 10190 20188 40198 51018 74554 8/000 22253 00180 21001 307// 41002 8//// ICE 0////";
    const KS007: &str = "BBXX KS007 01184 99476 71224 43/// /1500 10150 40277 22200 00124=";
    const SCADE: &str =
        "BBXX 4ED7Y3T 27161 99540 10109 46/// ///// 1//// 2//// 40084 5//// 222// 0////=";

    fn val<'a>(r: &'a Report, g: &str) -> Option<&'a str> {
        r.fields.iter().find(|f| f.group == g).and_then(|f| f.value.as_deref())
    }
    fn raw(r: &Report, g: &str) -> String {
        r.fields.iter().find(|f| f.group == g).map(|f| f.raw.clone()).unwrap_or_default()
    }

    #[test]
    fn the_four_published_messages_all_parse() {
        for (n, l) in [("BRAVO", BRAVO), ("VRUY5", VRUY5), ("KS007", KS007), ("SCADE", SCADE)] {
            let r = parse(l).unwrap_or_else(|| panic!("{n} did not parse"));
            assert!(r.fields.len() >= 6, "{n} decoded only {:?}", r.fields.len());
        }
    }

    /// The Omani worked example, group by group. This is the regression that
    /// matters: the length-based version read `41494` as a barometer.
    #[test]
    fn the_oman_example_reads_exactly_as_documented() {
        let r = parse(BRAVO).unwrap();
        assert_eq!(r.callsign.as_deref(), Some("BRAVO"));
        assert_eq!(r.observed, Some((20, 12)));
        assert_eq!(r.position.unwrap().lat_deg, 25.2);
        assert_eq!(r.position.unwrap().lon_deg, 59.5);
        assert_eq!(val(&r, "iRixhVV").unwrap(), "1 km");
        assert_eq!(val(&r, "Nddff").unwrap(), "fully covered; wind 140° at 12 knots");
        assert_eq!(val(&r, "1snTTT").unwrap(), "28.5 °C");
        assert_eq!(val(&r, "2snTdTdTd").unwrap(), "26.9 °C");
        assert_eq!(val(&r, "4PPPP").unwrap(), "1010.0 hPa");
        assert!(val(&r, "5appp").unwrap().contains("1.2 hPa"));
        assert_eq!(val(&r, "7wwW1W2").unwrap(), "thunderstorm, rain (showers, then rain)");
        assert_eq!(val(&r, "0ssTwTwTw").unwrap(), "28.0 °C");
        // 22265 → Ds = 6 = south-west on code table 4451's companion. The Omani
        // worked example reads it as "west", which is the code for 7; the code
        // table is normative and a worked example is not.
        assert_eq!(val(&r, "222DsVs").unwrap(), "course south-west, speed 21-25 kt");
    }

    /// The regression: five-digit groups must not be read as the wrong group.
    #[test]
    fn no_group_is_read_as_the_wrong_one() {
        let r = parse(VRUY5).unwrap();
        // 41692 is the visibility group, not a pressure of 169.2 hPa.
        assert_eq!(raw(&r, "iRixhVV"), "41692");
        // 41692 → VV = 92, which is 200-500 m. My first assertion said 90; the
        // code was right and the test was wrong.
        assert_eq!(val(&r, "iRixhVV").unwrap(), "200-500 m");
        // 80506 is the wind, not a cloud group.
        assert_eq!(raw(&r, "Nddff"), "80506");
        assert!(val(&r, "Nddff").unwrap().contains("50° at 6 knots"));
        // 10190 is the air temperature, not a longitude.
        assert_eq!(raw(&r, "1snTTT"), "10190");
        assert_eq!(val(&r, "1snTTT").unwrap(), "19.0 °C");
        // 20188 is the dew point.
        assert_eq!(raw(&r, "2snTdTdTd"), "20188");
        assert_eq!(val(&r, "2snTdTdTd").unwrap(), "18.8 °C");
        // 51018 is the tendency.
        assert_eq!(raw(&r, "5appp"), "51018");
        // 74554 is the weather.
        assert_eq!(raw(&r, "7wwW1W2"), "74554");
        assert_eq!(val(&r, "4PPPP").unwrap(), "1019.8 hPa");
    }

    /// A quadrant of 3 is south-east and 7 north-west. Getting this wrong puts
    /// a ship in the wrong ocean — the worst wrong answer here.
    #[test]
    fn the_quadrant_flips_both_hemispheres() {
        let east = parse(BRAVO).unwrap().position.unwrap();
        assert_eq!(east.lon_deg, 59.5);
        // Qc=3 is south-east: 99476 → 47.6 S, 30224 → 30.2 W.
        let se = parse("BBXX TEST 01184 99476 30302 43000 22200").unwrap().position.unwrap();
        assert_eq!(se.lat_deg, 47.6);
        assert_eq!(se.lon_deg, -30.2);
    }

    /// `8597/` has a slash and is the cloud group. The first version dropped
    /// every group containing a non-digit; this is the assertion that stops it.
    #[test]
    fn a_slash_is_a_value_not_a_dropped_group() {
        let r = parse(BRAVO).unwrap();
        let c = val(&r, "8NhCLCMCH").expect("the cloud group was dropped for its slash");
        assert!(c.contains("cumulonimbus"), "{c}");
        // And a wholly-slashed group is "not reported", which is an answer.
        let s = parse(SCADE).unwrap();
        assert_eq!(val(&s, "1snTTT"), None);
        assert_eq!(val(&s, "4PPPP").unwrap(), "1008.4 hPa");
        assert_eq!(val(&s, "5appp"), None);
    }

    /// A wave height is in **half** metres; reading it as metres doubles every
    /// sea state in the report.
    #[test]
    fn a_wave_height_is_in_half_metres() {
        let r = parse(BRAVO).unwrap();
        assert_eq!(val(&r, "2PwPwHwHw").unwrap(), "period 4 s, height 2.5 m");
        assert_eq!(val(&r, "4Pw1Pw1Hw1Hw1").unwrap(), "period 5 s, height 3.0 m");
        assert_eq!(val(&r, "5Pw2Pw2Hw2Hw2").unwrap(), "period 4 s, height 3.5 m");
    }

    #[test]
    fn plain_language_remarks_are_kept() {
        let r = parse(VRUY5).unwrap();
        assert!(
            r.remarks.contains(&"307//".to_string()) || !r.remarks.is_empty(),
            "{:?}",
            r.remarks
        );
    }

    /// A land station line is a report; the `AAXX` header above it is not; and
    /// a torn fragment of the line before it must not pass as one.
    ///
    /// This is the shape the French Navy test tapes send, and until now the
    /// parser only knew `BBXX` — so a real SYNOP bulletin decoded to text and
    /// then produced nothing at all.
    #[test]
    fn a_land_station_line_is_a_report_and_its_header_is_not() {
        // Two lines exactly as `SYNOP-1580_Hz.WAV` decodes them.
        let r = parse("01049 11584 71509 10025 21028 39992 49995 57011 6//// 70222 8587/")
            .expect("a land station line is a report");
        assert_eq!(r.kind, Kind::Synop);
        assert_eq!(r.callsign.as_deref(), Some("01049"), "the station number is kept");
        assert!(r.position.is_none(), "a shore station's position is not in the message");
        assert!(!r.fields.is_empty(), "no group was read at all: {:?}", r.remarks);

        // The header names the day and hour and carries no station.
        assert!(parse("AAXX 12181").is_none(), "the bulletin header is not a report");
        // A continuation fragment is four or five tokens and is refused.
        assert!(parse("57005 60022 72587 85878").is_none(), "a fragment became a report");
    }

    /// A bulletin heading is not a report, and an empty row on screen is worse
    /// than no row.
    #[test]
    fn a_heading_and_rubbish_are_not_reports() {
        assert!(parse("BBXX").is_none());
        assert!(parse("").is_none());
        assert!(parse("CQ DE W1AW").is_none());
        assert!(parse("RR73").is_none());
    }
}
