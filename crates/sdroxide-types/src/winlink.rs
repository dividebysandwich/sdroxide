//! Winlink radio-email vocabulary: the mailbox as the UI sees it, and the
//! account settings a session needs.
//!
//! The protocol itself lives in `sdroxide-winlink`, which is native-only. What
//! is here is only the shape of a mailbox and a session's status, because both
//! have to cross the wire to a browser client that links none of that code.
//!
//! The mailbox lives on the machine with the radio and is read **lazily**: a
//! listing carries metadata, and the body and attachments are fetched one
//! message at a time. That is the same arrangement the picture store uses, and
//! for the same reason — a mailbox with attachments in it is far too big to
//! mirror into every client on connect.

use serde::{Deserialize, Serialize};

/// Where a message sits in its life.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub enum MailFolder {
    /// Arrived from a forwarding partner.
    #[default]
    Inbox,
    /// Composed here, waiting for a session to carry it.
    Outbox,
    /// Handed to a forwarding partner and accepted.
    Sent,
    /// Kept, but out of the way.
    Archive,
}

impl MailFolder {
    pub const ALL: [MailFolder; 4] =
        [MailFolder::Inbox, MailFolder::Outbox, MailFolder::Sent, MailFolder::Archive];

    /// Directory name on disk. Also the stable key in the wire protocol, so it
    /// must not change once written.
    pub fn dir_name(self) -> &'static str {
        match self {
            MailFolder::Inbox => "inbox",
            MailFolder::Outbox => "outbox",
            MailFolder::Sent => "sent",
            MailFolder::Archive => "archive",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            MailFolder::Inbox => "INBOX",
            MailFolder::Outbox => "OUTBOX",
            MailFolder::Sent => "SENT",
            MailFolder::Archive => "ARCHIVE",
        }
    }
}

/// Most entries one listing request may ask for, so a client cannot make the
/// server build an unbounded response.
pub const MAIL_PAGE_MAX: u32 = 200;

/// One message's metadata — enough to draw a list row without fetching it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MailEntry {
    pub mid: String,
    /// `YYYY/MM/DD HH:MM`, UTC, as the message carries it.
    pub date: String,
    pub from: String,
    /// Recipients, already joined for display.
    pub to: String,
    pub subject: String,
    pub folder: MailFolder,
    /// Size of the stored message, so a list can say what a fetch will cost
    /// before pulling it down a phone link.
    pub bytes: u64,
    pub attachments: u32,
    /// Not yet opened. Only meaningful in the inbox.
    pub unread: bool,
}

/// One page of a folder.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MailListing {
    pub folder: MailFolder,
    /// Where `entries` starts within the folder.
    pub offset: u32,
    /// How many the folder holds altogether, so the UI can page exactly rather
    /// than guessing from what it was given.
    pub total: u32,
    pub entries: Vec<MailEntry>,
}

/// An attachment, with its bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MailAttachment {
    pub name: String,
    pub data: Vec<u8>,
}

/// One whole message, answering a fetch.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MailMessage {
    pub mid: String,
    pub date: String,
    /// `Private` for ordinary mail; also `Service`, `Inquiry`, `Position`.
    pub msg_type: String,
    pub from: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: String,
    pub body: String,
    pub attachments: Vec<MailAttachment>,
    pub folder: MailFolder,
}

/// A message the operator has composed, on its way to the outbox.
///
/// Separate from [`MailMessage`] because the fields the operator fills in are
/// not the fields a stored message has: the MID and date are minted server-side
/// at the moment it is filed, and a client must not choose either.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MailDraft {
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: String,
    pub body: String,
    pub attachments: Vec<MailAttachment>,
}

/// What a forwarding session is doing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WinlinkStatus {
    /// A session is running right now.
    pub busy: bool,
    /// One line on what it is doing, for the window header.
    pub activity: String,
    /// Why the last session ended badly, if it did.
    pub last_error: Option<String>,
    /// Unix seconds of the last session that completed cleanly.
    pub last_session: Option<i64>,
    /// Messages received and sent by the last session.
    pub last_received: u32,
    pub last_sent: u32,
    /// Per-folder counts, in [`MailFolder::ALL`] order, so the folder tabs can
    /// show totals without listing anything.
    pub counts: [u32; 4],
    /// The protocol transcript of the most recent session. Kept because a
    /// forwarding session that fails is unreadable without it.
    pub log: Vec<String>,
}

