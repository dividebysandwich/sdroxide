//! Two `TncController`s running Data2G on a simulated half-duplex channel, each behind a
//! model of the engine: key-up takes time, the transmitter plays through an
//! output buffer, and after unkey the station is deaf for a whole receive
//! block. This is the bridge the controller exists to make — the modem is
//! sample-clocked, the engine is not — and the ARQ session
//! only survives it if the modem's clock is kept on the wall clock across
//! every gap.
//!
//! Runs faster than real time: the controllers read a simulated clock.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime};

use sdroxide_digi::tnc_controller::Clock;
use sdroxide_digi::{DigiAction, DigiEngine, TncController};
use sdroxide_tnc::TncModem;
use sdroxide_types::{DigiConfig, Mode, TncLineKind, TncStatus};

// The modems' own rate, so the simulated air needs no resampling.
const RATE: usize = 8_000;
const TICK: usize = 80; // 10 ms, the engine's transmit pacing

/// One station's radio as the engine presents it.
struct Rig {
    ctl: TncController,
    /// Samples per receive read.
    block: usize,
    /// Key-up: the engine's interlock chain and the rig's CAT round trips.
    key_delay: usize,
    /// Transmit output buffer: a sample handed over at t is on the air at
    /// t + this.
    out_latency: usize,
    /// What this station put on the air, by absolute sample index.
    air: Vec<f32>,
    keyed_at: Option<usize>,
    transmitting: bool,
    /// The burst has ended; the engine is draining the output buffer.
    draining_until: Option<usize>,
    /// Deaf until this sample (one receive block after unkey).
    deaf_until: usize,
    next_read: usize,
    status: Option<TncStatus>,
}

impl Rig {
    fn new(
        call: &str,
        listen: bool,
        clock: Clock,
        block: usize,
        key_delay: usize,
        out_latency: usize,
    ) -> Self {
        let mut cfg = DigiConfig::default();
        cfg.tnc_mycall = call.into();
        cfg.tnc_listen = listen;
        let seed = call.bytes().map(u64::from).sum();
        let modem = sdroxide_data2g::Data2g::new(call, Some(seed), 0.05)
            .map(|m| Box::new(m) as Box<dyn TncModem>);
        let mut ctl = TncController::new(Mode::Data2g, cfg, RATE as f64, modem);
        ctl.set_clock(clock);
        Rig {
            ctl,
            block,
            key_delay,
            out_latency,
            air: Vec::new(),
            keyed_at: None,
            transmitting: false,
            draining_until: None,
            deaf_until: 0,
            next_read: block,
            status: None,
        }
    }

    fn put_air(&mut self, at: usize, s: &[f32]) {
        if self.air.len() < at + s.len() {
            self.air.resize(at + s.len(), 0.0);
        }
        self.air[at..at + s.len()].copy_from_slice(s);
    }

    /// One engine tick at sample `now`, hearing `other`'s air.
    fn tick(&mut self, now: usize, other: &[f32], noise: &mut u64) {
        for a in self.ctl.poll(SystemTime::now(), 0.0) {
            match a {
                DigiAction::KeyTx => self.keyed_at = Some(now + self.key_delay),
                DigiAction::Status(s) => self.status = s.tnc,
                _ => {}
            }
        }
        if let Some(at) = self.keyed_at
            && now >= at
        {
            self.keyed_at = None;
            self.transmitting = true;
        }
        if self.transmitting {
            let mut buf = vec![0.0f32; TICK];
            let done = self.ctl.fill_tx_block(&mut buf);
            self.put_air(now + self.out_latency, &buf);
            if done {
                self.transmitting = false;
                self.draining_until = Some(now + self.out_latency);
            }
        }
        if let Some(t) = self.draining_until
            && now >= t
        {
            self.draining_until = None;
            self.ctl.on_burst_done();
            self.deaf_until = now + self.block;
            self.next_read = now + self.block;
        }
        // The receive side: a block arrives when its last sample has.
        let rx_live =
            !self.transmitting && self.keyed_at.is_none() && self.draining_until.is_none();
        while self.next_read <= now {
            let end = self.next_read;
            self.next_read += self.block;
            if !rx_live || end <= self.deaf_until {
                continue;
            }
            let start = end - self.block;
            let blk: Vec<f32> = (start..end)
                .map(|i| {
                    *noise ^= *noise << 13;
                    *noise ^= *noise >> 7;
                    *noise ^= *noise << 17;
                    let n = ((*noise >> 11) as f64 / (1u64 << 53) as f64 - 0.5) as f32 * 0.01;
                    other.get(i).copied().unwrap_or(0.0) * 0.5 + n
                })
                .collect();
            self.ctl.on_rx_audio(&blk);
        }
    }

