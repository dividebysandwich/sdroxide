//! The announcer, driven by a synthetic clock and read back through a
//! recording sink.
//!
//! Every entry point takes `now` as an `f64`, so there is no `Instant`
//! anywhere and every case below is exact rather than timing-dependent.

use sdroxide_speech::announce::Announcer;
use sdroxide_speech::sink::{RecordingSink, SpeechLog};
use sdroxide_types::{
    AgcMode, Band, CwStatus, Decode, DigiConfig, DigiStatus, Meters, Mode, QsoStep, RadioState,
    SpeechSettings, TxMeters, Vfo,
};

fn harness() -> (Announcer, SpeechLog, RadioState) {
    let mut cfg = SpeechSettings::default();
    cfg.enabled = true;
    cfg.text.cw = true;
    cfg.text.rtty_psk = true;
    // The read-along tests want lock gating out of the way unless they ask.
    cfg.text.cw_only_when_locked = false;
    let (sink, log) = RecordingSink::new();
    let a = Announcer::with_sink(cfg, Box::new(sink));
    (a, log, RadioState::default())
}

fn with_cfg(f: impl FnOnce(&mut SpeechSettings)) -> (Announcer, SpeechLog, RadioState) {
    let mut cfg = SpeechSettings::default();
    cfg.enabled = true;
    f(&mut cfg);
    let (sink, log) = RecordingSink::new();
    (Announcer::with_sink(cfg, Box::new(sink)), log, RadioState::default())
}

/// Deliver a snapshot and run the tick that follows it.
fn step(a: &mut Announcer, s: &RadioState, now: f64) {
    a.on_state(s, now);
    a.tick(s, None, now);
}

fn meters(swr: Option<f32>, fwd: Option<f32>, keyed: bool) -> Meters {
    Meters {
        s_dbm: -100.0,
        pa_temp_c: None,
        adc_peak_dbfs: -30.0,
        adc_clip: 0.0,
        tx: keyed.then_some(TxMeters { fwd_w: fwd, swr, alc: 0.0, po: None }),
        stereo: false,
        tone: None,
    }
}

// ── seeding and the scroll wheel ────────────────────────────────────────

#[test]
fn the_first_state_snapshot_is_seeded_not_announced() {
    let (mut a, log, s) = harness();
    // Attaching to a radio must not make it recite its whole configuration.
    step(&mut a, &s, 0.0);
    assert!(log.is_empty(), "announced on connect: {:?}", log.all());
}

#[test]
fn a_scroll_does_not_chatter() {
    let (mut a, log, mut s) = harness();
    step(&mut a, &s, 0.0);

    // Forty detents, thirty milliseconds apart — the rate the engine really
    // echoes state back at while the wheel is turning.
    for i in 1..=40 {
        s.vfo_a_hz = 14_074_000.0 + i as f64 * 100.0;
        step(&mut a, &s, i as f64 * 0.03);
    }
    assert!(log.is_empty(), "spoke mid-scroll: {:?}", log.all());

    // ...then the operator lets go.
    a.tick(&s, None, 40.0 * 0.03 + 0.7);
    let said = log.all();
    assert_eq!(said.len(), 1, "expected one announcement, got {said:?}");
    assert_eq!(said[0], "fourteen point zero seven eight");
}

#[test]
fn settling_back_where_you_started_says_nothing() {
    let (mut a, log, mut s) = harness();
    step(&mut a, &s, 0.0);
    let home = s.vfo_a_hz;

    s.vfo_a_hz = home + 2000.0;
    step(&mut a, &s, 0.1);
    s.vfo_a_hz = home;
    step(&mut a, &s, 0.2);
    a.tick(&s, None, 1.0);
    assert!(log.is_empty(), "spoke about a round trip: {:?}", log.all());
}

#[test]
fn the_same_snapshot_twice_announces_once() {
    let (mut a, log, mut s) = harness();
    step(&mut a, &s, 0.0);
    s.split = true;
    // The optimistic local echo means the identical snapshot can arrive twice.
    step(&mut a, &s, 0.1);
    step(&mut a, &s, 0.2);
    a.tick(&s, None, 1.0);
    assert_eq!(log.all().iter().filter(|t| t.contains("split")).count(), 1);
}

