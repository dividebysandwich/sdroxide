//! `/solar-ws`: the read-only feed behind the browser's solar-system view.
//!
//! Three things make this a separate route rather than more traffic on `/ws`:
//!
//! * **It is a viewer, not a controller.** Nothing here reaches `cmd_tx`; the
//!   only inbound messages pick an SDO channel and ask for a refresh. So it does
//!   not take the single-client slot `/ws` guards with `Shared::busy`, and any
//!   number of maps can be open without touching who owns PTT.
//! * **The feed is expensive and optional.** [`SolarFeed`] talks to thirteen
//!   outbound endpoints. It starts when the first viewer connects and is dropped
//!   when the last one leaves — the same contract the native window has, and the
//!   one the manual promises: nobody watching means no outbound request at all.
//! * **It is separately versioned.** See [`sdroxide_proto::solar`].
//!
//! The feed is a synchronous, thread-owning thing and this is a tokio server, so
//! one bridge thread (`sdroxide-solarpump`) owns the snapshot handle, diffs it,
//! and publishes into a `broadcast` that each viewer's socket task forwards.
//! That is the same shape as [`crate::pump`], for the same reason.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use sdroxide_proto::solar::{SOLAR_PROTO_VERSION, SolarClientMsg, SolarServerMsg};
use sdroxide_proto::{decode, encode};
use sdroxide_solar::{FeedCmd, RawUpdate, SdoChannel, SolarData, SolarFeed};

use crate::Station;

/// How often the bridge thread re-reads the snapshot. The feed's own wake is
/// what usually drives a publish; this is the floor, and it is generous because
/// nothing here changes faster than every few minutes.
const POLL: Duration = Duration::from_millis(250);

/// Enough for the snapshot burst plus a comfortable backlog. A viewer that
/// falls this far behind is not going to catch up, and lagging it is better
/// than stalling the bridge for everyone else.
const BROADCAST_CAP: usize = 64;

/// What every viewer needs on connect, kept because a `broadcast` has no
/// history and the feed only republishes when something changes.
#[derive(Default)]
pub(crate) struct SolarLatest {
    pub sun: Option<SolarServerMsg>,
    pub aurora: Option<SolarServerMsg>,
    /// The two cloud channels, kept apart so a refreshed infrared frame does
    /// not drop the visible one a viewer has not seen yet.
    pub clouds_ir: Option<SolarServerMsg>,
    pub clouds_vis: Option<SolarServerMsg>,
    pub tles_amateur: Option<SolarServerMsg>,
    pub tles_geo: Option<SolarServerMsg>,
    pub weather: Option<SolarServerMsg>,
    pub events: Option<SolarServerMsg>,
    pub status: Option<SolarServerMsg>,
    /// Relayed from the radio's own event stream, not from the solar feed.
    pub digi: Option<SolarServerMsg>,
    pub decodes: Option<SolarServerMsg>,
    /// The operator's satellite frequency overrides, likewise from the radio
    /// side: they are part of the station config the engine announces.
    pub sat_freqs: Option<SolarServerMsg>,
    /// The propagation field, likewise built from what the radio decodes.
    pub prop: Option<SolarServerMsg>,
}

impl SolarLatest {
    /// The connect burst, cheapest first: the panels and the globe come up
    /// before the ~150 kB solar disk arrives.
    fn snapshot(&self) -> Vec<SolarServerMsg> {
        [
            &self.status,
            &self.weather,
            &self.events,
            &self.digi,
            &self.decodes,
            &self.prop,
            &self.sat_freqs,
            &self.tles_amateur,
            &self.tles_geo,
            &self.aurora,
            &self.clouds_ir,
            &self.clouds_vis,
            &self.sun,
        ]
        .into_iter()
        .flatten()
        .cloned()
        .collect()
    }

