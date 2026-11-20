//! Bring the VHF front end up by hand, one assumption at a time.
//!
//! ```text
//! cargo run -p sdroxide-rx888 --example vhf_check -- --mhz 93.2
//! ```
//!
//! Nothing in the VHF path has ever been on hardware, and it rests on four
//! assumptions that fail in four different ways. This walks them in the order
//! they can fail, so the first thing that breaks is also the first thing
//! reported:
//!
//! 1. **The I2C passthrough addressing.** The published API reference and the
//!    firmware source disagree about which of `wValue`/`wIndex` carries the
//!    device address. If this is wrong nothing else can work, and it fails
//!    silently — so the tuner's identity register is read before anything else.
//! 2. **The tuner's reference clock.** CLK2 has to be running before the tuner
//!    is spoken to: its initialisation calibrates the IF filter against a PLL,
//!    and with no clock that calibration cannot lock.
//! 3. **The RF switch and the tuning.** With an antenna on the VHF port and a
//!    known station in range, energy should appear at the IF.
//! 4. **The spectrum inversion.** High-side injection means the IF runs
//!    backwards, and the only way to be sure of the sign is to move the tuner
//!    and watch which way the signal goes.
//!
//! Receive only. Both bias tees are left off throughout.

use std::time::{Duration, Instant};

use nusb::transfer::{Bulk, In};
use sdroxide_dsp::WideSpectrum;
use sdroxide_r82xx::{Board, Chip, R82xx, R828D_I2C_ADDR};

use sdroxide_rx888::device::{Device, Settings};
use sdroxide_rx888::{band, convert, protocol, si5351, tuner};

