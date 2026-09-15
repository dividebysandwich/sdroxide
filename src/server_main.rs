//! Headless server mode: one engine per radio in the station roster, behind a
//! single WebSocket/HTTP frontend.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use sdroxide_config::Settings;
use sdroxide_radio::rtrb;
use sdroxide_radio::{
    AudioParams, EngineConfig, IqSource, MicParams, RadeWatch, StoreSync, TrSwitch, TxGate,
    start_engine,
};
use sdroxide_server::{RadioParams, ServerParams};

use crate::RadioBoot;

pub fn run(
    // The station's radios, in roster order, already opened by `main` — the
    // same list the GUI would have put in tabs. The first is what a client
    // that names no radio gets.
    radios: Vec<RadioBoot>,
    settings: &Settings,
    // Whether the engines refuse to key outside the amateur bands. Resolved in
    // `main` from `config.toml` and the `--oob-tx` flag.
    tx_ham_only: bool,
    port: u16,
    web_root: Option<PathBuf>,
    // A sanitized command line (radio-0 overrides stripped) for radios a client
    // adds while the server is running — the same one the GUI's "+" chip uses.
    factory_cli: crate::Cli,
) -> Result<()> {
    // Shared by every engine in this process, exactly as in the GUI: one
    // transmitter on the air at a time, and a store change made through one
    // radio seen by the rest. A server needs the interlock more than the shack
    // does, not less — nobody is sitting here to notice two radios keying.
    let gate = Arc::new(TxGate::new());
    let sync = Arc::new(StoreSync::new());
    let rade = Arc::new(RadeWatch::new());
    // The external T/R switch, shared for the same reason — and needed here
    // more than in the shack, not less: nobody is standing next to the antenna
    // relay to hear whether it threw.
    let tr = Arc::new(TrSwitch::new());

    let mut params = Vec::with_capacity(radios.len());
    for (i, boot) in radios.into_iter().enumerate() {
        // Demod audio ring (engine → server, interleaved stereo @48 k) and mic
        // ring (server → engine, mono @48 k), one pair per radio.
        let (audio_producer, audio_consumer) = rtrb::RingBuffer::<f32>::new(48_000 * 2);
        let (mic_producer, mic_consumer) = rtrb::RingBuffer::<f32>::new(48_000);

        let handles = start_engine(
            boot.source,
            boot.caps,
            EngineConfig {
                audio: Some(AudioParams { producer: audio_producer, out_rate: 48_000.0 }),
                mic: Some(MicParams { consumer: mic_consumer, rate: 48_000.0 }),
                cal_offset_db: settings.cal_offset_db as f32,
                initial_mode: boot.initial_mode,
                initial_antenna: boot.initial_antenna,
                tx_ham_only,
                cb_tx_allowed: settings.cb_tx_allowed,
                // The SWR guard matters MORE headless, not less: there is no
                // operator watching the meter, so an unattended beacon or a
                // remote client would otherwise keep transmitting into a fault
                // indefinitely. The latch is cleared by a connected client
                // acknowledging it.
                swr_guard: settings.swr_guard,
                swr_limit: settings.swr_limit,
                // A headless server is typically started before the rig it
                // talks to; the engine uses this to attach as soon as the
                // radio is there.
                reopen: boot.reopen,
                // The server *is* the radio for everyone connected to it, so it
                // is the side that remembers where the last session was left —
                // each radio in its own scope.
                remember_session: true,
                store: boot.store,
                instance: boot.id,
                record_iq: boot.record_iq.clone(),
                // Exactly one engine runs the station-wide network services,
                // for the same reason as in the GUI: they hold logins and
                // sockets that must not be opened once per radio.
                primary: i == 0,
                tx_gate: Some(gate.clone()),
                store_sync: Some(sync.clone()),
                rade_watch: Some(rade.clone()),
                tr_switch: Some(tr.clone()),
            },
        );

        params.push(RadioParams {
            id: boot.id,
            name: boot.name,
            cmd_tx: handles.cmd_tx,
            event_rx: handles.event_rx,
            spectrum_out: handles.spectrum_out,
            wide_spectrum_out: handles.wide_spectrum_out,
            audio_rx: audio_consumer,
            mic_tx: mic_producer,
        });
    }

    // A radio added by a client while this server is running. The same steps
    // the GUI's "+" chip takes, and for the same reason it takes them: the
    // scope on disk first, then an engine on the stand-in source, because a
    // new radio has no interface yet (`Backend::None`) and the open is
    // *meant* to fail. What it is comes next, from the client's Radio settings
    // page, which already reaches this machine's `radio.json`.
    //
    // Without this, a station with no screen could only gain a radio by
    // editing `radios.json` on the server and restarting it — dropping
    // everyone on the air to add a dongle.
    let add_gate = gate.clone();
    let add_sync = sync.clone();
    let add_rade = rade.clone();
    let add_tr = tr.clone();
    let add_radio: sdroxide_server::AddRadioFn = Box::new(move |name: &str| {
        let slot = sdroxide_config::create_radio(name).map_err(|e| e.to_string())?;
        // Read fresh: this is minutes or days after startup, and the operator
        // may have changed the station's settings from a client since.
        let settings = Settings::load();
        let store = sdroxide_config::Store::radio(slot.id);
        let mut c = factory_cli.clone();
        let initial_mode = c.apply_session(store.load_session());
        let (source, caps) =
            match crate::open_converted_source(&store.load_radio_config(), &c, &settings) {
                Ok(pair) => pair,
                Err(e) => (
                    Box::new(crate::null_source::NullSource::new(c.center_hz(), format!("{e}")))
                        as Box<dyn IqSource>,
                    crate::synthetic_caps("No radio"),
                ),
            };
        let (audio_producer, audio_consumer) = rtrb::RingBuffer::<f32>::new(48_000 * 2);
        let (mic_producer, mic_consumer) = rtrb::RingBuffer::<f32>::new(48_000);
        let handles = start_engine(
            source,
            caps,
            EngineConfig {
                audio: Some(AudioParams { producer: audio_producer, out_rate: 48_000.0 }),
                mic: Some(MicParams { consumer: mic_consumer, rate: 48_000.0 }),
                cal_offset_db: settings.cal_offset_db as f32,
                initial_mode,
                initial_antenna: (None, None),
                tx_ham_only,
                cb_tx_allowed: settings.cb_tx_allowed,
                swr_guard: settings.swr_guard,
                swr_limit: settings.swr_limit,
                reopen: Some(crate::reopen_factory_for(&c, store.clone(), slot.id)),
                remember_session: true,
                store,
                instance: slot.id,
                // A radio added at runtime is never the capture target
                // (`--record-iq` names one file), and never the primary: the
                // station's network services belong to the radio that was
                // already running them.
                record_iq: None,
                primary: false,
                tx_gate: Some(add_gate.clone()),
                store_sync: Some(add_sync.clone()),
                rade_watch: Some(add_rade.clone()),
                tr_switch: Some(add_tr.clone()),
            },
        );
        Ok(RadioParams {
            id: slot.id,
            name: slot.name,
            cmd_tx: handles.cmd_tx,
            event_rx: handles.event_rx,
            spectrum_out: handles.spectrum_out,
            wide_spectrum_out: handles.wide_spectrum_out,
            audio_rx: audio_consumer,
            mic_tx: mic_producer,
        })
    });

    sdroxide_server::run_blocking(ServerParams {
        radios: params,
        bind: settings.server_bind.clone(),
        port,
        web_root,
        // Re-read per connection rather than captured from `settings`: the
        // credentials are a file on this machine, and an operator who changes
        // their password — by hand, or from the settings dialog of the GUI
        // running beside this server — should not have to restart the server
        // and drop whoever is on it for the change to hold.
        access: Some(Box::new(sdroxide_config::load_remote_access)),
        // The same enumeration the local settings dialog uses, offered to
        // whoever is connected. Without it the Rescan / Discover / Test buttons
        // on a remote or browser client have nothing to answer them, and a
        // headless station's radio could only be changed by editing
        // `radio.json` on this machine and restarting.
        probe: Some(Box::new(crate::devices::probe)),
        add_radio: Some(add_radio),
        // The roster file only. The engine stops by itself once the server
        // lets go of its command channel, and the radio's own configuration
        // scope stays on disk — closing a radio is not a request to destroy
        // what it was set up as, here any more than at the station.
        remove_radio: Some(Box::new(|id| {
            sdroxide_config::remove_radio(id).map_err(|e| e.to_string())
        })),
        rename_radio: Some(Box::new(|id, name| {
            sdroxide_config::rename_radio(id, name).map_err(|e| e.to_string())
        })),
        // The roster file again, in both directions. The switch has to live
        // there and nowhere else: the factory that opens a radio's interface
        // reads the same file, so a client throwing this and the engine
        // rebuilding on it are looking at one answer. Without it a headless
        // station's radio could only be put down by stopping the whole server
        // — dropping everyone on the air to free one dongle.
        radio_power: Some(Box::new(|id, set| {
            if let Some(on) = set {
                sdroxide_config::set_radio_enabled(id, on).map_err(|e| e.to_string())?;
            }
            Ok(sdroxide_config::load_radios().is_enabled(id))
        })),
    })?;
    Ok(())
}
