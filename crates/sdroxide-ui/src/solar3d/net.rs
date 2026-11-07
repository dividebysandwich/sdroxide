//! The browser end of `/solar-ws`.
//!
//! Fills the very same `Arc<Mutex<SolarData>>` the native [`SolarFeed`] fills,
//! which is why the scene, overlay and renderer need no notion of where their
//! data came from.
//!
//! [`SolarFeed`]: sdroxide_solar::SolarFeed
//!
//! Two products arrive in their original form and are rebuilt here, using the
//! same functions the native feed calls:
//!
//! * the SDO image, as the fetched JPEG → [`imagery::decode`]
//! * satellites, as the fetched element set → [`satellites::parse_tles`]
//!
//! Both are markedly cheaper to ship than the decoded product, and neither is
//! serialisable anyway. Decoding a 1024² JPEG costs a few tens of milliseconds
//! and happens at most every ten minutes, so it runs inline rather than being
//! worth a worker.

use crate::i18n::tr;
use std::sync::{Arc, Mutex};

use ewebsock::{WsMessage, WsReceiver, WsSender};

use sdroxide_proto::solar::{SOLAR_PROTO_VERSION, SolarClientMsg, SolarServerMsg};
use sdroxide_proto::{decode as proto_decode, encode};
use sdroxide_solar::{SdoChannel, SolarData, clouds, imagery, satellites};

/// Connection state, as the overlay's status line reports it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Link {
    Connecting,
    Up,
    /// The socket closed or the handshake failed. The view keeps drawing
    /// whatever it already has — the ephemeris needs no server at all — so this
    /// is a status line, not an error screen.
    Down(String),
}

pub struct SolarClient {
    sender: WsSender,
    receiver: WsReceiver,
    pub link: Link,
    /// Where this viewer stands with the server's sign-in challenge. A viewer
    /// controls nothing, but it is shown the operator's QTH and everything the
    /// station is decoding, so a server that asks the control client for a
    /// password asks this one too.
    pub auth: sdroxide_types::AuthPhase,
    /// The snapshot handed to `SolarUi`, filled in place as messages arrive.
    data: Arc<Mutex<SolarData>>,
    /// Newest decode list, for the caller's own fade bookkeeping.
    pub decodes: Vec<sdroxide_types::Decode>,
    /// The propagation field, reassembled from the planes the server sends.
    ///
    /// Rebuilt rather than merged: the server sends every live band each time,
    /// so a band that has gone quiet disappears by being absent — which is the
    /// only way this tab can learn that it closed.
    pub prop: Arc<sdroxide_types::PropField>,
    pub my_grid: String,
    pub dx_grid: Option<String>,
    pub transmitting: bool,
    /// The station being worked, the mode and the dial — what the card over the
    /// QSO arc is built from. This tab has no radio and no sequencer of its
    /// own, so all three arrive with the status rather than being looked up.
    pub dx_call: Option<String>,
    pub mode: String,
    pub freq_hz: f64,
    /// When the contact began and what has been exchanged on it.
    pub qso: Option<sdroxide_types::QsoLive>,
    /// The operator's satellite frequency overrides, in the shape `SolarUi`
    /// takes. Replaced wholesale when the station's config changes, so the
    /// caller adopts it with an `Arc::ptr_eq` check rather than a comparison of
    /// the table itself.
    ///
    /// Only `freqs` is ever populated: it is the one field of the config this
    /// view reads, and the only one the relay carries — see
    /// [`SolarServerMsg::SatFreqs`].
    pub sat_cfg: Arc<sdroxide_types::SatConfig>,
    /// Channel/resolution last requested, so a change in the UI is sent once.
    sent_channel: Option<(u8, u16)>,
    /// What this feed was dialled at. Kept so the sign-in can tell which
    /// station is asking — the map is challenged by the same door as that
    /// station's radios.
    url: String,
}