// ── composite phrases ───────────────────────────────────────────────────

#[test]
fn a_band_change_folds_into_one_phrase() {
    let (mut a, log, mut s) = harness();
    step(&mut a, &s, 0.0);

    // One button press moves band, frequency and mode together.
    s.band = Band::M40;
    s.vfo_a_hz = 7_100_000.0;
    s.rx[0].mode = Mode::Lsb;
    step(&mut a, &s, 0.1);
    a.tick(&s, None, 1.0);

    let said = log.all();
    assert_eq!(said.len(), 1, "expected one phrase for one button press: {said:?}");
    assert_eq!(said[0], "forty meters, seven point one zero zero, L S B");
}

#[test]
fn a_vfo_swap_reads_the_new_frequency() {
    let (mut a, log, mut s) = harness();
    s.vfo_a_hz = 14_074_000.0;
    s.vfo_b_hz = 7_140_000.0;
    step(&mut a, &s, 0.0);

    s.active_vfo = Vfo::B;
    s.band = Band::M40;
    step(&mut a, &s, 0.1);
    a.tick(&s, None, 1.0);

    // The band change owns the phrase; the frequency rides with it either way,
    // and the frequency latch must not then repeat itself.
    let said = log.all();
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].contains("seven point one four zero"), "{said:?}");
}

#[test]
fn discrete_settings_speak_immediately() {
    let (mut a, log, mut s) = harness();
    step(&mut a, &s, 0.0);
    s.rx[0].agc = AgcMode::Slow;
    step(&mut a, &s, 0.1);
    assert_eq!(log.all(), ["A G C slow"]);
}

#[test]
fn levels_wait_for_the_slider_to_stop() {
    let (mut a, log, mut s) = harness();
    step(&mut a, &s, 0.0);
    for i in 1..=20 {
        s.tx.drive = 0.1 + i as f32 * 0.01;
        step(&mut a, &s, i as f64 * 0.02);
    }
    assert!(log.is_empty(), "spoke mid-drag: {:?}", log.all());
    a.tick(&s, None, 1.5);
    assert_eq!(log.all(), ["drive thirty percent"]);
}

// ── the band edge ───────────────────────────────────────────────────────

#[test]
fn leaving_an_amateur_band_warns_once() {
    let (mut a, log, mut s) = harness();
    s.vfo_a_hz = 14_074_000.0;
    step(&mut a, &s, 0.0);

    // Tune down out of 20 m.
    s.vfo_a_hz = 13_900_000.0;
    s.band = Band::Gen;
    step(&mut a, &s, 0.1);
    a.tick(&s, None, 1.0);
    assert!(log.any("outside the band"), "{:?}", log.all());

    // Sitting there must not repeat it.
    log.take();
    for i in 0..10 {
        step(&mut a, &s, 2.0 + i as f64 * 0.1);
    }
    a.tick(&s, None, 5.0);
    assert!(!log.any("outside the band"), "repeated the warning: {:?}", log.all());
}

#[test]
fn keying_outside_a_band_alerts_immediately_without_settling() {
    let (mut a, log, mut s) = with_cfg(|c| c.duck_on_ptt = false);
    s.vfo_a_hz = 13_900_000.0;
    s.band = Band::Gen;
    step(&mut a, &s, 0.0);
    log.take();

    // 350 ms of politeness here is 350 ms of transmitting where you should not.
    s.tx.ptt = true;
    a.on_state(&s, 0.1);
    a.tick(&s, None, 0.1);
    assert!(log.any("outside the band"), "{:?}", log.all());
}

// ── SWR while tuning ────────────────────────────────────────────────────

#[test]
fn swr_ticks_every_two_seconds_while_tuning() {
    let (mut a, log, mut s) = harness();
    step(&mut a, &s, 0.0);

    s.tx.tune = true;
    a.on_state(&s, 1.0);
    a.tick(&s, None, 1.0);
    log.take();

    let m = meters(Some(1.4), Some(20.0), true);
    // Feed meters at 10 Hz for six seconds.
    let mut ticks = 0;
    for i in 0..60 {
        let now = 1.0 + i as f64 * 0.1;
        a.on_meters(&m, &s, now);
        a.tick(&s, Some(&m), now);
    }
    for t in log.all() {
        if t.contains("to one") {
            ticks += 1;
        }
    }
    // 0.8 s settle then one every 2 s over the remaining ~5.2 s.
    assert!((2..=3).contains(&ticks), "expected two or three readings, got {ticks}");
    assert!(log.any("one point four to one"), "{:?}", log.all());
}

