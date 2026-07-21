//! OpenHPSDR Protocol 1 ("old protocol" / Metis / Ozy) — enough for RX + SSB TX,
//! which covers Hermes-Lite 2 and legacy Metis/Hermes boards.
//!
//! Everything runs over UDP port 1024. Data datagrams are 1032 bytes:
//! `0xEF 0xFE 0x01 <endpoint>` + a 4-byte big-endian sequence, then two 512-byte
//! "OZY" frames. Each frame is `0x7F 0x7F 0x7F` + five command-and-control bytes
//! (C0..C4) + 504 bytes of data (63 samples × 8 bytes for one receiver).
//!
//! - RX (endpoint 6, radio→host): each sample is I (24-bit signed big-endian) +
//!   Q (24-bit) + mic (16-bit) = 8 bytes.
//! - TX (endpoint 2, host→radio): each sample is L + R audio (16-bit) + TX I +
//!   TX Q (16-bit signed) = 8 bytes. We send zero audio and the modulator's I/Q.
//!
//! The C0..C4 registers are round-robined on outgoing frames. C0 bit 0 is MOX
//! (PTT); C0 bits 7..1 are the register address. Frequencies are the actual
//! value in **Hz** (32-bit big-endian in C1..C4), not a phase word.
//!
//! Offsets follow the g0orx/rustyHPSDR reference and the OpenHPSDR/Hermes-Lite 2
//! protocol docs; verify against hardware before trusting on-air behavior.

use std::net::SocketAddr;
use std::time::Instant;

use crate::net::{Ctrl, ThreadCtx, WATCHDOG};
use crate::protocol2::be24_to_f32;

const PORT: u16 = 1024;
/// Samples per 512-byte OZY frame for one receiver (504 data bytes / 8).
const SAMPLES_PER_FRAME: usize = 63;
/// Two frames per datagram → 126 sample-pairs = 252 interleaved floats.
const FLOATS_PER_DATAGRAM: usize = SAMPLES_PER_FRAME * 2 * 2;
const DATAGRAM_LEN: usize = 8 + 2 * 512;

/// C0 register bytes (address already shifted into bits 7..1; OR in the MOX
/// bit 0). Values confirmed against the rustyHPSDR reference.
const CC_CONFIG: u8 = 0x00; // frame #1 of every datagram: C1[1:0]=rate, C4=duplex|rx-count
const CC_TX_FREQ: u8 = 0x02; // C1..C4 = TX NCO frequency (Hz)
const CC_RX1_FREQ: u8 = 0x04; // C1..C4 = RX1 NCO frequency (Hz)
const CC_DRIVE: u8 = 0x12; // C1 = TX drive level 0..255

/// Fixed TX drive; the engine already scales the I/Q amplitude in software.
const TX_DRIVE: u8 = 255;
/// Config C4: duplex on (bit 2) so RX keeps streaming during TX, plus
/// `(receivers - 1) << 3` in bits 5:3 — one receiver → 0.
const CONFIG_C4: u8 = 0x04;

/// Sample-rate code for the config register (48/96/192/384 kHz → 0..3).
fn speed_code(rate_hz: f64) -> u8 {
    match rate_hz.round() as u32 {
        r if r <= 48_000 => 0,
        r if r <= 96_000 => 1,
        r if r <= 192_000 => 2,
        _ => 3,
    }
}

