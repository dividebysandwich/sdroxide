//! End-to-end server test: real engine (signal generator) + real WebSocket
//! client. Covers the handshake, state echo, spectrum/audio streaming, and
//! the single-client Busy rule.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use sdroxide_proto::{AudioCaps, ClientMsg, PROTO_VERSION, ServerMsg, decode, encode};
use sdroxide_radio::{AudioParams, EngineConfig, MicParams, SigGenSource, start_engine};
use sdroxide_server::{ServerParams, serve};
use sdroxide_types::{Command, DeviceCaps, Vfo};

const PORT: u16 = 39471;

async fn recv_msg(
    ws: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
) -> ServerMsg {
    loop {
        let m = tokio::time::timeout(Duration::from_secs(5), ws.next())
            .await
            .expect("timeout waiting for server message")
            .expect("stream ended")
            .expect("ws error");
        if let Message::Binary(bytes) = m {
            return decode::<ServerMsg>(&bytes).expect("decode");
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn full_session_flow() {
    // Engine on a signal generator.
    let (audio_producer, audio_consumer) = sdroxide_radio::rtrb::RingBuffer::<f32>::new(96_000);
    let (mic_producer, mic_consumer) = sdroxide_radio::rtrb::RingBuffer::<f32>::new(48_000);
    let source = SigGenSource::demo(1_536_000.0, 14_200_000.0);
    let caps = DeviceCaps {
        driver: "siggen".into(),
        label: "Test signal generator".into(),
        rx_channels: 1,
        freq_ranges_rx: vec![(0.0, 6e9)],
        ..DeviceCaps::default()
    };
    let handles = start_engine(
        Box::new(source),
        caps,
        EngineConfig {
            audio: Some(AudioParams { producer: audio_producer, out_rate: 48_000.0 }),
            mic: Some(MicParams { consumer: mic_consumer, rate: 48_000.0 }),
            ..Default::default()
        },
    );

    tokio::spawn(serve(ServerParams {
        cmd_tx: handles.cmd_tx,
        event_rx: handles.event_rx,
        spectrum_out: handles.spectrum_out,
        audio_rx: audio_consumer,
        mic_tx: mic_producer,
        bind: "127.0.0.1".into(),
        port: PORT,
        web_root: None,
    }));
    tokio::time::sleep(Duration::from_millis(400)).await;

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{PORT}/ws"))
        .await
        .expect("connect");

    // Hello → HelloAck with the device caps, PCM16 negotiated.
    let hello = ClientMsg::Hello {
        proto: PROTO_VERSION,
        audio: AudioCaps { opus_decode: false, opus_encode: false },
    };
    ws.send(Message::Binary(encode(&hello).unwrap().into())).await.unwrap();
    match recv_msg(&mut ws).await {
        ServerMsg::HelloAck { proto, caps, state, rx_codec, .. } => {
            assert_eq!(proto, PROTO_VERSION);
            assert_eq!(caps.label, "Test signal generator");
            assert!(state.sample_rate > 0.0);
            assert_eq!(rx_codec, sdroxide_proto::AudioCodec::Pcm16_48k);
        }
        other => panic!("expected HelloAck, got {other:?}"),
    }

    // Streams flow: within a few seconds we must see spectrum AND audio.
    let (mut got_spectrum, mut got_audio) = (false, false);
    while !(got_spectrum && got_audio) {
        match recv_msg(&mut ws).await {
            ServerMsg::Spectrum(f) => {
                assert!(!f.bins.is_empty());
                assert!(f.span_hz > 0.0);
                got_spectrum = true;
            }
            ServerMsg::RxAudio { payload, .. } => {
                assert_eq!(payload.len(), 1920, "20 ms of PCM16");
                got_audio = true;
            }
            _ => {}
        }
    }

    // Command → state echo.
    let cmd = ClientMsg::Command(Command::SetVfo { vfo: Vfo::A, hz: 14_100_000.0 });
    ws.send(Message::Binary(encode(&cmd).unwrap().into())).await.unwrap();
    loop {
        if let ServerMsg::State(s) = recv_msg(&mut ws).await {
            if (s.vfo_a_hz - 14_100_000.0).abs() < 1.0 {
                break;
            }
        }
    }

    // Second client gets Busy.
    let (mut ws2, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{PORT}/ws"))
        .await
        .expect("connect 2");
    ws2.send(Message::Binary(encode(&hello).unwrap().into())).await.unwrap();
    match recv_msg(&mut ws2).await {
        ServerMsg::Busy => {}
        other => panic!("expected Busy, got {other:?}"),
    }

    // First client drops; after cleanup a new client can connect again.
    drop(ws);
    drop(ws2);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let (mut ws3, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{PORT}/ws"))
        .await
        .expect("reconnect");
    ws3.send(Message::Binary(encode(&hello).unwrap().into())).await.unwrap();
    match recv_msg(&mut ws3).await {
        ServerMsg::HelloAck { .. } => {}
        other => panic!("expected HelloAck on reconnect, got {other:?}"),
    }
}
