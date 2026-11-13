//! RDS/RBDS through the whole WFM chain: a real broadcast multiplex, genuinely
//! frequency-modulated, demodulated by `make_demod(Mode::Wfm, …)`.
//!
//! Nothing here feeds the decoder a convenient baseband signal. The transmitter
//! below builds the composite the way a station does — programme audio, 19 kHz
//! pilot, 38 kHz difference channel, and a band-limited biphase data signal on a
//! suppressed 57 kHz carrier — and the receiver under test is the same one the
//! radio runs. The reason is recorded in `tests/ctcss.rs` and it applies twice
//! over here: a decoder handed a bare data waveform passes tests it has no
//! business passing.
//!
//! The block encoder is written out again rather than shared with the decoder.
//! A loopback against the decoder's own tables would agree with itself whatever
//! either of them said about the standard.

use num_complex::Complex;
use sdroxide_dsp::make_demod;
use sdroxide_types::{Mode, RdsData, RdsStandard, pty_name};

type C32 = Complex<f32>;

/// Deterministic noise, the same splitmix64 the other DSP tests use.
struct Rng(u64);

impl Rng {
    fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 40) as f32 / (1 << 23) as f32 - 1.0
    }
}

// ---------------------------------------------------------------------------
// The transmitter
// ---------------------------------------------------------------------------

/// g(x) = x¹⁰ + x⁸ + x⁷ + x⁵ + x⁴ + x³ + 1.
const TX_POLY: u32 = 0b101_1011_1001;
const TX_OFF_A: u16 = 0b00_1111_1100;
const TX_OFF_B: u16 = 0b01_1001_1000;
const TX_OFF_C: u16 = 0b01_0110_1000;
const TX_OFF_CP: u16 = 0b11_0101_0000;
const TX_OFF_D: u16 = 0b01_1011_0100;

/// Check word for sixteen information bits: the remainder of `info · x¹⁰`.
fn tx_check(info: u16) -> u16 {
    let mut reg = 0u32;
    let word = (info as u32) << 10;
    for i in (0..26).rev() {
        reg = (reg << 1) | ((word >> i) & 1);
        if reg & (1 << 10) != 0 {
            reg ^= TX_POLY;
        }
    }
    (reg & 0x3ff) as u16
}

/// Groups to a bit stream, most significant bit of each block first.
fn tx_bits(groups: &[[u16; 4]]) -> Vec<u8> {
    let mut bits = Vec::with_capacity(groups.len() * 104);
    for g in groups {
        // A version-B group puts the programme identification back in the third
        // block, and flags that by using C′ in place of C.
        let third = if g[1] & 0x0800 != 0 { TX_OFF_CP } else { TX_OFF_C };
        for (i, &info) in g.iter().enumerate() {
            let off = [TX_OFF_A, TX_OFF_B, third, TX_OFF_D][i];
            let word = ((info as u32) << 10) | (tx_check(info) ^ off) as u32;
            for k in (0..26).rev() {
                bits.push(((word >> k) & 1) as u8);
            }
        }
    }
    bits
}

/// Shape of the transmitted data symbol.
#[derive(Clone, Copy, PartialEq)]
enum Pulse {
    /// One period of a sine across the bit — smooth, confined close to the data
    /// band, and a fair stand-in for the cos²-shaped pulse the standard
    /// specifies. What a compliant encoder puts on the air.
    Shaped,
    /// A hard ±1 square. No encoder should transmit this, but it is what a
    /// receiver sees from an unshaped one, and it costs the matched filter a
    /// mismatch the shaped pulse does not — so it is worth proving the decoder
    /// survives it.
    Square,
}

/// Differential encoding, then biphase shaping, at `rate`.
///
/// Both pulse shapes are generated per sample from the symbol phase, with no
/// filtering. That matters more than it looks: a fixed-tap shaping filter has a
/// transition width proportional to the sample rate, so the same "2.4 kHz
/// low-pass" is a different filter at 192 kHz and at 256 kHz, and a rate sweep
/// then measures the generator instead of the decoder.
fn tx_baseband(bits: &[u8], rate: f64, pulse: Pulse) -> Vec<f32> {
    const BITRATE: f64 = 1_187.5;
    let mut state = 0u8;
    let symbols: Vec<f32> = bits
        .iter()
        .map(|&b| {
            state ^= b;
            if state == 1 { 1.0 } else { -1.0 }
        })
        .collect();

    // Biphase: positive over the first half of each bit period, negative over
    // the second, which is what gives the line code its null at DC.
    let n = (symbols.len() as f64 * rate / BITRATE) as usize;
    (0..n)
        .map(|i| {
            let pos = i as f64 * BITRATE / rate;
            let sym = symbols[(pos as usize).min(symbols.len() - 1)];
            match pulse {
                Pulse::Shaped => sym * (std::f64::consts::TAU * pos.fract()).sin() as f32,
                Pulse::Square => {
                    if pos.fract() < 0.5 {
                        sym
                    } else {
                        -sym
                    }
                }
            }
        })
        .collect()
}

