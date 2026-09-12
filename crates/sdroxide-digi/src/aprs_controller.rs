//! `AprsController` — the Automatic Packet Reporting System.
//!
//! The modem, the framing and CSMA are [`crate::ax25_channel::Ax25Channel`]'s,
//! shared with [`crate::PacketController`]: on the air an APRS frame is an
//! ordinary 1200 baud AX.25 UI frame and nothing about the link layer differs.
//! What is here is everything above it — the codec in `sdroxide-aprs`, the
//! stations it builds into a map, the message layer with its retries, and the
//! beacon.
//!
//! # What makes APRS different from packet
//!
//! **It is one channel, not a band.** Every station in radio range of every
//! other shares it, and the region decides which frequency (see
//! `sdroxide_types::aprs_dial`). That is why the transmit side is as
//! conservative as it is: the beacon is off until the operator turns it on,
//! the message retries back off, and the digipeater path is a setting with a
//! warning attached.
//!
//! **Everything is broadcast.** There is no connection, no sequence number and
//! no acknowledgement — except for messages, which bolt one on top by carrying
//! an identifier that the addressee echoes back. So a message is the only
//! thing here with a timer, and everything else is fire-and-forget.
//!
//! **Frames arrive several times.** A digipeater repeats what it hears, and on
//! a busy channel two or three of them repeat the same frame. Every incoming
//! path therefore has to be idempotent: a position simply overwrites, and a
//! message is de-duplicated by (sender, identifier) — but *re-acknowledged*,
//! because a repeat usually means the sender did not hear the first ack.

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sdroxide_aprs::{AprsData, MessageKind};
use sdroxide_ax25::{Addr, Packet, PacketType};
use sdroxide_types::{
    APRS_MESSAGE_MAX, APRS_MSG_RETRIES, APRS_STATION_MAX, APRS_TRACK_MAX, APRS_TRAFFIC_MAX,
    AprsEntryKind, AprsMessage, AprsMsgState, AprsPosition, AprsStation, AprsStatus, AprsSymbol,
    AprsTraffic, DigiConfig, DigiStatus, Mode, PacketBaud, QsoStep, TranscriptLine,
};

use crate::DigiEngine;
use crate::ax25_channel::Ax25Channel;
use crate::controller::DigiAction;

/// The destination address our frames carry.
///
/// On APRS the AX.25 destination is not an address at all — nothing is
/// addressed to it and no station answers it. It is a *software identifier*,
/// and the convention is `AP` followed by up to four characters from a
/// registry, with the `APZ` prefix set aside for software that is not in it.
/// That is where a program like this belongs: `APZSDR` says "sdroxide" to
/// anyone reading a raw feed and claims nothing that has been allocated.
///
/// (The one format that *does* use the destination for data is Mic-E, which
/// this station receives and does not transmit — see `sdroxide_aprs::mice`.)
const TOCALL: &str = "APZSDR";

/// First retry of an unacknowledged message, in seconds.
///
/// Thirty rather than the ten a link layer would use: the addressee may be a
/// mobile behind a hill, the acknowledgement has to travel back through the
/// same digipeaters, and a channel this crowded punishes impatience. The
/// interval doubles on each attempt after that.
const MSG_RETRY_S: u64 = 30;

/// A moving station has to have moved this far before the map keeps a new
/// track point.
///
/// Twenty metres: below it a stationary station's GPS jitter would fill the
/// track with a smudge, and above it a car turning a corner would cut it.
const TRACK_MIN_M: f64 = 20.0;

/// A message of ours waiting for its acknowledgement.
struct Outgoing {
    to: String,
    text: String,
    id: String,
    tries: u8,
    next_try: SystemTime,
}

pub struct AprsController {
    cfg: DigiConfig,
    tap_rate: f64,
    ch: Ax25Channel,

    stations: Vec<AprsStation>,
    messages: Vec<AprsMessage>,
    traffic: Vec<AprsTraffic>,
    bad_frames: u32,
    non_aprs: u32,

    /// Messages of ours still being retried.
    outbox: Vec<Outgoing>,
    /// The next message identifier to mint. Wraps at 1000 — the field is five
    /// characters but nobody needs more than three, and a short one is three
    /// fewer bytes on every message.
    next_id: u32,
    /// `(sender, identifier)` of every message already delivered, so a
    /// digipeated repeat is acknowledged again without appearing twice.
    seen_msgs: HashMap<String, i64>,

    /// When the next beacon is due. `None` disables it.
    next_beacon: Option<SystemTime>,

    queued: Vec<DigiAction>,
    status_dirty: bool,
    last_status: Option<SystemTime>,
}

impl AprsController {
    pub fn new(cfg: DigiConfig, tap_rate: f64) -> Self {
        AprsController {
            // APRS is 1200 baud Bell 202 and nothing else. The 9600 and 300
            // baud settings belong to the packet mode; a stale one from a
            // packet session must not follow the operator onto the APRS
            // channel and leave them decoding nothing.
            ch: Ax25Channel::new(PacketBaud::Vhf1200, &cfg, tap_rate),
            cfg,
            tap_rate,
            stations: Vec::new(),
            messages: Vec::new(),
            traffic: Vec::new(),
            bad_frames: 0,
            non_aprs: 0,
            outbox: Vec::new(),
            next_id: 1,
            seen_msgs: HashMap::new(),
            next_beacon: None,
            queued: Vec::new(),
            status_dirty: true,
            last_status: None,
        }
    }

    /// Our own address, or `None` when the operator has not set one.
    ///
    /// An APRS station with no callsign must not transmit — it would be an
    /// unidentified signal, which is illegal everywhere. Every transmit path
    /// goes through this and gives up quietly when it returns `None`.
    fn mycall(&self) -> Option<Addr> {
        let call = self.cfg.aprs_call();
        if call.is_empty() {
            return None;
        }
        Addr::new(&call).ok()
    }

    /// The digipeater path as addresses.
    fn via(&self) -> Vec<Addr> {
        sdroxide_aprs::parse_path(&self.cfg.aprs_path)
            .iter()
            .filter_map(|p| Addr::new(p).ok())
            .collect()
    }

