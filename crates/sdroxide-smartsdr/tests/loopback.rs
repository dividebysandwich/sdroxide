//! The client driven against the simulator over real sockets.
//!
//! Nobody here has a FLEX-6600 on the bench, so this is the only thing standing
//! between the protocol code and a user's radio. It deliberately asserts on
//! *wire* outcomes — the commands the radio received, the IQ that came back, the
//! audio packets that arrived — rather than on the client's internal state,
//! because internal state would still agree with itself if the bytes were wrong.

use std::time::Duration;

use sdroxide_smartsdr::sim::{SimConfig, SimRadio};
use sdroxide_smartsdr::{FlexHandle, FlexUpdate};
use sdroxide_types::Mode;

/// Long enough to absorb a loaded CI machine; every wait exits as soon as its
/// condition holds, so a passing run does not spend it.
const PATIENCE: Duration = Duration::from_secs(5);

fn sim() -> SimRadio {
    SimRadio::start(SimConfig::default()).expect("start simulator")
}

fn connect(sim: &SimRadio, rate: f64) -> FlexHandle {
    FlexHandle::connect(&sim.address(), rate, 1, "test").expect("connect to simulator")
}

/// Drain IQ until `want` samples have arrived or patience runs out.
fn collect_iq(h: &mut FlexHandle, want: usize) -> Vec<f32> {
    let mut got = Vec::with_capacity(want);
    let mut buf = vec![0f32; 8192];
    let deadline = std::time::Instant::now() + PATIENCE;
    while got.len() < want && std::time::Instant::now() < deadline {
        let n = h.rx_read(&mut buf);
        if n == 0 {
            std::thread::sleep(Duration::from_millis(2));
            continue;
        }
        got.extend_from_slice(&buf[..n]);
    }
    got
}

#[test]
fn connects_and_reports_the_radio_identity() {
    let sim = sim();
    let h = connect(&sim, 48_000.0);

    assert_eq!(h.ident.version, "3.3.28.0");
    assert_ne!(h.ident.handle, 0, "the radio must assign a client handle");
    assert!(h.is_alive());

    // The model arrives in the `radio` status that answers `client gui`, not in
    // the V/H handshake, so it is only present if the connect path reads the
    // status burst it read past on the way to that reply. It decides which
    // frequency ranges the caller advertises, so an empty one is not cosmetic.
    assert_eq!(h.ident.model, "FLEX-6600");
    assert_eq!(h.ident.nickname, "sdroxide-sim");
    assert_eq!(h.ident.display_name(), "sdroxide-sim (FLEX-6600)");

    // The handshake order is the radio's protocol state machine, not a
    // preference: identity, then GUI registration, then the UDP port, then
    // subscriptions. Getting it wrong is refused by real firmware.
    let cmds = sim.stats.commands();
    let pos = |prefix: &str| {
        cmds.iter()
            .position(|c| c.starts_with(prefix))
            .unwrap_or_else(|| panic!("never sent {prefix:?}; sent: {cmds:#?}"))
    };
    assert!(pos("client program") < pos("client gui"));
    assert!(pos("client gui") < pos("client udpport"));
    assert!(pos("client gui") < pos("sub slice"));
}

#[test]
fn dax_iq_flows_and_carries_the_synthetic_scene() {
    let sim = sim();
    let mut h = connect(&sim, 48_000.0);

    let iq = collect_iq(&mut h, 4096);
    assert!(iq.len() >= 4096, "only {} IQ samples arrived", iq.len());
    assert_eq!(iq.len() % 2, 0, "I/Q must stay paired");

    // The scene has a carrier on the tuned frequency, so the samples must carry
    // real power. A byte-order mistake in the DAX IQ decoder yields denormals
    // ≈ 0 here rather than an error, which is exactly the failure this catches.
    let power: f64 = iq.iter().map(|&v| (v as f64) * (v as f64)).sum::<f64>() / iq.len() as f64;
    assert!(power > 1e-4, "IQ power {power:e} is at the noise floor — byte order?");
    // …and the other end of the same rail. A FLEX puts its DAX IQ floats on the
    // wire at 2^15 full scale, so a client that forgets to divide them down is
    // 90 dB hot: everything clips, the waterfall saturates and the S-meter pins
    // (issue #184). The scene is well under full scale, so anything at or past
    // it is that mistake rather than a loud band.
    assert!(power < 1.0, "IQ power {power:e} is past full scale — DAX IQ scaling?");
    assert!(iq.iter().all(|v| v.is_finite()));
}

