//! The engine host's interface configuration, over a real socket.
//!
//! This is the whole of what makes an SDR's own settings reachable from another
//! machine: the engine announces `radio.json`, the server keeps it and replays
//! it to whoever connects, and an edit sent back is written where the radio is.
//! An RTL-SDR's AGC mode, its ppm correction and its bias tee are not gain
//! stages and have no `DeviceCaps` entry to ride — this lane is the only route
//! they have.
//!
//! The last part of the test is the same lane carrying the biggest change of
//! all: *which interface* the far end has open. On a headless server there is
//! nobody standing at the machine to pick one, so it has to be possible from
//! here, and the radio that comes back has to be announced.
//!
//! Its own test binary because it redirects the config directory through the
//! environment, which is process-global: the tests in `session.rs` read that
//! directory, and must not find it moved out from under them.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use sdroxide_proto::{AudioCaps, ClientMsg, PROTO_VERSION, ServerMsg, decode, encode};
use sdroxide_radio::{EngineConfig, SigGenSource, start_engine};
use sdroxide_server::{RadioParams, ServerParams, serve};
use sdroxide_types::{
    Backend, Command, DeviceCaps, RadioConfig, RtlSdrAgc, RtlSdrConfig, RtlSdrHfMode,
};

const PORT: u16 = 39473;

async fn recv_msg(
    ws: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
) -> ServerMsg {
    loop {
        let m = tokio::time::timeout(Duration::from_secs(15), ws.next())
            .await
            .expect("timeout waiting for server message")
            .expect("stream ended")
            .expect("ws error");
        if let Message::Binary(bytes) = m {
            return decode::<ServerMsg>(&bytes).expect("decode");
        }
    }
}

/// Wait for the next `RadioConfig`, ignoring the streams running underneath it.
async fn next_radio_config(
    ws: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
) -> RadioConfig {
    loop {
        if let ServerMsg::RadioConfig(c) = recv_msg(ws).await {
            return *c;
        }
    }
}

async fn send(
    ws: &mut (impl SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin),
    m: &ClientMsg,
) {
    ws.send(Message::Binary(encode(m).unwrap().into())).await.unwrap();
}

