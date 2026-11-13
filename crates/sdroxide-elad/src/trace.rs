//! A self-contained session trace, so a user with an ELAD can produce a report
//! that a developer without one can read.
//!
//! This backend was written from ELAD's own GNU Radio module and their CAT
//! manual rather than on a bench, which makes the trace load-bearing rather
//! than a nicety. Everything here also goes to `tracing`, but `tracing` is the
//! wrong shape for a bug report: it needs `RUST_LOG` set before the run, and
//! asking somebody to reproduce an open failure with the right filter set is
//! asking them to reproduce it twice. A [`Trace`] is always on, bounded, and
//! dumps as one text block.
//!
//! # What it records
//!
//! Every vendor request with its `wValue`, `wIndex`, the length asked for, the
//! length that came back and either the decode or the exact transfer error; the
//! whole EEPROM calibration; the tuning arithmetic for the current dial
//! including the three fields actually sent; and the head of the first bulk
//! completion both as hex and as decoded `(re, im)` pairs.
//!
//! The two questions this driver cannot answer for itself are both settled from
//! that last line and from [`Trace::measured_rate`]: whether the samples really
//! are `I` before `Q`, and what rate the DDC was actually running at.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use sdroxide_dsp::Complex32;

/// What the settings panel tells an operator, and what the manual repeats.
pub const FIELD_REPORT_HINT: &str = "Only receiving on an FDM-DUO has been run on \
    real hardware. If it misbehaves, please attach the session trace \
    (Settings → Radio → Copy diagnostic report) to a bug report — it contains \
    every command exchanged with the receiver.";

#[derive(Debug, Default)]
struct Inner {
    lines: VecDeque<String>,
    dropped: u64,
    identity: Option<String>,
}

/// A shared, bounded record of one ELAD session.
///
/// Cloning shares the same buffer — the stream thread and the `IqSource` both
/// hold one.
#[derive(Debug, Clone)]
pub struct Trace {
    inner: Arc<Mutex<Inner>>,
    started: Instant,
}

impl Default for Trace {
    fn default() -> Self {
        Trace::new()
    }
}

impl Trace {
    /// How many lines are kept before the oldest are discarded. At roughly 80
    /// bytes a line this is a few hundred kilobytes — small enough to hold
    /// permanently, long enough to cover an open plus several minutes of use.
    pub const CAPACITY: usize = 4000;

    pub fn new() -> Trace {
        Trace { inner: Arc::new(Mutex::new(Inner::default())), started: Instant::now() }
    }

    /// Record a free-form event.
    pub fn note(&self, what: impl AsRef<str>) {
        self.push(format!("{:>9.3}  {}", self.started.elapsed().as_secs_f64(), what.as_ref()));
    }

    /// Record a control transfer and what came of it.
    ///
    /// `got` is the reply length for an IN request and `None` for an OUT one.
    /// It is a separate column rather than part of the outcome text because
    /// "asked for 32, got 8" is the shape of every framing surprise a vendor
    /// protocol has, and it must be greppable.
    // Eight columns, and every one of them is a field of the USB setup packet
    // or its outcome. A struct here would be a struct with eight fields.
    #[allow(clippy::too_many_arguments)]
    pub fn ctrl(
        &self,
        request: u8,
        name: &str,
        value: u16,
        index: u16,
        want: usize,
        got: Option<usize>,
        outcome: &str,
    ) {
        let len = match got {
            Some(n) if n == want => format!("len {n}"),
            Some(n) => format!("len {n}/{want} SHORT"),
            None => format!("len {want}"),
        };
        self.note(format!(
            "req 0x{request:02X} {name:<20} val 0x{value:04x} idx 0x{index:04x} {len:<16} {outcome}"
        ));
        tracing::debug!(target: "elad::ctrl", request, name, value, index, want, ?got, outcome);
    }

