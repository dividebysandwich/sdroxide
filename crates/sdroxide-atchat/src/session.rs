//! `AtChatSession` — the sync facade the DigiEngine controller wraps.
//!
//! The [`Station`](crate::protocol::Station) is tokio-async and the DigiEngine
//! hooks are sync (audio thread), so the session runs a one-thread tokio
//! runtime and the two sides meet over shared rings + a snapshot mutex + a
//! command channel.
//!
//! On the real radio the Station's channel is [`RadioConnector`]: transmit
//! audio goes to a queue [`AtChatSession::drain_tx_pcm`] empties, receive audio
//! arrives via [`AtChatSession::push_rx_pcm`] and is energy-segmented into
//! bursts, and "the channel is busy" is a real carrier-sense on the tap. In
//! *virtual* mode the Station instead speaks to `atchat-channeld` over TCP and
//! the two PCM hooks are inert.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::broadcast;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::channel::{RadioBridge, RadioConnector, TcpConnector};
use crate::netproto::Mode;
use crate::protocol::{
    ChatScope, Role, RosterStatus, Station, StationConfig, StationEvent, TransferDir,
};

const LOG_CAP: usize = 300;
const CHAT_CAP: usize = 400;
const FILE_CAP: usize = 64;

fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn looks_like_image(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    [".png", ".jpg", ".jpeg", ".gif", ".bmp", ".webp"].iter().any(|e| n.ends_with(e))
}

// ------------------------------------------------------------------ //
// Public snapshot
// ------------------------------------------------------------------ //

#[derive(Clone, Debug)]
pub struct ChatLine {
    pub from: String,
    pub dst: String,
    pub text: String,
    pub own: bool,
    pub private: bool,
    pub when: u64,
}

#[derive(Clone, Debug)]
pub struct TransferLine {
    pub id: String,
    pub filename: String,
    pub peer: String,
    pub incoming: bool,
    pub have: usize,
    pub total: usize,
    pub complete: bool,
}

#[derive(Clone, Debug)]
pub struct RecvFile {
    pub from: String,
    pub filename: String,
    pub path: String,
    pub is_image: bool,
    pub when: u64,
}

#[derive(Clone, Debug, Default)]
pub struct AtChatSnapshot {
    pub my_call: String,
    pub connected: bool,
    pub role: Option<&'static str>,
    pub master: Option<String>,
    pub roster: Vec<(String, &'static str, f64)>,
    pub chat: Vec<ChatLine>,
    pub transfers: Vec<TransferLine>,
    pub files: Vec<RecvFile>,
    pub log: Vec<String>,
    pub keyed: bool,
    pub carrier: bool,
    pub virtual_addr: Option<String>,
}

// ------------------------------------------------------------------ //
// Session
// ------------------------------------------------------------------ //

enum Cmd {
    Chat {
        dst: String,
        text: String,
    },
    SendFile {
        path: PathBuf,
        dst: String,
    },
    Drop,
    Reconnect,
    SetCallsign(String),
    SetVirtual(Option<String>),
    ClearChat,
    /// A line from the engine-side controller (transmit keying, burst end) for
    /// the station log — the controller runs on the audio thread and has no
    /// other way into the snapshot.
    Note(String),
}

pub struct AtChatSession {
    cmd_tx: UnboundedSender<Cmd>,
    snap: Arc<Mutex<AtChatSnapshot>>,
    bridge: RadioBridge,
    _thread: std::thread::JoinHandle<()>,
}

impl AtChatSession {
    /// Start with a callsign and, optionally, a virtual-channel address
    /// (`None` = the real radio).
    pub fn new(callsign: &str, virtual_addr: Option<String>) -> Self {
        let (cmd_tx, cmd_rx) = unbounded_channel();
        let snap = Arc::new(Mutex::new(AtChatSnapshot {
            my_call: callsign.to_uppercase(),
            virtual_addr: virtual_addr.clone(),
            ..Default::default()
        }));
        let bridge = RadioBridge::new();
        let (snap2, bridge2, call) = (Arc::clone(&snap), bridge.clone(), callsign.to_uppercase());

        let thread = std::thread::Builder::new()
            .name("atchat-session".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("tokio runtime");
                rt.block_on(session_main(cmd_rx, snap2, bridge2, call, virtual_addr));
            })
            .expect("session thread");

        Self { cmd_tx, snap, bridge, _thread: thread }
    }