const XFER: usize = 256 * 1024;
const IN_FLIGHT: usize = 8;
const FFT: usize = 8192;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sdroxide_rx888=info".into()),
        )
        .init();

    let dial_hz = std::env::args()
        .skip_while(|a| a != "--mhz")
        .nth(1)
        .and_then(|s| s.parse::<f64>().ok())
        .map(|m| m * 1e6)
        .unwrap_or(93_200_000.0);

    let settings = Settings::default();
    let mut dev = match Device::open(&settings, None) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("open failed: {e}");
            std::process::exit(1);
        }
    };
    let adc = dev.adc_rate_hz();
    println!("Receiver: {}", dev.label());
    println!("  ADC clock  {:.3} Msps  (Nyquist {:.3} MHz)", adc / 1e6, adc / 2e6);
    println!("  target     {:.4} MHz", dial_hz / 1e6);
    if !band::adc_rate_allows_vhf(adc) {
        println!("  ** this ADC clock has no room for the tuner's IF; VHF cannot work here");
        return;
    }

    // ---- 1. addressing ---------------------------------------------------
    println!("\n1. I2C passthrough — does the tuner answer?");
    let mut id = [0u8; 1];
    match dev.usb().i2c_read(R828D_I2C_ADDR, 0, &mut id) {
        Ok(()) => {
            println!(
                "   register 0 reads 0x{:02x} (want 0x{:02x}, raw — the part's identity byte)",
                id[0],
                sdroxide_r82xx::R82XX_CHECK_VAL
            );
            if !tuner::present(dev.usb(), R828D_I2C_ADDR) {
                println!(
                    "   ** the transfer succeeded but the value is wrong. Either the device\n\
                     ** address and register are swapped, or there is no R828D on this board."
                );
                return;
            }
            println!("   => an R828D is present and the addressing is right.");
        }
        Err(e) => {
            println!("   ** read failed: {e}");
            println!("   ** a STALL here usually means the firmware rejected the addressing.");
            return;
        }
    }

    // ---- 2. the reference clock ------------------------------------------
    println!("\n2. Tuner reference clock on Si5351 CLK2");
    match si5351::set_clk2_hz(dev.usb(), si5351::TUNER_REF_HZ) {
        Ok(hz) => println!("   CLK2 programmed to {:.6} MHz", hz / 1e6),
        Err(e) => {
            println!("   ** failed: {e}");
            return;
        }
    }
    // The ADC clock is on the same chip. If PLLA got disturbed, the stream
    // stops or changes rate, and that must be caught here rather than blamed on
    // the tuner later.
    match dev.stats() {
        Ok(s) => println!(
            "   ADC clock still {} (si5351 clk0 {})",
            if s.clock_running() { "running" } else { "** STOPPED **" },
            s.si5351_clk0_control
        ),
        Err(e) => println!("   (could not read stats: {e})"),
    }

    // ---- 3. the front end and the tuner ----------------------------------
    println!("\n3. VHF front end");
    // Kill the HF chain: both front ends share the ADC.
    if let Err(e) = dev.set_attenuator_db(-31.5) {
        println!("   ** attenuator: {e}");
    }
    if let Err(e) = dev.set_vhf_en(true) {
        println!("   ** VHF_EN: {e}");
    }
    // Upstream's fixed VGA setting for this path: high range, code 3.
    if let Err(e) = dev.set_vga_reg(0x80 | 3) {
        println!("   ** VGA: {e}");
    }
    println!("   gpio 0x{:08x} (VHF_EN {})", dev.gpio_word(), on_off(dev.gpio_word(), 15));

    let mut tuner = R82xx::new(Chip::R828D, Board::Plain, 0);
    if let Err(e) = tuner.init(dev.usb()) {
        println!("   ** tuner init failed: {e}");
        println!("   ** a PLL that will not lock here usually means CLK2 is not reaching it.");
        cleanup(&mut dev, &mut tuner);
        return;
    }
    match tuner.set_bandwidth(dev.usb(), band::IF_BW_HZ) {
        Ok(iff) => println!("   IF filter {:.3} MHz wide, carrier at {:.4} MHz", 8.0, iff / 1e6),
        Err(e) => println!("   ** set_bandwidth: {e}"),
    }
    if let Err(e) = tuner.set_freq(dev.usb(), dial_hz) {
        println!("   ** tune failed: {e}");
        cleanup(&mut dev, &mut tuner);
        return;
    }
    println!("   tuned; LO should be {:.4} MHz", (dial_hz + band::IF_CENTER_HZ) / 1e6);

    // ---- streaming -------------------------------------------------------
    if let Err(e) = dev.start() {
        println!("   ** STARTFX3: {e}");
        cleanup(&mut dev, &mut tuner);
        return;
    }
    let mut ep = match dev.usb().interface().endpoint::<Bulk, In>(protocol::BULK_EP) {
        Ok(e) => e,
        Err(e) => {
            println!("   ** bulk endpoint: {e}");
            cleanup(&mut dev, &mut tuner);
            return;
        }
    };
    for _ in 0..IN_FLIGHT {
        ep.submit(ep.allocate(XFER));
    }

    let lo_if = band::IF_CENTER_HZ - band::IF_BW_HZ / 2.0;
    let hi_if = band::IF_CENTER_HZ + band::IF_BW_HZ / 2.0;
    println!("\n   strongest bins in the IF window ({:.2}–{:.2} MHz):", lo_if / 1e6, hi_if / 1e6);
    let base = match peak(&mut ep, adc, dev.randomized(), lo_if, hi_if) {
        Some(p) => {
            println!("   peak at {:.4} MHz IF, {:.1} dBFS", p.0 / 1e6, p.1);
            p
        }
        None => {
            println!("   ** no samples arrived");
            drain(&mut ep);
            cleanup(&mut dev, &mut tuner);
            return;
        }
    };

    // ---- 4. which way does the spectrum run? -----------------------------
    //
    // Moving the tuner and watching which way the peak goes does *not* settle
    // this on its own: a fixed station's IF moves with the LO under either
    // injection sense, only in opposite directions, and "the peak went up" is
    // equally consistent with a different station having become the strongest.
    //
    // What settles it is consistency. Each injection sense implies a different
    // RF for the same IF reading, so solving both hypotheses at two LO
    // positions and seeing which one names the *same station twice* is proof
    // rather than inference.
    println!("\n4. Spectrum sense — solving for the station at two LO positions");
    let moved_dial = dial_hz + 1_000_000.0;
    if let Err(e) = tuner.set_freq(dev.usb(), moved_dial) {
        println!("   ** retune failed: {e}");
    }
    std::thread::sleep(Duration::from_millis(200));
    match peak(&mut ep, adc, dev.randomized(), lo_if, hi_if) {
        Some(p) => {
            println!("   peak now at {:.4} MHz IF, {:.1} dBFS", p.0 / 1e6, p.1);
            println!("   IF moved {:+.4} MHz for an LO moved +1.0000 MHz", (p.0 - base.0) / 1e6);

            // High side: LO = dial + IF_CENTER, so RF = LO - if.
            let high = |dial: f64, iff: f64| dial + band::IF_CENTER_HZ - iff;
            // Low side: LO = dial - IF_CENTER, so RF = LO + if.
            let low = |dial: f64, iff: f64| dial - band::IF_CENTER_HZ + iff;

            let (h1, h2) = (high(dial_hz, base.0), high(moved_dial, p.0));
            let (l1, l2) = (low(dial_hz, base.0), low(moved_dial, p.0));
            println!(
                "   if high-side: {:.4} then {:.4} MHz  (differ by {:+.1} kHz)",
                h1 / 1e6,
                h2 / 1e6,
                (h2 - h1) / 1e3
            );
            println!(
                "   if low-side:  {:.4} then {:.4} MHz  (differ by {:+.1} kHz)",
                l1 / 1e6,
                l2 / 1e6,
                (l2 - l1) / 1e3
            );

            const AGREE_HZ: f64 = 100_000.0;
            match ((h2 - h1).abs() < AGREE_HZ, (l2 - l1).abs() < AGREE_HZ) {
                (true, false) => println!(
                    "   => HIGH-SIDE injection: only that hypothesis names one station twice.\n\
                     =>    The IF is therefore mirrored, so `conjugate` is true in VHF and the\n\
                     =>    downconverter moves down as the dial moves up. band.rs is correct."
                ),
                (false, true) => println!(
                    "   ** LOW-SIDE injection — the spectrum is NOT mirrored. `conjugate` must be\n\
                     ** false and `ddc_center_hz` must be IF_CENTER + (dial - lo). Both signs in\n\
                     ** band.rs are wrong and have to be flipped together."
                ),
                _ => println!(
                    "   ** inconclusive: either the strongest signal changed between the two\n\
                     ** readings, or the tuner did not move. Try a quieter --mhz."
                ),
            }
        }
        None => println!("   ** no samples"),
    }

    drain(&mut ep);
    cleanup(&mut dev, &mut tuner);
    println!("\nDone. HF path restored.");
}