    /// Store the one-off identity summary. Kept out of the ring so a long
    /// session cannot age it out — it is the first thing anybody reading a
    /// report needs.
    pub fn set_identity(&self, summary: impl Into<String>) {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).identity = Some(summary.into());
    }

    /// Record raw bytes as hex.
    pub fn wire_head(&self, what: &str, bytes: &[u8], n: usize) {
        self.note(format!("▷ {what}: {}", hex_head(bytes, n)));
    }

    /// The head of the first bulk completion, hex *and* decoded.
    ///
    /// Point the receiver at a known strong carrier and these two lines
    /// together answer a question this driver cannot answer for itself: are the
    /// samples really I before Q? A mirrored spectrum and a correct one look
    /// identical in hex.
    pub fn first_samples(&self, bytes: &[u8], decoded: &[Complex32]) {
        self.note(format!("▷ first bulk bytes: {}", hex_head(bytes, 64)));
        let pairs: Vec<String> =
            decoded.iter().take(8).map(|s| format!("({:+.4},{:+.4})", s.re, s.im)).collect();
        self.note(format!("▷ decoded as (re,im): {}", pairs.join(" ")));
    }

    /// The rate the stream was measured at against the rate it is being read
    /// as.
    ///
    /// The single most useful line in the file for this backend. On a sampler
    /// it says whether the FPGA image [`crate::fpga`] loaded is the one that
    /// went in; on the transceiver, whose decimation nothing here can command,
    /// the configured rate is a *guess* at what the radio was left in and this
    /// is the only evidence of whether the guess was right.
    pub fn measured_rate(&self, configured_hz: f64, measured_hz: f64) {
        let err =
            if configured_hz > 0.0 { (measured_hz / configured_hz - 1.0) * 100.0 } else { 0.0 };
        self.note(format!(
            "▷ stream rate: reading as {configured_hz:.0} Hz, measured {measured_hz:.0} Hz \
             ({err:+.1}%)"
        ));
    }

    /// Render the whole trace as one text block, ready to paste into an issue.
    pub fn dump(&self) -> String {
        let g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let mut out = String::with_capacity(64 * 1024);
        out.push_str("=== sdroxide ELAD session trace ===\n");
        out.push_str(&format!("elapsed: {:.1} s\n", self.started.elapsed().as_secs_f64()));
        if g.dropped > 0 {
            out.push_str(&format!(
                "note: {} earlier lines were discarded (ring holds {})\n",
                g.dropped,
                Trace::CAPACITY
            ));
        }
        out.push_str("\n--- identity ---\n");
        match &g.identity {
            Some(c) => {
                out.push_str(c);
                out.push('\n');
            }
            None => out.push_str("(the device was never identified)\n"),
        }
        // The FDM-DUO has no loader: it boots its own image, and wider spans
        // come from ELAD's DLL. Only the FDM-S samplers use `elad-firmware`.
        let duo = g.identity.as_deref().is_some_and(|c| c.contains("FDM-DUO"));
        if duo {
            out.push_str("\n--- fpga image (FDM-DUO) ---\n");
            out.push_str(&crate::remote::status_line());
        } else {
            out.push_str("\n--- fpga loader ---\n");
            out.push_str(&crate::fpga::status_line());
        }
        out.push('\n');
        out.push_str("\n--- session ---\n");
        for line in &g.lines {
            out.push_str(line);
            out.push('\n');
        }
        out
    }

    fn push(&self, line: String) {
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if g.lines.len() >= Trace::CAPACITY {
            g.lines.pop_front();
            g.dropped += 1;
        }
        g.lines.push_back(line);
    }
}

/// Format the first `n` bytes as spaced uppercase hex.
pub fn hex_head(bytes: &[u8], n: usize) -> String {
    bytes.iter().take(n).map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")
}

/// Where the always-on traces of the most recent sessions live, so the settings
/// UI can offer them as copyable text after the device has already failed and
/// the handle has been dropped.
///
/// Open sessions and probes get separate slots. With one slot, a Rescan or a
/// `--probe` after a streaming fault would replace the streaming trace — the
/// entire point of the report — with an enumeration.
static LAST_OPEN: Mutex<Option<Trace>> = Mutex::new(None);
static LAST_PROBE: Mutex<Option<Trace>> = Mutex::new(None);

/// Remember an open (streaming) session's trace. The stored clone shares the
/// live buffer, so the slot keeps filling for as long as the session runs.
pub fn remember(trace: &Trace) {
    *LAST_OPEN.lock().unwrap_or_else(|e| e.into_inner()) = Some(trace.clone());
}

/// Remember a probe's trace, in its own slot so it cannot displace the evidence
/// of an open session.
pub fn remember_probe(trace: &Trace) {
    *LAST_PROBE.lock().unwrap_or_else(|e| e.into_inner()) = Some(trace.clone());
}