impl SolarClient {
    /// The address this feed is on.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// `wake` is called from the socket thread whenever a message arrives —
    /// pass `ctx.request_repaint` so a new solar image appears without waiting
    /// for the next scheduled frame.
    pub fn connect(url: &str, wake: impl Fn() + Send + Sync + 'static) -> Result<Self, String> {
        // The handshake waits for `WsEvent::Opened` — see `poll`.
        let (sender, receiver) =
            ewebsock::connect_with_wakeup(url, ewebsock::Options::default(), wake)
                .map_err(|e| e.to_string())?;
        Ok(SolarClient {
            sender,
            receiver,
            url: url.to_string(),
            link: Link::Connecting,
            auth: sdroxide_types::AuthPhase::Open,
            data: Arc::new(Mutex::new(SolarData::default())),
            decodes: Vec::new(),
            prop: Default::default(),
            my_grid: String::new(),
            dx_grid: None,
            transmitting: false,
            dx_call: None,
            mode: String::new(),
            freq_hz: 0.0,
            qso: None,
            sat_cfg: Default::default(),
            sent_channel: None,
        })
    }

    /// The handle to hand to `SolarUi::data`.
    pub fn shared(&self) -> Arc<Mutex<SolarData>> {
        Arc::clone(&self.data)
    }

    pub fn send(&mut self, msg: &SolarClientMsg) {
        if let Ok(bytes) = encode(msg) {
            self.sender.send(WsMessage::Binary(bytes));
        }
    }

    /// Forward a channel or resolution change, once per change.
    ///
    /// Mirrors `Solar3d::ensure_feed`: the UI writes its preference into
    /// `Solar3dView` every frame, and only a difference is worth a message.
    pub fn sync_channel(&mut self, channel: u8, resolution: u16) {
        // Nothing sent before the handshake completes would arrive — and worse,
        // `sent_channel` would record it as sent. The first frames run while
        // the socket is still connecting, so this is the normal path, not an
        // edge case.
        if self.link != Link::Up || self.sent_channel == Some((channel, resolution)) {
            return;
        }
        let previous = self.sent_channel.replace((channel, resolution));
        match previous {
            Some((c, r)) => {
                if c != channel {
                    self.send(&SolarClientMsg::SetChannel(channel));
                }
                if r != resolution {
                    self.send(&SolarClientMsg::SetResolution(resolution));
                }
            }
            // First frame: the server already started its feed on the defaults,
            // so only say something if this viewer wants something else.
            None => {
                if channel != 0 {
                    self.send(&SolarClientMsg::SetChannel(channel));
                }
                if resolution != 1024 {
                    self.send(&SolarClientMsg::SetResolution(resolution));
                }
            }
        }
    }

    /// Drain the socket. Call once per frame.
    pub fn poll(&mut self) {
        while let Some(ev) = self.receiver.try_recv() {
            match ev {
                // The handshake has to wait for this, not go out at connect
                // time. In the browser a `WebSocket` is still `CONNECTING` when
                // its constructor returns, and `send` on it throws — so an
                // eagerly-sent Hello is silently lost and the server closes the
                // socket on its handshake timeout. Natively the socket is
                // already open by then, which is exactly what makes the bug
                // invisible until you run it in a browser.
                ewebsock::WsEvent::Opened => {
                    self.send(&SolarClientMsg::Hello { proto: SOLAR_PROTO_VERSION });
                }
                ewebsock::WsEvent::Message(WsMessage::Binary(bytes)) => {
                    match proto_decode::<SolarServerMsg>(&bytes) {
                        Ok(m) => self.apply(m),
                        Err(e) => {
                            self.link =
                                Link::Down(tr!("solarnet-bad-message", error = e.to_string()))
                        }
                    }
                }
                ewebsock::WsEvent::Message(_) => {}
                ewebsock::WsEvent::Error(e) => self.link = Link::Down(e),
                ewebsock::WsEvent::Closed => {
                    self.link = Link::Down(tr!("solarnet-connection-closed").into());
                }
            }
        }
    }

    /// Answer the server's challenge.
    pub fn send_auth(&mut self, username: String, password: String) {
        self.auth = sdroxide_types::AuthPhase::Checking;
        self.send(&SolarClientMsg::Auth { username, password });
    }

