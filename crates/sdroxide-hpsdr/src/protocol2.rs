//! OpenHPSDR Protocol 2 ("new protocol") wire format: constants and pure
//! packet builders/parsers.
//!
//! Byte offsets follow the g0orx/rustyHPSDR reference and the N4MTT
//! "openhpsdr-e" Wireshark dissector. They are cross-checked but should be
//! verified field-by-field against the TAPR Protocol 2 documentation before
//! trusting on-air behavior; see the notes on individual builders.

/// UDP ports. Host→radio use these as the *destination* port; radio→host DDC IQ
/// arrives with a *source* port of [`port::DDC_IQ_BASE`]` + ddc_index`.
pub mod port {
    /// Discovery + general/run/watchdog command (host→radio) and command reply.
    pub const GENERAL: u16 = 1024;
    /// DDC (receiver) configuration command.
    pub const DDC_COMMAND: u16 = 1025;
    /// DUC (transmitter) configuration command.
    pub const DUC_COMMAND: u16 = 1026;
    /// High-priority command: NCO frequencies, PTT/MOX, drive.
    pub const HIGH_PRIORITY: u16 = 1027;
    /// DUC I/Q data out to the radio (TX).
    pub const DUC_IQ: u16 = 1029;
    /// Radio→host: DDC I/Q streams start here (DDC0 = 1035).
    pub const DDC_IQ_BASE: u16 = 1035;
}

/// Master sample clock (Hz) used for the NCO phase-word math on
/// Hermes/Angelia/Orion/Saturn boards.
pub const CLOCK_HZ: f64 = 122_880_000.0;
/// Hermes-Lite 2 (Protocol 1) clock; kept for reference / future P1 support.
#[allow(dead_code)]
pub const CLOCK_HZ_HL2: f64 = 76_800_000.0;

/// Packet sizes (bytes).
pub const GENERAL_LEN: usize = 60;
pub const DDC_COMMAND_LEN: usize = 1444;
pub const DUC_COMMAND_LEN: usize = 60;
pub const HIGH_PRIORITY_LEN: usize = 1444;
/// 4-byte sequence + 240 IQ pairs × 6 bytes.
pub const DUC_IQ_LEN: usize = 4 + DUC_SAMPLES_PER_PKT * 6;
/// I/Q pairs per DUC (TX) datagram (rustyHPSDR `IQ_BUFFER_SIZE`).
pub const DUC_SAMPLES_PER_PKT: usize = 240;
/// Header bytes preceding the I/Q payload in a DDC (RX) datagram.
pub const DDC_IQ_HEADER_LEN: usize = 16;

/// Full-scale for 24-bit samples (2^23). RX divides by this; TX multiplies by
/// (this − 1) to avoid overflow at +1.0.
const FULL_SCALE: f32 = 8_388_608.0;

/// The 32-bit NCO phase word for `freq_hz` at `clock_hz`.
pub fn phase_word(freq_hz: f64, clock_hz: f64) -> u32 {
    let f = freq_hz.max(0.0);
    ((f / clock_hz) * 4_294_967_296.0).round() as u32
}

/// Decode a 24-bit big-endian two's-complement sample.
pub fn be24_to_i32(b: [u8; 3]) -> i32 {
    let u = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | (b[2] as u32);
    if u & 0x0080_0000 != 0 {
        (u | 0xFF00_0000) as i32
    } else {
        u as i32
    }
}

/// Encode a 24-bit big-endian two's-complement sample.
pub fn i32_to_be24(v: i32) -> [u8; 3] {
    let u = (v as u32) & 0x00FF_FFFF;
    [(u >> 16) as u8, (u >> 8) as u8, u as u8]
}

/// `-1.0..=1.0` float → 24-bit BE sample bytes.
pub fn f32_to_be24(x: f32) -> [u8; 3] {
    let v = (x.clamp(-1.0, 1.0) * (FULL_SCALE - 1.0)).round() as i32;
    i32_to_be24(v)
}

/// 24-bit BE sample bytes → `-1.0..=1.0` float.
pub fn be24_to_f32(b: [u8; 3]) -> f32 {
    be24_to_i32(b) as f32 / FULL_SCALE
}

/// Write a 32-bit big-endian value at `buf[off..off+4]`.
fn put_u32_be(buf: &mut [u8], off: usize, v: u32) {
    buf[off..off + 4].copy_from_slice(&v.to_be_bytes());
}