    // --- audio-thread facing (real radio only) ---

    /// Feed demodulated receive audio at 8 kHz mono.
    pub fn push_rx_pcm(&self, pcm_8k: &[i16]) {
        self.bridge.push_rx(pcm_8k);
    }

    /// Drain up to `max` modulated transmit samples (8 kHz mono) into `out`.
    /// Returns how many were written.
    pub fn drain_tx_pcm(&self, out: &mut Vec<i16>, max: usize) -> usize {
        self.bridge.drain_tx(out, max)
    }

    pub fn tx_pending(&self) -> bool {
        self.bridge.tx_pending()
    }

    // --- control ---

    pub fn snapshot(&self) -> AtChatSnapshot {
        self.snap.lock().unwrap().clone()
    }

    pub fn send_chat(&self, dst: &str, text: &str) {
        let _ = self.cmd_tx.send(Cmd::Chat { dst: dst.to_string(), text: text.to_string() });
    }
    pub fn send_file(&self, path: PathBuf, dst: &str) {
        let _ = self.cmd_tx.send(Cmd::SendFile { path, dst: dst.to_string() });
    }
    pub fn drop_link(&self) {
        let _ = self.cmd_tx.send(Cmd::Drop);
    }
    pub fn reconnect(&self) {
        let _ = self.cmd_tx.send(Cmd::Reconnect);
    }
    pub fn set_callsign(&self, call: &str) {
        let _ = self.cmd_tx.send(Cmd::SetCallsign(call.to_uppercase()));
    }
    pub fn set_virtual(&self, addr: Option<String>) {
        let _ = self.cmd_tx.send(Cmd::SetVirtual(addr));
    }
    pub fn clear_chat(&self) {
        let _ = self.cmd_tx.send(Cmd::ClearChat);
    }
    /// Put a diagnostic line into the station log from the engine side.
    pub fn note(&self, msg: impl Into<String>) {
        let _ = self.cmd_tx.send(Cmd::Note(msg.into()));
    }
}

// ------------------------------------------------------------------ //
// Engine thread
// ------------------------------------------------------------------ //

fn role_str(r: Role) -> &'static str {
    match r {
        Role::Master => "MASTER",
        Role::Backup => "BACKUP",
        Role::Listener => "LISTENER",
    }
}
fn roster_str(s: RosterStatus) -> &'static str {
    match s {
        RosterStatus::Active => "active",
        RosterStatus::Lost => "lost",
    }
}

/// One station, over one of the two channel backends. The variants keep the
/// engine free of a `Box<dyn>` the generic `Station` cannot form.
enum Backed {
    Radio(Station<RadioConnector>),
    Tcp(Station<TcpConnector>),
}

impl Backed {
    fn subscribe(&self) -> broadcast::Receiver<StationEvent> {
        match self {
            Backed::Radio(s) => s.subscribe(),
            Backed::Tcp(s) => s.subscribe(),
        }
    }
    fn chat_bg(&self, text: &str, dst: &str) {
        match self {
            Backed::Radio(s) => s.chat_bg(text, dst),
            Backed::Tcp(s) => s.chat_bg(text, dst),
        }
    }
    fn send_file(&self, p: PathBuf, dst: &str) {
        match self {
            Backed::Radio(s) => s.send_file(p, dst),
            Backed::Tcp(s) => s.send_file(p, dst),
        }
    }
    fn drop_bg(&self) {
        match self {
            Backed::Radio(s) => s.drop_bg(),
            Backed::Tcp(s) => s.drop_bg(),
        }
    }
    fn reconnect_bg(&self) {
        match self {
            Backed::Radio(s) => s.reconnect_bg(),
            Backed::Tcp(s) => s.reconnect_bg(),
        }
    }
    fn is_connected(&self) -> bool {
        match self {
            Backed::Radio(s) => s.is_connected(),
            Backed::Tcp(s) => s.is_connected(),
        }
    }
    fn write_snapshot(&self, snap: &Mutex<AtChatSnapshot>, bridge: &RadioBridge) {
        let (st, connected) = match self {
            Backed::Radio(s) => (s.snapshot(), s.is_connected()),
            Backed::Tcp(s) => (s.snapshot(), s.is_connected()),
        };
        let mut o = snap.lock().unwrap();
        o.my_call = st.callsign;
        o.connected = connected;
        o.role = Some(role_str(st.role));
        o.master = st.master;
        o.roster = st.roster.iter().map(|(c, s, a)| (c.clone(), roster_str(*s), *a)).collect();
        o.transfers = st
            .transfers_in
            .iter()
            .map(|t| TransferLine {
                id: t.id.clone(),
                filename: t.filename.clone(),
                peer: t.peer.clone(),
                incoming: true,
                have: t.have,
                total: t.total,
                complete: t.complete,
            })
            .chain(st.transfers_out.iter().map(|t| TransferLine {
                id: t.id.clone(),
                filename: t.filename.clone(),
                peer: t.peer.clone(),
                incoming: false,
                have: t.have.min(t.total),
                total: t.total,
                complete: t.complete,
            }))
            .collect();
        o.keyed = bridge.tx_pending();
        o.carrier = bridge.carrier_sense();
    }
}