    /// Where the operator says this station is.
    ///
    /// A locator is a square, not a point, and it comes back saying so: a
    /// six-character one is a couple of kilometres across, which is honest for
    /// a fixed station and is exactly what position ambiguity is for.
    fn my_position(&self) -> Option<AprsPosition> {
        if self.cfg.aprs_use_grid {
            sdroxide_aprs::position_from_grid(&self.cfg.my_grid)
        } else if self.cfg.aprs_lat == 0.0 && self.cfg.aprs_lon == 0.0 {
            // Null Island is not a position, it is an empty settings dialog.
            None
        } else {
            Some(AprsPosition { lat: self.cfg.aprs_lat, lon: self.cfg.aprs_lon, ambiguity: 0 })
        }
    }

    /// Wrap an information field in a UI frame and hand it to CSMA.
    fn transmit(&mut self, info: String) {
        let (Some(me), Ok(dest)) = (self.mycall(), Addr::new(TOCALL)) else {
            return;
        };
        let p = Packet::ui_via(me, dest, self.via(), info.into_bytes());
        self.ch.queue(p.serialize(false));
        self.status_dirty = true;
    }

    /// Send one position beacon, if there is a callsign and a position.
    pub fn queue_beacon(&mut self) {
        let Some(pos) = self.my_position() else { return };
        let sym = self.cfg.aprs_symbol;
        let comment = self.cfg.aprs_comment.clone();
        // Messaging-capable unless the operator has switched acknowledgements
        // off: the bit tells other stations whether it is worth writing to us,
        // and claiming it while refusing to answer is worse than not claiming
        // it.
        let messaging = self.cfg.aprs_ack_messages;
        let info = if self.cfg.aprs_compressed {
            sdroxide_aprs::encode_compressed_position(pos, sym, &comment, messaging)
        } else {
            sdroxide_aprs::encode_position(pos, sym, &comment, messaging)
        };
        self.transmit(info);
    }

    /// Queue a message to `to`.
    ///
    /// It is given an identifier, which is what asks the addressee to
    /// acknowledge it, and it is retried until they do or until
    /// [`APRS_MSG_RETRIES`] attempts have gone by.
    pub fn queue_message(&mut self, to: String, text: String) {
        if self.mycall().is_none() || to.trim().is_empty() || text.trim().is_empty() {
            return;
        }
        let to = to.trim().to_ascii_uppercase();
        let id = format!("{}", self.next_id);
        self.next_id = self.next_id % 999 + 1;
        let at = now_unix();
        self.messages.push(AprsMessage {
            at,
            from: self.cfg.aprs_call(),
            to: to.clone(),
            text: text.clone(),
            id: id.clone(),
            state: AprsMsgState::Queued,
            tries: 0,
        });
        self.trim_messages();
        self.outbox.push(Outgoing {
            to,
            text,
            id,
            tries: 0,
            // Now: the first attempt goes out with the next clear slot.
            next_try: SystemTime::now(),
        });
        self.status_dirty = true;
    }

    /// Stop retrying one of our messages and say what became of it.
    fn close_outgoing(&mut self, peer: &str, id: &str, state: AprsMsgState) {
        self.outbox.retain(|o| !(o.id == id && o.to.eq_ignore_ascii_case(peer)));
        self.set_msg_state(id, peer, state);
    }

    /// Move a message of ours to a new state, by identifier.
    fn set_msg_state(&mut self, id: &str, to: &str, state: AprsMsgState) {
        for m in self.messages.iter_mut().rev() {
            if m.id == id && m.to == to && m.from != to {
                m.state = state;
                break;
            }
        }
        self.status_dirty = true;
    }

    /// One received frame.
    fn on_frame(&mut self, bytes: &[u8]) {
        let Ok(p) = Packet::parse(bytes, None) else {
            // It passed a 16-bit check sequence, so it is almost certainly a
            // real frame of a kind the codec does not handle.
            self.non_aprs = self.non_aprs.saturating_add(1);
            self.status_dirty = true;
            return;
        };
        // Only UI frames carry APRS. A connected-mode session sharing the
        // channel is somebody else's traffic and there is nothing to show.
        let PacketType::Ui(ui) = p.packet_type() else {
            self.non_aprs = self.non_aprs.saturating_add(1);
            self.status_dirty = true;
            return;
        };
        let from = p.src().call().to_string();
        let dest = p.dst().call().to_string();
        // A digipeater address with its high bit set has already repeated the
        // frame, which is what a monitor prints as the asterisk in `WIDE1-1*`.
        let via: Vec<String> = p
            .digipeaters()
            .iter()
            .map(|a| if a.highbit() { format!("{}*", a.call()) } else { a.call().to_string() })
            .collect();
        // Direct means nothing repeated it — which on a channel where
        // everything is digipeated is how you tell who is actually in range.
        let direct = !p.digipeaters().iter().any(sdroxide_ax25::Addr::highbit);
        let payload = ui.payload.clone();
        self.absorb(&from, &dest, &via, direct, &payload, false);
    }

