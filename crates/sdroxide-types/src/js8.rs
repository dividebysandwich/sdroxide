//! JS8 vocabulary shared by the engine, the wire protocol and the panel.
//!
//! **Interface facts only.** Every constant transcribed from JS8Call — Costas
//! arrays, LDPC tables, the CRC polynomial, the Huffman and dictionary tables —
//! lives in `sdroxide-digi`, which already carries the GPL obligation. What is
//! here is the vocabulary the UI and the wire need to talk about JS8 at all:
//! how long a slot is, how wide a signal is, who was heard and what they said.
//! Nothing here is derived from JS8Call source, so this crate stays
//! permissive-intent and wasm-clean. Please keep it that way — the tables
//! belong next to the decoder, not next to the enum that names them.

use serde::{Deserialize, Serialize};

/// JS8 transmission speed.
///
/// All four share one 79-symbol frame (21 sync + 58 data, 8-FSK) and differ
/// only in how long a symbol lasts, which trades sensitivity against latency.
/// JS8Call calls these submodes A/B/C/E; a fifth, "Ultra" (JS8I, 384 samples
/// per symbol), exists upstream but ships disabled (`commons.h`
/// `JS8_ENABLE_JS8I 0`), so it is deliberately absent here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Js8Speed {
    /// JS8A — 15 s slots, 50 Hz wide. The band convention; nearly all traffic.
    #[default]
    Normal,
    /// JS8B — 10 s slots, 80 Hz wide.
    Fast,
    /// JS8C — 6 s slots, 160 Hz wide. Local and VHF work.
    Turbo,
    /// JS8E — 30 s slots, 25 Hz wide. The weak-signal end.
    Slow,
}

impl Js8Speed {
    /// Every speed, in declaration order — which is the *wire* order, since
    /// postcard numbers variants by it. Iteration order for anything that has
    /// to touch all four; [`Js8Speed::UI_ORDER`] is what a picker uses.
    pub const ALL: [Js8Speed; 4] =
        [Js8Speed::Normal, Js8Speed::Fast, Js8Speed::Turbo, Js8Speed::Slow];

    /// Every speed as an operator reads them: slowest and most sensitive
    /// first, fastest and widest last (issue #389).
    ///
    /// [`Js8Speed::ALL`] cannot simply be reordered — its order is the wire's
    /// — and the declaration order is an accident of which submodes JS8Call
    /// shipped first (A, B, C, then E), which is not a scale of anything. What
    /// these four *are* is one dial from 30-second slots 25 Hz wide to
    /// 6-second slots 160 Hz wide, and a row of buttons that jumps about in
    /// that dial reads as four unrelated choices instead of one.
    pub const UI_ORDER: [Js8Speed; 4] =
        [Js8Speed::Slow, Js8Speed::Normal, Js8Speed::Fast, Js8Speed::Turbo];

