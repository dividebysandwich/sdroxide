//! The JS8 engine seam: slot timing, decode dispatch, and the transmit queue.
//!
//! Structurally this is [`crate::DigiController`] with two things taken away
//! and one added.
//!
//! **No even/odd discipline.** FT8 stations alternate, and the FT8 controller
//! spends real effort staying opposite the station it is working. JS8 stations
//! transmit when they have something to say, so there is no period to be on the
//! wrong side of.
//!
//! **No QSO machine.** FT8's `QsoMachine` sequences a contest exchange through
//! Tx1–Tx6. JS8 carries a conversation: what to send next is whatever the
//! operator typed. The automatic traffic is limited to:
//! * replies to direct questions ([`crate::js8::directed`]);
//! * heartbeats;
//! * group auto-responses the operator wrote and switched on
//!   ([`crate::js8::autorule`]).
//!
//! **A frame queue instead.** A message longer than one frame occupies
//! consecutive slots, so transmit is a queue drained one frame per slot. Every
//! message joins the back of it rather than jumping it, which stops the station
//! keying twice in a slot. Each frame records which message it belongs to and
//! who queued it. That lets an operator's SEND replace their own unsent
//! traffic without dropping an automatic answer or cutting off a message
//! already on the air.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::time::SystemTime;

use sdroxide_dsp::MonoResampler;
use sdroxide_types::{
    DigiConfig, DigiStatus, HB_BAND_HI_HZ, HB_BAND_LO_HZ, HB_SLOT_HZ, JS8_AUTO_RULES_MAX,
    Js8AutoPending, Js8Heard, Js8Msg, Js8Speed, Js8Status, Mode, QsoStep,
};

use crate::DigiEngine;
use crate::clock::ClockMonitor;
use crate::controller::{BurstPlayer, DigiAction};
use crate::js8::assembler::Js8Assembler;
use crate::js8::autorule;
use crate::js8::decode::{Js8Decode, Js8Depth, decode_slot_for};
use crate::js8::directed::{
    Js8Intent, Js8Reply, LastTx, ReplyPolicy, StationInfo, auto_reply, intent, repeat_reply,
};
use crate::js8::frame::{self, Compound, DIRECTED_CMDS, Directed, Js8Flags};
use crate::js8::message::Js8Payload;
use crate::js8::modem::{self, DECODE_RATE};
use crate::js8::varicode;
use crate::params::DigiParams;
use crate::scheduler::SlotScheduler;

const OUT_RATE: f64 = 48_000.0;

/// Decodes kept for the activity list.
const ACTIVITY_CAP: usize = 200;
/// Reassembled messages kept for the conversation view.
const MSG_CAP: usize = 200;
/// Stations kept in the heard list.
const HEARD_CAP: usize = 50;

/// Least time between two acknowledgements of the same station's heartbeats.
///
/// Upstream's guard against being spammed by `@ALLCALL` traffic is fifteen
/// minutes (`mainwindow.cpp`, `m_txAllcallCommandCache`), and a heartbeat is an
/// `@ALLCALL`. A station beaconing every ten minutes therefore gets an answer
/// from us roughly every other beacon rather than every one.
const HB_ACK_COOLDOWN_S: i64 = 15 * 60;

/// Stations remembered for the acknowledgement cooldown.
const HB_ACK_CAP: usize = 200;

/// How long a decode keeps its slot marked busy when choosing where to beacon.
///
/// Upstream's figure (`mainwindow.cpp`, `isFreqOffsetFree`: activity older than
/// thirty seconds does not count). Two JS8 slots at Normal, so a station that
/// transmitted in either of the last two slots still holds its frequency —
/// which is the property that matters, and why
/// [`Js8Controller::activity_window_s`] stretches it at Slow rather than using
/// this number flat.
const ACTIVITY_WINDOW_S: i64 = 30;

/// Decodes remembered for that judgement. A busy slot decodes a couple of dozen
/// signals; this is well clear of any real band and stops a pathological one
/// growing the list without bound.
const ACTIVITY_CAP_DECODES: usize = 500;

/// Most group auto-responses sent in any hour, across all rules and
/// stations.
///
/// The last line of defence: per-station cooldowns and repeat limits already
/// bound each rule. This bounds the whole station if a busy net, or two
/// stations' rules answering each other, gets past them. Sends due beyond it
/// wait rather than vanish, and the panel shows them waiting.
const AUTO_RULE_MAX_PER_HOUR: usize = 20;

/// (rule, station) pairs remembered for the auto-response cooldown.
const AUTO_COOLDOWN_CAP: usize = 200;

/// A group auto-response that fired and still has sends to make.
#[derive(Debug, Clone)]
struct AutoSend {
    /// Index into `js8_auto_rules`, for the cooldown and de-duplication.
    rule: usize,
    label: String,
    /// The station whose message triggered it.
    trigger: String,
    /// The reply, addressee included, exactly as it will be sent.
    text: String,
    sent: u8,
    total: u8,
    gap_s: i64,
    next_unix: i64,
    /// The message id of the last send, while its frames are still queued.
    /// The next repeat waits until it has gone, so repeats cannot pile up
    /// behind a long queue and then go out back to back.
    queued: Option<u32>,
}

/// Where a queued frame goes out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TxBand {
    /// The operator's working frequency.
    Working,
    /// A free slot in the heartbeat sub-band. Resolved when the frame is
    /// actually sent rather than when it is queued, so the choice is made
    /// against the band as it is at that moment — a beacon can sit behind a
    /// long message for minutes, by which time a slot picked at queue time
    /// would be somebody else's.
    Beacon,
}

/// Who put a frame in the queue, which decides what an operator's SEND may
/// replace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TxOrigin {
    /// Typed, or a panel button. A new SEND replaces whatever the operator
    /// queued before it and has not gone out yet.
    Operator,
    /// An answer to a query, a heartbeat acknowledgement, or a scheduled
    /// beacon. It survives a SEND. Before, typing a message silently dropped
    /// the answer to a question someone had just asked.
    Auto,
    /// A group auto-response. It belongs to the operator's own traffic, so a
    /// SEND replaces it like anything else they queued.
    Rule,
}

/// A frame waiting for its slot, and where it is going.
#[derive(Debug, Clone, Copy)]
struct TxFrame {
    payload: Js8Payload,
    band: TxBand,
    origin: TxOrigin,
    /// Which message the frame belongs to. The frames of one message must
    /// stay together: the receiver joins them by frequency and FIRST/LAST
    /// flags, so another message squeezed in between would be spliced into
    /// the middle of it.
    msg: u32,
}

/// Work handed to the decode thread.
struct DecodeJob {
    audio: Vec<i16>,
    slot_utc: i64,
    speed: Js8Speed,
}

/// Jobs allowed to be waiting for the decode worker before a new slot is
/// dropped instead of queued.
///
/// One speed cannot outrun the worker — a slot's decode takes far less than a
/// slot — but four of them on a slow machine could, and a queue that grows
/// without bound turns a busy minute into a receiver permanently minutes
/// behind the band. Two per speed: enough that a slot arriving while the
/// previous one is still being decoded waits rather than being thrown away,
/// and few enough that the backlog can never become the problem.
const MAX_JOBS_PER_SPEED: usize = 2;

/// One speed's own slot clock and the audio accumulating for it.
///
/// The four JS8 speeds are not four settings of one receiver: they are four
/// waveforms on four different slot clocks, 30 / 15 / 10 / 6 seconds long, and
/// a slot of one is not a slot of any other. So decoding more than one at a
/// time is not a matter of running the decoder twice over the same audio —
/// each speed needs its own boundary, its own buffer, and its own submission
/// (issue #358).
struct SpeedSlot {
    speed: Js8Speed,
    scheduler: SlotScheduler,
    /// The audio since this speed's last boundary, at [`DECODE_RATE`].
    buf: Vec<i16>,
    /// Which slot the buffer is filling. `i64::MIN` before the first boundary,
    /// which is the one slot that is thrown away — it started mid-slot and is
    /// not a whole transmission of anything.
    last_idx: i64,
}

impl SpeedSlot {
    fn new(speed: Js8Speed) -> SpeedSlot {
        let params = DigiParams::for_js8(speed);
        SpeedSlot {
            speed,
            scheduler: SlotScheduler::new(params.slot_s, params.tx_offset_s),
            buf: Vec::new(),
            last_idx: i64::MIN,
        }
    }

    fn slot_samples(&self) -> usize {
        (self.speed.slot_s() * DECODE_RATE) as usize
    }

    /// Take the resampled tap. A slot and a quarter is plenty of slack for a
    /// late boundary; more just costs memory.
    fn push(&mut self, samples: &[f32]) {
        self.buf.extend(samples.iter().map(|&s| (s.clamp(-1.0, 1.0) * 28_000.0) as i16));
        let cap = self.slot_samples() * 5 / 4;
        if self.buf.len() > cap {
            let drop = self.buf.len() - cap;
            self.buf.drain(..drop);
        }
    }

    /// The finished slot, when `now` has crossed this speed's boundary.
    ///
    /// Half a slot is the floor: a buffer shorter than that is what a speed
    /// switched on mid-slot, or a receive path that was paused, leaves behind,
    /// and there is no transmission of this speed inside it to find.
    fn take_finished(&mut self, now: SystemTime) -> Option<DecodeJob> {
        let idx = self.scheduler.slot_index(now);
        if idx == self.last_idx {
            return None;
        }
        let job =
            (self.last_idx != i64::MIN && self.buf.len() >= self.slot_samples() / 2).then(|| {
                DecodeJob {
                    audio: std::mem::take(&mut self.buf),
                    slot_utc: self.scheduler.slot_start_unix(self.last_idx) as i64,
                    speed: self.speed,
                }
            });
        self.buf.clear();
        self.last_idx = idx;
        job
    }
}

/// The speeds a station is listening on: all four when the operator asked for
/// multi-speed decoding, otherwise only the one being worked (issue #358).
fn listening_speeds(cfg: &DigiConfig) -> Vec<SpeedSlot> {
    if cfg.js8_multi_decode {
        Js8Speed::ALL.into_iter().map(SpeedSlot::new).collect()
    } else {
        vec![SpeedSlot::new(cfg.js8_speed)]
    }
}

pub struct Js8Controller {
    cfg: DigiConfig,
    speed: Js8Speed,
    params: DigiParams,
    scheduler: SlotScheduler,

    resampler: Option<MonoResampler>,
    /// One per speed being listened for — one entry normally, four with
    /// multi-speed decoding on. See [`SpeedSlot`].
    rx_slots: Vec<SpeedSlot>,
    tap_scratch: Vec<f32>,
    /// Slots handed to the worker and not yet decoded, so four speeds cannot
    /// queue up faster than one thread can get through them. See
    /// [`MAX_JOBS_PER_SPEED`].
    jobs_queued: Arc<AtomicUsize>,

    dial_hz: f64,
    audio_hz: f32,

    job_tx: Sender<DecodeJob>,
    res_rx: Receiver<(i64, Vec<Js8Decode>)>,

    assembler: Js8Assembler,
    heard: Vec<Js8Heard>,
    messages: Vec<Js8Msg>,

    /// Reassemble only frames whose burst finished. Keeping this separate from
    /// RX prevents our own text from generating replies or heard-station spots.
    tx_assembler: Js8Assembler,
    tx_inflight: Option<(Js8Decode, i64)>,

    /// Frames still to go out, one per slot.
    tx_frames: VecDeque<TxFrame>,
    tx_total: u8,
    /// Id for the next queued message. See [`TxFrame::msg`].
    next_msg: u32,
    /// The message whose first frame has gone out, if any. Its remaining
    /// frames may not be replaced.
    on_air_msg: Option<u32>,
    tx_fired_slot: i64,
    burst: Option<BurstPlayer>,
    keyed: bool,

    /// Unix time the next automatic heartbeat is due, or 0 when none is
    /// scheduled (heartbeats off, no callsign, or a speed that may not beacon).
    next_hb_unix: i64,
    /// How many automatic heartbeats this session has sent, which is what makes
    /// the jitter below differ from one to the next.
    hb_count: u32,
    /// Seconds until the next heartbeat, as the panel shows it. Kept rather
    /// than computed in `status()`, which has no clock.
    hb_countdown_s: Option<u32>,
    /// Group auto-responses that fired and have sends to go.
    auto_pending: Vec<AutoSend>,
    /// When each (rule, station) last triggered a response, for the cooldown.
    auto_cooldown: HashMap<(usize, String), i64>,
    /// When each auto-response send of the last hour went out, for
    /// [`AUTO_RULE_MAX_PER_HOUR`].
    auto_sent_at: VecDeque<i64>,
    /// The clock as of the last poll, for the countdowns in `status()` (which
    /// has no clock of its own).
    now_unix: i64,
    /// The second the auto-response countdown was last published.
    auto_status_s: i64,
    /// What we last sent, to whom, and when. This is what ` AGN?` repeats.
    last_tx: Option<LastTx>,
    /// When each station's heartbeat was last acknowledged, for the cooldown.
    hb_acked: HashMap<String, i64>,
    /// Where the last beacon went out, for the panel.
    hb_hz: Option<f32>,
    /// Recently decoded signals as `(audio Hz, slot)`, which is all the band
    /// occupancy choosing a beacon slot needs.
    activity: Vec<(f32, i64)>,
    clock: ClockMonitor,
    status_dirty: bool,
}

impl Js8Controller {
    pub fn new(cfg: DigiConfig, tap_rate: f64) -> Self {
        let speed = cfg.js8_speed;
        let params = DigiParams::for_js8(speed);
        let (job_tx, job_rx) = std::sync::mpsc::channel::<DecodeJob>();
        let (res_tx, res_rx) = std::sync::mpsc::channel::<(i64, Vec<Js8Decode>)>();
        let jobs_queued = Arc::new(AtomicUsize::new(0));
        let worker_jobs = Arc::clone(&jobs_queued);

        // Belief propagation must not run on the audio thread; a Slow slot is
        // 28 s of spectrogram and even Turbo is far more than one 10 ms block.
        std::thread::Builder::new()
            .name("sdroxide-js8-decode".into())
            .spawn(move || {
                while let Ok(job) = job_rx.recv() {
                    // Ordered statistics runs only where belief propagation
                    // gave up, and measurably costs nothing: a busy Normal
                    // slot decodes in ~20 ms either way, because the sync gate
                    // turns most candidates away long before the FEC. There is
                    // no reason to make an operator opt into it.
                    let decodes = decode_slot_for(job.speed, &job.audio, Js8Depth::BpOsd);
                    // Counted down after the decode, not after the result is
                    // taken: what the cap is protecting is this thread's
                    // backlog, and the slot stops being part of it here.
                    worker_jobs.fetch_sub(1, Ordering::Relaxed);
                    if res_tx.send((job.slot_utc, decodes)).is_err() {
                        break;
                    }
                }
            })
            .expect("spawn js8 decode worker");

        let mut assembler = Js8Assembler::new(&Self::call_of(&cfg));
        assembler.set_timeout(i64::from(cfg.js8_assembly_timeout_s));
        assembler.set_my_groups(cfg.js8_groups.clone());

        Js8Controller {
            speed,
            params,
            scheduler: SlotScheduler::new(params.slot_s, params.tx_offset_s),
            resampler: MonoResampler::new(tap_rate, DECODE_RATE),
            rx_slots: listening_speeds(&cfg),
            tap_scratch: Vec::new(),
            jobs_queued,
            dial_hz: 0.0,
            audio_hz: 1500.0,
            job_tx,
            res_rx,
            assembler,
            heard: Vec::new(),
            messages: Vec::new(),
            tx_assembler: Js8Assembler::new(""),
            tx_inflight: None,
            tx_frames: VecDeque::new(),
            tx_total: 0,
            next_msg: 0,
            on_air_msg: None,
            tx_fired_slot: i64::MIN,
            burst: None,
            keyed: false,
            next_hb_unix: 0,
            hb_count: 0,
            hb_countdown_s: None,
            hb_acked: HashMap::new(),
            auto_pending: Vec::new(),
            auto_cooldown: HashMap::new(),
            auto_sent_at: VecDeque::new(),
            now_unix: 0,
            auto_status_s: 0,
            last_tx: None,
            hb_hz: None,
            activity: Vec::new(),
            clock: ClockMonitor::default(),
            status_dirty: true,
            cfg,
        }
    }

