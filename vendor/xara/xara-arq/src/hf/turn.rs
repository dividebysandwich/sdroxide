use crate::hf::frames::{payload_len, text_cap};
use xara_phy::hf::level::{Bandwidth, Record, start_level_max};

pub const HOLD_EMPTY_MS: i64 = 3300;
pub const HOLD_QUEUED_MS: i64 = 4500;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TurnKind {
    #[default]
    Partial,
    Prompt,
    Overflow,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HintOverride {
    #[default]
    Off,
    Max,
    TableMinimum,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurnStart {
    Hold,
    Idle,
    Data(HintOverride),
}

#[derive(Clone, Copy, Debug)]
pub struct TurnState {
    pub answerer_hold: bool,
    pub chat: bool,
    pub queued: usize,
    pub since_header_ms: i64,
    pub kind: TurnKind,
    pub offered: Record,
    pub next_tx: Record,
    pub text: bool,
    pub last_held: usize,
}

pub fn turn_start(bw: Bandwidth, t: &TurnState) -> (TurnStart, usize) {
    let held = t.last_held;
    if t.answerer_hold && !t.chat {
        let s = match t.queued {
            0 if t.since_header_ms < HOLD_EMPTY_MS => TurnStart::Hold,
            0 => TurnStart::Idle,
            _ if t.since_header_ms < HOLD_QUEUED_MS && t.kind == TurnKind::Partial => TurnStart::Hold,
            _ if t.kind == TurnKind::Prompt => TurnStart::Data(HintOverride::Off),
            _ => TurnStart::Data(HintOverride::TableMinimum),
        };
        return (s, held);
    }
    if t.queued == 0 {
        return (TurnStart::Idle, held);
    }
    let cap = text_cap(start_level_max(bw), t.text);
    let n = t.offered.min(bw.top_level());
    if t.queued <= payload_len(bw, n) && t.offered > cap && t.queued != held {
        return (TurnStart::Hold, t.queued);
    }
    (TurnStart::Data(if t.next_tx > cap { HintOverride::Max } else { HintOverride::Off }), held)
}
