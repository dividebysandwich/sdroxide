//! Decode a recording and say what happened, in enough detail to tell a deaf
//! aerial from a broken decoder.
//!
//! ```text
//! cargo run --release -p sdroxide-vdl2 --example vdl2_replay -- /tmp/vdl2.iq 2400000 136825000
//! ```
//!
//! The file is interleaved little-endian `f32`, the format `sdroxide --file`
//! reads and `--record-iq` writes. `rate` and `center` are the recording's, in
//! hertz.
//!
//! # Reading the summary
//!
//! The counters are printed per channel, in the order the chain fails in, and
//! the first one that stops counting names the problem:
//!
//! - **no bursts** — nothing is arriving. The aerial, or the receiver is not
//!   looking here.
//! - **bursts, no syncs** — something is arriving and it is not VDL2.
//! - **syncs, no headers** — it *is* VDL2 and the bits are being read wrongly.
//!   A decoder problem, not a propagation one.
//! - **headers, HDLC bad** — the header reads and the data field does not
//!   unwrap as a flagged, bit-stuffed frame. A decoder problem.
//! - **headers, Reed-Solomon failures** — real frames, arriving damaged. Note
//!   that a failure here is not fatal: the block is tried uncorrected too and
//!   the frame check sequence decides, which is what **FEC bypassed** counts.
//! - **frames, and a comparable count of bad FCS** — a marginal path. The bits
//!   are arriving damaged and nothing repairs them, because the Reed-Solomon
//!   parameters do not fit what is on the air (see `rs.rs`): one bad bit loses
//!   the frame. The EVM printed per frame says how marginal — under about 4°
//!   is comfortable, 7° is on the edge.
//!
//! `--syndromes` is accepted and ignored; it used to print how many
//! Reed-Solomon blocks were clean before correction, and the answer on real
//! traffic is none of them, for the reason above.

use std::io::Read;

use sdroxide_dsp::{Complex32, Ddc};
use sdroxide_types::{Vdl2Payload, Vdl2Settings};
use sdroxide_vdl2::block::InterleaveOrder;
use sdroxide_vdl2::channel::ChannelRx;
use sdroxide_vdl2::{plan, station};