    fn call_of(cfg: &DigiConfig) -> String {
        if cfg.js8_call.is_empty() {
            cfg.my_call.to_uppercase()
        } else {
            cfg.js8_call.to_uppercase()
        }
    }

    fn my_call(&self) -> String {
        Self::call_of(&self.cfg)
    }

    fn station(&self) -> StationInfo {
        StationInfo {
            call: self.my_call(),
            grid: self.cfg.my_grid.to_uppercase(),
            status: self.cfg.js8_status.clone(),
            info: self.cfg.js8_info.clone(),
            hearing: self.heard.iter().take(4).map(|h| h.call.clone()).collect(),
        }
    }

    /// Queue one message's frames behind whatever is already waiting.
    ///
    /// An operator's message first removes any of the operator's own traffic
    /// that has not gone out: typing a new message means "send this
    /// instead". It never removes these:
    /// * answers and beacons the station queued by itself ([`TxOrigin::Auto`]);
    /// * the rest of a message already on the air. Cutting that off would
    ///   leave the receiver holding half a sentence.
    fn enqueue(&mut self, origin: TxOrigin, band: TxBand, frames: Vec<Js8Payload>) {
        if frames.is_empty() {
            return;
        }
        let idle = self.tx_frames.is_empty() && !self.keyed;
        if origin == TxOrigin::Operator {
            // The operator is at the keyboard and has something to say. A
            // canned reply still waiting to repeat is no longer what the
            // station should be doing.
            if !self.auto_pending.is_empty() {
                self.auto_pending.clear();
            }
            let live = self.on_air_msg;
            let before = self.tx_frames.len();
            self.tx_frames.retain(|f| f.origin == TxOrigin::Auto || Some(f.msg) == live);
            let gone = (before - self.tx_frames.len()).min(255) as u8;
            self.tx_total = self.tx_total.saturating_sub(gone);
        }
        let msg = self.next_msg;
        self.next_msg = self.next_msg.wrapping_add(1);
        let n = frames.len().min(255) as u8;
        self.tx_frames.extend(frames.into_iter().map(|payload| TxFrame {
            payload,
            band,
            origin,
            msg,
        }));
        // The panel's "frame n of m" counts the whole run of traffic, from
        // when the queue last emptied.
        self.tx_total = if idle { n } else { self.tx_total.saturating_add(n) };
        self.status_dirty = true;
    }

    fn remember_message(&mut self, msg: Js8Msg) {
        self.messages.push(msg);
        if self.messages.len() > MSG_CAP {
            self.messages.remove(0);
        }
    }

    fn outgoing_message(&self, mut msg: Js8Msg) -> Js8Msg {
        // Undirected text has no sender on the wire; we know who sent it.
        if msg.from.is_empty() {
            msg.from = self.my_call();
        }
        msg.to_me = false;
        msg
    }

    fn finish_partial_tx(&mut self) {
        for msg in self.tx_assembler.in_progress() {
            self.remember_message(self.outgoing_message(msg));
        }
        self.tx_assembler = Js8Assembler::new("");
    }

    fn message_history(&self) -> Vec<Js8Msg> {
        let mut messages = self.messages.clone();
        messages
            .extend(self.tx_assembler.in_progress().into_iter().map(|m| self.outgoing_message(m)));
        if messages.len() > MSG_CAP {
            messages.drain(..messages.len() - MSG_CAP);
        }
        messages
    }

    /// Queue an operator's message on the working frequency.
    fn queue_frames(&mut self, frames: Vec<Js8Payload>) {
        self.enqueue(TxOrigin::Operator, TxBand::Working, frames);
    }

    /// The heartbeat someone asked for by typing it.
    ///
    /// A heartbeat is a *compound* frame, not a directed command — there is no
    /// ` HB` in the command table — so [`Js8Controller::directed_head`] cannot
    /// produce one and the panel's HB button would otherwise put the literal
    /// text "@ALLCALL HB" on the air.
    fn heartbeat_for(&self, text: &str) -> Option<Js8Payload> {
        let t = text.trim().to_ascii_uppercase();
        let body = t.strip_prefix("@ALLCALL").map(str::trim).unwrap_or(t.as_str());
        if !matches!(body, "HB" | "HEARTBEAT") {
            return None;
        }
        let call = self.my_call();
        if call.is_empty() {
            return None;
        }
        let grid = self.cfg.my_grid.to_ascii_uppercase();
        Compound::heartbeat(&call, (!grid.is_empty()).then_some(grid.as_str()))
            .pack()
            .map(Self::single)
    }

    /// The CQ someone asked for by typing it.
    ///
    /// Like the heartbeat, a CQ is a *compound* frame — callsign and grid — and
    /// not a directed command, so once the panel's CQ chip became a way to fill
    /// the compose box rather than a command of its own, a typed `"CQ"` would
    /// otherwise put the two literal letters on the air (issue #472).
    fn cq_for(&self, text: &str) -> Option<Js8Payload> {
        let t = text.trim().to_ascii_uppercase();
        let body = t.strip_prefix("@ALLCALL").map(str::trim).unwrap_or(t.as_str());
        if body != "CQ" {
            return None;
        }
        let call = self.my_call();
        if call.is_empty() {
            return None;
        }
        let grid = self.cfg.my_grid.to_ascii_uppercase();
        Compound::cq(&call, (!grid.is_empty()).then_some(grid.as_str()), 0).pack().map(Self::single)
    }

    /// The directed frame a message opens with, and how much text it ate.
    ///
    /// Anything addressed to a callsign gets one: `"KN4CRD SNR?"` because JS8
    /// has a frame for that question, and `"KN4CRD HELLO"` because the frame is
    /// also what tells KN4CRD the words are meant for *them* — free text is
    /// command 31. A message with no addressee has nothing to direct and gets
    /// none, which is most of what an operator types.
    ///
    /// The frame swallows the command and, for the two report commands, its
    /// number; whatever follows is the caller's to send as text. That is
    /// upstream's split too (`varicode.cpp`, `packDirectedMessage` returns the
    /// matched length and `buildMessageFrames` passes the remainder on), and it
    /// is why the number pattern there is guarded by a lookbehind on `SNR`: no
    /// other command has anywhere to put one.
    fn directed_head(&self, up: &str) -> Option<(Directed, usize)> {
        let (to, rest) = up.split_once(char::is_whitespace)?;
        let from = self.my_call();
        // Upstream refuses a message addressed to the sender: that is not a
        // directed command, it is someone typing their own callsign.
        if !addressable(to) || to.eq_ignore_ascii_case(&from) {
            return None;
        }
        let trimmed = rest.trim_start();
        let cmd_at = up.len() - trimmed.len();
        // Longest match wins, or " HW CPY?" would be read as no command at all
        // and " QUERY MSGS" as " QUERY".
        let named = DIRECTED_CMDS
            .iter()
            .map(|(t, c)| (t.trim_start(), c))
            .filter(|(t, c)| !t.is_empty() && !CHECKSUMMED_CMDS.contains(c))
            .filter(|(t, _)| {
                trimmed == *t || trimmed.strip_prefix(t).is_some_and(|r| r.starts_with(' '))
            })
            .max_by_key(|(t, _)| t.len());

        // No named command: this is text addressed to a station, which JS8 has
        // its own command code for. Worth sending as one rather than as bare
        // text — it is what tells the station at the other end the words are
        // meant for *them*, and it is what upstream sends (`varicode.cpp`,
        // `optional_cmd_pattern` ends in `[?> ]`, so a plain space matches).
        let (cmd_text, code) = named.unwrap_or(("", &FREE_TEXT_CMD));
        let mut used = cmd_at + cmd_text.len();

        // Only the report commands have a number field. For everything else the
        // digits are text, and swallowing them here would lose them.
        if frame::SNR_CMDS.contains(code) {
            let tail = &up[used..];
            let spaced = tail.strip_prefix(' ').unwrap_or(tail);
            if let Some((num, len)) = leading_num(spaced) {
                used = up.len() - spaced.len() + len;
                return Some((self.directed_to(to, *code, Some(num)), used));
            }
        }
        Some((self.directed_to(to, *code, None), used))
    }

    fn directed_to(&self, to: &str, cmd: i8, num: Option<i32>) -> Directed {
        let from = self.my_call();
        Directed {
            portable_from: from.ends_with("/P"),
            portable_to: to.ends_with("/P"),
            from,
            to: to.to_string(),
            cmd,
            num,
        }
    }

    /// The frames a message becomes: a directed command when it opens with one,
    /// then as many data frames as the rest of the text needs.
    ///
    /// A grid, a status line, a list of stations heard — none of them fit in a
    /// directed frame, which has room for a command and a six-bit number. They
    /// ride behind it as free text, and the receiving assembler stitches the
    /// two halves back into one message.
    fn frames_for_text(&self, text: &str) -> Vec<Js8Payload> {
        let up = text.trim().to_ascii_uppercase();
        if let Some((directed, used)) = self.directed_head(&up)
            && let Some(mut out) = self.head_frames(&directed)
        {
            // The space between the command and its text belongs to neither:
            // upstream strips it (`buildMessageFrames` lstrips the remainder of
            // every buffered command, and every entry in the table contains a
            // space, so that is all of them). Sending it costs a character of
            // an already small frame.
            out.extend(self.data_frames(up[used..].trim_start()));
            return Self::flag_sequence(out);
        }
        Self::flag_sequence(self.data_frames(&up))
    }

    /// The frame or frames that open a directed message.
    ///
    /// Usually this is one directed frame. When either end is a callsign the
    /// directed frame's 28 bits cannot hold, it is two compound frames:
    /// * a custom group like `@EMCOMM`;
    /// * a compound call like `W1AW/4`, or ours being `KN4CRD/QRP`.
    ///
    /// The first frame names us and our grid. The second names the addressee
    /// and carries the command (`buildMessageFrames`, `varicode.cpp:2120`,
    /// cases 1-3). Without this, a message to a custom group went out as bare
    /// text with no sender and no addressee.
    fn head_frames(&self, d: &Directed) -> Option<Vec<Js8Payload>> {
        if !varicode::needs_compound(&d.from) && !varicode::needs_compound(&d.to) {
            return d.pack().map(|f| vec![f]);
        }
        let grid = self.cfg.my_grid.to_ascii_uppercase();
        let grid = (grid.len() >= 4).then_some(grid.as_str());
        let ident = Compound::ident(&d.from, grid).pack()?;
        let to = Compound::directed(&d.to, d.cmd, d.num).pack()?;
        Some(vec![ident, to])
    }

    /// The frames an auto-reply becomes.
    fn frames_for_reply(&self, reply: Js8Reply) -> Vec<Js8Payload> {
        let Some(mut out) = self.head_frames(&reply.directed) else { return Vec::new() };
        if let Some(body) = reply.body.as_deref().map(str::trim).filter(|b| !b.is_empty()) {
            out.extend(self.data_frames(&body.to_ascii_uppercase()));
        }
        Self::flag_sequence(out)
    }

    /// Split text across as many data frames as it needs, unflagged.
    fn data_frames(&self, text: &str) -> Vec<Js8Payload> {
        let mut out = Vec::new();
        let mut rest = text.to_ascii_uppercase();
        while !rest.trim().is_empty() && out.len() < 32 {
            let Some((payload, used)) = frame::pack_data(&rest) else {
                break;
            };
            if used == 0 {
                break;
            }
            out.push(payload);
            rest = rest[used.min(rest.len())..].to_string();
        }
        out
    }

    /// Stamp FIRST on the opening frame and LAST on the closing one.
    fn flag_sequence(mut frames: Vec<Js8Payload>) -> Vec<Js8Payload> {
        let last = frames.len().saturating_sub(1);
        for (i, f) in frames.iter_mut().enumerate() {
            let mut flags = 0u8;
            if i == 0 {
                flags |= Js8Flags::FIRST;
            }
            if i == last {
                flags |= Js8Flags::LAST;
            }
            f.frame_type = flags;
        }
        frames
    }

    /// A single self-contained frame — a directed command or a heartbeat.
    fn single(mut payload: Js8Payload) -> Js8Payload {
        payload.frame_type = Js8Flags::FIRST | Js8Flags::LAST;
        payload
    }

    // ── Heartbeats ──────────────────────────────────────────────────────────

    /// Fire the first group auto-response `msg` triggers, if any.
    ///
    /// Only the first matching rule counts. One message gets at most one
    /// canned reply, however many rules the operator wrote that could apply.
    /// A (rule, station) pair already running, or still inside its cooldown,
    /// does nothing.
    fn trigger_auto_rule(&mut self, msg: &Js8Msg, unix_now: i64) {
        if !self.cfg.js8_auto_rules_on {
            return;
        }
        let call = self.my_call();
        if call.is_empty() {
            return;
        }
        let Some((i, rule)) = self
            .cfg
            .js8_auto_rules
            .iter()
            .take(JS8_AUTO_RULES_MAX)
            .enumerate()
            .find(|(_, r)| autorule::matches(r, msg, &call))
        else {
            return;
        };
        let rule = rule.clamped();
        let who = msg.from.trim().to_ascii_uppercase();
        let key = (i, who.clone());
        let cooldown = i64::from(rule.cooldown_min) * 60;
        if self.auto_cooldown.get(&key).is_some_and(|&t| unix_now - t < cooldown) {
            return;
        }
        if self.auto_pending.iter().any(|p| p.rule == i && p.trigger == who) {
            return;
        }
        let me = autorule::Me { call, grid: self.cfg.my_grid.clone() };
        let Some(text) = autorule::render(&rule, msg, &me) else { return };

        if self.auto_cooldown.len() >= AUTO_COOLDOWN_CAP {
            self.auto_cooldown.retain(|_, &mut t| unix_now - t < 240 * 60);
        }
        self.auto_cooldown.insert(key, unix_now);
        self.auto_pending.push(AutoSend {
            rule: i,
            label: rule.label().to_string(),
            trigger: who,
            text,
            sent: 0,
            total: rule.repeat,
            gap_s: i64::from(rule.gap_min) * 60,
            // The first send is due at once: the asker is listening now.
            next_unix: unix_now,
            queued: None,
        });
        self.status_dirty = true;
    }

    /// Queue every group auto-response that is due.
    ///
    /// A due send waits in three cases:
    /// * someone's message is still coming in, so we do not key over them;
    /// * its own previous send is still in the queue;
    /// * the hourly cap is reached.
    ///
    /// Waiting moves it later. It does not skip it.
    fn tick_auto_rules(&mut self, unix_now: i64) {
        while self.auto_sent_at.front().is_some_and(|&t| unix_now - t >= 3600) {
            self.auto_sent_at.pop_front();
        }
        if self.auto_pending.is_empty() {
            return;
        }
        // Two slots of quiet, as for choosing a beacon slot. A message whose
        // last frame was missed must not hold the reply back for minutes.
        let busy = self.assembler.active_within(unix_now, 2 * self.params.slot_s as i64);
        for i in 0..self.auto_pending.len() {
            let p = &self.auto_pending[i];
            if unix_now < p.next_unix || busy {
                continue;
            }
            if p.queued.is_some_and(|m| self.tx_frames.iter().any(|f| f.msg == m)) {
                continue;
            }
            if self.auto_sent_at.len() >= AUTO_RULE_MAX_PER_HOUR {
                continue;
            }
            let text = p.text.clone();
            let frames = self.frames_for_text(&text);
            if frames.is_empty() {
                // Nothing that can be framed. Drop the entry rather than
                // count sends that never happened.
                self.auto_pending[i].sent = self.auto_pending[i].total;
                continue;
            }
            let id = self.next_msg;
            self.enqueue(TxOrigin::Rule, TxBand::Working, frames);
            self.note_sent(&text, unix_now);
            self.auto_sent_at.push_back(unix_now);
            let p = &mut self.auto_pending[i];
            p.queued = Some(id);
            p.sent = p.sent.saturating_add(1);
            p.next_unix = unix_now + p.gap_s;
            self.status_dirty = true;
        }
        self.auto_pending.retain(|p| p.sent < p.total);
        // The panel shows a countdown, so publish once a second while
        // anything is waiting. Once a second, not once a poll, which is the
        // same rule the heartbeat countdown follows.
        if !self.auto_pending.is_empty() && self.auto_status_s != unix_now {
            self.auto_status_s = unix_now;
            self.status_dirty = true;
        }
    }