/// The gateway the CMS telnet transport dials by default.
pub const DEFAULT_CMS_ADDRESS: &str = "server.winlink.org:8772";

/// Which lane a forwarding session should use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WinlinkLane {
    /// The CMS over the internet.
    #[default]
    Telnet,
    /// An RMS gateway over the air, using the radio's current packet mode.
    Packet,
    /// An RMS gateway over the air on VARA HF. Appended — postcard numbers
    /// variants by position.
    Vara,
}

/// One RMS gateway the operator has written down.
///
/// Winlink publishes the real gateway list through `api.winlink.org`'s
/// `ChannelList`, which refuses every request without an access key we do not
/// have. So the operator keeps their own list, which is what a packet operator
/// does anyway: the two or three gateways actually reachable from a given
/// location are learned by trying, and they do not change often.
///
/// When a key does arrive, the fetched list populates exactly this structure —
/// nothing above it needs to know where the entries came from.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WinlinkGateway {
    /// The gateway's callsign, including SSID — `OE1XAR-10`.
    pub callsign: String,
    /// Digipeaters to reach it through, in order. Usually empty.
    pub via: Vec<String>,
    /// Where to tune. Zero means "wherever the radio already is", which is the
    /// right answer for an operator who parks on one channel.
    pub freq_hz: f64,
    /// Speed this gateway runs. Selecting the gateway sets the modem to match,
    /// because calling a 1200-baud gateway at 9600 produces silence and no
    /// explanation.
    pub baud: crate::PacketBaud,
    /// Free text for the operator — "Vienna, 24/7" — shown in the picker.
    pub label: String,
}

/// Account settings for Winlink.
///
/// The password is the Winlink *account* password, which answers the secure
/// login challenge. It is stored in plaintext alongside the other service
/// credentials in [`crate::NetworkConfig`] — the same posture as QRZ, LoTW and
/// Club Log, and worth knowing rather than assuming otherwise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WinlinkConfig {
    /// The account callsign. Upper-cased on use.
    pub callsign: String,
    /// Account password. **Case-sensitive** — the challenge response differs
    /// for `FOOBAR` and `FooBar`, so it must be entered exactly as issued.
    pub password: String,
    /// Maidenhead locator reported in the session greeting. Cosmetic.
    pub locator: String,
    /// Which CMS to dial. Overridable mainly for testing.
    pub cms_address: String,
    /// The client name announced in our SID.
    ///
    /// Winlink's production CMS refuses names it does not know, answering
    /// `*** Unknown client types are not allowed on production servers`. Until
    /// "sdroxide" is registered with the Winlink Development Team this is the
    /// knob that lets an operator get in, and it is deliberately visible in
    /// settings rather than silently defaulted to somebody else's client name.
    pub app_name: String,
    /// Which lane [`crate::Command::WinlinkConnect`] uses by default.
    #[serde(default)]
    pub lane: WinlinkLane,
    /// The RMS gateway to call over the air, e.g. `OE1XAR-10`.
    #[serde(default)]
    pub gateway: String,
    /// Digipeaters to route through, in order. Usually empty.
    #[serde(default)]
    pub gateway_via: Vec<String>,
    /// The speed the gateway named above runs.
    ///
    /// Held here rather than read from the modem because it is a property of
    /// *that gateway*, not of the band or of the operator's last session: a
    /// 2 m RMS may be 1200 or 9600, and the only way to know is to have been
    /// told. Applied to the modem when the packet lane connects.
    #[serde(default)]
    pub gateway_baud: crate::PacketBaud,
    /// Where the gateway listens. Zero means "wherever the radio already is",
    /// which is the right answer for an operator who parks on one channel.
    #[serde(default)]
    pub gateway_freq_hz: f64,
    /// The operator's own gateway list, since the published one needs an API
    /// key. Picking one fills in `gateway`, `gateway_via`, `gateway_baud` and
    /// `gateway_freq_hz`.
    #[serde(default)]
    pub gateways: Vec<WinlinkGateway>,
    /// Connect automatically on a timer.
    pub auto_connect: bool,
    /// How often to connect when `auto_connect` is on.
    pub auto_connect_minutes: u32,
}

impl Default for WinlinkConfig {
    fn default() -> Self {
        WinlinkConfig {
            callsign: String::new(),
            password: String::new(),
            locator: String::new(),
            cms_address: DEFAULT_CMS_ADDRESS.to_string(),
            app_name: "sdroxide".to_string(),
            lane: WinlinkLane::default(),
            gateway: String::new(),
            gateway_via: Vec::new(),
            gateway_baud: crate::PacketBaud::default(),
            gateway_freq_hz: 0.0,
            gateways: Vec::new(),
            auto_connect: false,
            auto_connect_minutes: 30,
        }
    }
}