    fn lines(&self, kind: TncLineKind) -> Vec<String> {
        self.status
            .as_ref()
            .map(|s| {
                s.transcript.iter().filter(|l| l.kind == kind).map(|l| l.text.clone()).collect()
            })
            .unwrap_or_default()
    }

    fn connected(&self) -> bool {
        self.status.as_ref().is_some_and(|s| s.connected)
    }
}

fn link(block: usize, key_delay_ms: usize, out_latency_ms: usize) {
    let base = Instant::now();
    let t = Arc::new(AtomicU64::new(0));
    let clock: Clock = {
        let t = t.clone();
        Arc::new(move || base + Duration::from_nanos(t.load(Ordering::SeqCst)))
    };
    let ms = RATE / 1000;
    let mut a =
        Rig::new("OE1AAA", false, clock.clone(), block, key_delay_ms * ms, out_latency_ms * ms);
    let mut b = Rig::new("OE1BBB", true, clock, block, key_delay_ms * ms, out_latency_ms * ms);
    let mut noise = 0x9E37_79B9_7F4A_7C15u64;

    let mut now = 0usize;
    let mut step = |a: &mut Rig, b: &mut Rig, now: &mut usize| {
        *now += TICK;
        t.store((*now as u64) * 1_000_000_000 / RATE as u64, Ordering::SeqCst);
        let b_air = std::mem::take(&mut b.air);
        a.tick(*now, &b_air, &mut noise);
        b.air = b_air;
        let a_air = std::mem::take(&mut a.air);
        b.tick(*now, &a_air, &mut noise);
        a.air = a_air;
    };

    // Let B claim the modem for listening, then call it.
    for _ in 0..20 {
        step(&mut a, &mut b, &mut now);
    }
    a.ctl.tnc_connect("OE1BBB".into());
    let deadline = now + 90 * RATE;
    while now < deadline && !(a.connected() && b.connected()) {
        step(&mut a, &mut b, &mut now);
    }
    assert!(
        a.connected() && b.connected(),
        "no link (block {block}, key {key_delay_ms} ms, out {out_latency_ms} ms): A {:?} / {:?}; B {:?}",
        a.lines(TncLineKind::Event),
        a.lines(TncLineKind::Error),
        b.lines(TncLineKind::Event),
    );

    let msg = "Hello from A, over a link that keys and unkeys like a real radio does.";
    for _ in 0..5 {
        a.ctl.tnc_send(msg.into());
    }
    let deadline = now + 240 * RATE;
    let got = |b: &Rig| b.lines(TncLineKind::Rx).concat().matches("Hello from A").count();
    while now < deadline && got(&b) < 5 {
        step(&mut a, &mut b, &mut now);
    }
    assert_eq!(
        got(&b),
        5,
        "B heard {:?}; A errors {:?}",
        b.lines(TncLineKind::Rx),
        a.lines(TncLineKind::Error)
    );

    b.ctl.tnc_send("73".into());
    let deadline = now + 120 * RATE;
    while now < deadline && !a.lines(TncLineKind::Rx).concat().contains("73") {
        step(&mut a, &mut b, &mut now);
    }
    assert!(
        a.lines(TncLineKind::Rx).concat().contains("73"),
        "A heard {:?}",
        a.lines(TncLineKind::Rx)
    );

    a.ctl.tnc_disconnect();
    let deadline = now + 60 * RATE;
    while now < deadline && (a.connected() || b.connected()) {
        step(&mut a, &mut b, &mut now);
    }
    assert!(!a.connected() && !b.connected(), "no clean hang-up");
    eprintln!(
        "block {block}, key {key_delay_ms} ms, out {out_latency_ms} ms: done at {:.1} s",
        now as f64 / RATE as f64
    );
}

/// An IQ SDR: quick key-up, short reads, almost no output buffer.
#[test]
fn a_link_survives_sdr_turnaround() {
    link(160, 10, 20);
}

/// A sound-card CAT rig reading in the small blocks the controller asks for
/// (~20 ms), with a CAT key-up and a sound card's output buffer.
#[test]
fn a_link_survives_cat_rig_turnaround() {
    link(160, 200, 120);
}

// No test at the engine's default 16384-sample read: at 341 ms deaf after
// every over the handshake fails outright, which is why the controller asks
// for short reads (`low_latency_rx`) and why
// `sdroxide-radio/tests/tx_turnaround.rs` checks the engine honours it.