#[test]
fn the_key_up_transient_is_skipped() {
    let (mut a, log, mut s) = harness();
    step(&mut a, &s, 0.0);
    s.tx.tune = true;
    a.on_state(&s, 1.0);
    a.tick(&s, None, 1.0);
    log.take();

    // The PA is still ramping: the bridge reads badly and it means nothing.
    let bad = meters(Some(8.0), Some(1.0), true);
    for i in 0..7 {
        let now = 1.0 + i as f64 * 0.1;
        a.on_meters(&bad, &s, now);
        a.tick(&s, Some(&bad), now);
    }
    assert!(!log.any("high S W R"), "alarmed on the key-up transient: {:?}", log.all());
}

#[test]
fn a_rig_without_a_bridge_says_so_once_and_then_stops() {
    let (mut a, log, mut s) = harness();
    s.tx.tune_drive = 0.05;
    step(&mut a, &s, 0.0);
    s.tx.tune = true;
    a.on_state(&s, 1.0);
    a.tick(&s, None, 1.0);

    // Keyed and reporting, but with no SWR figure in the report.
    let m = meters(None, Some(5.0), true);
    for i in 0..100 {
        let now = 1.0 + i as f64 * 0.1;
        a.on_meters(&m, &s, now);
        a.tick(&s, Some(&m), now);
    }
    let n = log.all().iter().filter(|t| t.contains("no S W R reading")).count();
    assert_eq!(n, 1, "expected exactly one, got {n}: {:?}", log.all());
}

#[test]
fn the_high_swr_alarm_latches_and_clears_with_hysteresis() {
    let (mut a, log, mut s) = harness();
    step(&mut a, &s, 0.0);
    s.tx.tune = true;
    a.on_state(&s, 1.0);
    a.tick(&s, None, 1.0);

    let feed = |a: &mut Announcer, s: &RadioState, swr: f32, from: f64, n: usize| {
        let m = meters(Some(swr), Some(20.0), true);
        for i in 0..n {
            let now = from + i as f64 * 0.1;
            a.on_meters(&m, s, now);
            a.tick(s, Some(&m), now);
        }
    };

    feed(&mut a, &s, 4.2, 2.0, 10);
    assert!(log.any("high S W R"), "did not alarm: {:?}", log.all());
    log.take();

    // Still latched between the thresholds — this is the whole point of the
    // hysteresis band.
    feed(&mut a, &s, 2.7, 3.0, 10);
    assert!(!log.any("S W R normal"), "cleared inside the band: {:?}", log.all());

    // Below the clear threshold, and held there.
    feed(&mut a, &s, 1.5, 4.0, 20);
    assert!(log.any("S W R normal"), "never cleared: {:?}", log.all());
}

#[test]
fn the_alarm_speaks_through_the_transmit_gag() {
    let (mut a, log, mut s) = harness();
    step(&mut a, &s, 0.0);
    // Keyed for real, not tuning: everything below an alert is refused.
    s.tx.ptt = true;
    a.on_state(&s, 1.0);
    a.tick(&s, None, 1.0);
    log.take();

    let m = meters(Some(5.0), Some(50.0), true);
    for i in 0..20 {
        let now = 2.0 + i as f64 * 0.1;
        a.on_meters(&m, &s, now);
        a.tick(&s, Some(&m), now);
    }
    assert!(log.any("high S W R"), "the gag swallowed the alarm: {:?}", log.all());
}

