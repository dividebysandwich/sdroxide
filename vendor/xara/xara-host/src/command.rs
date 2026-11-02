#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bw {
    Hz500,
    Hz2300,
    Hz2750,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Comp {
    Off,
    Text,
    Files,
}

pub const DRIVE_MIN_DB: f64 = -30.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drive {
    Db(i32),
    OutOfRange,
    NotNumeric,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Invalid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Connect { mycall: String, remote: String },
    ConnectVia { mycall: String, remote: String, vias: Vec<String> },
    Mycall(Result<Vec<String>, Invalid>),
    Gateway,
    CqFrame { call: String, bw: Option<String> },
    Disconnect,
    Abort,
    Chat(bool),
    WinlinkSession,
    P2pSession,
    Retries(Result<u8, Invalid>),
    Listen(bool),
    Public(bool),
    Cwid(bool),
    Compression(Comp),
    IgnoreKissDcd(bool),
    Bw(Bw),
    TuneOff,
    TuneQuery,
    Tune(Drive),
    DriveLevel(Drive),
    CleanTxBuffer,
    Version,
}

pub fn fix_suffix(t: &str) -> String {
    match t.as_bytes() {
        b if b.len() == 8 && !b.contains(&b'-') && b"RTXL".contains(&b[7]) => format!("{}-{}", &t[..7], &t[7..]),
        _ => t.to_string(),
    }
}

pub fn valid_call(t: &str) -> Option<String> {
    let u = t.to_ascii_uppercase();
    let (base, suffix) = match u.find('-') {
        Some(i) => (&u[..i], Some(&u[i + 1..])),
        None => (&u[..], None),
    };
    if !(1..=7).contains(&base.len()) || !base.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()) {
        return None;
    }
    let ok = match suffix.map(str::as_bytes) {
        None => true,
        Some([c]) => b"TRXL".contains(c) || (b'1'..=b'9').contains(c),
        Some([a, b]) => a.is_ascii_digit() && b.is_ascii_digit() && (1..=19).contains(&((a - b'0') * 10 + b - b'0')),
        Some(_) => false,
    };
    ok.then_some(u)
}

fn call(t: &str) -> Option<String> {
    valid_call(&fix_suffix(t))
}

fn vb_val(s: &str) -> f64 {
    vb_num(s).unwrap_or(0.0)
}

fn vb_num(s: &str) -> Option<f64> {
    let s: String = s.chars().filter(|c| !matches!(c, '\t' | '\n' | '\r' | ' ' | '\u{3000}')).collect();
    let u = s.to_ascii_uppercase();
    if let Some(r) = u.strip_prefix("&H") {
        return vb_radix(r, 16, 4);
    }
    if let Some(r) = u.strip_prefix("&O") {
        return vb_radix(r, 8, 6);
    }
    let b = s.as_bytes();
    let digits = |i: usize| b[i..].iter().take_while(|c| c.is_ascii_digit()).count();
    let mut i = usize::from(matches!(b.first(), Some(b'-' | b'+')));
    let int = digits(i);
    i += int;
    let mut frac = 0;
    if b.get(i) == Some(&b'.') {
        frac = digits(i + 1);
        i += 1 + frac;
    }
    if int + frac == 0 {
        return None;
    }
    if matches!(b.get(i), Some(b'D' | b'd' | b'E' | b'e')) {
        let j = i + 1 + usize::from(matches!(b.get(i + 1), Some(b'-' | b'+')));
        let n = digits(j);
        if n > 0 {
            return format!("{}e{}", &s[..i], &s[i + 1..j + n]).parse().ok();
        }
    }
    s[..i].parse().ok()
}

fn vb_radix(s: &str, radix: u32, short: usize) -> Option<f64> {
    let n = s.chars().take_while(|c| c.is_digit(radix)).count();
    let v = u32::from_str_radix(&s[..n], radix).ok()?;
    Some(if n <= short && v <= 0xffff { f64::from(v as u16 as i16) } else { f64::from(v as i32) })
}