async fn build_station(
    call: &str,
    virtual_addr: &Option<String>,
    bridge: &RadioBridge,
) -> anyhow::Result<Backed> {
    let cfg = StationConfig::default();
    match virtual_addr {
        Some(addr) => {
            let conn = TcpConnector::new(addr.clone(), call);
            Ok(Backed::Tcp(Station::start(conn, call, Mode::Qpsk, cfg).await?))
        }
        None => {
            let conn = RadioConnector::new(bridge.clone());
            Ok(Backed::Radio(Station::start(conn, call, Mode::Qpsk, cfg).await?))
        }
    }
}

fn push_log(snap: &Mutex<AtChatSnapshot>, line: String) {
    let mut o = snap.lock().unwrap();
    o.log.push(line);
    while o.log.len() > LOG_CAP {
        o.log.remove(0);
    }
}

fn push_chat_dedup(snap: &Mutex<AtChatSnapshot>, line: ChatLine) {
    let mut o = snap.lock().unwrap();
    let dup = o
        .chat
        .iter()
        .rev()
        .take(16)
        .any(|c| c.from == line.from && c.text == line.text && c.dst == line.dst);
    if !dup {
        o.chat.push(line);
        while o.chat.len() > CHAT_CAP {
            o.chat.remove(0);
        }
    }
}

