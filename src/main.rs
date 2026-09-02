mod airspy_source;
mod airspyhf_source;
mod audio_cat_source;
mod console;
mod device_registry;
mod devices;
mod dial;
mod elad_source;
mod fobos_source;
mod gui_main;
mod hackrf_source;
mod hpsdr_source;
mod hydrasdr_source;
mod icomnet_source;
mod kiwisdr_source;
mod lime_source;
mod local_controller;
mod null_source;
mod panadapter_source;
mod pluto_source;
mod rtlsdr_source;
mod rx888_source;
mod sdrplay_source;
mod server_main;
mod session_trace;
mod smartsdr_source;
mod spyserver_source;
mod tci_source;

use anyhow::{Context, bail};
use clap::Parser;
use sdroxide_config::Settings;
use sdroxide_radio::{ConvertedSource, FileSource, IqSource, SigGenSource, override_caps_ranges};
#[cfg(feature = "soapy")]
use sdroxide_radio::{DeviceInfo, SoapyDevice, enumerate_devices};
use sdroxide_types::{Backend, DeviceCaps, FobosPort, IcomNetConfig, RadioConfig};

#[derive(Parser, Debug, Clone)]
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

    /// Record every raw IQ sample to a file, in the same interleaved CF32 format
    /// --file reads back.
    ///
    /// For capturing a band to work on offline — what a decoder does with a real
    /// signal is not a question a synthetic one can answer. Large: 8 bytes a
    /// sample, so 16 MB a second at 2 Msps.
    #[arg(long, value_name = "PATH")]
    record_iq: Option<std::path::PathBuf>,

    /// Center frequency in Hz (default: where the last session was left)
    #[arg(long)]
    freq: Option<f64>,

    /// Sample rate in Hz (default: from config)
    #[arg(long)]
    rate: Option<f64>,

    /// Overall RX gain in dB (default: hardware AGC or a moderate value)
    #[arg(long)]
    gain: Option<f64>,

    /// Initial mode (USB, LSB, CW, AM, SAM, NFM, WFM, DIGU, DIGL, DSB, ISB, SPEC, FT8,
    /// FT4, FT2, PSK, RTTY, PACKET, APRS, ADS-B, VDL2, AIS, SSTV, RIFP, OLIVIA, THOR, FSQ)
    ///
    /// Default: the mode the last session was left in.
    #[arg(long)]
    mode: Option<sdroxide_types::Mode>,

    /// RX antenna port, as the device names it ("LNAH", "TX/RX"). Run --probe
    /// to list what the front end offers.
    ///
    /// Default: the port the last session was left on, and failing that
    /// whatever the driver selects when it opens the device. Worth setting on a
    /// headless server, where nobody is at the machine to pick one.
    #[arg(long, value_name = "NAME")]
    antenna: Option<String>,

    /// TX antenna port, likewise ("BAND1", "BAND2")
    #[arg(long, value_name = "NAME")]
    tx_antenna: Option<String>,

    /// Headless TX smoke test: key a tune carrier for SECS seconds at the
    /// configured (minimal) drive and gains, then exit
    #[arg(long, value_name = "SECS")]
    tx_tune: Option<f64>,

    /// Headless FT8 smoke test: call CQ (with a test callsign) at minimal
    /// power for ~SECS seconds, report whether a slot-aligned burst keyed
    #[arg(long, value_name = "SECS")]
    ft8_cq: Option<f64>,

    /// Headless RADE smoke test: run the digital-voice receiver for ~SECS
    /// seconds (pair with --file) and report whether the modem reached sync
    #[arg(long, value_name = "SECS")]
    rade_rx: Option<f64>,

    /// Connect to FreeDV Reporter read-only for ~SECS seconds and print what
    /// arrives. Uses the server's "view" role: nothing is reported and this
    /// station does not appear on qso.freedv.org. Needs no radio.
    #[arg(long, value_name = "SECS")]
    freedv_reporter_probe: Option<f64>,

    /// FreeDV Reporter host for --freedv-reporter-probe
    #[arg(long, value_name = "HOST[:PORT]", default_value = "qso.freedv.org")]
    freedv_reporter_host: String,

    /// Run as a server: HTTP web client + WebSocket streaming backend
    #[arg(long)]
    server: bool,

    /// Connect as a native remote client to a running sdroxide server
    /// (e.g. "host:4950" or a full ws:// URL)
    ///
    /// A bare address reaches the station's first radio. To reach one of its
    /// others, name it: "host:4950/ws/1" — the server lists which id is which
    /// at http://host:4950/radios.
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

    /// Whether `--freq` was given, kept across [`Cli::apply_session`] filling it
    /// in from the remembered session — see there.
    #[arg(skip)]
    freq_named: bool,

    /// Allow transmit on any frequency the hardware supports, not just the
    /// amateur bands
    ///
    /// Overrides `tx_ham_only` in config.toml for this run only, and puts a
    /// warning on screen that has to be dismissed by hand. For licensed
    /// out-of-band use — MARS/CAP, a commercial or experimental licence, a
    /// dummy load — where transmitting outside the amateur allocations is
    /// something you are authorised to do. Everywhere else it is an offence,
    /// and the band edges are the last thing standing between a mistyped
    /// frequency and an interference complaint.
    #[arg(long)]
    oob_tx: bool,
}

impl Cli {
    /// The dial to open the front end on. Always resolved by
    /// [`Cli::restore_session`] before anything reads it; the fallback is only
    /// so this can never panic on a code path that skipped that.
    fn center_hz(&self) -> f64 {
        self.freq.unwrap_or_else(|| sdroxide_config::Session::default().freq_hz)
    }

    /// Fill in the frequency and mode the operator did not ask for from where
    /// the last session was left, so the program comes back up where it was.
    /// Returns the mode the engine should start in.
    fn restore_session(&mut self) -> Option<sdroxide_types::Mode> {
        self.apply_session(sdroxide_config::load_session())
    }

    /// The command line always wins; whatever it left out comes from the
    /// remembered session.
    ///
    /// The dial taken from the session is the one the operator was working on,
    /// which is not always VFO A. Only that one reaches the front end — the
    /// other VFO never had a centre frequency to ask for — and the engine
    /// restores the pair once it has read the same session itself.
    fn apply_session(&mut self, session: sdroxide_config::Session) -> Option<sdroxide_types::Mode> {
        // Remembered before the session fills it in, because after that
        // `freq.is_some()` is true whether the operator typed one or not — and
        // an I/Q capture's own centre frequency has to lose to the command line
        // and win against the last session (issue #217).
        self.freq_named = self.freq.is_some();
        self.freq = Some(self.freq.unwrap_or_else(|| session.active_dial_hz()));
        self.mode = Some(self.mode.unwrap_or(session.mode));
        self.mode
    }

    /// The antenna ports named on the command line, RX then TX.
    ///
    /// Not merged with the session here, the way the dial and mode are: the
    /// front end has to be open before a port name means anything, so the engine
    /// does that merge once it can check the names against what the device
    /// actually offers.
    fn initial_antenna(&self) -> (Option<String>, Option<String>) {
        (self.antenna.clone(), self.tx_antenna.clone())
    }

    /// Whether the engine should refuse to key outside the amateur bands.
    ///
    /// The flag can only ever *loosen* the config, never tighten it: a build
    /// without it behaves exactly as before.
    fn tx_ham_only(&self, settings: &Settings) -> bool {
        settings.tx_ham_only && !self.oob_tx
    }
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let mut cli = Cli::parse();
    let settings = Settings::load();
    // Before anything reads a band edge — the console panadapter, the headless
    // smoke tests and the GUI all do. The engine sets these again from the same
    // files when it starts; doing it here as well means even the paths that
    // never build an engine are on the station's band plan. `load_band_plan`
    // also seeds `bandplan.json` from the built-in tables on a first run.
    //
    // Skipped when connecting to somebody else's station: the band plan that
    // matters is theirs, it arrives in the first `StationConfig`, and seeding a
    // file here would leave a document in this machine's config directory that
    // looks authoritative and changes nothing.
    if cli.connect.is_none() {
        sdroxide_types::set_band_plan(sdroxide_config::load_band_plan());
        sdroxide_types::set_region(settings.region);
    }