    pub fn label(self) -> &'static str {
        match self {
            Js8Speed::Normal => "NORMAL",
            Js8Speed::Fast => "FAST",
            Js8Speed::Turbo => "TURBO",
            Js8Speed::Slow => "SLOW",
        }
    }

    /// One letter for a place too narrow for the name — the tag on a decoded
    /// message, which has to say which of four waveforms carried it without
    /// taking a column from the text (issue #389).
    pub fn tag(self) -> &'static str {
        match self {
            Js8Speed::Normal => "N",
            Js8Speed::Fast => "F",
            Js8Speed::Turbo => "T",
            Js8Speed::Slow => "S",
        }
    }

    /// Slot period in seconds — how often a transmission may start.
    pub fn slot_s(self) -> f64 {
        match self {
            Js8Speed::Normal => 15.0,
            Js8Speed::Fast => 10.0,
            Js8Speed::Turbo => 6.0,
            Js8Speed::Slow => 30.0,
        }
    }

    /// Delay from the slot boundary to the first symbol, in seconds.
    ///
    /// Unlike FT8, this is *not* the same for every speed — JS8Call sets it per
    /// submode (`commons.h` `JS8*_START_DELAY_MS`).
    pub fn start_delay_s(self) -> f64 {
        match self {
            Js8Speed::Normal | Js8Speed::Slow => 0.5,
            Js8Speed::Fast => 0.2,
            Js8Speed::Turbo => 0.1,
        }
    }

    /// Length of audio a decoder examines, in seconds.
    ///
    /// Equal to the slot period for every speed except Slow, which analyses a
    /// 28 s window inside its 30 s cycle (`NTXDUR` in `js8e_params.f90`).
    /// Conflating the two costs Slow two seconds of search range.
    pub fn decode_window_s(self) -> f64 {
        match self {
            Js8Speed::Slow => 28.0,
            other => other.slot_s(),
        }
    }

    /// Occupied bandwidth in Hz — eight tones at one tone-spacing apart.
    pub fn bandwidth_hz(self) -> f32 {
        match self {
            Js8Speed::Normal => 50.0,
            Js8Speed::Fast => 80.0,
            Js8Speed::Turbo => 160.0,
            Js8Speed::Slow => 25.0,
        }
    }

    /// Tone spacing and symbol rate in Hz (they are numerically equal).
    pub fn tone_spacing_hz(self) -> f32 {
        self.bandwidth_hz() / 8.0
    }

    /// On-air duration of one transmission, in seconds.
    pub fn burst_s(self) -> f64 {
        79.0 / f64::from(self.tone_spacing_hz())
    }

    /// The whole clock for this speed, in the shape every other slotted mode
    /// states it in — so the scheduler and the panels can take JS8's timing
    /// from the same type as FT8's rather than special-casing the one mode
    /// whose slot length is a setting.
    pub fn slot_timing(self) -> crate::SlotTiming {
        crate::SlotTiming {
            slot_s: self.slot_s(),
            tx_offset_s: self.start_delay_s(),
            burst_s: self.burst_s(),
        }
    }

    /// Whether this speed may beacon — send heartbeats, and acknowledge them.
    ///
    /// Turbo may not. It is the local and VHF speed, 160 Hz wide and six
    /// seconds a slot, and an unattended beacon there costs a great deal of a
    /// small band to reach nobody far away. Upstream draws the line in the
    /// same place (`mainwindow.cpp`, `canCurrentModeSendHeartbeat`).
    pub fn allows_heartbeat(self) -> bool {
        !matches!(self, Js8Speed::Turbo)
    }
}

/// Bottom of the heartbeat sub-band, in Hz above the dial.
///
/// Heartbeats have a home so that a station watching for beacons knows where to
/// look, and so that an unattended transmitter does not land on a conversation.
/// JS8Call picks a free slot in 500–1000 Hz for every heartbeat and heartbeat
/// acknowledgement (`mainwindow.cpp`, `findFreeFreqOffset(500, 1000, 50)`).
pub const HB_BAND_LO_HZ: f32 = 500.0;
/// Top of the heartbeat sub-band, in Hz above the dial.
pub const HB_BAND_HI_HZ: f32 = 1000.0;
/// Spacing of the slots the sub-band is divided into.
pub const HB_SLOT_HZ: f32 = 50.0;

/// Which JS8 frame layout a decode came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Js8FrameKind {
    /// A station announcing itself.
    Heartbeat,
    /// A compound (portable/prefixed) callsign, sent alongside other frames.
    Compound,
    /// A compound callsign carrying a directed command.
    CompoundDirected,
    /// A command addressed to a station, a group, or @ALLCALL.
    Directed,
    /// Free text, one frame of possibly several.
    Data,
    /// Free text compressed against the shared dictionary.
    DataCompressed,
    /// Decoded cleanly but not a layout we model.
    #[default]
    Unknown,
}

impl Js8FrameKind {
    pub fn label(self) -> &'static str {
        match self {
            Js8FrameKind::Heartbeat => "HB",
            Js8FrameKind::Compound => "CMP",
            Js8FrameKind::CompoundDirected => "CMPD",
            Js8FrameKind::Directed => "DIR",
            Js8FrameKind::Data => "DATA",
            Js8FrameKind::DataCompressed => "DATAC",
            Js8FrameKind::Unknown => "?",
        }
    }
}

/// Per-decode JS8 detail, carried alongside the shared `Decode` fields.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Js8FrameInfo {
    pub kind: Js8FrameKind,
    pub speed: Js8Speed,
    /// True when this frame closes a multi-frame transmission.
    pub last: bool,
}

/// A station heard on the band.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Js8Heard {
    pub call: String,
    pub grid: Option<String>,
    pub snr_db: i16,
    pub audio_hz: f32,
    /// Unix seconds of the slot this station was last decoded in.
    pub last_utc: i64,
    pub speed: Js8Speed,
}

