//! The modem performance curve under AWGN — reproduces the behaviour measured
//! in CLAUDE.md:
//!   - QPSK: 100% down to ~18 dB SNR, a sharp "cliff" at ~10–12 dB.
//!   - BPSK: still mostly working at 10 dB (while QPSK is ~0).
//!
//! `#[ignore]` by default because it is slow. To run it:
//!   cargo test -p modem --test awgn_sweep -- --ignored --nocapture

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rand_distr::{Distribution, Normal};
use sdroxide_atchat::modem::{Mode, Modem};

/// An exact equivalent of the AWGN part of `channel_server.py::_apply_channel`.
fn add_awgn(x: &[i16], snr_db: f64, rng: &mut StdRng) -> Vec<i16> {
    let sig_power: f64 =
        (x.iter().map(|&s| (s as f64) * (s as f64)).sum::<f64>() / x.len() as f64).max(1.0);
    let noise_power = sig_power / 10f64.powf(snr_db / 10.0);
    let n = Normal::new(0.0, noise_power.sqrt()).unwrap();
    x.iter()
        .map(|&s| {
            let v = s as f64 + n.sample(rng);
            v.clamp(-32768.0, 32767.0) as i16
        })
        .collect()
}

fn success_rate(mode: Mode, snr_db: f64, trials: usize, rng: &mut StdRng) -> usize {
    let m = Modem::new();
    let mut ok = 0;
    for _ in 0..trials {
        let payload: Vec<u8> = (0..200).map(|_| rng.r#gen()).collect();
        let lead = rng.gen_range(20..300);
        let clean = m.modulate_with_leadin(&payload, mode, lead);
        let noisy = add_awgn(&clean, snr_db, rng);
        if m.demodulate(&noisy).as_deref() == Some(payload.as_slice()) {
            ok += 1;
        }
    }
    ok
}

#[test]
#[ignore = "slow; run with -- --ignored --nocapture"]
fn awgn_performance_curve() {
    let trials = 20;
    let mut rng = StdRng::seed_from_u64(0x00A7_C4A7);

    println!("\n  SNR(dB) |  QPSK  |  BPSK   (successes / {trials})");
    println!("  --------+--------+-------");
    let mut results = Vec::new();
    for &snr in &[24.0, 18.0, 16.0, 14.0, 12.0, 10.0] {
        let q = success_rate(Mode::Qpsk, snr, trials, &mut rng);
        let b = success_rate(Mode::Bpsk, snr, trials, &mut rng);
        println!("  {snr:6.0}  |  {q:2}/{trials}  |  {b:2}/{trials}");
        results.push((snr, q, b));
    }

    let get = |snr: f64| results.iter().find(|(s, _, _)| *s == snr).unwrap();
    // At the clean end QPSK is nearly perfect.
    assert!(get(24.0).1 >= 18, "QPSK@24dB too low: {:?}", get(24.0));
    assert!(get(18.0).1 >= 17, "QPSK@18dB too low: {:?}", get(18.0));
    // The cliff: at 10 dB QPSK collapses, BPSK stays up.
    assert!(get(10.0).1 <= 6, "QPSK@10dB should have hit the cliff: {:?}", get(10.0));
    assert!(get(10.0).2 >= 12, "BPSK@10dB should have held up: {:?}", get(10.0));
}