fn main() {
    let mut args = std::env::args().skip(1);
    let mut path = None;
    let mut rate = 0f64;
    let mut center = sdroxide_types::VDL2_PLAN_CENTER_HZ;
    let mut threshold = 9.0f32;
    let mut order = InterleaveOrder::RoundRobin;
    let mut positional = 0;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--threshold" => threshold = args.next().and_then(|v| v.parse().ok()).unwrap_or(9.0),
            "--sequential" => order = InterleaveOrder::Sequential,
            "--syndromes" => {}
            _ => {
                match positional {
                    0 => path = Some(a),
                    1 => rate = a.parse().unwrap_or(0.0),
                    _ => center = a.parse().unwrap_or(center),
                }
                positional += 1;
            }
        }
    }
    let Some(path) = path else {
        eprintln!(
            "usage: vdl2_replay <file.iq> <rate_hz> [center_hz] [--threshold dB] [--sequential]"
        );
        std::process::exit(2);
    };
    if rate <= 0.0 {
        eprintln!("a sample rate is needed; the file does not carry one");
        std::process::exit(2);
    }

    let iq = read_cf32(&path);
    println!(
        "{path}: {} samples, {:.3} s at {:.3} Msps centred on {:.3} MHz",
        iq.len(),
        iq.len() as f64 / rate,
        rate / 1e6,
        center / 1e6
    );

    // The same window the engine would place, so the answer here is the answer
    // there.
    let window_rate = Ddc::rate_for(rate, plan::WINDOW_TARGET_RATE_HZ.min(rate));
    let window_center = plan::window_center_for(center, rate, window_rate);
    let reachable = plan::channels_in_window(window_center, window_rate);
    println!(
        "window {:.3} MHz / {:.0} kHz reaches {} of {} channels",
        window_center / 1e6,
        window_rate / 1e3,
        reachable.len(),
        plan::CHANNELS.len()
    );
    if reachable.is_empty() {
        println!("nothing to decode: no VDL2 channel is inside this recording");
        return;
    }

    let mut window = Ddc::new(rate, plan::WINDOW_TARGET_RATE_HZ.min(rate));
    window.set_offset_hz(window_center - center);
    let mut win_buf = Vec::new();
    window.process(&iq, &mut win_buf);

    let mut tracker = station::Tracker::new(Vdl2Settings::default());
    let mut total = Totals::default();
    for &i in &reachable {
        let ch = &plan::CHANNELS[i];
        let mut ddc = Ddc::new(window_rate, plan::CHANNEL_TARGET_RATE_HZ);
        ddc.set_offset_hz(ch.center_hz - window_center);
        let mut buf = Vec::new();
        ddc.process(&win_buf, &mut buf);

        let mut rx = ChannelRx::new(ch.center_hz, ddc.out_rate(), threshold);
        rx.set_interleave_order(order);
        let mut out = Vec::new();
        rx.push(&buf, &mut out);
        let c = *rx.counters();
        println!(
            "{:.3} MHz  {:.2} sps  floor {:6.1} dBFS  bursts {:4}  syncs {:5}  headers {:4} \
             (bad {:3}, insane {:3})  RS ok {:4} fail {:4} fixed {:4}  HDLC bad {:3}  \
             bad FCS {:3}  frames {:3} (FEC bypassed {:3})",
            ch.center_hz / 1e6,
            rx.samples_per_symbol(),
            rx.floor_dbfs(),
            c.bursts,
            c.syncs,
            c.header_ok,
            c.header_bad,
            c.length_insane,
            c.rs_ok,
            c.rs_fail,
            c.rs_corrected,
            c.hdlc_bad,
            c.fcs_bad,
            c.frames,
            c.fec_bypassed,
        );
        total.add(&c);
        for d in &out {
            tracker.absorb(d, 0);
        }
    }

    println!();
    for m in tracker.messages() {
        let payload = match &m.payload {
            Vdl2Payload::Acars(a) => format!("ACARS {}", a.summary()),
            Vdl2Payload::Xid(x) => {
                let extra = if let (Some(la), Some(lo)) = (x.lat, x.lon) {
                    format!(" at {la:.1} {lo:.1}")
                } else {
                    String::new()
                };
                format!("XID {}{extra}", x.kind)
            }
            Vdl2Payload::Other { note, .. } => note.clone(),
            Vdl2Payload::None => String::new(),
        };
        println!(
            "{:.3} {} {} -> {} {:>10}  SNR {:4.1}  EVM {:4.1}°  {:+6.0} Hz  {payload}",
            m.freq_hz / 1e6,
            if m.command { "cmd" } else { "rsp" },
            m.src_hex(),
            m.dst_hex(),
            m.frame.label(),
            m.snr_db,
            m.evm_deg,
            m.freq_err_hz,
        );
    }

    println!();
    println!("{} stations heard:", tracker.stations().len());
    let mut st = tracker.stations();
    st.sort_by_key(|s| s.addr);
    for s in &st {
        println!(
            "  {} {:<8} {:<8} {:>4} messages  {:.3} MHz",
            s.hex(),
            s.kind.short(),
            s.label(),
            s.messages,
            s.last_freq_hz / 1e6
        );
    }

    println!();
    total.report();
}

#[derive(Default)]
struct Totals {
    bursts: u64,
    syncs: u64,
    headers: u64,
    rs_ok: u64,
    rs_fail: u64,
    hdlc_bad: u64,
    fec_bypassed: u64,
    fcs_bad: u64,
    frames: u64,
}

impl Totals {
    fn add(&mut self, c: &sdroxide_vdl2::channel::Counters) {
        self.bursts += c.bursts;
        self.syncs += c.syncs;
        self.headers += c.header_ok;
        self.rs_ok += c.rs_ok;
        self.rs_fail += c.rs_fail;
        self.hdlc_bad += c.hdlc_bad;
        self.fec_bypassed += c.fec_bypassed;
        self.fcs_bad += c.fcs_bad;
        self.frames += c.frames;
    }