/// One reassembled JS8 transmission, as the conversation view shows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Js8Msg {
    pub from: String,
    /// Recipient: a callsign, a group, `@ALLCALL`, or empty when undirected.
    pub to: String,
    pub text: String,
    /// The directed command, when this was one (`SNR?`, `GRID?`, …).
    pub cmd: Option<String>,
    pub snr_db: i16,
    pub audio_hz: f32,
    pub first_slot_utc: i64,
    pub last_slot_utc: i64,
    /// Frames seen so far.
    pub frames: u8,
    /// False while a multi-frame message is still arriving.
    pub complete: bool,
    /// Addressed to us, to a group we are in, or to @ALLCALL.
    pub to_me: bool,
    /// The speed this message was decoded at.
    ///
    /// Every frame of one message is the same waveform — the assembler will
    /// not join frames from two speeds — so this is the message's, not the
    /// first frame's. With multi-speed decoding on, four waveforms share the
    /// sub-band and the list was the one place that did not say which of them
    /// a message came in on (issue #389).
    #[serde(default)]
    pub speed: Js8Speed,
}

/// JS8-specific engine status, `None` in every other mode.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Js8Status {
    pub speed: Js8Speed,
    pub heard: Vec<Js8Heard>,
    pub messages: Vec<Js8Msg>,
    /// Frames still queued to transmit, and how many the message started with.
    pub tx_frames_pending: u8,
    pub tx_frames_total: u8,
    /// Seconds until the next automatic heartbeat, when one is scheduled.
    pub next_hb_in_s: Option<u32>,
    /// Audio frequency the last beacon actually went out on, which is not the
    /// working frequency: heartbeats and their acknowledgements move to a free
    /// slot in the sub-band. Shown so an operator watching the waterfall can
    /// tell their own beacon from a stranger's.
    #[serde(default)]
    pub hb_hz: Option<f32>,
    /// Group auto-responses that have fired and still have sends to go.
    #[serde(default)]
    pub auto_pending: Vec<Js8AutoPending>,
}

/// Where an auto-response goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Js8AutoReplyTo {
    /// Directed back to the station that wrote to the group: `KN4CRD …`.
    #[default]
    Sender,
    /// To the whole group the message came in on: `@EMCOMM …`.
    Group,
    /// Exactly as written. The reply names its own addressee, or none.
    AsWritten,
}

impl Js8AutoReplyTo {
    pub const ALL: [Js8AutoReplyTo; 3] =
        [Js8AutoReplyTo::Sender, Js8AutoReplyTo::Group, Js8AutoReplyTo::AsWritten];

    pub fn label(self) -> &'static str {
        match self {
            Js8AutoReplyTo::Sender => "Sender",
            Js8AutoReplyTo::Group => "Group",
            Js8AutoReplyTo::AsWritten => "As written",
        }
    }
}

/// Most auto-response rules a station may hold.
pub const JS8_AUTO_RULES_MAX: usize = 16;
/// Most times one rule may send its reply for one trigger.
pub const JS8_AUTO_REPEAT_MAX: u8 = 10;

/// A canned reply to a message sent to a group.
///
/// For example: when anyone writes to @EMCOMM with "QRV?" in the text,
/// answer "@SENDER QRV @MYGRID" three times, two minutes apart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Js8AutoRule {
    pub enabled: bool,
    /// What the operator calls the rule. Shown in the panel while it runs.
    pub name: String,
    /// The group whose traffic triggers it, `@` included: `@EMCOMM`.
    pub group: String,
    /// A phrase the message must contain, compared ignoring case and spacing.
    /// Empty means any message to the group.
    pub contains: String,
    /// The reply. `@SENDER`, `@GROUP`, `@MYCALL` and `@MYGRID` are replaced
    /// when it is sent.
    pub reply: String,
    pub reply_to: Js8AutoReplyTo,
    /// How many times the reply goes out, 1 to [`JS8_AUTO_REPEAT_MAX`].
    pub repeat: u8,
    /// Minutes between repeats.
    pub gap_min: u16,
    /// Minutes before the same station can trigger this rule again.
    pub cooldown_min: u16,
}

impl Default for Js8AutoRule {
    fn default() -> Self {
        Js8AutoRule {
            enabled: true,
            name: String::new(),
            group: String::new(),
            contains: String::new(),
            reply: String::new(),
            reply_to: Js8AutoReplyTo::Sender,
            repeat: 1,
            gap_min: 2,
            cooldown_min: 30,
        }
    }
}

impl Js8AutoRule {
    /// The rule with every number pulled into its allowed range. A rule
    /// hand-edited in `digi.json` might otherwise ask to repeat 200 times.
    pub fn clamped(&self) -> Js8AutoRule {
        Js8AutoRule {
            repeat: self.repeat.clamp(1, JS8_AUTO_REPEAT_MAX),
            gap_min: self.gap_min.clamp(1, 60),
            cooldown_min: self.cooldown_min.clamp(1, 240),
            ..self.clone()
        }
    }

    /// A short label for the panel: the name, or the group when unnamed.
    pub fn label(&self) -> &str {
        if self.name.trim().is_empty() { self.group.trim() } else { self.name.trim() }
    }