    /// Decode one information field and fold it into the station list, the
    /// message list and the traffic log.
    ///
    /// `sent` marks our own traffic, which goes through here too: an operator
    /// needs to see their beacon go out, and a beacon that never appears looks
    /// exactly like a beacon that was never sent.
    fn absorb(
        &mut self,
        from: &str,
        dest: &str,
        via: &[String],
        direct: bool,
        info: &[u8],
        sent: bool,
    ) {
        let at = now_unix();
        let parsed = sdroxide_aprs::parse(dest, info);
        let kind = match &parsed {
            Ok(d) => d.kind().to_string(),
            Err(e) => e.to_string(),
        };
        self.note_traffic(AprsTraffic {
            at,
            from: from.to_string(),
            to: dest.to_string(),
            via: via.to_vec(),
            info: String::from_utf8_lossy(info).replace(['\r', '\n'], " ").trim().to_string(),
            kind,
            sent,
        });
        let Ok(data) = parsed else {
            self.non_aprs = self.non_aprs.saturating_add(1);
            self.status_dirty = true;
            return;
        };

        match data {
            AprsData::Position(p) => {
                let st = self.station_mut(from, AprsEntryKind::Station, "");
                st.symbol = p.symbol;
                st.comment = p.comment.clone();
                st.course_deg = p.course_deg;
                st.speed_kn = p.speed_kn;
                st.altitude_m = p.altitude_m;
                if let Some(w) = p.weather {
                    st.weather = Some(w);
                }
                if let Some(m) = p.mice {
                    // The Mic-E message is the only thing a Mic-E frame says
                    // in words, so it belongs where the operator reads words.
                    st.status = m.label().to_string();
                }
                let pos = p.pos;
                Self::place(st, pos);
                Self::touch(st, at, via, direct);
            }
            AprsData::Object { name, live, pos } | AprsData::Item { name, live, pos } => {
                let entry = AprsEntryKind::Object;
                let st = self.station_mut(&name, entry, from);
                st.symbol = pos.symbol;
                st.comment = pos.comment.clone();
                st.course_deg = pos.course_deg;
                st.speed_kn = pos.speed_kn;
                st.altitude_m = pos.altitude_m;
                st.killed = !live;
                let p = pos.pos;
                Self::place(st, p);
                Self::touch(st, at, via, direct);
            }
            AprsData::Status(text) => {
                let st = self.station_mut(from, AprsEntryKind::Station, "");
                st.status = text;
                Self::touch(st, at, via, direct);
            }
            AprsData::Weather(w) => {
                let st = self.station_mut(from, AprsEntryKind::Station, "");
                st.weather = Some(*w);
                Self::touch(st, at, via, direct);
            }
            AprsData::Grid { grid, symbol, comment } => {
                let pos = sdroxide_aprs::position_from_grid(&grid);
                let st = self.station_mut(from, AprsEntryKind::Station, "");
                st.symbol = symbol;
                st.comment = comment;
                if let Some(pos) = pos {
                    Self::place(st, pos);
                }
                Self::touch(st, at, via, direct);
            }
            AprsData::Telemetry(_) | AprsData::Query(_) => {
                let st = self.station_mut(from, AprsEntryKind::Station, "");
                Self::touch(st, at, via, direct);
            }
            AprsData::Message(m) => {
                {
                    let st = self.station_mut(from, AprsEntryKind::Station, "");
                    Self::touch(st, at, via, direct);
                }
                if !sent {
                    self.on_message(from, &m, at);
                }
            }
        }
        self.trim_stations();
        self.status_dirty = true;
    }

    /// A message frame addressed to somebody — possibly us.
    fn on_message(&mut self, from: &str, m: &sdroxide_aprs::Message, at: i64) {
        let me = self.cfg.aprs_call();
        let for_me = !me.is_empty() && m.addressee.eq_ignore_ascii_case(&me);

        // An acknowledgement is only ours if it is addressed to us.
        //
        // Matching on the identifier and the sender alone is not enough: the
        // identifiers are one to five characters, every station on the channel
        // mints its own, and two stations counting from 1 collide constantly.
        // Without this, VK2ABC acknowledging G0XYZ's message 3 would close our
        // message 3 to VK2ABC and stop it being retried.
        if for_me {
            match m.kind {
                MessageKind::Ack => {
                    self.close_outgoing(from, &m.id, AprsMsgState::Acked);
                    return;
                }
                MessageKind::Rej => {
                    self.close_outgoing(from, &m.id, AprsMsgState::Rejected);
                    return;
                }
                _ => {}
            }
            // A reply-ack folds an acknowledgement of ours into their reply,
            // which saves a whole transmission on a channel where that is the
            // scarce thing.
            if !m.reply_ack.is_empty() {
                let id = m.reply_ack.clone();
                self.close_outgoing(from, &id, AprsMsgState::Acked);
            }
        } else if matches!(m.kind, MessageKind::Ack | MessageKind::Rej) {
            // Somebody else's acknowledgement. It is in the raw log and that
            // is all it is.
            return;
        }

        // Bulletins are broadcast to everybody and are never acknowledged.
        let bulletin = m.kind == MessageKind::Bulletin;
        if !for_me && !bulletin {
            // Somebody else's traffic. It is in the raw log and that is where
            // it belongs; a message pane full of other people's conversations
            // is a message pane nobody reads.
            return;
        }

        // De-duplicate against the digipeaters, but acknowledge every copy: a
        // repeat almost always means the sender did not hear the first ack.
        let key = format!("{from}\u{1}{}", m.id);
        let repeat = !m.id.is_empty() && self.seen_msgs.contains_key(&key);
        if !m.id.is_empty() {
            self.seen_msgs.insert(key, at);
        }
        if !repeat {
            self.messages.push(AprsMessage {
                at,
                from: from.to_string(),
                to: m.addressee.clone(),
                text: m.text.clone(),
                id: m.id.clone(),
                state: AprsMsgState::Received,
                tries: 0,
            });
            self.trim_messages();
        }
        // Only a message carrying an identifier asked to be acknowledged, and
        // a bulletin never is however it is addressed.
        if for_me && !bulletin && !m.id.is_empty() && self.cfg.aprs_ack_messages {
            let ack = sdroxide_aprs::encode_ack(from, &m.id);
            self.transmit(ack);
        }
    }

    /// The station's entry, created if this is the first time it has been
    /// heard.
    fn station_mut(
        &mut self,
        name: &str,
        entry: AprsEntryKind,
        reported_by: &str,
    ) -> &mut AprsStation {
        let idx = self.stations.iter().position(|s| s.name == name && s.entry == entry);
        let idx = match idx {
            Some(i) => i,
            None => {
                self.stations.push(AprsStation {
                    name: name.to_string(),
                    reported_by: reported_by.to_string(),
                    entry,
                    symbol: AprsSymbol::default(),
                    pos: None,
                    track: Vec::new(),
                    course_deg: None,
                    speed_kn: None,
                    altitude_m: None,
                    comment: String::new(),
                    status: String::new(),
                    weather: None,
                    last_heard: 0,
                    packets: 0,
                    via: Vec::new(),
                    direct: false,
                    killed: false,
                });
                self.stations.len() - 1
            }
        };
        if !reported_by.is_empty() {
            self.stations[idx].reported_by = reported_by.to_string();
        }
        &mut self.stations[idx]
    }

    /// Move a station, keeping a track behind it if it has actually moved.
    fn place(st: &mut AprsStation, pos: AprsPosition) {
        let moved = st.pos.is_none_or(|old| {
            sdroxide_types::distance_km((old.lat, old.lon), (pos.lat, pos.lon)) * 1000.0
                > TRACK_MIN_M
        });
        if moved && let Some(old) = st.pos {
            st.track.push((old.lat, old.lon));
            if st.track.len() > APRS_TRACK_MAX {
                st.track.remove(0);
            }
        }
        st.pos = Some(pos);
    }

    fn touch(st: &mut AprsStation, at: i64, via: &[String], direct: bool) {
        st.last_heard = at;
        st.packets = st.packets.saturating_add(1);
        st.via = via.to_vec();
        st.direct = direct;
    }

