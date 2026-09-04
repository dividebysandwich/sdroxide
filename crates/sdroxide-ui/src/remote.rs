//! `RemoteController`: the same UI seam as `LocalController`, but over a
//! WebSocket speaking `sdroxide-proto`. Compiles for wasm32 and native.

use std::collections::VecDeque;

use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};
use sdroxide_proto::{AudioCaps, AudioCodec, ClientMsg, PROTO_VERSION, ServerMsg, decode, encode};
use sdroxide_types::{AudioDevices, AuthPhase, Command, RadioController, RadioEvent};

/// Platform audio glue: playback of received PCM and microphone capture.
/// The wasm client backs this with an AudioWorklet bridge.
pub trait AudioBridge {
    fn caps(&self) -> AudioCaps;
    /// Play mono 48 kHz PCM.
    fn play(&mut self, pcm: &[f32]);
    /// Append captured mic samples (mono 48 kHz) to `out`.
    fn pull_mic(&mut self, out: &mut Vec<f32>);
    /// Whether the microphone is wanted right now. The browser bridge opens
    /// the capture stream on the first `true` and not before: on iOS
    /// `getUserMedia` puts the audio session into play-and-record, which
    /// attenuates and reroutes playback for listeners who never transmit.
    fn set_mic_active(&mut self, active: bool) {
        let _ = active;
    }
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

/// How many pre-open messages to hold. The window is one connect round-trip
/// wide and the UI sends a handful of config commands in it, so this only ever
/// bites if the socket never opens at all.
const OUTBOX_LIMIT: usize = 64;

/// Open a socket that wakes the UI on every event.
fn dial(
    url: &str,
    wake: &std::sync::Arc<dyn Fn() + Send + Sync>,
) -> Result<(WsSender, WsReceiver), String> {
    let wake = std::sync::Arc::clone(wake);
    ewebsock::connect_with_wakeup(url, ewebsock::Options::default(), move || wake())
        .map_err(|e| e.to_string())
}

/// Outbound gate: nothing the UI produces reaches the socket until the
/// handshake has finished, because the UI starts issuing commands (a
/// first-frame `SetSpectrumCfg`, with no debounce) long before it has.
/// Anything sent that early waits here and flushes in order once `HelloAck`
/// arrives.
///
/// The gate is held until `HelloAck` rather than merely until the socket opens,
/// which is what it used to be. A server that asks for a password answers
/// `Hello` with a challenge and reads nothing else until it is answered, so
/// commands released at open time would be discarded unread — the UI would come
/// up on the engine's spectrum config instead of its own, silently, and only
/// against a server that asks.
///
/// Without a gate at all the two platforms fail differently and neither is
/// acceptable: ewebsock's native sender queues pre-open messages, so the
/// command reaches the server ahead of `Hello` and the session is closed with
/// "expected Hello"; its web sender calls `send()` on a still-CONNECTING
/// socket, which throws and drops the command on the floor.
///
/// `Hello` and `Auth` are the two messages that must cross while the gate is
/// shut, so both are written straight to the socket instead of going through
/// it.
#[derive(Default)]
struct Outbox {
    opened: bool,
    queued: VecDeque<ClientMsg>,
}

impl Outbox {
    /// `Some(msg)` to write now, `None` if it was held back.
    fn send(&mut self, msg: ClientMsg) -> Option<ClientMsg> {
        if self.opened {
            return Some(msg);
        }
        if self.queued.len() == OUTBOX_LIMIT {
            // Oldest first: these are latest-wins config commands.
            self.queued.pop_front();
        }
        self.queued.push_back(msg);
        None
    }