    /// The one sentence that says what to do next.
    fn report(&self) {
        if self.bursts == 0 {
            println!(
                "no bursts at all. Nothing rose above the noise on any channel — the aerial, \
                 or a receiver that is not looking here."
            );
        } else if self.syncs == 0 {
            println!(
                "{} bursts and no synchronisation words. Something is on these channels and \
                 it is not VDL2.",
                self.bursts
            );
        } else if self.headers == 0 {
            println!(
                "{} synchronisation words and no valid headers. The bursts are VDL2 and the \
                 bits are being read wrongly — a decoder problem, not a propagation one.",
                self.syncs
            );
        } else if self.frames == 0 && self.hdlc_bad > 0 {
            println!(
                "{} headers and no frames, {} of them holding no HDLC frame. The header is \
                 being read and the data field is not — a decoder problem, not a propagation \
                 one.",
                self.headers, self.hdlc_bad
            );
        } else if self.frames == 0 && self.rs_fail > 0 {
            println!(
                "{} headers, {} Reed-Solomon failures and no frames. Real transmissions \
                 arriving too damaged to repair: a weak signal, or the wrong aerial.",
                self.headers, self.rs_fail
            );
        } else if self.fcs_bad * 2 > self.frames {
            println!(
                "{} frames passed their check sequence and {} failed it. That is a marginal \
                 path: the bits are arriving damaged, and the Reed-Solomon layer that should \
                 be repairing them does not fit what is on the air (see `rs.rs`), so a frame \
                 with one bad bit is simply lost. More aerial or more gain is what moves this \
                 number — the per-burst EVM above says how marginal.",
                self.frames, self.fcs_bad
            );
        } else {
            println!(
                "{} frames from {} bursts. Reed-Solomon repaired {} blocks and refused {} — \
                 {} of the frames came through on their check sequence alone; {} failed it.",
                self.frames, self.bursts, self.rs_ok, self.rs_fail, self.fec_bypassed, self.fcs_bad
            );
        }
    }
}

/// Interleaved `f32` I/Q, either raw or inside the RIFF wrapper SDRoxide's own
/// I/Q recorder writes — a recording contributed on an issue arrives as the
/// latter, and stripping 152 bytes off half a gigabyte by hand to replay it is
/// a step nobody should have to work out.
fn read_cf32(path: &str) -> Vec<Complex32> {
    let mut f = std::fs::File::open(path).unwrap_or_else(|e| {
        eprintln!("{path}: {e}");
        std::process::exit(1);
    });
    let mut raw = Vec::new();
    f.read_to_end(&mut raw).expect("read");
    let body = wav_payload(&raw).unwrap_or(&raw);
    body.chunks_exact(8)
        .map(|c| {
            Complex32::new(
                f32::from_le_bytes([c[0], c[1], c[2], c[3]]),
                f32::from_le_bytes([c[4], c[5], c[6], c[7]]),
            )
        })
        .collect()
}

/// The samples inside a RIFF/WAVE file, or `None` if this is not one.
///
/// Only the chunk walk, not a WAV reader: the recorder writes 32-bit float
/// stereo and anything else here would be a different file altogether, so a
/// wrong format is refused rather than misread as I/Q.
fn wav_payload(raw: &[u8]) -> Option<&[u8]> {
    if raw.len() < 12 || &raw[..4] != b"RIFF" || &raw[8..12] != b"WAVE" {
        return None;
    }
    let u16_at = |o: usize| u16::from_le_bytes([raw[o], raw[o + 1]]);
    let u32_at =
        |o: usize| u32::from_le_bytes([raw[o], raw[o + 1], raw[o + 2], raw[o + 3]]) as usize;
    let mut pos = 12;
    let mut fmt_ok = false;
    while pos + 8 <= raw.len() {
        let id = &raw[pos..pos + 4];
        let len = u32_at(pos + 4);
        let body = pos + 8;
        if id == b"fmt " && len >= 16 {
            let (format, channels, bits) = (u16_at(body), u16_at(body + 2), u16_at(body + 14));
            // 3 is IEEE float; 0xFFFE is WAVE_FORMAT_EXTENSIBLE, whose real
            // format sits in the extension and which this walk does not read.
            if !(format == 3 || format == 0xfffe) || channels != 2 || bits != 32 {
                eprintln!(
                    "the WAV is format {format}, {channels} channels, {bits} bits — \
                     this reads 32-bit float stereo I/Q only"
                );
                std::process::exit(2);
            }
            fmt_ok = true;
        }
        if id == b"data" {
            if !fmt_ok {
                return None;
            }
            let end = (body + len).min(raw.len());
            return Some(&raw[body..end]);
        }
        // Chunks are word-aligned, and an odd length is followed by a pad byte.
        pos = body + len + (len & 1);
    }
    None
}