#[test]
fn the_tune_summary_reports_the_minimum_not_the_last() {
    let (mut a, log, mut s) = harness();
    step(&mut a, &s, 0.0);
    s.tx.tune = true;
    a.on_state(&s, 1.0);
    a.tick(&s, None, 1.0);

    // An ATU sweep: down to a good match, then back up to wherever it stopped.
    for (swr, t) in [(6.0f32, 2.0), (2.5, 2.5), (1.2, 3.0), (1.2, 3.5), (4.0, 4.0)] {
        let m = meters(Some(swr), Some(20.0), true);
        for i in 0..5 {
            let now = t + i as f64 * 0.05;
            a.on_meters(&m, &s, now);
            a.tick(&s, Some(&m), now);
        }
    }
    log.take();

    s.tx.tune = false;
    a.on_state(&s, 5.0);
    // The queue hands one utterance to the sink per tick; "tune off" is ahead
    // of the summary, so let a few frames go by as they would in the app.
    for i in 0..5 {
        a.tick(&s, None, 5.0 + i as f64 * 0.02);
    }
    let said = log.all();
    let summary = said.iter().find(|t| t.starts_with("tuned")).expect("no summary");
    assert!(summary.contains("one point"), "reported the last sample, not the best: {summary}");
}

// ── decoded messages ────────────────────────────────────────────────────

fn digi_with_call(call: &str) -> DigiStatus {
    // `DigiStatus` has no `Default` — `idle` is how the engine itself builds
    // one before any mode is entered.
    let mut cfg = DigiConfig::default();
    cfg.my_call = call.into();
    DigiStatus::idle(cfg)
}

fn decode(to: Option<&str>, from: &str, msg: &str, slot: i64) -> Decode {
    Decode {
        slot_utc: slot,
        snr_db: -12,
        dt: 0.1,
        audio_hz: 1200.0,
        message: msg.into(),
        to: to.map(str::to_string),
        from: Some(from.into()),
        grid: None,
        is_cq: false,
        cq_to: None,
        free_text: false,
        rr73_to: None,
    }
}

#[test]
fn an_ft8_message_to_me_is_spoken_once() {
    let (mut a, log, s) = harness();
    step(&mut a, &s, 0.0);
    let st = digi_with_call("OE3ABC");
    let d = decode(Some("OE3ABC"), "K1ABC", "OE3ABC K1ABC -12", 100);

    a.on_ft8(std::slice::from_ref(&d), &st, 1.0);
    a.tick(&s, None, 1.0);
    // Redelivered on reconnect: must not be read again.
    a.on_ft8(std::slice::from_ref(&d), &st, 2.0);
    a.tick(&s, None, 2.0);

    let n = log.all().iter().filter(|t| t.contains("kilo one alpha bravo charlie")).count();
    assert_eq!(n, 1, "{:?}", log.all());
    // What the message *was*, not merely that one arrived: this is their
    // report of us, and it is the number the operator has to write down.
    assert!(log.any("reports minus twelve"), "{:?}", log.all());
    // And only that number. Ours of them belongs on the call that opened the
    // contact, not appended to every message in it.
    assert_eq!(log.all()[0].matches("minus twelve").count(), 1, "{:?}", log.all());
}

/// Every stage of an exchange says which stage it is.
#[test]
fn each_message_of_a_qso_reads_as_itself() {
    for (msg, want) in [
        ("OE3ABC K1ABC JN88", "kilo one alpha bravo charlie calling you"),
        ("OE3ABC K1ABC -12", "kilo one alpha bravo charlie reports minus twelve"),
        ("OE3ABC K1ABC R-12", "kilo one alpha bravo charlie confirms minus twelve"),
        ("OE3ABC K1ABC RRR", "kilo one alpha bravo charlie rogers"),
        ("OE3ABC K1ABC RR73", "kilo one alpha bravo charlie rogers and seventy three"),
        ("OE3ABC K1ABC 73", "kilo one alpha bravo charlie says seventy three"),
        ("OE3ABC K1ABC HW CPY", "kilo one alpha bravo charlie calling you, how copy"),
    ] {
        let (mut a, log, s) = harness();
        step(&mut a, &s, 0.0);
        let st = digi_with_call("OE3ABC");
        a.on_ft8(&[decode(Some("OE3ABC"), "K1ABC", msg, 100)], &st, 1.0);
        a.tick(&s, None, 1.0);
        let said = log.all();
        assert!(said.iter().any(|t| t.starts_with(want)), "{msg:?} was read as {said:?}");
    }
}

// ── our own side of the exchange ────────────────────────────────────────

