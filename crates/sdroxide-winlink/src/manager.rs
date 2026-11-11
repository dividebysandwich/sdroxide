//! The engine-facing half: a mailbox plus a worker thread that runs one
//! forwarding session at a time.
//!
//! Shaped like `sdroxide-net`'s `SpotManager` and for the same reason — the
//! engine must never block. Everything slow (DNS, TCP, a whole B2F exchange)
//! happens on the worker; the engine only calls [`WinlinkManager::poll`] on
//! its existing loop and takes whatever has finished.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crossbeam_channel::{Receiver, Sender, TryRecvError, bounded};
use sdroxide_types::{
    MailAttachment, MailDraft, MailEntry, MailFolder, MailListing, MailMessage, WinlinkConfig,
    WinlinkStatus,
};

use crate::mailbox::{Mailbox, MailboxError};
use crate::message::{Attachment, Message, format_date, generate_mid};
use crate::session::{self, SessionConfig, SessionOutcome};
use crate::transport::{Ax25Transport, TelnetTransport};

/// How long to wait on the CMS before giving up.
const DIAL_TIMEOUT: Duration = Duration::from_secs(30);

/// How long to wait for a gateway to answer a call.
///
/// Far longer than the telnet figure, and deliberately: a SABM is retried
/// against T1 and N2, and on a marginal VHF path or at 300 baud on HF the
/// handshake legitimately takes tens of seconds. Giving up at thirty would
/// abandon sessions that were about to work.
const RADIO_DIAL_TIMEOUT: Duration = Duration::from_secs(120);

/// Where a forwarding session goes.
///
/// The `Transport` trait has always been able to carry a second lane; this is
/// the operator-facing choice of which one, and the reason `run_session` stops
/// hard-coding telnet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WinlinkRoute {
    /// The CMS over the internet.
    Telnet { address: String },
    /// An RMS gateway over the air, optionally through digipeaters.
    ///
    /// `baud` is carried rather than left implicit in the modem because a
    /// speed mismatch is the one packet failure that produces no evidence:
    /// calling a 1200-baud gateway at 9600 is indistinguishable from calling a
    /// gateway that is off the air. Naming it in the transcript is what makes
    /// the difference visible afterwards.
    Packet { gateway: String, via: Vec<String>, baud: sdroxide_types::PacketBaud },
}

/// What a finished session produced.
enum Done {
    Session(Box<SessionOutcome>, Option<String>),
}

/// Releases the busy flag and publishes a result **however** the worker ends —
/// including a panic.
///
/// Without this, a worker that panics leaves `busy` set for ever: the status
/// line keeps saying "calling …", `connect` refuses with "a session is already
/// running", and nothing short of restarting sdroxide gets the operator out of
/// it. Reported from the field, and the reason this is a guard rather than two
/// statements at the end of the closure.
struct SessionGuard {
    busy: Arc<AtomicBool>,
    done_tx: Sender<Done>,
    /// Taken by the worker when it finishes normally; a panic leaves it here
    /// and the drop publishes a failure instead of nothing.
    outcome: Option<(Box<SessionOutcome>, Option<String>)>,
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        // Release before publishing, so a client that reacts to the result can
        // immediately start another session.
        self.busy.store(false, Ordering::SeqCst);
        let (outcome, err) = self.outcome.take().unwrap_or_else(|| {
            (
                Box::new(SessionOutcome::default()),
                Some("the Winlink session ended unexpectedly".into()),
            )
        });
        let _ = self.done_tx.send(Done::Session(outcome, err));
    }
}

/// Owns the mailbox and the session worker.
pub struct WinlinkManager {
    mailbox: Mailbox,
    cfg: WinlinkConfig,
    status: WinlinkStatus,
    /// Set while a session thread is alive, so a second connect is refused
    /// rather than queued — two concurrent sessions on one account is not
    /// something the CMS expects.
    busy: Arc<AtomicBool>,
    /// Set by [`WinlinkManager::abort`]; the transport watches it and gives up.
    cancel: Arc<AtomicBool>,
    done_rx: Receiver<Done>,
    done_tx: Sender<Done>,
    /// The engine's packet link, when the radio is in a packet mode. `None`
    /// means the radio cannot carry a session right now — which is a refusal
    /// the operator can act on, not a failure to report afterwards.
    packet_port: Option<sdroxide_ax25::PortHandle>,
}