/// `-1.0..=1.0` float → 16-bit big-endian sample.
fn i16_be(x: f32) -> [u8; 2] {
    ((x.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_be_bytes()
}

/// The config register (C0 address 0). rustyHPSDR sends this as frame #1 of
/// *every* datagram: C1 = sample-rate code, C4 = duplex | receiver-count.
fn config_cc(speed: u8, mox: u8) -> [u8; 5] {
    [CC_CONFIG | mox, speed, 0, 0, CONFIG_C4]
}

/// The rotating register (frame #2 of each datagram): TX freq → RX1 freq →
/// drive, advancing one slot per datagram.
fn rotating_cc(slot: u8, rx_freq: u32, tx_freq: u32, mox: u8) -> [u8; 5] {
    match slot % 3 {
        0 => freq_cc(CC_TX_FREQ, tx_freq, mox),
        1 => freq_cc(CC_RX1_FREQ, rx_freq, mox),
        _ => [CC_DRIVE | mox, TX_DRIVE, 0, 0, 0],
    }
}

fn freq_cc(addr: u8, freq: u32, mox: u8) -> [u8; 5] {
    let f = freq.to_be_bytes();
    [addr | mox, f[0], f[1], f[2], f[3]]
}

/// Write one 512-byte OZY frame: sync + C&C + 63 TX samples (zero audio + I/Q).
fn write_ozy_frame(frame: &mut [u8], cc: [u8; 5], tx_iq: &[f32]) {
    frame[0] = 0x7F;
    frame[1] = 0x7F;
    frame[2] = 0x7F;
    frame[3..8].copy_from_slice(&cc);
    for s in 0..SAMPLES_PER_FRAME {
        let base = 8 + s * 8;
        // bytes [base..base+4] = L/R audio, left zero.
        let i = tx_iq.get(2 * s).copied().unwrap_or(0.0);
        let q = tx_iq.get(2 * s + 1).copied().unwrap_or(0.0);
        frame[base + 4..base + 6].copy_from_slice(&i16_be(i));
        frame[base + 6..base + 8].copy_from_slice(&i16_be(q));
    }
}

/// Build an EP2 (host→radio) datagram: frame #1 is always the config register,
/// frame #2 is the rotating register. Both frames also carry 63 TX samples.
fn build_ep2(
    seq: &mut u32,
    slot: &mut u8,
    speed: u8,
    rx_freq: u32,
    tx_freq: u32,
    ptt: bool,
    tx_iq: &[f32],
) -> [u8; DATAGRAM_LEN] {
    let mox = if ptt { 1 } else { 0 };
    let mut d = [0u8; DATAGRAM_LEN];
    d[0] = 0xEF;
    d[1] = 0xFE;
    d[2] = 0x01;
    d[3] = 0x02; // EP2
    d[4..8].copy_from_slice(&seq.to_be_bytes());
    *seq = seq.wrapping_add(1);

    // Frame #1: config register + TX samples 0..63.
    write_ozy_frame(&mut d[8..520], config_cc(speed, mox), tx_iq);
    // Frame #2: rotating register + TX samples 63..126.
    let cc = rotating_cc(*slot, rx_freq, tx_freq, mox);
    *slot = slot.wrapping_add(1);
    let chunk = &tx_iq[(SAMPLES_PER_FRAME * 2).min(tx_iq.len())..];
    write_ozy_frame(&mut d[520..1032], cc, chunk);
    d
}

/// Decode an EP6 (radio→host) datagram, appending interleaved I,Q floats.
/// Returns false if the datagram is not a valid EP6 frame.
fn decode_ep6(d: &[u8], out: &mut Vec<f32>) -> bool {
    if d.len() < DATAGRAM_LEN || d[0] != 0xEF || d[1] != 0xFE || d[2] != 0x01 || d[3] != 0x06 {
        return false;
    }
    for f in 0..2 {
        let frame = &d[8 + f * 512..8 + f * 512 + 512];
        if frame[0] != 0x7F || frame[1] != 0x7F || frame[2] != 0x7F {
            continue;
        }
        for s in 0..SAMPLES_PER_FRAME {
            let base = 8 + s * 8;
            let i = be24_to_f32([frame[base], frame[base + 1], frame[base + 2]]);
            let q = be24_to_f32([frame[base + 3], frame[base + 4], frame[base + 5]]);
            out.push(i);
            out.push(q);
        }
    }
    true
}

/// Metis start/stop command: `0xEF 0xFE 0x04 <run>` padded to 64 bytes. `run`
/// bit 0 starts the EP6 I/Q stream.
fn start_command(run: bool) -> [u8; 64] {
    let mut c = [0u8; 64];
    c[0] = 0xEF;
    c[1] = 0xFE;
    c[2] = 0x04;
    c[3] = if run { 0x01 } else { 0x00 };
    c
}

/// Run the Protocol 1 network loop until told to shut down.
pub(crate) fn run(ctx: ThreadCtx) {
    let ThreadCtx { socket, radio, rate_hz, mut rx, mut tx, ctrl } = ctx;
    let dest = SocketAddr::new(radio, PORT);
    let speed = speed_code(rate_hz);

    let mut out_seq: u32 = 0;
    let mut slot: u8 = 0;
    let mut rx_freq: u32 = 7_100_000;
    let mut tx_freq: u32 = 7_100_000;
    let mut ptt = false;

    // Prime the config/frequency registers (a couple of full rotations), then
    // start the EP6 I/Q stream — the order rustyHPSDR uses so the radio begins
    // with the correct sample rate and NCO already loaded.
    for _ in 0..6 {
        let d = build_ep2(&mut out_seq, &mut slot, speed, rx_freq, tx_freq, ptt, &[]);
        let _ = socket.send_to(&d, dest);
    }
    let _ = socket.send_to(&start_command(true), dest);

    let mut buf = [0u8; 2048];
    let mut rx_scratch: Vec<f32> = Vec::with_capacity(FLOATS_PER_DATAGRAM);
    let mut tx_scratch: Vec<f32> = Vec::with_capacity(FLOATS_PER_DATAGRAM);
    let mut last_send = Instant::now();

    loop {
        // 1) Control messages.
        while let Ok(msg) = ctrl.try_recv() {
            match msg {
                Ctrl::RxFreq(hz) => rx_freq = hz.max(0.0) as u32,
                Ctrl::TxOn(hz) => {
                    tx_freq = hz.max(0.0) as u32;
                    ptt = true;
                }
                Ctrl::TxOff => ptt = false,
                Ctrl::Shutdown => {
                    let _ = socket.send_to(&start_command(false), dest);
                    return;
                }
            }
        }

        // 2) One inbound datagram (EP6 RX I/Q).
        let mut got_ep6 = false;
        match socket.recv_from(&mut buf) {
            Ok((n, _src)) => {
                rx_scratch.clear();
                if decode_ep6(&buf[..n], &mut rx_scratch) {
                    got_ep6 = true;
                    for &s in &rx_scratch {
                        let _ = rx.push(s);
                    }
                }
            }
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(e) => {
                tracing::warn!("HPSDR P1 recv error: {e}; stopping");
                let _ = socket.send_to(&start_command(false), dest);
                return;
            }
        }

        // 3) Send one EP2 per received EP6 (paces TX to the sample rate); also on
        //    a keep-alive tick so C&C keeps flowing when idle.
        if got_ep6 || last_send.elapsed() >= WATCHDOG {
            tx_scratch.clear();
            if ptt {
                while tx_scratch.len() < FLOATS_PER_DATAGRAM {
                    match tx.pop() {
                        Ok(v) => tx_scratch.push(v),
                        Err(_) => break,
                    }
                }
            }
            let d =
                build_ep2(&mut out_seq, &mut slot, speed, rx_freq, tx_freq, ptt, &tx_scratch);
            let _ = socket.send_to(&d, dest);
            last_send = Instant::now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol2::f32_to_be24;

    #[test]
    fn speed_codes() {
        assert_eq!(speed_code(48_000.0), 0);
        assert_eq!(speed_code(96_000.0), 1);
        assert_eq!(speed_code(192_000.0), 2);
        assert_eq!(speed_code(384_000.0), 3);
    }

    #[test]
    fn freq_cc_is_hz_big_endian() {
        let cc = freq_cc(CC_RX1_FREQ, 14_074_000, 1);
        assert_eq!(cc[0], CC_RX1_FREQ | 1); // MOX bit set
        assert_eq!(u32::from_be_bytes([cc[1], cc[2], cc[3], cc[4]]), 14_074_000);
    }

    #[test]
    fn ep6_decode_roundtrip() {
        // Hand-build an EP6 datagram with one non-zero sample in each frame,
        // using the 24-bit encoder, and confirm decode recovers it.
        let mut d = [0u8; DATAGRAM_LEN];
        d[0] = 0xEF;
        d[1] = 0xFE;
        d[2] = 0x01;
        d[3] = 0x06;
        for f in 0..2 {
            let frame = &mut d[8 + f * 512..8 + f * 512 + 512];
            frame[0] = 0x7F;
            frame[1] = 0x7F;
            frame[2] = 0x7F;
            // sample 0: I=0.5, Q=-0.25
            frame[8..11].copy_from_slice(&f32_to_be24(0.5));
            frame[11..14].copy_from_slice(&f32_to_be24(-0.25));
        }
        let mut out = Vec::new();
        assert!(decode_ep6(&d, &mut out));
        // 2 frames × 63 samples × 2 floats.
        assert_eq!(out.len(), FLOATS_PER_DATAGRAM);
        assert!((out[0] - 0.5).abs() < 1e-4);
        assert!((out[1] + 0.25).abs() < 1e-4);
    }

    #[test]
    fn ep2_datagram_shape() {
        let mut seq = 0u32;
        let mut slot = 0u8;
        let d = build_ep2(&mut seq, &mut slot, 0, 7_074_000, 7_074_000, false, &[]);
        assert_eq!(d.len(), 1032);
        assert_eq!(&d[0..4], &[0xEF, 0xFE, 0x01, 0x02]);
        assert_eq!(seq, 1); // advanced
        assert_eq!(slot, 1); // one rotating slot consumed per datagram
        // Both frames start with the OZY sync.
        assert_eq!(&d[8..11], &[0x7F, 0x7F, 0x7F]);
        assert_eq!(&d[520..523], &[0x7F, 0x7F, 0x7F]);
        // Frame #1 is the config register (address 0); frame #2 is rotating
        // slot 0 = the TX-frequency register (C0 = 0x02).
        assert_eq!(d[11], CC_CONFIG); // frame #1 C0
        assert_eq!(d[8 + 512 + 3], CC_TX_FREQ); // frame #2 C0
        // Config C4 carries the duplex/receiver-count field.
        assert_eq!(d[8 + 7], CONFIG_C4); // frame #1 C4
    }

    #[test]
    fn mox_bit_rides_registers() {
        assert_eq!(config_cc(0, 1)[0] & 1, 1);
        assert_eq!(config_cc(0, 0)[0] & 1, 0);
        assert_eq!(rotating_cc(0, 7_000_000, 7_000_000, 1)[0] & 1, 1);
        assert_eq!(rotating_cc(2, 7_000_000, 7_000_000, 0)[0] & 1, 0);
    }
}
