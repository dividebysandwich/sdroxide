//! Deciding what to say, and when.
//!
//! The announcer is a pure state machine: events in, utterances out, no clock
//! of its own and no audio. It is driven from the UI's event drain, and every
//! entry point takes `now` as egui's monotonic frame time — which is what lets
//! a test push forty snapshots thirty milliseconds apart and assert that
//! exactly one thing was said.
//!
//! # Where the events come from
//!
//! Every change to the radio, from every source — the UI, the scroll wheel, the
//! keyboard, a MIDI surface, the physical knob on a CAT rig, rigctld, TCI, the
//! scanner, a band-stack recall — converges on one `RadioEvent::State`
//! broadcast. Diffing that catches all of them; diffing commands would miss
//! everything the rig originates.
//!
//! It also has to be *that* event rather than the UI's own copy of the state.
//! The UI mutates its copy optimistically, before the engine confirms, and the
//! engine has the last word: an out-of-range or ham-band-locked tune snaps
//! back. Announcing the optimistic value would read out a frequency the radio
//! refused to go to.

pub mod alert;
pub mod decodes;
pub mod digest;
pub mod ft8;
pub mod settle;
pub mod swr;
pub mod tail;

use sdroxide_types::{
    AlertEvent, Band, Decode, DigiStatus, Meters, Mode, RadioState, SpeechSettings,
};

use crate::queue::{Gag, Key, Priority, Push, SpeechQueue, Utterance};
use crate::sink::{NullSink, SpeechSink};
use crate::text::Speaker;

use decodes::SeenDecodes;
use digest::SpeechDigest;
use ft8::QsoWatch;
use settle::{SETTLE_FREQ_S, SETTLE_LEVEL_S, SETTLE_OFFSET_S, SETTLE_OOB_S, Settle};
use swr::SwrWatch;
use tail::TextPump;

/// The settle latches, one per continuous quantity.
#[derive(Debug)]
struct SettleBank {
    freq: Settle<i64>,
    rit: Settle<i32>,
    xit: Settle<i32>,
    drive: Settle<u8>,
    tune_drive: Settle<u8>,
    mic: Settle<u8>,
    volume: Settle<u8>,
    agc_max: Settle<i16>,
    manual_gain: Settle<i16>,
    squelch: Settle<i16>,
    filter: Settle<(i32, i32)>,
    in_band: Settle<bool>,
}

impl Default for SettleBank {
    fn default() -> Self {
        SettleBank {
            freq: Settle::new(SETTLE_FREQ_S),
            rit: Settle::new(SETTLE_OFFSET_S),
            xit: Settle::new(SETTLE_OFFSET_S),
            drive: Settle::new(SETTLE_LEVEL_S),
            tune_drive: Settle::new(SETTLE_LEVEL_S),
            mic: Settle::new(SETTLE_LEVEL_S),
            volume: Settle::new(SETTLE_LEVEL_S),
            agc_max: Settle::new(SETTLE_LEVEL_S),
            manual_gain: Settle::new(SETTLE_LEVEL_S),
            squelch: Settle::new(SETTLE_LEVEL_S),
            filter: Settle::new(SETTLE_LEVEL_S),
            in_band: Settle::new(SETTLE_OOB_S),
        }
    }
}

impl SettleBank {
    fn observe(&mut self, d: &SpeechDigest, now: f64) {
        self.freq.observe(d.freq_hz, now);
        self.rit.observe(d.rit_hz, now);
        self.xit.observe(d.xit_hz, now);
        self.drive.observe(d.drive_pct, now);
        self.tune_drive.observe(d.tune_drive_pct, now);
        self.mic.observe(d.mic_pct, now);
        self.volume.observe(d.volume_pct, now);
        self.agc_max.observe(d.agc_max_gain_db, now);
        self.manual_gain.observe(d.manual_gain_db, now);
        self.squelch.observe(d.squelch_db, now);
        self.filter.observe((d.filter_lo, d.filter_hi), now);
        self.in_band.observe(d.in_ham_band, now);
    }

    fn seed(&mut self, d: &SpeechDigest) {
        self.freq.seed(d.freq_hz);
        self.rit.seed(d.rit_hz);
        self.xit.seed(d.xit_hz);
        self.drive.seed(d.drive_pct);
        self.tune_drive.seed(d.tune_drive_pct);
        self.mic.seed(d.mic_pct);
        self.volume.seed(d.volume_pct);
        self.agc_max.seed(d.agc_max_gain_db);
        self.manual_gain.seed(d.manual_gain_db);
        self.squelch.seed(d.squelch_db);
        self.filter.seed((d.filter_lo, d.filter_hi));
        self.in_band.seed(d.in_ham_band);
    }