/// How the data subcarrier sits against the pilot's third harmonic.
///
/// The standard permits either, and stations use both, so a decoder that only
/// works for one of them is broken in a way no single test signal would show.
#[derive(Clone, Copy, PartialEq)]
enum SubPhase {
    /// `sin ω₅₇t` — in phase with the pilot's third harmonic.
    InPhase,
    /// `cos ω₅₇t` — a quarter turn from it.
    Quadrature,
}

struct Station {
    /// Peak deviation contributed by the data subcarrier, as a fraction of the
    /// station's ±75 kHz. Real stations run 2–4 %.
    rds_injection: f64,
    phase: SubPhase,
    pulse: Pulse,
    pilot: bool,
    /// Added to the IQ, not to the multiplex: this is receiver noise.
    noise: f32,
}

impl Default for Station {
    fn default() -> Self {
        Station {
            rds_injection: 0.04,
            phase: SubPhase::InPhase,
            pulse: Pulse::Shaped,
            pilot: true,
            noise: 0.0,
        }
    }
}

/// Peak amplitude of each programme channel.
///
/// Chosen so the whole multiplex stays inside ±75 kHz with the pilot and the
/// subcarrier on top of it: `0.85·(|M| + |S|) + 0.1 + inj ≤ 1`. A station that
/// over-deviates does not fit its own channel, the receiver's front end truncates
/// the FM spectrum, and the resulting distortion lands right across the composite
/// — including on 57 kHz, where it swamps a normally injected subcarrier. That is
/// a broken test signal rather than a finding about the decoder, and it cost an
/// afternoon to tell apart from one.
const AUDIO_AMP: f64 = 0.45;