    /// Remember a message for ` AGN?`.
    fn note_sent(&mut self, text: &str, unix_now: i64) {
        let up = text.trim().to_ascii_uppercase();
        let to = up
            .split_whitespace()
            .next()
            .filter(|t| addressable(t) && !t.eq_ignore_ascii_case(&self.my_call()))
            .unwrap_or_default()
            .to_string();
        self.last_tx = Some(LastTx { to, text: up, unix: unix_now });
    }

    /// Beacon on the interval the operator set, if they set one.
    ///
    /// The first one is a whole interval away rather than immediate: switching
    /// a beacon on should not key the radio the same second, and upstream
    /// starts its clock the same way (`on_hbMacroButton_toggled` schedules the
    /// first at `now + interval`). Turning the interval back to zero — or
    /// changing to a speed that may not beacon — unschedules, so switching it
    /// on again starts a fresh interval rather than firing immediately.
    fn tick_heartbeat(&mut self, unix_now: i64) {
        let every = i64::from(self.cfg.js8_heartbeat_min) * 60;
        if every <= 0 || self.my_call().is_empty() || !self.speed.allows_heartbeat() {
            if self.next_hb_unix != 0 {
                self.next_hb_unix = 0;
                self.hb_countdown_s = None;
                self.status_dirty = true;
            }
            return;
        }
        if self.next_hb_unix == 0 {
            self.next_hb_unix = unix_now + every + self.hb_jitter_s();
        } else if unix_now >= self.next_hb_unix {
            self.next_hb_unix = unix_now + every + self.hb_jitter_s();
            self.hb_count = self.hb_count.wrapping_add(1);
            let grid = self.cfg.my_grid.to_uppercase();
            let hb =
                Compound::heartbeat(&self.my_call(), (!grid.is_empty()).then_some(grid.as_str()));
            if let Some(p) = hb.pack() {
                self.enqueue(TxOrigin::Auto, TxBand::Beacon, vec![Self::single(p)]);
            }
        }
        // The panel shows this ticking down, so a change of one second is a
        // change worth publishing — but only one per second, not one per poll.
        let left = (self.next_hb_unix - unix_now).max(0) as u32;
        if self.hb_countdown_s != Some(left) {
            self.hb_countdown_s = Some(left);
            self.status_dirty = true;
        }
    }

    /// A clear slot in the heartbeat sub-band to beacon in.
    ///
    /// Heartbeats have a home — [`HB_BAND_LO_HZ`]–[`HB_BAND_HI_HZ`] on a
    /// [`HB_SLOT_HZ`] grid — so that a station watching for beacons knows where
    /// to look, and so that an unattended transmitter does not land on somebody
    /// else's conversation. A slot is busy when anything was decoded within one
    /// signal width of it in the last [`ACTIVITY_WINDOW_S`] seconds.
    ///
    /// Two deliberate differences from upstream's `findFreeFreqOffset`, which
    /// draws ten random offsets and takes the first free one:
    ///
    /// * The grid is enumerated rather than sampled, so a free slot is never
    ///   missed the way ten random draws can miss the only one left.
    /// * When nothing is free, upstream falls back to the bottom of the band —
    ///   where every station in that position piles up together. This takes the
    ///   slot that has been quiet longest instead, which is the same answer when
    ///   the band is empty and a better one when it is not.
    ///
    /// The clearance is this speed's own bandwidth rather than upstream's fixed
    /// 50 Hz: Fast is 80 Hz wide and would otherwise be placed half on top of a
    /// signal it had just measured as far enough away.
    /// How far back band activity counts, in seconds.
    ///
    /// [`ACTIVITY_WINDOW_S`] is upstream's flat thirty, which is two slots at
    /// Normal but less than one at Slow — where it would forget a station
    /// between its transmissions and beacon straight on top of it. Two slots is
    /// the floor whatever the speed.
    fn activity_window_s(&self) -> i64 {
        ACTIVITY_WINDOW_S.max(2 * self.params.slot_s as i64)
    }

    fn free_beacon_hz(&self, unix_now: i64, slot_idx: i64) -> f32 {
        let bw = self.speed.bandwidth_hz().max(HB_SLOT_HZ);
        // Stop where the signal still fits inside the sub-band. For Normal —
        // 50 Hz wide, on a 50 Hz grid — that is upstream's ten slots exactly.
        let top = (HB_BAND_HI_HZ - bw).max(HB_BAND_LO_HZ);
        let n = ((top - HB_BAND_LO_HZ) / HB_SLOT_HZ) as usize + 1;
        let slots: Vec<f32> = (0..n).map(|i| HB_BAND_LO_HZ + i as f32 * HB_SLOT_HZ).collect();

        // The most recent activity within a signal width of each slot.
        let last_at = |f: f32| {
            self.activity
                .iter()
                .filter(|(hz, _)| (hz - f).abs() < bw)
                .map(|&(_, t)| t)
                .max()
                .unwrap_or(i64::MIN)
        };
        // Saturating: a slot nothing has ever been heard on reports `i64::MIN`,
        // and the plain subtraction overflows on it.
        let window = self.activity_window_s();
        let free: Vec<f32> = slots
            .iter()
            .copied()
            .filter(|&f| unix_now.saturating_sub(last_at(f)) >= window)
            .collect();
        if !free.is_empty() {
            // Among the free ones, spread stations out: two hearing the same
            // quiet band must not both choose its bottom slot.
            return free[self.beacon_pick(slot_idx, free.len())];
        }
        slots.into_iter().min_by_key(|&f| last_at(f)).unwrap_or(HB_BAND_LO_HZ)
    }

    /// Which of `n` free slots to take. Hashed rather than random so a run is
    /// reproducible, and keyed on the callsign so two stations differ.
    fn beacon_pick(&self, slot_idx: i64, n: usize) -> usize {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.my_call().hash(&mut h);
        slot_idx.hash(&mut h);
        (h.finish() % n as u64) as usize
    }

    /// One slot of jitter on a quarter of beacons.
    ///
    /// Without it, stations that share an interval and started together stay
    /// together, colliding every time. Upstream rolls a die for the same reason
    /// (`scheduleHeartbeat`: "25% of the time, switch intervals"); this hashes
    /// the callsign and the beacon number instead, so two stations differ but a
    /// single station's run is reproducible and the tests are not flaky.
    fn hb_jitter_s(&self) -> i64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.my_call().hash(&mut h);
        self.hb_count.hash(&mut h);
        if h.finish().is_multiple_of(4) { self.params.slot_s as i64 } else { 0 }
    }

    /// Whether *this* heartbeat may be acknowledged right now.
    ///
    /// Every rule is upstream's, and every one exists to stop the station
    /// answering more than it should:
    ///
    /// * The operator has to have asked for it, and the speed has to allow
    ///   beaconing at all.
    /// * Not while a multi-frame message is still arriving — the acknowledgement
    ///   would be interleaved with someone's sentence
    ///   (`mainwindow.cpp`: "do not process HB activity if buffer is not empty").
    /// * Not while we already have something to send. Ours would join the back
    ///   of that queue and go out minutes later carrying a stale report, which
    ///   is upstream's reason for pausing acknowledgements during a QSO.
    /// * At most one per station per [`HB_ACK_COOLDOWN_S`]. A busy band carries
    ///   a heartbeat every slot; a station that answered all of them is the
    ///   flood heartbeats exist to avoid.
    fn hb_ack_allowed(&self, msg: &Js8Msg, unix_now: i64) -> bool {
        self.cfg.js8_hb_ack
            && self.speed.allows_heartbeat()
            && !self.assembler.has_partials()
            && self.tx_frames.is_empty()
            && self.hb_acked.get(&msg.from).is_none_or(|&t| unix_now - t >= HB_ACK_COOLDOWN_S)
    }

    fn note_hb_ack(&mut self, call: &str, unix_now: i64) {
        if self.hb_acked.len() >= HB_ACK_CAP {
            self.hb_acked.retain(|_, &mut t| unix_now - t < HB_ACK_COOLDOWN_S);
        }
        self.hb_acked.insert(call.to_string(), unix_now);
    }

    /// Record a station heard. `speed` is the speed the message came in on,
    /// which with multi-speed decoding is not necessarily the one we transmit
    /// at. Recording ours put Turbo stations in the list as Normal.
    fn note_heard(
        &mut self,
        call: &str,
        grid: Option<String>,
        snr: i16,
        hz: f32,
        utc: i64,
        speed: Js8Speed,
    ) {
        if call.is_empty() {
            return;
        }
        if let Some(h) = self.heard.iter_mut().find(|h| h.call == call) {
            h.snr_db = snr;
            h.audio_hz = hz;
            h.last_utc = utc;
            h.speed = speed;
            if grid.is_some() {
                h.grid = grid;
            }
        } else {
            self.heard.push(Js8Heard {
                call: call.to_string(),
                grid,
                snr_db: snr,
                audio_hz: hz,
                last_utc: utc,
                speed,
            });
        }
        // Most recently heard first, which is the order the panel wants and
        // the order `HEARING?` should answer in.
        self.heard.sort_by(|a, b| b.last_utc.cmp(&a.last_utc));
        self.heard.truncate(HEARD_CAP);
    }

    /// The directed frame a message would send, for the tests.
    #[cfg(test)]
    fn directed_sent(&self, text: &str) -> Option<Directed> {
        Directed::unpack(&self.frames_for_text(text)[0])
    }

    fn synth_burst_48k(&self, payload: Js8Payload, hz: f32) -> Option<BurstPlayer> {
        let audio = modem::encode_frame_12k(self.speed, payload, hz, 0.5);
        let mut rs = MonoResampler::new(DECODE_RATE, OUT_RATE)?;
        let mut out = Vec::new();
        rs.push(&audio, &mut out);
        Some(BurstPlayer { samples: out, pos: 0 })
    }
}

/// The command code for text addressed to a station — `" "`, `varicode.cpp`'s
/// catch-all. The frame says who the words are for and carries none of them.
const FREE_TEXT_CMD: i8 = 31;

/// Commands whose text upstream follows with a checksum
/// (`varicode.cpp`, `checksum_cmds`): relay, the message store, the query
/// family and ` CMD`.
///
/// This station decodes those and displays them but never acts on them — see
/// [`crate::js8::directed`] — so it has no business originating one either. A
/// buffered command with no checksum is worse than plain text: the station at
/// the other end would file it. They go out as text addressed to the station
/// instead, which reads the same to a human and asks nothing of their software.
const CHECKSUMMED_CMDS: &[i8] = &[5, 9, 10, 11, 12, 13, 24];

/// The report at the head of `s`, and how many characters it took.
///
/// Upstream's `optional_num_pattern` is `[-+]?(?:3[01]|[0-2]?[0-9])`, anchored
/// where the command ended — so two digits only when they read 0–31. "SNR 99"
/// therefore sends 9 as the report and leaves the second 9 as text, which looks
/// like an accident until you realise it is what every JS8Call transmits.
fn leading_num(s: &str) -> Option<(i32, usize)> {
    let b = s.as_bytes();
    let sign = usize::from(matches!(b.first(), Some(b'-' | b'+')));
    let digits = b[sign..].iter().take(2).take_while(|c| c.is_ascii_digit()).count();
    let take = match digits {
        2 if s[sign..sign + 2].parse::<i32>().is_ok_and(|v| v <= 31) => 2,
        1 | 2 => 1,
        _ => return None,
    };
    let end = sign + take;
    Some((s[..end].parse().ok()?, end))
}

/// True for something that can be the *to* of a directed frame: a group, or a
/// token shaped like a callsign.
///
/// The digit test is what keeps an ordinary sentence out of the directed path.
/// "HELLO 73" would otherwise pack — `pack_callsign` accepts any token that
/// fits its six-character pattern — and go out addressed to a station called
/// HELLO instead of as the words someone typed.
fn addressable(token: &str) -> bool {
    if token.starts_with('@') {
        return true;
    }
    let core = token.strip_suffix("/P").unwrap_or(token);
    (2..=10).contains(&core.len())
        && core.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'/')
        && core.bytes().any(|c| c.is_ascii_digit())
        && core.bytes().any(|c| c.is_ascii_alphabetic())
}

/// A Maidenhead locator of exactly four or six characters.
///
/// Stricter than [`sdroxide_types::grid_to_latlon`], which reads the first four
/// characters and ignores whatever follows — so "FN42 HELLO" passes there and
/// would be stored as a grid.
fn is_grid(s: &str) -> bool {
    let b = s.as_bytes();
    matches!(b.len(), 4 | 6)
        && b[0].is_ascii_uppercase()
        && b[1].is_ascii_uppercase()
        && b[2].is_ascii_digit()
        && b[3].is_ascii_digit()
        && (b.len() == 4 || (b[4].is_ascii_alphabetic() && b[5].is_ascii_alphabetic()))
        && sdroxide_types::grid_to_latlon(s).is_some()
}

/// The locator a message carries, if it carries one.
///
/// A heartbeat or CQ packs its grid into the frame and the assembler leaves it
/// at the head of the message text; a ` GRID` reply says the same thing
/// explicitly. Nothing else is trusted: free text that happens to read like a
/// locator is prose, and placing a station on the map from it would be a guess.
fn grid_of(msg: &Js8Msg) -> Option<String> {
    if !matches!(msg.cmd.as_deref(), Some("HB" | "CQ" | "GRID")) {
        return None;
    }
    // The grid is the *head* of the text; a multi-frame CQ appends its free
    // text straight onto it, sometimes without a space in between.
    let t = msg.text.trim_start().to_ascii_uppercase();
    [6usize, 4].into_iter().find_map(|n| {
        let head = t.get(..n)?;
        let ends_here = !t.as_bytes().get(n).is_some_and(u8::is_ascii_alphanumeric);
        (ends_here && is_grid(head)).then(|| head.to_string())
    })
}

/// The reception report a completed message is worth, or `None` when it names
/// nobody (issue #357).
///
/// Once per completed message rather than once per frame, which is the
/// difference between JS8 and every slotted mode here: a JS8 decode is
/// seventy-two bits, a station's message can be a dozen of them, and only the
/// last one to arrive has a whole message — and therefore a callsign — behind
/// it. Reporting per frame would post a twelve-frame message as twelve
/// receptions of the same station in the same minute.
///
/// Whether it is *our own* callsign is not decided here: the engine holds the
/// station identity, and it applies the same test to every mode's reports.
fn heard_report(msg: &Js8Msg, grid: Option<&str>) -> Option<DigiAction> {
    let call = msg.from.trim();
    if call.is_empty() {
        return None;
    }
    Some(DigiAction::Heard {
        call: call.to_string(),
        grid: grid.unwrap_or_default().to_string(),
        audio_hz: msg.audio_hz,
        snr_db: msg.snr_db,
        slot_utc: msg.last_slot_utc,
    })
}

impl DigiEngine for Js8Controller {
    fn mode(&self) -> Mode {
        Mode::Js8
    }

    fn on_rx_audio(&mut self, tap: &[f32]) {
        let Some(rs) = self.resampler.as_mut() else {
            return;
        };
        self.tap_scratch.clear();
        rs.push(tap, &mut self.tap_scratch);
        // Resampled once and handed to each speed being listened for: the
        // 12 kHz stream is the same for all of them, only the slot boundaries
        // differ.
        for slot in &mut self.rx_slots {
            slot.push(&self.tap_scratch);
        }
    }

