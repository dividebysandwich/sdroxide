use crate::bits::{Bits, Reader, vb_val};
use crate::kiss::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Short,
    BadCall(String),
    Format,
}

pub(crate) const UI_CTRL: u8 = 0x03;
pub(crate) const PID_NO_L3: u8 = 0xF0;
pub const CALL_ALPHABET: &[u8; 37] = b" ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AprsType {
    #[default]
    Text = 0,
    Binary = 1,
    Position = 2,
    TimestampedPosition = 3,
    Message = 4,
    ThirdParty = 6,
}

impl AprsType {
    pub const fn code(self) -> u8 {
        self as u8
    }

    const fn from_code(c: u64) -> Option<Self> {
        Some(match c {
            0 => Self::Text,
            1 => Self::Binary,
            2 => Self::Position,
            3 => Self::TimestampedPosition,
            4 => Self::Message,
            6 => Self::ThirdParty,
            _ => return None,
        })
    }

    pub const fn has_position(self) -> bool {
        matches!(self, Self::Position | Self::TimestampedPosition)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Call<'a> {
    pub base: &'a str,
    pub ssid: u8,
}

impl<'a> Call<'a> {
    pub fn parse(call: &'a str) -> Option<Self> {
        let (base, ssid) = call.split_once('-').unwrap_or((call, "0"));
        let ssid = ssid.parse::<u8>().ok().filter(|&s| s <= 15)?;
        ((1..=7).contains(&base.len()) && base.bytes().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()))
            .then_some(Call { base, ssid })
    }
}
const LONG: &[u8; 53] = b" ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!:#$%&'()*+,-./;";
const ALIASES: [&str; 5] = ["WIDE", "PNW", "MW", "OR", "WA"];
const DICTIONARY: [&[u8]; 9] = [b"/A=00", b"VARA", b"PinPoint", b"HF ", b"ack", b"APRS", b"ping{", b"Digi", b"Gate"];
const SYMBOL_TABLES: &[u8; 8] = b"/\\IRVTDS";
const SYMBOL_TABLES_CODED: &[u8] = b"/\\IRVDS";
const WIDE_TOKENS: [&str; 8] = ["WIDE1-1", "WIDE1", "WIDE1-2", "WIDE1-3", "WIDE2", "WIDE2-1", "WIDE2-2", "WIDE2-3"];
const PAIRS: [(u8, &str, usize, usize, usize); 5] = [
    (b'!', "WIDE", 4, 6, 2),
    (b':', "PNW", 3, 5, 1),
    (b'#', "MW", 2, 4, 1),
    (b'$', "OR", 2, 4, 1),
    (b'%', "WA", 2, 4, 1),
];
const FIXED: [(&str, u8); 7] = [
    ("ALL", b'&'),
    ("CQ", b'\''),
    ("APBPQ1", b'+'),
    ("APJYC1", b','),
    ("APOPYT", b'-'),
    ("APU25N", b'.'),
    ("APVARA", b'/'),
];
const NUMBERED: [(u8, &str, u64, u64, usize); 3] =
    [(b'(', "APIN", 20, 36, 4), (b')', "APWW", 11, 19, 3), (b'*', "APDR", 16, 48, 5)];

fn call_len(cmd: Command) -> usize {
    if cmd == Command::Ax25Long { 7 } else { 6 }
}

fn addr_end(digis: usize, l: usize) -> usize {
    (digis + 2) * (l + 1)
}

fn sym(c: u8) -> u64 {
    LONG.iter().position(|&x| x == c).unwrap() as u64
}

fn alpha(c: u8) -> Option<u64> {
    CALL_ALPHABET.iter().position(|&x| x == c).map(|i| i as u64)
}

fn slice(d: &[u8], a: usize, b: usize) -> &[u8] {
    &d[a.min(d.len())..b.min(d.len())]
}

fn at(d: &[u8], i: usize) -> Result<u8, Error> {
    d.get(i).copied().ok_or(Error::Short)
}

fn bad_call(call: &str) -> Error {
    Error::BadCall(call.to_string())
}