/// An FT8 status, as the sequencer broadcasts one mid-QSO.
fn ft8_status(call: &str, step: QsoStep, dx: Option<&str>, pending: Option<&str>) -> DigiStatus {
    let mut st = digi_with_call(call);
    st.mode = Mode::Ft8;
    st.step = step;
    st.dx_call = dx.map(str::to_string);
    st.tx_next = pending.is_some();
    st.tx_pending_msg = pending.map(str::to_string);
    st
}

/// A whole contact, both sides of it, in the order the operator hears it.
///
/// This is the test the two complaints behind this code are really about: the
/// old announcer said "kilo one alpha bravo charlie calling you, minus twelve"
/// five times over and never once said the QSO had finished.
#[test]
fn a_whole_exchange_reads_as_a_conversation() {
    let (mut a, log, s) = harness();
    step(&mut a, &s, 0.0);

    // A slot: their message, then the plan for ours, then the queue drains —
    // fifteen seconds of it, in a real one.
    let slot = |a: &mut Announcer, st: &DigiStatus, rx: Option<&str>, snr: i16, t: f64| {
        if let Some(msg) = rx {
            let mut d = decode(Some("OE3ABC"), "K1ABC", msg, 100 + t as i64);
            d.snr_db = snr;
            a.on_ft8(&[d], st, t);
        }
        a.on_digi(st, &s, t);
        for k in 0..6 {
            a.tick(&s, None, t + k as f64 * 0.05);
        }
    };

    let cq = ft8_status("OE3ABC", QsoStep::CallingCq, None, Some("CQ OE3ABC JN88"));
    // Sitting in FT8 with nothing to send: adopted, and silent.
    slot(&mut a, &ft8_status("OE3ABC", QsoStep::Idle, None, None), None, 0, 1.0);
    slot(&mut a, &cq, None, 0, 2.0);
    let tx_rpt = ft8_status("OE3ABC", QsoStep::TxReport, Some("K1ABC"), Some("K1ABC OE3ABC -09"));
    slot(&mut a, &tx_rpt, Some("OE3ABC K1ABC FN42"), -12, 3.0);
    let tx_r = ft8_status("OE3ABC", QsoStep::TxRReport, Some("K1ABC"), Some("K1ABC OE3ABC R-09"));
    slot(&mut a, &tx_r, Some("OE3ABC K1ABC -14"), -12, 4.0);
    let tx_73 = ft8_status("OE3ABC", QsoStep::TxRr73, Some("K1ABC"), Some("K1ABC OE3ABC RR73"));
    slot(&mut a, &tx_73, Some("OE3ABC K1ABC R-14"), -11, 5.0);
    let done = ft8_status("OE3ABC", QsoStep::Confirming, Some("K1ABC"), None);
    slot(&mut a, &done, Some("OE3ABC K1ABC 73"), -11, 6.0);

    assert_eq!(
        log.all(),
        [
            "calling C Q",
            "kilo one alpha bravo charlie calling you, minus twelve",
            "sending minus nine to kilo one alpha bravo charlie",
            "kilo one alpha bravo charlie reports minus fourteen",
            "sending roger minus nine",
            "kilo one alpha bravo charlie confirms minus fourteen",
            "sending roger roger seventy three",
            // No number on a sign-off: nothing is decided after one.
            "kilo one alpha bravo charlie says seventy three",
            "Q S O with kilo one alpha bravo charlie complete",
        ]
    );
}

#[test]
fn what_we_are_about_to_send_is_announced_once() {
    let (mut a, log, s) = harness();
    step(&mut a, &s, 0.0);
    // Attaching mid-QSO adopts the state without reciting it.
    a.on_digi(
        &ft8_status("OE3ABC", QsoStep::TxReport, Some("K1ABC"), Some("K1ABC OE3ABC -12")),
        &s,
        1.0,
    );
    a.tick(&s, None, 1.0);
    assert!(log.is_empty(), "recited the exchange on connect: {:?}", log.all());

    // The plan changes: the exchange has advanced, and that is worth saying.
    let st = ft8_status("OE3ABC", QsoStep::TxRReport, Some("K1ABC"), Some("K1ABC OE3ABC R-12"));
    a.on_digi(&st, &s, 2.0);
    a.tick(&s, None, 2.0);
    assert!(log.any("sending roger minus twelve"), "{:?}", log.all());

    // The same plan, slot after slot after slot: said once.
    let before = log.len();
    for (i, t) in [3.0, 4.0, 5.0].iter().enumerate() {
        let _ = i;
        a.on_digi(&st, &s, *t);
        a.tick(&s, None, *t);
    }
    assert_eq!(log.len(), before, "repeated an unchanged plan: {:?}", log.all());
}