fn drive(rest: &str) -> Drive {
    match vb_num(rest) {
        None => Drive::NotNumeric,
        Some(v) if (DRIVE_MIN_DB..=0.0).contains(&v) => Drive::Db(v.round_ties_even() as i32),
        Some(_) => Drive::OutOfRange,
    }
}

fn cbyte(v: f64) -> Result<u8, Invalid> {
    let r = v.round_ties_even();
    (0.0..=255.0).contains(&r).then_some(r as u8).ok_or(Invalid)
}

fn exact(line: &str) -> Option<Command> {
    use Command::*;
    Some(match line {
        "DISCONNECT" => Disconnect,
        "ABORT" => Abort,
        "LISTEN CQ" | "CHAT ON" => Chat(true),
        "CHAT OFF" => Chat(false),
        "WINLINK SESSION" => WinlinkSession,
        "P2P SESSION" => P2pSession,
        "LISTEN ON" => Listen(true),
        "LISTEN OFF" => Listen(false),
        "PUBLIC ON" => Public(true),
        "PUBLIC OFF" => Public(false),
        "CWID ON" => Cwid(true),
        "CWID OFF" => Cwid(false),
        "COMPRESSION OFF" => Compression(Comp::Off),
        "COMPRESSION ON" | "COMPRESSION TEXT" => Compression(Comp::Text),
        "COMPRESSION FILES" => Compression(Comp::Files),
        "IGNOREKISSDCD ON" => IgnoreKissDcd(true),
        "IGNOREKISSDCD OFF" => IgnoreKissDcd(false),
        "BW2300" => Bw(self::Bw::Hz2300),
        "BW2750" => Bw(self::Bw::Hz2750),
        "BW500" => Bw(self::Bw::Hz500),
        "CLEANTXBUFFER" => CleanTxBuffer,
        "VERSION" => Version,
        _ => return None,
    })
}

pub fn parse(line: &str) -> Option<Command> {
    if line.starts_with("CONNECT ") {
        let t: Vec<&str> = line.split(' ').collect();
        return match t.len() {
            3 => Some(Command::Connect { mycall: call(t[1])?, remote: call(t[2])? }),
            5 | 6 if t[3] == "VIA" => Some(Command::ConnectVia {
                mycall: call(t[1])?,
                remote: call(t[2])?,
                vias: t[4..].iter().map(|v| call(v)).collect::<Option<_>>()?,
            }),
            _ => None,
        };
    }
    if line.starts_with("MYCALL ") {
        let t: Vec<&str> = line.split(' ').skip(1).collect();
        if t[0].is_empty() || t.len() > 5 {
            return Some(Command::Mycall(Err(Invalid)));
        }
        return Some(Command::Mycall(t.iter().map(|c| call(c)).collect::<Option<_>>().ok_or(Invalid)));
    }
    if line.starts_with("GATEWAY ") {
        return Some(Command::Gateway);
    }
    if line.starts_with("CQFRAME ") {
        let t: Vec<&str> = line.split(' ').collect();
        if !(2..=3).contains(&t.len()) || t.last().is_some_and(|w| w.is_empty()) {
            return None;
        }
        return Some(Command::CqFrame { call: call(t[1])?, bw: t.get(2).map(|s| s.to_string()) });
    }
    if let Some(c) = exact(line) {
        return Some(c);
    }
    if let Some(rest) = line.strip_prefix("RETRIES ") {
        return Some(Command::Retries(cbyte(vb_val(rest))));
    }
    if let Some(rest) = line.strip_prefix("TUNE ") {
        return Some(match rest.trim() {
            a if a.eq_ignore_ascii_case("OFF") => Command::TuneOff,
            "?" => Command::TuneQuery,
            _ => Command::Tune(drive(rest)),
        });
    }
    if let Some(rest) = line.strip_prefix("DRIVELEVEL ") {
        return Some(Command::DriveLevel(drive(rest)));
    }
    None
}