    fn apply(&mut self, msg: SolarServerMsg) {
        match msg {
            SolarServerMsg::HelloAck { proto } => {
                self.auth = sdroxide_types::AuthPhase::Open;
                self.link = if proto == SOLAR_PROTO_VERSION {
                    Link::Up
                } else {
                    Link::Down(tr!(
                        "solarnet-solar-protocol-mismatch-server-client",
                        proto = proto,
                        value = SOLAR_PROTO_VERSION
                    ))
                };
            }
            // The link is not down — the socket is open and the server is
            // talking — it is simply waiting for a password. Left as
            // `Connecting`, which is what it is.
            SolarServerMsg::AuthRequired => {
                self.auth = sdroxide_types::AuthPhase::Prompt(None);
            }
            SolarServerMsg::AuthRejected(why) => {
                self.auth = sdroxide_types::AuthPhase::Prompt(Some(why));
            }
            SolarServerMsg::Error(e) => self.link = Link::Down(e),
            SolarServerMsg::Pong => {}
            SolarServerMsg::Digi {
                my_grid,
                dx_grid,
                transmitting,
                dx_call,
                mode,
                freq_hz,
                qso,
            } => {
                self.my_grid = my_grid;
                self.dx_grid = dx_grid;
                self.transmitting = transmitting;
                self.dx_call = dx_call;
                self.mode = mode;
                self.freq_hz = freq_hz;
                self.qso = qso;
            }
            SolarServerMsg::Decodes(d) => self.decodes = d,
            SolarServerMsg::Propagation { halflife_s, planes } => {
                let mut f = sdroxide_types::PropField {
                    halflife_s,
                    generation: self.prop.generation.wrapping_add(1),
                    ..Default::default()
                };
                for (i, plane) in planes {
                    if let Some(slot) = f.planes.get_mut(i as usize) {
                        *slot = Some(plane);
                    }
                }
                self.prop = Arc::new(f);
            }
            SolarServerMsg::SatFreqs(freqs) => {
                self.sat_cfg = Arc::new(sdroxide_types::SatConfig { freqs, ..Default::default() });
            }
            // Decode outside the lock, exactly as the feed's worker does: a
            // 2048² JPEG is over a hundred milliseconds, which is far too long
            // to hold a lock the renderer takes every frame.
            SolarServerMsg::Sun { channel, fetched_unix, jpeg } => {
                let ch = SdoChannel::from_u8(channel);
                if let Some(img) = imagery::decode(&jpeg, ch, fetched_unix) {
                    let mut d = self.lock();
                    d.sun = Some(Arc::new(img));
                    d.sun_gen += 1;
                }
            }
            SolarServerMsg::Tles { geo, text } => {
                let sats = satellites::parse_tles(&text);
                let mut d = self.lock();
                if geo {
                    d.sats_geo = sats;
                } else {
                    d.sats_amateur = sats;
                }
            }
            SolarServerMsg::Aurora(oval) => {
                let mut d = self.lock();
                d.aurora = Some(Arc::new(oval));
                d.aurora_gen += 1;
            }
            // Same reasoning as the solar image above: decoding two 1024×512
            // PNGs and estimating the clear-sky background under them is tens
            // of milliseconds, and it all happens before the lock is taken.
            SolarServerMsg::Clouds { infrared, frame_unix, fetched_unix, png } => {
                let band = if infrared { clouds::Band::Longwave } else { clouds::Band::Visible };
                if let Some(plane) = clouds::parse_plane(band, &png, frame_unix, fetched_unix) {
                    let mut d = self.lock();
                    if infrared {
                        d.cloud_ir = Some(Arc::new(plane));
                    } else {
                        d.cloud_vis = Some(Arc::new(plane));
                    }
                    d.rebuild_clouds();
                }
            }
            SolarServerMsg::Weather { weather, aurora_power, kp_forecast } => {
                let mut d = self.lock();
                d.weather = weather;
                d.aurora_power = aurora_power;
                d.kp_forecast = kp_forecast;
            }
            SolarServerMsg::Events { cmes, flares, regions } => {
                let mut d = self.lock();
                d.cmes = cmes;
                d.flares = flares;
                d.regions = regions;
            }
            SolarServerMsg::Status(v) => {
                let mut d = self.lock();
                // A server with a different source list than this build would
                // otherwise shift every age readout by one column.
                for (slot, s) in d.status.iter_mut().zip(v) {
                    *slot = s;
                }
            }
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, SolarData> {
        self.data.lock().unwrap_or_else(|e| e.into_inner())
    }
}