pub fn token_encode(call: &str, call_len: usize, out: &mut Bits) -> Result<(), Error> {
    let c = call.as_bytes();
    let num = |a: usize, b: usize| vb_val(slice(c, a, b));
    let nbit = |i: usize| u64::from(c.get(i) == Some(&b'2'));
    if WIDE_TOKENS.contains(&call) {
        out.push(sym(b'!'), 6);
        out.push(nbit(4), 1);
        out.push(num(6, 7), 2);
        return Ok(());
    }
    for &(s, name, ni, si, w) in &PAIRS[1..] {
        if [format!("{name}1-1"), format!("{name}1"), format!("{name}2"), format!("{name}2-1")]
            .iter()
            .any(|n| n == call)
        {
            out.push(sym(s), 6);
            out.push(nbit(ni), 1);
            out.push(num(si, si + 1), w);
            return Ok(());
        }
    }
    if let Some(&(_, s)) = FIXED.iter().find(|(n, _)| *n == call) {
        out.push(sym(s), 6);
        return Ok(());
    }
    let plain_ap = call.starts_with("AP") && !call.contains('-');
    for &(s, pre, lo, hi, w) in &NUMBERED {
        let v = num(4, 6);
        if call.starts_with(pre) && !call.contains('-') && (lo..hi).contains(&v) {
            out.push(sym(s), 6);
            out.push(v - lo, w);
            return Ok(());
        }
    }
    if plain_ap {
        out.push(sym(b';'), 6);
        for &ch in &c[2..] {
            out.push(alpha(ch).ok_or_else(|| bad_call(call))?, 6);
        }
        if c.len() < call_len {
            out.push(0, 6);
        }
        return Ok(());
    }
    let (base, ssid) = call.split_once('-').unwrap_or((call, ""));
    let base = base.to_ascii_uppercase();
    for &ch in base.as_bytes() {
        out.push(alpha(ch).ok_or_else(|| bad_call(call))?, 6);
    }
    if base.len() < call_len {
        out.push(0, 6);
    }
    out.push(vb_val(ssid.as_bytes()), 4);
    Ok(())
}

pub fn token_decode(r: &mut Reader, call_len: usize) -> Option<String> {
    let ch = *LONG.get(r.take(6)? as usize)?;
    if let Some(&(_, name, _, _, w)) = PAIRS.iter().find(|p| p.0 == ch) {
        let n = if r.bit()? { '2' } else { '1' };
        let nn = vb_val(r.text(w)?.as_bytes());
        return Some(if nn > 0 { format!("{name}{n}-{nn}") } else { format!("{name}{n}") });
    }
    if let Some(&(name, _)) = FIXED.iter().find(|f| f.1 == ch) {
        return Some(name.into());
    }
    if let Some(&(_, pre, lo, _, w)) = NUMBERED.iter().find(|n| n.0 == ch) {
        return Some(format!("{pre}{:02}", r.take(w)? + lo));
    }
    if ch == b';' {
        let mut call = String::from("AP");
        for _ in 2..call_len {
            let c = *CALL_ALPHABET.get(r.take(6)? as usize)?;
            if c == b' ' {
                break;
            }
            call.push(c as char);
        }
        return Some(call);
    }
    let mut call = String::from(ch as char);
    for _ in 1..call_len {
        match CALL_ALPHABET.get(r.take(6)? as usize) {
            Some(b' ') => break,
            Some(&c) => call.push(c as char),
            None => {}
        }
    }
    let ssid = r.take(4)?;
    if ssid > 0 {
        call += &format!("-{ssid}");
    }
    Some(call)
}

