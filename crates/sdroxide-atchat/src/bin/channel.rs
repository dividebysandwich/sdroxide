//! `atchat-channel` — the headless TCP channel, for RF-less testing.
//!
//! Not part of sdroxide itself (the ATCHAT mode talks to the radio, or to this
//! over a socket in "virtual channel" mode). This is the bench version of the
//! channel physics: the same command line and the same wire as
//! `channel_server.py`, so `atchat-client`, `atchat-monitor` and the Python
//! `client.py` / `monitor.py` all connect to it unchanged.
//!
//!   atchat-channel --port 6000
//!   atchat-channel --port 6000 --snr 13 --multipath-delay-ms 3 --multipath-gain 0.2

use std::sync::Arc;

use clap::Parser;
use sdroxide_atchat::channel::{ChannelConfig, ChannelCore, ChannelEvent, tcp_server};
use tokio::net::TcpListener;

#[derive(Parser, Debug)]
#[command(about = "AtCHAT NET channel simulator (real-audio) — RF-less test tool")]
struct Args {
    #[arg(long, default_value_t = 6000)]
    port: u16,

    /// AWGN level (dB). Omitted means a clean channel.
    #[arg(long)]
    snr: Option<f64>,

    /// Multipath echo delay (ms). The guard interval is 8 ms.
    #[arg(long = "multipath-delay-ms", default_value_t = 0.0)]
    multipath_delay_ms: f64,

    /// Echo gain relative to the direct signal (0–1).
    #[arg(long = "multipath-gain", default_value_t = 0.0)]
    multipath_gain: f64,
}

fn now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
    format!("{h:02}:{m:02}:{s:02}")
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = Args::parse();
    let cfg = ChannelConfig {
        snr_db: args.snr,
        multipath_delay_ms: args.multipath_delay_ms,
        multipath_gain: args.multipath_gain,
        ..Default::default()
    };
    let core = ChannelCore::spawn(cfg);

    let listener = TcpListener::bind(("127.0.0.1", args.port)).await?;
    println!(
        "[CHAN {}] listening on 127.0.0.1:{}  (snr={:?}, multipath={}ms@{})",
        now(),
        args.port,
        args.snr,
        args.multipath_delay_ms,
        args.multipath_gain
    );

    let mut ev = core.subscribe_events();
    tokio::spawn(async move {
        loop {
            match ev.recv().await {
                Ok(e) => log_event(&e),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    });

    let serve = tcp_server::serve(listener, Arc::clone(&core));
    tokio::select! {
        r = serve => r,
        _ = tokio::signal::ctrl_c() => {
            println!("\n[CHAN {}] shutting down", now());
            Ok(())
        }
    }
}

fn log_event(e: &ChannelEvent) {
    let t = now();
    match e {
        ChannelEvent::Joined { callsign, active } => {
            println!("[CHAN {t}] {callsign} joined ({active} active)")
        }
        ChannelEvent::Left { callsign, active } => {
            println!("[CHAN {t}] {callsign} left ({active} active)")
        }
        ChannelEvent::TxGranted { src, n_samples, duration } => println!(
            "[CHAN {t}] {src:8} -> ALL      | audio | {n_samples:6} samples | {duration:.2}s"
        ),
        ChannelEvent::TxDenied { src, retry_after } => {
            println!("[CHAN {t}] {src:8} -> BUSY     | retry_after={retry_after:.2}s")
        }
        ChannelEvent::Decoded { duration, summary } => match summary {
            Some(s) => println!("[MON  {t}] {s} | {duration:.2}s | decoded"),
            None => println!("[MON  {t}] {duration:.2}s | undecoded"),
        },
        ChannelEvent::Delivered { .. } => {}
    }
}
