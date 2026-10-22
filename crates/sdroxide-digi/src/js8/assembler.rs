//! Multi-frame reassembly: turning a run of frames into one message.
//!
//! A JS8 transmission longer than one frame occupies consecutive slots, and the
//! frames carry no sequence number — only the [`Js8Flags::FIRST`] and
//! [`Js8Flags::LAST`] bits. What ties them together is **frequency**: a station
//! stays where it is for the length of a message. JS8Call keys its own buffers
//! the same way (`QMap<int, MessageBuffer> m_messageBuffer; // freq -> …`), with
//! a tolerance for drift.
//!
//! That has a consequence worth stating plainly: two stations transmitting
//! within [`DRIFT_HZ`] of each other in overlapping slots cannot be told apart
//! by this layer, and their text will interleave. Upstream has the same
//! property. Widening the tolerance makes drift-following better and collisions
//! worse; this is the trade JS8 makes.
//!
//! Only the sender's callsign is recoverable, and only when the transmission
//! opened with a directed or compound frame — plain data frames carry no
//! identity at all. A message whose opening frame was missed is still assembled
//! and shown, just without a sender.

use std::collections::HashMap;

use sdroxide_types::{Js8Msg, Js8Speed};

use super::decode::Js8Decode;
use super::frame::{self, Js8Content, Js8Flags};

/// How far a station may drift and still be recognised as the same one.
///
/// Wide enough to follow a warming crystal across a long message, narrow
/// enough that adjacent signals in a crowded sub-band stay distinct.
pub const DRIFT_HZ: f32 = 15.0;

/// Partial messages older than this are given up on.
pub const DEFAULT_TIMEOUT_S: i64 = 300;

/// Most frames one message may span before we stop believing in it.
const MAX_FRAMES: u8 = 64;

/// Partial messages held at once, across all frequencies.
const MAX_PARTIALS: usize = 32;

#[derive(Debug, Clone)]
struct Partial {
    audio_hz: f32,
    speed: Js8Speed,
    from: String,
    to: String,
    cmd: Option<String>,
    text: String,
    frames: u8,
    first_slot_utc: i64,
    last_slot_utc: i64,
    snr_db: i16,
}

impl Partial {
    fn to_msg(&self, complete: bool, my_call: &str, my_groups: &[String]) -> Js8Msg {
        let to = self.to.clone();
        let to_me = !to.is_empty()
            && (to.eq_ignore_ascii_case(my_call)
                || to.eq_ignore_ascii_case("@ALLCALL")
                || my_groups.iter().any(|g| g.eq_ignore_ascii_case(&to)));
        Js8Msg {
            from: self.from.clone(),
            to,
            text: self.text.clone(),
            cmd: self.cmd.clone(),
            snr_db: self.snr_db,
            audio_hz: self.audio_hz,
            first_slot_utc: self.first_slot_utc,
            last_slot_utc: self.last_slot_utc,
            frames: self.frames,
            complete,
            to_me,
            speed: self.speed,
        }
    }
}

/// Gathers frames into messages.
pub struct Js8Assembler {
    partials: Vec<Partial>,
    timeout_s: i64,
    my_call: String,
    my_groups: Vec<String>,
    /// Last time each callsign was heard, for the heard list.
    heard: HashMap<String, i64>,
}

impl Js8Assembler {
    pub fn new(my_call: &str) -> Self {
        Js8Assembler {
            partials: Vec::new(),
            timeout_s: DEFAULT_TIMEOUT_S,
            my_call: my_call.to_ascii_uppercase(),
            my_groups: Vec::new(),
            heard: HashMap::new(),
        }
    }

    pub fn set_my_call(&mut self, call: &str) {
        self.my_call = call.to_ascii_uppercase();
    }

    pub fn set_my_groups(&mut self, groups: Vec<String>) {
        self.my_groups = groups;
    }

    pub fn set_timeout(&mut self, seconds: i64) {
        self.timeout_s = seconds;
    }

    /// Callsigns heard, with the slot each was last decoded in.
    pub fn heard(&self) -> &HashMap<String, i64> {
        &self.heard
    }