    fn note_traffic(&mut self, t: AprsTraffic) {
        if self.traffic.len() >= APRS_TRAFFIC_MAX {
            self.traffic.remove(0);
        }
        self.traffic.push(t);
    }

    fn trim_messages(&mut self) {
        while self.messages.len() > APRS_MESSAGE_MAX {
            self.messages.remove(0);
        }
    }

    /// Drop the stations nobody has heard from lately, and the oldest of what
    /// is left once the map is full.
    fn trim_stations(&mut self) {
        let ttl = i64::from(self.cfg.aprs_station_ttl_min.max(1)) * 60;
        let cutoff = now_unix() - ttl;
        self.stations.retain(|s| s.last_heard >= cutoff);
        while self.stations.len() > APRS_STATION_MAX {
            // The least recently heard, not the first added: a digipeater
            // heard every minute for an hour should outlive a car that passed
            // through once.
            if let Some((i, _)) = self.stations.iter().enumerate().min_by_key(|(_, s)| s.last_heard)
            {
                self.stations.remove(i);
            } else {
                break;
            }
        }
        // Message de-duplication cannot grow for ever either. The same window
        // the map uses: past it, a repeat is a new message.
        self.seen_msgs.retain(|_, at| *at >= cutoff);
    }

    /// Retry the messages that have gone unanswered.
    fn poll_outbox(&mut self, now: SystemTime) {
        let mut send = Vec::new();
        let mut give_up = Vec::new();
        for o in &mut self.outbox {
            if now < o.next_try {
                continue;
            }
            if o.tries >= APRS_MSG_RETRIES {
                give_up.push((o.id.clone(), o.to.clone()));
                continue;
            }
            o.tries += 1;
            // Doubling, from thirty seconds: a channel this crowded is worse
            // off for a station that hammers a message at a fixed interval,
            // and an addressee who has not answered in a minute is usually
            // out of range rather than slow.
            o.next_try = now + Duration::from_secs(MSG_RETRY_S << (o.tries.min(5) - 1));
            send.push((o.to.clone(), o.text.clone(), o.id.clone(), o.tries));
        }
        self.outbox.retain(|o| !give_up.iter().any(|(id, to)| *id == o.id && *to == o.to));
        for (id, to) in give_up {
            self.set_msg_state(&id, &to, AprsMsgState::Failed);
        }
        for (to, text, id, tries) in send {
            let info = sdroxide_aprs::encode_message(&to, &text, &id);
            self.transmit(info);
            for m in self.messages.iter_mut().rev() {
                if m.id == id && m.to == to {
                    m.state = AprsMsgState::Sent;
                    m.tries = tries;
                    break;
                }
            }
        }
    }

    fn aprs_status(&self) -> AprsStatus {
        let next_beacon_s = self
            .next_beacon
            .and_then(|at| at.duration_since(SystemTime::now()).ok().map(|d| d.as_secs() as u32));
        AprsStatus {
            dcd: self.ch.dcd,
            level: self.ch.level(),
            stations: self.stations.clone(),
            messages: self.messages.clone(),
            traffic: self.traffic.clone(),
            bad_frames: self.bad_frames,
            non_aprs: self.non_aprs,
            my_pos: self.my_position(),
            next_beacon_s,
            tx_queue: self.ch.queued() as u32,
        }
    }

    fn digi_status(&self) -> DigiStatus {
        DigiStatus {
            mode: Mode::Aprs,
            step: QsoStep::Idle,
            dx_call: None,
            dx_grid: None,
            tx_next: false,
            tx_pending_msg: None,
            // Centred on the carrier: FM, so there is no audio offset.
            audio_hz: 0.0,
            tx_even: false,
            transmitting: self.ch.keyed,
            tx_watchdog: false,
            transcript: Vec::<TranscriptLine>::new(),
            config: self.cfg.clone(),
            text_rx: String::new(),
            tx_sent: 0,
            fsq_heard: Vec::new(),
            fsq_messages: Vec::new(),
            rade: None,
            packet: None,
            navtex: None,
            aprs: Some(Box::new(self.aprs_status())),
            js8: None,
            atchat: None,
            fox_queue: Vec::new(),
            call_queue: Vec::new(),
            clock_offset_s: None,
            cw: None,
            wspr: None,
            qso: None,
        }
    }
}

fn now_unix() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

impl DigiEngine for AprsController {
    fn mode(&self) -> Mode {
        Mode::Aprs
    }

    fn on_rx_audio(&mut self, tap: &[f32]) {
        let r = self.ch.on_rx_audio(tap, &self.cfg);
        for f in r.frames {
            self.on_frame(&f);
        }
        if r.bad > 0 {
            self.bad_frames = self.bad_frames.saturating_add(r.bad);
            self.status_dirty = true;
        }
        if r.dcd_changed {
            self.status_dirty = true;
        }
    }

    fn poll(&mut self, now_in: SystemTime, _dial_hz: f64) -> Vec<DigiAction> {
        let mut actions = std::mem::take(&mut self.queued);

        // The beacon, scheduled here rather than on the audio clock because
        // minutes are exactly what `poll`'s cadence is good for.
        let every = self.cfg.aprs_beacon_minutes;
        if every == 0 || self.my_position().is_none() || self.mycall().is_none() {
            self.next_beacon = None;
        } else {
            match self.next_beacon {
                // Wait one interval before the first. Keying the moment the
                // operator selects the mode is startling, and the settings
                // they are about to type are not in yet.
                None => {
                    self.next_beacon = Some(now_in + Duration::from_secs(u64::from(every) * 60));
                    self.status_dirty = true;
                }
                Some(at) if now_in >= at => {
                    self.queue_beacon();
                    self.next_beacon = Some(now_in + Duration::from_secs(u64::from(every) * 60));
                }
                Some(_) => {}
            }
        }

        self.poll_outbox(now_in);

        // CSMA has cleared us: key up through the engine's normal PTT path so
        // the station interlock and the band rails apply.
        if let Some(sent) = self.ch.take_over(&self.cfg) {
            let me = self.cfg.aprs_call();
            let via = sdroxide_aprs::parse_path(&self.cfg.aprs_path);
            for f in sent {
                if let Ok(p) = Packet::parse(&f, None)
                    && let PacketType::Ui(ui) = p.packet_type()
                {
                    let payload = ui.payload.clone();
                    self.absorb(&me, TOCALL, &via, true, &payload, true);
                }
            }
            self.status_dirty = true;
            actions.push(DigiAction::KeyTx);
        }

        let now = SystemTime::now();
        let due = self
            .last_status
            .map(|t| now.duration_since(t).map(|d| d.as_secs_f32() > 0.2).unwrap_or(true))
            .unwrap_or(true);
        if self.status_dirty || due {
            self.status_dirty = false;
            self.last_status = Some(now);
            actions.push(DigiAction::Status(self.digi_status()));
        }
        actions
    }

