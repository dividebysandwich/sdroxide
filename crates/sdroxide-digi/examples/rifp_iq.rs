//! RIFP ↔ raw-IQ bridge, for interoperability testing against other
//! implementations (the reference `radiofax_sender.py` / `radiofax_receiver.py`
//! read and write exactly this format with `--iq-output` / `--iq-input`).
//!
//! It drives the *real* modulator and demodulator the engine uses, so what this
//! writes is what sdroxide transmits, and what it decodes is what sdroxide
//! would decode off the air.
//!
//! ```text
//! cargo run -p sdroxide-digi --example rifp_iq -- send picture.png out.iq [zlib|png|g4|raw|rle8|jpeg|auto] [bits]
//! cargo run -p sdroxide-digi --example rifp_iq -- recv in.iq out.png
//! ```
//!
//! The IQ is interleaved little-endian `complex64` (two f32s per sample) at
//! 48 kHz — ten samples per 4800-baud symbol, which the profile permits (§10:
//! "A sender MAY use a different sample rate when it is an integer multiple of
//! the symbol rate").

use std::time::{Duration, SystemTime};

use sdroxide_digi::{DigiEngine, RifpController, controller::DigiAction};
use sdroxide_dsp::{Complex32, make_demod, make_modulator};
use sdroxide_types::{DigiConfig, Mode, RifpEncoding};

const RATE: f64 = 48_000.0;
const BLOCK: usize = 480;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("send") if args.len() >= 4 => send(&args)?,
        Some("recv") if args.len() >= 4 => recv(&args[2], &args[3])?,
        _ => {
            eprintln!(
                "usage:\n  rifp_iq send <picture.png> <out.iq> [encoding] [bits]\n  \
                 rifp_iq recv <in.iq> <out.png>"
            );
            std::process::exit(2);
        }
    }
    Ok(())
}

fn encoding_from(name: Option<&String>) -> RifpEncoding {
    match name.map(String::as_str) {
        Some("png") => RifpEncoding::Png,
        Some("jpeg") => RifpEncoding::Jpeg,
        Some("raw") => RifpEncoding::Raw,
        Some("rle8") => RifpEncoding::Rle8,
        Some("g4" | "group4") => RifpEncoding::Group4,
        Some("zlib") => RifpEncoding::Zlib,
        _ => RifpEncoding::Auto,
    }
}

fn send(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let img = image::open(&args[2])?.to_rgb8();
    let (w, h) = (img.width() as u16, img.height() as u16);
    let cfg = DigiConfig {
        my_call: std::env::var("RIFP_CALL").unwrap_or_else(|_| "SDROXIDE".into()),
        rifp_encoding: encoding_from(args.get(4)),
        rifp_bits_per_pixel: args.get(5).and_then(|b| b.parse().ok()).unwrap_or(4),
        rifp_content_hint: "sdroxide interop".into(),
        ..DigiConfig::default()
    };

    let mut tx = RifpController::new(cfg, RATE);
    tx.set_rifp_image(img.into_raw(), w, h);
    let mut modulator =
        make_modulator(Mode::Rifp, RATE, Mode::Rifp.default_filter()).ok_or("no RIFP modulator")?;

    let mut out: Vec<u8> = Vec::new();
    let mut symbols = [0.0f32; BLOCK];
    let mut iq: Vec<Complex32> = Vec::new();
    // A little silence each side, so a receiver's energy detector sees an edge.
    let lead = vec![0.0f32; (RATE * 0.05) as usize];
    for chunk in lead.chunks(BLOCK) {
        iq.clear();
        modulator.process(chunk, &mut iq);
        write_iq(&mut out, &iq);
    }
    loop {
        let done = tx.fill_tx_block(&mut symbols);
        iq.clear();
        modulator.process(&symbols, &mut iq);
        write_iq(&mut out, &iq);
        if done {
            break;
        }
    }
    for chunk in lead.chunks(BLOCK) {
        iq.clear();
        modulator.process(chunk, &mut iq);
        write_iq(&mut out, &iq);
    }

    std::fs::write(&args[3], &out)?;
    println!(
        "wrote {} ({} samples, {:.1} s at {} Hz)",
        args[3],
        out.len() / 8,
        out.len() as f64 / 8.0 / RATE,
        RATE
    );
    Ok(())
}

fn write_iq(out: &mut Vec<u8>, iq: &[Complex32]) {
    for z in iq {
        out.extend_from_slice(&z.re.to_le_bytes());
        out.extend_from_slice(&z.im.to_le_bytes());
    }
}

fn recv(input: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    let raw = std::fs::read(input)?;
    if !raw.len().is_multiple_of(8) {
        return Err("IQ file is not a whole number of complex64 samples".into());
    }
    let iq: Vec<Complex32> = raw
        .chunks_exact(8)
        .map(|c| {
            Complex32::new(
                f32::from_le_bytes(c[0..4].try_into().expect("4 bytes")),
                f32::from_le_bytes(c[4..8].try_into().expect("4 bytes")),
            )
        })
        .collect();
    println!("read {} samples ({:.1} s)", iq.len(), iq.len() as f64 / RATE);

    // The engine's own receive chain: the RIFP discriminator, then the
    // controller.
    let mut demod = make_demod(Mode::Rifp, RATE).ok_or("no RIFP demodulator")?;
    let mut rx = RifpController::new(DigiConfig::default(), RATE);
    let mut now = SystemTime::UNIX_EPOCH;
    let mut actions = Vec::new();
    let mut audio = Vec::new();
    for block in iq.chunks(BLOCK) {
        audio.clear();
        demod.process(block, &mut audio);
        rx.on_rx_audio(&audio);
        now += Duration::from_millis(10);
        actions.extend(rx.poll(now, 0.0));
    }
    // Flush the modem's pipeline.
    rx.on_rx_audio(&vec![0.0; BLOCK]);
    actions.extend(rx.poll(now, 0.0));

    let mut written = 0;
    for action in &actions {
        if let DigiAction::RifpImage { meta, w, h, rgb, .. } = action {
            println!(
                "picture from {}: {}×{} {}-bit, {} / {}, {} octets in {} chunks ({} first pass), \
                 session {}, file {:?}",
                meta.sender.as_deref().unwrap_or("(unidentified)"),
                meta.width,
                meta.height,
                meta.bits_per_pixel,
                meta.media_type,
                meta.content_encoding,
                meta.encoded_size,
                meta.chunk_count,
                meta.chunks_first_pass,
                meta.session,
                meta.filename,
            );
            if let Some(hint) = &meta.hint {
                println!("  hint: {hint}");
            }
            let img = image::RgbImage::from_raw(*w as u32, *h as u32, rgb.clone())
                .ok_or("bad picture buffer")?;
            img.save(output)?;
            println!("  wrote {output}");
            written += 1;
        }
    }
    if let Some(DigiAction::RifpStatus(s)) =
        actions.iter().rev().find(|a| matches!(a, DigiAction::RifpStatus(_)))
    {
        println!(
            "frames {} · bad CRC {} · pictures {}{}",
            s.rx_frames,
            s.rx_bad_frames,
            s.rx_objects,
            s.last_error.as_ref().map(|e| format!(" · last error: {e}")).unwrap_or_default()
        );
        for session in &s.sessions {
            println!(
                "  incomplete session {}: {}/{} chunks, manifest {}",
                session.session,
                session.have,
                session.total,
                if session.have_manifest { "yes" } else { "no" }
            );
        }
    }
    if written == 0 {
        return Err("no picture was completed".into());
    }
    Ok(())
}