    /// The handshake finished: everything that was waiting, in order.
    fn release(&mut self) -> Vec<ClientMsg> {
        self.opened = true;
        self.queued.drain(..).collect()
    }
}

pub struct RemoteController {
    sender: WsSender,
    receiver: WsReceiver,
    /// What to dial, and how to wake the UI when the socket has something —
    /// both kept so the session can be re-established in place after the link
    /// drops, without rebuilding the app around a new controller.
    url: String,
    wake: std::sync::Arc<dyn Fn() + Send + Sync>,
    outbox: Outbox,
    /// Where this connection stands with the server's sign-in challenge, for
    /// the UI to put a dialog up against.
    auth: AuthPhase,
    audio: Option<Box<dyn AudioBridge>>,
    pending: VecDeque<RadioEvent>,
    tx_codec: Option<AudioCodec>,
    transmitting: bool,
    /// The engine is recording a voice-keyer message. Its microphone is *our*
    /// microphone, so the uplink has to run for this too — otherwise a remote
    /// operator's recording comes back silent.
    voice_recording: bool,
    mic_buf: Vec<f32>,
    mic_seq: u32,
    /// The engine host's `radio.json` as last announced, with whatever the
    /// settings dialog has since edited into it. `None` until the server says —
    /// which it does on connect — so the Radio tab knows to wait rather than
    /// offer defaults.
    radio_cfg: Option<sdroxide_types::RadioConfig>,
    /// An edit is waiting to go out, and the wall-clock second the last one
    /// did. See [`RADIO_CFG_COALESCE_S`].
    radio_cfg_dirty: bool,
    radio_cfg_sent: f64,
    /// A front-end centre waiting to go out, and the wall-clock second the last
    /// one did. See [`CENTER_COALESCE_S`].
    center_pending: Option<f64>,
    center_sent: f64,
    /// Answers to the settings dialog's device questions, in the order the
    /// server ran them. Drained by [`RadioController::poll_probe`].
    probe_answers: VecDeque<sdroxide_types::ProbeAnswer>,
    /// The browser's one-output gate ([`RadioController::set_muted`]): this
    /// radio is not the focused tab, so its audio is not played *here*. The
    /// stream keeps arriving and the engine keeps decoding, recording and
    /// metering — only local playback stops. The operator's mute is not this;
    /// it goes to the engine as [`Command::SetMute`] like any other command.
    muted: bool,
    /// The far end's roster and which of it this session is on, as announced
    /// in the handshake. `None` until it lands, and re-announced on every
    /// reconnect — the station may have gained or lost a radio in between.
    ///
    /// Re-announced *within* a session as well, whenever the station's roster
    /// changes: by this client, by another one, or at the station itself.
    peers: Option<(u32, Vec<sdroxide_proto::RadioInfo>)>,
    /// Whether that station takes roster edits from here, as it said with the
    /// roster ([`RadioController::station_roster_editable`]).
    peers_editable: bool,
}

/// How long an edited interface configuration is held before it goes out.
///
/// The settings dialog hands the whole configuration over on every frame in
/// which anything differs from the last, so dragging a gain slider produces one
/// of these per frame. Each is a socket message *and* a `radio.json` write on
/// the engine's machine — frequently a Pi on an SD card — so they are coalesced
/// to the last value in the window. Nothing is waiting on it: the knob itself
/// already reached the hardware through `Command::SetGain` as it moved, and
/// this is only the copy that survives a restart. Apply skips the wait.
const RADIO_CFG_COALESCE_S: f64 = 0.25;

/// How long a commanded front-end centre is held before it goes out.
///
/// A panadapter drag that has run out of view to slide moves the *window*
/// instead, so the operator's gesture produces one of these a frame for as long
/// as their hand is down (issue #133). Locally that is a retune the hardware
/// keeps up with; at the far end of a network it is also a message, a retune, a
/// skimmer restart and a state broadcast to every other client on the station —
/// sixty times a second, on a Pi that is already carrying the radio. What comes
/// back is then a picture whose centre is a round trip behind the view it is
/// drawn in, which is what issue #188 saw as the waterfall tearing under a drag.
///
/// A tenth of a second: fast enough that the window follows the hand — the view
/// itself already moves every frame, and this only decides how often the
/// capture catches up — and slow enough to cost the far end six retunes a
/// second rather than sixty.
const CENTER_COALESCE_S: f64 = 0.1;

/// Whether `window` seconds have passed since something was last sent at
/// `sent`, on a *wall* clock.
///
/// The second test is what makes it a wall clock rather than a stopwatch: an
/// NTP step backwards would otherwise leave `sent` in the future and hold the
/// queued value there until the clock caught up — which for the last nudge of a
/// panadapter drag means the window simply never arrives where the operator put
/// it. A `now` that is behind `sent` is nonsense, and the safe reading of
/// nonsense here is "send it".
fn window_elapsed(now: f64, sent: f64, window: f64) -> bool {
    now < sent || now - sent >= window
}

/// The address of one of a station's radios, built from the address a
/// connection is already on: everything up to the endpoint, then the radio's
/// own. Keeps whatever the operator typed — the scheme, a port, a reverse
/// proxy's path prefix — and changes only which radio is being asked for.
fn radio_url(connected_to: &str, id: u32) -> String {
    let base = match connected_to.rfind("/ws") {
        Some(i) => &connected_to[..i],
        None => connected_to.trim_end_matches('/'),
    };
    format!("{base}/ws/{id}")
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
        let wake: std::sync::Arc<dyn Fn() + Send + Sync> = std::sync::Arc::new(wake);
        let (sender, receiver) = dial(url, &wake)?;
        Ok(RemoteController {
            sender,
            receiver,
            url: url.to_string(),
            wake,
            outbox: Outbox::default(),
            auth: AuthPhase::Open,
            audio,
            pending: VecDeque::new(),
            tx_codec: None,
            transmitting: false,
            voice_recording: false,
            mic_buf: Vec::new(),
            mic_seq: 0,
            radio_cfg: None,
            radio_cfg_dirty: false,
            radio_cfg_sent: 0.0,
            center_pending: None,
            center_sent: 0.0,
            probe_answers: VecDeque::new(),
            muted: false,
            peers: None,
            peers_editable: false,
        })
    }