fn text_encode(data: &[u8], mut i: usize, mode: AprsType, out: &mut Bits) -> Result<(), Error> {
    while i < data.len() {
        match DICTIONARY.iter().position(|d| data[i..].starts_with(d)) {
            None => out.push_bfi(u64::from(data[i]), 7),
            Some(m) => {
                out.push_bfi(m as u64 + 1, 7);
                if m == 0 && i + 8 < data.len() && mode.has_position() {
                    let alt = std::str::from_utf8(&data[i + 5..i + 9]).ok().and_then(|s| s.trim().parse::<u64>().ok());
                    out.push_bfi(alt.ok_or(Error::Format)?, 14);
                    i += 4;
                }
                i += DICTIONARY[m].len() - 1;
            }
        }
        i += 1;
    }
    Ok(())
}

fn text_decode(r: &mut Reader, mode: AprsType) -> Vec<u8> {
    let mut out = Vec::new();
    while let Some(v) = r.take(7) {
        match v {
            0 => {}
            32.. => out.push(v as u8),
            1 => {
                out.extend_from_slice(DICTIONARY[0]);
                if mode.has_position() {
                    let n = r.remaining().min(14);
                    let alt = r.take(n).unwrap_or(0);
                    r.pos += 14 - n;
                    out.extend(format!("{alt:04}").bytes());
                }
            }
            2..=9 => out.extend_from_slice(DICTIONARY[v as usize - 1]),
            _ => {}
        }
    }
    out
}

fn latlon_encode(data: &[u8], mut p: usize, mode: AprsType, out: &mut Bits) -> Result<(), Error> {
    let joined = |a: usize, b: usize, c: usize, d: usize| vb_val(&[slice(data, a, b), slice(data, c, d)].concat());
    out.push_bfi(joined(p, p + 4, p + 5, p + 7), 20);
    match at(data, p + 7)? {
        b'N' => out.bit(true),
        b'S' => out.bit(false),
        _ => {}
    }
    p += 8;
    let tbl = at(data, p)?;
    if SYMBOL_TABLES_CODED.contains(&tbl) {
        out.bit(true);
        out.push(SYMBOL_TABLES.iter().position(|&t| t == tbl).unwrap() as u64, 3);
    } else {
        out.bit(false);
        out.push_bfi(u64::from(tbl), 7);
    }
    p += 1;
    out.push_bfi(joined(p, p + 5, p + 6, p + 8), 21);
    match at(data, p + 8)? {
        b'W' => out.bit(true),
        b'E' => out.bit(false),
        _ => {}
    }
    p += 9;
    out.push_bfi(u64::from(at(data, p)?), 7);
    text_encode(data, p + 1, mode, out)
}

fn aprs_encode(mode: AprsType, data: &[u8], p: usize, call_len: usize, out: &mut Bits) -> Result<(), Error> {
    match mode {
        AprsType::Text => text_encode(data, p, mode, out),
        AprsType::Position => latlon_encode(data, p, mode, out),
        AprsType::TimestampedPosition => {
            out.push_bfi(vb_val(slice(data, p, p + 6)), 20);
            latlon_encode(data, p + 7, mode, out)
        }
        _ => {
            let addressee = String::from_utf8_lossy(trim_whitespace(slice(data, p, p + 9))).to_ascii_uppercase();
            token_encode(&addressee, call_len, out)?;
            text_encode(data, p + 10, AprsType::Message, out)
        }
    }
}

fn latlon_decode(r: &mut Reader, mode: AprsType) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let lat = format!("{:06}", r.take(20)?);
    out.extend(format!("{}.{}", &lat[..4], &lat[4..]).bytes());
    out.push(if r.bit()? { b'N' } else { b'S' });
    if r.bit()? {
        out.push(SYMBOL_TABLES[r.take(3)? as usize]);
    } else {
        out.push(r.take(7)? as u8);
    }
    let lon = format!("{:07}", r.take(21)?);
    out.extend(format!("{}.{}", &lon[..5], &lon[5..]).bytes());
    out.push(if r.bit()? { b'W' } else { b'E' });
    out.push(r.take(7)? as u8);
    out.extend(text_decode(r, mode));
    Some(out)
}