    fn poll(&mut self, now: SystemTime, dial_hz: f64) -> Vec<DigiAction> {
        self.dial_hz = dial_hz;
        let mut actions = Vec::new();
        let unix_now = SlotScheduler::unix_now(now) as i64;
        self.now_unix = unix_now;

        // 1. Drain the decode worker.
        while let Ok((slot_utc, decodes)) = self.res_rx.try_recv() {
            if decodes.is_empty() {
                continue;
            }
            // Band occupancy, for choosing where to beacon. Every decode
            // counts, not only the ones whose callsign we recovered: a signal
            // holds its frequency whether or not we know who it is.
            let window = self.activity_window_s();
            self.activity.retain(|&(_, t)| slot_utc - t < window);
            self.activity.extend(decodes.iter().map(|d| (d.audio_hz, slot_utc)));
            if self.activity.len() > ACTIVITY_CAP_DECODES {
                let drop = self.activity.len() - ACTIVITY_CAP_DECODES;
                self.activity.drain(..drop);
            }

            let mut shared = Vec::new();
            for d in &decodes {
                if let Some(msg) = self.assembler.push(d, slot_utc) {
                    let grid = grid_of(&msg);
                    self.note_heard(
                        &msg.from,
                        grid.clone(),
                        msg.snr_db,
                        msg.audio_hz,
                        msg.last_slot_utc,
                        msg.speed,
                    );
                    // ...and to the reporting networks, which is the same fact
                    // said outwards (issue #357).
                    actions.extend(heard_report(&msg, grid.as_deref()));
                    // Answer before storing, so a reply is queued even if the
                    // conversation list is already full.
                    let policy = ReplyPolicy {
                        auto_reply: self.cfg.js8_auto_reply,
                        hb_ack: self.hb_ack_allowed(&msg, unix_now),
                    };
                    if let Some(reply) = auto_reply(&msg, &self.station(), policy) {
                        // A grid or a status line does not fit in the directed
                        // frame, so an answer can be several frames long.
                        // An acknowledgement is a beacon too: it belongs in
                        // the sub-band, where the station it answers is
                        // listening.
                        let beacon = intent(&msg) == Js8Intent::Heartbeat;
                        let band = if beacon { TxBand::Beacon } else { TxBand::Working };
                        let frames = self.frames_for_reply(reply);
                        self.enqueue(TxOrigin::Auto, band, frames);
                        if beacon {
                            self.note_hb_ack(&msg.from, unix_now);
                        }
                    }
                    // ` AGN?` repeats a whole message, not a directed answer,
                    // so it goes through the text path rather than
                    // `auto_reply`.
                    if let Some(text) = repeat_reply(
                        &msg,
                        &self.station(),
                        self.cfg.js8_auto_reply,
                        self.last_tx.as_ref(),
                        unix_now,
                    ) {
                        let frames = self.frames_for_text(&text);
                        self.enqueue(TxOrigin::Auto, TxBand::Working, frames);
                    }
                    self.trigger_auto_rule(&msg, unix_now);
                    self.remember_message(msg);
                }
                // The activity list shows the raw frame; the conversation
                // view shows the reassembled message. Callsign parsing happens
                // in the assembler, which is the only layer that sees enough
                // frames to know one.
                shared.push(sdroxide_types::Decode {
                    slot_utc,
                    snr_db: d.snr_db.round() as i16,
                    dt: d.dt_sec,
                    audio_hz: d.audio_hz,
                    message: d.payload.to_chars(),
                    to: None,
                    from: None,
                    grid: None,
                    is_cq: false,
                    cq_to: None,
                    rr73_to: None,
                    free_text: true,
                });
            }
            shared.truncate(ACTIVITY_CAP);
            // The clock estimate wants the whole slot's decodes at once — it
            // takes their median dt, which one decode cannot supply.
            self.clock.observe(&shared);
            actions.push(DigiAction::Decodes(shared));
            self.status_dirty = true;
        }

        for expired in self.assembler.expire(unix_now) {
            self.remember_message(expired);
            self.status_dirty = true;
        }

        // 2. Slot boundaries — hand each speed's finished slot to the worker.
        //    One speed normally; all four when the operator asked to hear the
        //    whole band rather than one quarter of it (issue #358), each on its
        //    own clock, because a Turbo slot is not a sixth of a Normal one.
        let cap = self.rx_slots.len() * MAX_JOBS_PER_SPEED;
        for slot in &mut self.rx_slots {
            let Some(job) = slot.take_finished(now) else {
                continue;
            };
            // A dropped job costs one slot of decodes; a queue that grows
            // without bound costs the whole session.
            if self.jobs_queued.load(Ordering::Relaxed) >= cap {
                continue;
            }
            self.jobs_queued.fetch_add(1, Ordering::Relaxed);
            if self.job_tx.send(job).is_err() {
                self.jobs_queued.fetch_sub(1, Ordering::Relaxed);
            }
        }
        let idx = self.scheduler.slot_index(now);

        // 3. Periodic heartbeat, if the operator asked for one, and any group
        //    auto-response that is due.
        self.tick_heartbeat(unix_now);
        self.tick_auto_rules(unix_now);

        // 4. Transmit one frame per slot, inside the window where it still fits.
        if self.burst.is_none() && idx != self.tx_fired_slot && !self.tx_frames.is_empty() {
            let into = self.scheduler.secs_into_slot(now);
            let latest = self.params.slot_s - self.params.burst_s + self.params.tx_offset_s;
            if into >= self.params.tx_offset_s && into <= latest {
                if let Some(frame) = self.tx_frames.pop_front() {
                    self.on_air_msg = Some(frame.msg);
                    // A beacon moves to the sub-band unless the operator asked
                    // for it to stay put; everything else goes out where they
                    // are working.
                    let hz = match frame.band {
                        TxBand::Beacon if !self.cfg.js8_hb_anywhere => {
                            self.free_beacon_hz(unix_now, idx)
                        }
                        _ => self.audio_hz,
                    };
                    if let Some(b) = self.synth_burst_48k(frame.payload, hz) {
                        self.tx_inflight = Some((
                            Js8Decode {
                                payload: frame.payload,
                                audio_hz: hz,
                                dt_sec: 0.0,
                                snr_db: 0.0,
                                sync_score: 0.0,
                                hard_errors: 0,
                                speed: self.speed,
                            },
                            self.scheduler.slot_start_unix(idx) as i64,
                        ));
                        if frame.band == TxBand::Beacon {
                            self.hb_hz = Some(hz);
                        }
                        self.burst = Some(b);
                        self.keyed = true;
                        self.tx_fired_slot = idx;
                        actions.push(DigiAction::KeyTx);
                        self.status_dirty = true;
                    }
                }
            }
        }

        if self.status_dirty {
            actions.push(DigiAction::Status(self.status()));
            self.status_dirty = false;
        }
        actions
    }

    fn tx_burst_active(&self) -> bool {
        self.burst.is_some()
    }

    fn fill_tx_block(&mut self, out: &mut [f32]) -> bool {
        let Some(b) = self.burst.as_mut() else {
            out.fill(0.0);
            return true;
        };
        let mut done = false;
        for slot in out.iter_mut() {
            if b.pos < b.samples.len() {
                *slot = b.samples[b.pos];
                b.pos += 1;
            } else {
                *slot = 0.0;
                done = true;
            }
        }
        if done {
            self.burst = None;
        }
        done
    }

    fn on_burst_done(&mut self) {
        // KeyTx can still be refused by the radio engine. Logging here, after
        // the audio has drained, avoids calling queued or rejected text sent.
        if let Some((frame, slot_utc)) = self.tx_inflight.take() {
            if Js8Flags(frame.payload.frame_type).is_first() {
                self.finish_partial_tx();
            }
            if let Some(msg) = self.tx_assembler.push(&frame, slot_utc) {
                self.remember_message(self.outgoing_message(msg));
            }
        }
        self.burst = None;
        self.keyed = false;
        // Once its last frame has gone, the message is no longer "on the air"
        // and an operator's SEND may replace whatever follows it.
        if !self.tx_frames.front().is_some_and(|f| Some(f.msg) == self.on_air_msg) {
            self.on_air_msg = None;
        }
        if self.tx_frames.is_empty() {
            self.tx_total = 0;
        }
        self.status_dirty = true;
    }

    fn abort(&mut self) {
        self.abort_tx();
        for slot in &mut self.rx_slots {
            slot.buf.clear();
        }
    }

    fn abort_tx(&mut self) {
        self.tx_inflight = None;
        // Retain the prefix that really went out, marked incomplete, without
        // claiming the interrupted frame or the unsent queue was delivered.
        self.finish_partial_tx();
        self.burst = None;
        self.keyed = false;
        self.tx_frames.clear();
        self.tx_total = 0;
        self.on_air_msg = None;
        // ABORT means stop transmitting, not "pause until the next repeat".
        self.auto_pending.clear();
        self.status_dirty = true;
    }

    fn set_config(&mut self, cfg: DigiConfig) {
        // Every speed is a different waveform, so a change means the decoder
        // has to be rebuilt; the engine does that by recreating the controller.
        self.speed = cfg.js8_speed;
        self.params = DigiParams::for_js8(self.speed);
        self.scheduler = SlotScheduler::new(self.params.slot_s, self.params.tx_offset_s);
        // Which speeds are being listened for is the one thing here that can
        // change without the controller being rebuilt, so it is rebuilt in
        // place — and only when it really changed, or every config write would
        // throw away a slot that was half collected.
        let want: Vec<Js8Speed> = listening_speeds(&cfg).into_iter().map(|s| s.speed).collect();
        if self.rx_slots.iter().map(|s| s.speed).ne(want.iter().copied()) {
            self.rx_slots = want.into_iter().map(SpeedSlot::new).collect();
        }
        self.assembler.set_my_call(&Self::call_of(&cfg));
        self.assembler.set_timeout(i64::from(cfg.js8_assembly_timeout_s));
        self.assembler.set_my_groups(cfg.js8_groups.clone());
        // Changed rules, or rules switched off, take everything they had
        // scheduled with them. A repeat of a rule the operator just deleted or
        // rewrote is not what they asked for. Cooldown indices also point at
        // rules that may have moved.
        if cfg.js8_auto_rules != self.cfg.js8_auto_rules || !cfg.js8_auto_rules_on {
            self.auto_pending.clear();
            self.auto_cooldown.clear();
        }
        self.cfg = cfg;
        self.status_dirty = true;
    }

    fn set_audio_hz(&mut self, hz: f32) {
        self.audio_hz = hz;
        self.status_dirty = true;
    }

    fn audio_hz(&self) -> f32 {
        self.audio_hz
    }

    fn call_cq(&mut self) {
        let grid = self.cfg.my_grid.to_uppercase();
        let cq = Compound::cq(&self.my_call(), (!grid.is_empty()).then_some(grid.as_str()), 0);
        if let Some(p) = cq.pack() {
            self.queue_frames(vec![Self::single(p)]);
        }
    }

    /// Queue a message.
    ///
    /// `"HB"` becomes a heartbeat, one self-contained frame, and `"CQ"` a CQ
    /// with this station's callsign and grid. Everything else goes through
    /// [`Js8Controller::frames_for_text`]: a directed command when the message
    /// opens with one, then text for whatever follows.
    fn send_text(&mut self, text: String) {
        if text.trim().is_empty() {
            self.abort_tx();
            return;
        }
        // Both are compound frames rather than text, and both are how the
        // panel's chips reach the air now that they compose instead of
        // transmit (issue #472).
        if let Some(p) = self.cq_for(&text) {
            self.queue_frames(vec![p]);
            return;
        }
        if let Some(p) = self.heartbeat_for(&text) {
            self.enqueue(TxOrigin::Operator, TxBand::Beacon, vec![p]);
            return;
        }
        self.queue_frames(self.frames_for_text(&text));
        self.note_sent(&text, self.now_unix);
    }

    fn stop_qso(&mut self) {
        self.abort_tx();
    }

    /// Empty the conversation. The heard list stays — it is a separate pane,
    /// and it is what `HEARING?` is answered from.
    fn clear_rx(&mut self) {
        self.messages.clear();
        self.status_dirty = true;
    }

    fn status(&self) -> DigiStatus {
        DigiStatus {
            mode: Mode::Js8,
            step: QsoStep::Idle,
            dx_call: None,
            dx_grid: None,
            tx_next: self.keyed,
            tx_pending_msg: None,
            audio_hz: self.audio_hz,
            tx_even: false,
            transmitting: self.keyed,
            tx_watchdog: false,
            tx_refused: None,
            transcript: Vec::new(),
            config: self.cfg.clone(),
            jtty: None,
            tnc: None,
            text_rx: String::new(),
            tx_sent: 0,
            fsq_heard: Vec::new(),
            fsq_messages: Vec::new(),
            rade: None,
            packet: None,
            navtex: None,
            acars: None,
            aprs: None,
            js8: Some(Js8Status {
                speed: self.speed,
                heard: self.heard.clone(),
                messages: self.message_history(),
                tx_frames_pending: self.tx_frames.len().min(255) as u8,
                tx_frames_total: self.tx_total,
                next_hb_in_s: self.hb_countdown_s,
                hb_hz: self.hb_hz,
                auto_pending: self
                    .auto_pending
                    .iter()
                    .map(|p| Js8AutoPending {
                        rule: p.label.clone(),
                        trigger: p.trigger.clone(),
                        text: p.text.clone(),
                        sent: p.sent,
                        total: p.total,
                        next_in_s: (p.next_unix - self.now_unix).max(0) as u32,
                    })
                    .collect(),
            }),
            atchat: None,
            fox_queue: Vec::new(),
            call_queue: Vec::new(),
            clock_offset_s: self.clock.offset_s(),
            cw: None,
            wspr: None,
            pi4: None,
            qso: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `SystemTime` at `unix` seconds, for the slot-boundary arithmetic.
    fn at(unix: f64) -> SystemTime {
        std::time::UNIX_EPOCH + std::time::Duration::from_secs_f64(unix)
    }

    fn cfg() -> DigiConfig {
        DigiConfig {
            my_call: "N0JDS".into(),
            my_grid: "FN42".into(),
            js8_speed: Js8Speed::Normal,
            ..Default::default()
        }
    }

    fn conversation(c: &Js8Controller) -> Vec<Js8Msg> {
        c.status().js8.expect("JS8 status").messages
    }

    fn finish_burst(c: &mut Js8Controller) {
        let mut audio = [0.0; 4096];
        while !c.fill_tx_block(&mut audio) {}
        c.on_burst_done();
    }

    fn transmit_frame(c: &mut Js8Controller, slot_utc: i64) {
        let actions = c.poll(at(slot_utc as f64 + 1.0), 7_078_000.0);
        assert!(actions.iter().any(|a| matches!(a, DigiAction::KeyTx)));
        finish_burst(c);
    }

    #[test]
    fn sent_text_enters_history_only_after_its_burst_finishes() {
        let mut c = Js8Controller::new(cfg(), 48_000.0);
        c.send_text("KN4CRD SNR -12".into());
        assert!(conversation(&c).is_empty(), "queued is not sent");
        c.poll(at(1_700_000_010.5), 7_078_000.0);
        assert!(conversation(&c).is_empty(), "keying is not completion");
        finish_burst(&mut c);
        let history = conversation(&c);
        assert_eq!(history.len(), 1);
        let msg = &history[0];
        assert_eq!(msg.from, "N0JDS");
        assert_eq!(msg.to, "KN4CRD");
        assert_eq!(msg.cmd.as_deref(), Some("SNR"));
        assert_eq!(msg.text, "-12");
        assert_eq!(msg.last_slot_utc, 1_700_000_010);
        assert_eq!(msg.audio_hz, 1500.0);
        assert!(msg.complete);
        assert!(!msg.to_me);
        assert!(c.heard.is_empty(), "our own TX must not become a received spot");
        c.clear_rx();
        assert!(conversation(&c).is_empty());
    }

    #[test]
    fn a_sent_multi_frame_message_is_one_history_entry() {
        let mut c = Js8Controller::new(cfg(), 48_000.0);
        let text = "HELLO FROM THE HILLS THIS IS A LONG MESSAGE THAT NEEDS SEVERAL FRAMES";
        c.send_text(format!("KN4CRD {text}"));
        let frames = c.tx_frames.len();
        assert!(frames > 2);
        for i in 0..frames {
            transmit_frame(&mut c, 1_700_000_010 + i as i64 * 15);
            let history = conversation(&c);
            assert_eq!(history.len(), 1, "frame {i} duplicated the message");
            assert_eq!(history[0].complete, i == frames - 1);
            assert_eq!(history[0].frames as usize, i + 1);
        }
        let history = conversation(&c);
        assert_eq!(history[0].from, "N0JDS");
        assert_eq!(history[0].to, "KN4CRD");
        assert_eq!(history[0].text.trim(), text);
    }

    #[test]
    fn abort_keeps_only_the_frames_that_finished() {
        let mut c = Js8Controller::new(cfg(), 48_000.0);
        c.send_text("KN4CRD SNR -12".into());
        c.abort_tx();
        assert!(conversation(&c).is_empty());
        c.send_text("KN4CRD SNR -12".into());
        c.poll(at(1_700_000_010.5), 7_078_000.0);
        c.abort_tx(); // Includes a KeyTx refused by the engine's safety rails.
        assert!(conversation(&c).is_empty());

        let text = "HELLO FROM THE HILLS THIS IS A LONG MESSAGE THAT NEEDS SEVERAL FRAMES";
        c.send_text(format!("KN4CRD {text}"));
        transmit_frame(&mut c, 1_700_000_025);
        transmit_frame(&mut c, 1_700_000_040);
        let prefix = conversation(&c)[0].text.clone();
        assert!(!prefix.trim().is_empty());
        assert!(text.starts_with(prefix.trim()));
        assert_ne!(prefix.trim(), text);
        c.poll(at(1_700_000_055.5), 7_078_000.0);
        c.abort_tx();
        let history = conversation(&c);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].frames, 2);
        assert!(!history[0].complete);
        assert_eq!(history[0].text, prefix, "an interrupted frame was counted as sent");

        c.send_text("KN4CRD SNR -5".into());
        transmit_frame(&mut c, 1_700_000_070);
        assert_eq!(conversation(&c).len(), 2);
        assert_eq!(conversation(&c)[1].text, "-5");
    }