    fn tx_burst_active(&self) -> bool {
        self.ch.keyed
    }

    fn tx_peak(&self) -> f32 {
        self.ch.tx_peak()
    }

    fn fill_tx_block(&mut self, out: &mut [f32]) -> bool {
        self.ch.fill_tx_block(out)
    }

    /// As for packet, whose modem this shares: built at the tap rate, so the
    /// audio comes back at the tap rate.
    fn tx_rate(&self) -> f64 {
        self.tap_rate
    }

    fn on_burst_done(&mut self) {
        self.ch.on_burst_done();
        self.status_dirty = true;
    }

    fn abort(&mut self) {
        self.ch.abort();
        // The operator stopping transmit stops the retries too: a message
        // still being retried is a transmitter that keys itself, and "stop"
        // has to mean it.
        for o in &self.outbox {
            let (id, to) = (o.id.clone(), o.to.clone());
            for m in self.messages.iter_mut().rev() {
                if m.id == id && m.to == to {
                    m.state = AprsMsgState::Failed;
                    break;
                }
            }
        }
        self.outbox.clear();
        self.status_dirty = true;
    }

    fn abort_tx(&mut self) {
        self.ch.abort_tx();
        self.status_dirty = true;
    }

    fn set_config(&mut self, cfg: DigiConfig) {
        let slot_changed = cfg.packet_slottime_ms != self.cfg.packet_slottime_ms;
        self.cfg = cfg;
        if slot_changed {
            self.ch.rebuild(PacketBaud::Vhf1200, &self.cfg, self.tap_rate);
        }
        // A changed interval takes effect from now, not from whenever the old
        // one would have fired.
        self.next_beacon = None;
        self.status_dirty = true;
    }

    /// On FM the signal is centred on the dial, so there is no offset to set.
    fn set_audio_hz(&mut self, _hz: f32) {}

    fn audio_hz(&self) -> f32 {
        0.0
    }

    fn aprs_beacon_now(&mut self) {
        self.queue_beacon();
    }

    fn aprs_send_message(&mut self, to: String, text: String) {
        self.queue_message(to, text);
    }

    /// Empty the map, the messages and the log. The outbox goes with them: a
    /// message being retried against a cleared list would keep transmitting
    /// with nothing on screen to say so.
    fn clear_rx(&mut self) {
        self.stations.clear();
        self.messages.clear();
        self.traffic.clear();
        self.seen_msgs.clear();
        self.outbox.clear();
        self.bad_frames = 0;
        self.non_aprs = 0;
        self.status_dirty = true;
    }