    /// Write straight to the socket, bypassing the gate. Only for messages the
    /// gate has already released.
    fn write(&mut self, msg: &ClientMsg) {
        if let Ok(bytes) = encode(msg) {
            self.sender.send(WsMessage::Binary(bytes));
        }
    }

    fn send_msg(&mut self, msg: ClientMsg) {
        if let Some(msg) = self.outbox.send(msg) {
            self.write(&msg);
        }
    }

    fn on_server_msg(&mut self, msg: ServerMsg) {
        match msg {
            ServerMsg::HelloAck { caps, state, tx_codec, .. } => {
                self.tx_codec = Some(tx_codec);
                // Whatever the server wanted, it has it: the handshake is over
                // and everything the UI produced while it ran can go out now.
                self.auth = AuthPhase::Open;
                let queued = self.outbox.release();
                for msg in queued {
                    self.write(&msg);
                }
                self.pending.push_back(RadioEvent::Capabilities(caps));
                self.pending.push_back(RadioEvent::State(state));
            }
            // The server wants a password before it will go any further. The
            // socket stays open and the UI puts a dialog up; nothing else
            // happens until [`RadioController::send_auth`] answers it.
            ServerMsg::AuthRequired => self.auth = AuthPhase::Prompt(None),
            ServerMsg::AuthRejected(why) => self.auth = AuthPhase::Prompt(Some(why)),
            ServerMsg::State(s) => {
                self.transmitting = s.tx.ptt || s.tx.tune;
                self.pending.push_back(RadioEvent::State(s));
            }
            ServerMsg::Spectrum(f) => self.pending.push_back(RadioEvent::Spectrum(f)),
            ServerMsg::WideSpectrum(f) => self.pending.push_back(RadioEvent::WideSpectrum(f)),
            ServerMsg::Meters(m) => self.pending.push_back(RadioEvent::Meters(m)),
            ServerMsg::Memories(m) => self.pending.push_back(RadioEvent::Memories(m)),
            ServerMsg::MemoryFolders(f) => self.pending.push_back(RadioEvent::MemoryFolders(f)),
            ServerMsg::Scanner(c) => self.pending.push_back(RadioEvent::Scanner(c)),
            // Dropped rather than decoded while another tab holds the page's
            // single output: the work saved is the point on a browser tab
            // holding several radios.
            ServerMsg::RxAudio { .. } if self.muted => {}
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
            ServerMsg::Notice(n) => self.pending.push_back(RadioEvent::Notice(n)),
            ServerMsg::Ft8Decodes(d) => self.pending.push_back(RadioEvent::Ft8Decodes(d)),
            ServerMsg::WsprSpots(s) => self.pending.push_back(RadioEvent::WsprSpots(s)),
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
            ServerMsg::WefaxLine { image_id, y, gray } => {
                self.pending.push_back(RadioEvent::WefaxLine { image_id, y, gray })
            }
            ServerMsg::WefaxImage { image_id, w, h, png } => {
                self.pending.push_back(RadioEvent::WefaxImage { image_id, w, h, png })
            }
            ServerMsg::WefaxStatus(s) => self.pending.push_back(RadioEvent::WefaxStatus(s)),
            ServerMsg::Rds(d) => self.pending.push_back(RadioEvent::Rds(d)),
            ServerMsg::Drm(d) => self.pending.push_back(RadioEvent::Drm(d)),
            ServerMsg::IsmReports(r) => self.pending.push_back(RadioEvent::IsmReports(r)),
            ServerMsg::IsmStatus(s) => self.pending.push_back(RadioEvent::IsmStatus(s)),
            ServerMsg::AdsbStatus(s) => self.pending.push_back(RadioEvent::AdsbStatus(s)),
            ServerMsg::Vdl2Status(s) => self.pending.push_back(RadioEvent::Vdl2Status(s)),
            ServerMsg::AisStatus(s) => self.pending.push_back(RadioEvent::AisStatus(s)),
            ServerMsg::RifpRows { image_id, y, w, h, rows } => {
                self.pending.push_back(RadioEvent::RifpRows { image_id, y, w, h, rows })
            }
            ServerMsg::RifpImage { image_id, meta, png } => {
                self.pending.push_back(RadioEvent::RifpImage { image_id, meta, png })
            }
            ServerMsg::RifpStatus(s) => self.pending.push_back(RadioEvent::RifpStatus(s)),
            ServerMsg::DigiImage { png } => self.pending.push_back(RadioEvent::DigiImage { png }),
            ServerMsg::HellColumns { seq, rows, cols } => {
                self.pending.push_back(RadioEvent::HellColumns { seq, rows, cols })
            }
            ServerMsg::VoiceStatus(v) => {
                self.voice_recording = v.recording.is_some();
                self.pending.push_back(RadioEvent::VoiceStatus(v));
            }
            ServerMsg::Spots(s) => self.pending.push_back(RadioEvent::Spots(s)),
            ServerMsg::NetStatus(s) => self.pending.push_back(RadioEvent::NetStatus(s)),
            ServerMsg::CallsignResult(c) => self.pending.push_back(RadioEvent::CallsignResult(c)),
            ServerMsg::Upload(r) => self.pending.push_back(RadioEvent::Upload(r)),
            ServerMsg::Confirmations(r) => self.pending.push_back(RadioEvent::Confirmations(r)),
            ServerMsg::RigctldStatus { running, addr, clients, error } => {
                self.pending.push_back(RadioEvent::RigctldStatus { running, addr, clients, error })
            }
            ServerMsg::TciServerStatus { running, addr, clients, error } => self
                .pending
                .push_back(RadioEvent::TciServerStatus { running, addr, clients, error }),
            ServerMsg::ImagePresets(p) => self.pending.push_back(RadioEvent::ImagePresets(p)),
            ServerMsg::ImageSlotSource { slot, version, png } => {
                self.pending.push_back(RadioEvent::ImageSlotSource { slot, version, png })
            }
            ServerMsg::ImageListing(l) => self.pending.push_back(RadioEvent::ImageListing(l)),
            ServerMsg::ImageFile { kind, name, png } => {
                self.pending.push_back(RadioEvent::ImageFile { kind, name, png })
            }
            ServerMsg::ImageSaved(e) => self.pending.push_back(RadioEvent::ImageSaved(e)),
            ServerMsg::ImageDeleted { kind, name } => {
                self.pending.push_back(RadioEvent::ImageDeleted { kind, name })
            }
            // Winlink: straight through to the app, same as the picture store.
            ServerMsg::WinlinkStatus(st) => self.pending.push_back(RadioEvent::WinlinkStatus(st)),
            ServerMsg::MailListing(l) => self.pending.push_back(RadioEvent::MailListing(l)),
            ServerMsg::MailMessage(m) => self.pending.push_back(RadioEvent::MailMessage(m)),
            ServerMsg::MailSaved(mid) => self.pending.push_back(RadioEvent::MailSaved(mid)),
            ServerMsg::MailDeleted { folder, mid } => {
                self.pending.push_back(RadioEvent::MailDeleted { folder, mid })
            }
            ServerMsg::StationConfig(c) => self.pending.push_back(RadioEvent::StationConfig(c)),
            ServerMsg::TleSubStatus(s) => self.pending.push_back(RadioEvent::TleSubStatus(s)),
            ServerMsg::SatTrack(t) => self.pending.push_back(RadioEvent::SatTrack(t)),
            ServerMsg::RotatorStatus { connected, az_deg, el_deg, error } => self
                .pending
                .push_back(RadioEvent::RotatorStatus { connected, az_deg, el_deg, error }),
            ServerMsg::RelayStatus(st) => self.pending.push_back(RadioEvent::RelayStatus(st)),
            ServerMsg::RadioConfig(c) => {
                // Adopted only when nothing of ours is waiting to go out. This
                // message is usually the echo of our own write, but it can
                // arrive mid-drag — and a slider that jumped back to the value
                // the server had a moment ago, every time it answered, would be
                // unusable.
                if !self.radio_cfg_dirty {
                    self.radio_cfg = Some((*c).clone());
                }
                self.pending.push_back(RadioEvent::RadioConfig(c));
            }
            // Its own queue rather than the event stream: this is the answer to
            // a question the settings dialog asked, not something the engine
            // announced, and the UI drains it where it asked.
            ServerMsg::ProbeAnswer(a) => self.probe_answers.push_back(*a),
            // The far end opened a different radio. The UI already knows what
            // to do with this — it is the same event an in-process engine
            // sends when it adopts a source.
            ServerMsg::Capabilities(c) => self.pending.push_back(RadioEvent::Capabilities(c)),
            // The same radio, revising itself — a band-dependent gain ladder,
            // an antenna list the rig only now answered for. Kept apart from
            // the above all the way to the screen, because what the screen
            // does about a *new* radio is throw away everything it holds about
            // the old one.
            ServerMsg::CapabilitiesUpdated(c) => {
                self.pending.push_back(RadioEvent::CapabilitiesUpdated(c));
            }
            // Which of the station's radios this session is on, and what else
            // it has. Kept rather than turned into an event: it is a fact about
            // the *connection*, which the shell reads to put the station's
            // other radios in tabs of their own.
            ServerMsg::Radios { me, radios, editable } => {
                self.peers = Some((me, radios));
                self.peers_editable = editable;
            }
        }
    }

