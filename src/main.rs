mod audio_cat_source;
mod console;
mod gui_main;
mod hpsdr_source;
mod local_controller;
mod null_source;
mod server_main;
mod tci_source;

use anyhow::{Context, bail};
use clap::Parser;
use sdroxide_config::Settings;
use sdroxide_radio::{FileSource, IqSource, SigGenSource};
#[cfg(feature = "soapy")]
use sdroxide_radio::{SoapyDevice, enumerate_devices};
use sdroxide_types::{Backend, DeviceCaps, RadioConfig};

/// sdroxide — SDR transceiver client.
///
/// M1 scope: `--probe` prints device capabilities, `--console` shows a live
/// terminal waterfall. The GUI arrives in milestone M2, server mode in M6.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// SoapySDR device args, e.g. "driver=hackrf" (default: config, then first device)
    #[arg(long)]
    device: Option<String>,

    /// List devices and their probed capabilities, then exit
    #[arg(long)]
    probe: bool,

    /// Terminal waterfall mode
    #[arg(long)]
    console: bool,

    /// Use the built-in signal generator instead of hardware
    #[arg(long)]
    siggen: bool,

    /// Play a raw interleaved CF32 IQ file instead of hardware
    #[arg(long)]
    file: Option<std::path::PathBuf>,

    /// Center frequency in Hz
    #[arg(long, default_value_t = 14_200_000.0)]
    freq: f64,

    /// Sample rate in Hz (default: from config)
    #[arg(long)]
    rate: Option<f64>,

    /// Overall RX gain in dB (default: hardware AGC or a moderate value)
    #[arg(long)]
    gain: Option<f64>,

    /// Initial mode (USB, LSB, CW, AM, SAM, NFM, WFM, DIGU, DIGL, DSB, SPEC)
    #[arg(long)]
    mode: Option<sdroxide_types::Mode>,

    /// Headless TX smoke test: key a tune carrier for SECS seconds at the
    /// configured (minimal) drive and gains, then exit
    #[arg(long, value_name = "SECS")]
    tx_tune: Option<f64>,

    /// Headless FT8 smoke test: call CQ (with a test callsign) at minimal
    /// power for ~SECS seconds, report whether a slot-aligned burst keyed
    #[arg(long, value_name = "SECS")]
    ft8_cq: Option<f64>,

    /// Run as a server: HTTP web client + WebSocket streaming backend
    #[arg(long)]
    server: bool,

    /// Connect as a native remote client to a running sdroxide server
    /// (e.g. "host:4950" or a full ws:// URL)
    #[arg(long, value_name = "HOST[:PORT]")]
    connect: Option<String>,

    /// Server port (default: from config)
    #[arg(long)]
    port: Option<u16>,

    /// Directory with the trunk-built web client (default: embedded assets
    /// if compiled with --features embed-web)
    #[arg(long)]
    web_root: Option<std::path::PathBuf>,

    /// Spectrum FFT size
    #[arg(long, default_value_t = 4096)]
    fft: usize,

    /// Console waterfall lines per second
    #[arg(long, default_value_t = 15)]
    fps: u32,

    /// Display floor in dBFS
    #[arg(long, default_value_t = -110.0, allow_negative_numbers = true)]
    db_floor: f32,

    /// Display ceiling in dBFS
    #[arg(long, default_value_t = -10.0, allow_negative_numbers = true)]
    db_ceil: f32,

    /// Console spectrum width in characters
    #[arg(long, default_value_t = 100)]
    width: usize,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cli = Cli::parse();
    let settings = Settings::load();

    if cli.probe {
        return probe(&cli, &settings);
    }
    if cli.console {
        let (source, _caps) = open_source(&cli, &settings)?;
        return console::run(
            source,
            console::Options {
                fft_size: cli.fft,
                fps: cli.fps.max(1),
                db_floor: cli.db_floor,
                db_ceil: cli.db_ceil,
                width: cli.width.clamp(16, 400),
            },
        );
    }

    if let Some(secs) = cli.tx_tune {
        let (source, caps) = open_source(&cli, &settings)?;
        return tx_tune_test(source, caps, &settings, secs.clamp(0.2, 10.0));
    }
    if let Some(secs) = cli.ft8_cq {
        let (source, caps) = open_source(&cli, &settings)?;
        return ft8_cq_test(source, caps, &settings, secs.clamp(16.0, 60.0));
    }
    if cli.server {
        let (source, caps) = open_source(&cli, &settings)?;
        let port = cli.port.unwrap_or(settings.server_port);
        return server_main::run(source, caps, &settings, cli.mode, port, cli.web_root.clone());
    }
    if let Some(target) = &cli.connect {
        let url = if target.contains("://") {
            target.clone()
        } else {
            format!("ws://{target}/ws")
        };
        return gui_main::run_remote(&url);
    }

    let (source, caps) = open_source(&cli, &settings)?;
    gui_main::run(source, caps, &settings, cli.mode)
}

