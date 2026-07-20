//! The single remote WebSocket session: Hello handshake, codec negotiation,
//! three-lane sender, and the command/mic receive loop.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tracing::{info, warn};

use sdroxide_proto::{AudioCodec, ClientMsg, PROTO_VERSION, ServerMsg, decode, encode};
use sdroxide_types::Command;

use crate::{SessionTx, Shared};

pub async fn ws_route(State(shared): State<Arc<Shared>>, upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(|socket| session(socket, shared))
}

fn msg(m: &ServerMsg) -> Message {
    Message::Binary(encode(m).expect("encode").into())
}

async fn session(mut socket: WebSocket, shared: Arc<Shared>) {
    // Single-client rule: the loser gets Busy and is closed immediately.
    if shared.busy.swap(true, Ordering::SeqCst) {
        let _ = socket.send(msg(&ServerMsg::Busy)).await;
        let _ = socket.close().await;
        return;
    }
    run_session(&mut socket, &shared).await;

    // Cleanup — whatever happened, release the slot and drop the key.
    *shared.session.lock().unwrap() = None;
    shared.busy.store(false, Ordering::SeqCst);
    let _ = shared.cmd_tx.send(Command::SetPtt(false));
    let _ = shared.cmd_tx.send(Command::SetTune(false));
    info!("remote session ended");
}

async fn run_session(socket: &mut WebSocket, shared: &Arc<Shared>) {
    // --- Hello handshake (5 s budget) ---------------------------------
    let hello = tokio::time::timeout(Duration::from_secs(5), socket.recv()).await;
    let audio_caps = match hello {
        Ok(Some(Ok(Message::Binary(bytes)))) => match decode::<ClientMsg>(&bytes) {
            Ok(ClientMsg::Hello { proto, audio }) if proto == PROTO_VERSION => audio,
            Ok(ClientMsg::Hello { proto, .. }) => {
                let _ = socket
                    .send(msg(&ServerMsg::Error(format!(
                        "protocol mismatch: server {PROTO_VERSION}, client {proto}"
                    ))))
                    .await;
                return;
            }
            _ => {
                let _ = socket.send(msg(&ServerMsg::Error("expected Hello".into()))).await;
                return;
            }
        },
        _ => return,
    };

    let rx_codec =
        if audio_caps.opus_decode { AudioCodec::Opus48kMono } else { AudioCodec::Pcm16_48k };
    let tx_codec =
        if audio_caps.opus_encode { AudioCodec::Opus48kMono } else { AudioCodec::Pcm16_48k };

    let (caps, state, memories) = {
        let latest = shared.latest.lock().unwrap();
        (latest.caps.clone(), latest.state.clone(), latest.memories.clone())
    };
    let ack = ServerMsg::HelloAck { proto: PROTO_VERSION, caps, state, rx_codec, tx_codec };
    if socket.send(msg(&ack)).await.is_err() {
        return;
    }
    let _ = socket.send(msg(&ServerMsg::Memories(memories))).await;
    info!(?rx_codec, ?tx_codec, "remote client connected");

    // --- register lanes -----------------------------------------------
    let (rel_tx, mut rel_rx) = mpsc::channel::<ServerMsg>(256);
    let (aud_tx, mut aud_rx) = mpsc::channel::<ServerMsg>(8);
    *shared.session.lock().unwrap() =
        Some(SessionTx { reliable: rel_tx, audio: aud_tx, rx_codec });

    let (mut ws_tx, mut ws_rx) = futures_util::StreamExt::split(socket);

    // Sender: reliable first, then audio, then latest spectrum.
    let mut spectrum_rx = shared.spectrum_rx.clone();
    let sender = async {
        let mut last_spectrum_seq = 0u32;
        loop {
            tokio::select! {
                biased;
                m = rel_rx.recv() => {
                    let Some(m) = m else { break };
                    if ws_tx.send(msg(&m)).await.is_err() { break; }
                }
                m = aud_rx.recv() => {
                    let Some(m) = m else { break };
                    if ws_tx.send(msg(&m)).await.is_err() { break; }
                }
                changed = spectrum_rx.changed() => {
                    if changed.is_err() { break; }
                    let frame = spectrum_rx.borrow_and_update().clone();
                    if let Some(f) = frame {
                        if f.seq != last_spectrum_seq {
                            last_spectrum_seq = f.seq;
                            if ws_tx.send(msg(&ServerMsg::Spectrum(f))).await.is_err() { break; }
                        }
                    }
                }
            }
        }
    };

    // Receiver: commands, mic frames, pings.
    let receiver = async {
        let mut opus_dec: Option<opus::Decoder> = None;
        let mut pcm = vec![0.0f32; 5760];
        while let Some(Ok(m)) = ws_rx.next().await {
            let Message::Binary(bytes) = m else {
                if matches!(m, Message::Close(_)) {
                    break;
                }
                continue;
            };
            match decode::<ClientMsg>(&bytes) {
                Ok(ClientMsg::Command(cmd)) => {
                    let _ = shared.cmd_tx.send(cmd);
                }
                Ok(ClientMsg::MicFrame { payload, .. }) => {
                    let n = match tx_codec {
                        AudioCodec::Opus48kMono => {
                            let dec = opus_dec.get_or_insert_with(|| {
                                opus::Decoder::new(48_000, opus::Channels::Mono)
                                    .expect("opus decoder")
                            });
                            match dec.decode_float(&payload, &mut pcm, false) {
                                Ok(n) => n,
                                Err(e) => {
                                    warn!("opus decode: {e}");
                                    continue;
                                }
                            }
                        }
                        AudioCodec::Pcm16_48k => {
                            let mut n = 0;
                            for (i, c) in payload.chunks_exact(2).enumerate() {
                                if i >= pcm.len() {
                                    break;
                                }
                                pcm[i] = i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0;
                                n += 1;
                            }
                            n
                        }
                    };
                    let mut mic = shared.mic_tx.lock().unwrap();
                    for &s in &pcm[..n] {
                        if mic.push(s).is_err() {
                            break; // ring full — engine will catch up
                        }
                    }
                }
                Ok(ClientMsg::Ping(t)) => {
                    if let Some(s) = shared.session.lock().unwrap().as_ref() {
                        let _ = s.reliable.try_send(ServerMsg::Pong(t));
                    }
                }
                Ok(ClientMsg::Hello { .. }) => {} // ignore late Hello
                Err(e) => warn!("bad client message: {e}"),
            }
        }
    };

    tokio::select! {
        _ = sender => {}
        _ = receiver => {}
    }
}