    /// Feed one decode. Returns a message when this frame completed one.
    ///
    /// Frames that are complete in themselves — a heartbeat, or a directed
    /// command with both FIRST and LAST — come straight back out.
    pub fn push(&mut self, d: &Js8Decode, slot_utc: i64) -> Option<Js8Msg> {
        let flags = Js8Flags(d.payload.frame_type);
        let content = frame::interpret(&d.payload, flags)?;

        // Find the partial this frame belongs to, or start one.
        let idx = self.slot_for(d, slot_utc, flags);
        let p = &mut self.partials[idx];
        p.frames = p.frames.saturating_add(1);
        p.last_slot_utc = slot_utc;
        p.snr_db = d.snr_db.round() as i16;
        p.audio_hz = d.audio_hz;

        match content {
            Js8Content::Directed(dir) => {
                p.from = dir.from.clone();
                p.to = dir.to.clone();
                p.cmd = frame::cmd_text(dir.cmd)
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());
                if let Some(n) = dir.num {
                    p.text = n.to_string();
                }
                self.heard.insert(dir.from, slot_utc);
            }
            Js8Content::Compound(c) => {
                p.from = c.call.clone();
                if c.is_cq() {
                    p.to = "@ALLCALL".into();
                    p.cmd = Some("CQ".into());
                } else {
                    // A heartbeat invites reports from everyone hearing it;
                    // auto_reply rejects messages without a destination.
                    p.to = "@ALLCALL".into();
                    p.cmd = Some("HB".into());
                }
                if let Some(g) = c.grid() {
                    p.text = g;
                }
                self.heard.insert(c.call, slot_utc);
            }
            Js8Content::Text(t) => p.text.push_str(&t),
        }