/// Headless tune-carrier smoke test. Relies on the engine safety rails:
/// TX hardware gains at minimum, tune drive default 5%, ham-band lockout.
fn tx_tune_test(
    source: Box<dyn IqSource>,
    caps: sdroxide_types::DeviceCaps,
    settings: &Settings,
    secs: f64,
) -> anyhow::Result<()> {
    use sdroxide_types::{Command, RadioEvent};
    use std::time::Duration;

    let mut handles = sdroxide_radio::start_engine(
        source,
        caps,
        sdroxide_radio::EngineConfig {
            tx_ham_only: settings.tx_ham_only,
            ..Default::default()
        },
    );
    let engine_thread = handles.thread.take();
    std::thread::sleep(Duration::from_millis(400));
    handles.cmd_tx.send(Command::SetTune(true))?;
    std::thread::sleep(Duration::from_secs_f64(secs));
    handles.cmd_tx.send(Command::SetTune(false))?;
    std::thread::sleep(Duration::from_millis(400));

    let mut keyed = false;
    let mut failure = None;
    while let Ok(ev) = handles.event_rx.try_recv() {
        match ev {
            RadioEvent::State(s) => keyed |= s.tx.tune,
            RadioEvent::ConnectionLost(e) => failure = Some(e),
            _ => {}
        }
    }
    let outcome = match (keyed, failure) {
        (_, Some(e)) => Err(anyhow::anyhow!("TX test failed: {e}")),
        (false, None) => {
            Err(anyhow::anyhow!("TX was refused (safety rails or device limits) — see log"))
        }
        (true, None) => {
            println!("TX tune test OK: carrier keyed for {secs:.1} s and released.");
            Ok(())
        }
    };
    drop(handles);
    if let Some(t) = engine_thread {
        let _ = t.join();
    }
    outcome
}