    /// Forget everything the solar feed produced, leaving what the radio
    /// produced alone.
    ///
    /// The two have different lifecycles. Feed products die with the feed:
    /// serving them to a viewer half an hour later would present the readings
    /// of a stopped feed as current. `digi`, `decodes` and `sat_freqs` come
    /// from the radio, which is still running — and both the operator config
    /// and the station config are announced at engine start, so dropping them
    /// means no later viewer ever learns the QTH (the globe loses its home
    /// marker and its QSO arcs) or the operator's satellite frequencies.
    fn clear_feed_products(&mut self) {
        self.sun = None;
        self.aurora = None;
        self.clouds_ir = None;
        self.clouds_vis = None;
        self.tles_amateur = None;
        self.tles_geo = None;
        self.weather = None;
        self.events = None;
        self.status = None;
    }

    fn record(&mut self, msg: &SolarServerMsg) {
        let slot = match msg {
            SolarServerMsg::Sun { .. } => &mut self.sun,
            SolarServerMsg::Aurora(_) => &mut self.aurora,
            SolarServerMsg::Clouds { infrared: true, .. } => &mut self.clouds_ir,
            SolarServerMsg::Clouds { infrared: false, .. } => &mut self.clouds_vis,
            SolarServerMsg::Tles { geo: false, .. } => &mut self.tles_amateur,
            SolarServerMsg::Tles { geo: true, .. } => &mut self.tles_geo,
            SolarServerMsg::Weather { .. } => &mut self.weather,
            SolarServerMsg::Events { .. } => &mut self.events,
            SolarServerMsg::Status(_) => &mut self.status,
            SolarServerMsg::Digi { .. } => &mut self.digi,
            SolarServerMsg::Decodes(_) => &mut self.decodes,
            SolarServerMsg::SatFreqs(_) => &mut self.sat_freqs,
            SolarServerMsg::Propagation { .. } => &mut self.prop,
            // Per-connection handshake traffic is not part of a snapshot.
            SolarServerMsg::HelloAck { .. }
            | SolarServerMsg::Error(_)
            | SolarServerMsg::Pong
            | SolarServerMsg::AuthRequired
            | SolarServerMsg::AuthRejected(_) => {
                return;
            }
        };
        *slot = Some(msg.clone());
    }
}

/// How often the propagation field is relayed, at most.
///
/// It is a few hundred kilobytes of grid and it moves on every decode; a
/// viewer needs it to be roughly current, not to the second. Thirty seconds is
/// well inside the field's own 45-minute memory.
const PROP_RELAY_INTERVAL: Duration = Duration::from_secs(30);

/// The viewer registry and the feed it keeps alive.
pub(crate) struct SolarHub {
    tx: broadcast::Sender<Arc<SolarServerMsg>>,
    pub latest: Mutex<SolarLatest>,
    /// The propagation field, folded from the decodes that pass through this
    /// server.
    ///
    /// Built here rather than relayed from a client: the solar tab has its own
    /// socket and no logbook, no decode window and no operator identity of its
    /// own, so the only place that can turn a stream of decodes into a field is
    /// the station itself.
    prop: Mutex<(sdroxide_types::PropStore, Option<Instant>)>,
    /// Guards the feed handle and the viewer count together: they change as one
    /// (first viewer starts, last viewer stops) and must never disagree.
    feed: Mutex<FeedState>,
}

struct FeedState {
    viewers: usize,
    feed: Option<SolarFeed>,
    /// Tells the bridge thread to exit. A generation counter rather than a
    /// flag, so a feed restarted before the old thread noticed cannot make the
    /// new thread exit.
    generation: u64,
    channel: SdoChannel,
    resolution: u32,
    /// The operator's satellite additions, as the engine last announced them.
    /// Held rather than pushed straight through because the feed is only alive
    /// while somebody is watching: a subscription added with the map shut has
    /// to be waiting for the next feed that starts.
    sat: sdroxide_types::SatConfig,
}

impl Default for SolarHub {
    fn default() -> Self {
        SolarHub {
            tx: broadcast::channel(BROADCAST_CAP).0,
            latest: Mutex::new(SolarLatest::default()),
            prop: Mutex::new((Default::default(), None)),
            feed: Mutex::new(FeedState {
                viewers: 0,
                feed: None,
                generation: 0,
                // Matches `Solar3dView::default()`: HMI continuum at 1024², so
                // the first thing a viewer sees is white light with real spots.
                channel: SdoChannel::from_u8(0),
                resolution: 1024,
                sat: Default::default(),
            }),
        }
    }
}