#[test]
fn a_completed_qso_says_so() {
    let (mut a, log, s) = harness();
    step(&mut a, &s, 0.0);
    a.on_digi(&ft8_status("OE3ABC", QsoStep::TxRr73, Some("K1ABC"), None), &s, 1.0);
    a.tick(&s, None, 1.0);
    log.take();

    // `Confirming` is the sequencer's "logged".
    let done = ft8_status("OE3ABC", QsoStep::Confirming, Some("K1ABC"), None);
    a.on_digi(&done, &s, 2.0);
    a.tick(&s, None, 2.0);
    assert!(log.any("Q S O with kilo one alpha bravo charlie complete"), "{:?}", log.all());

    // The re-send window holds the contact in `Confirming` for minutes; the
    // contact only completes once.
    log.take();
    for t in [3.0, 4.0] {
        a.on_digi(&done, &s, t);
        a.tick(&s, None, t);
    }
    assert!(log.is_empty(), "said it again: {:?}", log.all());
}

/// The keyboard modes put the operator's own typing in `tx_pending_msg`.
/// Reading it back to them is not an announcement.
#[test]
fn a_typed_buffer_is_not_a_transmission_plan() {
    let (mut a, log, s) = harness();
    step(&mut a, &s, 0.0);
    let mut st = digi_with_call("OE3ABC");
    st.mode = Mode::Rtty;
    st.tx_next = true;
    st.tx_pending_msg = Some("HELLO OM".into());
    a.on_digi(&st, &s, 1.0);
    a.tick(&s, None, 1.0);
    st.tx_pending_msg = Some("HELLO OM ES 73".into());
    a.on_digi(&st, &s, 2.0);
    a.tick(&s, None, 2.0);
    assert!(log.is_empty(), "read the operator's own typing back: {:?}", log.all());
}

#[test]
fn an_ordinary_cq_is_not_spoken() {
    let (mut a, log, s) = harness();
    step(&mut a, &s, 0.0);
    let st = digi_with_call("OE3ABC");
    let mut d = decode(None, "K1ABC", "CQ K1ABC FN42", 100);
    d.is_cq = true;

    a.on_ft8(&[d], &st, 1.0);
    a.tick(&s, None, 1.0);
    assert!(log.is_empty(), "read out a CQ nobody asked for: {:?}", log.all());
}

#[test]
fn an_rr73_addressed_to_me_is_spoken() {
    let (mut a, log, s) = harness();
    step(&mut a, &s, 0.0);
    let st = digi_with_call("OE3ABC");
    // A Fox closing our contact: the RR73 names us in `rr73_to`, never in `to`.
    let mut d = decode(Some("W9XYZ"), "DX1FOX", "OE3ABC RR73; W9XYZ <DX1FOX> +03", 100);
    d.rr73_to = Some("OE3ABC".into());

    a.on_ft8(&[d], &st, 1.0);
    a.tick(&s, None, 1.0);
    assert!(!log.is_empty(), "missed the completion a Hound waits for");
}

#[test]
fn a_js8_message_is_spoken_only_when_complete() {
    use sdroxide_types::{Js8Msg, Js8Status};
    let (mut a, log, s) = harness();
    step(&mut a, &s, 0.0);

    let msg = |complete: bool, frames: u8| Js8Msg {
        from: "K1ABC".into(),
        to: "OE3ABC".into(),
        text: "HW CPY".into(),
        cmd: None,
        snr_db: -5,
        audio_hz: 1500.0,
        first_slot_utc: 100,
        last_slot_utc: 100 + frames as i64,
        frames,
        complete,
        to_me: true,
    };

    let mut st = digi_with_call("OE3ABC");
    st.js8 = Some(Js8Status { messages: vec![msg(false, 1)], ..Js8Status::default() });
    a.on_digi(&st, &s, 1.0);
    a.tick(&s, None, 1.0);
    assert!(log.is_empty(), "read a partial: {:?}", log.all());

    // The same message, finished. Spoken once, and not again on the next
    // snapshot carrying it.
    st.js8 = Some(Js8Status { messages: vec![msg(true, 3)], ..Js8Status::default() });
    a.on_digi(&st, &s, 2.0);
    a.tick(&s, None, 2.0);
    a.on_digi(&st, &s, 3.0);
    a.tick(&s, None, 3.0);
    assert_eq!(log.all().len(), 1, "{:?}", log.all());
    assert!(log.any("kilo one alpha bravo charlie"), "{:?}", log.all());
}