    #[test]
    fn broadcasts_and_undirected_text_also_enter_sent_history() {
        let mut c = Js8Controller::new(cfg(), 48_000.0);
        for (i, text) in ["HB", "CQ", "HELLO WORLD"].into_iter().enumerate() {
            c.send_text(text.into());
            transmit_frame(&mut c, 1_700_000_010 + i as i64 * 15);
        }
        let history = conversation(&c);
        assert_eq!(history.len(), 3);
        assert!(history.iter().all(|m| m.from == "N0JDS" && m.complete && !m.to_me));
        assert_eq!(history[0].cmd.as_deref(), Some("HB"));
        assert_eq!(history[1].cmd.as_deref(), Some("CQ"));
        assert_eq!(history[2].text.trim(), "HELLO WORLD");
    }

    #[test]
    fn automatic_query_replies_are_recorded_with_the_received_question() {
        // Directed queries already work on main, so this test is independent
        // of the separate fix for receiving compound heartbeat frames.
        for (query, command, text) in [
            (" SNR?", "SNR", "-11"),
            (" GRID?", "GRID", "FN42"),
            (" STATUS?", "STATUS", "PORTABLE"),
        ] {
            let mut c =
                Js8Controller::new(DigiConfig { js8_status: "PORTABLE".into(), ..cfg() }, 48_000.0);
            let payload = Directed {
                from: "KN4CRD".into(),
                to: "N0JDS".into(),
                cmd: frame::cmd_code(query).expect("command"),
                num: None,
                portable_from: false,
                portable_to: false,
            }
            .pack()
            .expect("packs");
            let (tx, rx) = std::sync::mpsc::channel();
            c.res_rx = rx;
            tx.send((
                1_700_000_010,
                vec![Js8Decode {
                    payload: Js8Controller::single(payload),
                    audio_hz: 1250.0,
                    dt_sec: 0.0,
                    snr_db: -11.0,
                    sync_score: 3.0,
                    hard_errors: 0,
                    speed: Js8Speed::Normal,
                }],
            ))
            .expect("delivered");
            c.poll(at(1_700_000_025.0), 7_078_000.0);
            assert_eq!(conversation(&c).len(), 1, "an unsent answer was logged");
            let frames = c.tx_frames.len();
            assert!(frames > 0);
            for i in 0..frames {
                transmit_frame(&mut c, 1_700_000_025 + i as i64 * 15);
            }
            let history = conversation(&c);
            assert_eq!(history.len(), 2, "{query}");
            assert_eq!(history[0].from, "KN4CRD");
            assert_eq!(history[1].from, "N0JDS");
            assert_eq!(history[1].to, "KN4CRD");
            assert_eq!(history[1].cmd.as_deref(), Some(command));
            assert_eq!(history[1].text.trim(), text);
            assert!(history[1].complete);
            assert_eq!(c.heard.len(), 1, "TX must not create a heard station");
        }
    }

