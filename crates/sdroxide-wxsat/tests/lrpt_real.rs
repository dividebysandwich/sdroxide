//! The LRPT receiver on a real Meteor-M2 pass: 3 seconds recorded by F4NRM at
//! 137.900 MHz (I/Q, 187.5 kS/s). See `tests/testdata/lrpt/README.md`.

use sdroxide_dsp::{Complex32, ComplexResampler};
use sdroxide_wxsat::lrpt::demod::LRPT_LANE_RATE_HZ;
use sdroxide_wxsat::lrpt::rx::LrptRx;

/// The test recording as complex samples and its rate.
fn recording() -> (Vec<Complex32>, f64) {
    let raw = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/testdata/lrpt/meteor_lrpt_137900_3s.wav"
    ))
    .unwrap();
    assert_eq!(&raw[..4], b"RIFF");
    let rate = u32::from_le_bytes(raw[24..28].try_into().unwrap()) as f64;
    assert_eq!(u16::from_le_bytes(raw[22..24].try_into().unwrap()), 2, "I and Q");
    assert_eq!(u16::from_le_bytes(raw[34..36].try_into().unwrap()), 16);
    assert_eq!(&raw[36..40], b"data");
    let iq = raw[44..]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| {
            Complex32::new(
                f32::from(i16::from_le_bytes([c[0], c[1]])) / 32768.0,
                f32::from(i16::from_le_bytes([c[2], c[3]])) / 32768.0,
            )
        })
        .collect();
    (iq, rate)
}

fn run() -> (LrptRx, Vec<sdroxide_wxsat::lrpt::jpeg::Segment>) {
    let (iq, rate) = recording();
    let mut rs = ComplexResampler::new(rate, LRPT_LANE_RATE_HZ).expect("resampler");
    let mut rx = LrptRx::new();
    let mut segs = Vec::new();
    let mut lane = Vec::new();
    for chunk in iq.chunks(4096) {
        lane.clear();
        rs.push(chunk, &mut lane);
        segs.extend(rx.process(&lane));
    }
    (rx, segs)
}

#[test]
fn a_real_pass_gives_frames_and_picture_segments() {
    let (rx, segs) = run();
    let st = rx.stats();
    let (carrier, evm) = rx.demod_figures();
    eprintln!("stats {st:?} carrier {carrier:.1} Hz evm {evm:.3} locked {}", rx.locked());
    // 2.6 s of symbols hold about 22 frames of 8192 bits at 72 kbit/s.
    assert!(st.frames_ok >= 18, "only {} good frames", st.frames_ok);
    assert_eq!(st.frames_bad, 0, "a frame that Reed-Solomon refused");
    assert!(st.segments >= 60, "{} segments", st.segments);
    assert_eq!(segs.len() as u64, st.segments);
    assert!(evm < 0.35, "the constellation is not clean: {evm}");
    assert!((carrier - 1170.0).abs() < 40.0, "carrier {carrier}");
    // Image channels of this satellite, and nothing else.
    assert!(segs.iter().all(|s| (64..=69).contains(&s.apid)));
    // They make a picture.
    let mut pic = sdroxide_wxsat::lrpt::picture::LrptPicture::new();
    segs.iter().for_each(|s| pic.push(s));
    assert!(pic.rows() >= 2 && pic.rows() <= 6, "{} rows", pic.rows());
}

/// The whole 105 s pass, for the person who has the file:
/// `LRPT_FULL_WAV=/path/LRPT_16-29-02_137900kHz.wav cargo test -p sdroxide-wxsat
/// --release --test lrpt_real -- --ignored --nocapture`. Writes one PGM per
/// channel next to the input.
#[test]
#[ignore = "needs the full recording"]
fn the_whole_pass() {
    let path = std::env::var("LRPT_FULL_WAV").expect("LRPT_FULL_WAV");
    let raw = std::fs::read(&path).unwrap();
    let rate = u32::from_le_bytes(raw[24..28].try_into().unwrap()) as f64;
    let iq: Vec<Complex32> = raw[44..]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| {
            Complex32::new(
                f32::from(i16::from_le_bytes([c[0], c[1]])) / 32768.0,
                f32::from(i16::from_le_bytes([c[2], c[3]])) / 32768.0,
            )
        })
        .collect();
    let mut rs = ComplexResampler::new(rate, LRPT_LANE_RATE_HZ).unwrap();
    let mut rx = LrptRx::new();
    let mut pic = sdroxide_wxsat::lrpt::picture::LrptPicture::new();
    let mut lane = Vec::new();
    let t = std::time::Instant::now();
    for chunk in iq.chunks(4096) {
        lane.clear();
        rs.push(chunk, &mut lane);
        rx.process(&lane).iter().for_each(|s| pic.push(s));
    }
    eprintln!(
        "{:?} in {:.1} s for {:.0} s of signal; {} rows",
        rx.stats(),
        t.elapsed().as_secs_f64(),
        iq.len() as f64 / rate,
        pic.rows()
    );
    for apid in pic.channels() {
        let (w, h, g) = pic.gray(apid).unwrap();
        let mut f = format!("P5\n{w} {h}\n255\n").into_bytes();
        f.extend_from_slice(&g);
        std::fs::write(format!("{path}.apid{apid}.pgm"), f).unwrap();
    }
}