    fn status(&self) -> DigiStatus {
        self.digi_status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn station(call: &str) -> DigiConfig {
        DigiConfig {
            aprs_mycall: call.into(),
            my_grid: "JN88ec".into(),
            // Take the channel as soon as one slot has passed: the tests are
            // about the protocol, and leaving the dice in would make them
            // flaky for a reason that has nothing to do with what they check.
            packet_persist: 255,
            packet_slottime_ms: 1,
            ..Default::default()
        }
    }

    /// A frame off the air becomes a station on the map.
    #[test]
    fn a_position_report_puts_a_station_on_the_map() {
        let mut c = AprsController::new(DigiConfig::default(), 48_000.0);
        c.absorb("OE3JJS-9", "APZSDR", &[], true, b"!4903.50N/07201.75W>mobile", false);
        assert_eq!(c.stations.len(), 1);
        let s = &c.stations[0];
        assert_eq!(s.name, "OE3JJS-9");
        assert_eq!(s.symbol.kind(), sdroxide_types::AprsSymbolKind::Car);
        assert!(s.pos.is_some());
        assert!(s.direct, "no digipeater in the path means direct");
    }

    /// The same frame arriving again through a digipeater must not become a
    /// second station.
    #[test]
    fn a_digipeated_repeat_updates_rather_than_duplicates() {
        let mut c = AprsController::new(DigiConfig::default(), 48_000.0);
        for _ in 0..3 {
            c.absorb(
                "OE3JJS-9",
                "APZSDR",
                &["WIDE2-1*".into()],
                false,
                b"!4903.50N/07201.75W>",
                false,
            );
        }
        assert_eq!(c.stations.len(), 1);
        assert_eq!(c.stations[0].packets, 3);
        assert!(!c.stations[0].direct);
    }

    /// A station that moves leaves a trail; one that jitters on the spot does
    /// not.
    #[test]
    fn a_moving_station_leaves_a_track() {
        let mut c = AprsController::new(DigiConfig::default(), 48_000.0);
        c.absorb("VK2ABC-9", "APZSDR", &[], true, b"!4903.50N/07201.75W>", false);
        // A hundredth of a minute is about 18 m — inside the jitter floor.
        c.absorb("VK2ABC-9", "APZSDR", &[], true, b"!4903.51N/07201.75W>", false);
        assert!(c.stations[0].track.is_empty(), "GPS jitter must not draw a track");
        // A tenth of a degree is six nautical miles.
        c.absorb("VK2ABC-9", "APZSDR", &[], true, b"!4909.50N/07201.75W>", false);
        assert_eq!(c.stations[0].track.len(), 1);
    }

    /// A message addressed to us appears and is acknowledged.
    #[test]
    fn a_message_to_us_is_shown_and_acknowledged() {
        let mut c = AprsController::new(station("OE3JJS"), 48_000.0);
        c.absorb("VK2ABC", "APZSDR", &[], true, b":OE3JJS   :are you there?{7", false);
        assert_eq!(c.messages.len(), 1);
        assert_eq!(c.messages[0].text, "are you there?");
        assert_eq!(c.messages[0].state, AprsMsgState::Received);
        assert_eq!(c.ch.queued(), 1, "no acknowledgement was queued");
    }

    /// A repeat of that message must be acknowledged again — the sender is
    /// repeating it because they did not hear the first ack — but must not
    /// appear twice in the pane.
    #[test]
    fn a_repeated_message_is_re_acknowledged_but_not_re_delivered() {
        let mut c = AprsController::new(station("OE3JJS"), 48_000.0);
        for _ in 0..3 {
            c.absorb("VK2ABC", "APZSDR", &[], true, b":OE3JJS   :hello{7", false);
        }
        assert_eq!(c.messages.len(), 1, "the same message appeared more than once");
        assert_eq!(c.ch.queued(), 3, "a repeat must be acknowledged again");
    }

    /// Somebody else's conversation stays out of the message pane. It is in
    /// the raw log, which is where other people's traffic belongs.
    #[test]
    fn another_stations_message_is_not_delivered_to_us() {
        let mut c = AprsController::new(station("OE3JJS"), 48_000.0);
        c.absorb("VK2ABC", "APZSDR", &[], true, b":G0XYZ    :hello{7", false);
        assert!(c.messages.is_empty());
        assert_eq!(c.ch.queued(), 0, "acknowledged a message that was not ours");
        assert_eq!(c.traffic.len(), 1, "it still belongs in the raw log");
    }

    /// A bulletin is broadcast to everybody, so it is shown — and never
    /// acknowledged, because a hundred stations acknowledging one bulletin
    /// would take the channel down.
    #[test]
    fn a_bulletin_is_shown_and_never_acknowledged() {
        let mut c = AprsController::new(station("OE3JJS"), 48_000.0);
        c.absorb("VK2ABC", "APZSDR", &[], true, b":BLN1     :club net tuesday{3", false);
        assert_eq!(c.messages.len(), 1);
        assert_eq!(c.ch.queued(), 0, "acknowledged a bulletin");
    }

    /// Switching acknowledgements off has to actually stop the transmitter —
    /// a receive-only setup must be able to be silent.
    #[test]
    fn acknowledgements_can_be_switched_off() {
        let cfg = DigiConfig { aprs_ack_messages: false, ..station("OE3JJS") };
        let mut c = AprsController::new(cfg, 48_000.0);
        c.absorb("VK2ABC", "APZSDR", &[], true, b":OE3JJS   :hello{7", false);
        assert_eq!(c.messages.len(), 1, "it should still be shown");
        assert_eq!(c.ch.queued(), 0, "transmitted with acknowledgements off");
    }

    /// The APRS callsign field is a *refinement* of the station callsign, not
    /// a second place to type it.
    ///
    /// It was neither for one release, and the symptom was the worst kind:
    /// an operator with their callsign filled in on the General tab pressed
    /// BEACON and nothing happened at all — no frame, no error, no line in the
    /// log. Every transmit path here gives up quietly when there is no
    /// callsign, so "no callsign" must not be something an operator can be in
    /// without meaning to be.
    #[test]
    fn an_empty_aprs_call_falls_back_to_the_station_callsign() {
        let cfg = DigiConfig {
            my_call: "oe3jjs".into(),
            aprs_mycall: String::new(),
            my_grid: "JN88ec".into(),
            packet_persist: 255,
            packet_slottime_ms: 1,
            ..Default::default()
        };
        let mut c = AprsController::new(cfg.clone(), 48_000.0);
        c.queue_beacon();
        assert_eq!(c.ch.queued(), 1, "a beacon must go out on the station callsign alone");
        for _ in 0..4 {
            c.ch.on_rx_audio(&[0.0; 4800], &cfg);
        }
        let sent = c.ch.take_over(&cfg).expect("the beacon never left the queue");
        let p = Packet::parse(&sent[0], None).unwrap();
        assert_eq!(p.src().call(), "OE3JJS", "and under that call, upper-cased");

        // ...and the APRS-specific one still wins where it is set, which is
        // what the SSID convention needs.
        let cfg = DigiConfig { aprs_mycall: "OE3JJS-9".into(), ..cfg };
        let mut c = AprsController::new(cfg.clone(), 48_000.0);
        c.queue_beacon();
        for _ in 0..4 {
            c.ch.on_rx_audio(&[0.0; 4800], &cfg);
        }
        let sent = c.ch.take_over(&cfg).unwrap();
        assert_eq!(Packet::parse(&sent[0], None).unwrap().src().call(), "OE3JJS-9");
    }

    /// A message goes out on the station callsign too, and is filed under it.
    #[test]
    fn a_message_uses_the_station_callsign_when_no_aprs_call_is_set() {
        let cfg = DigiConfig {
            my_call: "OE3JJS".into(),
            packet_persist: 255,
            packet_slottime_ms: 1,
            ..Default::default()
        };
        let mut c = AprsController::new(cfg, 48_000.0);
        c.queue_message("VK2ABC".into(), "hello".into());
        assert_eq!(c.messages.len(), 1, "the message was refused");
        assert_eq!(c.messages[0].from, "OE3JJS");
        c.poll_outbox(SystemTime::now());
        assert_eq!(c.ch.queued(), 1);
        // ...and an acknowledgement addressed to that call is recognised.
        let id = c.messages[0].id.clone();
        c.absorb("VK2ABC", "APZSDR", &[], true, format!(":OE3JJS   :ack{id}").as_bytes(), false);
        assert_eq!(c.messages[0].state, AprsMsgState::Acked);
    }

    /// A station with no callsign must never transmit — not a beacon, not a
    /// message, not an acknowledgement. Empty is the state the config ships
    /// in, so this is the default path rather than an edge case.
    #[test]
    fn a_station_with_no_callsign_never_transmits() {
        // Neither field set — which, since the fallback, is the only way to be
        // without a callsign.
        let cfg = DigiConfig {
            my_grid: "JN88ec".into(),
            my_call: String::new(),
            aprs_mycall: String::new(),
            ..Default::default()
        };
        let mut c = AprsController::new(cfg, 48_000.0);
        c.queue_beacon();
        c.queue_message("VK2ABC".into(), "hello".into());
        c.absorb("VK2ABC", "APZSDR", &[], true, b":N0CALL   :hello{7", false);
        assert_eq!(c.ch.queued(), 0, "transmitted without a callsign");
        assert!(c.messages.is_empty(), "a message was queued with no callsign to send it from");
    }

    /// The beacon reads back as our own position, and puts us on our own map.
    #[test]
    fn a_beacon_is_a_position_report_from_our_own_call() {
        let mut c = AprsController::new(station("OE3JJS-10"), 48_000.0);
        c.queue_beacon();
        assert_eq!(c.ch.queued(), 1);
        // Round-trip it the way a receiving station would.
        let cfg = c.cfg.clone();
        for _ in 0..4 {
            c.ch.on_rx_audio(&[0.0; 4800], &cfg);
        }
        let sent = c.ch.take_over(&cfg).expect("the beacon never left the queue");
        let p = Packet::parse(&sent[0], None).unwrap();
        assert_eq!(p.src().call(), "OE3JJS-10");
        assert_eq!(p.dst().call(), TOCALL);
        assert_eq!(
            p.digipeaters().iter().map(|a| a.call().to_string()).collect::<Vec<_>>(),
            vec!["WIDE1-1", "WIDE2-1"],
            "the beacon must carry the configured path"
        );
        let PacketType::Ui(ui) = p.packet_type() else { panic!("a beacon must be a UI frame") };
        let data =
            sdroxide_aprs::parse(TOCALL, &ui.payload).expect("our own beacon does not parse");
        let pos = data.position().expect("no position in the beacon");
        // JN88ec is Vienna.
        assert!((pos.pos.lat - 48.2).abs() < 0.2, "{}", pos.pos.lat);
        assert!((pos.pos.lon - 16.4).abs() < 0.4, "{}", pos.pos.lon);
    }

    /// An acknowledgement stops the retries and marks the message.
    #[test]
    fn an_acknowledgement_closes_a_message() {
        let mut c = AprsController::new(station("OE3JJS"), 48_000.0);
        c.queue_message("VK2ABC".into(), "on my way".into());
        c.poll_outbox(SystemTime::now());
        assert_eq!(c.outbox.len(), 1);
        assert_eq!(c.messages[0].state, AprsMsgState::Sent);
        let id = c.messages[0].id.clone();
        c.absorb("VK2ABC", "APZSDR", &[], true, format!(":OE3JJS   :ack{id}").as_bytes(), false);
        assert!(c.outbox.is_empty(), "kept retrying an acknowledged message");
        assert_eq!(c.messages[0].state, AprsMsgState::Acked);
    }

    /// ...and an unanswered one gives up rather than transmitting for ever.
    #[test]
    fn an_unanswered_message_gives_up() {
        let mut c = AprsController::new(station("OE3JJS"), 48_000.0);
        c.queue_message("VK2ABC".into(), "hello?".into());
        let mut t = SystemTime::now();
        for _ in 0..APRS_MSG_RETRIES + 1 {
            c.poll_outbox(t);
            // Past the longest back-off, so every attempt is due.
            t += Duration::from_secs(MSG_RETRY_S * 64);
        }
        assert!(c.outbox.is_empty(), "an unanswered message must not be retried for ever");
        assert_eq!(c.messages[0].state, AprsMsgState::Failed);
        assert_eq!(c.messages[0].tries, APRS_MSG_RETRIES);
    }

    /// Two stations both counting their messages from 1 is the normal state
    /// of an APRS channel, so an acknowledgement meant for somebody else must
    /// not close ours.
    #[test]
    fn an_acknowledgement_addressed_to_somebody_else_is_ignored() {
        let mut c = AprsController::new(station("OE3JJS"), 48_000.0);
        c.queue_message("VK2ABC".into(), "hello".into());
        c.poll_outbox(SystemTime::now());
        let id = c.messages[0].id.clone();
        // VK2ABC acknowledging G0XYZ's message of the same number.
        c.absorb("VK2ABC", "APZSDR", &[], true, format!(":G0XYZ    :ack{id}").as_bytes(), false);
        assert_eq!(c.outbox.len(), 1, "closed a message on somebody else's acknowledgement");
        assert_eq!(c.messages[0].state, AprsMsgState::Sent);
    }

    /// A rejection stops the retries too, which silence would not.
    #[test]
    fn a_rejection_stops_the_retries() {
        let mut c = AprsController::new(station("OE3JJS"), 48_000.0);
        c.queue_message("VK2ABC".into(), "hello".into());
        c.poll_outbox(SystemTime::now());
        let id = c.messages[0].id.clone();
        c.absorb("VK2ABC", "APZSDR", &[], true, format!(":OE3JJS   :rej{id}").as_bytes(), false);
        assert!(c.outbox.is_empty());
        assert_eq!(c.messages[0].state, AprsMsgState::Rejected);
    }

    /// A Mic-E frame from a commercial radio has to land on the map. This is
    /// the format that needs the destination address, and it reaches the codec
    /// through here.
    #[test]
    fn a_mic_e_frame_from_a_radio_lands_on_the_map() {
        let mut c = AprsController::new(DigiConfig::default(), 48_000.0);
        c.absorb("SS2UVT", "SS2UVT", &[], true, b"`(_Hn\"Oj/", false);
        let s = &c.stations[0];
        let pos = s.pos.expect("no position from a Mic-E frame");
        assert!((pos.lat - 33.427).abs() < 0.01, "{}", pos.lat);
        assert!((pos.lon + 112.124).abs() < 0.01, "{}", pos.lon);
        assert_eq!(s.status, "En Route");
    }

    /// A frame that is not APRS at all is counted, not shown as a station.
    #[test]
    fn a_non_aprs_frame_is_counted_rather_than_mapped() {
        let mut c = AprsController::new(DigiConfig::default(), 48_000.0);
        c.absorb("OE3JJS", "APZSDR", &[], true, b"%not a data type", false);
        assert_eq!(c.non_aprs, 1);
        assert!(c.stations.is_empty());
        assert_eq!(c.traffic.len(), 1, "it still belongs in the raw log");
    }

    /// An object is somebody else's report about a third thing, and has to be
    /// filed under its own name rather than the reporter's.
    #[test]
    fn an_object_is_filed_under_its_own_name() {
        let mut c = AprsController::new(DigiConfig::default(), 48_000.0);
        c.absorb(
            "OE3JJS",
            "APZSDR",
            &[],
            true,
            b";NET      *211245z4903.50N/07201.75Wr80m net",
            false,
        );
        let s = c.stations.iter().find(|s| s.name == "NET").expect("no object");
        assert_eq!(s.entry, AprsEntryKind::Object);
        assert_eq!(s.reported_by, "OE3JJS");
        assert!(!s.killed);
        // ...and killing it greys it out rather than making it vanish.
        c.absorb("OE3JJS", "APZSDR", &[], true, b";NET      _211300z4903.50N/07201.75Wr", false);
        assert!(c.stations.iter().find(|s| s.name == "NET").unwrap().killed);
    }

    /// The whole receive chain, end to end: an APRS frame is built, framed,
    /// modulated into Bell 202 audio, and fed back in as if it had come off
    /// the air.
    ///
    /// Every test above this one hands `absorb` an information field
    /// directly, which is the right way to test the protocol and says nothing
    /// about whether a real signal ever reaches it. This is the one that does:
    /// bit stuffing, the check sequence, the tone pair, the clock recovery and
    /// the deframer are all in the path, and a break in any of them shows up
    /// here as a station that never appears.
    #[test]
    fn a_modulated_frame_reaches_the_map() {
        use sdroxide_ax25::Framer;
        use sdroxide_dsp::{AfskProfile, AfskTx};

        let rate = 48_000.0;
        let mut c = AprsController::new(DigiConfig::default(), rate);

        let me = Addr::new("VK2ABC-9").unwrap();
        let dest = Addr::new(TOCALL).unwrap();
        let via = vec![Addr::new("WIDE1-1").unwrap()];
        let info = sdroxide_aprs::encode_position(
            AprsPosition { lat: -33.8688, lon: 151.2093, ambiguity: 0 },
            AprsSymbol::new('/', '>'),
            "on the road",
            true,
        );
        let frame = Packet::ui_via(me, dest, via, info.into_bytes()).serialize(false);

        let mut framer = Framer::new();
        // A real preamble, so the demodulator has something to lock to before
        // the frame starts — exactly what TXDELAY buys on the air.
        framer.push_flags(40);
        framer.push_frame(&frame);
        framer.push_flags(8);
        let bits = framer.take();

        let mut tx = AfskTx::new(rate, AfskProfile::Vhf1200);
        tx.push_bits(&bits);
        let mut audio = Vec::new();
        let mut block = [0.0f32; 1024];
        loop {
            let n = tx.next_block(&mut block);
            if n == 0 {
                break;
            }
            audio.extend_from_slice(&block[..n]);
            if tx.idle() && n < block.len() {
                break;
            }
        }
        assert!(audio.len() > 4000, "the modem produced almost nothing: {} samples", audio.len());

        for chunk in audio.chunks(480) {
            c.on_rx_audio(chunk);
        }

        let s = c.stations.iter().find(|s| s.name == "VK2ABC-9").unwrap_or_else(|| {
            panic!("nothing decoded: {} bad frames, {} non-APRS", c.bad_frames, c.non_aprs)
        });
        let pos = s.pos.expect("decoded but with no position");
        assert!((pos.lat + 33.8688).abs() < 1e-3, "{}", pos.lat);
        assert!((pos.lon - 151.2093).abs() < 1e-3, "{}", pos.lon);
        assert_eq!(s.comment, "on the road");
        assert_eq!(s.symbol.kind(), sdroxide_types::AprsSymbolKind::Car);
        assert_eq!(s.via, vec!["WIDE1-1"], "the path has to survive the round trip");
        assert!(s.direct, "nothing set the repeated bit, so this arrived direct");
        assert_eq!(c.bad_frames, 0);
    }

    /// The bytes on the air, checked against what every other APRS station
    /// puts there.
    ///
    /// Nothing in this codebase reads the command/response bits of a UI frame,
    /// so a wrong pairing decodes perfectly here and forever — which is
    /// exactly why it is worth pinning. A frame that does not look like the
    /// rest of the channel's is one some digipeater firmware is entitled to
    /// drop, and the operator would only ever see it as silence from the
    /// network.
    #[test]
    fn a_beacon_looks_like_every_other_aprs_frame_on_the_wire() {
        let cfg = DigiConfig {
            my_call: "OE3JJS-9".into(),
            my_grid: "JN88ec".into(),
            packet_persist: 255,
            packet_slottime_ms: 1,
            ..Default::default()
        };
        let mut c = AprsController::new(cfg.clone(), 48_000.0);
        c.queue_beacon();
        for _ in 0..4 {
            c.ch.on_rx_audio(&[0.0; 4800], &cfg);
        }
        let sent = c.ch.take_over(&cfg).expect("no beacon");
        let f = &sent[0];

        // Destination, then source, then the path — 7 bytes each, the last
        // address marked by bit 0 of its SSID byte.
        let ssid_byte = |n: usize| f[n * 7 + 6];
        // A *command*: destination C bit set, source C bit clear. The other
        // pairing is a response, which a broadcast is not.
        assert_eq!(ssid_byte(0) & 0x80, 0x80, "destination C bit: this is a command");
        assert_eq!(ssid_byte(1) & 0x80, 0x00, "source C bit: this is a command");
        // The two reserved bits are unused and stay set on every address.
        for n in 0..4 {
            assert_eq!(ssid_byte(n) & 0x60, 0x60, "address {n} has a reserved bit clear");
        }
        // Only the last address ends the field.
        assert_eq!(ssid_byte(0) & 1, 0);
        assert_eq!(ssid_byte(1) & 1, 0);
        assert_eq!(ssid_byte(2) & 1, 0, "WIDE1-1 is not the last address");
        assert_eq!(ssid_byte(3) & 1, 1, "WIDE2-1 must end the address field");
        // Nothing has repeated it yet, so no H bit is set in the path.
        assert_eq!(ssid_byte(2) & 0x80, 0, "WIDE1-1 is marked as already repeated");
        assert_eq!(ssid_byte(3) & 0x80, 0, "WIDE2-1 is marked as already repeated");
        // UI, no layer 3.
        assert_eq!(f[28], 0x03, "control field must be UI");
        assert_eq!(f[29], 0xf0, "PID must be `no layer 3`");
    }

    /// A station nobody has heard from inside the window drops off the map.
    #[test]
    fn a_stale_station_is_dropped() {
        let cfg = DigiConfig { aprs_station_ttl_min: 30, ..Default::default() };
        let mut c = AprsController::new(cfg, 48_000.0);
        c.absorb("OLD", "APZSDR", &[], true, b"!4903.50N/07201.75W>", false);
        c.stations[0].last_heard = now_unix() - 60 * 60;
        c.absorb("NEW", "APZSDR", &[], true, b"!4903.50N/07201.75W>", false);
        assert_eq!(c.stations.len(), 1);
        assert_eq!(c.stations[0].name, "NEW");
    }
}