fn on_off(word: u32, bit: u32) -> &'static str {
    if word & (1 << bit) != 0 { "set" } else { "clear" }
}

/// Put the receiver back the way an HF user expects to find it.
fn cleanup(dev: &mut Device, tuner: &mut R82xx) {
    let _ = tuner.standby(dev.usb());
    let _ = si5351::clk2_off(dev.usb());
    let _ = dev.set_vhf_en(false);
    let _ = dev.set_vga_db(12.0);
    let _ = dev.set_attenuator_db(0.0);
    let _ = dev.stop();
}

fn drain(ep: &mut nusb::Endpoint<Bulk, In>) {
    ep.cancel_all();
    let until = Instant::now() + Duration::from_millis(500);
    while ep.pending() > 0 && Instant::now() < until {
        if ep.wait_next_complete(Duration::from_millis(50)).is_none() {
            break;
        }
    }
}

/// Strongest bin between `lo_hz` and `hi_hz`, as `(frequency, dBFS)`.
fn peak(
    ep: &mut nusb::Endpoint<Bulk, In>,
    adc_hz: f64,
    randomized: bool,
    lo_hz: f64,
    hi_hz: f64,
) -> Option<(f64, f64)> {
    // Everything already queued predates the caller's retune.
    for _ in 0..(IN_FLIGHT * 3) {
        while ep.pending() < IN_FLIGHT {
            ep.submit(ep.allocate(XFER));
        }
        if let Some(c) = ep.wait_next_complete(Duration::from_millis(300)) {
            let mut b = c.buffer;
            b.clear();
            ep.submit(b);
        }
    }

    let mut wide = WideSpectrum::new(adc_hz, FFT, 1000.0);
    let mut carry = None;
    let mut real = Vec::new();
    let mut frame = None;
    for _ in 0..40 {
        while ep.pending() < IN_FLIGHT {
            ep.submit(ep.allocate(XFER));
        }
        let c = ep.wait_next_complete(Duration::from_millis(300))?;
        if c.status.is_ok() {
            convert::to_f32(&c.buffer[..], randomized, &mut carry, &mut real);
            wide.process(&real);
            if let Some(f) = wide.take() {
                frame = Some(f);
                break;
            }
        }
        let mut b = c.buffer;
        b.clear();
        ep.submit(b);
    }

    let f = frame?;
    let bin_hz = adc_hz / 2.0 / f.len() as f64;
    let lo = (lo_hz / bin_hz) as usize;
    let hi = ((hi_hz / bin_hz) as usize).min(f.len());
    let (mut best, mut best_db) = (lo, f64::NEG_INFINITY);
    for (i, v) in f.iter().enumerate().take(hi).skip(lo) {
        if f64::from(*v) > best_db {
            best_db = f64::from(*v);
            best = i;
        }
    }
    Some((best as f64 * bin_hz, best_db))
}