    if cli.probe {
        return probe(&cli, &settings);
    }
    if cli.console {
        cli.restore_session();
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

    // The headless smoke tests below deliberately do *not* restore the
    // remembered dial. They are diagnostics — one of them keys a carrier — and
    // they have to run somewhere predictable rather than wherever the last
    // session happened to end. `--freq` still moves them.
    if let Some(secs) = cli.tx_tune {
        let (source, caps) = open_source(&cli, &settings)?;
        return tx_tune_test(source, caps, cli.tx_ham_only(&settings), secs.clamp(0.2, 10.0));
    }
    if let Some(secs) = cli.ft8_cq {
        let (source, caps) = open_source(&cli, &settings)?;
        return ft8_cq_test(source, caps, cli.tx_ham_only(&settings), secs.clamp(16.0, 60.0));
    }
    if let Some(secs) = cli.rade_rx {
        let (source, caps) = open_source(&cli, &settings)?;
        return rade_rx_test(source, caps, cli.tx_ham_only(&settings), secs.clamp(2.0, 120.0));
    }
    // Before any radio setup: the probe talks to the network and nothing else.
    if let Some(secs) = cli.freedv_reporter_probe {
        return freedv_reporter_probe(&cli.freedv_reporter_host, secs.clamp(2.0, 300.0));
    }
    if cli.server {
        // The same roster the GUI brings up, for the same reason: a station
        // with two radios has two radios whether or not anybody is sitting at
        // it. Each is served under its own address — see `sdroxide-server`.
        let radios = boot_radios(&mut cli, &settings)?;
        let port = cli.port.unwrap_or(settings.server_port);
        // Sanitized for the radios a client adds later, exactly as the GUI
        // does for its "+" chip: radio 0's overrides are radio 0's.
        let factory_cli = secondary_cli(&cli);
        return server_main::run(
            radios,
            &settings,
            cli.tx_ham_only(&settings),
            port,
            cli.web_root.clone(),
            factory_cli,
        );
    }
    // A remote client drives somebody else's engine; that engine is the one
    // that remembers where its radio was left.
    if let Some(target) = &cli.connect {
        // A bare address means the station's first radio. One that already
        // carries a path is left alone — that is how a client asks for one of
        // the station's *other* radios ("host:4950/ws/1"), and appending our
        // own `/ws` to it would break exactly the case it was typed for.
        let url = match target {
            t if t.contains("://") => t.clone(),
            t if t.contains('/') => format!("ws://{t}"),
            t => format!("ws://{t}/ws"),
        };
        return gui_main::run_remote(&url);
    }

    let radios = boot_radios(&mut cli, &settings)?;
    let factory_cli = secondary_cli(&cli);
    gui_main::run_multi(radios, &settings, cli.tx_ham_only(&settings), factory_cli)
}

/// Everything `main` resolved for one radio before it is put on screen or on
/// the air: its (possibly stand-in) front end and where its configuration
/// lives.
pub struct RadioBoot {
    pub id: u32,
    pub name: String,
    /// Whether the operator has this radio switched on — the `enabled` field of
    /// its entry in the roster ([`sdroxide_config::RadioSlot`]). A radio that is off boots on the quiet stand-in, and the
    /// tab shell needs to know which state its switch is in.
    pub enabled: bool,
    pub source: Box<dyn IqSource>,
    pub caps: DeviceCaps,
    pub initial_mode: Option<sdroxide_types::Mode>,
    /// Antenna ports named on the command line (RX, TX) — radio 0 only; the
    /// remembered session fills in whichever the operator left out.
    pub initial_antenna: (Option<String>, Option<String>),
    pub reopen: Option<sdroxide_radio::ReopenFn>,
    pub store: sdroxide_config::Store,
    /// Where to write a raw IQ capture, from `--record-iq`. Radio 0 only: the
    /// flag names one file, and two radios writing it would interleave two
    /// different bands into one stream.
    pub record_iq: Option<std::path::PathBuf>,
}

/// Open the station's radio roster: the legacy single radio unless more have
/// been added. Radio 0 keeps every command-line override and the legacy config
/// paths; the others boot purely from their own scope, and a device that isn't
/// there right now costs a "reconnecting" radio, not the launch.
///
/// Shared by the GUI and the server, which differ in what they do with the
/// radios, not in which ones the station has.
fn boot_radios(cli: &mut Cli, settings: &Settings) -> anyhow::Result<Vec<RadioBoot>> {
    let roster = sdroxide_config::load_radios();
    let owners = panadapter_owners();
    let mut radios = Vec::new();
    for (i, slot) in roster.radios.iter().enumerate() {
        let store = sdroxide_config::Store::radio(slot.id);
        // Somebody else's receiver: its device belongs to the radio that
        // borrowed it, and opening it twice would take it away from them. It
        // still gets an engine and a scope of its own, so its interface stays
        // configurable and detaching it puts it straight back on the air.
        if let Some(&owner) = owners.get(&slot.id) {
            let mut c = secondary_cli(cli);
            let initial_mode = c.apply_session(store.load_session());
            tracing::info!("radio {} is radio {}'s panadapter receiver", slot.id + 1, owner + 1);
            radios.push(RadioBoot {
                id: slot.id,
                name: slot.name.clone(),
                enabled: slot.enabled,
                source: Box::new(null_source::NullSource::attached(c.center_hz(), owner)),
                // Says which radio has it, so the shell can keep this one off
                // the tab strip and name the borrower.
                caps: DeviceCaps { lent_to: Some(owner), ..synthetic_caps("Panadapter receiver") },
                initial_mode,
                initial_antenna: (None, None),
                // A real factory, which refuses while the pairing stands: the
                // engine's retry is then all it takes for this radio to come
                // back the moment the borrower lets go of its device.
                reopen: Some(reopen_factory_for(&c, store.clone(), slot.id)),
                store,
                record_iq: None,
            });
            continue;
        }
        // Switched off: the roster says so, so nothing of this radio's
        // interface is opened — while its scope, its engine and its tab are all
        // still here, which is what keeps it configurable and one button away
        // from coming back. The command line's own front ends (`--siggen`,
        // `--file`) still win, exactly as they do over a panadapter pairing:
        // they are this session's explicit instruction, and they only ever
        // apply to radio 0.
        let overridden = i == 0 && (cli.siggen || cli.file.is_some());
        if !slot.enabled && !overridden {
            // The remembered dial is restored either way: switched off is a
            // radio put down where it was, not one wound back to nothing.
            let (c, initial_mode) = if i == 0 {
                let mode = cli.restore_session();
                (cli.clone(), mode)
            } else {
                let mut c = secondary_cli(cli);
                let mode = c.apply_session(store.load_session());
                (c, mode)
            };
            tracing::info!("radio {} is switched off", slot.id + 1);
            radios.push(RadioBoot {
                id: slot.id,
                name: slot.name.clone(),
                enabled: false,
                source: Box::new(null_source::NullSource::off(c.center_hz())),
                caps: synthetic_caps("Switched off"),
                initial_mode,
                initial_antenna: (None, None),
                reopen: Some(reopen_factory_for(&c, store.clone(), slot.id)),
                store,
                record_iq: None,
            });
            continue;
        }
        let boot = if i == 0 {
            let initial_mode = cli.restore_session();
            let (source, caps) = open_source(cli, settings)?;
            RadioBoot {
                id: slot.id,
                name: slot.name.clone(),
                enabled: slot.enabled,
                source,
                caps,
                initial_mode,
                initial_antenna: cli.initial_antenna(),
                reopen: Some(reopen_factory(cli)),
                store,
                record_iq: cli.record_iq.clone(),
            }
        } else {
            let mut c = secondary_cli(cli);
            let session = store.load_session();
            let initial_mode = c.apply_session(session);
            let (source, caps) =
                match open_converted_source(&store.load_radio_config(), &c, settings) {
                    Ok(pair) => pair,
                    Err(e) => {
                        // Names are often empty now (the tab derives one from
                        // the interface); the id is what the engine's own
                        // messages use, so log that.
                        tracing::warn!("radio {} unavailable: {e:#}", slot.id + 1);
                        let msg = format!(
                            "{e}. Retrying — or open Settings → Radio to choose another interface."
                        );
                        (
                            Box::new(null_source::NullSource::new(c.center_hz(), msg))
                                as Box<dyn IqSource>,
                            synthetic_caps("No radio"),
                        )
                    }
                };
            RadioBoot {
                id: slot.id,
                name: slot.name.clone(),
                enabled: slot.enabled,
                source,
                caps,
                initial_mode,
                initial_antenna: (None, None),
                reopen: Some(reopen_factory_for(&c, store.clone(), slot.id)),
                store,
                record_iq: None,
            }
        };
        radios.push(boot);
    }
    Ok(radios)
}

/// Factory the engine calls to rebuild the interface at runtime: when the
/// operator switches interface (Settings → Radio → Apply), so that never needs a
/// restart, and when the engine reconnects an interface that wasn't there yet.
/// Re-reads the persisted radio config + settings each call and opens at the
/// current dial. Fallible so a bad new config leaves the current interface
/// running.
fn reopen_factory(cli: &Cli) -> sdroxide_radio::ReopenFn {
    reopen_factory_for(cli, sdroxide_config::Store::station(), 0)
}

/// [`reopen_factory`], reading `radio.json` from one radio's scope. The
/// `--siggen`/`--file` overrides only ever apply to the station scope (radio
/// 0); every caller building a factory for another radio passes a
/// [`secondary_cli`], which has them stripped.
///
/// `id` is the roster id the scope belongs to, which the factory needs for two
/// questions it cannot ask a `Store`. The first is whether this radio has been
/// lent to another as its panadapter receiver. While it has, the factory
/// refuses — the device is open on the borrower's engine, and opening it here
/// would take it away from them. Refusing rather than handing back a stand-in
/// is what makes undoing the pairing enough on its own: the engine's own retry
/// keeps asking, and the first attempt after the borrower lets go succeeds.
///
/// The second is whether the operator has switched this radio off. Then the
/// factory hands back the quiet stand-in instead — an *answer*, not a refusal,
/// because a refusal leaves whatever is running running, and the whole point of
/// switching a radio off is that its device is let go.
fn reopen_factory_for(
    cli: &Cli,
    store: sdroxide_config::Store,
    id: u32,
) -> sdroxide_radio::ReopenFn {
    let cli = cli.clone();
    // Whether the last thing this factory handed back was the switched-off
    // stand-in — which, alone among the stand-ins, has stopped asking to be
    // reopened. So the attempt that switches the radio back *on* is the one
    // attempt whose failure may not be reported as a plain refusal: that would
    // leave the engine holding a front end that will never retry, and a radio
    // switched on that stays dark until somebody presses something again. On
    // that one path a device that isn't there yet becomes the ordinary
    // reconnecting stand-in instead, and the engine takes it from there.
    let mut was_off = !sdroxide_config::load_radios().is_enabled(id);
    Box::new(move |center: f64| {
        let mut c = cli.clone();
        c.freq = Some(center);
        let settings = Settings::load();
        if c.siggen || c.file.is_some() {
            was_off = false;
            return open_source(&c, &settings).map_err(|e| format!("{e:#}"));
        }
        if let Some(owner) = panadapter_owners().get(&id) {
            return Err(format!("this radio is radio {}'s panadapter receiver", owner + 1));
        }
        if !sdroxide_config::load_radios().is_enabled(id) {
            was_off = true;
            return Ok((
                Box::new(null_source::NullSource::off(c.center_hz())) as Box<dyn IqSource>,
                synthetic_caps("Switched off"),
            ));
        }
        let radio = store.load_radio_config();
        let opened = open_converted_source(&radio, &c, &settings).map_err(|e| format!("{e:#}"));
        let coming_back = std::mem::take(&mut was_off);
        match opened {
            Err(e) if coming_back => Ok((
                Box::new(null_source::NullSource::new(
                    c.center_hz(),
                    format!("{e} Retrying — or open Settings → Radio to choose another interface."),
                )) as Box<dyn IqSource>,
                synthetic_caps("No radio"),
            )),
            other => other,
        }
    })
}

/// The command line, minus everything that belongs to radio 0 alone: the
/// synthetic sources (`--siggen`, `--file`) and the tuning/antenna/device
/// overrides. A secondary radio boots purely from its own scope — the flags
/// were typed for the radio the operator has always had.
fn secondary_cli(cli: &Cli) -> Cli {
    let mut c = cli.clone();
    c.siggen = false;
    c.file = None;
    c.freq = None;
    c.mode = None;
    c.rate = None;
    c.gain = None;
    c.antenna = None;
    c.tx_antenna = None;
    c
}

/// Headless tune-carrier smoke test. Relies on the engine safety rails:
/// TX hardware gains at minimum, tune drive default 5%, ham-band lockout.
fn tx_tune_test(
    source: Box<dyn IqSource>,
    caps: sdroxide_types::DeviceCaps,
    tx_ham_only: bool,
    secs: f64,
) -> anyhow::Result<()> {
    use sdroxide_types::{Command, RadioEvent};
    use std::time::Duration;

    let mut handles = sdroxide_radio::start_engine(
        source,
        caps,
        sdroxide_radio::EngineConfig { tx_ham_only, ..Default::default() },
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
    tx_ham_only: bool,
    secs: f64,
) -> anyhow::Result<()> {
    use sdroxide_types::{Command, DigiConfig, Mode, RadioEvent, RxId};
    use std::time::Duration;

    let mut handles = sdroxide_radio::start_engine(
        source,
        caps,
        sdroxide_radio::EngineConfig {
            tx_ham_only,
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

/// Read-only FreeDV Reporter check: connect, listen, and print what the server
/// sends.
///
/// This uses the reporter's `"view"` role, which receives every broadcast but
/// never joins the public roster — so it is safe to point at qso.freedv.org.
/// Going *visible* requires an operator enabling the feature in Settings with a
/// real callsign; there is deliberately no flag for it here.
fn freedv_reporter_probe(host_arg: &str, secs: f64) -> anyhow::Result<()> {
    let (host, port) = match host_arg.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().unwrap_or(80)),
        None => (host_arg, 80),
    };
    println!("FreeDV Reporter probe (view role — not visible to others): {host}:{port}");
    println!("listening for {secs:.0} s…\n");

    let s = sdroxide_net::freedv_reporter_probe(host, port, secs);

    if s.event_counts.is_empty() {
        println!("events: (none)");
    } else {
        let counts: Vec<String> =
            s.event_counts.iter().map(|(name, n)| format!("{name} {n}")).collect();
        println!("events: {}", counts.join("  "));
    }
    let with_freq = s.stations.iter().filter(|st| st.freq_hz > 0).count();
    println!("stations: {} ({with_freq} with a frequency)\n", s.stations.len());
    for st in &s.stations {
        println!(
            "  {:<10} {:<8} {:>10.3} kHz  {:<8} {:<3} {}",
            st.call,
            st.grid,
            st.freq_hz as f64 / 1000.0,
            st.mode,
            if st.tx {
                "TX"
            } else if st.rx_only {
                "RX"
            } else {
                ""
            },
            st.version,
        );
    }
    println!(
        "\nhandshake: open={} connect_ack={} connection_successful={}",
        s.got_open, s.got_connect_ack, s.got_connection_successful
    );
    if let Some(status) = &s.last_status {
        println!("status: {status}");
    }
    if s.ok() {
        println!("\nPASS");
        Ok(())
    } else {
        bail!("FreeDV Reporter probe did not complete a session with at least one station")
    }
}

/// Headless RADE receive check: bring the engine up in RADE mode and report
/// what the modem made of whatever the source is feeding it.
///
/// Pair with `--file` and an IQ file from `rade-harness iq` for a repeatable
/// end-to-end run with no radio and no sound card.
fn rade_rx_test(
    source: Box<dyn IqSource>,
    caps: sdroxide_types::DeviceCaps,
    tx_ham_only: bool,
    secs: f64,
) -> anyhow::Result<()> {
    use sdroxide_types::{Command, Mode, RadioEvent, RxId};
    use std::time::Duration;

    // The receive chain — and with it the decoder's audio tap — only exists
    // when the engine has somewhere to play audio. Give it a ring buffer we
    // drain and throw away, so the test needs no sound card.
    let (producer, mut sink) = rtrb::RingBuffer::<f32>::new(96_000);
    let mut handles = sdroxide_radio::start_engine(
        source,
        caps,
        sdroxide_radio::EngineConfig {
            tx_ham_only,
            initial_mode: Some(Mode::Rade),
            audio: Some(sdroxide_radio::AudioParams { producer, out_rate: 48_000.0 }),
            ..Default::default()
        },
    );
    let engine_thread = handles.thread.take();
    handles.cmd_tx.send(Command::SetMode { rx: RxId::Main, mode: Mode::Rade })?;

    let (mut synced, mut best_snr, mut eoo, mut dropped) = (false, f32::MIN, 0u64, 0u64);
    let mut failure = None;
    let deadline = std::time::Instant::now() + Duration::from_secs_f64(secs);
    while std::time::Instant::now() < deadline {
        while let Ok(ev) = handles.event_rx.try_recv() {
            match ev {
                RadioEvent::Ft8Status(s) => {
                    if let Some(r) = s.rade {
                        if r.sync {
                            synced = true;
                            best_snr = best_snr.max(r.snr_db);
                        }
                        eoo = eoo.max(r.eoo_count);
                        dropped = dropped.max(r.dropped);
                    }
                }
                RadioEvent::ConnectionLost(e) => failure = Some(e),
                _ => {}
            }
        }
        while sink.pop().is_ok() {}
        std::thread::sleep(Duration::from_millis(50));
    }

    let outcome = match (synced, failure) {
        (_, Some(e)) => Err(anyhow::anyhow!("RADE test failed: {e}")),
        (false, None) => Err(anyhow::anyhow!(
            "RADE never reached sync in {secs:.0}s — is the source carrying a RADE signal?"
        )),
        (true, None) => {
            println!(
                "RADE RX test OK: sync reached, best SNR {best_snr:.1} dB, \
                 {eoo} end-of-over frame(s), {dropped} samples dropped."
            );
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

fn probe(cli: &Cli, settings: &Settings) -> anyhow::Result<()> {
    // RTL-SDR first, and in every build: the native driver needs no system
    // library, so this half of `--probe` works even in the non-SoapySDR
    // variant. It is the field-diagnosis tool for "does this machine see my
    // dongle, and may this user have it?".
    probe_rtlsdr();
    probe_rx888();
    probe_airspyhf();
    probe_airspy();
    probe_hydrasdr();
    probe_hackrf();
    probe_sdrplay();
    probe_elad();
    probe_lime();
    probe_soapy(cli, settings)
}

fn probe_rtlsdr() {
    let devices = sdroxide_rtlsdr::list();
    if devices.is_empty() {
        println!("No RTL-SDR dongles found on USB.");
    } else {
        println!("=== RTL-SDR (native USB driver) ===");
        for (i, d) in devices.iter().enumerate() {
            println!("  {}: {}  [usb {:04x}:{:04x}]", i, d.label(), d.vid, d.pid);
        }
    }
    println!();
}

fn probe_rx888() {
    let devices = sdroxide_rx888::list();
    if devices.is_empty() {
        println!("No RX-888 receivers found on USB.");
    } else {
        println!("=== RX-888 (native USB driver) ===");
        for (i, d) in devices.iter().enumerate() {
            // A receiver in its boot ROM always reports USB 2.0, so saying
            // "not SuperSpeed" there would be alarming and wrong.
            let link = if d.needs_firmware {
                "link speed unknown until programmed"
            } else if d.superspeed {
                "SuperSpeed"
            } else {
                "USB 2.0 — use a USB 3 cable and port for the full rate"
            };
            println!("  {}: {}  [{}]", i, d.label(), link);
        }
    }
    println!();
}

fn probe_airspyhf() {
    let devices = sdroxide_airspyhf::list();
    if devices.is_empty() {
        println!("No Airspy HF+ receivers found on USB.");
    } else {
        println!("=== Airspy HF+ (native USB driver) ===");
        for (i, d) in devices.iter().enumerate() {
            // Which model this is cannot be known without opening it — every
            // HF+ enumerates as the same 03eb:800c — so the list does not
            // pretend to. `--example probe` opens one and says.
            println!("  {}: {}", i, d.label());
        }
    }
    println!();
}

/// The Lime boards LimeSuite reports.
///
/// Unlike the `nusb` scans above this one is not free — LimeSuite opens each
/// candidate to read its identity — and it can fail in a way the others cannot:
/// the library may simply not be installed. Each of those is a different thing
/// to say.
fn probe_lime() {
    match sdroxide_lime::try_list() {
        Err(e) => println!("LimeSDR: {e}\n"),
        Ok(found) => {
            if found.devices.is_empty() {
                println!("No LimeSDR boards found by LimeSuite.");
            } else {
                println!("=== LimeSDR family (LimeSuite) ===");
                for (i, d) in found.devices.iter().enumerate() {
                    println!("  {i}: {}", d.label());
                }
            }
            // Named rather than silently dropped: LimeSuite claims the bare
            // Cypress FX3 id an unprogrammed RX-888 also presents, and "my
            // board is missing from the list" is a worse thing to debug than a
            // line saying what was skipped and why.
            if !found.rejected.is_empty() {
                println!(
                    "  ({} device(s) LimeSuite listed but this backend does not drive: {})",
                    found.rejected.len(),
                    found.rejected.join("; ")
                );
            }
            println!();
        }
    }
}

fn probe_elad() {
    let devices = sdroxide_elad::list();
    if devices.is_empty() {
        println!("No ELAD FDM-DUO or FDM-S receivers found on USB.");
    } else {
        println!("=== ELAD FDM-DUO / FDM-S (native USB driver) ===");
        for (i, d) in devices.iter().enumerate() {
            // No serial number here on purpose: ELAD keeps it in the device's
            // EEPROM rather than in the USB descriptor, so reading it would mean
            // claiming a device that may be streaming. `--example probe` opens
            // one and says.
            println!("  {}: {}  [usb {:04x}:{:04x}]", i, d.label(), 0x1721, d.pid);
        }
    }
    println!();
}

fn probe_airspy() {
    let devices = sdroxide_airspy::list();
    if devices.is_empty() {
        println!("No Airspy R2 or Mini receivers found on USB.");
    } else {
        println!("=== Airspy R2 / Mini (native USB driver) ===");
        for (i, d) in devices.iter().enumerate() {
            // An R2 and a Mini share 1d50:60a1 and the same product string;
            // only the rate list separates them, and that needs the device
            // open. The list does not pretend to know.
            println!("  {}: {}", i, d.label());
        }
    }
    println!();
}

/// The HydraSDR list, and the one thing it has to be careful about.
///
/// A prototype RFOne enumerates on the Airspy R2's own USB id, so the
/// enumeration only claims such a board when its descriptors say HydraSDR — see
/// `sdroxide_hydrasdr::usb`. Anything on that id that does *not* is left to the
/// Airspy list above, which is why running both probes is worth it.
fn probe_hydrasdr() {
    let devices = sdroxide_hydrasdr::list();
    if devices.is_empty() {
        println!("No HydraSDR RFOne receivers found on USB.");
    } else {
        println!("=== HydraSDR RFOne (native USB driver) ===");
        for (i, d) in devices.iter().enumerate() {
            println!("  {}: {}", i, d.label());
        }
    }
    println!();
}

fn probe_hackrf() {
    let devices = sdroxide_hackrf::list();
    if devices.is_empty() {
        println!("No HackRFs found on USB.");
    } else {
        println!("=== HackRF (native USB driver) ===");
        for (i, d) in devices.iter().enumerate() {
            // The board *revision* needs a control transfer, so it is not here;
            // the product id and product string are what enumeration gives away
            // for free, and between them they separate a HackRF One from a Pro,
            // a Jawbreaker or a rad1o — which matters, because the four do not
            // take the same sample rates.
            println!("  {}: {}", i, d.label());
        }
    }
    println!();
}

fn probe_sdrplay() {
    // Unlike the USB probes this one can fail three different ways, each with
    // a different fix, so the failure text is the whole point of printing it.
    match sdroxide_sdrplay::try_list() {
        Ok(devices) if devices.is_empty() => {
            println!("No SDRplay RSPs reported by the SDRplay API service.");
        }
        Ok(devices) => {
            println!("=== SDRplay RSP (vendor API service) ===");
            for (i, d) in devices.iter().enumerate() {
                println!("  {}: {}", i, d.label());
                if let Some(w) = d.identity_warning() {
                    println!("     ! {w}");
                }
            }
        }
        Err(e) => println!("SDRplay: {e}"),
    }
    println!();
}

#[cfg(feature = "soapy")]
fn probe_soapy(cli: &Cli, settings: &Settings) -> anyhow::Result<()> {
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
fn probe_soapy(_cli: &Cli, _settings: &Settings) -> anyhow::Result<()> {
    println!("This build has no SoapySDR support (built with --no-default-features).");
    Ok(())
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
    // Both directions are always printed, including the empty case: a driver
    // that implements no frequency-range call is a thing an operator needs to
    // be told, not a line that quietly goes missing. This is the device's own
    // answer — any range stated in radio.json is applied when the radio is
    // opened for use, not here.
    for (name, ranges, chan) in [
        ("RX freq", &caps.freq_ranges_rx, caps.rx_channels),
        ("TX freq", &caps.freq_ranges_tx, caps.tx_channels),
    ] {
        if chan == 0 {
            continue;
        }
        if ranges.is_empty() {
            println!("  {name:<13} : not published by this driver (set one in Settings → Radio)");
        } else {
            let list: Vec<String> = ranges
                .iter()
                .map(|&(lo, hi)| format!("{} – {}", fmt_mhz(lo), fmt_mhz(hi)))
                .collect();
            println!("  {:<13} : {}", name, list.join(", "));
        }
    }
    if !caps.sample_rates.is_empty() {
        let list: Vec<String> =
            caps.sample_rates.iter().map(|r| format!("{:.3}", r / 1e6)).collect();
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
            Box::new(SigGenSource::demo(rate, cli.center_hz())),
            synthetic_caps("Signal generator"),
        ));
    }
    if let Some(path) = &cli.file {
        let label = format!("IQ file {}", path.display());
        return Ok((
            Box::new(
                FileSource::open_with(
                    path,
                    rate,
                    cli.center_hz(),
                    cli.rate.is_some(),
                    cli.freq_named,
                )
                .with_context(|| format!("opening IQ file {}", path.display()))?,
            ),
            synthetic_caps(&label),
        ));
    }

    // Try the configured radio interface. If it can't be opened (no SoapySDR
    // device, HPSDR unreachable, CAT port missing, TCI server not up yet, …)
    // fall back to a null source so the GUI — and the Settings dialog — still
    // come up, instead of the program refusing to launch. The engine keeps
    // retrying this same interface in the background (`IqSource::needs_reopen`),
    // so a rig that simply wasn't ready attaches by itself; Settings → Radio is
    // only needed to choose a *different* one.
    let radio = sdroxide_config::load_radio_config();
    match open_converted_source(&radio, cli, settings) {
        Ok(pair) => Ok(pair),
        Err(e) => {
            tracing::warn!("radio interface unavailable: {e:#}");
            let msg =
                format!("{e}. Retrying — or open Settings → Radio to choose another interface.");
            Ok((
                Box::new(null_source::NullSource::new(cli.center_hz(), msg)),
                synthetic_caps("No radio"),
            ))
        }
    }
}

/// [`open_configured_source`], with the operator's own tuning ranges and any
/// external frequency converter folded in.
///
/// The distinction that makes this a separate function: the centre
/// `open_configured_source` passes to each back end is a *hardware* frequency,
/// while the one this is called with — from `--freq`, from the restored session,
/// or from the engine's current dial on a reopen — is the operator's. So the
/// offset goes on here, once, and [`ConvertedSource`] takes it off again for
/// everything the source reports back.
///
/// Stated ranges go on *after* the offset, because they are the one thing here
/// the operator typed: a transverter owner who writes 144-148 means the band
/// they tune, not the 28-32 MHz I.F. the radio is really on. Only the device's
/// own published ranges are in the hardware's domain, and `shift_caps` moves
/// those into the operator's before the stated ones replace them (issue #279).
fn open_converted_source(
    radio: &RadioConfig,
    cli: &Cli,
    settings: &Settings,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    if radio.panadapter.is_attached() {
        return open_paired_source(radio, cli, settings);
    }
    let plan = converter_plan(radio);
    if plan.is_transparent() {
        let (source, caps) = open_configured_source(radio, cli, settings)?;
        return Ok((source, stated_ranges(caps, radio)));
    }
    let mut c = cli.clone();
    // Not simply `dial + offset`: a dial from before the converter was set up is
    // still in the hardware's own domain, and that sum is below DC — see
    // `converter_open_hz`.
    let offset = plan.offset_for(cli.center_hz());
    let hw = sdroxide_radio::converter_open_hz(cli.center_hz(), offset);
    if hw != cli.center_hz() + offset {
        tracing::info!(
            "the dial was on {:.6} MHz, which is where the hardware is rather than where this \
             converter puts it: opening there, so the same signal comes back on {:.6} MHz",
            cli.center_hz() / 1e6,
            (hw - offset) / 1e6,
        );
    }
    c.freq = Some(hw);
    let (source, caps) = open_configured_source(radio, &c, settings)?;
    // Device ranges down into the operator's domain first, then whatever the
    // operator stated on top of them — that order is what makes a typed range
    // mean the dial rather than the I.F.
    let caps = sdroxide_radio::plan_caps(caps, &plan, &radio.freq_ranges_rx, &radio.freq_ranges_tx);
    log_stated_ranges(radio, &caps);
    Ok((Box::new(ConvertedSource::with_plan(source, plan)), caps))
}

/// What is in front of this radio, band by band: the transverter table first,
/// then the single whole-dial converter offset as the last resort.
///
/// Both live in `radio.json` and both mean the same thing — the hardware is
/// tuned to `dial + offset` — so they are one list here rather than two
/// mechanisms. A dial that no transverter covers falls through to the single
/// offset, and to the bare radio when that is zero as well, which is what keeps
/// HF working on a station whose only converter is a 2 m transverter
/// (issue #278).
fn converter_plan(radio: &RadioConfig) -> sdroxide_radio::ConverterPlan {
    use sdroxide_radio::ConverterStep;
    let mut steps: Vec<ConverterStep> = Vec::new();
    for x in radio.transverters.iter().take(sdroxide_types::MAX_TRANSVERTERS) {
        if !x.enabled || !x.is_band() || !x.offset_hz.is_finite() {
            continue;
        }
        let tx_offset = x.tx.offset_hz(x.offset_hz);
        steps.push(ConverterStep {
            band: Some((x.rf_lo_hz, x.rf_hi_hz)),
            rx_offset_hz: x.offset_hz,
            tx_offset_hz: tx_offset,
            // A ceiling of 1.0 is no ceiling; anything less is one.
            tx_drive: (x.tx_drive < 1.0).then(|| x.tx_drive.clamp(0.0, 1.0)),
        });
        tracing::info!(
            "transverter {}: {:.3}–{:.3} MHz on the dial, hardware {:.6} MHz {}; transmit {}",
            x.describe(),
            x.rf_lo_hz / 1e6,
            x.rf_hi_hz / 1e6,
            x.offset_hz.abs() / 1e6,
            if x.offset_hz < 0.0 { "below" } else { "above" },
            match tx_offset {
                None => "withdrawn".to_string(),
                Some(_) => format!("at up to {:.0}% drive", x.tx_drive * 100.0),
            }
        );
    }
    let offset = radio.converter_offset_hz;
    if offset != 0.0 && offset.is_finite() {
        // The transmit line is a separate fact about the station, and the
        // operator states it — see `sdroxide_types::ConverterTx`.
        let tx_offset = radio.converter_tx.offset_hz(offset);
        tracing::info!(
            "frequency converter: hardware tuned {:.6} MHz above the dial; transmit {}",
            offset / 1e6,
            match tx_offset {
                None => "withdrawn".to_string(),
                Some(t) if t == 0.0 =>
                    "not converted (the radio transmits on the dial)".to_string(),
                Some(t) => format!("tuned {:.6} MHz above the dial", t / 1e6),
            }
        );
        steps.push(ConverterStep {
            band: None,
            rx_offset_hz: offset,
            tx_offset_hz: tx_offset,
            tx_drive: None,
        });
    }
    sdroxide_radio::ConverterPlan::from_steps(steps)
}

/// Open a radio that borrows another roster radio's receiver as its panadapter:
/// both devices, wrapped as one [`crate::panadapter_source::PanadapterSource`].
///
/// The receiver is opened through [`open_converted_source`] on *its own*
/// configuration, so its converter offset, its stated tuning ranges and every
/// setting on its own Radio page apply exactly as they would if it were running
/// as a radio of its own. What it is asked for is the operator's dial plus the
/// pairing's offset — the receiver's centre is the one frequency in this whole
/// arrangement that is not on the dial's scale.
fn open_paired_source(
    radio: &RadioConfig,
    cli: &Cli,
    settings: &Settings,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let pan = radio.panadapter.clone();
    let id = pan.source_radio.context("no panadapter receiver selected")?;
    // A receiver that has been removed from the roster is gone for good, and a
    // transceiver must not be taken off the air by the disappearance of the
    // radio that used to draw its spectrum. Say so and open it on its own; the
    // Panadapter box is empty of candidates now, so the operator's next visit to
    // the dialog shows the pairing already undone.
    if !sdroxide_config::load_radios().radios.iter().any(|s| s.id == id) {
        tracing::warn!(
            "the panadapter receiver (radio {}) is no longer in the roster; opening this radio on \
             its own",
            id + 1
        );
        let mut plain = radio.clone();
        plain.panadapter = Default::default();
        return open_converted_source(&plain, cli, settings);
    }
    let rx_cfg = sdroxide_config::Store::radio(id).load_radio_config();
    // One level only. Chains and cycles are refused here rather than allowed to
    // recurse: two radios attached to each other would each try to open the
    // other's device, and neither would ever finish.
    if rx_cfg.panadapter.is_attached() {
        bail!(
            "radio {} is itself using another radio as its panadapter — a receiver can only be \
             borrowed once",
            id + 1
        );
    }
    if rx_cfg.backend == Backend::None {
        bail!("radio {} has no interface selected to receive with", id + 1);
    }

    // The offset picks itself once the mode is known; until then the pairing
    // opens on the plain one and the engine corrects it before the first block
    // (see `PanadapterSource::mode`).
    let mut rx_cli = cli.clone();
    rx_cli.freq = Some(cli.center_hz() + pan.offset_hz);
    let (rx, rx_caps) = open_converted_source(&rx_cfg, &rx_cli, settings)
        .with_context(|| format!("opening radio {} as the panadapter receiver", id + 1))?;
    let (ctrl, ctrl_caps) = open_configured_source(radio, cli, settings)?;
    let ctrl_caps = stated_ranges(ctrl_caps, radio);

    let caps = crate::panadapter_source::PanadapterSource::merge_caps(&rx_caps, &ctrl_caps, &pan);
    let source = crate::panadapter_source::PanadapterSource::new(rx, ctrl, pan, None);
    Ok((Box::new(source), caps))
}

/// Which roster radios are somebody else's panadapter receiver, and whose.
///
/// Read once at boot from every scope's `radio.json`. Such a radio does not
/// open its own device — the radio that borrowed it does — so it comes up on
/// the stand-in source instead, with a tab that says as much.
fn panadapter_owners() -> std::collections::HashMap<u32, u32> {
    let roster = sdroxide_config::load_radios();
    let mut owners = std::collections::HashMap::new();
    for slot in &roster.radios {
        // A radio that is switched off opens nothing, so it borrows nothing:
        // the receiver it was lent goes back to being a radio of its own for as
        // long as the borrower is off, rather than being held by a radio that
        // is not running.
        if !slot.enabled {
            continue;
        }
        let cfg = sdroxide_config::Store::radio(slot.id).load_radio_config();
        let Some(rx) = cfg.panadapter.source_radio else { continue };
        // Neither of the two ways a pairing can eat itself gets to boot.
        if rx == slot.id {
            tracing::warn!("radio {} is attached to itself as a panadapter; ignoring", slot.id + 1);
            continue;
        }
        if !roster.radios.iter().any(|s| s.id == rx) {
            tracing::warn!(
                "radio {} names radio {} as its panadapter, which is not in the roster; ignoring",
                slot.id + 1,
                rx + 1
            );
            continue;
        }
        // First claim wins, so two radios naming the same receiver cannot both
        // open it. The second is left as an ordinary radio and says so.
        if let Some(first) = owners.get(&rx) {
            tracing::warn!(
                "radios {} and {} both claim radio {} as a panadapter; leaving it with radio {}",
                first + 1,
                slot.id + 1,
                rx + 1,
                first + 1
            );
            continue;
        }
        owners.insert(rx, slot.id);
    }
    owners
}

/// Apply the tuning ranges stated in `radio.json`, and say so in the log —
/// a range the operator set months ago is otherwise invisible when a band later
/// refuses to come up.
fn stated_ranges(caps: DeviceCaps, radio: &RadioConfig) -> DeviceCaps {
    let out = override_caps_ranges(caps, &radio.freq_ranges_rx, &radio.freq_ranges_tx);
    log_stated_ranges(radio, &out);
    out
}

/// Say in the log which of the ranges now in `caps` came from the configuration
/// rather than from the device.
fn log_stated_ranges(radio: &RadioConfig, caps: &DeviceCaps) {
    for (dir, stated, applied) in [
        ("RX", &radio.freq_ranges_rx, &caps.freq_ranges_rx),
        ("TX", &radio.freq_ranges_tx, &caps.freq_ranges_tx),
    ] {
        if !stated.is_empty() {
            tracing::info!(
                "{dir} tuning range set in the configuration: {} MHz",
                sdroxide_types::format_freq_ranges(applied)
            );
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
    // `--rate` reaches only the backends that have somewhere to put it. Saying
    // so is the point: it used to be accepted and silently dropped, which reads
    // as "the flag works and the radio ignored it". Once per run, not once per
    // reconnect — this is called again every time the engine reopens.
    if let Some(rate) = cli.rate {
        if !matches!(radio.backend, Backend::Soapy | Backend::Pluto | Backend::Lime | Backend::Auto)
        {
            static SAID: std::sync::Once = std::sync::Once::new();
            SAID.call_once(|| {
                tracing::warn!(
                    "--rate {:.3} Msps does not apply to the {} interface — set its sample \
                     rate in radio.json instead",
                    rate / 1e6,
                    radio.backend.label()
                );
            });
        }
    }
    match radio.backend {
        // A freshly created radio tab: nothing is opened — and nothing may be,
        // or this tab would grab a device another radio is already running —
        // until the operator picks an interface.
        Backend::None => bail!("no radio interface selected — choose one in Settings → Radio"),
        Backend::Cat => open_cat_source(radio),
        Backend::Hpsdr => open_hpsdr_source(radio, cli.center_hz()),
        Backend::Tci => open_tci_source(radio, cli.center_hz()),
        Backend::IcomNet => open_icomnet_source(radio),
        Backend::SmartSdr => open_smartsdr_source(radio, cli.center_hz()),
        Backend::Pluto => open_pluto_source(radio, cli.center_hz(), cli.rate),
        Backend::RtlSdr => open_rtlsdr_source(radio, cli.center_hz()),
        Backend::RtlTcp => open_rtltcp_source(radio, cli.center_hz()),
        Backend::SpyServer => open_spyserver_source(radio, cli.center_hz()),
        Backend::SpyServerVfo => open_spyserver_vfo_source(radio, cli.center_hz()),
        Backend::KiwiSdr => open_kiwisdr_source(radio, cli.center_hz()),
        Backend::Rx888 => open_rx888_source(radio, cli.center_hz()),
        Backend::AirspyHf => open_airspyhf_source(radio, cli.center_hz()),
        Backend::Airspy => open_airspy_source(radio, cli.center_hz()),
        Backend::HydraSdr => open_hydrasdr_source(radio, cli.center_hz()),
        Backend::HackRf => open_hackrf_source(radio, cli.center_hz()),
        Backend::SdrPlay => open_sdrplay_source(radio, cli.center_hz()),
        Backend::Elad => open_elad_source(radio, cli.center_hz()),
        Backend::Lime => open_lime_source(radio, cli.center_hz(), cli.rate),
        Backend::Fobos => open_fobos_source(radio, cli.center_hz()),
        Backend::Soapy => open_soapy_source(cli, settings, radio),
        Backend::Auto => {
            #[cfg(feature = "soapy")]
            {
                // "Is a SoapySDR radio present?" — a sound card must not answer
                // yes here either, or auto-detect lands on it instead of falling
                // through to the CAT rig.
                let filter = device_filter(cli, settings);
                if selectable_soapy_devices(&filter)
                    .map(|(real, _)| real.is_empty())
                    .unwrap_or(true)
                {
                    open_cat_source(radio)
                } else {
                    open_soapy_source(cli, settings, radio)
                }
            }
            #[cfg(not(feature = "soapy"))]
            {
                open_cat_source(radio)
            }
        }
    }
}

/// Whether the operator's filter names a driver at all. A filter that says
/// `driver=audio` is an explicit request for the sound card and is obeyed; an
/// empty one, or one that only narrows by serial, is not.
#[cfg(feature = "soapy")]
fn filter_names_driver(filter: &str) -> bool {
    filter.to_ascii_lowercase().contains("driver=")
}

/// The devices an automatic pick may choose from, and the ones held back:
/// everything, minus the modules that are not radios, unless the filter asked
/// for one by name.
///
/// Without this the sound card wins on any bundle install — SoapyAudio
/// enumerates ahead of the real hardware, opens happily, and produces a
/// plausible-looking spectrum of the machine's line input on whatever
/// frequency the dial claims. See [`SoapyDeviceInfo::driver_is_pseudo`].
///
/// The held-back list comes back rather than being dropped so the caller can
/// tell "nothing is plugged in" from "the only thing here is a sound card",
/// which need entirely different advice.
#[cfg(feature = "soapy")]
fn selectable_soapy_devices(filter: &str) -> anyhow::Result<(Vec<DeviceInfo>, Vec<DeviceInfo>)> {
    use sdroxide_types::SoapyDeviceInfo;
    let all = enumerate_devices(filter).context("SoapySDR enumeration failed")?;
    if filter_names_driver(filter) {
        return Ok((all, Vec::new()));
    }
    let (real, pseudo): (Vec<_>, Vec<_>) =
        all.into_iter().partition(|d| !SoapyDeviceInfo::driver_is_pseudo(&d.driver));
    for d in &pseudo {
        tracing::info!(
            driver = %d.driver,
            label = %d.label,
            "skipping a SoapySDR module that is not a radio — name it with \
             --device driver={} to open it anyway",
            d.driver.to_ascii_lowercase(),
        );
    }
    Ok((real, pseudo))
}

/// Open the first available SoapySDR device (feature-gated).
#[cfg(feature = "soapy")]
fn open_soapy_source(
    cli: &Cli,
    settings: &Settings,
    radio: &RadioConfig,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    // `--rate` first, then the operator's SoapySDR block, then the app-wide
    // rate. The middle one is new; a configuration written before it existed
    // leaves it at zero and lands on the app-wide rate exactly as before.
    let rate = cli.rate.unwrap_or_else(|| {
        if radio.soapy.sample_rate_hz > 0.0 {
            radio.soapy.sample_rate_hz
        } else {
            settings.sample_rate
        }
    });
    let filter = device_filter(cli, settings);
    let (devices, skipped) = selectable_soapy_devices(&filter)?;
    let Some(info) = devices.first() else {
        // Two different situations, two different fixes: nothing plugged in at
        // all, versus a machine whose only "SDR" is its sound card.
        if skipped.is_empty() {
            bail!("no SoapySDR devices found (filter: {:?})", filter);
        }
        bail!(
            "no SoapySDR radios found (filter: {:?}) — the only devices SoapySDR reports \
             are sound cards ({}), which are not radios and are never opened automatically. \
             Pick a native interface in Settings → Radio, or ask for the sound card by name \
             with --device driver=audio.",
            filter,
            skipped.iter().map(|d| d.label.as_str()).collect::<Vec<_>>().join(", "),
        );
    };
    // Which one, not just that there was one: on a bundle install this is the
    // only line that says whether the radio being opened is the radio meant.
    tracing::info!(driver = %info.driver, label = %info.label, "SoapySDR device selected");
    let dev =
        SoapyDevice::open(&info.args).with_context(|| format!("opening device {}", info.label))?;
    let caps = dev.caps().clone();
    Ok((Box::new(dev.rx_source(rate, cli.center_hz(), cli.gain, &radio.soapy)?), caps))
}

#[cfg(not(feature = "soapy"))]
fn open_soapy_source(
    _cli: &Cli,
    _settings: &Settings,
    _radio: &RadioConfig,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    // The message a fresh install of a `-no-soapysdr` package meets, because
    // `Backend::Soapy` is what an unconfigured `radio.json` says (issue #220).
    // So it has to name the way out rather than only the fault.
    bail!(
        "SoapySDR support is not compiled into this build — open Settings → Radio and pick a \
         built-in interface (HackRF, RTL-SDR, Airspy, AirspyHF+, SDRplay, RX-888, LimeSDR, \
         PlutoSDR, HPSDR, a CAT rig, a network radio…), or install the SoapySDR build of \
         sdroxide"
    )
}

/// Build the CAT + sound-card source and its capabilities from radio.json.
fn open_cat_source(radio: &RadioConfig) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let src = audio_cat_source::AudioCatSource::open(
        radio.cat.clone(),
        radio.radio_audio_in.as_deref(),
        radio.radio_audio_out.as_deref(),
    )
    .context("opening CAT rig")?;
    // The antenna sockets are the family's, so they come off the open rig
    // rather than out of the config: only an ELAD FDM-DUO has two, and it has
    // them whether it is reached through this interface or its own.
    let mut caps = cat_caps(radio);
    caps.antennas_rx = src.antennas().iter().map(|a| a.to_string()).collect();
    Ok((Box::new(src), caps))
}

/// Build the HPSDR (ethernet SDR) source from radio.json. The target IP is the
/// manual override, else the persisted selection, else the first device found by
/// a discovery scan; the protocol is detected when the connection opens.
fn open_hpsdr_source(
    radio: &RadioConfig,
    center_hz: f64,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let ip: std::net::Ipv4Addr = if let Some(s) = radio.hpsdr.target_ip() {
        s.trim().parse().with_context(|| format!("invalid HPSDR IP address {s:?}"))?
    } else {
        let found = sdroxide_hpsdr::discover_default();
        let dev = found.iter().find(|d| d.supported()).ok_or_else(|| {
            anyhow::anyhow!("no HPSDR device found on the network — enter a target IP in Settings")
        })?;
        dev.ip.parse().with_context(|| format!("discovered HPSDR IP {:?}", dev.ip))?
    };

    let src = hpsdr_source::HpsdrSource::open(ip, &radio.hpsdr, center_hz)
        .context("opening HPSDR device")?;
    let caps = hpsdr_caps(
        src.board(),
        src.sample_rate_hz(),
        src.protocol(),
        src.has_lna_gain(),
        radio.hpsdr.ddc,
    );
    Ok((Box::new(src), caps))
}

/// Capabilities for an HPSDR board: wideband IQ (not `audio_mode`), TX-capable,
/// half-duplex. The board enforces its own limits. Protocol 1 boards top out at
/// 384 kHz, and a Hermes-Lite 2 samples at 76.8 MHz, so its Nyquist limit is
/// 38.4 MHz — tuning past that on one only aliases. A secondary DDC (`ddc >
/// 0`) has no transmitter: the board has one DUC, and it belongs to the DDC-0
/// radio.
fn hpsdr_caps(board: &str, sample_rate: f64, protocol: u8, has_lna: bool, ddc: u8) -> DeviceCaps {
    let hermes_lite = sdroxide_hpsdr::board_has_lna_gain(board);
    let nyquist = if hermes_lite { 38_400_000.0 } else { 61_440_000.0 };
    let gains = if has_lna {
        vec![sdroxide_types::GainElement::db(
            sdroxide_hpsdr::LNA_GAIN_ELEMENT,
            sdroxide_types::Direction::Rx,
            sdroxide_hpsdr::LNA_GAIN_MIN_DB,
            sdroxide_hpsdr::LNA_GAIN_MAX_DB,
            1.0,
        )]
    } else {
        Vec::new()
    };
    let (label, tx_channels) = if ddc == 0 {
        (format!("{board} (HPSDR P{protocol}, {:.3} Msps)", sample_rate / 1e6), 1)
    } else {
        (format!("{board} DDC{} (HPSDR P{protocol}, {:.3} Msps)", ddc + 1, sample_rate / 1e6), 0)
    };
    DeviceCaps {
        driver: "hpsdr".into(),
        label,
        rx_channels: 1,
        tx_channels,
        audio_mode: false,
        freq_ranges_rx: vec![(0.0, nyquist)],
        freq_ranges_tx: if tx_channels > 0 {
            vec![(1_800_000.0, if hermes_lite { 30_000_000.0 } else { 54_000_000.0 })]
        } else {
            Vec::new()
        },
        sample_rates: sdroxide_types::HpsdrConfig::rates_for(protocol).to_vec(),
        gains,
        ..DeviceCaps::default()
    }
}

/// Build the RTL-SDR source from radio.json. The dongle is picked by USB
/// serial, or the first one found when none is configured.
fn open_rtlsdr_source(
    radio: &RadioConfig,
    center_hz: f64,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let src = rtlsdr_source::RtlSdrSource::open(&radio.rtlsdr, center_hz)
        .context("opening RTL-SDR dongle")?;
    let caps = rtlsdr_caps(&src, "rtlsdr");
    Ok((Box::new(src), caps))
}

/// The same dongle on another machine, reached through its `rtl_tcp` server.
fn open_rtltcp_source(
    radio: &RadioConfig,
    center_hz: f64,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let src = rtlsdr_source::RtlSdrSource::connect(&radio.rtltcp, center_hz)
        .with_context(|| format!("connecting to rtl_tcp at {}", radio.rtltcp.endpoint()))?;
    let caps = rtlsdr_caps(&src, "rtltcp");
    Ok((Box::new(src), caps))
}

/// Build the RX-888 source from radio.json. Uploads the FX3 firmware first if
/// the receiver is still sitting in its boot ROM, which it will be on every
/// fresh plug-in.
fn open_rx888_source(
    radio: &RadioConfig,
    center_hz: f64,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let src = rx888_source::Rx888Source::open(&radio.rx888, center_hz)
        .context("opening RX-888 receiver")?;
    let caps = rx888_caps(&src);
    Ok((Box::new(src), caps))
}

/// Capabilities for an RX-888: wideband IQ, receive only.
///
/// Two things differ from every other backend here. The sample rate advertised
/// is the *downconverter's* output, not the ADC clock — the hardware has no DDC,
/// so the conversion from 64.8 Msps of real samples to complex baseband happens
/// on the host. And the frequency range is two ranges, not one: direct sampling
/// up to the ADC's Nyquist limit, then the R828D tuner above it. On a slow ADC
/// clock the two do not meet, and the gap is published rather than papered over
/// — see `sdroxide_rx888::band::freq_ranges`.
fn rx888_caps(src: &rx888_source::Rx888Source) -> DeviceCaps {
    use sdroxide_types::{Direction, GainElement, Rx888Config};
    let rate = src.sample_rate_hz();
    let mut gains = vec![
        // The AD8370's vernier is linear in voltage, so the dB step
        // varies; a request is snapped to the nearest code and reported
        // back, which makes a fine slider honest enough.
        GainElement::db(Rx888Config::VGA_ELEMENT, Direction::Rx, -6.0, 34.0, 0.5),
        GainElement::db(Rx888Config::ATT_ELEMENT, Direction::Rx, -31.5, 0.0, 0.5),
    ];
    // Only offer the tuner's gain on a receiver that has one, so the control
    // does not appear on a board where it would do nothing.
    if src.vhf_capable() {
        gains.push(
            // 29 discrete steps, snapped and reported back like the two above.
            GainElement::db(
                Rx888Config::TUNER_GAIN_ELEMENT,
                Direction::Rx,
                0.0,
                Rx888Config::TUNER_GAIN_MAX_DB,
                0.1,
            ),
        );
    }
    DeviceCaps {
        driver: "rx888".into(),
        label: src.describe(),
        rx_channels: 1,
        tx_channels: 0,
        audio_mode: false,
        freq_ranges_rx: src.freq_ranges(),
        sample_rates: vec![rate],
        gains,
        ..DeviceCaps::default()
    }
}

/// Build the Airspy HF+ source from radio.json. The receiver is picked by the
/// serial in its USB descriptor, or the first one found when none is set.
fn open_airspyhf_source(
    radio: &RadioConfig,
    center_hz: f64,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let src = airspyhf_source::AirspyHfSource::open(&radio.airspyhf, center_hz)
        .context("opening Airspy HF+ receiver")?;
    let caps = airspyhf_caps(&src);
    Ok((Box::new(src), caps))
}

/// Capabilities for an Airspy HF+: wideband IQ, receive only, HF plus the VHF
/// window.
///
/// Two things here are the *device's* answer rather than a constant. The sample
/// rates depend on the model and the firmware together, so they are read off
/// the opened receiver the way [`rx888_caps`] reads its converter rate; and the
/// attenuator's range and step come from the table the receiver reports, which
/// differs between models. The frequency ranges do come from a table, because
/// nothing on the device publishes them — see `AirspyHfModel::freq_ranges`.
///
/// The one gain element is the attenuator, carried negative so more slider is
/// more signal, like the RX-888's. The switches (AGC and its threshold, the
/// preamp, the bias tee, the calibration and the host DSP) ride pseudo-elements
/// that are deliberately not listed here, so only the Airspy HF+ settings panel
/// renders them.
fn airspyhf_caps(src: &airspyhf_source::AirspyHfSource) -> DeviceCaps {
    use sdroxide_types::{AirspyHfConfig, Direction, GainElement};
    let (att_max_db, att_step_db) = src.attenuator_range_db();
    DeviceCaps {
        driver: "airspyhf".into(),
        label: src.describe(),
        rx_channels: 1,
        tx_channels: 0,
        audio_mode: false,
        freq_ranges_rx: src.model().freq_ranges().to_vec(),
        sample_rates: src.available_rates().to_vec(),
        gains: vec![GainElement::db(
            AirspyHfConfig::ATT_ELEMENT,
            Direction::Rx,
            -att_max_db,
            0.0,
            att_step_db,
        )],
        ..DeviceCaps::default()
    }
}

/// Build the ELAD source from radio.json.
///
/// Three of the radio's blocks meet here, which is what an FDM-DUO is: the
/// `elad` block for the USB receiver, the `cat` block for the transceiver's
/// serial control link (the same one `Backend::Cat` uses, so an operator who
/// had the rig working there keeps their port and baud rate), and
/// `radio_audio_out` for transmit audio.
fn open_elad_source(
    radio: &RadioConfig,
    center_hz: f64,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let src = elad_source::EladSource::open(
        &radio.elad,
        &radio.cat,
        radio.radio_audio_out.as_deref(),
        center_hz,
    )
    .context("opening ELAD")?;
    let caps = elad_caps(&src);
    Ok((Box::new(src), caps))
}

/// Capabilities for an ELAD.
///
/// Two things here are decided by the model rather than published as a
/// constant. **Transmit** belongs to the FDM-DUO alone, and only when there is
/// a control path to key it with — a DUO whose serial port is unset still has
/// the USB gateway, but an FDM-S has nothing, so `tx_channels` follows
/// `can_transmit()` rather than the product id. And the **meters** need a link
/// that answers: over the USB gateway the rig cannot be asked anything, so
/// claiming an SWR sensor there would put a needle on screen that never moves.
///
/// `full_duplex` is false. Whether the DDC keeps streaming through an over has
/// not been checked on hardware, and half duplex is the assumption that is safe
/// to be wrong about: it costs a receiver that goes quiet for the length of a
/// transmission, where the other way round costs a panadapter painting the
/// station's own transmitter.
fn elad_caps(src: &elad_source::EladSource) -> DeviceCaps {
    use sdroxide_types::{Direction, EladConfig, GainElement};
    let model = src.model();
    let tx = src.can_transmit();
    DeviceCaps {
        driver: "elad".into(),
        label: src.describe(),
        rx_channels: 1,
        tx_channels: usize::from(tx),
        full_duplex: false,
        // Wideband I/Q in, audio out — the same shape as TCI.
        tx_audio: tx,
        freq_ranges_rx: vec![model.rx_range_hz()],
        // The transceiver's own amateur coverage, which is narrower than what
        // its receiver hears: the PA and its low-pass bank are ham-band only.
        freq_ranges_tx: if tx { vec![(1_800_000.0, 54_000_000.0)] } else { Vec::new() },
        // The rate is not commandable, so this list is what the stream may be
        // read as rather than what the device can be told to do. See
        // `EladConfig::sample_rate_hz`.
        sample_rates: sdroxide_types::ELAD_SAMPLE_RATES.iter().map(|&r| r as f64).collect(),
        // One real gain: the input pad, in or out. The pre-selection filter
        // switch is a pseudo-element and deliberately absent, so only this
        // backend's own settings tab draws it.
        gains: vec![GainElement::db(
            EladConfig::ATT_ELEMENT,
            Direction::Rx,
            -sdroxide_types::ELAD_ATTENUATOR_DB,
            0.0,
            sdroxide_types::ELAD_ATTENUATOR_DB,
        )],
        // The transceiver's two antenna sockets, on either control path — the
        // rig's `AN` command. Receive only, because that is all `AN` moves: it
        // chooses whether the receiver listens on the shared RTX socket or on
        // the RX-only one, and transmit leaves by RTX either way. An FDM-S has
        // one input and no way to be told anything, so it lists nothing and the
        // control never appears.
        antennas_rx: if src.switches_antenna() {
            sdroxide_types::EladAntenna::names()
        } else {
            Vec::new()
        },
        has_swr_sensor: tx && src.reads_rig(),
        has_fwd_power_sensor: false,
        ..DeviceCaps::default()
    }
}

/// Build the Airspy R2 / Mini source from radio.json. The receiver is picked by
/// the suffix of its USB serial, or the first one found when none is configured.
fn open_airspy_source(
    radio: &RadioConfig,
    center_hz: f64,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let src = airspy_source::AirspySource::open(&radio.airspy, center_hz)
        .context("opening Airspy R2/Mini")?;
    let caps = airspy_caps(&src);
    Ok((Box::new(src), caps))
}

/// Capabilities for an Airspy R2 / Mini: wideband IQ, receive only, VHF/UHF.
///
/// The **sample rates come off the device**, not from a table. An R2 offers 10
/// and 2.5 Msps and a Mini 6 and 3, the two are indistinguishable on the USB
/// bus, and publishing a union would offer every owner two rates their receiver
/// does not have. The tuning range is the R820T2's and is the same on both, so
/// that one is a constant.
///
/// One gain element: a step along the selected curve, 0 to 21. It is not a dB
/// figure — how much each step is worth depends on the curve and the band — so
/// `step_db` is 1 and the settings tab says what it means. The switches (the
/// curve itself, the two AGC loops, the bias tee, packing and the DC blocker)
/// ride pseudo-elements that are deliberately not listed here, so only the
/// Airspy panel renders them.
fn airspy_caps(src: &airspy_source::AirspySource) -> DeviceCaps {
    use sdroxide_types::{AirspyConfig, Direction, GainElement};
    DeviceCaps {
        driver: "airspy".into(),
        label: src.describe(),
        rx_channels: 1,
        tx_channels: 0,
        audio_mode: false,
        freq_ranges_rx: vec![AirspyConfig::FREQ_RANGE],
        sample_rates: src.available_rates().to_vec(),
        gains: vec![GainElement::db(
            AirspyConfig::GAIN_ELEMENT,
            Direction::Rx,
            0.0,
            (AirspyConfig::GAIN_STEPS - 1) as f64,
            1.0,
        )],
        ..DeviceCaps::default()
    }
}

/// Build the HydraSDR RFOne source from radio.json. The receiver is picked by
/// the suffix of its USB serial, or the first one found when none is configured.
fn open_hydrasdr_source(
    radio: &RadioConfig,
    center_hz: f64,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let src = hydrasdr_source::HydraSdrSource::open(&radio.hydrasdr, center_hz)
        .context("opening HydraSDR RFOne")?;
    let caps = hydrasdr_caps(&src);
    Ok((Box::new(src), caps))
}

/// Capabilities for a HydraSDR RFOne: wideband IQ, receive only, VHF/UHF.
///
/// The **sample rates come off the device**, not from a table — and here that
/// is not only good manners. Three of the seven this radio has are the ones its
/// firmware lists; the other four live in an alternate table that no
/// enumeration mentions, and an older build may not carry them. The driver
/// tries each and drops the ones that are refused, so what reaches here is what
/// this particular board will actually do.
///
/// The tuning range is the R828D's and is fixed, so that one is a constant.
///
/// One gain element: a step along the selected curve, 0 to 21. It is not a dB
/// figure — how much each step is worth depends on the curve and the band — so
/// `step_db` is 1 and the settings tab says what it means. The switches (the
/// curve itself, the two AGC loops, the RF port, the bias tee, packing and the
/// DC blocker) ride pseudo-elements that are deliberately not listed here, so
/// only the HydraSDR panel renders them.
fn hydrasdr_caps(src: &hydrasdr_source::HydraSdrSource) -> DeviceCaps {
    use sdroxide_types::{Direction, GainElement, HydraSdrConfig};
    DeviceCaps {
        driver: "hydrasdr".into(),
        label: src.describe(),
        rx_channels: 1,
        tx_channels: 0,
        audio_mode: false,
        freq_ranges_rx: vec![HydraSdrConfig::FREQ_RANGE],
        sample_rates: src.available_rates().to_vec(),
        gains: vec![GainElement::db(
            HydraSdrConfig::GAIN_ELEMENT,
            Direction::Rx,
            0.0,
            (HydraSdrConfig::GAIN_STEPS - 1) as f64,
            1.0,
        )],
        ..DeviceCaps::default()
    }
}

/// Build the Fobos SDR source from radio.json. The receiver is picked by its
/// serial, or the first one found when none is configured.
fn open_fobos_source(
    radio: &RadioConfig,
    center_hz: f64,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let src =
        fobos_source::FobosSource::open(&radio.fobos, center_hz).context("opening Fobos SDR")?;
    let caps = fobos_caps(&src, radio.fobos.port);
    Ok((Box::new(src), caps))
}

/// Capabilities for a Fobos SDR: wideband I/Q, receive only.
///
/// The tuning range depends on which port is open. On [`FobosPort::Rf`] it's
/// the RFFC507x wideband synthesiser's own documented range (25 MHz –
/// 5.4 GHz, from `libfobos`'s header comment on the low-level chip-tuning
/// call this backend never makes directly) — the chip's bound, not a
/// measured receiver-wide figure, so treated as approximate rather than
/// pinned. **Partially confirmed live**: sdroxide's own default startup
/// frequency (14.2 MHz, nothing to do with this backend) landed below this
/// floor on a real run, and `fobos_rx_set_frequency` genuinely refused it
/// (`FOBOS_ERR_UNSUPPORTED`) rather than silently mistuning — consistent
/// with the 25 MHz floor rather than just citing the same header comment
/// again. On the HF ports it's `0..adc_rate/2`, computed from what the
/// device actually reported at open (80 Msps on the unit this was verified
/// against) rather than a constant.
///
/// The **sample rates come off the device** on `Rf`; the HF ports don't have
/// a comparable discrete list (`sdroxide_dsp::WbDdc` snaps to the nearest
/// power-of-two bin count instead — see `sdroxide-fobos::stream::pick_bins`),
/// so the same device-reported list is offered there too as reasonable
/// targets, understanding the achieved rate can differ.
///
/// Two real gain elements, `Rf` only — see [`FobosPort`]'s own doc comment
/// for why they do nothing on the HF ports. The clock-source switch, and —
/// on [`FobosPort::HfDual`] — the diversity filter's mode/rate/hold, ride
/// pseudo-elements that are deliberately not listed here: the clock switch
/// so only the Fobos panel renders it, the diversity ones because
/// `diversity: true` below is what puts them on the main window's own
/// shared DIV strip instead (`sdroxide_types::DIV_MODE_ELEMENT`'s own doc
/// comment).
fn fobos_caps(src: &fobos_source::FobosSource, port: FobosPort) -> DeviceCaps {
    use sdroxide_types::{Direction, FobosConfig, GainElement};
    let rates = src.model_rates().to_vec();
    let freq_range = match port {
        FobosPort::Rf => (25_000_000.0, 5_400_000_000.0),
        // The full Nyquist span the wideband stream actually carries, not
        // just the narrower set of frequencies the downconverter's own
        // centre can land on (see sdroxide_dsp::reachable_range_hz's own
        // doc comment) — the same convention sdroxide-rx888's own
        // freq_ranges uses for the identical WbDdc clamp. A dial anywhere
        // in this range is genuinely receivable without a retune as long as
        // it stays inside the current span; only a request that would need
        // the *centre* to move somewhere unreachable needs the narrower
        // range, and that case is what achieved_center_hz/poll_control
        // corrects after the fact rather than what this range exists to
        // pre-empt. Publishing the narrower range here instead looked
        // right for a drag past the low end (refused outright, no flash)
        // but broke the much more common case: clicking anywhere still
        // inside the current passband, below the centre's own floor, was
        // refused too, even though nothing needed to retune for it at all.
        FobosPort::Hf1 | FobosPort::Hf2 | FobosPort::HfDual => (0.0, src.adc_rate_hz() / 2.0),
    };
    DeviceCaps {
        driver: "fobos".into(),
        label: src.describe(),
        rx_channels: 1,
        tx_channels: 0,
        audio_mode: false,
        freq_ranges_rx: vec![freq_range],
        sample_rates: rates,
        gains: if port == FobosPort::Rf {
            vec![
                GainElement::db(
                    FobosConfig::LNA_GAIN_ELEMENT,
                    Direction::Rx,
                    0.0,
                    f64::from(FobosConfig::LNA_GAIN_MAX),
                    1.0,
                ),
                GainElement::db(
                    FobosConfig::VGA_GAIN_ELEMENT,
                    Direction::Rx,
                    0.0,
                    f64::from(FobosConfig::VGA_GAIN_MAX),
                    1.0,
                ),
            ]
        } else {
            Vec::new()
        },
        diversity: port == FobosPort::HfDual,
        ..DeviceCaps::default()
    }
}

/// Build the HackRF source from radio.json. The radio is picked by the suffix
/// of its USB serial, or the first one found when none is configured.
fn open_hackrf_source(
    radio: &RadioConfig,
    center_hz: f64,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let src =
        hackrf_source::HackRfSource::open(&radio.hackrf, center_hz).context("opening HackRF")?;
    let caps = hackrf_caps(&src);
    Ok((Box::new(src), caps))
}

/// Capabilities for a HackRF: wideband IQ, half duplex, and a transmitter that
/// is only published when it has been armed.
///
/// `tx_channels` is the load-bearing field. With `tx_enabled` off it is zero,
/// which makes `DeviceCaps::is_transmit_capable()` false, which is what the
/// engine's own transmit gate reads — so an unarmed radio cannot be keyed by
/// any path, not merely by the ones that remembered to check. A HackRF is a
/// wideband transmitter with poor harmonic suppression that wants an external
/// low-pass filter; somebody who plugged one in to listen should not be one
/// PTT away from radiating.
///
/// `full_duplex` stays false: receive genuinely stops for the length of an
/// over, and claiming otherwise would let the engine keep a receive chain
/// running against a stream that has been torn down.
///
/// The frequency range is the firmware's, not the board's specified coverage:
/// DC – 7.25 GHz on every HackRF, matching libhackrf and SoapyHackRF. A HackRF
/// is deaf below 1 MHz and above 6 GHz, but it does tune there and people do
/// use it there, so the range that reaches the dial is the one the radio will
/// accept. See `sdroxide_hackrf::protocol::TUNING_RANGE_HZ`.
///
/// Three real gain elements, LNA first: `gains[0]` is what the main window's
/// Gain slider reaches, and the LNA is the stage that actually changes
/// sensitivity. The switches — both amp settings, the bias tee, the baseband
/// filter, ppm and the host-side IQ correction — ride pseudo-elements that are
/// deliberately absent here, so only the HackRF settings panel renders them.
fn hackrf_caps(src: &hackrf_source::HackRfSource) -> DeviceCaps {
    use sdroxide_types::{Direction, GainElement, HackRfConfig};
    let range = src.freq_range();
    let tx = src.tx_enabled();
    let mut gains = vec![
        GainElement::db(HackRfConfig::LNA_ELEMENT, Direction::Rx, 0.0, 40.0, 8.0),
        GainElement::db(HackRfConfig::VGA_ELEMENT, Direction::Rx, 0.0, 62.0, 2.0),
    ];
    if tx {
        gains.push(GainElement::db(HackRfConfig::TXVGA_ELEMENT, Direction::Tx, 0.0, 47.0, 1.0));
    }
    DeviceCaps {
        driver: "hackrf".into(),
        label: src.describe(),
        rx_channels: 1,
        tx_channels: usize::from(tx),
        full_duplex: false,
        audio_mode: false,
        freq_ranges_rx: vec![range],
        freq_ranges_tx: if tx { vec![range] } else { Vec::new() },
        sample_rates: HackRfConfig::SAMPLE_RATES.to_vec(),
        gains,
        ..DeviceCaps::default()
    }
}

/// Build the LimeSDR source from radio.json.
///
/// `--rate` is honoured here, unlike on the other USB backends: this interface
/// supersedes `--device driver=lime` on the SoapySDR one, which has always
/// taken the flag, and a migrating operator should not have it silently
/// dropped.
fn open_lime_source(
    radio: &RadioConfig,
    center_hz: f64,
    rate: Option<f64>,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let mut cfg = radio.lime.clone();
    if let Some(r) = rate {
        cfg.sample_rate_hz = r;
    }
    let src = lime_source::LimeSource::open(&cfg, center_hz).context("opening LimeSDR")?;
    let caps = lime_caps(&src);
    Ok((Box::new(src), caps))
}

/// Capabilities for a LimeSDR: wideband I/Q both ways, genuinely full duplex,
/// and a transmitter published only once it has been armed.
///
/// `tx_channels` is load-bearing in the same way it is for the HackRF: with
/// `tx_enabled` off it is zero, `DeviceCaps::is_transmit_capable()` is false,
/// and the engine's own gate refuses to key by any path.
///
/// `full_duplex` is true and earned — this radio has separate receive and
/// transmit chains and the USB 3 link to carry both, which is why
/// `read_available` is overridden in the source.
///
/// The frequency and rate ranges are **read from the board, never assumed**.
/// That is not politeness: a LimeSDR asked for a frequency below the LMS7002M's
/// range reconfigures its interface clock, fails half way, and then delivers
/// nothing at all until the process restarts — the fault recorded on the
/// SoapySDR path and guarded by the engine's retune limit, which can only work
/// from a published range.
///
/// One real gain element per direction. `LMS_SetGaindB` distributes a single
/// number across the LNA, TIA and PGA itself, and reaching those stages
/// individually needs a register-level call; three sliders that silently fought
/// the combined one would be worse than the one that works. Everything else —
/// the analog filters, the calibration trigger, the host-side IQ correction,
/// every LimeRFE control, and the second receive chain's gain and its diversity
/// or predistortion loop — rides pseudo-elements that are deliberately absent
/// here, so only this backend's own settings panel renders them.
fn lime_caps(src: &lime_source::LimeSource) -> DeviceCaps {
    use sdroxide_types::{Direction, GainElement, LimeConfig};
    let tx = src.tx_enabled();
    let mut gains = vec![
        // LimeSuite takes an unsigned number of decibels; anything finer is
        // truncated by the library, so offering finer would be a fiction.
        GainElement::db(
            LimeConfig::RX_GAIN_ELEMENT,
            Direction::Rx,
            LimeConfig::GAIN_MIN_DB,
            LimeConfig::GAIN_MAX_DB,
            1.0,
        ),
    ];
    if tx {
        gains.push(GainElement::db(
            LimeConfig::TX_GAIN_ELEMENT,
            Direction::Tx,
            LimeConfig::GAIN_MIN_DB,
            LimeConfig::GAIN_MAX_DB,
            1.0,
        ));
    }
    DeviceCaps {
        driver: "lime".into(),
        label: src.describe(),
        rx_channels: 1,
        tx_channels: usize::from(tx),
        full_duplex: true,
        audio_mode: false,
        freq_ranges_rx: src.freq_range(false).into_iter().collect(),
        freq_ranges_tx: if tx { src.freq_range(true).into_iter().collect() } else { Vec::new() },
        sample_rates: LimeConfig::SAMPLE_RATES.to_vec(),
        rate_ranges: src.rate_range().into_iter().collect(),
        gains,
        antennas_rx: src.antennas(false),
        antennas_tx: if tx { src.antennas(true) } else { Vec::new() },
        ..DeviceCaps::default()
    }
}

/// Build the SDRplay RSP source from radio.json. The device is picked by the
/// API's serial, or the first one found when none is configured.
fn open_sdrplay_source(
    radio: &RadioConfig,
    center_hz: f64,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let src = sdrplay_source::SdrPlaySource::open(&radio.sdrplay, center_hz)
        .context("opening SDRplay RSP")?;
    let caps = sdrplay_caps(&src);
    Ok((Box::new(src), caps))
}

/// Capabilities for an SDRplay RSP: wideband IQ, receive only, 1 kHz–2 GHz on
/// every model.
///
/// The two real gain elements come from [`SdrPlaySource::rx_gain_elements`],
/// which is also what the engine re-asks after a retune — the LNA ladder's
/// length belongs to the band, so this is a snapshot rather than a fact, and
/// one place has to own it. The switches (AGC, notches, bias tee, HDR) ride
/// pseudo-elements that are deliberately not listed here, so only the SDRplay
/// settings panel renders them.
fn sdrplay_caps(src: &sdrplay_source::SdrPlaySource) -> DeviceCaps {
    use sdroxide_types::SdrPlayConfig;
    DeviceCaps {
        driver: "sdrplay".into(),
        label: src.describe(),
        rx_channels: 1,
        tx_channels: 0,
        audio_mode: false,
        freq_ranges_rx: vec![(1_000.0, 2_000_000_000.0)],
        // With both RSPduo tuners running the API fixes the ADC clock and
        // downconverts from a low IF, which leaves a much shorter ladder —
        // and publishing the long one would offer spans this session cannot
        // reach. True however the pair is being used: two radios sharing the
        // board are as bound by its one clock as one radio combining them.
        sample_rates: if src.dual_tuner() || src.split_tuner() {
            SdrPlayConfig::DUAL_SAMPLE_RATES.to_vec()
        } else {
            SdrPlayConfig::SAMPLE_RATES.to_vec()
        },
        gains: src.rx_gain_elements(),
        antennas_rx: src.antennas().to_vec(),
        // Two aerials arriving as one span: what puts the filter's controls on
        // the main strip rather than only in the settings dialog (issue #165).
        diversity: src.dual_tuner(),
        ..DeviceCaps::default()
    }
}

/// Build the PlutoSDR source from radio.json. The address is the operator's
/// typed one, else a persisted discovery selection, else the USB gadget's
/// default — see [`sdroxide_types::PlutoConfig::target`].
///
/// `rate_override` is `--rate`, which for this backend takes precedence over
/// the configured rate: on a headless install the command line is the part of
/// the configuration that lives in the unit file, and a flag that is accepted
/// and then quietly ignored is worse than one that does not exist.
fn open_pluto_source(
    radio: &RadioConfig,
    center_hz: f64,
    rate_override: Option<f64>,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let mut cfg = radio.pluto.clone();
    if let Some(rate) = rate_override.filter(|r| r.is_finite() && *r > 0.0) {
        if (rate - cfg.sample_rate_hz).abs() > 1.0 {
            tracing::info!(
                "PlutoSDR: --rate {:.3} Msps overrides the {:.3} Msps in radio.json",
                rate / 1e6,
                cfg.sample_rate_hz / 1e6
            );
        }
        cfg.sample_rate_hz = rate;
    }
    let address = cfg.target();
    let src = pluto_source::PlutoSource::open(&address, &cfg, center_hz)
        .with_context(|| format!("opening PlutoSDR at {address}"))?;
    let caps = pluto_caps(&src, cfg.rx);
    Ok((Box::new(src), caps))
}

/// Capabilities for a PlutoSDR: wideband IQ, transmit-capable, half duplex
/// unless the operator has said their link carries both directions.
///
/// Everything here except the duplex flag is read off the device rather than
/// written down, because the two boards this backend serves genuinely differ: a
/// stock AD9363 covers 325 MHz–3.8 GHz and one unlocked to AD9364 covers
/// 70 MHz–6 GHz, and the receive gain range moves with frequency. Quoting
/// either set of numbers as a constant would leave half the Plutos in
/// circulation refusing frequencies they can reach.
///
/// The duplex flag is the exception because it is not a fact about the board.
/// The AD9361 *is* a full-duplex part; what cannot carry both directions at a
/// megasample per second is the USB 2.0 Ethernet gadget a Pluto is normally
/// reached over. So the default stands receive down for the length of an over,
/// and `PlutoConfig::full_duplex` — set by whoever can see the network the
/// radio is on — lifts it.
fn pluto_caps(src: &pluto_source::PlutoSource, rx: u8) -> DeviceCaps {
    use sdroxide_types::{Direction, GainElement, PlutoConfig};
    let Some(limits) = src.limits() else {
        return DeviceCaps { driver: "pluto".into(), label: src.describe(), ..Default::default() };
    };
    // The transmitter belongs to the chain-0 radio — the device has one DUC
    // path wired here.
    let tx_capable = rx == 0;
    let mut gains = vec![GainElement::db(
        PlutoConfig::RF_GAIN_ELEMENT,
        Direction::Rx,
        limits.rx_gain_db.0,
        limits.rx_gain_db.1,
        limits.rx_gain_db.2,
    )];
    if tx_capable {
        // Transmit "gain" is the AD9361's attenuator, so this range is
        // negative: 0 dB is full output.
        gains.push(GainElement::db(
            PlutoConfig::TX_GAIN_ELEMENT,
            Direction::Tx,
            limits.tx_gain_db.0,
            limits.tx_gain_db.1,
            limits.tx_gain_db.2,
        ));
    }
    DeviceCaps {
        driver: "pluto".into(),
        label: src.describe(),
        rx_channels: 1,
        tx_channels: if tx_capable { 1 } else { 0 },
        // Only the chain that owns the transmitter has an over to receive
        // through; a second chain's radio never keys and is never stood down.
        full_duplex: tx_capable && src.full_duplex(),
        audio_mode: false,
        freq_ranges_rx: vec![limits.rx_lo_hz],
        freq_ranges_tx: if tx_capable { vec![limits.tx_lo_hz] } else { Vec::new() },
        sample_rates: PlutoConfig::SAMPLE_RATES.to_vec(),
        rate_ranges: vec![limits.sample_rate_hz],
        gains,
        antennas_rx: limits.rx_ports.clone(),
        antennas_tx: if tx_capable { limits.tx_ports.clone() } else { Vec::new() },
        // On a 2R2T firmware the chains share the one synthesiser, so a
        // sibling radio's retune moves this radio's centre too (and vice
        // versa). Stated only when a sibling is possible.
        shared_lo_rx: src.rx_chains() > 1,
        ..DeviceCaps::default()
    }
}

/// The receive ranges an RTL-SDR publishes, given what its front end can do.
///
/// A Blog V4 upconverts HF in hardware, so it is continuous from DC. Anything
/// else reaches HF only by sampling the ADC directly, which is good for
/// everything below the ADC's own clock — the first Nyquist zone up to
/// 14.4 MHz, and its second zone above that, which the DDC's frequency word
/// wraps into on its own. See `sdroxide_rtlsdr::DIRECT_SAMPLING_TOP_HZ`.
///
/// That upper half is what this used to stop short of, and stopping short of it
/// greyed out 17 m and 15 m — the two bands with nowhere else to go, above the
/// ADC's Nyquist limit and below the tuner's floor (issue #179). The two ranges
/// overlap by design and that is fine: `DeviceCaps::can_rx_hz` is an `any` over
/// the list.
///
/// Split out from [`rtlsdr_caps`] so the front ends can be checked without a
/// dongle on the bus.
fn rtlsdr_rx_ranges(is_blog_v4: bool, hf_capable: bool) -> Vec<(f64, f64)> {
    let tuner = (sdroxide_rtlsdr::TUNER_MIN_HZ, sdroxide_rtlsdr::TUNER_MAX_HZ);
    if is_blog_v4 {
        vec![(0.0, sdroxide_rtlsdr::TUNER_MAX_HZ)]
    } else if hf_capable {
        vec![(0.0, sdroxide_rtlsdr::DIRECT_SAMPLING_TOP_HZ), tuner]
    } else {
        vec![tuner]
    }
}

/// Capabilities for an RTL-SDR: wideband IQ, receive only.
///
/// `driver` distinguishes the two ways in — `rtlsdr` over USB, `rtltcp` over
/// the network — because it is what names the interface in the UI and what a
/// radio tab is called before the operator names it. The capabilities
/// themselves are the same: it is the same dongle either way, and the sample
/// rate and HF answers come from the source, which knows which link it is on.
fn rtlsdr_caps(src: &rtlsdr_source::RtlSdrSource, driver: &str) -> DeviceCaps {
    let rate = src.sample_rate_hz();
    let freq_ranges_rx = rtlsdr_rx_ranges(src.is_blog_v4(), src.hf_capable());
    DeviceCaps {
        driver: driver.into(),
        label: format!("{} ({}, {:.3} Msps)", src.describe(), src.tuner(), rate / 1e6),
        rx_channels: 1,
        tx_channels: 0,
        audio_mode: false,
        freq_ranges_rx,
        sample_rates: sdroxide_types::RtlSdrConfig::SAMPLE_RATES.to_vec(),
        gains: vec![
            // The hardware only has 29 discrete steps; a request is snapped to
            // the nearest and reported back, so a fine slider is honest enough.
            sdroxide_types::GainElement::db(
                sdroxide_types::RtlSdrConfig::TUNER_GAIN_ELEMENT,
                sdroxide_types::Direction::Rx,
                0.0,
                sdroxide_types::RtlSdrConfig::GAIN_MAX_DB,
                0.1,
            ),
        ],
        ..DeviceCaps::default()
    }
}

/// Build the wideband SpyServer source from radio.json.
fn open_spyserver_source(
    radio: &RadioConfig,
    center_hz: f64,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let src = spyserver_source::SpyServerSource::connect_wideband(&radio.spyserver, center_hz)
        .with_context(|| {
            format!("connecting to the SpyServer at {}", radio.spyserver.endpoint())
        })?;
    let caps = spyserver_caps(&src, "spyserver");
    Ok((Box::new(src), caps))
}

/// The same server in its low-bandwidth shape: a narrow I/Q window that
/// follows the dial, and the server's FFT for the full-band strip.
fn open_spyserver_vfo_source(
    radio: &RadioConfig,
    center_hz: f64,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let src = spyserver_source::SpyServerSource::connect_vfo(&radio.spyserver_vfo, center_hz)
        .with_context(|| {
            format!("connecting to the SpyServer at {}", radio.spyserver_vfo.endpoint())
        })?;
    let caps = spyserver_caps(&src, "spyserver-vfo");
    Ok((Box::new(src), caps))
}

/// Capabilities for a SpyServer: wideband or narrowband I/Q, receive only.
///
/// Unlike `rtl_tcp`, this protocol answers, so almost everything here is what
/// the far end actually said rather than what was asked for. Two things still
/// depend on *who owns the receiver*, and both matter:
///
/// The tuning range is the whole device only while this client has control.
/// When another client owns it, all this end may do is slide its own window
/// inside the slice that client is receiving, and publishing the device's full
/// range would offer frequencies that reach nothing.
///
/// The gain is the owner's, not ours, on such a server — so no gain element is
/// published at all. A slider that is silently ignored is worse than no slider.
///
/// Both are read once here, at open. A device centre the owner moves later
/// leaves this range stale, which is why the refusal in
/// `SpyServerSource::set_center_hz` reads the live figures instead: the caps
/// only decide what the UI *offers*.
fn spyserver_caps(src: &spyserver_source::SpyServerSource, driver: &str) -> DeviceCaps {
    let info = *src.info();
    let (lo, hi) = src.tuning_window();
    let gains = if info.maximum_gain_index == 0 || !src.can_control() {
        Vec::new()
    } else {
        vec![
            // An index into the far end's gain table, carried in a field named
            // for decibels because `GainElement` has no other — the same thing
            // the SDRplay backend's LNA state already does. What an index means
            // depends on the receiver and on the band, so no dB mapping is
            // invented; the settings tab says so instead.
            sdroxide_types::GainElement::db(
                sdroxide_types::SpyServerConfig::GAIN_ELEMENT,
                sdroxide_types::Direction::Rx,
                0.0,
                f64::from(info.maximum_gain_index),
                1.0,
            ),
        ]
    };
    DeviceCaps {
        driver: driver.into(),
        label: src.describe(),
        rx_channels: 1,
        tx_channels: 0,
        audio_mode: false,
        freq_ranges_rx: vec![(lo, hi)],
        sample_rates: src.available_rates(),
        gains,
        // Another client's retunes move this receiver out from under us, and
        // the engine has to adopt such moves rather than answer them — which is
        // exactly what this flag means.
        shared_lo_rx: !src.can_control(),
        ..DeviceCaps::default()
    }
}

/// Build the KiwiSDR source: a ~12 kHz I/Q window that follows the dial, plus
/// the receiver's own 0-30 MHz waterfall for the full-band strip.
fn open_kiwisdr_source(
    radio: &RadioConfig,
    center_hz: f64,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    // Who to announce ourselves as. Resolved here rather than in the driver
    // crate, which has no business reading the station configuration — and it
    // is the station callsign rather than the radio's, because it is the
    // operator the receiver's owner sees.
    let ident = radio.kiwi.ident_or(&sdroxide_config::load_digi_config().my_call);
    let src = kiwisdr_source::KiwiSdrSource::connect(&radio.kiwi, &ident, center_hz)
        .with_context(|| format!("connecting to the KiwiSDR at {}", radio.kiwi.endpoint()))?;
    let caps = kiwisdr_caps(&src);
    Ok((Box::new(src), caps))
}

/// Capabilities for a KiwiSDR: one narrow I/Q channel, receive only.
///
/// The tuning range comes from the receiver's own opening burst rather than
/// from the directory listing, because a privately-run Kiwi is in no listing —
/// and because the two disagree often enough to matter: the listing carries
/// what its operator typed, the receiver carries what it is actually set to.
///
/// One sample rate, and it is not a round number. The receiver states its own
/// (11998.876765 Hz on the one this was measured against) and there is nothing
/// to choose: a Kiwi's user channel is the width it is.
///
/// The gain elements are the receiver's AGC and its manual gain. Both are on
/// the far side of the link and ahead of the I/Q, which is why the S-meter is
/// read from the frames instead — see `kiwisdr_source`.
fn kiwisdr_caps(src: &kiwisdr_source::KiwiSdrSource) -> DeviceCaps {
    let (lo, hi) = src.tuning_range();
    DeviceCaps {
        driver: "kiwisdr".into(),
        label: src.describe(),
        rx_channels: 1,
        tx_channels: 0,
        audio_mode: false,
        freq_ranges_rx: vec![(lo, hi)],
        sample_rates: vec![src.sample_rate()],
        gains: vec![
            sdroxide_types::GainElement::db(
                sdroxide_types::KiwiConfig::AGC_ELEMENT,
                sdroxide_types::Direction::Rx,
                0.0,
                1.0,
                1.0,
            ),
            // The receiver's own scale, carried in a field named for
            // decibels because `GainElement` has no other - the same thing
            // the SpyServer's gain index and the SDRplay's LNA state do.
            sdroxide_types::GainElement::db(
                sdroxide_types::KiwiConfig::MAN_GAIN_ELEMENT,
                sdroxide_types::Direction::Rx,
                0.0,
                90.0,
                1.0,
            ),
        ],
        ..DeviceCaps::default()
    }
}

/// Build the TCI (WebSocket) source from radio.json: wideband IQ receive +
/// audio transmit.
fn open_tci_source(
    radio: &RadioConfig,
    center_hz: f64,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let src = tci_source::TciSource::open(
        &radio.tci.address,
        radio.tci.iq_sample_rate_hz,
        center_hz,
        radio.tci.rx,
        radio.tci.stream_delay_ms,
    )
    .context("connecting to TCI server")?;
    let caps = tci_caps(&radio.tci.address, src.sample_rate_hz(), radio.tci.rx);
    Ok((Box::new(src), caps))
}

/// Capabilities for a TCI rig: wideband IQ RX (not `audio_mode`), TX via raw
/// audio (`tx_audio`) which the rig modulates. The rig enforces its own limits.
/// A secondary receiver (`rx > 0`) has no transmitter: TCI rigs have one, and
/// it belongs to the receiver-0 radio.
///
/// The transmit envelope covers 2 m as well as HF and 6 m. Every Expert
/// Electronics rig that speaks TCI and has a 2 m section — the SunSDR2 PRO and
/// DX, the MB1 — transmits there, and receive already reached 160 MHz, so
/// stopping transmit at 54 MHz left a 2 m-capable transceiver hearing the band
/// it was refusing to key up on. The upper edge is 148 MHz so that Regions 2
/// and 3 get their whole allocation; the amateur-band gate that runs straight
/// after this one is region-aware and holds a Region 1 station to 146 MHz,
/// and the rig declines anything it cannot do regardless.
fn tci_caps(address: &str, iq_rate: f64, rx: u32) -> DeviceCaps {
    let (label, tx_channels, tx_audio) = if rx == 0 {
        (format!("TCI {address} ({:.0} kHz IQ)", iq_rate / 1000.0), 1, true)
    } else {
        (format!("TCI RX{} {address} ({:.0} kHz IQ)", rx + 1, iq_rate / 1000.0), 0, false)
    };
    DeviceCaps {
        driver: "tci".into(),
        label,
        rx_channels: 1,
        tx_channels,
        audio_mode: false,
        tx_audio,
        freq_ranges_rx: vec![(0.0, 160_000_000.0)],
        freq_ranges_tx: if tx_channels > 0 {
            vec![(1_800_000.0, 54_000_000.0), (144_000_000.0, 148_000_000.0)]
        } else {
            Vec::new()
        },
        sample_rates: sdroxide_types::TciConfig::IQ_RATES.to_vec(),
        // No RX gains: the SunSDR2DX ATT/Preamp is not reachable over TCI
        // (verified against ExpertSDR3 — no command spelling drives it, and
        // toggling it in the GUI emits nothing on the wire). TCI gain control
        // is deferred until a controllable path is found.
        ..DeviceCaps::default()
    }
}

/// Build the Icom LAN source from radio.json.
///
/// The centre is not passed in: an Icom is the rig as well as the front end,
/// and the dial belongs to the radio. The source adopts whatever it is tuned
/// to, the way the CAT backend does — starting a session by retuning somebody's
/// transceiver to a frequency out of a config file is not the behaviour a
/// transceiver operator expects.
fn open_icomnet_source(radio: &RadioConfig) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let src = icomnet_source::IcomNetSource::open(&radio.icomnet)
        .context("connecting to the Icom over LAN")?;
    let caps = icomnet_caps(&radio.icomnet, &src);
    Ok((Box::new(src), caps))
}

/// Capabilities for an Icom over LAN.
///
/// `audio_mode` follows the receive path: demodulated audio takes the engine's
/// audio-band bypass, while the 12 kHz IF is ordinary complex baseband and does
/// not. `tx_audio` is set either way — transmit is always audio the radio
/// modulates, whatever the receive stream is carrying.
///
/// The frequency ranges are deliberately wide. This one backend covers rigs
/// from an HF-only IC-7300MK2 to an IC-905 at 10 GHz and a receive-only
/// IC-R8600, the protocol carries no band table, and an operator who wants the
/// dial held to their radio's real coverage can state it in Settings — which is
/// what `freq_ranges_rx` is for.
fn icomnet_caps(cfg: &IcomNetConfig, src: &icomnet_source::IcomNetSource) -> DeviceCaps {
    let audio_mode = cfg.effective_rx_source() == sdroxide_types::IcomRxSource::Af;
    let tx = src.can_transmit();
    DeviceCaps {
        driver: "icomnet".into(),
        label: src.describe(),
        rx_channels: 1,
        tx_channels: usize::from(tx),
        audio_mode,
        tx_audio: tx,
        freq_ranges_rx: vec![(30_000.0, 10_500_000_000.0)],
        freq_ranges_tx: if tx { vec![(1_800_000.0, 10_500_000_000.0)] } else { Vec::new() },
        has_swr_sensor: tx,
        ..DeviceCaps::default()
    }
}

/// Build the SmartSDR (FlexRadio) source from radio.json: DAX IQ receive +
/// DAX audio transmit over the LAN.
fn open_smartsdr_source(
    radio: &RadioConfig,
    center_hz: f64,
) -> anyhow::Result<(Box<dyn IqSource>, DeviceCaps)> {
    let src = smartsdr_source::SmartSdrSource::open(&radio.smartsdr, center_hz)
        .context("connecting to the FlexRadio")?;
    let caps = smartsdr_caps(src.model(), src.describe());
    Ok((Box::new(src), caps))
}

/// Capabilities for a FlexRadio: wideband IQ RX over DAX (not `audio_mode`), TX
/// via DAX audio (`tx_audio`) which the radio modulates.
///
/// The frequency ranges are the radio's own published coverage rather than
/// anything this backend imposes: a FLEX receives from 30 kHz to 54 MHz (the
/// 8000 series and the 6600/6700 add 2 m), and the radio declines anything it
/// cannot do. The sample rates are the four DAX IQ stream rates — 192 kHz is
/// the ceiling, which makes it this backend's widest span.
///
/// The transmit envelope follows the receive one onto 2 m on the models that
/// have a VHF section. A FLEX's own PA is HF and 6 m, but a transverter on the
/// XVTR port transmits at the band the slice is showing — 2 m being the usual
/// one — and this backend cannot see SmartSDR's transverter table to tell the
/// two cases apart. Refusing to key would break the transverter operator; the
/// radio refusing a transmit it cannot do costs nothing, which is the same
/// trade this function's receive ranges already make.
fn smartsdr_caps(model: &str, label: String) -> DeviceCaps {
    // The 6600/6700 and the whole 8000 family have a 2 m receiver; the rest stop
    // at 6 m. Getting this wrong only costs a refused tune, so infer it.
    let vhf = matches!(model, "FLEX-6600" | "FLEX-6600M" | "FLEX-6700" | "FLEX-6700R")
        || model.starts_with("FLEX-8");
    let rx_top = if vhf { 165_000_000.0 } else { 54_000_000.0 };
    let mut tx = vec![(1_800_000.0, 54_000_000.0)];
    if vhf {
        tx.push((144_000_000.0, 148_000_000.0));
    }
    DeviceCaps {
        driver: "smartsdr".into(),
        label,
        rx_channels: 1,
        tx_channels: 1,
        audio_mode: false,
        tx_audio: true,
        freq_ranges_rx: vec![(30_000.0, rx_top)],
        freq_ranges_tx: tx,
        sample_rates: sdroxide_types::SmartSdrConfig::IQ_RATES.to_vec(),
        // No RX gain elements: a FLEX has no user-settable front-end gain in the
        // sense this list means. Its `display pan rfgain` is a per-panadapter
        // preamp/attenuator whose steps differ by model and are only discoverable
        // by asking the radio (`display pan rfgain_info`), so it is left out
        // until that query can be verified against hardware.
        ..DeviceCaps::default()
    }
}

/// Capabilities for a CAT rig. TX-capable unless PTT is VOX-only-with-no-audio;
/// we advertise TX so the UI shows PTT and the safety rails apply. The rig
/// enforces its own limits over CAT.
///
/// The RX floor is deliberately below any rig's: the range is what the engine
/// refuses to tune past, and a general-coverage receiver that reaches the LF
/// time signals must not be held back by a figure invented here. A rig asked
/// for something it cannot do simply declines over CAT, which costs nothing.
///
/// The ceilings are above any rig's for exactly the same reason, and they used
/// to be well below several: this one backend covers rigs from an HF-only
/// FT-891 to an FT-991A on 70 cm, an IC-9700 on 23 cm and an IC-905 at 10 GHz,
/// and CAT carries no band table to tell them apart. A 148 MHz receive ceiling
/// greyed out the 70 cm button on a rig that has the band, and a 54 MHz
/// transmit ceiling refused to key a 2 m rig that was hearing the band
/// perfectly well. What holds a licensed operator in bounds is the
/// amateur-band gate — region-aware, and 70 cm is the highest band sdroxide's
/// table knows — plus the rig's own refusal. An operator who wants a firmer
/// limit than that can state one in Settings.
fn cat_caps(radio: &RadioConfig) -> DeviceCaps {
    let demod = matches!(radio.cat.format, sdroxide_types::SoundFormat::DemodAudio);
    DeviceCaps {
        driver: "cat".into(),
        label: format!("{} (CAT)", radio.cat.family.label()),
        rx_channels: 1,
        tx_channels: 1,
        audio_mode: demod,
        // Set whatever the receive stream carries, the same as the Icom LAN and
        // ELAD backends: a CAT rig modulates the audio we put into its sound
        // card, and there is no transmit path here that does anything else.
        // `audio_mode` covers only the demod-audio half of that, so a rig
        // sending quadrature keyed up, reached `tx_write` — which this source
        // does not implement, because it has no I/Q transmitter — and failed
        // the over with "device is not transmit capable" on a radio that plainly
        // can.
        tx_audio: true,
        freq_ranges_rx: vec![(10_000.0, 10_500_000_000.0)],
        freq_ranges_tx: vec![(1_800_000.0, 10_500_000_000.0)],
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

#[cfg(test)]
mod tests {
    use super::*;

    fn cli(oob: bool) -> Cli {
        let mut c = Cli::parse_from(["sdroxide"]);
        c.oob_tx = oob;
        c
    }

    /// `--oob-tx` may only ever *loosen* the lockout. A build that never passes
    /// it has to behave exactly as it did before the flag existed, and no
    /// combination of flags may turn the lockout on when the config has turned
    /// it off — that would be a surprise in the dangerous direction.
    #[test]
    fn the_flag_only_ever_loosens_the_band_lockout() {
        let locked = Settings { tx_ham_only: true, ..Settings::default() };
        let open = Settings { tx_ham_only: false, ..Settings::default() };

        assert!(cli(false).tx_ham_only(&locked), "the default must keep the lockout");
        assert!(!cli(true).tx_ham_only(&locked), "--oob-tx must lift it");
        // Already unlocked in the config: the flag changes nothing either way.
        assert!(!cli(false).tx_ham_only(&open));
        assert!(!cli(true).tx_ham_only(&open));
    }

    /// The flag is opt-in on the command line and nowhere else.
    #[test]
    fn the_flag_is_off_unless_asked_for() {
        assert!(!Cli::parse_from(["sdroxide"]).oob_tx);
        assert!(Cli::parse_from(["sdroxide", "--oob-tx"]).oob_tx);
        assert!(Settings::default().tx_ham_only, "the shipped default is locked");
    }

    /// A SunSDR2 DX hears 2 m and transmits on it, and it took an override in
    /// Settings to make sdroxide agree — the published transmit envelope
    /// stopped at 6 m. Both halves of the band matter: the HF/6 m block is what
    /// every TCI rig has, and 2 m is what the VHF ones add.
    #[test]
    fn a_tci_rig_may_transmit_on_two_metres() {
        let caps = tci_caps("127.0.0.1:40001", 312_000.0, 0);
        assert!(caps.may_tx_hz(14_200_000.0), "HF");
        assert!(caps.may_tx_hz(50_150_000.0), "6 m");
        assert!(caps.may_tx_hz(144_300_000.0), "2 m, Region 1");
        assert!(caps.may_tx_hz(146_520_000.0), "2 m, Regions 2 and 3");
        // The envelope is still an envelope: the gap between the bands is not
        // an amateur allocation anywhere, and neither is anything above 2 m.
        assert!(!caps.may_tx_hz(100_000_000.0), "broadcast FM");
        assert!(!caps.may_tx_hz(435_000_000.0), "70 cm — no TCI rig reaches it");

        // A second receiver still has no transmitter of its own.
        assert!(tci_caps("127.0.0.1:40001", 312_000.0, 1).freq_ranges_tx.is_empty());
    }

    /// A CAT rig is whatever the operator plugged in, and the envelope here is
    /// only what the engine refuses to tune past: the rig's own refusal and the
    /// amateur-band gate are the real limits. The ceilings used to sit below
    /// several shipping rigs.
    #[test]
    fn a_cat_rig_reaches_the_vhf_and_uhf_bands() {
        let caps = cat_caps(&RadioConfig::default());
        for hz in [145_500_000.0, 435_000_000.0, 1_296_000_000.0] {
            assert!(caps.may_rx_hz(hz), "receive at {:.3} MHz", hz / 1e6);
            assert!(caps.may_tx_hz(hz), "transmit at {:.3} MHz", hz / 1e6);
        }
        // Still an envelope at both ends: below the transmit floor and above
        // any rig this backend drives.
        assert!(caps.may_rx_hz(60_000.0), "LF time signals stay reachable");
        assert!(!caps.may_tx_hz(500_000.0));
        assert!(!caps.may_tx_hz(24_000_000_000.0));
    }

    /// A CAT rig transmits by feeding its sound card, whichever way its receive
    /// stream comes back. The demod-audio half of that rides on `audio_mode`,
    /// which is why the quadrature half went unnoticed: PTT on an I/Q rig sent
    /// the engine down the modulated-I/Q path to an `IqSource` that has no
    /// `tx_write`, and the over died with "device is not transmit capable".
    #[test]
    fn a_cat_rig_transmits_audio_in_either_sound_format() {
        use sdroxide_types::SoundFormat;
        for format in [SoundFormat::DemodAudio, SoundFormat::Iq] {
            let mut radio = RadioConfig::default();
            radio.cat.format = format;
            let caps = cat_caps(&radio);
            assert!(caps.is_transmit_capable(), "{format:?}");
            assert!(caps.tx_audio, "the rig modulates the audio we send it ({format:?})");
        }
        // And the receive path still follows the format, since that is the one
        // thing the two halves genuinely differ about.
        let mut radio = RadioConfig::default();
        radio.cat.format = SoundFormat::DemodAudio;
        assert!(cat_caps(&radio).audio_mode);
        radio.cat.format = SoundFormat::Iq;
        assert!(!cat_caps(&radio).audio_mode, "quadrature is ordinary wideband I/Q");
    }

    /// A FLEX with a VHF section may be showing a 2 m transverter's output on
    /// the slice, and the models without one are unchanged.
    #[test]
    fn a_vhf_flex_may_transmit_on_two_metres() {
        let vhf = smartsdr_caps("FLEX-6700", "FLEX-6700".into());
        assert!(vhf.may_rx_hz(145_500_000.0));
        assert!(vhf.may_tx_hz(145_500_000.0));
        assert!(vhf.may_tx_hz(14_200_000.0), "HF is untouched");

        let hf_only = smartsdr_caps("FLEX-6400", "FLEX-6400".into());
        assert!(!hf_only.may_rx_hz(145_500_000.0), "no VHF receiver, no 2 m");
        assert!(!hf_only.may_tx_hz(145_500_000.0));
        assert!(hf_only.may_tx_hz(50_150_000.0), "6 m is untouched");
    }

    /// Issue #179: on a dongle that reaches HF by direct sampling, 17 m and
    /// 15 m were the only two amateur bands with no published range — above the
    /// ADC's 14.4 MHz Nyquist limit, below the tuner's 24 MHz floor — so their
    /// band buttons were greyed out and the engine refused to tune there. The
    /// ADC hears them in its second Nyquist zone; every band from 160 m up must
    /// now be reachable.
    #[test]
    fn a_direct_sampling_dongle_reaches_every_hf_band() {
        let caps = |ranges: Vec<(f64, f64)>| DeviceCaps {
            freq_ranges_rx: ranges,
            ..DeviceCaps::default()
        };

        let ds = caps(rtlsdr_rx_ranges(false, true));
        for (band, hz) in [
            ("160 m", 1_840_000.0),
            ("40 m", 7_074_000.0),
            ("20 m", 14_074_000.0),
            ("17 m", 18_100_000.0),
            ("15 m", 21_074_000.0),
            ("12 m", 24_915_000.0),
            ("10 m", 28_074_000.0),
            ("2 m", 144_174_000.0),
        ] {
            assert!(ds.may_rx_hz(hz), "{band} on a direct-sampling dongle");
        }
        assert!(!ds.may_rx_hz(2_000_000_000.0), "still an envelope at the top");

        // A V4 upconverts and never had the gap; a dongle with HF switched off
        // still stops at the tuner's floor, which is the honest answer for it.
        let v4 = caps(rtlsdr_rx_ranges(true, true));
        assert!(v4.may_rx_hz(1_840_000.0) && v4.may_rx_hz(21_074_000.0));
        let tuner_only = caps(rtlsdr_rx_ranges(false, false));
        assert!(!tuner_only.may_rx_hz(21_074_000.0));
        assert!(tuner_only.may_rx_hz(28_074_000.0), "10 m is the tuner's own");
    }

    fn session(freq_hz: f64, mode: sdroxide_types::Mode) -> sdroxide_config::Session {
        sdroxide_config::Session { freq_hz, mode, ..Default::default() }
    }

    /// Starting with no arguments is the case this exists for: come back up on
    /// the frequency and mode the radio was left on, not on a fixed default.
    #[test]
    fn a_bare_start_comes_up_where_the_last_session_ended() {
        let mut c = Cli::parse_from(["sdroxide"]);
        let mode = c.apply_session(session(7_074_000.0, sdroxide_types::Mode::Ft8));
        assert_eq!(c.center_hz(), 7_074_000.0);
        assert_eq!(mode, Some(sdroxide_types::Mode::Ft8));
        assert_eq!(c.mode, mode, "the engine and the source agree on the mode");
    }

    /// A radio put down on VFO B has to be picked up on VFO B: the front end is
    /// opened on the dial that was actually being listened to, not on A's.
    #[test]
    fn a_start_on_vfo_b_opens_on_bs_dial() {
        let left_on_b = sdroxide_config::Session {
            vfo_b_hz: Some(7_100_000.0),
            active_vfo: sdroxide_types::Vfo::B,
            ..session(14_200_000.0, sdroxide_types::Mode::Usb)
        };
        let mut c = Cli::parse_from(["sdroxide"]);
        c.apply_session(left_on_b);
        assert_eq!(c.center_hz(), 7_100_000.0);
    }

    /// The command line is an instruction, not a suggestion: what it names must
    /// survive the restore, and only what it left out may be filled in.
    #[test]
    fn the_command_line_outranks_the_remembered_session() {
        let mut both = Cli::parse_from(["sdroxide", "--freq", "50150000", "--mode", "cw"]);
        both.apply_session(session(7_074_000.0, sdroxide_types::Mode::Ft8));
        assert_eq!(both.center_hz(), 50_150_000.0);
        assert_eq!(both.mode, Some(sdroxide_types::Mode::Cw));

        // Each half is independent — naming one must not discard the other.
        let mut freq_only = Cli::parse_from(["sdroxide", "--freq", "50150000"]);
        freq_only.apply_session(session(7_074_000.0, sdroxide_types::Mode::Ft8));
        assert_eq!(freq_only.center_hz(), 50_150_000.0);
        assert_eq!(freq_only.mode, Some(sdroxide_types::Mode::Ft8), "the mode is still restored");

        let mut mode_only = Cli::parse_from(["sdroxide", "--mode", "cw"]);
        mode_only.apply_session(session(7_074_000.0, sdroxide_types::Mode::Ft8));
        assert_eq!(mode_only.center_hz(), 7_074_000.0, "the frequency is still restored");
        assert_eq!(mode_only.mode, Some(sdroxide_types::Mode::Cw));
    }

    /// A run that never restores anything — the headless smoke tests — still
    /// has to open a front end somewhere, on the frequency this program has
    /// always defaulted to.
    /// The on/off switch, from the side that decides what actually opens: the
    /// interface factory. Redirects the config directory through the
    /// environment — process-global state, so it is one test, and no other test
    /// in this binary reads a config file.
    #[test]
    fn the_switch_decides_what_the_interface_factory_opens() {
        let root = std::env::temp_dir().join(format!("sdroxide-power-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("radio-1")).expect("scratch dir");
        std::fs::write(
            root.join("radios.json"),
            r#"{"radios":[{"id":0,"name":""},{"id":1,"name":"","enabled":false}],"next_id":2}"#,
        )
        .unwrap();
        // No interface at all, so an attempt to open this radio always fails —
        // which is exactly the case that has to come back retrying.
        std::fs::write(root.join("radio-1/radio.json"), r#"{"backend":"None"}"#).unwrap();
        // SAFETY: single-threaded within this test; no other test in this
        // binary reads the variable.
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };

        let cli = secondary_cli(&Cli::parse_from(["sdroxide"]));
        let mut factory = reopen_factory_for(&cli, sdroxide_config::Store::radio(1), 1);

        // Switched off: an answer, not a refusal — a refusal would leave the
        // engine holding whatever it already had open — and one that has
        // stopped asking to be reopened.
        let (source, caps) = factory(14_200_000.0).expect("a radio that is off still answers");
        assert_eq!(caps.label, "Switched off");
        assert!(!source.needs_reopen(), "a radio that is off is not a radio waiting to come back");

        // Switched on with nothing there to open: the ordinary reconnecting
        // stand-in, never a refusal, or the engine would sit for ever on the
        // stand-in above — which does not retry.
        sdroxide_config::set_radio_enabled(1, true).unwrap();
        let (source, caps) = factory(14_200_000.0).expect("switching on must not leave a corpse");
        assert_eq!(caps.label, "No radio");
        assert!(source.needs_reopen(), "the engine has to keep trying from here");

        // And from then on it is an ordinary interface change again: a failure
        // is reported as one, leaving whatever is running running.
        assert!(factory(14_200_000.0).is_err(), "no longer coming back from off");

        unsafe { std::env::remove_var("SDROXIDE_CONFIG_DIR") };
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_run_that_skips_the_restore_keeps_the_old_default() {
        assert_eq!(Cli::parse_from(["sdroxide"]).center_hz(), 14_200_000.0);
        assert_eq!(Cli::parse_from(["sdroxide", "--freq", "50150000"]).center_hz(), 50_150_000.0);
    }
}
