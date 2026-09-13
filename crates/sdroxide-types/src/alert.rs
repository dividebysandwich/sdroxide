//! Audible alert preferences (persisted in `config.toml` under `[alerts]`).
//! Kept wasm-safe (no I/O), like every other settings struct here.
//!
//! This is a *client-side* preference, in the same sense as `[ui]` and
//! `[speech]`: it says what the operator sitting at this screen wants to hear,
//! not how the station is configured. It never travels as a [`crate::Command`],
//! and no tab has to wait for the engine to seed it.

use serde::{Deserialize, Serialize};

use crate::Decode;
use crate::awards::Novelty;
use crate::cq_is_for_us;

/// The alarm sounds an event can be answered with.
///
/// Synthesised at playback time rather than shipped as files — a handful of
/// short tones is a few dozen lines of oscillator, and there is no WAV to
/// install, lose, or license. Each is distinct enough to tell apart from the
/// other room, which is the point of having more than one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AlertSound {
    /// A single soft "ding" — the plainest, least intrusive alert.
    #[default]
    Ding,
    /// A low-high pair, the classic "attention" signature.
    TwoTone,
    /// Three rising notes.
    Triplet,
    /// A slow rising-and-falling wail, for the ones you must not miss.
    Warble,
    /// Three short hard beeps — the most electronic of the set.
    Digital,
}

impl AlertSound {
    pub const ALL: [AlertSound; 5] = [
        AlertSound::Ding,
        AlertSound::TwoTone,
        AlertSound::Triplet,
        AlertSound::Warble,
        AlertSound::Digital,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AlertSound::Ding => "Ding",
            AlertSound::TwoTone => "Two-tone",
            AlertSound::Triplet => "Triplet",
            AlertSound::Warble => "Warble",
            AlertSound::Digital => "Digital",
        }
    }
}

/// What one kind of alert is answered with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AlertRule {
    /// Whether this event sounds at all.
    pub enabled: bool,
    /// The sound it plays.
    pub sound: AlertSound,
}

impl Default for AlertRule {
    fn default() -> Self {
        AlertRule { enabled: true, sound: AlertSound::Ding }
    }
}

impl AlertRule {
    /// A rule that never fires, for the events that would be noise if on by
    /// default — a directed CQ on a busy band repeats every slot.
    fn off() -> Self {
        AlertRule { enabled: false, sound: AlertSound::default() }
    }
}

/// The events that can sound, each with its own rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AlertEvents {
    /// A station calling us — a decode addressed to our callsign.
    pub called: AlertRule,
    /// A directed CQ we could answer. Off by default: a busy evening on twenty
    /// metres is a hundred of these a minute.
    pub cq: AlertRule,
    /// A decode of a DXCC entity never worked, on any band.
    pub new_dxcc: AlertRule,
    /// A decoded station whose entity is worked, but not from this band.
    pub new_dxcc_band: AlertRule,
    /// A decode of a grid square never worked.
    pub new_grid: AlertRule,
}

impl Default for AlertEvents {
    fn default() -> Self {
        AlertEvents {
            called: AlertRule::default(),
            cq: AlertRule::off(),
            new_dxcc: AlertRule::default(),
            new_dxcc_band: AlertRule::default(),
            new_grid: AlertRule::default(),
        }
    }
}

/// Everything the alerts are configured by.
///
/// **Field order matters.** `sdroxide-config` writes this as a TOML table and
/// serde emits fields in declaration order, so every scalar must precede every
/// sub-table — a table declared above a scalar swallows it on the next write.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AlertSettings {
    // ── scalars ─────────────────────────────────────────────────────────
    /// Master switch. Off until the operator asks for it: a radio that starts
    /// beeping on first run is a bug, not a feature.
    pub enabled: bool,
    /// Alert volume, independent of the radio's AF gain.
    pub volume: f32,
    /// Sound device for alerts. `None` is the system default. Alerts get their
    /// own output stream, so this can be a different device from the radio's —
    /// the band in the headphones, the alarm in the room.
    pub device: Option<String>,

    // ── sub-tables: everything below emits as a TOML table ───────────────
    pub events: AlertEvents,
}

impl Default for AlertSettings {
    fn default() -> Self {
        AlertSettings { enabled: false, volume: 0.7, device: None, events: AlertEvents::default() }
    }
}