    #[test]
    fn a_transmitted_heartbeat_acknowledgement_keeps_its_report_and_beacon_frequency() {
        let mut c = Js8Controller::new(DigiConfig { js8_hb_ack: true, ..cfg() }, 48_000.0);
        // Use the existing directed-reply fixture: heartbeat reception is
        // covered by the separate heartbeat patch, not a prerequisite here.
        let reply = auto_reply(
            &heartbeat_from("KN4CRD", -11),
            &c.station(),
            ReplyPolicy { auto_reply: true, hb_ack: true },
        )
        .expect("reply");
        let frames = c.frames_for_reply(reply);
        c.enqueue(TxOrigin::Auto, TxBand::Beacon, frames);
        transmit_frame(&mut c, 1_700_000_010);
        let history = conversation(&c);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].from, "N0JDS");
        assert_eq!(history[0].to, "KN4CRD");
        assert_eq!(history[0].cmd.as_deref(), Some("HEARTBEAT SNR"));
        assert_eq!(history[0].text, "-11");
        assert_eq!(Some(history[0].audio_hz), c.hb_hz);
        assert!(history[0].audio_hz >= HB_BAND_LO_HZ);
        assert!(history[0].audio_hz < HB_BAND_HI_HZ);
        assert!(history[0].complete);
    }

    #[test]
    fn sent_history_preserves_the_selected_speed_and_js8_callsign() {
        for speed in Js8Speed::ALL {
            let mut c = Js8Controller::new(
                DigiConfig { js8_speed: speed, js8_call: "W1ABC".into(), ..cfg() },
                48_000.0,
            );
            c.send_text("KN4CRD SNR -3".into());
            transmit_frame(&mut c, 1_700_000_040);
            let history = conversation(&c);
            assert_eq!(history.len(), 1);
            assert_eq!(history[0].from, "W1ABC");
            assert_eq!(history[0].speed, speed);
            assert_eq!(history[0].first_slot_utc, 1_700_000_040);
            assert_eq!(history[0].last_slot_utc, 1_700_000_040);
        }
    }

    #[test]
    fn sent_history_evicts_the_oldest_message_when_full() {
        let mut c = Js8Controller::new(cfg(), 48_000.0);
        c.messages = (0..MSG_CAP)
            .map(|i| {
                let mut m = heartbeat_from("KN4CRD", -11);
                m.last_slot_utc = i as i64;
                m
            })
            .collect();
        c.send_text("KN4CRD SNR -7".into());
        transmit_frame(&mut c, 1_700_000_010);
        let history = conversation(&c);
        assert_eq!(history.len(), MSG_CAP);
        assert_eq!(history[0].last_slot_utc, 1);
        assert_eq!(history.last().expect("newest").from, "N0JDS");
        assert_eq!(history.last().expect("newest").text, "-7");
    }

    /// What a receiver makes of a frame sequence — the only measure of whether
    /// what we transmitted actually said anything.
    fn reassemble(frames: &[Js8Payload]) -> String {
        let mut a = Js8Assembler::new("KN4CRD");
        let mut out = None;
        for (i, f) in frames.iter().enumerate() {
            let d = Js8Decode {
                payload: *f,
                audio_hz: 1500.0,
                dt_sec: 0.0,
                snr_db: 0.0,
                sync_score: 3.0,
                hard_errors: 0,
                speed: Js8Speed::Normal,
            };
            out = a.push(&d, 1000 + i as i64 * 15);
        }
        out.map(|m| m.text).unwrap_or_default()
    }

    #[test]
    fn a_short_message_becomes_one_frame_flagged_first_and_last() {
        let c = Js8Controller::new(cfg(), 48_000.0);
        let frames = c.frames_for_text("HELLO");
        assert_eq!(frames.len(), 1);
        let flags = Js8Flags(frames[0].frame_type);
        assert!(flags.is_first() && flags.is_last(), "a single frame is both ends");
    }

    #[test]
    fn a_long_message_spans_frames_flagged_only_at_the_ends() {
        let c = Js8Controller::new(cfg(), 48_000.0);
        let frames = c.frames_for_text(
            "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG AND THEN KEEPS ON RUNNING FOR A WHILE",
        );
        assert!(frames.len() > 2, "expected several frames, got {}", frames.len());
        let first = Js8Flags(frames[0].frame_type);
        let last = Js8Flags(frames[frames.len() - 1].frame_type);
        assert!(first.is_first() && !first.is_last());
        assert!(last.is_last() && !last.is_first());
        for mid in &frames[1..frames.len() - 1] {
            let f = Js8Flags(mid.frame_type);
            assert!(!f.is_first() && !f.is_last(), "a middle frame claims an end");
        }
    }

    #[test]
    fn a_message_round_trips_through_the_assembler() {
        // The two halves of the mode have to agree: what we transmit is what a
        // receiver reassembles.
        let c = Js8Controller::new(cfg(), 48_000.0);
        let text = "HELLO WORLD THIS IS A LONGER MESSAGE SPANNING SEVERAL FRAMES";
        let frames = c.frames_for_text(text);
        let mut a = Js8Assembler::new("KN4CRD");

        let mut got = None;
        for (i, f) in frames.iter().enumerate() {
            let d = Js8Decode {
                payload: *f,
                audio_hz: 1500.0,
                dt_sec: 0.0,
                snr_db: 0.0,
                sync_score: 3.0,
                hard_errors: 0,
                speed: Js8Speed::Normal,
            };
            got = a.push(&d, 1000 + i as i64 * 15);
        }
        let msg = got.expect("the last frame completes the message");
        assert!(text.starts_with(msg.text.trim_end()), "got {:?}", msg.text);
        assert_eq!(msg.frames as usize, frames.len());
    }

    #[test]
    fn aborting_clears_the_queue_rather_than_pausing_it() {
        let mut c = Js8Controller::new(cfg(), 48_000.0);
        c.send_text("A LONG MESSAGE THAT NEEDS SEVERAL FRAMES TO GET ALL THE WAY OUT".into());
        assert!(!c.tx_frames.is_empty());
        c.abort_tx();
        assert!(c.tx_frames.is_empty(), "abort must not leave frames to resume");
        assert_eq!(c.tx_total, 0);
    }

    #[test]
    fn empty_text_cancels_instead_of_transmitting_nothing() {
        let mut c = Js8Controller::new(cfg(), 48_000.0);
        c.send_text("HELLO".into());
        c.send_text("   ".into());
        assert!(c.tx_frames.is_empty());
    }

    #[test]
    fn calling_cq_queues_exactly_one_self_contained_frame() {
        let mut c = Js8Controller::new(cfg(), 48_000.0);
        c.call_cq();
        assert_eq!(c.tx_frames.len(), 1);
        let flags = Js8Flags(c.tx_frames[0].payload.frame_type);
        assert!(flags.is_first() && flags.is_last());
        let cq = Compound::unpack(&c.tx_frames[0].payload).expect("a compound frame");
        assert!(cq.is_cq());
        assert_eq!(cq.call, "N0JDS");
    }

    #[test]
    fn the_mode_reports_itself_as_js8() {
        let c = Js8Controller::new(cfg(), 48_000.0);
        assert_eq!(c.mode(), Mode::Js8);
        assert_eq!(c.status().mode, Mode::Js8);
        assert!(c.status().js8.is_some(), "the panel keys off this being Some");
    }

    #[test]
    fn changing_speed_retimes_the_scheduler() {
        let mut c = Js8Controller::new(cfg(), 48_000.0);
        assert_eq!(c.params.slot_s, 15.0);
        c.set_config(DigiConfig { js8_speed: Js8Speed::Turbo, ..cfg() });
        assert_eq!(c.params.slot_s, 6.0);
        assert_eq!(c.status().js8.expect("js8 status").speed, Js8Speed::Turbo);
    }

    /// MULTI listens on all four speeds at once; without it a station on any
    /// other speed is simply not there, which is why an exchange between two
    /// speeds could not happen at all (issue #358). The transmit speed is
    /// untouched by it: what goes out is still the one chip that is lit.
    #[test]
    fn multi_decode_listens_on_every_speed_without_changing_what_is_transmitted() {
        let mut c = Js8Controller::new(cfg(), 48_000.0);
        assert_eq!(c.rx_slots.iter().map(|s| s.speed).collect::<Vec<_>>(), [Js8Speed::Normal]);

        c.set_config(DigiConfig { js8_multi_decode: true, ..cfg() });
        assert_eq!(c.rx_slots.iter().map(|s| s.speed).collect::<Vec<_>>(), Js8Speed::ALL);
        assert_eq!(c.speed, Js8Speed::Normal, "the transmit speed is not what MULTI changes");
        assert_eq!(c.params.slot_s, 15.0, "and neither is the transmit slot clock");

        // Each speed keeps its own boundary, which is the whole difficulty: a
        // Turbo slot is six seconds and a Slow one thirty, so one clock cannot
        // serve both.
        for slot in &c.rx_slots {
            assert_eq!(slot.scheduler.slot_index(at(60.0)), (60.0 / slot.speed.slot_s()) as i64);
        }

        // Turning it back off leaves the one speed being worked.
        c.set_config(cfg());
        assert_eq!(c.rx_slots.iter().map(|s| s.speed).collect::<Vec<_>>(), [Js8Speed::Normal]);
    }

    /// A speed's slot is submitted on *its own* boundary, with the audio that
    /// arrived inside it, and the first partial slot is thrown away rather than
    /// decoded — it started mid-transmission and holds a whole frame of
    /// nothing.
    #[test]
    fn each_speed_hands_over_its_own_slot_and_only_a_whole_one() {
        let mut slot = SpeedSlot::new(Js8Speed::Turbo);
        let full = slot.slot_samples();

        // Opening mid-slot: the boundary passes, and what was collected before
        // it goes nowhere.
        slot.push(&vec![0.1; full]);
        assert!(slot.take_finished(at(6.0)).is_none(), "the first boundary only starts the clock");
        assert!(slot.buf.is_empty());

        // A slot too short to hold a transmission is dropped, not decoded.
        slot.push(&vec![0.1; full / 4]);
        assert!(slot.take_finished(at(12.0)).is_none(), "a quarter slot has nothing in it to find");

        // A whole one is handed over, stamped with the slot it filled.
        slot.push(&vec![0.1; full]);
        let job = slot.take_finished(at(18.0)).expect("a full slot is decoded");
        assert_eq!(job.speed, Js8Speed::Turbo);
        assert_eq!(job.slot_utc, 12, "the slot it filled, not the one it was handed over in");
        assert_eq!(job.audio.len(), full);
        // ...and taking it leaves nothing behind for the next slot to inherit.
        assert!(slot.buf.is_empty());
        assert!(slot.take_finished(at(18.0)).is_none(), "the same boundary twice is one slot");
    }

    #[test]
    fn a_typed_command_goes_out_as_a_directed_frame() {
        let c = Js8Controller::new(cfg(), 48_000.0);
        for (typed, to, cmd, num) in [
            ("KN4CRD SNR?", "KN4CRD", " SNR?", None),
            ("kn4crd snr?", "KN4CRD", " SNR?", None),
            ("KN4CRD SNR -12", "KN4CRD", " SNR", Some(-12)),
            ("KN4CRD HW CPY?", "KN4CRD", " HW CPY?", None),
            ("KN4CRD 73", "KN4CRD", " 73", None),
            ("@ALLCALL HEARING?", "@ALLCALL", " HEARING?", None),
            // A shorthand renders as the command it stands for.
            ("KN4CRD ?", "KN4CRD", " SNR?", None),
        ] {
            let d = c.directed_sent(typed).unwrap_or_else(|| panic!("{typed:?} was not directed"));
            assert_eq!(d.from, "N0JDS", "{typed:?}");
            assert_eq!(d.to, to, "{typed:?}");
            assert_eq!(frame::cmd_text(d.cmd), Some(cmd), "{typed:?}");
            assert_eq!(d.num, num, "{typed:?}");
        }
    }

    /// The transmit framing, against JS8Call's own encoder.
    ///
    /// Expected values produced by linking upstream `varicode.cpp` and calling
    /// `Varicode::buildMessageFrames("N0JDS", "FN42", "", <text>, false, false,
    /// JS8CallNormal, nullptr)` at the commit in `vendor/js8call/PROVENANCE.md`
    /// — the same technique as `tests/js8_varicode_vectors`, and the only way
    /// to know our frames are the ones every other station sends rather than
    /// merely ones our own decoder happens to read back. The trailing number is
    /// upstream's `TransmissionType`: 1 = first, 2 = last, 3 = both, 0 = middle,
    /// which is exactly what [`Js8Controller::flag_sequence`] stamps.
    #[test]
    fn the_frames_we_transmit_are_the_frames_js8call_transmits() {
        let c = Js8Controller::new(cfg(), 48_000.0);
        for (typed, want) in [
            ("KN4CRD GRID FN42", &["VlCZhHSNwyy0/1", "uskC++++++++/2"][..]),
            ("KN4CRD STATUS PORTABLE ON A HILL", &["VlCZhHSNwyS0/1", "-mvbbTyi3+++/2"][..]),
            (
                "KN4CRD HEARING VK3ABC G0ABC",
                &["VlCZhHSNwyW0/1", "ikGAVd55AdvV/0", "tG++++++++++/2"][..],
            ),
            ("KN4CRD SNR -12", &["VlCZhHSNwzaJ/3"][..]),
            ("KN4CRD SNR?", &["VlCZhHSNwy00/3"][..]),
            ("KN4CRD HW CPY?", &["VlCZhHSNwzC0/3"][..]),
            // Upstream's number pattern is `[-+]?(?:3[01]|[0-2]?[0-9])`, so it
            // takes one 9 as the report and leaves the other as text.
            ("KN4CRD SNR 99", &["VlCZhHSNwzae/1", "y8++++++++++/2"][..]),
            // Text addressed to a station is command 31, not a bare data frame:
            // that is what tells them the words are meant for them.
            ("KN4CRD HELLO THERE", &["VlCZhHSNwzy0/1", "tYuNz6V+++++/2"][..]),
        ] {
            let got: Vec<String> = c
                .frames_for_text(typed)
                .iter()
                .map(|p| format!("{}/{}", p.to_chars(), p.frame_type))
                .collect();
            assert_eq!(got, want, "{typed:?}");
        }
    }

    /// Messages whose sender or addressee does not fit a directed frame go out
    /// exactly as JS8Call sends them:
    /// * a custom group such as @MAGNET;
    /// * a slashed call such as W1AW/4;
    /// * our own call as KN4CRD/QRP.
    ///
    /// Until this was fixed, a message to a custom group went out as bare text
    /// with nobody named on it.
    #[test]
    fn compound_messages_are_framed_as_js8call_frames_them() {
        for &(mycall, grid, text, want) in crate::js8_vectors::MESSAGES {
            let c = Js8Controller::new(
                DigiConfig { my_call: mycall.into(), my_grid: grid.into(), ..cfg() },
                48_000.0,
            );
            let got: Vec<String> = c
                .frames_for_text(text)
                .iter()
                .map(|p| format!("{}/{}", p.to_chars(), p.frame_type))
                .collect();
            assert_eq!(got.join(" "), want, "{mycall}: {text:?}");
        }
    }

    /// Relay, the message store and the query family carry a checksum upstream
    /// appends after the text. This station does not act on any of them, so it
    /// does not originate one either — a buffered command with no checksum is
    /// worse than plain text, because the far end would file it.
    #[test]
    fn commands_this_station_does_not_implement_go_out_as_text() {
        let c = Js8Controller::new(cfg(), 48_000.0);
        for typed in ["KN4CRD QUERY CALL VK3ABC", "KN4CRD MSG TO: VK3ABC HI", "KN4CRD CMD OFF"] {
            let frames = c.frames_for_text(typed);
            let d = Directed::unpack(&frames[0]).unwrap_or_else(|| panic!("{typed:?}"));
            assert_eq!(d.cmd, FREE_TEXT_CMD, "{typed:?} claimed a command we cannot complete");
            assert_eq!(d.to, "KN4CRD", "{typed:?}");
            // The words still go out; only the machine-readable claim does not.
            let text = reassemble(&frames);
            assert!(text.contains("VK3ABC") || text.contains("OFF"), "{typed:?} lost its text");
        }
    }

    #[test]
    fn conversation_is_not_mistaken_for_a_command() {
        // A message with no addressee is text and nothing else — there is no
        // station to direct it at.
        let c = Js8Controller::new(cfg(), 48_000.0);
        for typed in [
            "HELLO 73",         // a sentence, not a station called HELLO
            "GOOD MORNING ALL", // no addressee at all
            "73",               // no addressee
            "N0JDS SNR?",       // our own callsign: not a directed command
        ] {
            let up = typed.to_ascii_uppercase();
            assert!(c.directed_head(&up).is_none(), "{typed:?} became a directed frame");
        }
        // And a near-miss on a real command is carried as the words someone
        // typed, not as the command it nearly was.
        let frames = c.frames_for_text("KN4CRD SNRX?");
        let d = Directed::unpack(&frames[0]).expect("addressed to KN4CRD");
        assert_eq!(d.cmd, FREE_TEXT_CMD);
        assert!(reassemble(&frames).contains("SNRX?"));
    }

    /// The half of a reply a directed frame cannot hold.
    ///
    /// A grid, a status line or a list of stations heard is text, and the frame
    /// has room for a command and a six-bit number. Upstream sends the command
    /// and then the words; so must we, or the station transmits a bare " GRID"
    /// and answers nothing.
    #[test]
    fn a_command_that_carries_text_sends_the_text_too() {
        let c = Js8Controller::new(cfg(), 48_000.0);
        for (typed, cmd, expect) in [
            ("KN4CRD GRID FN42", " GRID", "FN42"),
            ("KN4CRD STATUS PORTABLE ON A HILL", " STATUS", "PORTABLE ON A HILL"),
            ("KN4CRD INFO 100W DIPOLE", " INFO", "100W DIPOLE"),
        ] {
            let frames = c.frames_for_text(typed);
            assert!(frames.len() >= 2, "{typed:?} produced {} frame(s)", frames.len());
            let d = Directed::unpack(&frames[0]).unwrap_or_else(|| panic!("{typed:?}"));
            assert_eq!(frame::cmd_text(d.cmd), Some(cmd), "{typed:?}");
            assert_eq!(d.num, None, "{typed:?} put its text in the number field");
            assert!(reassemble(&frames).contains(expect), "{typed:?} lost its text");
        }
        // Longest match wins: " STATUS?" is a question, " STATUS" an answer.
        let q = c.frames_for_text("KN4CRD STATUS?");
        assert_eq!(
            frame::cmd_text(Directed::unpack(&q[0]).expect("directed").cmd),
            Some(" STATUS?")
        );
        assert_eq!(q.len(), 1, "a question needs nothing after it");
    }

    /// Only ` SNR` and ` HEARTBEAT SNR` have a number field. For anything else
    /// the digits are text, and swallowing them here would lose them.
    #[test]
    fn only_the_report_commands_swallow_a_number() {
        let c = Js8Controller::new(cfg(), 48_000.0);
        for (typed, num) in [
            ("KN4CRD SNR -12", Some(-12)),
            ("KN4CRD SNR +15", Some(15)),
            ("KN4CRD SNR 05", Some(5)),
            ("KN4CRD SNR 31", Some(31)),
        ] {
            let frames = c.frames_for_text(typed);
            assert_eq!(frames.len(), 1, "{typed:?} sent text it did not need");
            assert_eq!(Directed::unpack(&frames[0]).and_then(|d| d.num), num, "{typed:?}");
        }
        // A grid is not a number, and must survive as text.
        let frames = c.frames_for_text("KN4CRD GRID 42");
        assert_eq!(Directed::unpack(&frames[0]).and_then(|d| d.num), None);
        assert!(reassemble(&frames).contains("42"), "the text was swallowed");
    }

    #[test]
    fn an_auto_reply_puts_its_answer_on_the_air() {
        // The regression this exists for: `auto_reply` used to return the
        // command alone, so an answer to GRID? / STATUS? / HEARING? went out as
        // a bare command with the answer left behind.
        let mut c = Js8Controller::new(cfg(), 48_000.0);
        c.cfg.js8_status = "PORTABLE".into();
        c.note_heard("VK3ABC", None, -3, 1400.0, 1000, Js8Speed::Normal);
        for (cmd, want_cmd, want_text) in [
            ("GRID?", " GRID", Some("FN42")),
            ("STATUS?", " STATUS", Some("PORTABLE")),
            ("HEARING?", " HEARING", Some("VK3ABC")),
            ("SNR?", " SNR", None),
        ] {
            let msg = Js8Msg {
                from: "KN4CRD".into(),
                to: "N0JDS".into(),
                text: String::new(),
                cmd: Some(cmd.into()),
                snr_db: -7,
                audio_hz: 1500.0,
                first_slot_utc: 1000,
                last_slot_utc: 1000,
                frames: 1,
                complete: true,
                to_me: true,
                speed: Js8Speed::Normal,
            };
            let reply = auto_reply(&msg, &c.station(), ReplyPolicy::default())
                .unwrap_or_else(|| panic!("{cmd} went unanswered"));
            let frames = c.frames_for_reply(reply);
            let d = Directed::unpack(&frames[0]).unwrap_or_else(|| panic!("{cmd}"));
            assert_eq!(frame::cmd_text(d.cmd), Some(want_cmd), "{cmd}");
            assert_eq!(d.to, "KN4CRD", "{cmd}");
            match want_text {
                Some(t) => {
                    assert!(frames.len() >= 2, "{cmd} answered in one frame with no room for {t}");
                    assert!(reassemble(&frames).contains(t), "{cmd} lost its answer");
                }
                // A report rides in the frame's number field; a second frame to
                // say it again would double the transmission.
                None => {
                    assert_eq!(frames.len(), 1, "{cmd} sent text it did not need");
                    assert_eq!(d.num, Some(-7));
                }
            }
            // Whatever its length, the sequence has to be closed at both ends
            // or the receiver never completes the message.
            assert!(Js8Flags(frames[0].frame_type).is_first(), "{cmd}");
            assert!(Js8Flags(frames[frames.len() - 1].frame_type).is_last(), "{cmd}");
        }
    }

    #[test]
    fn a_typed_heartbeat_becomes_a_heartbeat_frame() {
        // There is no ` HB` in the command table, so without this the panel's
        // HB button puts the literal text "@ALLCALL HB" on the air.
        let mut c = Js8Controller::new(cfg(), 48_000.0);
        for typed in ["@ALLCALL HB", "HB", "hb", "HEARTBEAT"] {
            c.send_text(typed.into());
            assert_eq!(c.tx_frames.len(), 1, "{typed:?}");
            let hb =
                Compound::unpack(&c.tx_frames[0].payload).unwrap_or_else(|| panic!("{typed:?}"));
            assert_eq!(hb.call, "N0JDS", "{typed:?}");
            assert!(!hb.is_cq(), "{typed:?} came out as a CQ");
            assert_eq!(hb.grid().as_deref(), Some("FN42"), "{typed:?}");
        }
    }

    /// The same for a CQ: there is no ` CQ` command either, so once the panel's
    /// CQ chip composes instead of transmitting, a typed "CQ" would go out as
    /// the two literal letters without this (issue #472).
    #[test]
    fn a_typed_cq_becomes_a_cq_frame() {
        let mut c = Js8Controller::new(cfg(), 48_000.0);
        for typed in ["@ALLCALL CQ", "CQ", "cq"] {
            c.send_text(typed.into());
            assert_eq!(c.tx_frames.len(), 1, "{typed:?}");
            let cq =
                Compound::unpack(&c.tx_frames[0].payload).unwrap_or_else(|| panic!("{typed:?}"));
            assert_eq!(cq.call, "N0JDS", "{typed:?}");
            assert!(cq.is_cq(), "{typed:?} did not come out as a CQ");
            assert_eq!(cq.grid().as_deref(), Some("FN42"), "{typed:?}");
        }
    }

    /// A station heard is reported outwards as well as listed on screen, so a
    /// JS8 station appears on the PSK Reporter map like an FT8 one — issue #357
    /// is that it never did, because a JS8 *decode* is one frame of a message
    /// and names nobody.
    #[test]
    fn a_completed_message_is_worth_one_reception_report() {
        let msg = Js8Msg {
            from: "KN4CRD".into(),
            to: "@ALLCALL".into(),
            text: "EM73".into(),
            cmd: Some("HB".into()),
            snr_db: -11,
            audio_hz: 1234.0,
            first_slot_utc: 1000,
            last_slot_utc: 1030,
            frames: 12,
            complete: true,
            to_me: false,
            speed: Js8Speed::Normal,
        };
        let Some(DigiAction::Heard { call, grid, audio_hz, snr_db, slot_utc }) =
            heard_report(&msg, grid_of(&msg).as_deref())
        else {
            panic!("a named station has to be reported");
        };
        assert_eq!(call, "KN4CRD");
        assert_eq!(grid, "EM73");
        assert_eq!(audio_hz, 1234.0);
        assert_eq!(snr_db, -11);
        assert_eq!(slot_utc, 1030, "the slot it finished in, not the one it opened in");

        // A grid nobody sent is no grid; PSK Reporter places the station from
        // its own database rather than from one we invented.
        let plain = Js8Msg { text: "HELLO".into(), ..msg.clone() };
        assert!(matches!(
            heard_report(&plain, grid_of(&plain).as_deref()),
            Some(DigiAction::Heard { ref grid, .. }) if grid.is_empty()
        ));

        // A message whose sender the assembler never recovered names nobody,
        // and a report with no callsign in it is not a report.
        let anon = Js8Msg { from: String::new(), ..msg };
        assert!(heard_report(&anon, None).is_none());
    }

    #[test]
    fn a_station_that_sent_a_grid_is_remembered_with_it() {
        let hb = |text: &str, cmd: &str| Js8Msg {
            from: "KN4CRD".into(),
            to: "@ALLCALL".into(),
            text: text.into(),
            cmd: Some(cmd.into()),
            snr_db: -5,
            audio_hz: 1500.0,
            first_slot_utc: 1000,
            last_slot_utc: 1000,
            frames: 1,
            complete: true,
            to_me: true,
            speed: Js8Speed::Normal,
        };
        assert_eq!(grid_of(&hb("EM73", "HB")).as_deref(), Some("EM73"));
        assert_eq!(grid_of(&hb("QF22", "CQ")).as_deref(), Some("QF22"));
        assert_eq!(grid_of(&hb("FN42AB", "GRID")).as_deref(), Some("FN42AB"));
        // A CQ's free text is appended straight onto the grid, so the head has
        // to end where the locator does.
        assert_eq!(grid_of(&hb("EM73 CQ CQ CQ", "CQ")).as_deref(), Some("EM73"));
        assert_eq!(grid_of(&hb("EM73HELLO", "CQ")), None);
        // Prose that opens like a locator is still prose.
        assert_eq!(grid_of(&hb("FN42 AND MORE", "SNR")), None);
        assert_eq!(grid_of(&hb("HELLO", "HB")), None);
    }

    #[test]
    fn a_decoded_heartbeat_is_answered_only_when_allowed() {
        // Exercise the real compound-frame receive path, not a hand-built
        // Js8Msg whose to_me flag already assumes the assembler is correct.
        for grid in [Some("EM73"), None] {
            for speed in Js8Speed::ALL {
                for auto in [false, true] {
                    for acknowledge in [false, true] {
                        for from in ["KN4CRD", "N0JDS"] {
                            let mut c = Js8Controller::new(
                                DigiConfig {
                                    js8_speed: speed,
                                    js8_auto_reply: auto,
                                    js8_hb_ack: acknowledge,
                                    ..cfg()
                                },
                                48_000.0,
                            );
                            let (tx, rx) = std::sync::mpsc::channel();
                            c.res_rx = rx;
                            let payload = Compound::heartbeat(from, grid).pack().expect("packs");
                            let decode = Js8Decode {
                                payload: Js8Payload {
                                    bits: payload.bits,
                                    frame_type: Js8Flags::FIRST | Js8Flags::LAST,
                                },
                                audio_hz: 750.0,
                                dt_sec: 0.0,
                                snr_db: -11.0,
                                sync_score: 3.0,
                                hard_errors: 0,
                                speed,
                            };
                            // Divisible by every JS8 slot length; poll before
                            // TX can start so the exact reply can be inspected.
                            let t0 = 1_700_000_040;
                            tx.send((t0 - speed.slot_s() as i64, vec![decode])).expect("delivered");
                            c.poll(at(t0 as f64), 7_078_000.0);
                            let answer =
                                auto && acknowledge && speed.allows_heartbeat() && from != "N0JDS";
                            assert_eq!(c.messages.len(), 1);
                            assert_eq!(c.messages[0].to, "@ALLCALL");
                            assert!(c.messages[0].to_me);
                            assert_eq!(
                                c.tx_frames.len(),
                                usize::from(answer),
                                "grid={grid:?}, speed={speed:?}, auto={auto}, ack={acknowledge}, from={from}"
                            );
                            if answer {
                                let queued = c.tx_frames.front().expect("reply queued");
                                assert_eq!(queued.band, TxBand::Beacon);
                                let reply = Directed::unpack(&queued.payload).expect("directed");
                                assert_eq!(reply.from, "N0JDS");
                                assert_eq!(reply.to, from);
                                assert_eq!(frame::cmd_text(reply.cmd), Some(" HEARTBEAT SNR"));
                                assert_eq!(reply.num, Some(-11));
                            }
                            let actions = c.poll(at(t0 as f64 + 1.0), 7_078_000.0);
                            assert_eq!(
                                actions.iter().any(|a| matches!(a, DigiAction::KeyTx)),
                                answer
                            );
                            assert_eq!(c.tx_burst_active(), answer);
                            c.abort_tx();
                        }
                    }
                }
            }
        }
    }

    /// A heartbeat message, as the assembler hands one over.
    fn heartbeat_from(call: &str, snr: i16) -> Js8Msg {
        Js8Msg {
            from: call.into(),
            to: "@ALLCALL".into(),
            text: "EM73".into(),
            cmd: Some("HB".into()),
            snr_db: snr,
            audio_hz: 1500.0,
            first_slot_utc: 1000,
            last_slot_utc: 1000,
            frames: 1,
            complete: true,
            to_me: true,
            speed: Js8Speed::Normal,
        }
    }

    #[test]
    fn switching_the_beacon_on_does_not_key_the_radio_immediately() {
        // The first beacon is a whole interval away. Anything else means
        // choosing an interval transmits before the operator can change it.
        let mut c = Js8Controller::new(DigiConfig { js8_heartbeat_min: 10, ..cfg() }, 48_000.0);
        let t0 = 1_700_000_000;
        c.tick_heartbeat(t0);
        assert!(c.tx_frames.is_empty(), "beaconed the moment it was enabled");
        assert!(c.next_hb_unix >= t0 + 600, "scheduled sooner than the interval");

        // …and nothing goes out until it is due.
        c.tick_heartbeat(t0 + 599);
        assert!(c.tx_frames.is_empty());
        c.tick_heartbeat(c.next_hb_unix);
        assert_eq!(c.tx_frames.len(), 1, "the beacon never fired");
        let hb = Compound::unpack(&c.tx_frames[0].payload).expect("a heartbeat frame");
        assert_eq!(hb.call, "N0JDS");
        assert_eq!(hb.grid().as_deref(), Some("FN42"));
        assert!(!hb.is_cq());
    }

    #[test]
    fn the_countdown_the_panel_shows_tracks_the_next_beacon() {
        let mut c = Js8Controller::new(DigiConfig { js8_heartbeat_min: 10, ..cfg() }, 48_000.0);
        let t0 = 1_700_000_000;
        c.tick_heartbeat(t0);
        let first = c.hb_countdown_s.expect("scheduled");
        c.tick_heartbeat(t0 + 60);
        assert_eq!(c.hb_countdown_s, Some(first - 60));

        // Zero means off, and the panel has nothing to count down to.
        c.cfg.js8_heartbeat_min = 0;
        c.tick_heartbeat(t0 + 61);
        assert_eq!(c.hb_countdown_s, None);
        assert_eq!(c.next_hb_unix, 0, "a stale schedule would fire on re-enabling");
    }

    #[test]
    fn beacons_do_not_all_land_in_the_same_slot() {
        // Stations that share an interval and started together would otherwise
        // collide on every beacon, which is why upstream jitters too.
        let jitters = |call: &str| {
            let mut c = Js8Controller::new(DigiConfig { my_call: call.into(), ..cfg() }, 48_000.0);
            (0..24)
                .map(|i| {
                    c.hb_count = i;
                    c.hb_jitter_s()
                })
                .collect::<Vec<_>>()
        };
        let mine = jitters("N0JDS");
        assert!(mine.iter().any(|&j| j > 0), "every beacon landed on the same phase");
        assert!(mine.contains(&0), "every beacon was pushed late");
        assert!(mine.iter().all(|&j| j == 0 || j == 15), "jitter is one slot or none: {mine:?}");
        assert_ne!(mine, jitters("KN4CRD"), "two stations jitter identically");
    }

    #[test]
    fn turbo_does_not_beacon_at_all() {
        // 160 Hz and six seconds a slot, for local and VHF work: an unattended
        // beacon there costs a lot of a small band to reach nobody far away.
        let mut c = Js8Controller::new(
            DigiConfig { js8_heartbeat_min: 10, js8_speed: Js8Speed::Turbo, ..cfg() },
            48_000.0,
        );
        let t0 = 1_700_000_000;
        c.tick_heartbeat(t0);
        c.tick_heartbeat(t0 + 6000);
        assert!(c.tx_frames.is_empty(), "Turbo beaconed");
        assert_eq!(c.hb_countdown_s, None);
        // Nor does it acknowledge one.
        c.cfg.js8_hb_ack = true;
        assert!(!c.hb_ack_allowed(&heartbeat_from("KN4CRD", -7), t0));
    }

    #[test]
    fn a_beacon_lands_in_the_heartbeat_sub_band() {
        let c = Js8Controller::new(cfg(), 48_000.0);
        // Every slot index has to land on the grid, inside the band, with room
        // for the whole signal.
        for idx in 0..40 {
            let f = c.free_beacon_hz(1_700_000_000, idx);
            assert!((HB_BAND_LO_HZ..=HB_BAND_HI_HZ).contains(&f), "{f} Hz is outside the sub-band");
            assert!(f + c.speed.bandwidth_hz() <= HB_BAND_HI_HZ, "{f} Hz spills past the top");
            assert_eq!((f - HB_BAND_LO_HZ) % HB_SLOT_HZ, 0.0, "{f} Hz is off the grid");
        }
        // Normal is 50 Hz wide on a 50 Hz grid, so this is upstream's ten
        // slots exactly: 500, 550, … 950.
        let seen: std::collections::BTreeSet<i32> =
            (0..400).map(|i| c.free_beacon_hz(1_700_000_000, i) as i32).collect();
        assert_eq!(seen.len(), 10, "the quiet band was not spread across: {seen:?}");
        assert_eq!(*seen.first().expect("nonempty"), 500);
        assert_eq!(*seen.last().expect("nonempty"), 950);
    }

    #[test]
    fn a_beacon_avoids_a_slot_somebody_is_using() {
        let mut c = Js8Controller::new(cfg(), 48_000.0);
        let now = 1_700_000_000;
        // Everything busy but the two slots around 800 Hz.
        c.activity = (0..10)
            .map(|i| HB_BAND_LO_HZ + i as f32 * HB_SLOT_HZ)
            .filter(|f| !(775.0..=825.0).contains(f))
            .map(|f| (f, now))
            .collect();
        for idx in 0..40 {
            let f = c.free_beacon_hz(now, idx);
            assert_eq!(f, 800.0, "landed on a busy slot at index {idx}");
        }
        // Activity that has gone quiet stops holding its slot.
        let seen: std::collections::BTreeSet<i32> =
            (0..400).map(|i| c.free_beacon_hz(now + c.activity_window_s(), i) as i32).collect();
        assert_eq!(seen.len(), 10, "a stale decode still held its frequency");
    }

    #[test]
    fn slow_remembers_a_station_between_its_transmissions() {
        // Upstream's flat thirty seconds is two slots at Normal but less than
        // one at Slow, where it would forget a station between transmissions
        // and beacon straight on top of it.
        let normal = Js8Controller::new(cfg(), 48_000.0);
        assert_eq!(normal.activity_window_s(), ACTIVITY_WINDOW_S, "Normal is upstream's number");

        let mut slow =
            Js8Controller::new(DigiConfig { js8_speed: Js8Speed::Slow, ..cfg() }, 48_000.0);
        assert!(slow.activity_window_s() >= 60, "a Slow slot outlives the window");
        let now = 1_700_000_000;
        // A station that transmitted one Slow slot ago still holds its slot.
        slow.activity = vec![(700.0, now - 30)];
        for idx in 0..40 {
            let f = slow.free_beacon_hz(now, idx);
            assert!((f - 700.0).abs() >= HB_SLOT_HZ, "{f} Hz landed on a station heard 30 s ago");
        }
    }

    #[test]
    fn a_full_sub_band_takes_the_slot_that_has_been_quiet_longest() {
        // Upstream falls back to the bottom of the band, where every station in
        // this position piles up together. The stalest slot is a better answer
        // and the same one when the band is empty.
        let mut c = Js8Controller::new(cfg(), 48_000.0);
        let now = 1_700_000_000;
        c.activity =
            (0..10).map(|i| (HB_BAND_LO_HZ + i as f32 * HB_SLOT_HZ, now - i as i64)).collect();
        assert_eq!(c.free_beacon_hz(now, 0), 950.0, "did not take the stalest slot");
    }

    #[test]
    fn a_wide_speed_keeps_its_whole_signal_clear() {
        // Fast is 80 Hz wide. Upstream measures clearance at a fixed 50 Hz and
        // would place it half on top of a signal it had just cleared.
        let mut c = Js8Controller::new(DigiConfig { js8_speed: Js8Speed::Fast, ..cfg() }, 48_000.0);
        let now = 1_700_000_000;
        c.activity = vec![(600.0, now)];
        for idx in 0..40 {
            let f = c.free_beacon_hz(now, idx);
            assert!((f - 600.0).abs() >= 80.0, "{f} Hz sits within 80 Hz of a live signal");
            assert!(f + 80.0 <= HB_BAND_HI_HZ, "{f} Hz spills past the top of the band");
        }
    }

    #[test]
    fn two_stations_do_not_choose_the_same_quiet_slot_every_time() {
        let mine = Js8Controller::new(cfg(), 48_000.0);
        let theirs = Js8Controller::new(DigiConfig { my_call: "KN4CRD".into(), ..cfg() }, 48_000.0);
        let now = 1_700_000_000;
        let same = (0..40)
            .filter(|&i| mine.free_beacon_hz(now, i) == theirs.free_beacon_hz(now, i))
            .count();
        assert!(same < 30, "two stations picked the same slot {same}/40 times");
    }

    #[test]
    fn a_beacon_goes_to_the_sub_band_and_a_message_does_not() {
        // The queue has to remember which is which: a heartbeat moves, and the
        // sentence the operator typed stays where they are working.
        let mut c = Js8Controller::new(cfg(), 48_000.0);
        c.send_text("HB".into());
        assert_eq!(c.tx_frames[0].band, TxBand::Beacon);
        c.send_text("KN4CRD GOOD EVENING".into());
        assert!(c.tx_frames.iter().all(|f| f.band == TxBand::Working));
        c.abort_tx();

        // …and the operator can pin beacons to the working frequency.
        c.cfg.js8_hb_anywhere = true;
        c.send_text("HB".into());
        assert_eq!(c.tx_frames[0].band, TxBand::Beacon, "still tagged, resolved at transmit");
    }

    #[test]
    fn a_heartbeat_is_acknowledged_once_and_then_not_again_for_a_while() {
        let mut c = Js8Controller::new(DigiConfig { js8_hb_ack: true, ..cfg() }, 48_000.0);
        let t0 = 1_700_000_000;
        let msg = heartbeat_from("KN4CRD", -7);
        assert!(c.hb_ack_allowed(&msg, t0));
        c.note_hb_ack("KN4CRD", t0);
        assert!(!c.hb_ack_allowed(&msg, t0 + HB_ACK_COOLDOWN_S - 1), "answered twice in a row");
        assert!(c.hb_ack_allowed(&msg, t0 + HB_ACK_COOLDOWN_S));
        // The cooldown is per station, not a global gag.
        assert!(c.hb_ack_allowed(&heartbeat_from("VK3ABC", -3), t0 + 1));
    }

    #[test]
    fn acknowledging_waits_for_a_clear_moment() {
        let mut c = Js8Controller::new(DigiConfig { js8_hb_ack: true, ..cfg() }, 48_000.0);
        let t0 = 1_700_000_000;
        let msg = heartbeat_from("KN4CRD", -7);
        assert!(c.hb_ack_allowed(&msg, t0));

        // Not while we already have something to send: ours would join the back
        // of that queue and go out carrying a stale report.
        c.send_text("KN4CRD A LONG MESSAGE THAT TAKES SEVERAL FRAMES TO GET OUT".into());
        assert!(!c.hb_ack_allowed(&msg, t0));
        c.abort_tx();

        // Nor part-way through somebody else's message.
        let (f, _) = crate::js8::frame::pack_data("HELLO ").expect("packs");
        c.assembler.push(
            &Js8Decode {
                payload: Js8Payload { bits: f.bits, frame_type: Js8Flags::FIRST },
                audio_hz: 2000.0,
                dt_sec: 0.0,
                snr_db: -5.0,
                sync_score: 3.0,
                hard_errors: 0,
                speed: Js8Speed::Normal,
            },
            t0,
        );
        assert!(!c.hb_ack_allowed(&msg, t0), "keyed up mid-message");
    }

    #[test]
    fn the_acknowledgement_carries_the_report_and_nothing_else() {
        let mut c = Js8Controller::new(DigiConfig { js8_hb_ack: true, ..cfg() }, 48_000.0);
        let msg = heartbeat_from("KN4CRD", -11);
        let policy = ReplyPolicy { auto_reply: true, hb_ack: true };
        let reply = auto_reply(&msg, &c.station(), policy).expect("acknowledged");
        let frames = c.frames_for_reply(reply);
        assert_eq!(frames.len(), 1, "an acknowledgement is one frame");
        let d = Directed::unpack(&frames[0]).expect("a directed frame");
        assert_eq!(frame::cmd_text(d.cmd), Some(" HEARTBEAT SNR"));
        assert_eq!(d.to, "KN4CRD");
        assert_eq!(d.from, "N0JDS");
        assert_eq!(d.num, Some(-11));
        assert!(Js8Flags(frames[0].frame_type).is_first());
        assert!(Js8Flags(frames[0].frame_type).is_last());
        c.abort_tx();
    }

    #[test]
    fn acknowledging_is_off_until_the_operator_asks_for_it() {
        let c = Js8Controller::new(cfg(), 48_000.0);
        assert!(!c.cfg.js8_hb_ack, "a station must not start beaconing back unasked");
        assert!(!c.hb_ack_allowed(&heartbeat_from("KN4CRD", -7), 1_700_000_000));
    }

    #[test]
    fn an_idle_controller_transmits_silence_and_says_it_is_done() {
        let mut c = Js8Controller::new(cfg(), 48_000.0);
        let mut out = [1.0f32; 64];
        assert!(c.fill_tx_block(&mut out), "nothing queued means done");
        assert!(out.iter().all(|&s| s == 0.0), "and silence, not stale audio");
    }

    // ---- The message-level transmit queue -------------------------------

    #[test]
    fn typing_a_message_keeps_an_answer_already_queued() {
        // Before, SEND replaced the whole queue, so a reply to someone's
        // SNR? vanished the moment the operator typed anything.
        let mut c = Js8Controller::new(cfg(), 48_000.0);
        let answer = c.frames_for_text("KN4CRD SNR -7");
        c.enqueue(TxOrigin::Auto, TxBand::Working, answer);
        c.send_text("VK3ABC GOOD EVENING".into());
        assert_eq!(c.tx_frames[0].origin, TxOrigin::Auto, "the answer still goes first");
        assert!(c.tx_frames.iter().any(|f| f.origin == TxOrigin::Operator));
    }

    #[test]
    fn a_second_send_replaces_the_first_until_it_starts() {
        let mut c = Js8Controller::new(cfg(), 48_000.0);
        c.send_text("KN4CRD FIRST THOUGHT".into());
        c.send_text("KN4CRD SECOND THOUGHT".into());
        assert_eq!(reassemble_frames(&c), "SECOND THOUGHT");
    }

    #[test]
    fn a_message_on_the_air_is_not_cut_in_half() {
        let mut c = Js8Controller::new(cfg(), 48_000.0);
        c.send_text(
            "KN4CRD A LONG MESSAGE THAT NEEDS SEVERAL FRAMES TO GET ALL THE WAY OUT".into(),
        );
        let total = c.tx_frames.len();
        assert!(total > 2);
        // Its first frame goes out…
        let first = c.tx_frames.pop_front().expect("queued");
        c.on_air_msg = Some(first.msg);
        c.keyed = true;
        // …and the operator types again.
        c.send_text("VK3ABC HELLO".into());
        let rest: Vec<u32> = c.tx_frames.iter().map(|f| f.msg).collect();
        assert_eq!(rest.iter().filter(|&&m| m == first.msg).count(), total - 1, "frames lost");
        let split = rest.iter().position(|&m| m != first.msg).expect("the new message is queued");
        assert!(rest[split..].iter().all(|&m| m != first.msg), "spliced into the middle");
    }

    /// The text a receiver makes of everything queued.
    fn reassemble_frames(c: &Js8Controller) -> String {
        let frames: Vec<Js8Payload> = c.tx_frames.iter().map(|f| f.payload).collect();
        reassemble(&frames).trim().to_string()
    }

    // ---- Group auto-responses -------------------------------------------

    fn rules_cfg(rule: sdroxide_types::Js8AutoRule) -> DigiConfig {
        DigiConfig { js8_auto_rules_on: true, js8_auto_rules: vec![rule], ..cfg() }
    }

    fn qrv_rule() -> sdroxide_types::Js8AutoRule {
        sdroxide_types::Js8AutoRule {
            name: "check-in".into(),
            group: "@EMCOMM".into(),
            contains: "QRV?".into(),
            reply: "QRV @MYGRID".into(),
            repeat: 3,
            gap_min: 2,
            cooldown_min: 30,
            ..Default::default()
        }
    }

    fn group_msg(from: &str, group: &str, text: &str) -> Js8Msg {
        Js8Msg {
            from: from.into(),
            to: group.into(),
            text: text.into(),
            cmd: None,
            snr_db: -9,
            audio_hz: 1200.0,
            first_slot_utc: 1000,
            last_slot_utc: 1015,
            frames: 2,
            complete: true,
            to_me: false,
            speed: Js8Speed::Normal,
        }
    }

    /// Pretend the queue went out.
    fn transmit_all(c: &mut Js8Controller) {
        c.tx_frames.clear();
        c.on_air_msg = None;
    }

    #[test]
    fn a_rule_answers_a_group_message_directed_to_the_sender() {
        let mut c = Js8Controller::new(rules_cfg(qrv_rule()), 48_000.0);
        let t0 = 1_700_000_000;
        c.trigger_auto_rule(&group_msg("KN4CRD", "@EMCOMM", "ANY STATIONS QRV?"), t0);
        c.tick_auto_rules(t0);
        assert!(!c.tx_frames.is_empty(), "nothing queued");
        assert!(c.tx_frames.iter().all(|f| f.origin == TxOrigin::Rule));
        let d = Directed::unpack(&c.tx_frames[0].payload).expect("a directed frame");
        assert_eq!(d.to, "KN4CRD");
        assert_eq!(d.from, "N0JDS");
        let text = reassemble_frames(&c);
        assert_eq!(text, "QRV FN42");
        // The panel sees it running.
        let st = c.status().js8.expect("js8 status");
        assert_eq!(st.auto_pending.len(), 1);
        assert_eq!((st.auto_pending[0].sent, st.auto_pending[0].total), (1, 3));
        assert_eq!(st.auto_pending[0].trigger, "KN4CRD");
    }

    #[test]
    fn a_rule_repeats_on_its_gap_and_then_stops() {
        let mut c = Js8Controller::new(rules_cfg(qrv_rule()), 48_000.0);
        let t0 = 1_700_000_000;
        c.trigger_auto_rule(&group_msg("KN4CRD", "@EMCOMM", "QRV?"), t0);
        let mut sends = Vec::new();
        for t in (t0..t0 + 15 * 60).step_by(15) {
            c.tick_auto_rules(t);
            if !c.tx_frames.is_empty() {
                sends.push(t - t0);
                transmit_all(&mut c);
            }
        }
        assert_eq!(sends, vec![0, 120, 240], "three sends, two minutes apart");
        assert!(c.auto_pending.is_empty(), "finished");
    }

    #[test]
    fn a_repeat_waits_for_its_previous_send_to_go() {
        let mut c = Js8Controller::new(rules_cfg(qrv_rule()), 48_000.0);
        let t0 = 1_700_000_000;
        c.trigger_auto_rule(&group_msg("KN4CRD", "@EMCOMM", "QRV?"), t0);
        c.tick_auto_rules(t0);
        let queued = c.tx_frames.len();
        // Still not transmitted when the next repeat is due.
        c.tick_auto_rules(t0 + 120);
        assert_eq!(c.tx_frames.len(), queued, "repeats piled up behind each other");
        transmit_all(&mut c);
        c.tick_auto_rules(t0 + 121);
        assert!(!c.tx_frames.is_empty(), "the repeat went missing");
    }

    #[test]
    fn the_same_station_cannot_retrigger_a_rule_inside_its_cooldown() {
        let one = sdroxide_types::Js8AutoRule { repeat: 1, ..qrv_rule() };
        let mut c = Js8Controller::new(rules_cfg(one), 48_000.0);
        let t0 = 1_700_000_000;
        let m = group_msg("KN4CRD", "@EMCOMM", "QRV?");
        c.trigger_auto_rule(&m, t0);
        c.tick_auto_rules(t0);
        transmit_all(&mut c);
        c.trigger_auto_rule(&m, t0 + 29 * 60);
        assert!(c.auto_pending.is_empty(), "answered twice inside the cooldown");
        // Another station is not gagged by it.
        c.trigger_auto_rule(&group_msg("VK3ABC", "@EMCOMM", "QRV?"), t0 + 60);
        assert_eq!(c.auto_pending.len(), 1);
        c.auto_pending.clear();
        // And after the cooldown, the first station is answered again.
        c.trigger_auto_rule(&m, t0 + 30 * 60);
        assert_eq!(c.auto_pending.len(), 1);
    }

    #[test]
    fn abort_or_a_typed_message_cancels_the_repeats() {
        let t0 = 1_700_000_000;
        let mut c = Js8Controller::new(rules_cfg(qrv_rule()), 48_000.0);
        c.trigger_auto_rule(&group_msg("KN4CRD", "@EMCOMM", "QRV?"), t0);
        c.tick_auto_rules(t0);
        c.abort_tx();
        assert!(c.auto_pending.is_empty() && c.tx_frames.is_empty(), "abort left a repeat");

        let mut c = Js8Controller::new(rules_cfg(qrv_rule()), 48_000.0);
        c.trigger_auto_rule(&group_msg("KN4CRD", "@EMCOMM", "QRV?"), t0);
        c.tick_auto_rules(t0);
        c.send_text("KN4CRD THANKS, I HAVE YOU".into());
        assert!(c.auto_pending.is_empty(), "the canned reply kept repeating");
        assert!(
            c.tx_frames.iter().all(|f| f.origin == TxOrigin::Operator),
            "the unsent canned reply should have been replaced"
        );
    }

    #[test]
    fn rules_stay_silent_when_switched_off_or_when_it_is_us() {
        let t0 = 1_700_000_000;
        let off = DigiConfig { js8_auto_rules_on: false, ..rules_cfg(qrv_rule()) };
        let mut c = Js8Controller::new(off, 48_000.0);
        c.trigger_auto_rule(&group_msg("KN4CRD", "@EMCOMM", "QRV?"), t0);
        assert!(c.auto_pending.is_empty(), "the master switch is off");

        let mut c = Js8Controller::new(rules_cfg(qrv_rule()), 48_000.0);
        c.trigger_auto_rule(&group_msg("N0JDS", "@EMCOMM", "QRV?"), t0);
        assert!(c.auto_pending.is_empty(), "answered ourselves");
        c.trigger_auto_rule(&group_msg("KN4CRD", "@ARES", "QRV?"), t0);
        assert!(c.auto_pending.is_empty(), "answered another group");
    }

    #[test]
    fn changing_the_rules_drops_what_they_had_scheduled() {
        let t0 = 1_700_000_000;
        let mut c = Js8Controller::new(rules_cfg(qrv_rule()), 48_000.0);
        c.trigger_auto_rule(&group_msg("KN4CRD", "@EMCOMM", "QRV?"), t0);
        assert_eq!(c.auto_pending.len(), 1);
        let edited = sdroxide_types::Js8AutoRule { reply: "QRT".into(), ..qrv_rule() };
        c.set_config(rules_cfg(edited));
        assert!(c.auto_pending.is_empty());
        // An unrelated setting does not.
        c.trigger_auto_rule(&group_msg("KN4CRD", "@EMCOMM", "QRV?"), t0);
        let mut same = c.cfg.clone();
        same.js8_status = "AT HOME".into();
        c.set_config(same);
        assert_eq!(c.auto_pending.len(), 1);
    }

    #[test]
    fn the_hourly_cap_holds_however_many_stations_ask() {
        let one = sdroxide_types::Js8AutoRule { repeat: 1, ..qrv_rule() };
        let mut c = Js8Controller::new(rules_cfg(one), 48_000.0);
        let t0 = 1_700_000_000;
        let mut sent = 0;
        for i in 0..30 {
            c.trigger_auto_rule(&group_msg(&format!("K{i}ABC"), "@EMCOMM", "QRV?"), t0 + i);
            c.tick_auto_rules(t0 + i);
            if !c.tx_frames.is_empty() {
                sent += 1;
                transmit_all(&mut c);
            }
        }
        assert_eq!(sent, AUTO_RULE_MAX_PER_HOUR);
        assert_eq!(c.auto_pending.len(), 30 - AUTO_RULE_MAX_PER_HOUR, "the rest wait");
        // An hour later they go.
        c.tick_auto_rules(t0 + 3600 + 30);
        assert!(!c.tx_frames.is_empty());
    }

    #[test]
    fn a_reply_to_a_custom_group_names_both_ends() {
        // @MAGNET is not a reserved group, so the reply needs the compound
        // frame pair. A reply to the group must still say who it is from
        // and who it is for.
        let rule = sdroxide_types::Js8AutoRule {
            group: "@MAGNET".into(),
            reply_to: sdroxide_types::Js8AutoReplyTo::Group,
            contains: "ROLL CALL".into(),
            reply: "@MYCALL HERE".into(),
            repeat: 1,
            ..qrv_rule()
        };
        let mut c = Js8Controller::new(rules_cfg(rule), 48_000.0);
        let t0 = 1_700_000_000;
        c.trigger_auto_rule(&group_msg("KN4CRD", "@MAGNET", "ROLL CALL"), t0);
        c.tick_auto_rules(t0);
        let frames: Vec<Js8Payload> = c.tx_frames.iter().map(|f| f.payload).collect();
        let mut a = Js8Assembler::new("KN4CRD");
        let mut got = None;
        for (i, f) in frames.iter().enumerate() {
            let d = Js8Decode {
                payload: *f,
                audio_hz: 1500.0,
                dt_sec: 0.0,
                snr_db: 0.0,
                sync_score: 3.0,
                hard_errors: 0,
                speed: Js8Speed::Normal,
            };
            got = a.push(&d, t0 + 15 * i as i64);
        }
        let msg = got.expect("the reply completes");
        assert_eq!((msg.from.as_str(), msg.to.as_str()), ("N0JDS", "@MAGNET"));
        assert_eq!(msg.text.trim(), "N0JDS HERE");
    }

    #[test]
    fn what_we_last_said_is_remembered_for_agn() {
        let mut c = Js8Controller::new(cfg(), 48_000.0);
        c.now_unix = 5000;
        c.send_text("kn4crd qth is boston".into());
        let last = c.last_tx.clone().expect("remembered");
        assert_eq!((last.to.as_str(), last.unix), ("KN4CRD", 5000));
        assert_eq!(last.text, "KN4CRD QTH IS BOSTON");
        c.send_text("HELLO ALL".into());
        assert_eq!(c.last_tx.as_ref().map(|l| l.to.as_str()), Some(""), "undirected text");
    }

    /// The whole path, over the air: KN4CRD's transmitter makes the audio for
    /// "@EMCOMM ANY STATIONS QRV?", and our receiver must do the rest:
    /// * decode it slot by slot;
    /// * reassemble the frames into one message;
    /// * match the rule;
    /// * key the radio with the reply in the next slot.
    ///
    /// No step is called by hand.
    #[test]
    fn a_group_message_heard_on_the_air_is_answered_on_the_air() {
        use crate::DigiEngine;
        let mut rx = Js8Controller::new(rules_cfg(qrv_rule()), 48_000.0);
        let them = Js8Controller::new(DigiConfig { my_call: "KN4CRD".into(), ..cfg() }, 48_000.0);
        let frames = them.frames_for_text("@EMCOMM ANY STATIONS QRV?");
        assert!(frames.len() >= 2, "a group message is more than one frame");

        // Slot-aligned, so slot k starts at t0 + 15k.
        let t0 = 1_700_000_010.0;
        let slot = 15.0;
        let rate = 48_000.0;
        let chunk = 4_800; // 0.1 s
        rx.poll(at(t0 - 0.05), 14_078_000.0); // arms the first boundary

        let mut keyed = false;
        let mut text_seen = None;
        for k in 0..frames.len() + 1 {
            // This slot's audio: half a second of silence, the burst, then
            // silence to the end of the slot. The slot after the last frame
            // carries nothing, and is where the reply should go out.
            let mut audio = vec![0.0f32; (slot * rate) as usize];
            if let Some(f) = frames.get(k) {
                let burst = them.synth_burst_48k(*f, 1200.0).expect("synth").samples;
                let start = (0.5 * rate) as usize;
                for (i, s) in burst.iter().enumerate() {
                    if let Some(a) = audio.get_mut(start + i) {
                        *a = *s * 0.3;
                    }
                }
            }
            for (j, block) in audio.chunks(chunk).enumerate() {
                rx.on_rx_audio(block);
                let now = t0 + slot * k as f64 + (j * chunk) as f64 / rate;
                for a in rx.poll(at(now), 14_078_000.0) {
                    if matches!(a, DigiAction::KeyTx) {
                        keyed = true;
                    }
                }
                // Decoding runs on its own thread in real time, while this
                // clock is simulated. Hold the clock still just after the
                // boundary until the worker has finished the slot, so a slow
                // machine cannot push the reply past this slot's TX window.
                if j == 0 && k > 0 {
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
                    while rx.jobs_queued.load(Ordering::Relaxed) > 0
                        && std::time::Instant::now() < deadline
                    {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    // The worker counts the job down just before it sends
                    // the result.
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
            if text_seen.is_none() {
                text_seen = rx
                    .messages
                    .iter()
                    .find(|m| m.to == "@EMCOMM")
                    .map(|m| (m.from.clone(), m.text.clone()));
            }
        }
        let (from, text) = text_seen.expect("the group message was never reassembled");
        assert_eq!(from, "KN4CRD");
        assert!(text.contains("QRV?"), "got {text:?}");
        assert!(keyed, "the reply never keyed the radio");
        // What went out is the canned reply, directed back to KN4CRD.
        let sent = rx.burst.is_some() || !rx.tx_frames.is_empty() || rx.keyed;
        assert!(sent, "nothing on the air or queued");
        let pending = rx.status().js8.expect("status").auto_pending;
        assert_eq!(pending.len(), 1, "two repeats still to go");
        assert_eq!(pending[0].text, "KN4CRD QRV FN42");
        assert_eq!(pending[0].sent, 1);
    }
}
