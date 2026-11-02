use crate::crc::crc16_genibus;

pub fn call_hash(call: &[u8]) -> u32 {
    if call.is_empty() {
        return 0;
    }
    let c1 = u32::from(crc16_genibus(call));
    let c2 = u32::from(crc16_genibus(&call[call.len().saturating_sub(3)..]));
    ((c2 >> 8) << 16) | c1
}

pub fn ssid_code(ssid: &str) -> Option<u8> {
    Some(match ssid {
        "T" => 0x10,
        "R" => 0x11,
        "X" => 0x12,
        "L" => 0x13,
        _ => ssid.parse().ok()?,
    })
}

pub fn with_ssid(base: String, code: u8) -> String {
    match code {
        0 => base,
        0x10 => base + "-T",
        0x11 => base + "-R",
        0x12 => base + "-X",
        0x13 => base + "-L",
        n => format!("{base}-{n}"),
    }
}

pub fn pack_call(call: &[u8]) -> [u8; 6] {
    let (base, ssid) = match call.iter().position(|&c| c == b'-') {
        Some(i) => {
            let code = std::str::from_utf8(&call[i + 1..]).ok().and_then(ssid_code);
            (&call[..i], code.filter(|n| (1..=0x13).contains(n)).unwrap_or(0))
        }
        None => (call, 0),
    };
    let mut codes: Vec<u8> = base
        .iter()
        .map(|&c| match c {
            b'A'..=b'Z' => c - b'A' + 1,
            b'0'..=b'9' => c - b'0' + 27,
            _ => 0,
        })
        .collect();
    codes.resize(codes.len().max(7), 0);
    codes.push(ssid);
    let bits = codes[..8].iter().fold(0u64, |b, &c| (b << 6) | u64::from(c & 63));
    bits.to_be_bytes()[2..].try_into().unwrap()
}

pub fn is_us_call(call: &str) -> bool {
    let base = call.split('-').next().unwrap_or("").as_bytes();
    let n = base.len();
    let digits: Vec<usize> = (0..n).filter(|&i| base[i].is_ascii_digit()).collect();
    if !(4..=6).contains(&n) || !b"NKWA".contains(&base[0]) || digits.len() != 1 || !(1..=2).contains(&digits[0]) {
        return false;
    }
    if !base.iter().all(u8::is_ascii_alphanumeric) {
        return false;
    }
    if base[0] == b'A' && (b'M'..=b'Z').contains(&base[1]) {
        return false;
    }
    let (pre, suf) = (digits[0], n - digits[0] - 1);
    match (pre, suf) {
        (2, 3) => matches!(base[0], b'K' | b'W'),
        (2, 2) if base[0] == b'A' => (b'A'..=b'L').contains(&base[1]),
        _ => true,
    }
}

pub fn unpack_call(b: &[u8]) -> Option<String> {
    let b = b.first_chunk::<6>()?;
    let v = b.iter().fold(0u64, |v, &x| (v << 8) | u64::from(x));
    let base: String = (1..8)
        .rev()
        .map(|i| match ((v >> (6 * i)) & 63) as u8 {
            c @ 1..=26 => (b'A' + c - 1) as char,
            c @ 27..=36 => (b'0' + c - 27) as char,
            _ => ' ',
        })
        .collect();
    Some(with_ssid(base.trim_end().to_string(), (v & 0x1f) as u8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_examples() {
        assert_eq!(call_hash(b"N0CALL"), 0xD80986);
        assert_eq!(call_hash(b""), 0);
        assert_eq!(pack_call(b"N0CALL"), [0x39, 0xb0, 0xc1, 0x30, 0xc0, 0x00]);
        assert_eq!(pack_call(b"AB1CD-1"), [0x04, 0x27, 0x03, 0x10, 0x00, 0x01]);
        assert_eq!(pack_call(b"N0CALL-T"), [0x39, 0xb0, 0xc1, 0x30, 0xc0, 0x10]);
    }

    #[test]
    fn ssid_round_trip() {
        for c in
            ["N0CALL", "AB1CD-1", "AB1CD-11", "N0CALL-T", "N0CALL-R", "N0CALL-X", "N0CALL-L", "W1AW-15", "VE3ABCD-9"]
        {
            assert_eq!(unpack_call(&pack_call(c.as_bytes())).as_deref(), Some(c));
        }
    }

    fn from_codes(c: [u8; 8]) -> [u8; 6] {
        c.iter().fold(0u64, |v, &x| (v << 6) | u64::from(x)).to_be_bytes()[2..].try_into().unwrap()
    }

    #[test]
    fn ssid_as_vara_sends_it() {
        let n0call = [14, 27, 3, 1, 12, 12, 0];
        for (ssid, code) in [("2", 2), ("10", 10), ("15", 15), ("T", 16), ("R", 17), ("X", 18), ("L", 19), ("19", 19)] {
            let mut codes = [0; 8];
            codes[..7].copy_from_slice(&n0call);
            codes[7] = code;
            let call = format!("N0CALL-{ssid}");
            assert_eq!(pack_call(call.as_bytes()), from_codes(codes), "{call}");
        }
        assert_eq!(unpack_call(&from_codes([23, 28, 1, 23, 0, 0, 0, 7])).as_deref(), Some("W1AW-7"));
        assert_eq!(unpack_call(&from_codes([23, 28, 1, 2, 3, 4, 5, 9])).as_deref(), Some("W1ABCDE-9"));
        for bad in ["N0CALL-0", "N0CALL-Q", "N0CALL-20", "N0CALL-25", "N0CALL-"] {
            assert_eq!(pack_call(bad.as_bytes()), pack_call(b"N0CALL"), "{bad}");
        }
    }
}