#[test]
fn the_radio_delivers_the_requested_iq_rate() {
    let sim = sim();
    // A FLEX creates every dax_iq stream at 48 kHz and only moves on the
    // follow-up `stream set daxiq_rate`. The rate the packets actually carry is
    // read back off their class codes, so this checks the second command landed.
    let mut h = connect(&sim, 192_000.0);
    collect_iq(&mut h, 2048);

    assert!(sim.stats.saw_command("stream set"), "never asked for a rate change");
    assert_eq!(h.actual_rate_hz(), 192_000.0, "radio is streaming at the wrong rate");
}

/// The rate is what the packets say, and only once they have said it.
///
/// A real radio streams the 48 kHz it created the stream at for a while after
/// `daxiq_rate` is sent (260 ms on a FLEX-8400). The handle used to report the
/// *requested* rate before a single packet had arrived, so a caller that waited
/// for "the rate to settle" settled at once — and the engine built a 48 kHz
/// receive chain for a 192 kHz stream: issue #687.
#[test]
fn the_rate_is_not_claimed_before_the_radio_sends_it() {
    let sim = SimRadio::start(SimConfig {
        rate_change_delay: Duration::from_millis(1500),
        ..SimConfig::default()
    })
    .expect("start simulator");
    let mut h = connect(&sim, 192_000.0);

    let early = h.actual_rate_hz();
    assert!(early == 0.0 || early == 48_000.0, "claimed {early} Hz before the radio sent it");
    collect_iq(&mut h, 256);
    assert_eq!(h.actual_rate_hz(), 48_000.0, "the stream is still at its creation rate");

    let deadline = std::time::Instant::now() + PATIENCE;
    while h.actual_rate_hz() != 192_000.0 && std::time::Instant::now() < deadline {
        collect_iq(&mut h, 256);
    }
    assert_eq!(h.actual_rate_hz(), 192_000.0, "the rate change never showed up");
}