impl SolarHub {
    /// Publish to every viewer and remember it for the next one's snapshot.
    pub(crate) fn publish(&self, msg: SolarServerMsg) {
        self.latest.lock().unwrap().record(&msg);
        // An error here only means nobody is watching.
        let _ = self.tx.send(Arc::new(msg));
    }

    /// Fold decodes into the propagation field, relaying it if it is due.
    ///
    /// `grid` is the operator's own locator, without which there is no path to
    /// speak of; the store forgets everything if it changes, which is what
    /// keeps a field of paths from an address the station has left off the map.
    pub(crate) fn observe_decodes(
        &self,
        decodes: &[sdroxide_types::Decode],
        source: sdroxide_types::PropSource,
        dial_hz: f64,
        grid: &str,
        now_utc: i64,
    ) {
        let mut g = self.prop.lock().unwrap();
        g.0.set_home(grid);
        g.0.observe_decodes(decodes, source, dial_hz, grid, now_utc);
        self.relay_prop(&mut g, now_utc);
    }

    /// The same for WSPR receptions.
    pub(crate) fn observe_wspr(
        &self,
        spots: &[sdroxide_types::WsprSpot],
        grid: &str,
        now_utc: i64,
    ) {
        let mut g = self.prop.lock().unwrap();
        g.0.set_home(grid);
        g.0.observe_wspr(spots, grid, now_utc);
        self.relay_prop(&mut g, now_utc);
    }

    /// The same for paths between two other stations.
    ///
    /// No `set_home`, and deliberately: these paths are not measured from here,
    /// so they neither need the operator's locator nor should be forgotten when
    /// it changes. A skimmer network's view of the ionosphere is the same view
    /// whoever is watching it.
    pub(crate) fn observe_paths(
        &self,
        paths: &[sdroxide_types::PropPath],
        source: sdroxide_types::PropSource,
        now_utc: i64,
    ) {
        let mut g = self.prop.lock().unwrap();
        g.0.observe_paths(paths, source, now_utc);
        self.relay_prop(&mut g, now_utc);
    }

    /// Send the field on, no more often than [`PROP_RELAY_INTERVAL`].
    fn relay_prop(&self, g: &mut (sdroxide_types::PropStore, Option<Instant>), now_utc: i64) {
        let due = g.1.is_none_or(|t| t.elapsed() >= PROP_RELAY_INTERVAL);
        if !due {
            return;
        }
        let field = g.0.field(now_utc);
        // Only the live bands: a station on one band sends one plane rather
        // than fourteen, thirteen of them empty.
        let planes: Vec<(u8, sdroxide_types::BandPlane)> = sdroxide_types::Band::ALL
            .iter()
            .enumerate()
            .filter_map(|(i, b)| field.plane(*b).map(|p| (i as u8, p.clone())))
            .collect();
        g.1 = Some(Instant::now());
        if planes.is_empty() {
            return;
        }
        self.publish(SolarServerMsg::Propagation { halflife_s: field.halflife_s, planes });
    }

    fn send_cmd(&self, cmd: FeedCmd) {
        if let Some(f) = self.feed.lock().unwrap().feed.as_ref() {
            f.send(cmd);
        }
    }

    /// Adopt the station's satellite additions: hand the element sets to the
    /// feed, and the frequency overrides to the viewers.
    ///
    /// This is what makes the TLE tab mean something for a browser viewer. The
    /// tab edits the engine host's config, and both halves of what it changes
    /// live here: the tracker those satellites appear in is the feed *here*
    /// (without which the subscriptions would be persisted and never fetched,
    /// and the sky would come up with nothing but the geostationary belt), and
    /// the frequency table the pass window shows is a product this hub relays
    /// (without which a viewer shows the published figures where the operator
    /// has deliberately corrected them).
    ///
    /// The frequencies are published unconditionally rather than only on a
    /// change: [`SolarHub::publish`] is also what records the snapshot the next
    /// viewer receives, and the first announcement can be identical to the
    /// default this started on.
    pub(crate) fn set_sat_config(&self, cfg: sdroxide_types::SatConfig) {
        // Before the feed lock, not inside it: `publish` takes `latest`, and
        // the fewer places that hold two of these at once the better.
        self.publish(SolarServerMsg::SatFreqs(cfg.freqs.clone()));
        let mut st = self.feed.lock().unwrap();
        if st.sat == cfg {
            return;
        }
        st.sat = cfg;
        if let Some(f) = st.feed.as_ref() {
            for cmd in feed_tle_cmds(&st.sat) {
                f.send(cmd);
            }
        }
    }
}