// ── the rolling read-along buffer ───────────────────────────────────────

/// The exact truncation `cw_controller.rs`, `text_modem.rs` and
/// `fsq_controller.rs` all apply to the shared buffer. Copied rather than
/// approximated: it is the specific behaviour the tail algorithm has to
/// survive, and copying it means this test fails loudly if the controllers
/// ever change their cap.
const RX_TEXT_CAP: usize = 8000;

fn truncate_like_the_controllers(rx_text: &mut String) {
    if rx_text.len() > RX_TEXT_CAP {
        let cut = rx_text.len() - RX_TEXT_CAP;
        let cut =
            (cut..rx_text.len()).find(|&i| rx_text.is_char_boundary(i)).unwrap_or(rx_text.len());
        rx_text.drain(..cut);
    }
}

fn cw_status(text: &str) -> DigiStatus {
    let mut st = DigiStatus::idle(DigiConfig::default());
    st.text_rx = text.into();
    st.cw = Some(CwStatus { locked: true, wpm: 20.0, snr_db: 10.0, tone_hz: 700.0 });
    st
}

#[test]
fn the_rolling_tail_survives_front_truncation() {
    let (mut a, log, mut s) = harness();
    s.rx[0].mode = Mode::Cw;
    step(&mut a, &s, 0.0);

    // Fill the buffer past its cap and anchor on it — none of this backlog may
    // ever be read aloud.
    let mut buf = "DE W1AW ".repeat(1100);
    truncate_like_the_controllers(&mut buf);
    assert_eq!(buf.len(), RX_TEXT_CAP);
    a.on_digi(&cw_status(&buf), &s, 1.0);
    a.tick(&s, None, 1.0);
    assert!(log.is_empty(), "read the backlog: {:?}", log.all());

    // New text arrives, pushing the front off.
    buf.push_str("CQ TEST ");
    truncate_like_the_controllers(&mut buf);
    a.on_digi(&cw_status(&buf), &s, 2.0);
    a.tick(&s, None, 2.0);

    let said = log.all().join(" ");
    assert!(said.contains("C Q"), "lost the new text: {said:?}");
    assert!(!said.contains("doubleyou"), "replayed the backlog: {said:?}");
}

#[test]
fn a_cleared_buffer_reanchors_rather_than_replaying() {
    let (mut a, log, mut s) = harness();
    s.rx[0].mode = Mode::Cw;
    step(&mut a, &s, 0.0);
    a.on_digi(&cw_status("HELLO THERE "), &s, 1.0);
    a.tick(&s, None, 1.0);
    log.take();

    // The decoder cleared, then filled with something unrelated.
    a.on_digi(&cw_status(""), &s, 2.0);
    a.tick(&s, None, 2.0);
    a.on_digi(&cw_status("TOTALLY DIFFERENT TEXT "), &s, 3.0);
    a.tick(&s, None, 3.0);
    assert!(log.is_empty(), "replayed after a reset: {:?}", log.all());
}

#[test]
fn unlocked_cw_is_not_read() {
    let (mut a, log, mut s) = with_cfg(|c| {
        c.text.cw = true;
        c.text.cw_only_when_locked = true;
    });
    s.rx[0].mode = Mode::Cw;
    step(&mut a, &s, 0.0);

    let mut st = cw_status("SOME NOISE HERE ");
    st.cw = Some(CwStatus { locked: false, wpm: 0.0, snr_db: 0.0, tone_hz: 700.0 });
    a.on_digi(&st, &s, 1.0);
    a.tick(&s, None, 1.0);
    st.text_rx.push_str("MORE NOISE ");
    a.on_digi(&st, &s, 2.0);
    a.tick(&s, None, 2.0);
    assert!(log.is_empty(), "read an unlocked decoder: {:?}", log.all());
}