    fn deadline(&self) -> Option<f64> {
        [
            self.freq.deadline(),
            self.rit.deadline(),
            self.xit.deadline(),
            self.drive.deadline(),
            self.tune_drive.deadline(),
            self.mic.deadline(),
            self.volume.deadline(),
            self.agc_max.deadline(),
            self.manual_gain.deadline(),
            self.squelch.deadline(),
            self.filter.deadline(),
            self.in_band.deadline(),
        ]
        .into_iter()
        .flatten()
        .fold(None, |acc: Option<f64>, t| Some(acc.map_or(t, |a| a.min(t))))
    }
}

/// The whole announcement layer, above a [`SpeechSink`].
pub struct Announcer {
    cfg: SpeechSettings,
    sink: Box<dyn SpeechSink + Send>,
    queue: SpeechQueue,
    prev: Option<SpeechDigest>,
    settles: SettleBank,
    swr: SwrWatch,
    text: TextPump,
    seen: SeenDecodes,
    qso: QsoWatch,
    /// Monotonic id handed to the sink with each utterance.
    seq: u64,
    /// The last thing said, for the repeat hotkey.
    last_said: Option<String>,
    /// Whether the receiver is currently being ducked for speech.
    ducking: bool,
}

impl Announcer {
    pub fn new(cfg: SpeechSettings) -> Self {
        Announcer::with_sink(cfg, Box::new(NullSink::default()))
    }

    pub fn with_sink(cfg: SpeechSettings, sink: Box<dyn SpeechSink + Send>) -> Self {
        let mut a = Announcer {
            cfg,
            sink,
            queue: SpeechQueue::default(),
            prev: None,
            settles: SettleBank::default(),
            swr: SwrWatch::new(),
            text: TextPump::new(),
            seen: SeenDecodes::default(),
            qso: QsoWatch::new(),
            seq: 0,
            last_said: None,
            ducking: false,
        };
        a.apply_gag();
        a
    }

    pub fn settings(&self) -> &SpeechSettings {
        &self.cfg
    }

    pub fn set_settings(&mut self, cfg: SpeechSettings) {
        let was = self.cfg.enabled;
        self.cfg = cfg;
        if was && !self.cfg.enabled {
            self.silence();
        }
        self.apply_gag();
    }

    /// Replace the sink — after the operator picks a different voice or output
    /// device, or when speech is switched on for the first time.
    pub fn set_sink(&mut self, sink: Box<dyn SpeechSink + Send>) {
        self.sink.stop();
        self.sink = sink;
        self.queue.clear();
    }

    fn apply_gag(&mut self) {
        let gag = if self.cfg.enabled { Gag::None } else { Gag::Off };
        // Never downgrade a transmit gag from here; `on_state` owns that edge.
        if self.queue.gag() != Gag::Transmitting || gag == Gag::Off {
            self.queue.set_gag(gag);
        }
    }