/// The most recent session traces, for a bug report: the open session first —
/// it is the one with streaming evidence — then the most recent probe. `None`
/// before the first attempt of either kind.
pub fn diagnostics() -> Option<String> {
    let open = LAST_OPEN.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let probe = LAST_PROBE.lock().unwrap_or_else(|e| e.into_inner()).clone();
    if open.is_none() && probe.is_none() {
        return None;
    }
    let mut out = String::new();
    if let Some(t) = open {
        out.push_str("### radio session (open / stream)\n");
        out.push_str(&t.dump());
    }
    if let Some(t) = probe {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str("### probe\n");
        out.push_str(&t.dump());
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dump_reports_requests_and_the_identity() {
        let t = Trace::new();
        t.set_identity("ELAD FDM-DUO, serial 123456, hardware 1.4");
        t.ctrl(0xA2, "EEPROM serial", 0x4000, 0x0151, 32, Some(32), "\"123456\"");
        let d = t.dump();
        assert!(d.contains("req 0xA2 EEPROM serial"), "{d}");
        assert!(d.contains("123456"), "{d}");
        // The DUO boots its own FPGA image: the FDM-S loader line would only mislead.
        assert!(d.contains("fpga image (FDM-DUO)"), "{d}");
        assert!(!d.contains("elad-firmware"), "{d}");
    }

    #[test]
    fn an_fdm_s_still_reports_its_fpga_loader() {
        let t = Trace::new();
        t.set_identity("ELAD FDM-S2, serial 123456");
        assert!(t.dump().contains("--- fpga loader ---"));
    }

    #[test]
    fn a_short_reply_is_called_out_in_the_length_column() {
        let t = Trace::new();
        t.ctrl(0xA2, "EEPROM global", 0x4028, 0x0151, 4, Some(2), "short");
        assert!(t.dump().contains("len 2/4 SHORT"));
        // An exact reply is not flagged, or the flag would mean nothing.
        let t2 = Trace::new();
        t2.ctrl(0xA2, "EEPROM global", 0x4028, 0x0151, 4, Some(4), "+1.0 dB");
        assert!(!t2.dump().contains("SHORT"));
        // An OUT request has no reply length to compare against.
        let t3 = Trace::new();
        t3.ctrl(0xE1, "tune", 0x5678, 0xF234, 2, None, "14074 kHz");
        assert!(!t3.dump().contains("SHORT"));
    }

    /// The identity must survive a session long enough to overflow the ring —
    /// it is the first thing a reader needs and the ring would eat it.
    #[test]
    fn the_identity_outlives_the_line_ring() {
        let t = Trace::new();
        t.set_identity("ELAD FDM-DUO");
        for i in 0..(Trace::CAPACITY + 50) {
            t.note(format!("line {i}"));
        }
        let d = t.dump();
        assert!(d.contains("50 earlier lines were discarded"));
        assert!(d.contains("ELAD FDM-DUO"));
        assert!(d.contains(&format!("line {}", Trace::CAPACITY + 49)));
    }

    /// Hex alone cannot tell a mirrored spectrum from a correct one; the
    /// decoded pairs beside it can.
    #[test]
    fn the_first_samples_are_shown_as_bytes_and_as_numbers() {
        let t = Trace::new();
        let bytes = [0x00u8, 0x00, 0x00, 0x40];
        t.first_samples(&bytes, &[Complex32::new(0.5, 0.25)]);
        let d = t.dump();
        assert!(d.contains("00 00 00 40"), "{d}");
        assert!(d.contains("(+0.5000,+0.2500)"), "{d}");
    }

    /// The rate cannot be commanded, so the measurement is the only evidence
    /// that the configured one is right. It has to be in the report whether it
    /// agrees or not.
    #[test]
    fn the_measured_rate_is_recorded_with_its_error() {
        let t = Trace::new();
        t.measured_rate(192_000.0, 384_100.0);
        let d = t.dump();
        assert!(d.contains("reading as 192000 Hz"), "{d}");
        assert!(d.contains("measured 384100 Hz"), "{d}");
        assert!(d.contains("+100.1%"), "{d}");
    }

    #[test]
    fn an_empty_trace_still_dumps() {
        assert!(Trace::new().dump().contains("never identified"));
    }

    /// The mistake this guards against: an operator whose stream is misbehaving
    /// presses Rescan to check the device is still there, and that click
    /// replaces the streaming trace — the entire point of the report — with an
    /// enumeration.
    #[test]
    fn a_probe_does_not_displace_an_open_sessions_trace() {
        let open = Trace::new();
        open.note("streaming evidence");
        remember(&open);
        let probe = Trace::new();
        probe.note("just an enumeration");
        remember_probe(&probe);
        let d = diagnostics().expect("both slots filled");
        let open_at = d.find("radio session").expect("open section present");
        let probe_at = d.find("### probe").expect("probe section present");
        assert!(open_at < probe_at, "the open session must come first:\n{d}");
        assert!(d.contains("streaming evidence"), "{d}");
        assert!(d.contains("just an enumeration"), "{d}");
    }
}
