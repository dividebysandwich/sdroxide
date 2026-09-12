//! A port of the data structures in `client.py` + the events published to the GUI.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::netproto::Mode;
use tokio::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Listener,
    Master,
    Backup,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Listener => "LISTENER",
            Role::Master => "MASTER",
            Role::Backup => "BACKUP",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RosterStatus {
    Active,
    Lost,
}

#[derive(Debug, Clone)]
pub struct RosterEntry {
    pub last_seen: Instant,
    pub status: RosterStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDir {
    In,
    Out,
}

/// A bulk transfer this station is SENDING.
#[derive(Debug, Clone)]
pub struct TransferOut {
    pub transfer_id: String,
    pub filename: String,
    pub dst: String,
    pub blocks: BTreeMap<usize, Vec<u8>>,
    pub mode: Mode,
    pub arq_round: usize,
    /// Sent-block counter for the GUI progress bar (grows across ARQ rounds).
    pub sent: usize,
    pub done: bool,
}

/// A bulk transfer this station is RECEIVING.
#[derive(Debug, Clone)]
pub struct TransferIn {
    pub transfer_id: String,
    pub filename: String,
    pub total_blocks: usize,
    pub src: String,
    pub dst: String,
    pub received: BTreeMap<usize, Vec<u8>>,
    pub complete: bool,
    pub saved_path: Option<String>,
}

impl TransferIn {
    pub fn missing_blocks(&self) -> Vec<usize> {
        (0..self.total_blocks).filter(|s| !self.received.contains_key(s)).collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatScope {
    /// `dst == "ALL"`
    Broadcast,
    /// `dst == <this station>`
    Private,
}

/// The station events the GUI listens for. The structured equivalent of
/// `client.py`'s `print(...)` calls.
#[derive(Debug, Clone)]
pub enum StationEvent {
    /// `self.log(msg)` — a free-text log line.
    Log(String),
    /// An incoming chat message.
    Chat {
        from: String,
        scope: ChatScope,
        text: String,
    },
    RoleChanged(Role),
    /// The roster or transfer table changed — the GUI should re-read the snapshot.
    StateChanged,
    /// A transfer advanced or completed.
    Transfer {
        id: String,
        dir: TransferDir,
        filename: String,
        /// The other station (the source for an incoming transfer, the destination for an outgoing one).
        peer: String,
        have: usize,
        total: usize,
        done: bool,
        saved_path: Option<String>,
    },
}

/// Timing parameters. The defaults are the `netproto.py` constants (the GUI
/// uses these — the "real ~24 s master election" from CLAUDE.md). Tests shorten them.
#[derive(Debug, Clone)]
pub struct StationConfig {
    pub beacon_interval: Duration,
    pub beacon_timeout: Duration,
    pub lost_timeout: Duration,
    pub remove_timeout: Duration,
    /// `_send_blocks`: a control window every N blocks (CLAUDE.md bug #3).
    pub control_window_every: usize,
    pub control_window_pause: Duration,
    /// The directory received files are written to (`client.py`: "received").
    pub received_dir: PathBuf,
}

impl Default for StationConfig {
    fn default() -> Self {
        Self {
            beacon_interval: Duration::from_secs_f64(crate::netproto::BEACON_INTERVAL),
            beacon_timeout: Duration::from_secs_f64(crate::netproto::BEACON_TIMEOUT),
            lost_timeout: Duration::from_secs_f64(crate::netproto::LOST_TIMEOUT),
            remove_timeout: Duration::from_secs_f64(crate::netproto::REMOVE_TIMEOUT),
            control_window_every: 3,
            control_window_pause: Duration::from_millis(1200),
            received_dir: PathBuf::from("received"),
        }
    }
}

/// The station snapshot the GUI reads every frame.
#[derive(Debug, Clone)]
pub struct StationSnapshot {
    pub callsign: String,
    pub role: Role,
    pub master: Option<String>,
    pub backup: Option<String>,
    pub connected: bool,
    pub roster: Vec<(String, RosterStatus, f64)>, // (callsign, status, seconds since last seen)
    pub transfers_in: Vec<TransferSnapshot>,
    pub transfers_out: Vec<TransferSnapshot>,
}

#[derive(Debug, Clone)]
pub struct TransferSnapshot {
    pub id: String,
    pub filename: String,
    pub peer: String,
    pub have: usize,
    pub total: usize,
    pub complete: bool,
    pub arq_round: usize,
}
