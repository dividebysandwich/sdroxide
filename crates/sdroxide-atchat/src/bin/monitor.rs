//! `atchat-monitor` — a passive listener on the virtual channel, for RF-less
//! testing.
//!
//! Not part of sdroxide itself. It connects to `atchat-channel` (or
//! `channel_server.py`), never transmits, demodulates every burst delivered on
//! the channel and prints the frame it decoded — the text-log half of
//! `monitor.py`, without the scope / spectrum / waterfall the standalone GUI
//! draws.
//!
//!   atchat-monitor --connect 127.0.0.1:6000

use clap::Parser;
use sdroxide_atchat::channel::{LinkRx, TcpConnector, b64_to_samples};
use sdroxide_atchat::modem::Modem;
use sdroxide_atchat::netproto::{Frame, SAMPLE_RATE, ServerMsg};

#[derive(Parser, Debug)]
#[command(about = "Passive AtCHAT NET channel monitor — RF-less test tool")]
struct Args {
    /// The channel address to connect to.
    #[arg(long, default_value = "127.0.0.1:6000")]
    connect: String,
}

fn now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
    format!("{h:02}:{m:02}:{s:02}")
}

fn summary(f: &Frame) -> String {
    let kind = match f {
        Frame::JoinRequest { .. } => "JOIN_REQUEST",
        Frame::Beacon { .. } => "BEACON",
        Frame::Chat { .. } => "CHAT",
        Frame::BulkMeta { .. } => "BULK_META",
        Frame::BulkBlock { .. } => "BULK_BLOCK",
        Frame::BulkEnd { .. } => "BULK_END",
        Frame::BulkStatus { .. } => "BULK_STATUS",
        Frame::Unknown => "UNKNOWN",
    };
    let src = f.src().unwrap_or("?");
    match f {
        Frame::Chat { text, .. } => format!("{src:8} -> {:8} | {kind} | {text:?}", f.dst()),
        _ => format!("{src:8} -> {:8} | {kind}", f.dst()),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .init();

    let args = Args::parse();
    let (_tx, mut rx) = TcpConnector::connect_once(&args.connect, "MONITOR").await?;
    println!("[MON {}] listening on {}", now(), args.connect);

    let modem = Modem::new();
    while let Some(msg) = rx.recv().await {
        let ServerMsg::RxAudio { audio_b64 } = msg else { continue };
        let samples = match b64_to_samples(&audio_b64) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let secs = samples.len() as f64 / SAMPLE_RATE as f64;
        match modem.demodulate(&samples).and_then(|p| Frame::from_json_bytes(&p)) {
            Some(frame) => println!("[MON {}] {} | {:.2}s | decoded", now(), summary(&frame), secs),
            None => println!("[MON {}] {:.2}s | undecoded", now(), secs),
        }
    }

    println!("[MON {}] channel closed", now());
    Ok(())
}