impl WinlinkConfig {
    /// Whether a session could even be attempted.
    pub fn is_usable(&self) -> bool {
        !self.callsign.trim().is_empty() && !self.password.is_empty()
    }

    /// Make the `i`th saved gateway the one the next connect calls.
    ///
    /// Every field the gateway knows about itself moves, not just its name.
    /// This lives here rather than in the settings panel because copying only
    /// the callsign is a silent bug — the session then runs at whatever speed
    /// the modem happened to be on, and calling a 1200-baud gateway at 9600
    /// is indistinguishable from calling one that is off the air.
    ///
    /// Out-of-range indices are ignored, so a list edited concurrently by a
    /// second client cannot panic the one that was looking at it.
    pub fn select_gateway(&mut self, i: usize) {
        let Some(g) = self.gateways.get(i).cloned() else { return };
        self.gateway = g.callsign;
        self.gateway_via = g.via;
        self.gateway_baud = g.baud;
        self.gateway_freq_hz = g.freq_hz;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_dir_names_are_stable() {
        // These name directories on disk and ride the wire; changing one
        // orphans an operator's stored mail.
        let names: Vec<&str> = MailFolder::ALL.iter().map(|f| f.dir_name()).collect();
        assert_eq!(names, ["inbox", "outbox", "sent", "archive"]);
    }

    #[test]
    fn counts_line_up_with_folders() {
        let status = WinlinkStatus::default();
        assert_eq!(status.counts.len(), MailFolder::ALL.len());
    }

    #[test]
    fn default_config_is_not_usable() {
        // Nothing should try to connect until the operator has filled it in.
        assert!(!WinlinkConfig::default().is_usable());
        let cfg =
            WinlinkConfig { callsign: "OE3JJS".into(), password: "x".into(), ..Default::default() };
        assert!(cfg.is_usable());
        // Whitespace is not a callsign.
        let blank =
            WinlinkConfig { callsign: "   ".into(), password: "x".into(), ..Default::default() };
        assert!(!blank.is_usable());
    }

    fn two_gateways() -> WinlinkConfig {
        WinlinkConfig {
            gateways: vec![
                WinlinkGateway {
                    callsign: "OE1XAR-10".into(),
                    via: vec!["OE3XLR-1".into()],
                    freq_hz: 144_800_000.0,
                    baud: crate::PacketBaud::Vhf1200,
                    label: "Vienna".into(),
                },
                WinlinkGateway {
                    callsign: "DB0FHN-10".into(),
                    via: Vec::new(),
                    freq_hz: 438_425_000.0,
                    baud: crate::PacketBaud::Vhf9600,
                    label: String::new(),
                },
            ],
            ..Default::default()
        }
    }

    #[test]
    fn selecting_a_gateway_takes_its_speed_and_channel() {
        // The whole point: a 9600 gateway must arrive as 9600. Copying only
        // the callsign left the modem at whatever it was, and the session
        // then failed with no evidence of why.
        let mut cfg = two_gateways();
        cfg.select_gateway(1);
        assert_eq!(cfg.gateway, "DB0FHN-10");
        assert_eq!(cfg.gateway_baud, crate::PacketBaud::Vhf9600);
        assert_eq!(cfg.gateway_freq_hz, 438_425_000.0);
        assert!(cfg.gateway_via.is_empty());
    }

    #[test]
    fn selecting_another_gateway_clears_the_previous_path() {
        // Switching from a digipeated gateway to a direct one must not leave
        // the old path behind: the call would go out via a digipeater the new
        // gateway has never heard of.
        let mut cfg = two_gateways();
        cfg.select_gateway(0);
        assert_eq!(cfg.gateway_via, ["OE3XLR-1"]);
        cfg.select_gateway(1);
        assert!(cfg.gateway_via.is_empty(), "the Vienna digipeater followed us to Franconia");
        assert_eq!(cfg.gateway_baud, crate::PacketBaud::Vhf9600);
    }

    #[test]
    fn selecting_a_gateway_that_is_not_there_changes_nothing() {
        let mut cfg = two_gateways();
        cfg.select_gateway(0);
        let before = cfg.clone();
        cfg.select_gateway(7);
        assert_eq!(cfg, before);
    }
}