/// Build the General/run packet (dest port 1024). `run` sets the run bit;
/// clearing it stops streaming. Enables the hardware watchdog.
pub fn general_packet(seq: u32, run: bool) -> [u8; GENERAL_LEN] {
    let mut b = [0u8; GENERAL_LEN];
    put_u32_be(&mut b, 0, seq);
    b[4] = if run { 0x01 } else { 0x00 };
    b[23] = 0x00; // wideband disabled
    b[37] = 0x08; // phase-word / PA config
    b[38] = 0x01; // hardware watchdog enable
    b[58] = 0x01; // PA enable
    b[59] = 0x01; // ALEX enable (single ADC)
    b
}

/// Build the DDC command packet (dest port 1025): enable DDC0 at `rate_khz`,
/// 24 bits/sample, ADC0.
pub fn ddc_command_packet(seq: u32, rate_khz: u16) -> [u8; DDC_COMMAND_LEN] {
    let mut b = [0u8; DDC_COMMAND_LEN];
    put_u32_be(&mut b, 0, seq);
    b[4] = 1; // ADC count
    b[7] = 0x01; // DDC enable mask: DDC0 only
    // DDC0 descriptor (stride 6 from offset 17): ADC sel, rate (kHz, BE u16), bits.
    b[17] = 0; // ADC0
    b[18] = (rate_khz >> 8) as u8;
    b[19] = (rate_khz & 0xff) as u8;
    b[22] = 24; // bits per sample
    b
}

/// Build the DUC command packet (dest port 1026). Minimal linear-SSB config.
pub fn duc_command_packet(seq: u32) -> [u8; DUC_COMMAND_LEN] {
    let mut b = [0u8; DUC_COMMAND_LEN];
    put_u32_be(&mut b, 0, seq);
    b[5] = 0x00; // mode flags (no CW/keyer)
    b[50] = 0x00; // mic config
    b[51] = 0x00; // line-in / DUC gain
    b
}

/// Build the High-Priority command packet (dest port 1027): RX/TX NCO
/// frequencies, PTT/MOX, and drive level (0..=255).
///
/// Offsets: RX DDC0 NCO @ `buf[9..13]`, TX DUC0 NCO @ `buf[329..333]`, drive @
/// `buf[345]`, run/MOX flags @ `buf[4]` — canonical P2 values, verify against
/// the TAPR spec.
pub fn high_priority_packet(
    seq: u32,
    rx_phase: u32,
    tx_phase: u32,
    ptt: bool,
    drive: u8,
) -> [u8; HIGH_PRIORITY_LEN] {
    let mut b = [0u8; HIGH_PRIORITY_LEN];
    put_u32_be(&mut b, 0, seq);
    b[4] = 0x01 | if ptt { 0x02 } else { 0x00 }; // run + MOX
    put_u32_be(&mut b, 9, rx_phase); // DDC0 RX NCO
    put_u32_be(&mut b, 329, tx_phase); // DUC0 TX NCO
    b[345] = drive; // TX drive level 0..255
    b
}

/// Build one DUC I/Q datagram (dest port 1029) from up to
/// [`DUC_SAMPLES_PER_PKT`] interleaved I,Q float pairs. Pads with zeros.
pub fn duc_iq_packet(seq: u32, interleaved_iq: &[f32]) -> [u8; DUC_IQ_LEN] {
    let mut b = [0u8; DUC_IQ_LEN];
    put_u32_be(&mut b, 0, seq);
    let pairs = (interleaved_iq.len() / 2).min(DUC_SAMPLES_PER_PKT);
    for p in 0..pairs {
        let i = f32_to_be24(interleaved_iq[2 * p]);
        let q = f32_to_be24(interleaved_iq[2 * p + 1]);
        let off = 4 + p * 6;
        b[off..off + 3].copy_from_slice(&i);
        b[off + 3..off + 6].copy_from_slice(&q);
    }
    b
}