/// What a feed has to be told to track the operator's satellites: the pasted
/// element sets, then the listings to keep fetched.
fn feed_tle_cmds(cfg: &sdroxide_types::SatConfig) -> [FeedCmd; 2] {
    [
        FeedCmd::SetCustomTles(cfg.tle_text()),
        FeedCmd::SetTleSubs(cfg.live_subs().cloned().collect()),
    ]
}

pub async fn ws_route(
    State(station): State<Arc<Station>>,
    headers: axum::http::HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Response {
    // The feed and the door, not a radio: this endpoint shows the sky over the
    // station, which is the same sky however many radios stand under it.
    let (hub, auth) = (station.solar.clone(), station.auth.clone());
    // The sky is its own connection and asks for a password like any other, so
    // it has to honour the cookie too — otherwise the operator has been told
    // "you are signed in for the day" and then gets asked again for the tab
    // with the planets in it.
    let signed = crate::session_cookie::from_headers(&headers, &station.auth);
    upgrade.on_upgrade(move |socket| session(socket, hub, auth, signed))
}

fn msg(m: &SolarServerMsg) -> Message {
    Message::Binary(encode(m).expect("encode").into())
}

async fn session(
    mut socket: WebSocket,
    hub: Arc<SolarHub>,
    auth: Arc<crate::auth::AuthGate>,
    signed: Option<String>,
) {
    // Subscribe *before* the snapshot, so an update that lands between the two
    // is queued rather than missed.
    let mut rx = hub.tx.subscribe();

    // --- Hello handshake (5 s budget) ---------------------------------
    let hello = tokio::time::timeout(Duration::from_secs(5), socket.recv()).await;
    match hello {
        Ok(Some(Ok(Message::Binary(bytes)))) => match decode::<SolarClientMsg>(&bytes) {
            Ok(SolarClientMsg::Hello { proto }) if proto == SOLAR_PROTO_VERSION => {}
            Ok(SolarClientMsg::Hello { proto }) => {
                let _ = socket
                    .send(msg(&SolarServerMsg::Error(format!(
                        "solar protocol mismatch: server {SOLAR_PROTO_VERSION}, client {proto}"
                    ))))
                    .await;
                return;
            }
            _ => {
                let _ = socket.send(msg(&SolarServerMsg::Error("expected Hello".into()))).await;
                return;
            }
        },
        _ => return,
    }

    // A viewer controls nothing, but it is shown the operator's QTH, every
    // station they are decoding and every satellite frequency they have
    // corrected — so the same door applies here. Before `acquire_feed`, so a
    // stranger cannot start thirteen outbound fetches by opening a socket.
    let signed_in = crate::auth::challenge(
        &mut socket,
        &auth,
        crate::auth::Frames {
            what: "/solar-ws",
            required: encode(&SolarServerMsg::AuthRequired).expect("encode"),
            rejected: &|why| encode(&SolarServerMsg::AuthRejected(why.into())).expect("encode"),
            credentials: &|bytes| match decode::<SolarClientMsg>(bytes) {
                Ok(SolarClientMsg::Auth { username, password }) => Some((username, password)),
                _ => None,
            },
        },
        signed.is_some(),
    )
    .await;
    if !signed_in {
        let _ = socket.close().await;
        return;
    }

    if socket.send(msg(&SolarServerMsg::HelloAck { proto: SOLAR_PROTO_VERSION })).await.is_err() {
        return;
    }

    let viewers = acquire_feed(&hub);
    info!(viewers, "solar viewer connected");

    // Collect and release before sending: the guard must not be held across an
    // await, or the whole session future stops being `Send` and every publisher
    // blocks behind one slow socket.
    let snapshot = hub.latest.lock().unwrap().snapshot();
    for m in snapshot {
        if socket.send(msg(&m)).await.is_err() {
            release_feed(&hub);
            return;
        }
    }

    let (mut ws_tx, mut ws_rx) = futures_util::StreamExt::split(socket);

    let sender = async {
        loop {
            match rx.recv().await {
                Ok(m) => {
                    if ws_tx.send(msg(&m)).await.is_err() {
                        break;
                    }
                }
                // A viewer that fell behind has missed updates but is still
                // connected; the next publish of each product brings it back
                // into step, so keep going rather than dropping it.
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("solar viewer lagged {n} messages");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    let receiver = async {
        while let Some(Ok(m)) = ws_rx.next().await {
            let Message::Binary(bytes) = m else { continue };
            match decode::<SolarClientMsg>(&bytes) {
                // The feed is shared, so this changes the picture for every
                // viewer. Running one feed per viewer would multiply the
                // outbound traffic to NASA by the number of open tabs.
                Ok(SolarClientMsg::SetChannel(c)) => {
                    let ch = SdoChannel::from_u8(c);
                    hub.feed.lock().unwrap().channel = ch;
                    hub.send_cmd(FeedCmd::SetChannel(ch));
                }
                Ok(SolarClientMsg::SetResolution(r)) => {
                    let res = u32::from(r);
                    hub.feed.lock().unwrap().resolution = res;
                    hub.send_cmd(FeedCmd::SetResolution(res));
                }
                Ok(SolarClientMsg::RefreshAll) => hub.send_cmd(FeedCmd::RefreshAll),
                Ok(SolarClientMsg::Ping) => hub.publish(SolarServerMsg::Pong),
                Ok(SolarClientMsg::Hello { .. }) => {}
                // A late `Auth` is ignored for the same reason `/ws` ignores
                // one: this viewer is already in, and re-running the gate would
                // let it hold everybody else's sign-in up.
                Ok(SolarClientMsg::Auth { .. }) => {}
                Err(e) => debug!("solar viewer sent an undecodable message: {e}"),
            }
        }
    };

    tokio::select! {
        _ = sender => {}
        _ = receiver => {}
    }

    let left = release_feed(&hub);
    info!(viewers = left, "solar viewer disconnected");
}

/// Register a viewer, starting the feed if this is the first. Returns the new
/// viewer count.
fn acquire_feed(hub: &Arc<SolarHub>) -> usize {
    let mut st = hub.feed.lock().unwrap();
    st.viewers += 1;
    if st.feed.is_some() {
        return st.viewers;
    }

    st.generation += 1;
    let generation = st.generation;
    let (raw_tx, raw_rx) = crossbeam_channel::unbounded();
    let (wake_tx, wake_rx) = crossbeam_channel::bounded(1);
    let feed = SolarFeed::start_with_raw(
        st.channel,
        st.resolution,
        // Latest-wins: a full slot already means "there is work to do".
        move || {
            let _ = wake_tx.try_send(());
        },
        Some(raw_tx),
    );
    let data = feed.shared();
    // Whatever the operator has subscribed to, before the first fetch: the
    // cached listings go out immediately, so a viewer opening the map sees the
    // sky it saw last time rather than an empty one for the first few seconds.
    for cmd in feed_tle_cmds(&st.sat) {
        feed.send(cmd);
    }
    st.feed = Some(feed);
    let viewers = st.viewers;
    drop(st);

    let hub = Arc::clone(hub);
    std::thread::Builder::new()
        .name("sdroxide-solarpump".into())
        .spawn(move || solar_pump(hub, data, raw_rx, wake_rx, generation))
        .expect("spawn solar pump thread");
    info!("solar feed started");
    viewers
}

/// Deregister a viewer, dropping the feed when the last one goes. Returns the
/// remaining viewer count.
fn release_feed(hub: &Arc<SolarHub>) -> usize {
    let mut st = hub.feed.lock().unwrap();
    st.viewers = st.viewers.saturating_sub(1);
    if st.viewers == 0 {
        // Dropping the handle disconnects the worker's command channel, which
        // is how it learns to stop. This is what confines every outbound
        // request to the lifetime of an open map.
        st.feed = None;
        st.generation += 1;
        // The feed's cached products go with it: serving a viewer ten minutes
        // from now with readings from a feed that has since stopped would
        // present stale numbers as current. What the radio published stays.
        hub.latest.lock().unwrap().clear_feed_products();
        info!("solar feed stopped: no viewers left");
    }
    st.viewers
}

/// Bridge thread: the feed's snapshot and raw payloads → broadcast messages.
fn solar_pump(
    hub: Arc<SolarHub>,
    data: Arc<Mutex<SolarData>>,
    raw_rx: crossbeam_channel::Receiver<RawUpdate>,
    wake_rx: crossbeam_channel::Receiver<()>,
    generation: u64,
) {
    let mut sent = SentMarks::default();

    loop {
        // Either the feed woke us or the poll floor expired; both mean "look".
        let _ = wake_rx.recv_timeout(POLL);

        if hub.feed.lock().unwrap().generation != generation {
            break;
        }

        // Raw payloads first: a viewer should not see the status say the image
        // is current before the image itself has been published.
        while let Ok(u) = raw_rx.try_recv() {
            match u {
                RawUpdate::Sun { channel, fetched_unix, jpeg } => {
                    hub.publish(SolarServerMsg::Sun {
                        channel: channel.to_u8(),
                        fetched_unix,
                        jpeg,
                    });
                }
                RawUpdate::Tle { geo, text } => {
                    hub.publish(SolarServerMsg::Tles { geo, text });
                }
                RawUpdate::Clouds { band, frame_unix, fetched_unix, png } => {
                    hub.publish(SolarServerMsg::Clouds {
                        infrared: band == sdroxide_solar::Band::Longwave,
                        frame_unix,
                        fetched_unix,
                        png,
                    });
                }
            }
        }

        // Everything else is a diff against what this thread last published.
        let (weather, events, status, aurora) = {
            let d = data.lock().unwrap_or_else(|e| e.into_inner());
            let weather = (sent.weather.as_ref() != Some(&d.weather)
                || sent.aurora_power.as_ref() != Some(&d.aurora_power)
                || sent.kp_forecast.as_ref() != Some(&d.kp_forecast))
            .then(|| {
                sent.weather = Some(d.weather.clone());
                sent.aurora_power = Some(d.aurora_power);
                sent.kp_forecast = Some(d.kp_forecast.clone());
                SolarServerMsg::Weather {
                    weather: d.weather.clone(),
                    aurora_power: d.aurora_power,
                    kp_forecast: d.kp_forecast.clone(),
                }
            });

            let events = (sent.cmes.as_ref() != Some(&d.cmes)
                || sent.flares.as_ref() != Some(&d.flares)
                || sent.regions.as_ref() != Some(&d.regions))
            .then(|| {
                sent.cmes = Some(d.cmes.clone());
                sent.flares = Some(d.flares.clone());
                sent.regions = Some(d.regions.clone());
                SolarServerMsg::Events {
                    cmes: d.cmes.clone(),
                    flares: d.flares.clone(),
                    regions: d.regions.clone(),
                }
            });

            let status = (sent.status.as_deref() != Some(&d.status[..])).then(|| {
                sent.status = Some(d.status.to_vec());
                SolarServerMsg::Status(d.status.to_vec())
            });

            // The oval is the one product whose generation counter is the
            // cheapest thing to compare — the grid itself is 65 kB.
            let aurora = (d.aurora_gen != sent.aurora_gen)
                .then(|| {
                    sent.aurora_gen = d.aurora_gen;
                    d.aurora.as_deref().cloned().map(SolarServerMsg::Aurora)
                })
                .flatten();

            (weather, events, status, aurora)
        };

        for m in [weather, events, aurora, status].into_iter().flatten() {
            hub.publish(m);
        }
    }
    debug!("solar pump thread stopped");
}

/// What the bridge thread has already published, so it sends diffs rather than
/// the whole snapshot every quarter second.
#[derive(Default)]
struct SentMarks {
    weather: Option<sdroxide_solar::SpaceWeather>,
    aurora_power: Option<Option<sdroxide_solar::HemisphericPower>>,
    kp_forecast: Option<Vec<sdroxide_solar::KpPoint>>,
    cmes: Option<Vec<sdroxide_solar::CmeEvent>>,
    flares: Option<Vec<sdroxide_solar::FlareEvent>>,
    regions: Option<Vec<sdroxide_solar::ActiveRegion>>,
    status: Option<Vec<sdroxide_solar::SourceStatus>>,
    aurora_gen: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_solar::{AuroraOval, Source};

    /// The connect burst has to put the cheap products first: a viewer that
    /// waits on a megabyte of solar disk before it can draw the globe looks
    /// broken for the whole download.
    #[test]
    fn the_snapshot_sends_the_image_last() {
        let mut l = SolarLatest::default();
        for m in [
            SolarServerMsg::Sun { channel: 0, fetched_unix: 1, jpeg: vec![1] },
            SolarServerMsg::Status(vec![Default::default(); Source::ALL.len()]),
            SolarServerMsg::Weather {
                weather: Default::default(),
                aurora_power: None,
                kp_forecast: Vec::new(),
            },
        ] {
            l.record(&m);
        }
        let snap = l.snapshot();
        assert_eq!(snap.len(), 3);
        assert!(matches!(snap[0], SolarServerMsg::Status(_)));
        assert!(matches!(snap.last(), Some(SolarServerMsg::Sun { .. })));
    }

    /// Handshake traffic is per-connection. Caching a `HelloAck` would replay
    /// somebody else's handshake into the next viewer's snapshot.
    #[test]
    fn handshake_messages_never_enter_the_snapshot() {
        let mut l = SolarLatest::default();
        l.record(&SolarServerMsg::HelloAck { proto: SOLAR_PROTO_VERSION });
        l.record(&SolarServerMsg::Pong);
        l.record(&SolarServerMsg::Error("boom".into()));
        assert!(l.snapshot().is_empty());
    }

    /// Stopping the feed must not discard what the radio said. The operator
    /// config and the station config are both announced at engine start; if
    /// either is dropped when the last viewer leaves, every later viewer comes
    /// up without it — no QTH, so no home marker and no QSO arcs, or no
    /// frequency overrides, so the pass window shows the published figures
    /// where the operator has deliberately corrected them — and nothing will
    /// ever re-send it.
    #[test]
    fn stopping_the_feed_keeps_the_radio_side_of_the_snapshot() {
        let mut l = SolarLatest::default();
        l.record(&SolarServerMsg::Digi {
            my_grid: "JN78ve".into(),
            dx_grid: None,
            transmitting: false,
            dx_call: None,
            mode: "FT8".into(),
            freq_hz: 14_074_000.0,
            qso: None,
        });
        l.record(&SolarServerMsg::Decodes(Vec::new()));
        l.record(&SolarServerMsg::SatFreqs(vec![sdroxide_types::SatFreqs::new(
            25_544,
            "ISS",
            Vec::new(),
        )]));
        l.record(&SolarServerMsg::Sun { channel: 0, fetched_unix: 1, jpeg: vec![1] });
        l.record(&SolarServerMsg::Aurora(AuroraOval {
            observed_unix: 1,
            forecast_unix: 2,
            grid: vec![0],
        }));

        l.clear_feed_products();
        let snap = l.snapshot();
        assert_eq!(snap.len(), 3, "expected only the radio's messages, got {snap:?}");
        assert!(snap.iter().any(|m| matches!(m, SolarServerMsg::Digi { .. })));
        assert!(snap.iter().any(|m| matches!(m, SolarServerMsg::Decodes(_))));
        assert!(snap.iter().any(|m| matches!(m, SolarServerMsg::SatFreqs(_))));
    }

    /// Both element sets have to survive independently, or QO-100 overwrites
    /// the amateur list and the map loses every LEO satellite.
    #[test]
    fn the_two_element_sets_have_their_own_slots() {
        let mut l = SolarLatest::default();
        l.record(&SolarServerMsg::Tles { geo: false, text: "amateur".into() });
        l.record(&SolarServerMsg::Tles { geo: true, text: "qo100".into() });
        let snap = l.snapshot();
        assert_eq!(snap.len(), 2);
        assert!(snap.contains(&SolarServerMsg::Tles { geo: false, text: "amateur".into() }));
        assert!(snap.contains(&SolarServerMsg::Tles { geo: true, text: "qo100".into() }));
    }
}