    /// Send the queued interface configuration, if it is due.
    ///
    /// `reopen` is the operator pressing Apply: it goes out at once, whether or
    /// not anything is queued (Apply on an unchanged config still means
    /// "reconnect the radio"), and carries the instruction to rebuild the front
    /// end. See [`RADIO_CFG_COALESCE_S`] for why the rest wait.
    fn flush_radio_config(&mut self, reopen: bool) {
        if !reopen && !self.radio_cfg_dirty {
            return;
        }
        let now = crate::time::now_unix_f64();
        if !reopen && !window_elapsed(now, self.radio_cfg_sent, RADIO_CFG_COALESCE_S) {
            return;
        }
        let Some(cfg) = self.radio_cfg.clone() else { return };
        self.radio_cfg_dirty = false;
        self.radio_cfg_sent = now;
        self.send_msg(ClientMsg::Command(Command::SetRadioConfig { cfg: Box::new(cfg), reopen }));
    }

    /// Send the queued front-end centre, if the window has passed. See
    /// [`CENTER_COALESCE_S`].
    fn flush_center(&mut self) {
        let Some(hz) = self.center_pending else { return };
        let now = crate::time::now_unix_f64();
        if !window_elapsed(now, self.center_sent, CENTER_COALESCE_S) {
            return;
        }
        self.center_pending = None;
        self.center_sent = now;
        self.send_msg(ClientMsg::Command(Command::SetCenter(hz)));
    }