/// [`atu_reports`], also collecting the radio's reports of keying itself.
fn atu_reports_and(
    h: &FlexHandle,
    done: impl Fn(&sdroxide_types::AtuState) -> bool,
    rig_tx: &mut Vec<bool>,
) -> Vec<sdroxide_types::AtuState> {
    let mut seen = Vec::new();
    let deadline = std::time::Instant::now() + PATIENCE;
    while std::time::Instant::now() < deadline {
        for u in h.poll_updates() {
            match u {
                FlexUpdate::Atu(Some(a)) => seen.push(a),
                FlexUpdate::RigTx(on) => rig_tx.push(on),
                _ => {}
            }
        }
        if seen.last().is_some_and(&done) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    seen
}

/// Collect tuner reports until `done` holds for one or patience runs out.
fn atu_reports(
    h: &FlexHandle,
    done: impl Fn(&sdroxide_types::AtuState) -> bool,
) -> Vec<sdroxide_types::AtuState> {
    let mut seen = Vec::new();
    let deadline = std::time::Instant::now() + PATIENCE;
    while std::time::Instant::now() < deadline {
        for u in h.poll_updates() {
            if let FlexUpdate::Atu(Some(a)) = u {
                seen.push(a);
            }
        }
        if seen.last().is_some_and(&done) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    seen
}

#[test]
fn drives_the_radios_antenna_tuner() {
    use sdroxide_types::AtuStatus;
    let sim = SimRadio::start(SimConfig { atu: true, ..SimConfig::default() }).expect("start");
    let h = connect(&sim, 48_000.0);

    assert_eq!(h.ident.atu_present, Some(true), "`info` should say there is a tuner");
    let first = atu_reports(&h, |_| true);
    assert!(!first.is_empty(), "the subscription burst should report the tuner");

    h.atu_start();
    let mut rig_tx = Vec::new();
    let tune = atu_reports_and(&h, |a| a.status == AtuStatus::Tuned, &mut rig_tx);
    assert!(sim.stats.saw_command("atu start"), "never asked the radio to tune");
    assert!(
        tune.iter().any(|a| a.status == AtuStatus::Tuning),
        "a tune in progress should be reported: {tune:?}"
    );
    // Still in circuit after the copy of the report that leaves `atu_enabled`
    // out, which the simulator sends last, as the radio does.
    std::thread::sleep(Duration::from_millis(100));
    let after = atu_reports_and(&h, |_| true, &mut rig_tx);
    let last = after.last().or(tune.last()).copied();
    assert_eq!(last.map(|a| (a.status, a.enabled)), Some((AtuStatus::Tuned, true)));
    // The radio keyed itself for the tune and said so: on the air, then off it.
    assert_eq!(rig_tx, vec![true, false], "the tune's own over should be reported");

    h.atu_bypass();
    let bypass = atu_reports(&h, |a| a.status == AtuStatus::Bypass);
    assert!(sim.stats.saw_command("atu bypass"));
    assert_eq!(bypass.last().map(|a| (a.status, a.enabled)), Some((AtuStatus::Bypass, false)));
}

/// An over of sdroxide's own goes out on the same interlock, and must not come
/// back as the radio keying itself: the engine would mute for it and refuse
/// the next key-down as somebody else's.
#[test]
fn our_own_over_is_not_the_radio_keying_itself() {
    let sim = sim();
    let h = connect(&sim, 48_000.0);
    h.tx_begin(14_074_000.0);
    std::thread::sleep(Duration::from_millis(200));
    h.tx_end();
    std::thread::sleep(Duration::from_millis(300));
    let rig_tx: Vec<bool> = h
        .poll_updates()
        .into_iter()
        .filter_map(|u| if let FlexUpdate::RigTx(on) = u { Some(on) } else { None })
        .collect();
    assert!(sim.stats.saw_command("xmit 1") && sim.stats.saw_command("xmit 0"));
    assert!(!rig_tx.contains(&true), "our own over read back as the radio's: {rig_tx:?}");
}

#[test]
fn offers_no_tuner_on_a_radio_without_one() {
    let sim = sim();
    let h = connect(&sim, 48_000.0);

    assert_eq!(h.ident.atu_present, Some(false));
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        !h.poll_updates().into_iter().any(|u| matches!(u, FlexUpdate::Atu(Some(_)))),
        "a radio without a tuner must not grow one"
    );
}

#[test]
fn tuning_moves_the_radios_slice() {
    let sim = sim();
    let h = connect(&sim, 48_000.0);

    h.set_center(7_074_000.0);
    // The radio's own slice has to follow our centre, or a second client (and
    // the radio's front panel) shows a frequency we are not listening on.
    assert!(
        sim.wait_for(PATIENCE, |s| (s.slice_freq_hz() - 7_074_000.0).abs() < 1.0),
        "slice sat at {:.0} Hz, expected 7074000",
        sim.stats.slice_freq_hz()
    );

    // An IF offset puts the slice inside the span without moving the centre —
    // the panadapter must not be dragged along, or the offset never converges.
    h.set_if_offset(2_500.0);
    assert!(
        sim.wait_for(PATIENCE, |s| (s.slice_freq_hz() - 7_076_500.0).abs() < 1.0),
        "slice sat at {:.0} Hz, expected 7076500",
        sim.stats.slice_freq_hz()
    );
    assert!(
        sim.stats.commands().iter().any(|c| c.contains("autopan=0")),
        "slice tune must suppress autopan"
    );
}

#[test]
fn a_tuned_carrier_lands_where_it_should() {
    let sim = sim();
    let mut h = connect(&sim, 48_000.0);

    // The simulator anchors its carriers in RF, so retuning genuinely moves them
    // through the passband. Park the centre 10 kHz below a carrier and the
    // baseband tone must come out at +10 kHz — which a client that confuses the
    // centre with the dial gets wrong.
    h.set_center(14_090_000.0); // carrier at 14.100 MHz → +10 kHz
    std::thread::sleep(Duration::from_millis(250)); // let the retune take effect
    let mut buf = vec![0f32; 65536];
    let _ = h.rx_read(&mut buf); // discard samples from before the retune
    let iq = collect_iq(&mut h, 16384);
    assert!(iq.len() >= 16384);

    let rate = 48_000.0;
    let n = iq.len() / 2;
    // Correlate against the expected tone; a matched frequency integrates
    // coherently, a wrong one averages away.
    let power_at = |hz: f64| -> f64 {
        use std::f64::consts::TAU;
        let (mut re, mut im) = (0.0, 0.0);
        for k in 0..n {
            let ph = TAU * hz * k as f64 / rate;
            let (i, q) = (iq[2 * k] as f64, iq[2 * k + 1] as f64);
            re += i * ph.cos() + q * ph.sin();
            im += q * ph.cos() - i * ph.sin();
        }
        (re * re + im * im) / (n * n) as f64
    };

    let want = power_at(10_000.0);
    let decoy = power_at(-10_000.0);
    assert!(
        want > decoy * 10.0,
        "carrier not at +10 kHz: power there {want:e} vs {decoy:e} at −10 kHz \
         (a sign error in the tuning or a conjugated spectrum)"
    );
}

#[test]
fn transmit_audio_reaches_the_radio() {
    let sim = sim();
    let mut h = connect(&sim, 48_000.0);
    collect_iq(&mut h, 512); // let the session settle

    let rate = h.tx_begin(14_074_000.0);
    assert_eq!(rate, 24_000.0, "DAX TX audio is 24 kHz");

    // A second of a 1 kHz tone at half scale.
    let tone: Vec<f32> = (0..24_000)
        .map(|k| (std::f64::consts::TAU * 1000.0 * k as f64 / 24_000.0).sin() as f32 * 0.5)
        .collect();
    h.tx_write(&tone);

    assert!(
        sim.wait_for(PATIENCE, |s| s.tx_frames.load(std::sync::atomic::Ordering::Relaxed) > 4096),
        "the radio received only {} TX frames",
        sim.stats.tx_frames.load(std::sync::atomic::Ordering::Relaxed)
    );
    // Amplitude has to survive the trip: a byte-order mistake on the TX side
    // would deliver frames full of denormals, and the radio would transmit
    // silence at full power.
    let peak = sim.stats.tx_peak();
    assert!((0.4..=0.6).contains(&peak), "TX peak {peak} — expected ≈0.5");

    assert!(sim.stats.keyed.load(std::sync::atomic::Ordering::Relaxed), "radio is not keyed");
    // The modulator has to be switched to DAX, or the radio transmits mic audio.
    assert!(sim.stats.saw_command("transmit set dax=1"));

    h.tx_end();
    assert!(sim.wait_for(PATIENCE, |s| !s.keyed.load(std::sync::atomic::Ordering::Relaxed)));
    // And handed back, or the operator's next local transmission is silent.
    // Strictly after the unkey: releasing the modulator while still keyed would
    // put a moment of shack ambience on the air.
    assert!(sim.wait_for(PATIENCE, |s| s.saw_command("transmit set dax=0")));
    let cmds = sim.stats.commands();
    let pos = |c: &str| cmds.iter().rposition(|x| x == c).expect("command not sent");
    assert!(pos("xmit 0") < pos("transmit set dax=0"), "modulator released before unkeying");
}

#[test]
fn transmit_audio_is_paced_by_the_clock_not_by_receive_packets() {
    use std::sync::atomic::Ordering;

    let sim = sim();
    // 24 kHz IQ is the worst case for the coupling this guards against: the
    // radio sends only ~190 IQ packets a second, and a transmit pump that
    // emitted one audio packet per received one could not reach the 187
    // packets/s that 24 kHz DAX audio needs. Under-feeding the radio's buffer
    // does not sound like silence, it sounds like a chopped transmission.
    let mut h = connect(&sim, 24_000.0);
    collect_iq(&mut h, 512);

    h.tx_begin(14_074_000.0);
    // Two seconds of audio, handed over all at once the way the engine hands
    // over an FT8 burst.
    let tone: Vec<f32> = (0..48_000)
        .map(|k| (std::f64::consts::TAU * 700.0 * k as f64 / 24_000.0).sin() as f32 * 0.5)
        .collect();
    let started = std::time::Instant::now();
    h.tx_write(&tone);

    // After one second of wall clock, roughly one second of audio must have
    // reached the radio — not a fraction of it, and not all two seconds.
    assert!(
        sim.wait_for(Duration::from_millis(1400), |s| s.tx_frames.load(Ordering::Relaxed)
            >= 20_000)
    );
    let elapsed = started.elapsed().as_secs_f64();
    let frames = sim.stats.tx_frames.load(Ordering::Relaxed);
    let rate = frames as f64 / elapsed;
    assert!(
        (18_000.0..32_000.0).contains(&rate),
        "TX ran at {rate:.0} frames/s, expected ≈24000 ({frames} frames in {elapsed:.2} s)"
    );

    h.tx_end();
}

#[test]
fn swr_telemetry_arrives_while_keyed() {
    let sim = sim();
    let mut h = connect(&sim, 48_000.0);
    collect_iq(&mut h, 512);

    h.set_drive(0.5);
    h.tx_begin(14_074_000.0);
    let tone: Vec<f32> = (0..48_000).map(|k| ((k % 24) as f32 / 24.0) - 0.5).collect();
    h.tx_write(&tone);

    // Meter ids are per-session, so this only works if the client learned them
    // from the `meter` definition status — the `#`-separated one.
    // The definitions carry descriptions with spaces in them, as a real
    // radio's do — which is what once left every meter nameless.
    let deadline = std::time::Instant::now() + PATIENCE;
    let (mut swr, mut fwd_w) = (None, None);
    while std::time::Instant::now() < deadline && (swr.is_none() || fwd_w.is_none()) {
        if let Some(t) = h.poll_telemetry() {
            swr = swr.or(t.swr);
            fwd_w = fwd_w.or(t.fwd_w);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let swr = swr.expect("no SWR reading arrived while transmitting");
    assert!((swr - 1.3).abs() < 0.05, "SWR {swr}, expected ≈1.3");
    // Half drive: 47 dBm on the wire, which is 50 W and not 47.
    let fwd_w = fwd_w.expect("no forward power arrived while transmitting");
    assert!((fwd_w - 50.0).abs() < 2.0, "forward power {fwd_w} W, expected ≈50");

    h.tx_end();
}

/// The SWR belongs to the over, not to the audio queue. The engine hands TX
/// audio over in blocks and the queue runs dry between them, so a reading that
/// lands in one of those gaps is still a reading from the over — and with the
/// meter following the queue, a FLEX-8400 reported 1.5:1 at 13 W for a whole
/// over and not one reading reached the screen. Keyed with nothing queued at
/// all, the over is still on the air.
#[test]
fn swr_follows_the_over_not_the_audio_queue() {
    let sim = sim();
    let mut h = connect(&sim, 48_000.0);
    collect_iq(&mut h, 512);

    h.set_drive(0.5);
    h.tx_begin(14_074_000.0);
    let deadline = std::time::Instant::now() + PATIENCE;
    let mut swr = None;
    while std::time::Instant::now() < deadline && swr.is_none() {
        if let Some(t) = h.poll_telemetry() {
            swr = swr.or(t.swr);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let swr = swr.expect("no SWR reading arrived while keyed with an empty audio queue");
    assert!((swr - 1.3).abs() < 0.05, "SWR {swr}, expected ≈1.3");

    // Unkeyed, the meter is cleared rather than left frozen on its last reading.
    h.tx_end();
    let deadline = std::time::Instant::now() + PATIENCE;
    let mut cleared = false;
    while std::time::Instant::now() < deadline && !cleared {
        if let Some(t) = h.poll_telemetry() {
            cleared = t.swr.is_none() && t.fwd_w.is_none();
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(cleared, "the meter should be cleared at the end of the over");
}

#[test]
fn a_mode_change_reaches_the_radio_and_comes_back() {
    let sim = sim();
    let h = connect(&sim, 48_000.0);

    h.set_mode(Mode::Cw);
    assert!(sim.wait_for(PATIENCE, |s| s.saw_command("slice set 0 mode=CW")));

    // Every sdroxide digital mode asks the radio only for a sideband — the
    // decoding happens here, from the IQ.
    h.set_mode(Mode::Ft8);
    assert!(sim.wait_for(PATIENCE, |s| s.saw_command("slice set 0 mode=DIGU")));
}

#[test]
fn connecting_adopts_the_radios_current_frequency() {
    let sim = sim();
    let h = connect(&sim, 48_000.0);

    // Opening sdroxide must not yank a running station off frequency to
    // whatever was last in its own config; the radio is authoritative about
    // where it is tuned.
    let deadline = std::time::Instant::now() + PATIENCE;
    let mut adopted = None;
    while std::time::Instant::now() < deadline && adopted.is_none() {
        for u in h.poll_updates() {
            if let FlexUpdate::Freq(hz) = u {
                adopted = Some(hz);
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(adopted, Some(14_074_000.0), "did not adopt the radio's dial on connect");
}

#[test]
fn operator_changes_on_the_radio_are_reported_back() {
    let sim = sim();
    let h = connect(&sim, 48_000.0);
    // Let the client adopt slice 0 and settle, so the echo-suppression window
    // from its own setup commands has passed and the connect-time adoption
    // (see above) is out of the way.
    std::thread::sleep(Duration::from_millis(400));
    let _ = h.poll_updates();

    // Stand in for somebody turning the radio's own dial.
    h.send_raw("slice tune 0 21.074000");

    let deadline = std::time::Instant::now() + PATIENCE;
    let mut seen = None;
    while std::time::Instant::now() < deadline && seen.is_none() {
        for u in h.poll_updates() {
            if let FlexUpdate::Freq(hz) = u {
                seen = Some(hz);
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(seen, Some(21_074_000.0), "the client did not follow the radio's dial");
}

#[test]
fn a_refused_gui_registration_explains_itself() {
    let sim = SimRadio::start(SimConfig { refuse_gui: true, ..SimConfig::default() })
        .expect("start simulator");
    let msg = match FlexHandle::connect(&sim.address(), 48_000.0, 1, "test") {
        Ok(_) => panic!("registration should have been refused"),
        Err(e) => e.to_string(),
    };
    // The bare hex code is the most common thing in a Flex bug report and the
    // least useful one; the message has to say what to do about it.
    assert!(msg.contains("multiFLEX"), "unhelpful message: {msg}");
    assert!(msg.contains("F3000001"), "message drops the code: {msg}");
}

#[test]
fn the_session_trace_records_both_directions() {
    let sim = sim();
    let mut h = connect(&sim, 48_000.0);
    collect_iq(&mut h, 2048);

    let dump = h.trace.dump();
    // This is what a user is asked to send in, so its usefulness is a tested
    // property, not a hope.
    assert!(dump.contains("→ C"), "no outbound commands in the trace");
    assert!(dump.contains("← V3.3.28.0"), "no inbound lines in the trace");
    assert!(dump.contains("client gui"), "the handshake is missing");
    assert!(dump.contains("first packet stream="), "no VITA-49 stream record");
    assert!(dump.contains("--- streams ---"));
}

#[test]
fn dropping_the_handle_releases_the_radios_streams() {
    let sim = sim();
    {
        let mut h = connect(&sim, 48_000.0);
        collect_iq(&mut h, 512);
    } // dropped here

    // A radio whose streams are never removed keeps transmitting them to a port
    // nobody is listening on, and keeps the DAX channel out of the next
    // session's reach.
    assert!(sim.wait_for(PATIENCE, |s| s.saw_command("stream remove")));
    assert!(sim.stats.saw_command("display panafall remove"));
}

// ─── Identity: who gets evicted, and who does not ────────────────────────────

#[test]
fn a_client_id_somebody_else_holds_is_not_stolen() {
    // The default station name is "sdroxide" on every installation, so the id
    // derived from it is the same everywhere. Two sdroxide sessions therefore
    // arrive at a radio holding the same identity — and a radio resolves that by
    // evicting the incumbent, who reconnects and evicts the newcomer, forever.
    let held = sdroxide_smartsdr::gui_client_id("test");
    let sim = SimRadio::start(SimConfig {
        existing_clients: vec![(held.clone(), "OtherShack".into())],
        ..SimConfig::default()
    })
    .expect("start simulator");

    let h = connect(&sim, 48_000.0);
    let dump = h.trace.dump();

    // Registering under the id we would have derived is what evicts them.
    assert!(
        !dump.contains(&format!("client gui {held}")),
        "registered under an id another client already holds:\n{dump}"
    );
    assert!(dump.contains("client gui "), "no registration at all:\n{dump}");
    assert!(
        dump.contains("already registered by handle") && dump.contains("OtherShack"),
        "the trace does not say whose id it was:\n{dump}"
    );
    // And the incumbent is still there: the radio never had cause to throw them
    // off, so no duplicate-id notice was ever sent.
    assert!(
        !dump.contains("duplicate_client_id=1"),
        "another client was evicted after all:\n{dump}"
    );
}

#[test]
fn a_free_client_id_is_still_the_stable_one() {
    // The avoidance above must not cost the session restore in the ordinary
    // case: with nobody holding it, the derived id is what goes on the wire, and
    // it is the same string on the next run.
    let sim = sim();
    let h = connect(&sim, 48_000.0);
    let want = sdroxide_smartsdr::gui_client_id("test");
    assert!(
        h.trace.dump().contains(&format!("client gui {want}")),
        "the stable id was abandoned for no reason:\n{}",
        h.trace.dump()
    );
}

#[test]
fn an_eviction_for_a_duplicate_id_is_reported_as_one() {
    // A client that sees only the TCP close learns nothing, reconnects with the
    // same id, and starts the fight again. The radio does say why, once.
    let sim = SimRadio::start(SimConfig {
        evict_after: Some(Duration::from_millis(300)),
        ..SimConfig::default()
    })
    .expect("start simulator");
    let h = connect(&sim, 48_000.0);

    let deadline = std::time::Instant::now() + PATIENCE;
    while h.is_alive() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!h.is_alive(), "the session outlived its own eviction");
    assert!(
        h.evicted_for_duplicate_id(),
        "the eviction was not recognised, so a retry would repeat it:\n{}",
        h.trace.dump()
    );
}

#[test]
fn a_transient_identity_registers_under_an_id_of_its_own() {
    let sim = sim();
    let h = FlexHandle::connect_with(&sdroxide_smartsdr::ConnectOptions {
        address: sim.address(),
        iq_rate_hz: 48_000.0,
        station: "test".into(),
        transient_identity: true,
        ..sdroxide_smartsdr::ConnectOptions::default()
    })
    .expect("connect to simulator");

    let derived = sdroxide_smartsdr::gui_client_id("test");
    assert!(!h.trace.dump().contains(&format!("client gui {derived}")));
    assert!(h.is_alive());
}

#[test]
fn a_configured_client_id_is_used_verbatim() {
    // The way back to the radio's session restore for an operator whose station
    // name collides with somebody else's.
    let sim = sim();
    let mine = "11112222-3333-4444-8555-666677778888";
    let h = FlexHandle::connect_with(&sdroxide_smartsdr::ConnectOptions {
        address: sim.address(),
        iq_rate_hz: 48_000.0,
        station: "test".into(),
        client_id: mine.into(),
        ..sdroxide_smartsdr::ConnectOptions::default()
    })
    .expect("connect to simulator");
    assert!(h.trace.dump().contains(&format!("client gui {mine}")));
}

// ─── The data path ───────────────────────────────────────────────────────────

#[test]
fn iq_still_arrives_when_the_radio_refuses_client_udpport() {
    // The v1.4.0.0 protocol a FLEX-6600 speaks can answer `client udpport` with
    // "command not supported", leaving the registration datagram as the only
    // thing that tells the radio where to stream. A client that sends it once,
    // early, and never again has no second chance if that datagram is lost or
    // arrives before the stream exists.
    let sim = SimRadio::start(SimConfig { require_udp_register: true, ..SimConfig::default() })
        .expect("start simulator");
    let mut h = connect(&sim, 48_000.0);

    let iq = collect_iq(&mut h, 4096);
    assert!(
        iq.len() >= 4096,
        "no IQ arrived on a radio that ignores `client udpport` — got {} samples",
        iq.len()
    );
}

#[test]
fn a_dax_iq_stream_left_unbound_is_bound_again() {
    // An unbound DAX IQ stream is created, accepts a rate, reports itself
    // active, and carries nothing at all. It is the silent half of "connects
    // fine, no spectrum", and the radio does say so: `pan=0x0`.
    let sim = SimRadio::start(SimConfig { unbound_dax_iq: true, ..SimConfig::default() })
        .expect("start simulator");
    let mut h = connect(&sim, 48_000.0);

    let iq = collect_iq(&mut h, 2048);
    assert!(
        iq.len() >= 2048,
        "the stream was never bound to a panadapter — got {} samples:\n{}",
        iq.len(),
        h.trace.dump()
    );
    assert!(
        h.trace.dump().contains("not bound to a panadapter"),
        "the rebind happened without saying why:\n{}",
        h.trace.dump()
    );
}

#[test]
fn transmit_audio_goes_to_the_radios_data_port() {
    // 4992 carries the TCP protocol and the discovery broadcasts; the VITA-49
    // data plane is 4991. Audio addressed to the control port is dropped without
    // a word — the radio keys up and the operator transmits dead air.
    assert_eq!(sdroxide_smartsdr::VITA_PORT, 4991);
    assert_ne!(sdroxide_smartsdr::VITA_PORT, sdroxide_smartsdr::RADIO_PORT);
}

#[test]
fn iq_arrives_when_the_radio_streams_to_flexlibs_default_port() {
    // A radio that never recorded our port streams to 4991, FlexLib's default.
    // From inside the client that is indistinguishable from a firewall — the
    // control link is perfect and no packet ever arrives — so the client listens
    // there too, but only once the radio has refused the port it asked for.
    let sim = SimRadio::start(SimConfig {
        require_udp_register: true,
        stream_to_default_port: true,
        ..SimConfig::default()
    })
    .expect("start simulator");
    let mut h = connect(&sim, 48_000.0);

    if h.trace.dump().contains("is not available") {
        // Something on this machine already holds 4991 — SmartSDR, its DAX
        // service, or another run of this test. Refusing to bind it is the
        // correct behaviour, so there is nothing left here to assert.
        return;
    }

    let iq = collect_iq(&mut h, 2048);
    assert!(
        iq.len() >= 2048,
        "nothing was received on the fallback port — got {} samples:\n{}",
        iq.len(),
        h.trace.dump()
    );
    assert!(
        h.trace.dump().contains("`client udpport` did not take"),
        "the trace does not say the radio ignored our port:\n{}",
        h.trace.dump()
    );
}

#[test]
fn the_diagnostic_report_distinguishes_the_two_silences() {
    // "No spectrum" has two causes with different fixes: nothing reached this
    // machine (a network fault), or something did and we could not read it
    // (ours). A report that cannot tell them apart sends everybody to the wrong
    // one, so the raw datagram count is part of it.
    let sim = sim();
    let mut h = connect(&sim, 48_000.0);
    collect_iq(&mut h, 2048);
    let dump = h.trace.dump();
    assert!(dump.contains("datagrams received on the VITA-49 socket(s):"), "{dump}");
    assert!(!dump.contains("no UDP at all"), "packets arrived, so this must not be claimed");
}

// ─── Panadapters: the radio only has so many ─────────────────────────────────

/// A radio has two panadapters on a FLEX-6400 and four higher up, and it
/// restores a GUI client's own when that client comes back under an id it has
/// used before. A client that creates another every session therefore leaves
/// one behind every session, and after two of them the radio says no — which is
/// issue #309, `0x50000009` with an empty body.
#[test]
fn a_panadapter_the_radio_kept_for_us_is_taken_up_again() {
    let sim = SimRadio::start(SimConfig { restored_pan: true, ..SimConfig::default() })
        .expect("start simulator");
    let mut h = connect(&sim, 48_000.0);

    // Not one more asked for...
    assert!(
        !sim.stats.saw_command("display panafall create"),
        "a panadapter was already waiting for us and another was created anyway:\n{}",
        h.trace.dump()
    );
    // ...and the one that was waiting is carrying the receiver.
    assert!(!collect_iq(&mut h, 2048).is_empty(), "no IQ from the adopted panadapter");

    // Ours to take up is also ours to put back, which is what stops the next
    // session finding two.
    drop(h);
    assert!(sim.wait_for(PATIENCE, |s| s.saw_command("display panafall remove")));
}

/// And when the radio has none left at all, the receiver still comes up: it
/// borrows one rather than refusing to work, and says whose it is.
#[test]
fn a_radio_with_no_free_panadapter_still_receives() {
    let sim =
        SimRadio::start(SimConfig { no_free_pan: true, ..SimConfig::default() }).expect("start");
    let mut h = connect(&sim, 48_000.0);

    assert!(!collect_iq(&mut h, 2048).is_empty(), "no IQ after borrowing a panadapter");
    let trace = h.trace.dump();
    assert!(
        trace.contains("Borrowing"),
        "the operator was not told whose panadapter this is:\n{trace}"
    );

    // A borrowed one is not ours to remove: doing so would close somebody
    // else's display.
    drop(h);
    // Give the teardown the same window every other test gives it, then check
    // that the removal never came.
    std::thread::sleep(Duration::from_millis(400));
    assert!(
        !sim.stats.saw_command("display panafall remove"),
        "another client's panadapter was removed on the way out"
    );
}