/// Decode a DDC (RX) I/Q datagram, appending `-1.0..=1.0` interleaved I,Q pairs
/// to `out`. Returns the number of complex samples decoded, or `None` if the
/// packet is too short / malformed.
pub fn decode_ddc_iq(pkt: &[u8], out: &mut Vec<f32>) -> Option<usize> {
    if pkt.len() < DDC_IQ_HEADER_LEN {
        return None;
    }
    // Sample count lives in the header at [14..16] (BE u16); fall back to
    // deriving it from the payload length if the field looks wrong.
    let declared = u16::from_be_bytes([pkt[14], pkt[15]]) as usize;
    let payload = &pkt[DDC_IQ_HEADER_LEN..];
    let max_pairs = payload.len() / 6;
    let pairs = if declared > 0 && declared <= max_pairs { declared } else { max_pairs };
    for p in 0..pairs {
        let off = p * 6;
        let i = be24_to_f32([payload[off], payload[off + 1], payload[off + 2]]);
        let q = be24_to_f32([payload[off + 3], payload[off + 4], payload[off + 5]]);
        out.push(i);
        out.push(q);
    }
    Some(pairs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_word_math() {
        // Half the clock → top bit set.
        assert_eq!(phase_word(CLOCK_HZ / 2.0, CLOCK_HZ), 0x8000_0000);
        // DC → 0.
        assert_eq!(phase_word(0.0, CLOCK_HZ), 0);
        // 14.074 MHz on the 122.88 MHz clock (known ballpark).
        let p = phase_word(14_074_000.0, CLOCK_HZ);
        let back = p as f64 / 4_294_967_296.0 * CLOCK_HZ;
        assert!((back - 14_074_000.0).abs() < 1.0, "round-trips within 1 Hz, got {back}");
    }

    #[test]
    fn be24_roundtrip() {
        for v in [0, 1, -1, 8_388_607, -8_388_608, 1234, -4321] {
            assert_eq!(be24_to_i32(i32_to_be24(v)), v, "roundtrip {v}");
        }
        // Big-endian byte order.
        assert_eq!(i32_to_be24(0x123456), [0x12, 0x34, 0x56]);
        // Sign extension.
        assert_eq!(be24_to_i32([0xFF, 0xFF, 0xFF]), -1);
        assert_eq!(be24_to_i32([0x80, 0x00, 0x00]), -8_388_608);
    }

    #[test]
    fn f32_be24_roundtrip() {
        for x in [0.0f32, 0.5, -0.5, 0.999, -0.999] {
            let back = be24_to_f32(f32_to_be24(x));
            assert!((back - x).abs() < 1e-4, "roundtrip {x} -> {back}");
        }
        // Clamps and never overflows at the rails.
        let _ = f32_to_be24(2.0);
        let _ = f32_to_be24(-2.0);
    }

    #[test]
    fn ddc_iq_roundtrip_via_duc_encoder() {
        // Encode two IQ pairs into DUC form, then decode as a DDC packet by
        // prepending a 16-byte header with the right sample count. This exercises
        // both the encoder and decoder against each other.
        let iq = [0.25f32, -0.5, 0.75, -0.125];
        let duc = duc_iq_packet(7, &iq);
        // Build a DDC-shaped packet: 16-byte header (count=2) + the 12 IQ bytes.
        let mut ddc = vec![0u8; DDC_IQ_HEADER_LEN];
        ddc[14] = 0;
        ddc[15] = 2;
        ddc.extend_from_slice(&duc[4..4 + 12]);
        let mut out = Vec::new();
        assert_eq!(decode_ddc_iq(&ddc, &mut out), Some(2));
        assert_eq!(out.len(), 4);
        for (a, b) in out.iter().zip(iq.iter()) {
            assert!((a - b).abs() < 1e-4, "{a} vs {b}");
        }
    }

    #[test]
    fn high_priority_field_placement() {
        let rx = phase_word(7_100_000.0, CLOCK_HZ);
        let tx = phase_word(7_100_000.0, CLOCK_HZ);
        let b = high_priority_packet(0, rx, tx, true, 200);
        assert_eq!(b[4] & 0x02, 0x02, "MOX bit set");
        assert_eq!(u32::from_be_bytes([b[9], b[10], b[11], b[12]]), rx);
        assert_eq!(u32::from_be_bytes([b[329], b[330], b[331], b[332]]), tx);
        assert_eq!(b[345], 200);
    }

    #[test]
    fn ddc_command_encodes_rate() {
        let b = ddc_command_packet(0, 1536);
        assert_eq!(b[7], 0x01);
        assert_eq!(u16::from_be_bytes([b[18], b[19]]), 1536);
        assert_eq!(b[22], 24);
    }
}