impl WinlinkManager {
    /// Open the mailbox under `root` and stand by. Nothing connects until
    /// [`WinlinkManager::connect`] is called.
    pub fn new(root: impl Into<PathBuf>, cfg: WinlinkConfig) -> Result<Self, MailboxError> {
        let mailbox = Mailbox::open(root)?;
        let (done_tx, done_rx) = bounded(4);
        let mut mgr = WinlinkManager {
            mailbox,
            cfg,
            status: WinlinkStatus::default(),
            busy: Arc::new(AtomicBool::new(false)),
            cancel: Arc::new(AtomicBool::new(false)),
            done_rx,
            done_tx,
            packet_port: None,
        };
        mgr.refresh_counts();
        Ok(mgr)
    }

    /// Hand over (or take away) the engine's packet link.
    ///
    /// Called when a packet mode starts and again when it stops, because a mode
    /// change destroys the controller that owns the far end. Taking it away
    /// mid-session is deliberate: the session then fails with a transcript
    /// rather than blocking for ever on a link that no longer exists.
    /// The link handle the manager is holding, so the engine can tell a fresh
    /// port from the one a destroyed controller left behind.
    #[must_use]
    pub fn packet_port(&self) -> Option<&sdroxide_ax25::PortHandle> {
        self.packet_port.as_ref()
    }

    pub fn set_packet_port(&mut self, port: Option<sdroxide_ax25::PortHandle>) {
        self.packet_port = port;
    }

    /// True when a radio session could be started right now.
    #[must_use]
    pub fn packet_available(&self) -> bool {
        self.packet_port.is_some()
    }

    pub fn set_config(&mut self, cfg: WinlinkConfig) {
        self.cfg = cfg;
    }

    pub fn config(&self) -> &WinlinkConfig {
        &self.cfg
    }

    pub fn status(&self) -> &WinlinkStatus {
        &self.status
    }

    pub fn mailbox(&self) -> &Mailbox {
        &self.mailbox
    }

