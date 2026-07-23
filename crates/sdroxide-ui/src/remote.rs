//! `RemoteController`: the same UI seam as `LocalController`, but over a
//! WebSocket speaking `sdroxide-proto`. Compiles for wasm32 and native.

use std::collections::VecDeque;

use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};
use sdroxide_proto::{AudioCaps, AudioCodec, ClientMsg, PROTO_VERSION, ServerMsg, decode, encode};
use sdroxide_types::{AudioDevices, Command, RadioController, RadioEvent};

/// Platform audio glue: playback of received PCM and microphone capture.
/// The wasm client backs this with an AudioWorklet bridge.
pub trait AudioBridge {
    fn caps(&self) -> AudioCaps;
    /// Play mono 48 kHz PCM.
    fn play(&mut self, pcm: &[f32]);
    /// Append captured mic samples (mono 48 kHz) to `out`.
    fn pull_mic(&mut self, out: &mut Vec<f32>);
    /// Switchable sound devices, when the platform has any (native cpal
    /// bridge). The browser bridge keeps the default `None` — the browser
    /// owns device routing there.
    fn devices(&self) -> Option<AudioDevices> {
        None
    }
    /// Switch the output (`output = true`) or input device; `None` = default.
    fn set_device(&mut self, output: bool, name: Option<String>) {
        let _ = (output, name);
    }
}

pub struct RemoteController {
    sender: WsSender,
    receiver: WsReceiver,
    audio: Option<Box<dyn AudioBridge>>,
    pending: VecDeque<RadioEvent>,
    tx_codec: Option<AudioCodec>,
    transmitting: bool,
    mic_buf: Vec<f32>,
    mic_seq: u32,
}

impl RemoteController {
    /// `wake` is called from the socket thread whenever an event arrives —
    /// pass `ctx.request_repaint` so the UI wakes immediately instead of
    /// waiting for its next scheduled poll.
    pub fn connect(
        url: &str,
        audio: Option<Box<dyn AudioBridge>>,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self, String> {
        let (sender, receiver) =
            ewebsock::connect_with_wakeup(url, ewebsock::Options::default(), wake)
                .map_err(|e| e.to_string())?;
        Ok(RemoteController {
            sender,
            receiver,
            audio,
            pending: VecDeque::new(),
            tx_codec: None,
            transmitting: false,
            mic_buf: Vec::new(),
            mic_seq: 0,
        })
    }

    fn send_msg(&mut self, msg: &ClientMsg) {
        if let Ok(bytes) = encode(msg) {
            self.sender.send(WsMessage::Binary(bytes));
        }
    }

    fn on_server_msg(&mut self, msg: ServerMsg) {
        match msg {
            ServerMsg::HelloAck { caps, state, tx_codec, .. } => {
                self.tx_codec = Some(tx_codec);
                self.pending.push_back(RadioEvent::Capabilities(caps));
                self.pending.push_back(RadioEvent::State(state));
            }
            ServerMsg::State(s) => {
                self.transmitting = s.tx.ptt || s.tx.tune;
                self.pending.push_back(RadioEvent::State(s));
            }
            ServerMsg::Spectrum(f) => self.pending.push_back(RadioEvent::Spectrum(f)),
            ServerMsg::Meters(m) => self.pending.push_back(RadioEvent::Meters(m)),
            ServerMsg::Memories(m) => self.pending.push_back(RadioEvent::Memories(m)),
            ServerMsg::RxAudio { payload, .. } => {
                if let Some(bridge) = self.audio.as_mut() {
                    // Only the PCM16 downlink is decoded client-side; an
                    // Opus-capable bridge would advertise it in Hello.
                    let pcm: Vec<f32> = payload
                        .chunks_exact(2)
                        .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
                        .collect();
                    bridge.play(&pcm);
                }
            }
            ServerMsg::Pong(_) => {}
            ServerMsg::Busy => self.pending.push_back(RadioEvent::ConnectionLost(
                "server busy — another client is connected".into(),
            )),
            ServerMsg::Error(e) => self.pending.push_back(RadioEvent::ConnectionLost(e)),
            ServerMsg::Ft8Decodes(d) => self.pending.push_back(RadioEvent::Ft8Decodes(d)),
            ServerMsg::Ft8Status(s) => self.pending.push_back(RadioEvent::Ft8Status(s)),
            ServerMsg::Ft8QsoLogged(r) => self.pending.push_back(RadioEvent::Ft8QsoLogged(r)),
            ServerMsg::SkimmerSpots(s) => self.pending.push_back(RadioEvent::SkimmerSpots(s)),
            ServerMsg::SstvLine { image_id, y, rgb } => {
                self.pending.push_back(RadioEvent::SstvLine { image_id, y, rgb })
            }
            ServerMsg::SstvImage { image_id, mode, w, h, png } => {
                self.pending.push_back(RadioEvent::SstvImage { image_id, mode, w, h, png })
            }
            ServerMsg::SstvStatus(s) => self.pending.push_back(RadioEvent::SstvStatus(s)),
            ServerMsg::DigiImage { png } => self.pending.push_back(RadioEvent::DigiImage { png }),
        }
    }

    fn pump_mic(&mut self) {
        let Some(bridge) = self.audio.as_mut() else { return };
        if !self.transmitting {
            self.mic_buf.clear();
            // Keep draining the capture ring so it doesn't back up.
            let mut scratch = Vec::new();
            bridge.pull_mic(&mut scratch);
            return;
        }
        bridge.pull_mic(&mut self.mic_buf);
        while self.mic_buf.len() >= 960 {
            let payload: Vec<u8> = self.mic_buf[..960]
                .iter()
                .flat_map(|&s| ((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes())
                .collect();
            self.mic_seq = self.mic_seq.wrapping_add(1);
            let msg = ClientMsg::MicFrame { seq: self.mic_seq, payload };
            self.send_msg(&msg);
            self.mic_buf.drain(..960);
        }
    }
}

impl RadioController for RemoteController {
    fn send(&mut self, cmd: Command) {
        self.send_msg(&ClientMsg::Command(cmd));
    }

    fn poll_event(&mut self) -> Option<RadioEvent> {
        while let Some(ev) = self.receiver.try_recv() {
            match ev {
                WsEvent::Opened => {
                    let caps = self
                        .audio
                        .as_ref()
                        .map(|a| a.caps())
                        .unwrap_or(AudioCaps { opus_decode: false, opus_encode: false });
                    self.send_msg(&ClientMsg::Hello { proto: PROTO_VERSION, audio: caps });
                }
                WsEvent::Message(WsMessage::Binary(bytes)) => match decode::<ServerMsg>(&bytes) {
                    Ok(msg) => self.on_server_msg(msg),
                    Err(e) => self.pending.push_back(RadioEvent::ConnectionLost(format!(
                        "protocol error: {e}"
                    ))),
                },
                WsEvent::Message(_) => {}
                WsEvent::Error(e) => {
                    self.pending.push_back(RadioEvent::ConnectionLost(e));
                }
                WsEvent::Closed => {
                    self.pending
                        .push_back(RadioEvent::ConnectionLost("connection closed".into()));
                }
            }
        }
        self.pump_mic();
        self.pending.pop_front()
    }

    fn wants_repaint_soon(&self) -> bool {
        !self.pending.is_empty()
    }

    fn audio_devices(&self) -> Option<AudioDevices> {
        self.audio.as_ref().and_then(|a| a.devices())
    }

    fn set_audio_device(&mut self, output: bool, name: Option<String>) {
        if let Some(a) = self.audio.as_mut() {
            a.set_device(output, name);
        }
    }
}