/// What a dongle on the far end is configured to do. Deliberately not the
/// defaults: a message that arrived empty, or one whose fields had shifted,
/// would still look plausible against `RadioConfig::default()`.
fn far_end_dongle() -> RadioConfig {
    RadioConfig {
        backend: Backend::RtlSdr,
        rtlsdr: RtlSdrConfig {
            serial: Some("00000042".into()),
            sample_rate_hz: 1_536_000.0,
            ppm: -11,
            tuner_gain_db: 22.5,
            agc: RtlSdrAgc::Manual,
            hf_mode: RtlSdrHfMode::Auto,
            bias_tee: false,
            iq_correction: true,
            ..RtlSdrConfig::default()
        },
        ..RadioConfig::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_servers_interface_is_replayed_editable_and_switchable_from_a_client() {
    let root = std::env::temp_dir().join(format!("sdroxide-radiocfg-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("scratch config dir");
    // SAFETY: set before anything reads it, and this binary holds one test.
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };

    // Seeded through the same store the engine reads it with: a hand-rolled
    // file could pass this test against a shape the engine cannot load.
    let store = sdroxide_config::Store::station();
    let stored = far_end_dongle();
    store.save_radio_config(&stored).expect("seed radio.json");

    // The source is a signal generator rather than a dongle — this lane carries
    // the *configuration*, and does not care what the front end turned out to
    // be. What matters is that the engine's store is the scratch directory.
    // What the engine builds when a client asks it to reopen: the binary's
    // factory reads `radio.json` and opens whatever backend it names, and this
    // stands in for that, reporting the backend it was asked for as the label.
    // That is what makes the assertion at the end of this test possible — the
    // reopen is otherwise invisible from a socket.
    let factory_store = store.clone();
    let reopen: sdroxide_radio::ReopenFn = Box::new(move |center: f64| {
        let backend = factory_store.load_radio_config().backend;
        Ok((
            Box::new(SigGenSource::demo(1_536_000.0, center)) as Box<dyn sdroxide_radio::IqSource>,
            DeviceCaps {
                driver: "siggen".into(),
                label: format!("reopened on {backend:?}"),
                rx_channels: 1,
                freq_ranges_rx: vec![(0.0, 6e9)],
                ..DeviceCaps::default()
            },
        ))
    });
    let handles = start_engine(
        Box::new(SigGenSource::demo(1_536_000.0, 14_200_000.0)),
        DeviceCaps {
            driver: "siggen".into(),
            label: "Test signal generator".into(),
            rx_channels: 1,
            freq_ranges_rx: vec![(0.0, 6e9)],
            ..DeviceCaps::default()
        },
        EngineConfig { reopen: Some(reopen), ..EngineConfig::default() },
    );
    tokio::spawn(serve(ServerParams {
        radios: vec![RadioParams {
            id: 0,
            name: String::new(),
            cmd_tx: handles.cmd_tx,
            event_rx: handles.event_rx,
            spectrum_out: handles.spectrum_out,
            wide_spectrum_out: handles.wide_spectrum_out,
            audio_rx: sdroxide_radio::rtrb::RingBuffer::<f32>::new(96_000).1,
            mic_tx: sdroxide_radio::rtrb::RingBuffer::<f32>::new(48_000).0,
        }],
        bind: "127.0.0.1".into(),
        port: PORT,
        web_root: None,
        access: None,
        // Device questions have their own test binary (`probe.rs`); this one
        // is about the configuration, and answers none.
        probe: None,
        add_radio: None,
        remove_radio: None,
        rename_radio: None,
        radio_power: None,
        remote_clock_set: None,
    }));
    // Long enough that the engine's one startup announcement is already behind
    // us when the client arrives — which is the case the replay exists for.
    tokio::time::sleep(Duration::from_millis(800)).await;

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{PORT}/ws"))
        .await
        .expect("connect");
    send(
        &mut ws,
        &ClientMsg::Hello {
            proto: PROTO_VERSION,
            audio: AudioCaps { opus_decode: false, opus_encode: false },
        },
    )
    .await;

    // On connect: what the far end is set to, not what this machine's defaults
    // are. Without the replay the Radio tab would open on those defaults and
    // write them back over the operator's the moment anything was touched.
    let announced = next_radio_config(&mut ws).await;
    assert_eq!(announced, stored, "the connect-time replay did not carry the stored config");

    // ...and the settings that have no `DeviceCaps` entry survived the trip.
    // These are the ones a remote operator could not reach at all before.
    assert_eq!(announced.rtlsdr.agc, RtlSdrAgc::Manual);
    assert_eq!(announced.rtlsdr.ppm, -11);
    assert_eq!(announced.rtlsdr.tuner_gain_db, 22.5);

    // An edit from the client is written where the radio is, and echoed back.
    // `reopen: false` — this is the half that persists; the knob itself has
    // already reached the hardware through `SetGain`.
    let mut edited = stored.clone();
    edited.rtlsdr.tuner_gain_db = 41.2;
    edited.rtlsdr.agc = RtlSdrAgc::Tuner;
    edited.rtlsdr.bias_tee = true;
    send(
        &mut ws,
        &ClientMsg::Command(Command::SetRadioConfig {
            cfg: Box::new(edited.clone()),
            reopen: false,
        }),
    )
    .await;

    let echoed = next_radio_config(&mut ws).await;
    assert_eq!(echoed, edited, "the echo did not carry the edit");

    // The echo is read back from the store rather than bounced off the command,
    // so this is most of the assertion that the file was written — but read the
    // file too, because "announced" and "persisted" are the two halves that
    // have to agree for a restart to come up on the same settings.
    assert!(root.join("radio.json").exists(), "nothing was written where the radio is");
    assert_eq!(
        store.load_radio_config(),
        edited,
        "radio.json on the engine's machine was not updated"
    );

    // And the whole point of the lane: a client changes *which interface* the
    // far end has open and asks it to reopen. Nobody is at the other machine —
    // this is a headless server — so the switch has to happen from here, and
    // the radio that comes back has to be announced, because it is a different
    // radio with different gains, ports and ranges.
    let mut switched = edited.clone();
    switched.backend = Backend::Cat;
    send(
        &mut ws,
        &ClientMsg::Command(Command::SetRadioConfig {
            cfg: Box::new(switched.clone()),
            reopen: true,
        }),
    )
    .await;

    let caps = loop {
        if let ServerMsg::Capabilities(c) = recv_msg(&mut ws).await {
            break c;
        }
    };
    assert_eq!(
        caps.label, "reopened on Cat",
        "the engine did not reopen on the interface the client chose"
    );
    assert_eq!(
        store.load_radio_config().backend,
        Backend::Cat,
        "the switch was not persisted where the radio is"
    );

    let _ = std::fs::remove_dir_all(&root);
}