    /// The text this rule sends when `sender` writes to its group, addressee
    /// included, or `None` when there is nothing to send.
    ///
    /// The placeholders are replaced first: `@SENDER`, `@GROUP`, `@MYCALL`,
    /// and `@MYGRID` (four characters). The reply is then addressed as
    /// [`reply_to`](Self::reply_to) says, unless the operator already wrote
    /// that addressee at its start.
    pub fn render_for(&self, sender: &str, my_call: &str, my_grid: &str) -> Option<String> {
        let sender = sender.trim().to_ascii_uppercase();
        let group = js8_group_normalize(&self.group);
        let grid: String = my_grid.trim().chars().take(4).collect::<String>().to_ascii_uppercase();
        let mut body = self.reply.to_ascii_uppercase();
        // Longest name first, so `@MYGRID` is never read as `@MY…`.
        for (key, value) in [
            ("@SENDER", sender.as_str()),
            ("@MYGRID", grid.as_str()),
            ("@MYCALL", &my_call.trim().to_ascii_uppercase()),
            ("@GROUP", group.as_str()),
        ] {
            body = body.replace(key, value);
        }
        let body = js8_normalize_text(&body);
        if body.is_empty() {
            return None;
        }
        let to = match self.reply_to {
            Js8AutoReplyTo::Sender => sender,
            Js8AutoReplyTo::Group => group,
            Js8AutoReplyTo::AsWritten => return Some(body),
        };
        if to.is_empty() {
            return None;
        }
        if body.split_whitespace().next() == Some(to.as_str()) {
            return Some(body);
        }
        Some(format!("{to} {body}"))
    }

    /// True when the rule's own reply would trigger the rule again.
    ///
    /// This happens when the reply goes back to the group and still contains
    /// the phrase. One station alone is safe: it never answers itself. Two
    /// stations running the same rule would answer each other until their
    /// cooldowns ran out.
    pub fn self_retriggers(&self) -> bool {
        let reply = js8_normalize_text(&self.reply);
        let to_group = match self.reply_to {
            Js8AutoReplyTo::Group => true,
            Js8AutoReplyTo::Sender => false,
            Js8AutoReplyTo::AsWritten => reply
                .split_whitespace()
                .next()
                .is_some_and(|t| t == "@GROUP" || t == js8_group_normalize(&self.group)),
        };
        let want = js8_normalize_text(&self.contains);
        to_group && (want.is_empty() || reply.contains(&want))
    }
}

/// Text as auto-response matching compares it: upper case, with runs of
/// whitespace collapsed to single spaces.
pub fn js8_normalize_text(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_ascii_uppercase()
}

/// A group name in its canonical form: trimmed, upper case, with its `@`.
pub fn js8_group_normalize(group: &str) -> String {
    let g = group.trim().to_ascii_uppercase();
    if g.is_empty() || g.starts_with('@') { g } else { format!("@{g}") }
}

/// Whether `group` reads as a JS8 group name: `@`, then up to nine letters
/// and digits, with `/` allowed between them (`@EMCOMM`, `@DX/NA`).
///
/// This is a format check for the settings dialog, not the encoder's last
/// word. A name that passes but cannot be packed is still sent, as plain text.
pub fn js8_group_valid(group: &str) -> bool {
    let Some(name) = group.strip_prefix('@') else { return false };
    let chars = name.chars().filter(|&c| c != '/').count();
    (1..=9).contains(&chars)
        && name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '/')
        && !name.starts_with('/')
}