        if flags.is_last() || p.frames >= MAX_FRAMES {
            let msg = p.to_msg(true, &self.my_call, &self.my_groups);
            self.partials.remove(idx);
            return Some(msg);
        }
        None
    }

    /// Index of the partial this frame continues, creating one if needed.
    fn slot_for(&mut self, d: &Js8Decode, slot_utc: i64, flags: Js8Flags) -> usize {
        let existing = self
            .partials
            .iter()
            .position(|p| (p.audio_hz - d.audio_hz).abs() <= DRIFT_HZ && p.speed == d.speed);

        match existing {
            // A frame marked FIRST starts a new message even if something was
            // already in progress here — the previous one was abandoned.
            Some(i) if flags.is_first() => {
                self.partials[i] = Partial {
                    audio_hz: d.audio_hz,
                    speed: d.speed,
                    from: String::new(),
                    to: String::new(),
                    cmd: None,
                    text: String::new(),
                    frames: 0,
                    first_slot_utc: slot_utc,
                    last_slot_utc: slot_utc,
                    snr_db: 0,
                };
                i
            }
            Some(i) => i,
            None => {
                if self.partials.len() >= MAX_PARTIALS {
                    // Drop the stalest rather than growing without bound.
                    let oldest = self
                        .partials
                        .iter()
                        .enumerate()
                        .min_by_key(|(_, p)| p.last_slot_utc)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.partials.remove(oldest);
                }
                self.partials.push(Partial {
                    audio_hz: d.audio_hz,
                    speed: d.speed,
                    from: String::new(),
                    to: String::new(),
                    cmd: None,
                    text: String::new(),
                    frames: 0,
                    first_slot_utc: slot_utc,
                    last_slot_utc: slot_utc,
                    snr_db: 0,
                });
                self.partials.len() - 1
            }
        }
    }

    /// Give up on partials that have gone quiet, returning them incomplete so
    /// the operator sees what did arrive rather than nothing at all.
    pub fn expire(&mut self, now_utc: i64) -> Vec<Js8Msg> {
        let timeout = self.timeout_s;
        let (dead, alive): (Vec<_>, Vec<_>) = std::mem::take(&mut self.partials)
            .into_iter()
            .partition(|p| now_utc - p.last_slot_utc > timeout);
        self.partials = alive;
        dead.into_iter()
            .filter(|p| !p.text.is_empty() || !p.from.is_empty())
            .map(|p| p.to_msg(false, &self.my_call, &self.my_groups))
            .collect()
    }

    /// True while any station's multi-frame message is still coming in.
    ///
    /// What the heartbeat acknowledgement checks before transmitting: keying up
    /// mid-message would interleave our frame with someone's sentence.
    pub fn has_partials(&self) -> bool {
        !self.partials.is_empty()
    }

    /// Messages still arriving, for a live view of partial text.
    pub fn in_progress(&self) -> Vec<Js8Msg> {
        self.partials
            .iter()
            .filter(|p| !p.text.is_empty())
            .map(|p| p.to_msg(false, &self.my_call, &self.my_groups))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js8::frame::{Compound, Directed, cmd_code, pack_data, pack_fast_data};
    use crate::js8::message::Js8Payload;

    fn decode(payload: Js8Payload, flags: u8, hz: f32) -> Js8Decode {
        Js8Decode {
            payload: Js8Payload { bits: payload.bits, frame_type: flags },
            audio_hz: hz,
            dt_sec: 0.0,
            snr_db: -5.0,
            sync_score: 3.0,
            hard_errors: 0,
            speed: Js8Speed::Normal,
        }
    }

    fn directed(from: &str, to: &str, cmd: &str) -> Js8Payload {
        Directed {
            from: from.into(),
            to: to.into(),
            cmd: cmd_code(cmd).expect("command"),
            num: None,
            portable_from: false,
            portable_to: false,
        }
        .pack()
        .expect("packs")
    }

    #[test]
    fn a_single_frame_directed_message_completes_immediately() {
        let mut a = Js8Assembler::new("N0JDS");
        let d =
            decode(directed("KN4CRD", "N0JDS", " SNR?"), Js8Flags::FIRST | Js8Flags::LAST, 1500.0);
        let msg = a.push(&d, 1000).expect("completes");
        assert_eq!(msg.from, "KN4CRD");
        assert_eq!(msg.to, "N0JDS");
        assert_eq!(msg.cmd.as_deref(), Some("SNR?"));
        assert!(msg.complete);
        assert!(msg.to_me, "addressed to us");
        assert_eq!(msg.frames, 1);
    }

    #[test]
    fn three_frames_assemble_into_one_message() {
        let mut a = Js8Assembler::new("N0JDS");
        let (f1, _) = pack_data("HELLO ").expect("packs");
        let (f2, _) = pack_fast_data("WORLD ").expect("packs");
        let (f3, _) = pack_fast_data("AGAIN").expect("packs");

        assert!(a.push(&decode(f1, Js8Flags::FIRST, 1500.0), 1000).is_none());
        assert!(a.push(&decode(f2, Js8Flags::DATA, 1500.0), 1015).is_none());
        let msg = a
            .push(&decode(f3, Js8Flags::DATA | Js8Flags::LAST, 1500.0), 1030)
            .expect("completes on LAST");
        assert_eq!(msg.frames, 3);
        assert_eq!(msg.first_slot_utc, 1000);
        assert_eq!(msg.last_slot_utc, 1030);
        assert!(msg.text.starts_with("HELLO"), "got {:?}", msg.text);
        assert!(msg.text.contains("WORLD"), "got {:?}", msg.text);
        assert!(msg.text.ends_with("AGAIN"), "got {:?}", msg.text);
    }

    #[test]
    fn a_station_that_drifts_is_still_the_same_station() {
        let mut a = Js8Assembler::new("N0JDS");
        let (f1, _) = pack_data("HELLO ").expect("packs");
        let (f2, _) = pack_fast_data("WORLD").expect("packs");
        a.push(&decode(f1, Js8Flags::FIRST, 1500.0), 1000);
        // Within DRIFT_HZ, so it continues the same message.
        let msg = a
            .push(&decode(f2, Js8Flags::DATA | Js8Flags::LAST, 1500.0 + DRIFT_HZ - 1.0), 1015)
            .expect("completes");
        assert_eq!(msg.frames, 2);
        assert!(msg.text.contains("WORLD"));
    }

    #[test]
    fn two_stations_far_enough_apart_do_not_interleave() {
        let mut a = Js8Assembler::new("N0JDS");
        let (a1, _) = pack_data("HELLO ").expect("packs");
        let (b1, _) = pack_data("GOOD ").expect("packs");
        let (a2, _) = pack_fast_data("WORLD").expect("packs");
        let (b2, _) = pack_fast_data("MORNING").expect("packs");

        a.push(&decode(a1, Js8Flags::FIRST, 1000.0), 1000);
        a.push(&decode(b1, Js8Flags::FIRST, 2000.0), 1000);
        let msg_a = a
            .push(&decode(a2, Js8Flags::DATA | Js8Flags::LAST, 1000.0), 1015)
            .expect("a completes");
        let msg_b = a
            .push(&decode(b2, Js8Flags::DATA | Js8Flags::LAST, 2000.0), 1015)
            .expect("b completes");
        assert!(msg_a.text.contains("WORLD") && !msg_a.text.contains("MORNING"));
        assert!(msg_b.text.contains("MORNING") && !msg_b.text.contains("WORLD"));
    }

    #[test]
    fn a_new_first_frame_abandons_whatever_was_in_progress() {
        // The previous message was never finished — its sender gave up, or we
        // lost its closing frame. The new one must not inherit its text.
        let mut a = Js8Assembler::new("N0JDS");
        let (old, _) = pack_data("HELLO ").expect("packs");
        let (new, _) = pack_data("GOOD ").expect("packs");
        let (end, _) = pack_fast_data("MORNING").expect("packs");
        a.push(&decode(old, Js8Flags::FIRST, 1500.0), 1000);
        a.push(&decode(new, Js8Flags::FIRST, 1500.0), 1030);
        let msg =
            a.push(&decode(end, Js8Flags::DATA | Js8Flags::LAST, 1500.0), 1045).expect("completes");
        assert!(!msg.text.contains("HELLO"), "leaked the abandoned message: {:?}", msg.text);
        assert!(msg.text.contains("MORNING"));
    }

    #[test]
    fn an_abandoned_partial_expires_and_reports_what_arrived() {
        let mut a = Js8Assembler::new("N0JDS");
        let (f, _) = pack_data("HELLO ").expect("packs");
        a.push(&decode(f, Js8Flags::FIRST, 1500.0), 1000);
        assert!(a.expire(1000 + DEFAULT_TIMEOUT_S).is_empty(), "not yet stale");
        let expired = a.expire(1000 + DEFAULT_TIMEOUT_S + 1);
        assert_eq!(expired.len(), 1);
        assert!(!expired[0].complete, "an expired partial is not complete");
        assert!(expired[0].text.contains("HELLO"));
    }

    #[test]
    fn a_heartbeat_is_a_complete_message_on_its_own() {
        for grid in [Some("EM73"), None] {
            let mut a = Js8Assembler::new("N0JDS");
            let hb = Compound::heartbeat("KN4CRD", grid).pack().expect("packs");
            let msg = a
                .push(&decode(hb, Js8Flags::FIRST | Js8Flags::LAST, 750.0), 2000)
                .expect("completes");
            assert_eq!(msg.from, "KN4CRD");
            assert_eq!(msg.cmd.as_deref(), Some("HB"));
            assert_eq!(msg.to, "@ALLCALL");
            assert!(msg.to_me, "a heartbeat invites a report from anyone hearing it");
            assert!(msg.complete);
            assert_eq!(msg.text, grid.unwrap_or_default());
            assert_eq!(a.heard().get("KN4CRD"), Some(&2000));
        }
    }

    #[test]
    fn a_cq_is_addressed_to_everyone() {
        let mut a = Js8Assembler::new("N0JDS");
        let cq = Compound::cq("VK3ABC", Some("QF22"), 0).pack().expect("packs");
        let msg =
            a.push(&decode(cq, Js8Flags::FIRST | Js8Flags::LAST, 1200.0), 2000).expect("completes");
        assert_eq!(msg.cmd.as_deref(), Some("CQ"));
        assert_eq!(msg.to, "@ALLCALL");
        assert!(msg.to_me, "@ALLCALL reaches us");
    }

    #[test]
    fn messages_for_other_stations_are_not_marked_for_us() {
        let mut a = Js8Assembler::new("N0JDS");
        let d =
            decode(directed("KN4CRD", "VK3ABC", " SNR?"), Js8Flags::FIRST | Js8Flags::LAST, 1500.0);
        let msg = a.push(&d, 1000).expect("completes");
        assert!(!msg.to_me);
    }

    #[test]
    fn a_group_we_belong_to_counts_as_addressed_to_us() {
        let mut a = Js8Assembler::new("N0JDS");
        a.set_my_groups(vec!["@JS8NET".into()]);
        let d = decode(
            directed("KN4CRD", "@JS8NET", " STATUS?"),
            Js8Flags::FIRST | Js8Flags::LAST,
            1500.0,
        );
        assert!(a.push(&d, 1000).expect("completes").to_me);
    }

    #[test]
    fn a_message_whose_opening_frame_was_missed_is_still_shown() {
        // Tuning in mid-transmission is normal. Text without a sender beats
        // silently discarding it.
        let mut a = Js8Assembler::new("N0JDS");
        let (mid, _) = pack_fast_data("WORLD").expect("packs");
        let msg =
            a.push(&decode(mid, Js8Flags::DATA | Js8Flags::LAST, 1500.0), 1000).expect("completes");
        assert!(msg.from.is_empty(), "no sender is knowable");
        assert!(msg.text.contains("WORLD"));
    }

    #[test]
    fn the_partial_table_does_not_grow_without_bound() {
        let mut a = Js8Assembler::new("N0JDS");
        let (f, _) = pack_data("HELLO ").expect("packs");
        for i in 0..(MAX_PARTIALS * 2) {
            // Every one on its own frequency, none ever completed.
            a.push(&decode(f, Js8Flags::FIRST, 500.0 + i as f32 * 50.0), 1000 + i as i64);
        }
        assert!(a.partials.len() <= MAX_PARTIALS);
    }

    #[test]
    fn a_message_that_never_ends_is_cut_off() {
        let mut a = Js8Assembler::new("N0JDS");
        let (f, _) = pack_fast_data("AND").expect("packs");
        let mut completed = None;
        for i in 0..(MAX_FRAMES as i64 + 5) {
            if let Some(m) = a.push(&decode(f, Js8Flags::DATA, 1500.0), 1000 + i * 15) {
                completed = Some(m);
                break;
            }
        }
        let m = completed.expect("a runaway message is eventually closed");
        assert_eq!(m.frames, MAX_FRAMES);
    }
}