    fn pump_mic(&mut self) {
        let Some(bridge) = self.audio.as_mut() else { return };
        bridge.set_mic_active(self.transmitting || self.voice_recording);
        if !self.transmitting && !self.voice_recording {
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
            self.send_msg(msg);
            self.mic_buf.drain(..960);
        }
    }
}

impl RadioController for RemoteController {
    fn send(&mut self, cmd: Command) {
        // The one command a gesture produces once a frame for as long as it
        // lasts, and the most expensive one to act on at the far end — held to
        // a rate the network and the radio can carry, latest value wins. See
        // [`CENTER_COALESCE_S`]; `flush_center` releases it.
        if let Command::SetCenter(hz) = cmd {
            self.center_pending = Some(hz);
            self.flush_center();
            return;
        }
        self.send_msg(ClientMsg::Command(cmd));
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
                    // Straight to the socket, ahead of the gate: this is what
                    // the gate exists to stay behind. Everything the UI queued
                    // follows once the server answers — see `HelloAck`.
                    self.write(&ClientMsg::Hello { proto: PROTO_VERSION, audio: caps });
                }
                WsEvent::Message(WsMessage::Binary(bytes)) => match decode::<ServerMsg>(&bytes) {
                    Ok(msg) => self.on_server_msg(msg),
                    Err(e) => self
                        .pending
                        .push_back(RadioEvent::ConnectionLost(format!("protocol error: {e}"))),
                },
                WsEvent::Message(_) => {}
                WsEvent::Error(e) => {
                    self.pending.push_back(RadioEvent::ConnectionLost(e));
                }
                WsEvent::Closed => {
                    self.pending.push_back(RadioEvent::ConnectionLost("connection closed".into()));
                }
            }
        }
        self.pump_mic();
        self.flush_radio_config(false);
        self.flush_center();
        self.pending.pop_front()
    }

    fn wants_repaint_soon(&self) -> bool {
        // A queued interface edit counts: it is released by the clock rather
        // than by anything arriving, so without a frame to release it on, the
        // last nudge of a slider would sit here until something else woke the
        // app up. A queued centre is the same bargain — and there the value
        // left behind would be the window the drag ended on.
        !self.pending.is_empty()
            || self.radio_cfg_dirty
            || self.center_pending.is_some()
            || !self.probe_answers.is_empty()
    }

    fn can_reconnect(&self) -> bool {
        true
    }

    fn engine_is_remote(&self) -> bool {
        true
    }

    fn peer_radios(&self) -> Vec<sdroxide_types::PeerRadio> {
        let Some((me, radios)) = self.peers.as_ref() else { return Vec::new() };
        radios
            .iter()
            .filter(|r| r.id != *me)
            .map(|r| sdroxide_types::PeerRadio {
                id: r.id,
                name: r.name.clone(),
                named: r.named,
                url: radio_url(&self.url, r.id),
            })
            .collect()
    }

    fn peer_first_radio(&self) -> Option<u32> {
        self.peers.as_ref()?.1.first().map(|r| r.id)
    }

    fn station_roster_editable(&self) -> bool {
        self.peers_editable
    }

    fn add_station_radio(&mut self, name: &str) {
        self.send_msg(ClientMsg::AddRadio { name: name.to_string() });
    }

    fn remove_station_radio(&mut self, id: u32) {
        self.send_msg(ClientMsg::RemoveRadio { id });
    }

    fn rename_station_radio(&mut self, id: u32, name: &str) {
        self.send_msg(ClientMsg::RenameRadio { id, name: name.to_string() });
    }

    /// The switch's position as the station last announced it — for *this*
    /// connection's radio, which is the one the tab holding this controller
    /// draws a switch for. `None` where the station offers none.
    fn station_power(&self) -> Option<bool> {
        let (me, radios) = self.peers.as_ref()?;
        radios.iter().find(|r| r.id == *me)?.enabled
    }

    fn switch_station_radio(&mut self, id: u32, on: bool) {
        self.send_msg(ClientMsg::SetRadioEnabled { id, on });
    }

    fn peer_removed(&self) -> bool {
        // Only once a roster has actually arrived: before that there is
        // nothing to be missing from, and a tab that read silence as "you have
        // been closed" would shut itself on every slow connection.
        self.peers.as_ref().is_some_and(|(me, radios)| !radios.iter().any(|r| r.id == *me))
    }

    fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    fn peer_name(&self) -> Option<String> {
        let (me, radios) = self.peers.as_ref()?;
        // Only a name somebody typed: a derived one is this radio's interface,
        // which the tab reads off its own connection and keeps up to date.
        radios.iter().find(|r| r.id == *me && r.named).map(|r| r.name.clone())
    }

    fn peer_url(&self) -> Option<String> {
        // The canonical form once the far end has said which radio this is, so
        // that a session dialled at `/ws` and the same radio offered by a
        // sibling as `/ws/0` are recognised as one and the same. Before that,
        // whatever was dialled — it is all there is to go on.
        Some(match self.peers.as_ref() {
            Some((me, _)) => radio_url(&self.url, *me),
            None => self.url.clone(),
        })
    }

    fn auth_phase(&self) -> AuthPhase {
        self.auth.clone()
    }

    fn send_auth(&mut self, username: String, password: String) {
        // Past the gate, like `Hello`: the server is reading nothing else.
        self.auth = AuthPhase::Checking;
        self.write(&ClientMsg::Auth { username, password });
    }

    fn reconnect(&mut self) -> Result<(), String> {
        // Close first, and only then dial: the server allows one control
        // session at a time, so a new socket opened while the old one is still
        // registered is answered with `Busy` — the reconnect would fail on the
        // strength of the connection it is replacing.
        self.sender.close();
        let (sender, receiver) = dial(&self.url, &self.wake)?;
        self.sender = sender;
        self.receiver = receiver;
        // Everything below is per-session: the outbox gate has to hold commands
        // behind a fresh `Hello`, the codec is renegotiated in the handshake,
        // and neither queued events nor a half-sent microphone block from the
        // dead session may be carried into the new one.
        self.outbox = Outbox::default();
        // Including the sign-in: the new socket is challenged on its own
        // merits, so a dialog left over from the dead session must not be
        // showing against it — nor a `Checking` that nothing will ever answer.
        self.auth = AuthPhase::Open;
        self.pending.clear();
        self.tx_codec = None;
        self.transmitting = false;
        self.voice_recording = false;
        self.mic_buf.clear();
        self.mic_seq = 0;
        // ...and the roster, which the new session announces afresh: the
        // station may have gained or lost a radio while the link was down.
        self.peers = None;
        self.peers_editable = false;
        // The interface configuration is per-session too: the new socket may
        // reach a different station, and an edit queued against the dead one
        // must not be applied to it. The fresh session announces its own.
        self.radio_cfg = None;
        self.radio_cfg_dirty = false;
        // Likewise a centre queued against the dead session: the fresh one
        // announces the window its own front end is on, and the view is fitted
        // to that.
        self.center_pending = None;
        // And an answer from the dead session: it describes a machine this
        // socket may not even be reaching any more.
        self.probe_answers.clear();
        Ok(())
    }

    fn audio_devices(&self) -> Option<AudioDevices> {
        self.audio.as_ref().and_then(|a| a.devices())
    }

    fn set_audio_device(&mut self, output: bool, name: Option<String>) {
        if let Some(a) = self.audio.as_mut() {
            a.set_device(output, name);
        }
    }

    /// The engine host's interface configuration, as it announced it.
    ///
    /// This is what lets the Radio tab render here at all: the per-backend
    /// panels are driven entirely by the `Option<RadioConfig>` they are handed,
    /// so answering with the *server's* copy is the whole of what makes an
    /// RTL-SDR's AGC, ppm, HF path and bias tee reachable from another machine.
    /// `None` only in the moment before the connect-time announcement lands.
    fn radio_config(&self) -> Option<sdroxide_types::RadioConfig> {
        self.radio_cfg.clone()
    }

    fn set_radio_config(&mut self, cfg: sdroxide_types::RadioConfig) {
        self.radio_cfg = Some(cfg);
        self.radio_cfg_dirty = true;
        self.flush_radio_config(false);
    }

    fn reopen_source(&mut self) {
        self.flush_radio_config(true);
    }

    /// Ask the engine host, and answer nothing here: every one of these is a
    /// question about *its* buses, ports and network. `None` says so — the UI
    /// waits for [`RemoteController::poll_probe`].
    fn probe(&mut self, req: sdroxide_types::DeviceProbe) -> Option<sdroxide_types::ProbeAnswer> {
        self.send_msg(ClientMsg::Probe(req));
        None
    }

    fn poll_probe(&mut self) -> Option<sdroxide_types::ProbeAnswer> {
        self.probe_answers.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::SpectrumConfig;

    /// A station's other radios are dialled from the address this connection
    /// is already on — whatever shape the operator gave it.
    #[test]
    fn a_sibling_radio_is_dialled_beside_the_one_we_are_on() {
        // The bare endpoint, which is what `--connect host:4950` builds.
        assert_eq!(radio_url("ws://192.168.1.10:4950/ws", 1), "ws://192.168.1.10:4950/ws/1");
        // ...and from a session that is already on a named radio.
        assert_eq!(radio_url("ws://192.168.1.10:4950/ws/10", 0), "ws://192.168.1.10:4950/ws/0");
        // A reverse proxy's path prefix is part of the address, not of the
        // endpoint: it has to survive.
        assert_eq!(radio_url("wss://shack.example/sdr/ws", 2), "wss://shack.example/sdr/ws/2");
        // An address with no endpoint at all still names one.
        assert_eq!(radio_url("ws://host:4950", 3), "ws://host:4950/ws/3");
        assert_eq!(radio_url("ws://host:4950/", 3), "ws://host:4950/ws/3");
        // A host that begins with the endpoint's own letters must not be
        // mistaken for it.
        assert_eq!(radio_url("ws://wsserver:4950/ws", 1), "ws://wsserver:4950/ws/1");
    }

    /// The first-frame `SetSpectrumCfg` that the capture from the bug report
    /// caught on the wire ahead of `Hello`.
    fn first_frame_cfg() -> ClientMsg {
        ClientMsg::Command(Command::SetSpectrumCfg(SpectrumConfig {
            fft_size: 32768,
            display_bins: 4096,
            rows_per_sec: 56,
            fps: 60,
            avg_tc: 0.0,
            db_floor: -120.0,
            db_ceil: -20.0,
            viewport: Some((14_584_000.0, 14_968_000.0)),
        }))
    }

    /// The rate limiter both flushes are built on. The wall-clock guard is the
    /// half worth pinning: without it a clock stepping backwards parks the
    /// deadline in the future, and the value waiting behind it is the window a
    /// panadapter drag has just been let go on.
    #[test]
    fn a_coalescing_window_survives_a_clock_that_steps_back() {
        // Inside the window: held.
        assert!(!window_elapsed(1000.05, 1000.0, 0.1));
        // On the boundary and past it: sent.
        assert!(window_elapsed(1000.1, 1000.0, 0.1));
        assert!(window_elapsed(1001.0, 1000.0, 0.1));
        // Behind what was stamped — an NTP correction — is sent, not stranded.
        assert!(window_elapsed(999.0, 1000.0, 0.1));
    }

    /// What a drag costs the far end: the view still moves every frame, but the
    /// wire carries the centre about ten times a second. Sixty retunes a second
    /// is what issue #188's station was being asked for.
    ///
    /// A range rather than a count, because a frame clock and a tenth of a
    /// second do not divide evenly and a send lands on the first frame at or
    /// past the window — six frames sometimes, seven others. What the rule
    /// promises is a rate, not a fencepost.
    #[test]
    fn a_drag_is_held_to_the_window() {
        const SECS: u32 = 4;
        const FPS: u32 = 60;
        let (mut sent_at, mut sent) = (0.0_f64, 0);
        for frame in 0..SECS * FPS {
            let now = f64::from(frame) / f64::from(FPS);
            if window_elapsed(now, sent_at, CENTER_COALESCE_S) {
                sent_at = now;
                sent += 1;
            }
        }
        let per_sec = f64::from(sent) / f64::from(SECS);
        assert!(
            (8.0..=11.0).contains(&per_sec),
            "a drag asked the far end for {per_sec} retunes a second"
        );
    }

    /// The regression: on a link with real latency the UI issues commands
    /// before `Opened` arrives. Nothing may reach the socket until `Hello` has,
    /// or the server closes the session with "expected Hello".
    #[test]
    fn nothing_precedes_the_handshake_on_the_wire() {
        let mut ob = Outbox::default();
        assert!(ob.send(first_frame_cfg()).is_none(), "a pre-open command must not be written");

        let flushed = ob.release();
        assert_eq!(flushed, [first_frame_cfg()], "the held command must follow, not be lost");
    }

    /// The gate stays shut for the whole handshake, not just until the socket
    /// opens: a server that asks for a password reads nothing but `Auth` until
    /// it has one, so a command released at open time would be dropped unread.
    #[test]
    fn the_gate_stays_shut_across_a_sign_in() {
        let mut ob = Outbox::default();
        // The socket opens — `Hello` goes out around the gate, not through it —
        // and the server answers with a challenge rather than `HelloAck`.
        assert!(ob.send(first_frame_cfg()).is_none());
        // The operator types a password, gets it wrong, types it again. Still
        // nothing may leave.
        assert!(ob.send(ClientMsg::Ping(1)).is_none(), "still held during the challenge");
        // Only `HelloAck` opens it.
        assert_eq!(ob.release(), [first_frame_cfg(), ClientMsg::Ping(1)]);
    }

    /// Ordering is preserved across the gate, and once open there is no
    /// buffering left to reorder anything.
    #[test]
    fn queued_commands_keep_their_order_and_then_pass_through() {
        let mut ob = Outbox::default();
        for cmd in [Command::SetPtt(true), Command::SetPtt(false), Command::SetCenter(14_074_000.0)]
        {
            assert!(ob.send(ClientMsg::Command(cmd)).is_none());
        }
        assert_eq!(
            ob.release(),
            [
                ClientMsg::Command(Command::SetPtt(true)),
                ClientMsg::Command(Command::SetPtt(false)),
                ClientMsg::Command(Command::SetCenter(14_074_000.0)),
            ]
        );
        // After the handshake the gate is transparent.
        let passed = ob.send(ClientMsg::Ping(7));
        assert_eq!(passed, Some(ClientMsg::Ping(7)));
    }

    /// A socket that never opens — or a sign-in nobody ever completes — must
    /// not grow the queue without bound.
    #[test]
    fn the_outbox_is_bounded_and_drops_the_stalest_first() {
        let mut ob = Outbox::default();
        for f in 0..(OUTBOX_LIMIT as u64 + 10) {
            ob.send(ClientMsg::Ping(f));
        }
        let flushed = ob.release();
        assert_eq!(flushed.len(), OUTBOX_LIMIT);
        // The 10 oldest were dropped; the newest survived.
        assert_eq!(flushed[0], ClientMsg::Ping(10));
        assert_eq!(flushed[OUTBOX_LIMIT - 1], ClientMsg::Ping(OUTBOX_LIMIT as u64 + 9));
    }
}