impl AlertSettings {
    /// Volume clamped to a sane range.
    pub fn volume(&self) -> f32 {
        self.volume.clamp(0.0, 1.0)
    }
}

/// The event a decode matched. One per decode — the single most useful sound,
/// like the one-badge display in the decode list — rather than a chorus when a
/// station that calls us also happens to be a new entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlertEvent {
    Called,
    Cq,
    NewDxcc,
    NewDxccBand,
    NewGrid,
}

impl AlertEvent {
    pub const ALL: [AlertEvent; 5] = [
        AlertEvent::Called,
        AlertEvent::Cq,
        AlertEvent::NewDxcc,
        AlertEvent::NewDxccBand,
        AlertEvent::NewGrid,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AlertEvent::Called => "A station is calling me",
            AlertEvent::Cq => "A CQ I could answer",
            AlertEvent::NewDxcc => "New DXCC entity",
            AlertEvent::NewDxccBand => "New entity on this band",
            AlertEvent::NewGrid => "New grid square",
        }
    }

    /// A stable key for per-station cooldowns.
    pub fn as_str(self) -> &'static str {
        match self {
            AlertEvent::Called => "called",
            AlertEvent::Cq => "cq",
            AlertEvent::NewDxcc => "new_dxcc",
            AlertEvent::NewDxccBand => "new_dxcc_band",
            AlertEvent::NewGrid => "new_grid",
        }
    }

    /// The rule this event is configured by.
    pub fn rule(self, events: &AlertEvents) -> &AlertRule {
        match self {
            AlertEvent::Called => &events.called,
            AlertEvent::Cq => &events.cq,
            AlertEvent::NewDxcc => &events.new_dxcc,
            AlertEvent::NewDxccBand => &events.new_dxcc_band,
            AlertEvent::NewGrid => &events.new_grid,
        }
    }

    /// The rule this event is configured by, editable.
    pub fn rule_mut(self, events: &mut AlertEvents) -> &mut AlertRule {
        match self {
            AlertEvent::Called => &mut events.called,
            AlertEvent::Cq => &mut events.cq,
            AlertEvent::NewDxcc => &mut events.new_dxcc,
            AlertEvent::NewDxccBand => &mut events.new_dxcc_band,
            AlertEvent::NewGrid => &mut events.new_grid,
        }
    }

    /// How long a station is quiet after alarming for this event, in seconds.
    ///
    /// A decode repeats every slot for as long as it is sent, and novelty does
    /// not change until the contact is logged — without a cooldown a new
    /// entity calling CQ all evening would beep the whole evening.
    pub fn cooldown_s(self) -> u64 {
        match self {
            AlertEvent::Called => 60,
            AlertEvent::Cq => 45,
            AlertEvent::NewDxcc => 300,
            AlertEvent::NewDxccBand => 300,
            AlertEvent::NewGrid => 120,
        }
    }

    /// The alert a decode deserves, or `None` when it triggers none.
    ///
    /// The same tests the decode list makes its badges from: addressed-to-us
    /// (`to`, or `rr73_to` in Hound mode), directed-CQ-for-us, and the
    /// new/dupe judgement against the log. Novelty is read for *every* decode,
    /// not only our own traffic — a new entity appearing anywhere on the band
    /// is exactly what the operator asked to hear about.
    pub fn for_decode(d: &Decode, my_call: &str, my_grid: &str, novelty: Novelty) -> Option<Self> {
        let addressed = !my_call.is_empty()
            && (d.to.as_deref().is_some_and(|t| t.eq_ignore_ascii_case(my_call))
                || d.rr73_to.as_deref().is_some_and(|t| t.eq_ignore_ascii_case(my_call)));
        if addressed {
            return Some(AlertEvent::Called);
        }
        if novelty.new_dxcc {
            return Some(AlertEvent::NewDxcc);
        }
        if novelty.new_dxcc_band {
            return Some(AlertEvent::NewDxccBand);
        }
        if novelty.new_grid {
            return Some(AlertEvent::NewGrid);
        }
        if d.is_cq && cq_is_for_us(d, my_call, my_grid) {
            return Some(AlertEvent::Cq);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(to: Option<&str>, from: Option<&str>, cq: bool, rr73: Option<&str>) -> Decode {
        Decode {
            slot_utc: 0,
            snr_db: 0,
            dt: 0.0,
            audio_hz: 500.0,
            message: String::new(),
            to: to.map(str::to_string),
            from: from.map(str::to_string),
            grid: None,
            is_cq: cq,
            cq_to: None,
            free_text: false,
            rr73_to: rr73.map(str::to_string),
        }
    }

    /// Addressed-to-us wins even when the station is also new to the log: the
    /// actionable alert is that they are calling, whatever the awards say.
    #[test]
    fn a_station_calling_us_outranks_its_novelty() {
        let d = dec(Some("OE3ABC"), Some("26AT101"), false, None);
        let n = Novelty { new_dxcc: true, ..Novelty::default() };
        assert_eq!(AlertEvent::for_decode(&d, "OE3ABC", "", n), Some(AlertEvent::Called));
    }

    /// Novelty is judged for every decode, not only the ones addressed to us.
    #[test]
    fn a_new_entity_anywhere_on_the_band_rings() {
        // Two other stations in a QSO, no attention to us at all.
        let d = dec(Some("K1ABC"), Some("26AT101"), false, None);
        let n = Novelty { new_dxcc: true, ..Novelty::default() };
        assert_eq!(AlertEvent::for_decode(&d, "OE3ABC", "", n), Some(AlertEvent::NewDxcc));
    }

    #[test]
    fn a_directed_cq_is_the_lowest_priority() {
        let d = dec(None, Some("OE3ABC"), true, None);
        let d = Decode { message: "CQ EU OE3ABC JO63".to_string(), ..d };
        let n = Novelty::default();
        assert_eq!(AlertEvent::for_decode(&d, "DL1ABC", "JO63", n), Some(AlertEvent::Cq));
        // Same CQ, but the station is a new entity — novelty wins.
        let n = Novelty { new_dxcc: true, ..Novelty::default() };
        assert_eq!(AlertEvent::for_decode(&d, "DL1ABC", "JO63", n), Some(AlertEvent::NewDxcc));
    }

    /// A plain CQ is answerable by anyone — there is no region to exclude us —
    /// so it has exactly the same standing as a region one. What that alerts
    /// *worth like on a busy band* is handled by the rule being off by default
    /// and the per-station cooldown, not by the matcher.
    #[test]
    fn a_plain_cq_is_as_answerable_as_a_region_broadcast() {
        let d = dec(None, Some("OE3ABC"), true, None);
        let d = Decode { message: "CQ OE3ABC JO63".to_string(), ..d };
        assert_eq!(
            AlertEvent::for_decode(&d, "DL1ABC", "JO63", Novelty::default()),
            Some(AlertEvent::Cq)
        );
    }

    #[test]
    fn a_region_cq_aimed_elsewhere_is_silent() {
        let mut d = dec(None, Some("JA1ABC"), true, None);
        let d = Decode { cq_to: Some("JA".to_string()), ..d };
        // "CQ JA" is aimed at Japan, and this operator is in JO63.
        assert_eq!(AlertEvent::for_decode(&d, "DL1ABC", "JO63", Novelty::default()), None);
    }

    #[test]
    fn hound_completion_is_still_a_call() {
        // Hound mode: the Fox names us only through rr73_to.
        let d = dec(None, Some("VK9XX"), false, Some("DL1ABC"));
        assert_eq!(
            AlertEvent::for_decode(&d, "DL1ABC", "", Novelty::default()),
            Some(AlertEvent::Called)
        );
    }

    #[test]
    fn defaults_keep_the_radio_silent_until_asked() {
        let s = AlertSettings::default();
        assert!(!s.enabled);
        assert!(!s.events.cq.enabled, "directed CQs would beep a hundred times a minute");
        assert!(s.events.called.enabled, "a station calling us is the headline alert");
        assert!(s.events.new_dxcc.enabled);
    }

    #[test]
    fn every_event_has_a_sane_cooldown() {
        for e in AlertEvent::ALL {
            assert!(e.cooldown_s() > 0);
            assert_eq!(AlertEvent::ALL.iter().filter(|x| **x == e).count(), 1);
        }
    }
}