    /// Start a forwarding session on a worker thread.
    ///
    /// Returns an error only for reasons known before any I/O — no credentials,
    /// or a session already running. Anything that goes wrong on the wire
    /// arrives later through [`WinlinkManager::poll`].
    pub fn connect(&mut self, route: WinlinkRoute) -> Result<(), String> {
        if self.busy.load(Ordering::SeqCst) {
            return Err("a Winlink session is already running".into());
        }
        if !self.cfg.is_usable() {
            // Names the APPLY button on purpose: the settings tab keeps edits
            // in a scratch copy until it is pressed, so an account can be typed
            // in, look saved, and never reach here.
            return Err(
                "set a Winlink callsign and password in Settings \u{2192} Winlink, then press APPLY"
                    .into(),
            );
        }

        // Take a snapshot of the outbox now. The worker gets owned data and
        // never touches the mailbox, so the engine stays free to serve reads
        // while a session runs.
        let outbound: Vec<Message> = self
            .mailbox
            .list(MailFolder::Outbox)
            .map_err(|e| format!("reading the outbox: {e}"))?;

        // Fail before spawning anything when the radio cannot carry it. The
        // alternative — a worker that starts and immediately reports failure —
        // reads as a session that went wrong rather than one that was never
        // possible.
        let port = match &route {
            WinlinkRoute::Packet { .. } => match self.packet_port.clone() {
                Some(p) => Some(p),
                None => {
                    return Err(
                        "switch the radio to PACKET or PACKET-HF before forwarding over the air"
                            .into(),
                    );
                }
            },
            WinlinkRoute::Telnet { .. } => None,
        };

        let cfg = SessionConfig {
            callsign: self.cfg.callsign.trim().to_uppercase(),
            password: self.cfg.password.clone(),
            locator: self.cfg.locator.clone(),
            app_name: self.cfg.app_name.clone(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            target_call: match &route {
                WinlinkRoute::Telnet { .. } => crate::transport::CMS_TARGET_CALL.to_string(),
                WinlinkRoute::Packet { gateway, .. } => gateway.to_uppercase(),
            },
        };
        let busy = Arc::clone(&self.busy);
        let done_tx = self.done_tx.clone();
        // A previous abort must not cancel this session before it starts.
        self.cancel.store(false, Ordering::SeqCst);
        let cancel = Arc::clone(&self.cancel);

        busy.store(true, Ordering::SeqCst);
        self.status.busy = true;
        self.status.activity = match &route {
            WinlinkRoute::Telnet { address } => format!("connecting to {address}…"),
            WinlinkRoute::Packet { gateway, .. } => format!("calling {gateway}…"),
        };
        self.status.last_error = None;

        let spawned =
            std::thread::Builder::new().name("sdroxide-winlink".into()).spawn(move || {
                let mut guard = SessionGuard { busy, done_tx, outcome: None };
                let mut outcome = SessionOutcome::default();
                let err = run_session(&route, port, &cancel, &cfg, &outbound, &mut outcome);
                guard.outcome = Some((Box::new(outcome), err));
            });

        if let Err(e) = spawned {
            self.busy.store(false, Ordering::SeqCst);
            self.status.busy = false;
            self.status.activity.clear();
            return Err(format!("starting the Winlink worker: {e}"));
        }
        Ok(())
    }

    /// Stop the session in progress.
    ///
    /// Cooperative rather than a thread kill: the flag is set here and the
    /// transport notices it, so the link is torn down properly and the outcome
    /// — including whatever mail already arrived — is still filed. Harmless
    /// when nothing is running.
    pub fn abort(&mut self) {
        if !self.busy.load(Ordering::SeqCst) {
            return;
        }
        self.cancel.store(true, Ordering::SeqCst);
        self.status.activity = "stopping…".into();
    }

    /// Take whatever a finished session produced. Non-blocking; call it on the
    /// engine's existing tick.
    ///
    /// Returns `true` when the status changed and should be republished.
    pub fn poll(&mut self) -> bool {
        let outcome = match self.done_rx.try_recv() {
            Ok(Done::Session(outcome, err)) => (outcome, err),
            Err(TryRecvError::Empty) => {
                // Belt and braces for the bug that started this: a worker that
                // ends without publishing leaves the window saying "calling …"
                // for ever and `connect` refusing with "already running", and
                // nothing but a restart gets the operator out. `SessionGuard`
                // should make that impossible; this notices if it ever happens
                // anyway, because the cost of being wrong is that high.
                if self.status.busy && !self.busy.load(Ordering::SeqCst) {
                    self.status.busy = false;
                    self.status.activity.clear();
                    if self.status.last_error.is_none() {
                        self.status.last_error =
                            Some("the Winlink session ended without a result".into());
                    }
                    return true;
                }
                return false;
            }
            Err(TryRecvError::Disconnected) => return false,
        };
        let (outcome, err) = outcome;

        self.status.busy = false;
        self.status.activity.clear();
        self.status.log = outcome.log.clone();
        self.status.last_error = err;
        self.status.last_received = outcome.received.len() as u32;
        self.status.last_sent = outcome.sent.len() as u32;

        // File everything that arrived. A message we already hold is dropped
        // rather than overwritten: a redelivery must not clear the read flag or
        // resurrect something the operator has moved on from.
        for msg in &outcome.received {
            if self.mailbox.contains(MailFolder::Inbox, &msg.mid)
                || self.mailbox.contains(MailFolder::Archive, &msg.mid)
            {
                continue;
            }
            if let Err(e) = self.mailbox.put(MailFolder::Inbox, msg) {
                tracing::warn!("filing received message {}: {e}", msg.mid);
            }
        }

        // Anything the peer took moves out of the outbox.
        for mid in &outcome.sent {
            if let Err(e) = self.mailbox.move_to(MailFolder::Outbox, MailFolder::Sent, mid) {
                tracing::warn!("moving sent message {mid}: {e}");
            }
        }

        if self.status.last_error.is_none() {
            self.status.last_session =
                SystemTime::now().duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs() as i64);
        }

        self.refresh_counts();
        true
    }

