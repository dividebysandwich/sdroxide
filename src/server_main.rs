//! Headless server mode: engine + WebSocket/HTTP frontend.

use std::path::PathBuf;

use anyhow::Result;
use sdroxide_config::Settings;
use sdroxide_radio::rtrb;
use sdroxide_radio::{AudioParams, EngineConfig, IqSource, MicParams, start_engine};
use sdroxide_server::ServerParams;
use sdroxide_types::{DeviceCaps, Mode};

pub fn run(
    source: Box<dyn IqSource>,
    caps: DeviceCaps,
    settings: &Settings,
    initial_mode: Option<Mode>,
    port: u16,
    web_root: Option<PathBuf>,
) -> Result<()> {
    // Demod audio ring (engine → server, interleaved stereo @48 k) and mic
    // ring (server → engine, mono @48 k).
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
            tx_ham_only: settings.tx_ham_only,
            reopen: None,
        },
    );

    sdroxide_server::run_blocking(ServerParams {
        cmd_tx: handles.cmd_tx,
        event_rx: handles.event_rx,
        spectrum_out: handles.spectrum_out,
        audio_rx: audio_consumer,
        mic_tx: mic_producer,
        bind: settings.server_bind.clone(),
        port,
        web_root,
    })?;
    Ok(())
}