    fn speaker(&self) -> Speaker<'_> {
        Speaker::new(&self.cfg)
    }

    // ── events in ───────────────────────────────────────────────────────

    /// A full radio-state snapshot.
    pub fn on_state(&mut self, s: &RadioState, now: f64) {
        let d = SpeechDigest::of(s);
        let Some(prev) = self.prev else {
            // First sight, or a reconnect. Adopt everything without saying it:
            // otherwise attaching to a radio makes it recite its entire
            // configuration.
            self.settles.seed(&d);
            self.prev = Some(d);
            return;
        };
        self.prev = Some(d);

        // The transmit gag is an edge, and it is set before anything else can
        // be queued this tick.
        if self.cfg.duck_on_ptt {
            if d.ptt && !prev.ptt {
                self.queue.set_gag(Gag::Transmitting);
            } else if !d.ptt && prev.ptt && self.cfg.enabled {
                self.queue.set_gag(Gag::None);
            }
        }

        let mut out: Vec<Utterance> = Vec::new();
        self.discrete(&prev, &d, s, now, &mut out);
        self.settles.observe(&d, now);
        for u in out {
            self.push(u);
        }
    }

    /// A meter update. `Meters` is not part of `RadioState` — it arrives on its
    /// own cadence — so SWR is folded in here.
    pub fn on_meters(&mut self, m: &Meters, s: &RadioState, now: f64) {
        if !self.cfg.enabled {
            return;
        }
        let mut out = Vec::new();
        let sp = Speaker::new(&self.cfg);
        self.swr.on_meters(
            m,
            s.tx.ptt || s.tx.tune,
            now,
            &sp,
            self.cfg.tune.alarm_swr,
            self.cfg.tune.alarm_always,
            s.tx.tune,
            &mut out,
        );
        for u in out {
            self.push(u);
        }
    }

    /// A spoken audible alert, for a decode that matched an alert rule whose
    /// reply includes the voice.
    ///
    /// Distinct from [`Self::on_ft8`]: that reads out whatever is addressed to
    /// us, while this speaks only the alerts the operator asked to *hear* about
    /// — a new country, a new one on the band, a new grid, a call. It is fed by
    /// the alert runtime, which is why it can fire while this tab is not the
    /// focused one: reaching the operator in another window is the point of an
    /// alert.
    ///
    /// [`Priority::Notable`], not `Alert`: an alert phrase is news, not a
    /// fault, and must not get through the transmit gag — speech reaches the
    /// operator's speakers, and therefore the microphone.
    pub fn on_alert(
        &mut self,
        event: AlertEvent,
        call: &str,
        band: &str,
        country: Option<&str>,
        now: f64,
    ) {
        if !self.cfg.enabled {
            return;
        }
        let sp = self.speaker();
        let text = alert::phrase(event, &sp.callsign(call), band, country);
        self.push(Utterance::new(text, Priority::Notable, now));
    }

    /// Fresh FT8/FT4 decodes.
    pub fn on_ft8(&mut self, decodes: &[Decode], status: &DigiStatus, now: f64) {
        if !self.cfg.enabled {
            return;
        }
        let mut out = Vec::new();
        let sp = Speaker::new(&self.cfg);
        self.seen.ft8(decodes, &status.config, &self.cfg, &sp, now, &mut out);
        for u in out {
            self.push(u);
        }
    }

    /// A digital-mode status snapshot: JS8 and FSQ messages, and the rolling
    /// decoded-text buffer the keyboard modes share.
    pub fn on_digi(&mut self, st: &DigiStatus, s: &RadioState, now: f64) {
        if !self.cfg.enabled {
            return;
        }
        let mut out = Vec::new();
        {
            let sp = Speaker::new(&self.cfg);
            if let Some(js8) = st.js8.as_ref() {
                self.seen.js8(&js8.messages, &self.cfg, &sp, now, &mut out);
            }
            self.seen.fsq(&st.fsq_messages, &self.cfg, &sp, now, &mut out);
            self.qso.observe(st, &self.cfg, &sp, now, &mut out);
        }
        self.pump_text(st, s, now, &mut out);
        for u in out {
            self.push(u);
        }
    }

    /// The engine's own banner text.
    pub fn on_notice(&mut self, notice: Option<&str>, now: f64) {
        if !self.cfg.enabled || !self.cfg.cat.notices {
            return;
        }
        if let Some(text) = notice.map(str::trim).filter(|t| !t.is_empty()) {
            let sp = self.speaker();
            self.push(
                Utterance::new(sp.message(text), Priority::Alert, now)
                    .key(Key::Notice)
                    .interrupting(),
            );
        }
    }

    /// Something went wrong that the operator has to know about now.
    pub fn on_error(&mut self, what: &str, now: f64) {
        if !self.cfg.enabled || !self.cfg.cat.notices {
            return;
        }
        let sp = self.speaker();
        self.push(
            Utterance::new(sp.message(what), Priority::Alert, now).key(Key::Notice).interrupting(),
        );
    }

    /// A memory channel was recalled.
    pub fn on_memory_recall(&mut self, name: &str, now: f64) {
        if !self.cfg.enabled || !self.cfg.cat.memory_scan {
            return;
        }
        let sp = self.speaker();
        let text = format!("memory {}", sp.message(name));
        self.push(Utterance::new(text, Priority::Notable, now).key(Key::Memory));
    }

    /// The mode changed, or the decoder was cleared: re-anchor the read-along
    /// stream so its backlog is skipped rather than read.
    pub fn reset_text(&mut self) {
        self.text.reset();
    }

    /// Forget which decodes have been read. For a reconnect, where the same
    /// messages may arrive again with new identities.
    pub fn reset_decodes(&mut self) {
        self.seen.clear();
        self.qso.reset();
    }

    /// A reconnect, or a fresh engine: re-seed rather than announce the world.
    pub fn reseed(&mut self) {
        self.prev = None;
        self.text.reset();
    }

    // ── hotkeys ─────────────────────────────────────────────────────────

    /// Read the whole state out: what the operator would look at.
    pub fn say_status(&mut self, s: &RadioState, m: Option<&Meters>, now: f64) {
        let sp = Speaker::new(&self.cfg);
        let d = SpeechDigest::of(s);
        let mut parts = vec![
            sp.band(d.band).to_string(),
            sp.freq(s.active_freq_hz()),
            sp.mode(d.mode).to_string(),
        ];
        if d.split {
            parts.push(format!("split, transmit {}", sp.freq(s.tx_freq_hz())));
        }
        parts.push(format!("V F O {}", if d.vfo == sdroxide_types::Vfo::A { "A" } else { "B" }));
        if !d.in_ham_band {
            parts.push("outside the band".into());
        }
        if let Some(swr) = m.and_then(|m| m.tx.as_ref()).and_then(|t| t.swr) {
            parts.push(format!("S W R {}", sp.swr(swr)));
        } else if let Some(m) = m.filter(|_| !d.ptt && !d.tune) {
            parts.push(sp.s_meter(m));
        }
        // The operator asked, so it goes ahead of anything queued and is not
        // coalesced away by a change arriving in the same breath.
        self.push(
            Utterance::new(parts.join(", "), Priority::Alert, now)
                .key(Key::OnDemand)
                .interrupting(),
        );
    }

    /// Say the last thing again.
    pub fn repeat_last(&mut self, now: f64) {
        if let Some(text) = self.last_said.clone() {
            self.push(Utterance::new(text, Priority::Alert, now).key(Key::OnDemand).interrupting());
        }
    }

    /// Stop talking, now, and drop everything waiting.
    pub fn silence(&mut self) {
        self.sink.stop();
        self.queue.clear();
    }

    /// Speak a fixed line, for the settings dialog's TEST button.
    pub fn say_sample(&mut self, now: f64) {
        let sp = Speaker::new(&self.cfg);
        let text =
            format!("{}, {}, {}", sp.band(Band::M20), sp.freq(14_074_000.0), sp.mode(Mode::Usb));
        self.push(Utterance::new(text, Priority::Alert, now).key(Key::OnDemand).interrupting());
    }

    // ── the tick ────────────────────────────────────────────────────────

    /// Run the settle timers, the SWR ticker and the text pump, then hand the
    /// queue to the sink.
    ///
    /// Returns the earliest moment anything could next need to speak. The
    /// caller folds that into its repaint request: the timers only fire when a
    /// frame runs, and the UI idles at a quarter of a second, which would smear
    /// a 600 ms settle across most of a second.
    pub fn tick(&mut self, s: &RadioState, m: Option<&Meters>, now: f64) -> Option<f64> {
        if !self.cfg.enabled {
            return None;
        }
        let mut out: Vec<Utterance> = Vec::new();
        self.settled(s, now, &mut out);
        {
            let sp = Speaker::new(&self.cfg);
            self.swr.tick(s.tx.tune, m, s.tx.tune_drive, now, &sp, &mut out);
        }
        for u in out {
            self.push(u);
        }
        self.drain(now);

        [
            self.settles.deadline(),
            self.swr.deadline(),
            self.text.deadline(),
            self.queue.next_deadline(),
        ]
        .into_iter()
        .flatten()
        .fold(None, |acc: Option<f64>, t| Some(acc.map_or(t, |a: f64| a.min(t))))
    }

    /// Whether the receiver should be attenuated right now.
    pub fn wants_duck(&self) -> bool {
        self.cfg.enabled && self.cfg.duck_rx && (self.sink.busy() || !self.queue.is_empty())
    }

    /// The duck state changed since the last call — the caller sends a command
    /// only on the edge, rather than every frame.
    pub fn take_duck_change(&mut self) -> Option<f32> {
        let want = self.wants_duck();
        if want == self.ducking {
            return None;
        }
        self.ducking = want;
        Some(if want { self.cfg.duck_gain() } else { 1.0 })
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    pub fn stats(&self) -> crate::queue::QueueStats {
        self.queue.stats()
    }

    // ── internals ───────────────────────────────────────────────────────

    fn push(&mut self, u: Utterance) {
        if self.queue.push(u) == Push::InterruptNow {
            self.sink.stop();
            self.queue.finish();
        }
    }

    fn drain(&mut self, now: f64) {
        if self.sink.busy() {
            return;
        }
        self.queue.finish();
        if let Some(u) = self.queue.pop(now, false) {
            self.seq += 1;
            self.last_said = Some(u.text.clone());
            self.sink.speak(&u.text, self.seq);
            self.queue.begin(&u);
        }
    }

    /// Everything that changed discretely between two snapshots.
    ///
    /// Composite phrases come first and consume what they cover: a band change
    /// moves band, frequency and mode at once, and four separate utterances for
    /// one button press is unusable.
    fn discrete(
        &mut self,
        prev: &SpeechDigest,
        d: &SpeechDigest,
        s: &RadioState,
        now: f64,
        out: &mut Vec<Utterance>,
    ) {
        let sp = Speaker::new(&self.cfg);
        let cat = self.cfg.cat;
        let mut said_freq = false;
        let mut said_mode = false;

        // Keying up outside an amateur allocation bypasses the settle path
        // entirely. Being polite for 350 ms here is 350 ms of transmitting
        // where you should not be.
        if (d.ptt && !prev.ptt) || (d.tune && !prev.tune) {
            if !d.in_ham_band && !s.oob_tx && cat.band_edge {
                out.push(
                    Utterance::new("outside the band", Priority::Alert, now)
                        .key(Key::BandEdge)
                        .interrupting(),
                );
            }
            self.settles.in_band.seed(d.in_ham_band);
        }

        if cat.mode_band && d.band != prev.band {
            // A band change is a jump, not a drift: read the new frequency with
            // it rather than 600 ms later, and seed the latch so it does not
            // repeat itself.
            let mut parts = vec![sp.band(d.band).to_string(), sp.freq(s.active_freq_hz())];
            if d.mode != prev.mode || self.cfg.is_verbose() {
                parts.push(sp.mode(d.mode).to_string());
                said_mode = true;
            }
            out.push(Utterance::new(parts.join(", "), Priority::Routine, now).key(Key::Band));
            self.settles.freq.seed(d.freq_hz);
            said_freq = true;
        }

        if cat.vfo_split && d.vfo != prev.vfo && !said_freq {
            let which = if d.vfo == sdroxide_types::Vfo::A { "A" } else { "B" };
            out.push(
                Utterance::new(
                    format!("V F O {which}, {}", sp.freq(s.active_freq_hz())),
                    Priority::Notable,
                    now,
                )
                .key(Key::Vfo),
            );
            self.settles.freq.seed(d.freq_hz);
        }

        if cat.mode_band && d.mode != prev.mode && !said_mode {
            out.push(
                Utterance::new(sp.mode(d.mode).to_string(), Priority::Routine, now).key(Key::Mode),
            );
        }

        if cat.vfo_split && d.split != prev.split {
            let text = if d.split {
                format!("split on, transmit {}", sp.freq(s.tx_freq_hz()))
            } else {
                "split off".to_string()
            };
            out.push(Utterance::new(text, Priority::Notable, now).key(Key::Split));
        }

        if cat.ptt {
            if d.ptt != prev.ptt {
                let text = if d.ptt { "transmit" } else { "receive" };
                out.push(Utterance::new(text, Priority::Notable, now).key(Key::Ptt));
            }
            if d.tune != prev.tune {
                let text = if d.tune { "tune on" } else { "tune off" };
                out.push(Utterance::new(text, Priority::Notable, now).key(Key::Tune));
            }
        }

        if cat.agc_gain && d.agc != prev.agc {
            out.push(Utterance::new(sp.agc(d.agc), Priority::Routine, now).key(Key::Agc));
        }

        if cat.filters {
            if d.nr != prev.nr {
                out.push(Utterance::new(sp.nr(d.nr), Priority::Routine, now).key(Key::Nr));
            }
            if d.noise_blanker != prev.noise_blanker {
                let text = if d.noise_blanker { "noise blanker on" } else { "noise blanker off" };
                out.push(Utterance::new(text, Priority::Routine, now).key(Key::Nb));
            }
            if d.auto_notch != prev.auto_notch {
                let text = if d.auto_notch { "auto notch on" } else { "auto notch off" };
                out.push(Utterance::new(text, Priority::Routine, now).key(Key::Notch));
            }
        }

        if cat.rit_xit {
            if d.rit_on != prev.rit_on {
                let text = if d.rit_on {
                    format!("R I T on, {}", sp.hertz(d.rit_hz))
                } else {
                    "R I T off".to_string()
                };
                out.push(Utterance::new(text, Priority::Routine, now).key(Key::Rit));
                self.settles.rit.seed(d.rit_hz);
            }
            if d.xit_on != prev.xit_on {
                let text = if d.xit_on {
                    format!("X I T on, {}", sp.hertz(d.xit_hz))
                } else {
                    "X I T off".to_string()
                };
                out.push(Utterance::new(text, Priority::Routine, now).key(Key::Xit));
                self.settles.xit.seed(d.xit_hz);
            }
        }

        if cat.vfo_split && d.sub_rx != prev.sub_rx {
            let text = if d.sub_rx { "sub receiver on" } else { "sub receiver off" };
            out.push(Utterance::new(text, Priority::Routine, now).key(Key::SubRx));
        }

        if cat.levels && d.muted != prev.muted {
            let text = if d.muted { "muted" } else { "unmuted" };
            out.push(Utterance::new(text, Priority::Notable, now).key(Key::Volume));
        }

        if cat.memory_scan {
            if d.scan_running != prev.scan_running {
                let text = if d.scan_running { "scanning" } else { "scan stopped" };
                out.push(Utterance::new(text, Priority::Notable, now).key(Key::Scan));
            } else if d.scan_holding && !prev.scan_holding {
                out.push(
                    Utterance::new(
                        format!("holding, {}", sp.freq(s.active_freq_hz())),
                        Priority::Notable,
                        now,
                    )
                    .key(Key::Scan),
                );
                self.settles.freq.seed(d.freq_hz);
            }
        }

        if cat.notices && d.recording != prev.recording {
            let text = if d.recording { "recording started" } else { "recording stopped" };
            out.push(Utterance::new(text, Priority::Notable, now).key(Key::Recording));
        }

        if self.cfg.is_verbose() && d.antenna_rx != prev.antenna_rx && !s.antenna_rx.is_empty() {
            out.push(
                Utterance::new(
                    format!("antenna {}", sp.message(&s.antenna_rx)),
                    Priority::Routine,
                    now,
                )
                .key(Key::Antenna),
            );
        }
    }

    /// Everything whose settle timer came due.
    fn settled(&mut self, s: &RadioState, now: f64, out: &mut Vec<Utterance>) {
        let sp = Speaker::new(&self.cfg);
        let cat = self.cfg.cat;

        if let Some(in_band) = self.settles.in_band.poll(now)
            && cat.band_edge
        {
            let text = if in_band {
                format!("in band, {}", sp.band(Band::containing(s.tx_freq_hz())))
            } else {
                "outside the band".to_string()
            };
            let mut u = Utterance::new(text, Priority::Notable, now).key(Key::BandEdge);
            if !in_band {
                u.priority = Priority::Alert;
                u = u.interrupting();
            }
            out.push(u);
        }

        if let Some(hz) = self.settles.freq.poll(now)
            && cat.frequency
        {
            let mut text = sp.freq(hz as f64);
            // The segment rides on the frequency rather than announcing itself:
            // "data" alone means nothing, and it is what keeps an operator legal.
            if self.cfg.is_verbose() {
                let seg = sp.segment(sdroxide_types::segment_kind_at(s.rx_freq_hz()));
                if !seg.is_empty() {
                    text.push_str(", ");
                    text.push_str(seg);
                }
            }
            // Interrupting: hearing a frequency you have already spun past is
            // worse than silence.
            out.push(Utterance::new(text, Priority::Routine, now).key(Key::Freq).interrupting());
        }

        if cat.rit_xit {
            if let Some(hz) = self.settles.rit.poll(now)
                && s.rit.enabled
            {
                out.push(
                    Utterance::new(format!("R I T {}", sp.hertz(hz)), Priority::Routine, now)
                        .key(Key::Rit),
                );
            }
            if let Some(hz) = self.settles.xit.poll(now)
                && s.xit.enabled
            {
                out.push(
                    Utterance::new(format!("X I T {}", sp.hertz(hz)), Priority::Routine, now)
                        .key(Key::Xit),
                );
            }
        }

        if cat.levels {
            for (val, label, key) in [
                (self.settles.drive.poll(now), "drive", Key::Drive),
                (self.settles.tune_drive.poll(now), "tune level", Key::TuneDrive),
                (self.settles.mic.poll(now), "mic", Key::MicGain),
            ] {
                if let Some(pct) = val {
                    out.push(
                        Utterance::new(
                            format!("{label} {}", sp.percent(pct as f32 / 100.0)),
                            Priority::Routine,
                            now,
                        )
                        .key(key),
                    );
                }
            }
            if let Some(pct) = self.settles.volume.poll(now)
                && self.cfg.is_verbose()
            {
                out.push(
                    Utterance::new(
                        format!("volume {}", sp.percent(pct as f32 / 100.0)),
                        Priority::Routine,
                        now,
                    )
                    .key(Key::Volume),
                );
            }
        } else {
            // Keep the latches in step even when muted, so switching the
            // category on later does not immediately announce a stale change.
            self.settles.drive.poll(now);
            self.settles.tune_drive.poll(now);
            self.settles.mic.poll(now);
            self.settles.volume.poll(now);
        }

        if cat.agc_gain {
            if let Some(db) = self.settles.agc_max.poll(now)
                && self.cfg.is_verbose()
            {
                out.push(
                    Utterance::new(
                        format!("A G C ceiling {}", sp.decibels(db as f32)),
                        Priority::Routine,
                        now,
                    )
                    .key(Key::AgcMaxGain),
                );
            }
            if let Some(db) = self.settles.manual_gain.poll(now)
                && s.rx[0].agc == sdroxide_types::AgcMode::Off
            {
                out.push(
                    Utterance::new(
                        format!("gain {}", sp.decibels(db as f32)),
                        Priority::Routine,
                        now,
                    )
                    .key(Key::ManualGain),
                );
            }
        } else {
            self.settles.agc_max.poll(now);
            self.settles.manual_gain.poll(now);
        }

        if cat.filters {
            if let Some((lo, hi)) = self.settles.filter.poll(now) {
                out.push(
                    Utterance::new(format!("filter {}", sp.hertz(hi - lo)), Priority::Routine, now)
                        .key(Key::Filter),
                );
            }
            if let Some(db) = self.settles.squelch.poll(now)
                && db > sdroxide_types::SQUELCH_OPEN_DB as i16
            {
                out.push(
                    Utterance::new(
                        format!("squelch {}", sp.decibels(db as f32)),
                        Priority::Routine,
                        now,
                    )
                    .key(Key::Squelch),
                );
            }
        } else {
            self.settles.filter.poll(now);
            self.settles.squelch.poll(now);
        }
    }

    /// Feed the read-along stream, if this mode has one and it is switched on.
    fn pump_text(&mut self, st: &DigiStatus, s: &RadioState, now: f64, out: &mut Vec<Utterance>) {
        let mode = s.rx[0].mode;
        let is_cw = mode == Mode::Cw;
        let want = if is_cw { self.cfg.text.cw } else { self.cfg.text.rtty_psk };
        if !want || !mode_has_text(mode) {
            self.text.anchor_to(&st.text_rx);
            return;
        }
        // Your own sidetone feeds the decoder.
        if s.tx.ptt {
            self.text.anchor_to(&st.text_rx);
            return;
        }
        // Reading an unlocked CW decoder's output is worse than silence; when
        // lock returns, skip what it produced while lost rather than read it.
        if is_cw && self.cfg.text.cw_only_when_locked && !st.cw.as_ref().is_some_and(|c| c.locked) {
            self.text.anchor_to(&st.text_rx);
            return;
        }

        let sp = Speaker::new(&self.cfg);
        self.text.pump(
            &st.text_rx,
            now,
            &sp,
            self.cfg.text.max_backlog_chars,
            self.queue.has_pending(Key::Text),
            self.sink.busy(),
            out,
        );
    }
}

/// Whether a mode produces the shared rolling decoded-text buffer.
fn mode_has_text(m: Mode) -> bool {
    matches!(
        m,
        Mode::Cw
            | Mode::Rtty
            | Mode::Psk
            | Mode::Olivia
            | Mode::Mfsk
            | Mode::Contestia
            | Mode::Mt63
            | Mode::Ardop
            | Mode::Thor
            | Mode::Fsq
    )
}