/// FM-modulate a full broadcast multiplex carrying programme audio and RDS.
///
/// ```text
/// mpx = 0.85·[M + S·sin ω₃₈t] + 0.1·sin ω₁₉t + inj·d(t)·{sin|cos} ω₅₇t
/// ```
///
/// Sine phase for the pilot and difference channel, as in `chain.rs` and for the
/// same reason it says there.
fn broadcast(rate: f64, secs: f64, groups: &[[u16; 4]], st: &Station) -> Vec<C32> {
    let n = (rate * secs) as usize;
    // Repeat the group sequence for as long as the signal lasts.
    let per_group = 104.0 / 1_187.5;
    let cycles = (secs / (per_group * groups.len() as f64)).ceil() as usize + 1;
    let repeated: Vec<[u16; 4]> =
        groups.iter().copied().cycle().take(groups.len() * cycles).collect();
    let data = tx_baseband(&tx_bits(&repeated), rate, st.pulse);
    assert!(data.len() >= n, "generated {} data samples for {n} needed", data.len());

    let mut rng = Rng(0x5DE0_1DE0_1234_5678);
    let mut phase = 0.0f64;
    (0..n)
        .map(|i| {
            let t = i as f64 / rate;
            // Programme material: a pair of tones, panned, so the audio path is
            // carrying something real while the data is decoded.
            let l = AUDIO_AMP * (std::f64::consts::TAU * 900.0 * t).sin();
            let r = AUDIO_AMP * (std::f64::consts::TAU * 1_500.0 * t).sin();
            let (m, s) = ((l + r) / 2.0, (l - r) / 2.0);

            let sub = std::f64::consts::TAU * 57_000.0 * t;
            let carrier = match st.phase {
                SubPhase::InPhase => sub.sin(),
                SubPhase::Quadrature => sub.cos(),
            };
            let rds = st.rds_injection * data[i] as f64 * carrier;

            let mpx = if st.pilot {
                0.85 * (m + s * (std::f64::consts::TAU * 38_000.0 * t).sin())
                    + 0.1 * (std::f64::consts::TAU * 19_000.0 * t).sin()
                    + rds
            } else {
                // A mono station still carrying data — the case that proves the
                // carrier recovery is not leaning on the pilot.
                0.85 * m + rds
            };

            phase += std::f64::consts::TAU * 75_000.0 * mpx / rate;
            let (re, im) = (phase.cos() as f32, phase.sin() as f32);
            C32::new(re + st.noise * rng.next_f32(), im + st.noise * rng.next_f32())
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Group builders
// ---------------------------------------------------------------------------

const PI: u16 = 0xD3C2;
const PTY: u16 = 10; // "Pop Music" under RDS, "Country" under RBDS.
const TP: u16 = 1;

/// The fixed part of block B: group type, version, traffic programme, type.
fn block_b(gtype: u16, version_b: bool, rest: u16) -> u16 {
    (gtype << 12) | (u16::from(version_b) << 11) | (TP << 10) | (PTY << 5) | rest
}

const PS: &[u8; 8] = b"SDROXIDE";
const RT: &str = "THE PILOTS - THIRD HARMONIC";
/// Where the artist and title sit inside [`RT`], for the RadioText+ tags.
const RT_ARTIST: (usize, usize) = (0, 10);
const RT_TITLE: (usize, usize) = (13, 14);

/// Group 0A: two characters of the station name, plus two alternative
/// frequencies. AF codes 1 and 204 are the ends of the broadcast band.
fn group_0a(seg: u16) -> [u16; 4] {
    let ta = 0u16;
    let ms = 1u16; // music
    let b = block_b(0, false, (ta << 4) | (ms << 3) | seg);
    let c = (1u16 << 8) | 204;
    let i = seg as usize * 2;
    [PI, b, c, ((PS[i] as u16) << 8) | PS[i + 1] as u16]
}

/// Group 2A: four characters of RadioText. A carriage return marks the end, so
/// the text shows without waiting for all sixteen segments.
fn group_2a(seg: u16) -> [u16; 4] {
    let text: Vec<u8> = RT.bytes().chain(std::iter::once(0x0d)).collect();
    let at = |i: usize| *text.get(i).unwrap_or(&b' ') as u16;
    let base = seg as usize * 4;
    let b = block_b(2, false, seg); // A/B flag left at 0
    [PI, b, (at(base) << 8) | at(base + 1), (at(base + 2) << 8) | at(base + 3)]
}

/// Group 1A: the extended country code, variant 0. `0xE0` is the European block,
/// which is what makes this station read as RDS rather than RBDS.
fn group_1a(ecc: u16) -> [u16; 4] {
    [PI, block_b(1, false, 0), ecc, 0]
}

/// Group 3A: RadioText+ lives in group 11A on this station.
fn group_3a() -> [u16; 4] {
    let app_group = 11u16 << 1; // 11A
    [PI, block_b(3, false, app_group), 0, 0x4BD7]
}

/// Group 11A: the RadioText+ tags themselves — artist as content type 4, title
/// as content type 1, each a start and a length into the RadioText.
fn group_11a() -> [u16; 4] {
    let (ct1, start1, len1) = (4u16, RT_ARTIST.0 as u16, RT_ARTIST.1 as u16 - 1);
    let (ct2, start2, len2) = (1u16, RT_TITLE.0 as u16, RT_TITLE.1 as u16 - 1);
    let running = 1u16;
    let b = block_b(11, false, (running << 3) | (ct1 >> 3));
    let c = ((ct1 & 0x7) << 13) | (start1 << 7) | (len1 << 1) | (ct2 >> 5);
    let d = ((ct2 & 0x1f) << 11) | (start2 << 5) | len2;
    [PI, b, c, d]
}

/// Group 4A: 2026-08-17 14:32 UTC, two hours ahead locally.
const CLOCK_MJD: u32 = 61_269;
fn group_4a() -> [u16; 4] {
    let (hour, minute, offset) = (14u32, 32u16, 4u16);
    let b = block_b(4, false, ((CLOCK_MJD >> 15) & 0x3) as u16);
    let c = (((CLOCK_MJD & 0x7fff) << 1) as u16) | ((hour >> 4) & 1) as u16;
    let d = (((hour & 0xf) as u16) << 12) | (minute << 6) | offset;
    [PI, b, c, d]
}

const PTYN: &[u8; 8] = b"CHARTS  ";
fn group_10a(seg: u16) -> [u16; 4] {
    let b = block_b(10, false, seg);
    let i = seg as usize * 4;
    let w = |a: usize, b: usize| ((PTYN[a] as u16) << 8) | PTYN[b] as u16;
    [PI, b, w(i, i + 1), w(i + 2, i + 3)]
}

/// A minute of a station's output, in the proportions one actually transmits:
/// the name and the text come round often, the housekeeping rarely.
fn programme() -> Vec<[u16; 4]> {
    let mut g = Vec::new();
    for seg in 0..4 {
        g.push(group_0a(seg));
        g.push(group_2a(seg * 2));
        g.push(group_2a(seg * 2 + 1));
    }
    g.push(group_1a(0xE0));
    g.push(group_3a());
    g.push(group_11a());
    g.push(group_4a());
    g.push(group_10a(0));
    g.push(group_10a(1));
    g
}

// ---------------------------------------------------------------------------
// Running the receiver
// ---------------------------------------------------------------------------

/// Push `iq` through a WFM demod in varying block sizes and return the last RDS
/// snapshot it produced, plus every group it logged.
///
/// Block sizes vary on purpose: a decoder that quietly assumes the caller's
/// chunking lines up with anything is broken and this is what shows it.
fn receive(rate: f64, iq: &[C32]) -> (Option<RdsData>, usize) {
    let mut demod = make_demod(Mode::Wfm, rate).expect("WFM has a demodulator");
    let mut audio = Vec::new();
    let mut latest: Option<RdsData> = None;
    let mut logged = 0usize;
    let mut sizes = [4_096usize, 1_500, 8_192, 333, 2_048].into_iter().cycle();

    let mut at = 0usize;
    while at < iq.len() {
        let n = sizes.next().expect("cycle never ends").min(iq.len() - at);
        audio.clear();
        demod.process(&iq[at..at + n], &mut audio);
        at += n;
        if let Some(d) = demod.take_rds() {
            logged += d.groups.len();
            latest = Some(d);
        }
    }
    (latest, logged)
}

/// The station picture after eight seconds of a nominal signal.
fn decode_nominal(rate: f64, st: &Station) -> RdsData {
    let iq = broadcast(rate, 8.0, &programme(), st);
    let (data, logged) = receive(rate, &iq);
    let data = data.expect("the decoder produced no snapshot at all");
    assert!(logged > 0, "no groups were logged");
    data
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn a_broadcast_station_gives_up_its_name_and_text() {
    let data = decode_nominal(240_000.0, &Station::default());

    assert!(data.sync, "block sync was never confirmed");
    assert_eq!(data.pi, Some(PI));
    assert_eq!(data.ps.as_deref(), Some("SDROXIDE"));
    assert_eq!(data.radiotext.as_deref(), Some(RT));
    assert_eq!(data.pty, Some(PTY as u8));
    assert!(data.tp, "traffic programme flag");
    assert!(!data.ta, "no announcement is running");
    assert_eq!(data.music, Some(true));
    assert_eq!(data.ptyn.as_deref(), Some("CHARTS"));

    // The programme type is a raw code, read against whichever table applies.
    assert_eq!(pty_name(data.pty.expect("decoded above"), RdsStandard::Rds), "Pop Music");
    assert_eq!(pty_name(data.pty.expect("decoded above"), RdsStandard::Rbds), "Country");

    // The extended country code arrived, so Auto settles on RDS despite this
    // station's PI being outside the North American range anyway.
    assert_eq!(data.ecc, Some(0xE0));
    assert_eq!(RdsStandard::Auto.resolve(data.pi, data.ecc), RdsStandard::Rds);

    // Both ends of the broadcast band, from the two AF codes in every 0A.
    assert_eq!(data.af, vec![87_600_000, 107_900_000]);

    let clock = data.clock.expect("group 4A carries a clock");
    assert_eq!((clock.year, clock.month, clock.day), (2026, 8, 17));
    assert_eq!((clock.hour, clock.minute), (14, 32));
    assert_eq!(clock.offset_half_hours, 4);
    assert_eq!(clock.label(), "2026-08-17 14:32 UTC +02:00");
}

#[test]
fn radiotext_plus_pulls_the_artist_and_title_out_of_the_text() {
    let data = decode_nominal(240_000.0, &Station::default());
    let rtp = data.rt_plus.expect("group 11A carries RadioText+ tags");
    assert_eq!(rtp.artist.as_deref(), Some("THE PILOTS"));
    assert_eq!(rtp.title.as_deref(), Some("THIRD HARMONIC"));
    // The tags index into the text that was decoded, so they have to agree
    // with it — a tag lifted from a different RadioText would still be a string.
    let rt = data.radiotext.expect("text decoded above");
    assert!(rt.contains(rtp.artist.as_deref().expect("artist above")));
    assert!(rt.contains(rtp.title.as_deref().expect("title above")));
}

#[test]
fn the_subcarrier_decodes_in_either_phase_against_the_pilot() {
    // The standard allows the data subcarrier in phase *or* in quadrature with
    // the pilot's third harmonic. A receiver that derived its carrier from the
    // pilot's phase would decode exactly one of these and null the other.
    for phase in [SubPhase::InPhase, SubPhase::Quadrature] {
        let st = Station { phase, ..Default::default() };
        let data = decode_nominal(240_000.0, &st);
        assert!(data.sync);
        assert_eq!(data.pi, Some(PI));
        assert_eq!(data.ps.as_deref(), Some("SDROXIDE"));
    }
}

#[test]
fn a_mono_station_with_no_pilot_still_decodes() {
    // The pilot only ever *aids* the down-converter. With none at all the Costas
    // loop has to find the suppressed carrier on its own, which is the case a
    // pilot-derived design fails outright.
    let st = Station { pilot: false, ..Default::default() };
    let data = decode_nominal(240_000.0, &st);
    assert!(data.sync, "no sync without a pilot to lean on");
    assert_eq!(data.pi, Some(PI));
    assert_eq!(data.ps.as_deref(), Some("SDROXIDE"));
}

#[test]
fn every_channel_rate_the_wfm_path_produces_decodes() {
    // The DDC settles on a different channel rate for each device rate, and the
    // samples-per-symbol the timing loop sees follows it: 240 kHz gives 12.6,
    // 256 kHz gives 13.5, 192 kHz gives 13.5 off a shorter composite.
    for rate in [256_000.0, 240_000.0, 192_000.0] {
        let data = decode_nominal(rate, &Station::default());
        assert!(data.sync, "no sync at {rate} Hz");
        assert_eq!(data.pi, Some(PI), "wrong station at {rate} Hz");
        assert_eq!(data.ps.as_deref(), Some("SDROXIDE"), "no name at {rate} Hz");
    }
}

#[test]
fn a_weakly_injected_subcarrier_still_decodes() {
    // 2 % of peak deviation is the bottom of the range stations actually use,
    // and a decoder tuned on a comfortable 4 % can quietly fail there. On a clean
    // signal this decodes at 0.5 % with no measurable penalty — the floor here is
    // interference, not level — so 1 % is asserted to leave the test somewhere to
    // fail before the figure that matters does.
    for injection in [0.02, 0.01] {
        let st = Station { rds_injection: injection, ..Default::default() };
        let data = decode_nominal(240_000.0, &st);
        assert!(data.sync, "no sync at {:.1} % injection", injection * 100.0);
        assert_eq!(data.pi, Some(PI));
        assert_eq!(data.ps.as_deref(), Some("SDROXIDE"));
    }
}

#[test]
fn an_unshaped_encoder_is_survivable() {
    // A hard biphase square instead of the shaped pulse the standard calls for.
    // The receiver's matched filter is a sine, so this is the mismatched case —
    // and one the decoder has to ride out, because what is on the air is not
    // always what the standard says should be.
    let st = Station { pulse: Pulse::Square, ..Default::default() };
    let data = decode_nominal(240_000.0, &st);
    assert!(data.sync);
    assert_eq!(data.pi, Some(PI));
    assert_eq!(data.ps.as_deref(), Some("SDROXIDE"));
    assert_eq!(data.radiotext.as_deref(), Some(RT));
}

#[test]
fn a_noisy_signal_decodes_and_says_how_hard_it_was() {
    // Measured on this signal: block error rate runs 0.002 up to a noise
    // amplitude of 0.15, 0.08 at 0.20, 0.31 at 0.25, and sync starts dropping out
    // by 0.30. 0.20 is therefore a genuinely marginal signal that still decodes.
    // The point is not the exact figure but that the counters move — a block
    // error rate pinned at zero on a signal this noisy would mean the corrector
    // was accepting anything put in front of it.
    let st = Station { noise: 0.20, ..Default::default() };
    let iq = broadcast(240_000.0, 10.0, &programme(), &st);
    let (data, _) = receive(240_000.0, &iq);
    let data = data.expect("a snapshot even on a noisy signal");
    assert_eq!(data.pi, Some(PI), "the station's identity survived the noise");
    assert!(data.stats.groups > 20, "only {} groups decoded", data.stats.groups);
    let ber = data.stats.block_error_rate();
    assert!(ber > 0.0, "a signal this noisy cannot have a perfect block record");
    assert!(ber < 0.6, "block error rate {ber:.3} is past the point of decoding");
}

#[test]
fn a_dead_frequency_never_claims_a_station() {
    // The test that matters most. A decoder that invents a programme
    // identification out of noise is worse than one that decodes nothing.
    let mut rng = Rng(0xDEAD_BEEF_0BAD_F00D);
    let rate = 240_000.0;
    let iq: Vec<C32> =
        (0..(rate * 6.0) as usize).map(|_| C32::new(rng.next_f32(), rng.next_f32())).collect();
    let (data, _) = receive(rate, &iq);
    if let Some(d) = data {
        assert!(!d.sync, "claimed block sync on pure noise");
        assert!(d.pi.is_none(), "invented programme identification {:04X?}", d.pi);
        assert!(d.ps.is_none(), "invented a station name {:?}", d.ps);
        assert!(d.radiotext.is_none(), "invented radio text {:?}", d.radiotext);
    }
}

#[test]
fn a_stereo_station_without_rds_never_claims_a_station() {
    // The other half of the negative case: a real, strong, perfectly good
    // broadcast signal that simply has nothing on 57 kHz. The pilot is locked
    // and the audio is fine, so anything that reports RDS here is reading the
    // programme material.
    let rate = 240_000.0;
    let st = Station { rds_injection: 0.0, ..Default::default() };
    let iq = broadcast(rate, 6.0, &programme(), &st);
    let mut demod = make_demod(Mode::Wfm, rate).expect("WFM has a demodulator");
    let mut audio = Vec::new();
    let mut latest = None;
    for chunk in iq.chunks(4_096) {
        audio.clear();
        demod.process(chunk, &mut audio);
        if let Some(d) = demod.take_rds() {
            latest = Some(d);
        }
    }
    assert!(demod.stereo_locked(), "the pilot should still be locked");
    if let Some(d) = latest {
        assert!(!d.sync, "claimed sync on a station with no data subcarrier");
        assert!(d.pi.is_none(), "invented programme identification {:04X?}", d.pi);
        assert!(d.ps.is_none(), "invented a station name {:?}", d.ps);
    }
}

#[test]
fn a_retune_does_not_leave_the_previous_station_on_screen() {
    let rate = 240_000.0;
    let iq = broadcast(rate, 8.0, &programme(), &Station::default());
    let mut demod = make_demod(Mode::Wfm, rate).expect("WFM has a demodulator");
    let mut audio = Vec::new();
    let mut latest = None;
    for chunk in iq.chunks(4_096) {
        audio.clear();
        demod.process(chunk, &mut audio);
        if let Some(d) = demod.take_rds() {
            latest = Some(d);
        }
    }
    assert_eq!(latest.and_then(|d| d.ps), Some("SDROXIDE".to_string()));

    // The engine says so on a retune; the demod cannot see one for itself.
    demod.reset_rds();
    audio.clear();
    demod.process(&iq[..4_096], &mut audio);
    let after = demod.take_rds().expect("a cleared snapshot is still a snapshot");
    assert!(after.pi.is_none(), "the old station's identity survived a reset");
    assert!(after.ps.is_none(), "the old station's name survived a reset");
    assert!(!after.sync);
}

#[test]
fn the_group_log_is_a_delta_and_its_sequence_never_goes_backwards() {
    // The diagnostics view accumulates these client-side, so each snapshot must
    // hand over each group exactly once and say where the batch starts.
    let rate = 240_000.0;
    let iq = broadcast(rate, 6.0, &programme(), &Station::default());
    let mut demod = make_demod(Mode::Wfm, rate).expect("WFM has a demodulator");
    let mut audio = Vec::new();
    let mut expect_seq = 0u64;
    let mut total = 0usize;
    let mut kinds = 0usize;
    for chunk in iq.chunks(4_096) {
        audio.clear();
        demod.process(chunk, &mut audio);
        let Some(d) = demod.take_rds() else { continue };
        assert_eq!(d.group_seq, expect_seq, "a group was handed over twice or lost");
        expect_seq += d.groups.len() as u64;
        total += d.groups.len();
        kinds += d.groups.iter().filter(|g| g.kind().is_some()).count();
    }
    // About eleven groups a second, so six seconds is well over fifty.
    assert!(total > 50, "only {total} groups logged");
    assert!(kinds * 4 > total * 3, "most logged groups should name their type");
}

#[test]
fn the_station_survives_being_fed_one_sample_at_a_time() {
    // A pathological caller. Everything in the chain buffers internally, and a
    // decoder that only works on comfortable block sizes would show up here.
    let rate = 240_000.0;
    let iq = broadcast(rate, 8.0, &programme(), &Station::default());
    let mut demod = make_demod(Mode::Wfm, rate).expect("WFM has a demodulator");
    let mut audio = Vec::new();
    let mut latest = None;
    // One sample at a time for the first tenth of a second, then normally —
    // running the whole eight seconds this way is minutes of test time for no
    // extra coverage, since what is being checked is that state carries across
    // a call boundary at all.
    let split = (rate * 0.1) as usize;
    for i in 0..split {
        audio.clear();
        demod.process(&iq[i..i + 1], &mut audio);
        if let Some(d) = demod.take_rds() {
            latest = Some(d);
        }
    }
    for chunk in iq[split..].chunks(4_096) {
        audio.clear();
        demod.process(chunk, &mut audio);
        if let Some(d) = demod.take_rds() {
            latest = Some(d);
        }
    }
    let data = latest.expect("a snapshot");
    assert_eq!(data.pi, Some(PI));
    assert_eq!(data.ps.as_deref(), Some("SDROXIDE"));
}

/// Run `iq` through `demod` and keep the last snapshot it produced.
fn run_on(demod: &mut Box<dyn sdroxide_dsp::Demodulator>, iq: &[C32]) -> Option<RdsData> {
    let mut audio = Vec::new();
    let mut latest = None;
    for chunk in iq.chunks(4_096) {
        audio.clear();
        demod.process(chunk, &mut audio);
        if let Some(d) = demod.take_rds() {
            latest = Some(d);
        }
    }
    latest
}

/// Six seconds of an empty dial: receiver noise and nothing else.
fn dead_air(rate: f64) -> Vec<C32> {
    let mut rng = Rng(7);
    (0..(rate * 6.0) as usize).map(|_| C32::new(rng.next_f32(), rng.next_f32())).collect()
}

#[test]
fn a_retune_from_empty_dial_still_decodes_a_medium_station() {
    // The field report: a medium station showed NO SYNC, 99.5 % of blocks bad
    // and nearly all of the rest "corrected", until the operator tuned to a
    // strong station and came back. The hunt had taken a block with a bit error
    // for the wrong offset word and locked one block out of step, which the
    // corrector then never let fail. Whether it happens depends on where the
    // station's bits fall when the hunt starts, so the station is tried at eight
    // bit phases; on the old decoder one of them read 99.7 % bad with no name.
    let rate = 240_000.0;
    let medium = broadcast(rate, 8.0, &programme(), &Station { noise: 0.10, ..Default::default() });
    let symbol = rate / 1_187.5;
    for eighth in 0..8 {
        let skip = (eighth as f64 * symbol / 8.0) as usize;
        let mut demod = make_demod(Mode::Wfm, rate).expect("WFM has a demodulator");
        run_on(&mut demod, &dead_air(rate));
        demod.reset_rds();
        let data = run_on(&mut demod, &medium[skip..]).expect("a snapshot from a real station");
        let ber = data.stats.block_error_rate();
        assert!(data.sync, "no sync at bit phase {eighth}/8: {ber:.3} of blocks bad");
        assert_eq!(data.pi, Some(PI), "wrong identity at bit phase {eighth}/8");
        assert_eq!(data.ps.as_deref(), Some("SDROXIDE"), "no name at bit phase {eighth}/8");
        assert!(ber < 0.1, "{ber:.3} of blocks bad at bit phase {eighth}/8");
    }
}