async fn session_main(
    mut cmd_rx: UnboundedReceiver<Cmd>,
    snap: Arc<Mutex<AtChatSnapshot>>,
    bridge: RadioBridge,
    mut call: String,
    mut virtual_addr: Option<String>,
) {
    let mut last_err: Option<String> = None;
    loop {
        // (Re)build the station for the current callsign + backend.
        let station = match build_station(&call, &virtual_addr, &bridge).await {
            Ok(s) => s,
            Err(e) => {
                // The station is down. Say so in the snapshot, or a JOIN badge
                // left green by an earlier good station keeps claiming we are on
                // the net while nothing runs and nothing keys the radio. Log the
                // reason once until it changes, not every two seconds.
                {
                    let mut o = snap.lock().unwrap();
                    o.connected = false;
                    o.role = None;
                    o.master = None;
                    o.roster.clear();
                    o.virtual_addr = virtual_addr.clone();
                    o.my_call = call.clone();
                }
                let msg = format!("could not start: {e}");
                if last_err.as_deref() != Some(msg.as_str()) {
                    push_log(&snap, msg.clone());
                    last_err = Some(msg);
                }
                // Stay responsive to a backend switch during the backoff — an
                // operator turning the virtual channel off to fall back to the
                // radio must not have to wait for a connect that cannot succeed.
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(2)) => {}
                    c = cmd_rx.recv() => match c {
                        None => return,
                        Some(Cmd::SetCallsign(x)) => {
                            push_log(&snap, format!("callsign -> {x}; rebuilding"));
                            call = x;
                        }
                        Some(Cmd::SetVirtual(a)) => {
                            push_log(&snap, match &a {
                                Some(addr) => format!("switching to virtual channel {addr}"),
                                None => "switching to the radio (RF)".into(),
                            });
                            virtual_addr = a;
                        }
                        Some(Cmd::Note(s)) => push_log(&snap, s),
                        Some(_) => {}
                    },
                }
                continue;
            }
        };
        last_err = None;
        {
            let mut o = snap.lock().unwrap();
            o.virtual_addr = virtual_addr.clone();
            o.my_call = call.clone();
        }
        push_log(
            &snap,
            match &virtual_addr {
                Some(a) => format!("{call} — station up on virtual channel {a}"),
                None => format!("{call} — station up on the radio (RF)"),
            },
        );

        let mut ev = station.subscribe();
        let mut tick = tokio::time::interval(Duration::from_millis(120));
        let mut rebuild = false;

        while !rebuild {
            tokio::select! {
                _ = tick.tick() => {
                    station.write_snapshot(&snap, &bridge);
                    // Surface anything the RF channel logged since the last tick.
                    for line in bridge.drain_diag() {
                        push_log(&snap, line);
                    }
                }

                r = ev.recv() => match r {
                    Ok(StationEvent::Log(s)) => push_log(&snap, s),
                    Ok(StationEvent::Chat { from, scope, text }) => {
                        let private = scope == ChatScope::Private;
                        push_chat_dedup(&snap, ChatLine {
                            from,
                            dst: if private { "(private)".into() } else { "ALL".into() },
                            text, own: false, private, when: now_unix(),
                        });
                    }
                    Ok(StationEvent::Transfer {
                        dir: TransferDir::In, done: true,
                        saved_path: Some(path), filename, peer, ..
                    }) => {
                        let is_image = looks_like_image(&filename);
                        let mut o = snap.lock().unwrap();
                        o.files.push(RecvFile {
                            from: peer, filename, path, is_image, when: now_unix(),
                        });
                        while o.files.len() > FILE_CAP { o.files.remove(0); }
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(_) => {
                        push_log(&snap, "event stream closed — rebuilding the station".into());
                        rebuild = true;
                    }
                },

                c = cmd_rx.recv() => match c {
                    None => return,
                    Some(Cmd::Chat { dst, text }) => {
                        push_chat_dedup(&snap, ChatLine {
                            from: call.clone(),
                            dst: dst.clone(),
                            text: text.clone(),
                            own: true,
                            private: dst != "ALL",
                            when: now_unix(),
                        });
                        station.chat_bg(&text, &dst);
                    }
                    Some(Cmd::SendFile { path, dst }) => {
                        if looks_like_image(&path.to_string_lossy())
                            && let Ok(md) = std::fs::metadata(&path) {
                                let mut o = snap.lock().unwrap();
                                o.files.push(RecvFile {
                                    from: call.clone(),
                                    filename: path.file_name()
                                        .and_then(|s| s.to_str()).unwrap_or("file").into(),
                                    path: path.to_string_lossy().into_owned(),
                                    is_image: true,
                                    when: now_unix(),
                                });
                                let _ = md;
                                while o.files.len() > FILE_CAP { o.files.remove(0); }
                            }
                        station.send_file(path, &dst);
                    }
                    Some(Cmd::Drop) => {
                        push_log(&snap, "leave requested — dropping the link".into());
                        station.drop_bg();
                    }
                    Some(Cmd::Reconnect) => {
                        if station.is_connected() {
                            push_log(&snap, "rejoin requested, but the link is already up".into());
                        } else {
                            push_log(&snap, "rejoin requested — reconnecting".into());
                            station.reconnect_bg();
                        }
                    }
                    Some(Cmd::ClearChat) => { snap.lock().unwrap().chat.clear(); }
                    Some(Cmd::Note(s)) => push_log(&snap, s),
                    Some(Cmd::SetCallsign(c)) => {
                        push_log(&snap, format!("callsign -> {c}; rebuilding"));
                        call = c;
                        rebuild = true;
                    }
                    Some(Cmd::SetVirtual(a)) => {
                        push_log(&snap, match &a {
                            Some(addr) => format!("switching to virtual channel {addr}"),
                            None => "switching to the radio (RF)".into(),
                        });
                        virtual_addr = a;
                        rebuild = true;
                    }
                },
            }
        }
        // station drops here → tasks abort → loop rebuilds
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modem::{Mode as MMode, Modem};

    fn wait<F: Fn(&AtChatSnapshot) -> bool>(s: &AtChatSession, secs: u64, p: F) -> bool {
        let end = std::time::Instant::now() + Duration::from_secs(secs);
        while std::time::Instant::now() < end {
            if p(&s.snapshot()) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    #[test]
    fn rf_session_hears_a_chat_frame() {
        let s = AtChatSession::new("TA1ABC", None);
        // Wait for the station to come up on the radio backend.
        assert!(wait(&s, 5, |sn| sn.role.is_some()));

        // Synthesise a CHAT frame from TA2DEF and feed it as demodulated audio.
        let modem = Modem::new();
        let frame = br#"{"type":"CHAT","src":"TA2DEF","dst":"ALL","text":"hello over the radio"}"#;
        let wave = modem.modulate(frame, MMode::Qpsk);
        // Pre- and post-roll silence so the segmenter frames it as one burst.
        let sil = vec![0i16; 4000];
        for chunk in
            sil.iter().chain(wave.iter()).chain(sil.iter()).cloned().collect::<Vec<_>>().chunks(160)
        {
            s.push_rx_pcm(chunk);
            std::thread::sleep(Duration::from_millis(2));
        }

        assert!(
            wait(&s, 10, |sn| sn.chat.iter().any(|c| c.text == "hello over the radio" && !c.own)),
            "the demodulated CHAT frame should land in the transcript"
        );
    }

    /// Split a stream on silence, the way the radio segmenter does.
    fn segments(pcm: &[i16]) -> Vec<Vec<i16>> {
        const GATE: i32 = 200;
        const GAP: usize = 400;
        let mut out = Vec::new();
        let mut i = 0;
        while i < pcm.len() {
            if (pcm[i] as i32).abs() <= GATE {
                i += 1;
                continue;
            }
            let start = i.saturating_sub(80);
            let mut last = i;
            let mut j = i;
            while j < pcm.len() && j - last <= GAP {
                if (pcm[j] as i32).abs() > GATE {
                    last = j;
                }
                j += 1;
            }
            out.push(pcm[start..(last + 80).min(pcm.len())].to_vec());
            i = j;
        }
        out
    }

    #[test]
    fn rf_session_transmits_a_chat_frame() {
        let s = AtChatSession::new("TA1ABC", None);
        assert!(wait(&s, 5, |sn| sn.role.is_some()));

        // Discard the startup JOIN_REQUEST burst so only the CHAT is captured.
        let discard_end = std::time::Instant::now() + Duration::from_millis(700);
        while std::time::Instant::now() < discard_end {
            let mut junk = Vec::new();
            s.drain_tx_pcm(&mut junk, 48_000);
            std::thread::sleep(Duration::from_millis(20));
        }

        s.send_chat("ALL", "outgoing message");

        // Collect ~6 s of modulated transmit audio, then segment + demodulate.
        let mut all: Vec<i16> = Vec::new();
        let end = std::time::Instant::now() + Duration::from_secs(6);
        while std::time::Instant::now() < end {
            s.drain_tx_pcm(&mut all, 8_000);
            std::thread::sleep(Duration::from_millis(30));
        }

        let modem = Modem::new();
        let got = segments(&all).iter().any(|seg| {
            modem
                .demodulate(seg)
                .and_then(|p| serde_json::from_slice::<serde_json::Value>(&p).ok())
                .and_then(|v| v.get("text").and_then(|t| t.as_str()).map(str::to_string))
                .as_deref()
                == Some("outgoing message")
        });
        assert!(got, "send_chat should produce a modulated burst that demodulates back");
    }

    #[test]
    fn rf_session_stays_connected_across_a_rejoin() {
        let s = AtChatSession::new("TA1ABC", None);
        assert!(wait(&s, 5, |sn| sn.connected), "station comes up connected");

        s.drop_link();
        assert!(wait(&s, 3, |sn| !sn.connected), "drop takes it off the net");

        s.reconnect();
        assert!(wait(&s, 3, |sn| sn.connected), "rejoin brings it back");

        // The bug: the old carrier segmenter's teardown closed the receive
        // channel, and receive_loop read that as a drop and cleared `connected`
        // a beat after reconnect set it — so a rejoined station went silent.
        // It must now hold.
        std::thread::sleep(Duration::from_millis(800));
        assert!(s.snapshot().connected, "connected must not flip back after the rejoin settles");

        // And a transmit after the rejoin must actually reach the tx ring.
        let mut junk = Vec::new();
        s.drain_tx_pcm(&mut junk, 480_000);
        s.send_chat("ALL", "after rejoin");
        assert!(
            wait(&s, 6, |_| {
                let mut buf = Vec::new();
                s.drain_tx_pcm(&mut buf, 8_000);
                !buf.is_empty()
            }),
            "a rejoined station must be able to transmit"
        );
    }
}