/// An auto-response waiting to go out, as the panel shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Js8AutoPending {
    /// The rule's label.
    pub rule: String,
    /// The station whose message triggered it.
    pub trigger: String,
    /// The reply, as it will be sent.
    pub text: String,
    /// Sends done so far, and how many there will be in all.
    pub sent: u8,
    pub total: u8,
    /// Seconds until the next send.
    pub next_in_s: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_names_are_checked_for_shape_only() {
        for g in ["@EMCOMM", "@DX/NA", "@MAGNET", "@A", "@REGION/1"] {
            assert!(js8_group_valid(g), "{g}");
        }
        for g in ["EMCOMM", "@", "@emcomm", "@TOOLONGNAME", "@/X", "@EM COMM"] {
            assert!(!js8_group_valid(g), "{g}");
        }
        assert_eq!(js8_group_normalize(" emcomm "), "@EMCOMM");
        assert_eq!(js8_group_normalize("@Ares"), "@ARES");
        assert_eq!(js8_group_normalize("  "), "");
    }

    #[test]
    fn a_reply_that_would_trigger_itself_is_flagged() {
        let base = Js8AutoRule {
            group: "@EMCOMM".into(),
            contains: "QRV?".into(),
            reply: "QRV @MYGRID".into(),
            ..Default::default()
        };
        let loopy = Js8AutoRule {
            reply_to: Js8AutoReplyTo::Group,
            reply: "QRV? ANYONE".into(),
            ..base.clone()
        };
        assert!(loopy.self_retriggers());
        let any = Js8AutoRule {
            reply_to: Js8AutoReplyTo::Group,
            contains: String::new(),
            ..base.clone()
        };
        assert!(any.self_retriggers(), "an empty phrase matches every reply");
        let written = Js8AutoRule {
            reply_to: Js8AutoReplyTo::AsWritten,
            reply: "@GROUP QRV? ANYONE".into(),
            ..base.clone()
        };
        assert!(written.self_retriggers());
        // Back to the sender, it reaches no group, so it cannot trigger
        // anything.
        let sender = Js8AutoRule { reply: "QRV? ANYONE".into(), ..base.clone() };
        assert!(!sender.self_retriggers());
        let group = Js8AutoRule { reply_to: Js8AutoReplyTo::Group, ..base };
        assert!(!group.self_retriggers());
    }

    #[test]
    fn a_hand_edited_rule_cannot_ask_for_more_than_the_limits() {
        let wild = Js8AutoRule { repeat: 200, gap_min: 0, cooldown_min: 0, ..Default::default() };
        let r = wild.clamped();
        assert_eq!((r.repeat, r.gap_min, r.cooldown_min), (JS8_AUTO_REPEAT_MAX, 1, 1));
    }

    /// Issue #389: the buttons run from the slowest, narrowest waveform to the
    /// fastest, widest one, and every speed appears exactly once. The wire
    /// order [`Js8Speed::ALL`] carries is a different question and stays put.
    #[test]
    fn the_speed_buttons_run_slowest_to_fastest() {
        for s in Js8Speed::ALL {
            assert_eq!(
                Js8Speed::UI_ORDER.iter().filter(|x| **x == s).count(),
                1,
                "{} is not on the row exactly once",
                s.label()
            );
        }
        for w in Js8Speed::UI_ORDER.windows(2) {
            assert!(
                w[0].slot_s() > w[1].slot_s(),
                "{} does not come before {}",
                w[0].label(),
                w[1].label()
            );
            assert!(
                w[0].bandwidth_hz() < w[1].bandwidth_hz(),
                "{} is not narrower than {}",
                w[0].label(),
                w[1].label()
            );
        }
        // One letter each, and four different ones.
        let tags: Vec<&str> = Js8Speed::ALL.iter().map(|s| s.tag()).collect();
        for t in &tags {
            assert_eq!(t.len(), 1, "{t} is not one letter");
            assert_eq!(tags.iter().filter(|x| *x == t).count(), 1, "{t} is used twice");
        }
    }

    #[test]
    fn burst_always_fits_inside_its_slot() {
        // Every speed must leave room to key up, transmit 79 symbols and
        // unkey before the next slot starts.
        for s in Js8Speed::ALL {
            let used = s.start_delay_s() + s.burst_s();
            assert!(used < s.slot_s(), "{}: {used:.2}s does not fit in {}s", s.label(), s.slot_s());
        }
    }

    #[test]
    fn burst_lengths_match_the_published_figures() {
        // JS8Call advertises these in its own submode comments; they are an
        // independent check that the bandwidths above are right.
        let want = [
            (Js8Speed::Normal, 12.64),
            (Js8Speed::Fast, 7.90),
            (Js8Speed::Turbo, 3.95),
            (Js8Speed::Slow, 25.28),
        ];
        for (speed, expect) in want {
            assert!(
                (speed.burst_s() - expect).abs() < 0.01,
                "{}: got {:.2}s, want {expect}s",
                speed.label(),
                speed.burst_s()
            );
        }
    }

    #[test]
    fn every_slot_period_divides_a_minute() {
        // Slots are epoch-aligned, so this is what keeps them minute-aligned
        // too — and what lets two stations agree on where a slot starts.
        for s in Js8Speed::ALL {
            assert_eq!(60.0 % s.slot_s(), 0.0, "{}", s.label());
        }
    }

    #[test]
    fn only_slow_analyses_less_than_its_full_cycle() {
        for s in Js8Speed::ALL {
            if s == Js8Speed::Slow {
                assert_eq!(s.decode_window_s(), 28.0);
            } else {
                assert_eq!(s.decode_window_s(), s.slot_s());
            }
        }
    }
}