    /// One page of a folder, newest first.
    pub fn list(&self, folder: MailFolder, offset: u32, count: u32) -> MailListing {
        let count = count.min(sdroxide_types::MAIL_PAGE_MAX) as usize;
        let all = self.mailbox.list(folder).unwrap_or_default();
        let total = all.len() as u32;
        let entries = all
            .into_iter()
            .skip(offset as usize)
            .take(count)
            .map(|msg| entry_of(&msg, folder))
            .collect();
        MailListing { folder, offset, total, entries }
    }

    /// One whole message, with attachments.
    pub fn get(&self, folder: MailFolder, mid: &str) -> Option<MailMessage> {
        self.mailbox.get(folder, mid).ok().map(|msg| wire_of(&msg, folder))
    }

    /// File a composed message in the outbox, minting its MID and date.
    ///
    /// Both are chosen here rather than by the caller: a MID must be unique or
    /// the CMS rejects the message, and a client is in no position to
    /// guarantee that.
    pub fn compose(&mut self, draft: &MailDraft) -> Result<String, String> {
        let callsign = self.cfg.callsign.trim().to_uppercase();
        if callsign.is_empty() {
            return Err("set a Winlink callsign first".into());
        }
        if draft.to.iter().all(|t| t.trim().is_empty()) {
            return Err("a message needs at least one recipient".into());
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        let msg = Message {
            mid: generate_mid(&callsign, now.as_nanos()),
            date: format_date(now.as_secs() as i64),
            msg_type: "Private".into(),
            from: callsign.clone(),
            to: draft.to.iter().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect(),
            cc: draft.cc.iter().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect(),
            subject: draft.subject.clone(),
            mbo: callsign,
            body: draft.body.clone(),
            attachments: draft
                .attachments
                .iter()
                .map(|a| Attachment { name: a.name.clone(), data: a.data.clone() })
                .collect(),
        };

        self.mailbox
            .put(MailFolder::Outbox, &msg)
            .map_err(|e| format!("filing the message: {e}"))?;
        self.refresh_counts();
        Ok(msg.mid)
    }

    pub fn delete(&mut self, folder: MailFolder, mid: &str) -> Result<(), String> {
        self.mailbox.remove(folder, mid).map_err(|e| format!("deleting {mid}: {e}"))?;
        self.refresh_counts();
        Ok(())
    }

    pub fn move_to(&mut self, from: MailFolder, to: MailFolder, mid: &str) -> Result<(), String> {
        self.mailbox.move_to(from, to, mid).map_err(|e| format!("moving {mid}: {e}"))?;
        self.refresh_counts();
        Ok(())
    }

    fn refresh_counts(&mut self) {
        for (i, folder) in MailFolder::ALL.into_iter().enumerate() {
            self.status.counts[i] = self.mailbox.count(folder) as u32;
        }
    }
}

/// Dial and run one session, returning the error text if it failed.
fn run_session(
    route: &WinlinkRoute,
    port: Option<sdroxide_ax25::PortHandle>,
    cancel: &Arc<AtomicBool>,
    cfg: &SessionConfig,
    outbound: &[Message],
    outcome: &mut SessionOutcome,
) -> Option<String> {
    match route {
        WinlinkRoute::Telnet { address } => {
            let mut transport = match TelnetTransport::dial(address, &cfg.callsign, DIAL_TIMEOUT) {
                Ok(t) => t,
                Err(e) => return Some(e.to_string()),
            };
            outcome.log.extend(transport.login_log().iter().cloned());
            match session::run_into(&mut transport, cfg, outbound, outcome) {
                Ok(()) => None,
                Err(e) => Some(e.to_string()),
            }
        }
        WinlinkRoute::Packet { gateway, via, baud } => {
            let port = port.expect("connect() proved the link exists before spawning");
            outcome.log.push(format!("> calling {gateway} at {} baud", baud.label()));
            let mut transport = match Ax25Transport::connect(
                port,
                gateway,
                via,
                RADIO_DIAL_TIMEOUT,
                Arc::clone(cancel),
            ) {
                Ok(t) => t,
                Err(e) => return Some(e.to_string()),
            };
            outcome.log.push(format!("< connected to {gateway}"));
            match session::run_into(&mut transport, cfg, outbound, outcome) {
                Ok(()) => None,
                Err(e) => Some(e.to_string()),
            }
        }
    }
}

/// Metadata for a list row.
fn entry_of(msg: &Message, folder: MailFolder) -> MailEntry {
    MailEntry {
        mid: msg.mid.clone(),
        date: msg.date.clone(),
        from: msg.from.clone(),
        to: msg.to.join(", "),
        subject: msg.subject.clone(),
        folder,
        bytes: msg.encode().len() as u64,
        attachments: msg.attachments.len() as u32,
        // Read state is not stored yet; everything in the inbox reads as new.
        unread: folder == MailFolder::Inbox,
    }
}

/// The wire form of a whole message.
fn wire_of(msg: &Message, folder: MailFolder) -> MailMessage {
    MailMessage {
        mid: msg.mid.clone(),
        date: msg.date.clone(),
        msg_type: msg.msg_type.clone(),
        from: msg.from.clone(),
        to: msg.to.clone(),
        cc: msg.cc.clone(),
        subject: msg.subject.clone(),
        body: msg.body.clone(),
        attachments: msg
            .attachments
            .iter()
            .map(|a| MailAttachment { name: a.name.clone(), data: a.data.clone() })
            .collect(),
        folder,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);
    impl TempDir {
        fn new(tag: &str) -> TempDir {
            let p =
                std::env::temp_dir().join(format!("sdroxide-wlmgr-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&p);
            TempDir(p)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn config() -> WinlinkConfig {
        WinlinkConfig { callsign: "OE3JJS".into(), password: "secret".into(), ..Default::default() }
    }

    fn draft() -> MailDraft {
        MailDraft {
            to: vec!["OE1XYZ".into()],
            cc: vec![],
            subject: "hello".into(),
            body: "body\r\n".into(),
            attachments: vec![],
        }
    }

    #[test]
    fn composing_files_to_the_outbox() {
        let dir = TempDir::new("compose");
        let mut mgr = WinlinkManager::new(&dir.0, config()).unwrap();

        let mid = mgr.compose(&draft()).unwrap();
        assert_eq!(mgr.status().counts[MailFolder::Outbox as usize], 1);

        let msg = mgr.get(MailFolder::Outbox, &mid).unwrap();
        assert_eq!(msg.from, "OE3JJS");
        assert_eq!(msg.to, ["OE1XYZ"]);
        assert_eq!(msg.subject, "hello");
        // Minted server-side, not supplied by the caller.
        assert!(!msg.mid.is_empty());
        assert!(!msg.date.is_empty());
    }

    #[test]
    fn composing_needs_a_recipient_and_a_callsign() {
        let dir = TempDir::new("bad");
        let mut mgr = WinlinkManager::new(&dir.0, config()).unwrap();
        let empty = MailDraft { to: vec!["  ".into()], ..draft() };
        assert!(mgr.compose(&empty).is_err());

        let mut anon = WinlinkManager::new(&dir.0, WinlinkConfig::default()).unwrap();
        assert!(anon.compose(&draft()).is_err());
    }

    #[test]
    fn connecting_without_credentials_is_refused_before_any_io() {
        let dir = TempDir::new("nocreds");
        let mut mgr = WinlinkManager::new(&dir.0, WinlinkConfig::default()).unwrap();
        assert!(mgr.connect(WinlinkRoute::Telnet { address: String::new() }).is_err());
    }

    /// Aborting when nothing is running must be harmless, not a state change
    /// the operator then has to undo.
    #[test]
    fn aborting_an_idle_manager_does_nothing() {
        let dir = TempDir::new("abort-idle");
        let mut mgr = WinlinkManager::new(&dir.0, WinlinkConfig::default()).expect("open");
        mgr.abort();
        assert!(!mgr.status().busy);
        assert!(mgr.status().activity.is_empty());
    }

    /// The state the field report described: busy set, but the worker gone
    /// without publishing. The operator must get the window back rather than
    /// being locked out until a restart.
    #[test]
    fn a_lost_result_still_frees_the_window() {
        let dir = TempDir::new("lost-result");
        let mut mgr = WinlinkManager::new(&dir.0, WinlinkConfig::default()).expect("open");
        // Exactly what a vanished worker leaves behind.
        mgr.status.busy = true;
        mgr.status.activity = "calling OE1XIK…".into();

        assert!(mgr.poll(), "poll should have noticed and republished");
        assert!(!mgr.status().busy, "the window is still stuck on busy");
        assert!(mgr.status().activity.is_empty(), "the activity line never cleared");
        assert!(mgr.status().last_error.is_some(), "the operator was told nothing");
    }

    #[test]
    fn listing_pages_and_reports_the_total() {
        let dir = TempDir::new("page");
        let mut mgr = WinlinkManager::new(&dir.0, config()).unwrap();
        for i in 0..5 {
            let d = MailDraft { subject: format!("msg {i}"), ..draft() };
            mgr.compose(&d).unwrap();
        }

        let page = mgr.list(MailFolder::Outbox, 0, 2);
        assert_eq!(page.total, 5, "total is the folder size, not the page size");
        assert_eq!(page.entries.len(), 2);

        let rest = mgr.list(MailFolder::Outbox, 4, 10);
        assert_eq!(rest.entries.len(), 1);
    }

    #[test]
    fn listing_is_capped_even_if_a_client_asks_for_more() {
        let dir = TempDir::new("cap");
        let mgr = WinlinkManager::new(&dir.0, config()).unwrap();
        // The count comes off a socket; it must not be believed unbounded.
        let page = mgr.list(MailFolder::Inbox, 0, u32::MAX);
        assert!(page.entries.len() <= sdroxide_types::MAIL_PAGE_MAX as usize);
    }

    #[test]
    fn deleting_and_moving_update_the_counts() {
        let dir = TempDir::new("move");
        let mut mgr = WinlinkManager::new(&dir.0, config()).unwrap();
        let mid = mgr.compose(&draft()).unwrap();

        mgr.move_to(MailFolder::Outbox, MailFolder::Archive, &mid).unwrap();
        assert_eq!(mgr.status().counts[MailFolder::Outbox as usize], 0);
        assert_eq!(mgr.status().counts[MailFolder::Archive as usize], 1);

        mgr.delete(MailFolder::Archive, &mid).unwrap();
        assert_eq!(mgr.status().counts[MailFolder::Archive as usize], 0);
    }

    #[test]
    fn attachments_survive_compose_and_fetch() {
        let dir = TempDir::new("att");
        let mut mgr = WinlinkManager::new(&dir.0, config()).unwrap();
        let d = MailDraft {
            attachments: vec![MailAttachment {
                name: "photo.jpg".into(),
                data: vec![0xff, 0xd8, 0x00, 0x0a],
            }],
            ..draft()
        };
        let mid = mgr.compose(&d).unwrap();
        let got = mgr.get(MailFolder::Outbox, &mid).unwrap();
        assert_eq!(got.attachments[0].data, vec![0xff, 0xd8, 0x00, 0x0a]);
    }
}