#[test]
fn read_along_is_off_by_default() {
    let (mut a, log, mut s) = with_cfg(|_| {});
    s.rx[0].mode = Mode::Cw;
    step(&mut a, &s, 0.0);
    a.on_digi(&cw_status("CQ CQ DE W1AW "), &s, 1.0);
    a.tick(&s, None, 1.0);
    a.on_digi(&cw_status("CQ CQ DE W1AW K "), &s, 2.0);
    a.tick(&s, None, 2.0);
    assert!(log.is_empty(), "read text nobody asked for: {:?}", log.all());
}

// ── hotkeys and the master switch ───────────────────────────────────────

#[test]
fn the_status_hotkey_reads_the_whole_radio_out() {
    let (mut a, log, mut s) = harness();
    s.vfo_a_hz = 14_074_000.0;
    s.band = Band::M20;
    s.rx[0].mode = Mode::Usb;
    step(&mut a, &s, 0.0);

    a.say_status(&s, None, 1.0);
    a.tick(&s, None, 1.0);
    let said = log.all().join(" ");
    assert!(said.contains("twenty meters"), "{said}");
    assert!(said.contains("fourteen point zero seven four"), "{said}");
    assert!(said.contains("U S B"), "{said}");
}

#[test]
fn repeat_says_the_last_thing_again() {
    let (mut a, log, mut s) = harness();
    step(&mut a, &s, 0.0);
    s.split = true;
    step(&mut a, &s, 0.1);
    let first = log.take();

    a.repeat_last(1.0);
    a.tick(&s, None, 1.0);
    assert_eq!(log.all(), first);
}

#[test]
fn silence_drops_everything_waiting() {
    let (mut a, log, mut s) = harness();
    step(&mut a, &s, 0.0);
    s.split = true;
    a.on_state(&s, 0.1);
    a.silence();
    a.tick(&s, None, 0.2);
    assert!(log.is_empty(), "spoke after being silenced: {:?}", log.all());
}

#[test]
fn nothing_is_said_while_speech_is_off() {
    let (mut a, log, mut s) = with_cfg(|c| c.enabled = false);
    step(&mut a, &s, 0.0);
    s.split = true;
    s.rx[0].agc = AgcMode::Fast;
    step(&mut a, &s, 0.1);
    a.tick(&s, None, 1.0);
    assert!(log.is_empty(), "{:?}", log.all());
}

#[test]
fn a_switched_off_category_stays_quiet_but_keeps_its_latch() {
    let (mut a, log, mut s) = with_cfg(|c| c.cat.levels = false);
    step(&mut a, &s, 0.0);
    s.tx.drive = 0.9;
    step(&mut a, &s, 0.1);
    a.tick(&s, None, 1.0);
    assert!(log.is_empty(), "{:?}", log.all());

    // Switching the category on must not then announce the change that
    // happened while it was off.
    let mut cfg = a.settings().clone();
    cfg.cat.levels = true;
    a.set_settings(cfg);
    a.tick(&s, None, 2.0);
    assert!(log.is_empty(), "announced a stale change: {:?}", log.all());
}

// ── ducking ─────────────────────────────────────────────────────────────

#[test]
fn ducking_is_reported_on_the_edges_only() {
    let (mut a, _log, mut s) = harness();
    step(&mut a, &s, 0.0);
    assert_eq!(a.take_duck_change(), None);

    s.split = true;
    a.on_state(&s, 0.1);
    // Something is queued: the receiver should dip.
    assert_eq!(a.take_duck_change(), Some(0.25));
    assert_eq!(a.take_duck_change(), None, "sent the same duck twice");

    a.tick(&s, None, 0.2);
    assert_eq!(a.take_duck_change(), Some(1.0), "never restored the receiver");
}

#[test]
fn the_repaint_deadline_covers_a_pending_settle() {
    let (mut a, _log, mut s) = harness();
    step(&mut a, &s, 0.0);
    s.vfo_a_hz += 1000.0;
    a.on_state(&s, 1.0);
    // The UI idles at a quarter of a second; without this the 600 ms settle
    // would smear across most of a second.
    let deadline = a.tick(&s, None, 1.0).expect("no deadline for a pending settle");
    assert!((deadline - 1.6).abs() < 1e-9, "deadline was {deadline}");
}