/// Headless FT8 smoke test: configure a test callsign, enter FT8, call CQ,
/// and confirm the engine keys a slot-aligned burst. Minimal drive / min TX
/// gain (same emission level as `--tx-tune`).
fn ft8_cq_test(
    source: Box<dyn IqSource>,
    caps: sdroxide_types::DeviceCaps,
    settings: &Settings,
    secs: f64,
) -> anyhow::Result<()> {
    use sdroxide_types::{Command, DigiConfig, Mode, RadioEvent, RxId};
    use std::time::Duration;

    let mut handles = sdroxide_radio::start_engine(
        source,
        caps,
        sdroxide_radio::EngineConfig {
            tx_ham_only: settings.tx_ham_only,
            initial_mode: Some(Mode::Ft8),
            ..Default::default()
        },
    );
    let engine_thread = handles.thread.take();
    std::thread::sleep(Duration::from_millis(400));

    let cfg = DigiConfig { my_call: "AB1CD".into(), my_grid: "FN42".into(), ..Default::default() };
    handles.cmd_tx.send(Command::SetMode { rx: RxId::Main, mode: Mode::Ft8 })?;
    handles.cmd_tx.send(Command::SetDigiConfig(cfg))?;
    handles.cmd_tx.send(Command::DigiCallCq)?;

    let mut keyed = false;
    let mut failure = None;
    let deadline = std::time::Instant::now() + Duration::from_secs_f64(secs);
    while std::time::Instant::now() < deadline {
        while let Ok(ev) = handles.event_rx.try_recv() {
            match ev {
                RadioEvent::State(s) => keyed |= s.tx.ptt,
                RadioEvent::Ft8Status(s) if s.transmitting => keyed = true,
                RadioEvent::ConnectionLost(e) => failure = Some(e),
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    handles.cmd_tx.send(Command::DigiStopQso)?;
    handles.cmd_tx.send(Command::DigiAbortTx)?;
    std::thread::sleep(Duration::from_millis(300));

    let outcome = match (keyed, failure) {
        (_, Some(e)) => Err(anyhow::anyhow!("FT8 CQ test failed: {e}")),
        (false, None) => Err(anyhow::anyhow!(
            "no FT8 burst keyed in {secs:.0}s — check UTC clock / safety rails (see log)"
        )),
        (true, None) => {
            println!("FT8 CQ test OK: a slot-aligned burst keyed and released.");
            Ok(())
        }
    };
    drop(handles);
    if let Some(t) = engine_thread {
        let _ = t.join();
    }
    outcome
}

#[cfg(feature = "soapy")]
fn device_filter(cli: &Cli, settings: &Settings) -> String {
    cli.device.clone().unwrap_or_else(|| settings.device_args.clone())
}

#[cfg(feature = "soapy")]
fn probe(cli: &Cli, settings: &Settings) -> anyhow::Result<()> {
    let filter = device_filter(cli, settings);
    let devices = enumerate_devices(&filter).context("SoapySDR enumeration failed")?;
    if devices.is_empty() {
        println!("No SoapySDR devices found (filter: {:?}).", filter);
        return Ok(());
    }
    for (i, d) in devices.iter().enumerate() {
        println!("=== Device {}: {} [{}] ===", i, d.label, d.driver);
        match SoapyDevice::open(&d.args) {
            Ok(dev) => print_caps(dev.caps()),
            Err(e) => println!("  failed to open: {e}"),
        }
    }
    Ok(())
}

#[cfg(not(feature = "soapy"))]
fn probe(_cli: &Cli, _settings: &Settings) -> anyhow::Result<()> {
    bail!("this build has no SoapySDR support (built with --no-default-features)")
}

#[cfg(feature = "soapy")]
fn print_caps(caps: &sdroxide_types::DeviceCaps) {
    let fmt_mhz = |hz: f64| format!("{:.3} MHz", hz / 1e6);
    println!("  driver        : {}", caps.driver);
    println!("  label         : {}", caps.label);
    println!(
        "  channels      : {} RX, {} TX{}",
        caps.rx_channels,
        caps.tx_channels,
        if caps.tx_channels > 0 {
            if caps.full_duplex { " (full duplex)" } else { " (half duplex)" }
        } else {
            " (receive only)"
        }
    );
    for (name, ranges) in [("RX freq", &caps.freq_ranges_rx), ("TX freq", &caps.freq_ranges_tx)] {
        if !ranges.is_empty() {
            let list: Vec<String> = ranges
                .iter()
                .map(|&(lo, hi)| format!("{} – {}", fmt_mhz(lo), fmt_mhz(hi)))
                .collect();
            println!("  {:<13} : {}", name, list.join(", "));
        }
    }
    if !caps.sample_rates.is_empty() {
        let list: Vec<String> = caps.sample_rates.iter().map(|r| format!("{:.3}", r / 1e6)).collect();
        println!("  rates (Msps)  : {}", list.join(", "));
    }
    for &(lo, hi) in &caps.rate_ranges {
        println!("  rate range    : {:.3} – {:.3} Msps", lo / 1e6, hi / 1e6);
    }
    for g in &caps.gains {
        println!(
            "  gain {:<8} : {:?} {} to {} dB (step {})",
            g.name, g.direction, g.min_db, g.max_db, g.step_db
        );
    }
    if !caps.antennas_rx.is_empty() {
        println!("  RX antennas   : {}", caps.antennas_rx.join(", "));
    }
    if !caps.antennas_tx.is_empty() {
        println!("  TX antennas   : {}", caps.antennas_tx.join(", "));
    }
    if !caps.sensors.is_empty() {
        println!("  sensors       : {}", caps.sensors.join(", "));
    }
}

fn open_source(cli: &Cli, settings: &Settings) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let rate = cli.rate.unwrap_or(settings.sample_rate);

    if cli.siggen {
        return Ok((
            Box::new(SigGenSource::demo(rate, cli.freq)),
            synthetic_caps("Signal generator"),
        ));
    }
    if let Some(path) = &cli.file {
        let label = format!("IQ file {}", path.display());
        return Ok((
            Box::new(
                FileSource::open(path, rate, cli.freq)
                    .with_context(|| format!("opening IQ file {}", path.display()))?,
            ),
            synthetic_caps(&label),
        ));
    }

    // Try the configured radio interface. If it can't be opened (no SoapySDR
    // device, HPSDR unreachable, CAT port missing, …) fall back to a null source
    // so the GUI — and the Settings dialog — still come up and the user can pick
    // a working interface and restart, instead of the program refusing to launch.
    let radio = sdroxide_config::load_radio_config();
    match open_configured_source(&radio, cli, settings) {
        Ok(pair) => Ok(pair),
        Err(e) => {
            tracing::warn!("radio interface unavailable: {e:#}");
            let msg =
                format!("{e}. Open Settings to choose a radio interface, then restart.");
            Ok((Box::new(null_source::NullSource::new(cli.freq, msg)), synthetic_caps("No radio")))
        }
    }
}

/// Open the interface selected in `radio.json`. `Auto` prefers a SoapySDR device
/// and falls back to CAT when none is present (or the binary has no soapy).
fn open_configured_source(
    radio: &RadioConfig,
    cli: &Cli,
    settings: &Settings,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    match radio.backend {
        Backend::Cat => open_cat_source(radio),
        Backend::Hpsdr => open_hpsdr_source(radio, cli.freq),
        Backend::Tci => open_tci_source(radio, cli.freq),
        Backend::Soapy => open_soapy_source(cli, settings),
        Backend::Auto => {
            #[cfg(feature = "soapy")]
            {
                let filter = device_filter(cli, settings);
                if enumerate_devices(&filter).map(|d| d.is_empty()).unwrap_or(true) {
                    open_cat_source(radio)
                } else {
                    open_soapy_source(cli, settings)
                }
            }
            #[cfg(not(feature = "soapy"))]
            {
                open_cat_source(radio)
            }
        }
    }
}

/// Open the first available SoapySDR device (feature-gated).
#[cfg(feature = "soapy")]
fn open_soapy_source(
    cli: &Cli,
    settings: &Settings,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let rate = cli.rate.unwrap_or(settings.sample_rate);
    let filter = device_filter(cli, settings);
    let devices = enumerate_devices(&filter).context("SoapySDR enumeration failed")?;
    let Some(info) = devices.first() else {
        bail!("no SoapySDR devices found (filter: {:?})", filter);
    };
    let dev =
        SoapyDevice::open(&info.args).with_context(|| format!("opening device {}", info.label))?;
    let caps = dev.caps().clone();
    Ok((Box::new(dev.rx_source(rate, cli.freq, cli.gain)?), caps))
}

#[cfg(not(feature = "soapy"))]
fn open_soapy_source(
    _cli: &Cli,
    _settings: &Settings,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    bail!("SoapySDR support is not compiled into this build")
}

/// Build the CAT + sound-card source and its capabilities from radio.json.
fn open_cat_source(radio: &RadioConfig) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let src = audio_cat_source::AudioCatSource::open(
        radio.cat.clone(),
        radio.radio_audio_in.as_deref(),
        radio.radio_audio_out.as_deref(),
    )
    .context("opening CAT rig")?;
    let caps = cat_caps(radio);
    Ok((Box::new(src), caps))
}

/// Build the HPSDR (ethernet SDR, Protocol 2) source from radio.json. The target
/// IP is the manual override, else the persisted selection, else the first
/// Protocol-2 device found by a discovery scan.
fn open_hpsdr_source(
    radio: &RadioConfig,
    center_hz: f64,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let ip: std::net::Ipv4Addr = if let Some(s) = radio.hpsdr.target_ip() {
        s.trim().parse().with_context(|| format!("invalid HPSDR IP address {s:?}"))?
    } else {
        let found = sdroxide_hpsdr::discover_default();
        let dev = found.iter().find(|d| d.supported()).ok_or_else(|| {
            anyhow::anyhow!(
                "no HPSDR (Protocol 2) device found on the network — enter a target IP in Settings"
            )
        })?;
        dev.ip.parse().with_context(|| format!("discovered HPSDR IP {:?}", dev.ip))?
    };

    let src = hpsdr_source::HpsdrSource::open(ip, radio.hpsdr.sample_rate_hz, center_hz)
        .context("opening HPSDR device")?;
    let caps = hpsdr_caps(src.board(), src.sample_rate_hz(), src.protocol());
    Ok((Box::new(src), caps))
}

/// Capabilities for an HPSDR board: wideband IQ (not `audio_mode`), TX-capable,
/// half-duplex, HF+6m coverage. The board enforces its own limits. Protocol 1
/// boards top out at 384 kHz.
fn hpsdr_caps(board: &str, sample_rate: f64, protocol: u8) -> DeviceCaps {
    DeviceCaps {
        driver: "hpsdr".into(),
        label: format!("{board} (HPSDR P{protocol}, {:.3} Msps)", sample_rate / 1e6),
        rx_channels: 1,
        tx_channels: 1,
        audio_mode: false,
        freq_ranges_rx: vec![(0.0, 61_440_000.0)],
        freq_ranges_tx: vec![(1_800_000.0, 54_000_000.0)],
        sample_rates: sdroxide_types::HpsdrConfig::rates_for(protocol).to_vec(),
        ..DeviceCaps::default()
    }
}

/// Build the TCI (WebSocket) source from radio.json: wideband IQ receive +
/// audio transmit.
fn open_tci_source(
    radio: &RadioConfig,
    center_hz: f64,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let src =
        tci_source::TciSource::open(&radio.tci.address, radio.tci.iq_sample_rate_hz, center_hz)
            .context("connecting to TCI server")?;
    let caps = tci_caps(&radio.tci.address, src.sample_rate_hz());
    Ok((Box::new(src), caps))
}

/// Capabilities for a TCI rig: wideband IQ RX (not `audio_mode`), TX via raw
/// audio (`tx_audio`) which the rig modulates. The rig enforces its own limits.
fn tci_caps(address: &str, iq_rate: f64) -> DeviceCaps {
    DeviceCaps {
        driver: "tci".into(),
        label: format!("TCI {address} ({:.0} kHz IQ)", iq_rate / 1000.0),
        rx_channels: 1,
        tx_channels: 1,
        audio_mode: false,
        tx_audio: true,
        freq_ranges_rx: vec![(0.0, 160_000_000.0)],
        freq_ranges_tx: vec![(1_800_000.0, 54_000_000.0)],
        sample_rates: sdroxide_types::TciConfig::IQ_RATES.to_vec(),
        // No RX gains: the SunSDR2DX ATT/Preamp is not reachable over TCI
        // (verified against ExpertSDR3 — no command spelling drives it, and
        // toggling it in the GUI emits nothing on the wire). TCI gain control
        // is deferred until a controllable path is found.
        ..DeviceCaps::default()
    }
}

/// Capabilities for a CAT rig. TX-capable unless PTT is VOX-only-with-no-audio;
/// we advertise TX so the UI shows PTT and the safety rails apply. Frequency
/// range covers HF+6m (the rig enforces its own limits over CAT).
fn cat_caps(radio: &RadioConfig) -> DeviceCaps {
    let demod = matches!(radio.cat.format, sdroxide_types::SoundFormat::DemodAudio);
    DeviceCaps {
        driver: "cat".into(),
        label: format!("{} (CAT)", radio.cat.family.label()),
        rx_channels: 1,
        tx_channels: 1,
        audio_mode: demod,
        freq_ranges_rx: vec![(100_000.0, 148_000_000.0)],
        freq_ranges_tx: vec![(1_800_000.0, 54_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// Capabilities for non-hardware sources (RX-only, unlimited tuning).
fn synthetic_caps(label: &str) -> DeviceCaps {
    DeviceCaps {
        driver: "none".into(),
        label: label.into(),
        rx_channels: 1,
        tx_channels: 0,
        freq_ranges_rx: vec![(0.0, 6e9)],
        ..DeviceCaps::default()
    }
}
