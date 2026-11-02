pub fn crc16_genibus(data: &[u8]) -> u16 {
    let mut c: u16 = 0xFFFF;
    for &b in data {
        c ^= u16::from(b) << 8;
        for _ in 0..8 {
            c = if c & 0x8000 != 0 { (c << 1) ^ 0x1021 } else { c << 1 };
        }
    }
    c ^ 0xFFFF
}

pub fn crc_ok(frame: &[u8]) -> bool {
    let n = frame.len();
    n >= 3 && crc16_genibus(&frame[..n - 2]).to_be_bytes() == frame[n - 2..]
}

#[cfg(test)]
mod tests {
    #[test]
    fn check_value() {
        assert_eq!(super::crc16_genibus(b"123456789"), 0xD64E);
    }
}