fn aprs_decode(mode: AprsType, r: &mut Reader, call_len: usize) -> Option<Vec<u8>> {
    Some(match mode {
        AprsType::Text => text_decode(r, mode),
        AprsType::Position => [b"!".to_vec(), latlon_decode(r, mode)?].concat(),
        AprsType::TimestampedPosition => {
            let t = format!("@{:06}h", r.take(20)?);
            [t.into_bytes(), latlon_decode(r, mode)?].concat()
        }
        _ => {
            let addressee = token_decode(r, call_len)?;
            [format!(":{addressee:<9}:").into_bytes(), text_decode(r, AprsType::Message)].concat()
        }
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub call_len: usize,
    pub dest: String,
    pub src: String,
    pub digis: Vec<String>,
    pub h: Vec<bool>,
    pub ctrl: u8,
    pub pid: u8,
    pub alias: usize,
    pub aprs_type: AprsType,
}

fn trim_whitespace(s: &[u8]) -> &[u8] {
    let ws = |c: &u8| matches!(c, 9..=13 | 28..=32);
    let a = s.iter().position(|c| !ws(c)).unwrap_or(s.len());
    let b = s.iter().rposition(|c| !ws(c)).map_or(a, |i| i + 1);
    &s[a..b]
}

fn addr_call(frame: &[u8], off: usize, n: usize) -> Result<String, Error> {
    let raw: Vec<u8> = frame.get(off..off + n).ok_or(Error::Short)?.iter().map(|b| b >> 1).collect();
    let mut call = String::from_utf8_lossy(trim_whitespace(&raw)).into_owned();
    let ssid = (at(frame, off + n)? & 0x1E) >> 1;
    if ssid != 0 {
        call += &format!("-{ssid}");
    }
    Ok(call)
}

pub fn parse(frame: &[u8], cmd: Command) -> Result<Record, Error> {
    let l = call_len(cmd);
    let a = l + 1;
    let mut rec = Record {
        call_len: l,
        dest: addr_call(frame, 0, l)?.to_ascii_uppercase(),
        src: addr_call(frame, a, l)?.to_ascii_uppercase(),
        digis: Vec::new(),
        h: Vec::new(),
        ctrl: 0,
        pid: 0,
        alias: 0,
        aprs_type: AprsType::Text,
    };
    let mut pos = 2 * a - 1;
    while at(frame, pos)? & 1 == 0 && rec.digis.len() < 8 {
        rec.digis.push(addr_call(frame, pos + 1, l)?);
        rec.h.push(at(frame, pos + a)? >> 7 == 1);
        pos += a;
    }
    rec.ctrl = at(frame, pos + 1)?;
    rec.pid = at(frame, pos + 2)?;
    let n = rec.digis.len();
    if n > 0 {
        rec.alias = ALIASES
            .iter()
            .position(|al| (1..=n).all(|d| rec.digis[d - 1] == format!("{al}{d}-1")))
            .map_or(0, |k| k + 1);
    }
    rec.aprs_type = classify(frame, &rec, cmd);
    Ok(rec)
}

fn classify(f: &[u8], rec: &Record, cmd: Command) -> AprsType {
    let n = rec.digis.len();
    let i = addr_end(n, rec.call_len) + 2;
    if f.get(i..).is_some_and(|s| s.iter().any(|&b| b > 0x7F)) {
        return AprsType::Binary;
    }
    if !rec.dest.starts_with("AP") || cmd == Command::Ax25Long || i >= f.len() {
        return AprsType::Text;
    }
    let ub = f.len() - 1;
    let dig = |k: usize| f[k].is_ascii_digit();
    let ch = |k: usize| f[k];
    let c = f[i];
    if i + 19 <= ub
        && b"!=".contains(&c)
        && dig(i + 1)
        && ch(i + 5) == b'.'
        && b"NS".contains(&ch(i + 8))
        && ch(i + 15) == b'.'
        && b"WE".contains(&ch(i + 18))
    {
        return AprsType::Position;
    }
    if i + 26 <= ub
        && b"@/".contains(&c)
        && dig(i + 1)
        && ch(i + 7) == b'h'
        && ch(i + 12) == b'.'
        && b"NS".contains(&ch(i + 15))
        && ch(i + 22) == b'.'
        && b"WE".contains(&ch(i + 25))
    {
        return AprsType::TimestampedPosition;
    }
    if i + 10 <= ub && c == b':' && ch(i + 10) == b':' {
        return AprsType::Message;
    }
    if i + 20 <= ub && c == b'}' {
        let (mut gt, mut comma) = (0, 0);
        for j in i..=ub {
            match f[j] {
                b'>' => gt += 1,
                b',' => {
                    if comma == 0 && slice(f, j + 1, j + 7) != b"TCPIP," {
                        return AprsType::Text;
                    }
                    comma += 1;
                }
                b'*' if gt == 1 && comma == 2 && f.get(j + 1) == Some(&b':') => return AprsType::ThirdParty,
                _ => {}
            }
        }
    }
    AprsType::Text
}

fn collect(f: &[u8], i: usize, stop: u8, span: usize) -> Result<(String, usize), Error> {
    let mut s = String::new();
    for j in i..=i + span {
        let c = at(f, j)?;
        if c == stop {
            return Ok((s, j + 1));
        }
        s.push(c as char);
    }
    Ok((s, i))
}

fn third_party_call(s: &str) -> String {
    String::from_utf8_lossy(trim_whitespace(s.as_bytes())).to_ascii_uppercase()
}

pub fn encode(frame: &[u8], cmd: Command) -> Result<(Vec<u8>, Record), Error> {
    let rec = parse(frame, cmd)?;
    let (l, n, typ) = (rec.call_len, rec.digis.len(), rec.aprs_type);
    let ui = rec.ctrl == UI_CTRL && rec.pid == PID_NO_L3;
    let mut hdr = if cmd == Command::Ax25Long { 0x10 } else { 0x00 };
    if ui {
        hdr |= 0x40;
    }
    let mut b = Bits::default();
    if n > 0 {
        hdr |= 0x08 | (n as u8 - 1);
        for &h in &rec.h {
            b.bit(h);
        }
    }
    token_encode(&rec.dest, l, &mut b)?;
    token_encode(&rec.src, l, &mut b)?;
    let ap = rec.dest.starts_with("AP");
    if n > 0 {
        if ap && rec.alias > 0 {
            b.bit(true);
            b.push(rec.alias as u64, 3);
        } else {
            if ap {
                b.bit(false);
            }
            for d in &rec.digis {
                token_encode(d, l, &mut b)?;
            }
        }
    }
    b.push(u64::from(typ.code()), if ap { 3 } else { 1 });
    match typ {
        AprsType::Text | AprsType::Binary => {
            let start = addr_end(n, l) + if ui { 2 } else { 0 };
            let body = frame.get(start..).unwrap_or(&[]);
            if typ == AprsType::Text {
                b.push_text7(body);
            } else {
                for &c in body {
                    b.push(u64::from(c), 8);
                }
            }
        }
        AprsType::Position | AprsType::TimestampedPosition | AprsType::Message => {
            aprs_encode(typ, frame, addr_end(n, l) + 3, l, &mut b)?
        }
        AprsType::ThirdParty => {
            let (s, i) = collect(frame, addr_end(n, l) + 3, b'>', 9)?;
            token_encode(&third_party_call(&s), l, &mut b)?;
            let (s, i) = collect(frame, i, b',', 9)?;
            token_encode(&third_party_call(&s), l, &mut b)?;
            let (_, j) = collect(frame, i, b',', 13)?;
            if j == i {
                return Err(Error::Format);
            }
            let (s, j2) = collect(frame, j, b'*', 9)?;
            let i = if j2 != j { j2 + 1 } else { j };
            token_encode(&third_party_call(&s), l, &mut b)?;
            let (c, dig) = (at(frame, i)?, at(frame, i + 1)?.is_ascii_digit());
            let (inner, mode, p) = match c {
                b'!' | b'=' if dig => (0, AprsType::Position, i + 1),
                b'@' | b'/' if dig => (1, AprsType::TimestampedPosition, i + 1),
                b':' => (2, AprsType::Message, i + 1),
                _ => (3, AprsType::Text, i),
            };
            b.push(inner, 2);
            aprs_encode(mode, frame, p, l, &mut b)?;
        }
    }
    while b.len() % 8 != 0 {
        b.bit(false);
    }
    Ok(([vec![hdr], b.pack()].concat(), rec))
}

fn call_bytes(call: &str, l: usize, out: &mut Vec<u8>) {
    let (base, ssid) = call.split_once('-').unwrap_or((call, ""));
    let ss = match ssid {
        "T" => 16,
        "R" => 17,
        "X" => 18,
        "L" => 19,
        s => vb_val(s.as_bytes()),
    };
    let base = base.to_ascii_uppercase();
    out.extend(format!("{base:<l$}").bytes().take(l).map(|c| c.wrapping_mul(2)));
    out.push(0x60 | (ss as u8).wrapping_mul(2));
}

pub fn decode(buf: &[u8]) -> Option<(Command, Vec<u8>)> {
    let hdr = *buf.first()?;
    let cmd = if hdr & 0x10 != 0 { Command::Ax25Long } else { Command::Ax25 };
    let l = call_len(cmd);
    let bits = Bits::unpack(&buf[1..]);
    let mut r = Reader::new(&bits.0);
    let n = if hdr & 8 != 0 { usize::from(hdr & 7) + 1 } else { 0 };
    let h: Vec<bool> = (0..n).map(|_| r.bit()).collect::<Option<_>>()?;
    let dest = token_decode(&mut r, l)?;
    let src = token_decode(&mut r, l)?;
    let ap = dest.starts_with("AP");
    let mut digis = Vec::new();
    if n > 0 {
        if ap && r.bit()? {
            let a = ALIASES.get((r.take(3)? as usize).checked_sub(1)?)?;
            digis = (1..=n).map(|k| format!("{a}{k}-1")).collect();
        } else {
            for _ in 0..n {
                digis.push(token_decode(&mut r, l)?);
            }
        }
    }
    let typ = AprsType::from_code(r.take(if ap { 3 } else { 1 })?)?;
    let mut out = Vec::new();
    for c in [&dest, &src].into_iter().chain(&digis) {
        call_bytes(c, l, &mut out);
    }
    *out.last_mut()? |= 0x01;
    for (k, &hb) in h.iter().enumerate() {
        if hb {
            out[addr_end(k + 1, l) - 1] |= 0x80;
        }
    }
    if hdr & 0x40 != 0 {
        out.extend([UI_CTRL, PID_NO_L3]);
    }
    match typ {
        AprsType::Text => out.extend(r.text7()?),
        AprsType::Binary => {
            while let Some(v) = r.take(8) {
                out.push(v as u8);
            }
        }
        AprsType::Position | AprsType::TimestampedPosition | AprsType::Message => {
            out.extend(aprs_decode(typ, &mut r, l)?)
        }
        AprsType::ThirdParty => {
            let s1 = token_decode(&mut r, l)?;
            let s2 = token_decode(&mut r, l)?;
            let s3 = token_decode(&mut r, l)?;
            out.extend(format!("}}{s1}>{s2},TCPIP,{s3}*:").bytes());
            let mode = [AprsType::Position, AprsType::TimestampedPosition, AprsType::Message, AprsType::Text]
                [r.take(2)? as usize];
            out.extend(aprs_decode(mode, &mut r, l)?);
        }
    }
    Some((cmd, out))
}

#[cfg(test)]
mod tests {
    use super::Call;

    #[test]
    fn call_parse() {
        assert_eq!(Call::parse("N0CALL-15"), Some(Call { base: "N0CALL", ssid: 15 }));
        assert_eq!(Call::parse("N0CALL"), Some(Call { base: "N0CALL", ssid: 0 }));
        for bad in ["N0CALL-16", "N0CALL-", "n0call", "TOOLONG1", "", "-1", "N0C@LL"] {
            assert_eq!(Call::parse(bad), None, "{bad}");
        }
    }
}
