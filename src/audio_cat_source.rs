//! An [`IqSource`] for a CAT-controlled rig whose audio arrives over a USB
//! sound card: control (frequency/mode/PTT) goes over serial via
//! [`sdroxide_cat`], RX audio comes from the radio's capture device, and TX
//! audio goes to the radio's playback device. Two sound formats are supported:
//! stereo **IQ** (complex baseband → normal engine path) and mono **demod
//! audio** (real → the engine's audio-band bypass, `DeviceCaps.audio_mode`).

use sdroxide_dsp::{IqCorrect, MonoResampler, Nco};
use sdroxide_radio::rtrb;
use sdroxide_radio::{Complex32, ControlUpdate, DC_BLOCK_HZ, IqSource, Result};
use sdroxide_types::{CatConfig, Mode, SoundFormat, TxTelemetry};

use crate::dial::Dial;

pub struct AudioCatSource {
    // RX audio from the rig (mono for demod, interleaved L/R for IQ). `None`
    // when the capture device could not be opened — the app still runs so the
    // user can fix the device in Settings; RX is just silent until then.
    in_stream: Option<sdroxide_audio::AudioInput>,
    in_consumer: rtrb::Consumer<f32>,
    in_rate: f64,
    /// Capture frames the sound card had to throw away, and when that was last
    /// looked at. Watched because dropping them is silent everywhere else: a
    /// spliced I/Q stream still paints a healthy panadapter, and only the audio
    /// — which has to be continuous — gives it away. See [`Self::check_dropped`].
    drops: DropWatch,
    /// Whether the two capture channels are actually in quadrature. A rig set
    /// to send demodulated audio down a stereo cable puts the same signal on
    /// both of them, which is a real signal wearing an I/Q stream's clothes —
    /// see [`QuadratureWatch`].
    quad: QuadratureWatch,
    format: SoundFormat,
    /// What the right channel is multiplied by on the way into the complex
    /// stream: `1.0` normally, `-1.0` when the rig's I and Q are the other way
    /// round (`CatConfig::invert_spectrum`). Conjugating the sample is the
    /// whole of the fix, and folding it into a factor keeps the inner loop the
    /// same shape for both.
    q_sign: f32,
    /// Brings a shifted I.F. back to the dial: an oscillator running at
    /// `+CatConfig::iq_offset_hz`, mixed into every block as it arrives. `None`
    /// when the rig's LO is on its dial, which is the ordinary case and the one
    /// that must cost nothing.
    iq_shift: Option<Nco>,
    /// Undoes what the rig's quadrature mixer and the sound card's two channels
    /// did to the stream: the DC spike in the middle of the span, and the
    /// mirror image of every signal reflected about it
    /// (`CatConfig::iq_correction`, `CatConfig::iq_dc_block_hz`). `None` when
    /// the operator has both off, and on demod audio, which is one real signal
    /// with neither defect to have.
    iq_correct: Option<IqCorrect>,
    audio_bw: f64,

    // TX audio to the rig (interleaved stereo playback ring).
    out: Option<(sdroxide_audio::AudioOutput, rtrb::Producer<f32>)>,
    tx_resampler: Option<MonoResampler>,
    tx_scratch: Vec<f32>,

    cat: sdroxide_cat::CatHandle,
    /// Top of the `27 00` amplitude scale on the rig the model list names —
    /// 160 on the IC-7300 generation, 200 on an IC-7760. Taken at open, since
    /// the model cannot change under a session.
    scope_full_scale: f32,
    /// Whether the CW panel keys this rig as audio (`CwKeying::Audio`), so the
    /// rig must be held on a sideband instead of being put in CW. See
    /// [`IqSource::cw_audio_keyed`].
    cw_mcw: bool,
    dial: Dial,
    /// Whether the rig has ever answered its control port, and so whether its
    /// dial is something this end can move at all. Seeded from the startup
    /// query and latched on by the first thing the rig says afterwards, so a
    /// radio switched on after sdroxide is picked up rather than left out for
    /// the session. See [`IqSource::center_is_dial`].
    dial_reachable: bool,
    label: String,
    /// Warning captured at open time (RX device unavailable / mono-for-IQ),
    /// surfaced to the UI. `None` when RX came up cleanly.
    status: Option<String>,
    /// Latest SWR the rig reported while keyed (via CI-V meter reads), held so
    /// the engine's 100 ms meter poll sees the most recent value between the
    /// rig's ~5 Hz updates. Cleared on unkey.
    last_telem: Option<TxTelemetry>,
    /// Latest S-meter reading (dBm) the rig reported while receiving, and when
    /// it arrived. Held for the same reason as `last_telem` — the engine's
    /// meter ticks far faster than the rig answers, and a gap between answers
    /// is not a signal that went away — but only for as long as it can still be
    /// called current: a rig that has stopped answering must not leave a needle
    /// standing at the last thing it said.
    last_signal: Option<(std::time::Instant, f32)>,
    /// How long the reading above stands in for the next one. Derived from the
    /// configured poll rate rather than fixed, because that rate is what sets
    /// the gap between two honest answers: a window shorter than it would blank
    /// the needle between every pair of them. See `sdroxide_cat::signal_max_age`.
    signal_max_age: std::time::Duration,
    /// Which antenna socket the rig says its receiver is on, for the one family
    /// that has two (an ELAD FDM-DUO's `AN`). Empty on every other rig, where
    /// there is no port to choose and nothing publishes a list to choose from.
    ///
    /// Starts empty rather than at a guess: it is filled in by the rig's own
    /// answer to the read the control port sends as it opens.
    antenna: String,
}

impl AudioCatSource {
    /// Open the radio's sound-card streams and the CAT serial thread. `audio_in`
    /// / `audio_out` are cpal device names (`None` = system default).
    pub fn open(
        cfg: CatConfig,
        audio_in: Option<&str>,
        audio_out: Option<&str>,
    ) -> anyhow::Result<Self> {
        // Adopt the rig's current dial/mode before we start commanding it.
        // Whether anything answered at all is kept too: it is the only evidence
        // there is that there *is* a control link, and the whole shape of
        // tuning hangs on it — see [`IqSource::center_is_dial`].
        let reply = sdroxide_cat::query_once(&cfg);
        let dial_reachable = reply.is_some();
        let (init_freq, _init_mode) = reply.unwrap_or((None, None));
        let center = init_freq.unwrap_or(14_074_000.0);

        // A rig with no sound card named falls back to the machine's default
        // input, which is almost never the radio — it is the operator's headset,
        // or, at a station with two rigs on two identical USB codecs, the *other*
        // radio's card. Worth saying out loud: the symptom is one radio with
        // audio and one without, and nothing else points at the cause.
        if audio_in.is_none() || audio_out.is_none() {
            tracing::warn!(
                "no sound card chosen for the {} rig on {} ({}) — falling back to the system \
                 default, which is not this radio unless it happens to be the default. Pick its \
                 card under Settings → General → Radio audio.",
                cfg.family.label(),
                sdroxide_cat::link_label(&cfg),
                match (audio_in.is_none(), audio_out.is_none()) {
                    (true, true) => "receive and transmit",
                    (true, false) => "receive",
                    _ => "transmit",
                },
            );
        }

        // RX capture is best-effort: a missing/unsupported device leaves RX
        // silent but keeps the app (and its Settings dialog) alive.
        // The I/Q card is opened at whatever rate the operator chose, because on
        // a quadrature rig that rate *is* the panadapter width — the stream
        // spans all of it. Demod audio is fixed at 48 kHz: it arrives already
        // inside the radio's own filter, so a faster card would only digitise
        // more silence either side of it.
        let opened = match cfg.format {
            SoundFormat::Iq => sdroxide_audio::start_input_stereo(audio_in, cfg.iq_rate_hz),
            SoundFormat::DemodAudio => sdroxide_audio::start_input(audio_in, 48_000),
        };
        let dev_label = audio_in.unwrap_or("system default");
        // A dummy, always-empty ring keeps `read` returning silence when RX is
        // unavailable or guarded off.
        let silent = || {
            let (_p, c) = rtrb::RingBuffer::<f32>::new(1);
            c
        };
        let (in_stream, in_consumer, in_rate, status) = match opened {
            // Mono guard: I/Q needs two channels (I on left, Q on right); a
            // mono capture device physically can't carry it. Refuse rather than
            // silently duplicating one channel into a degenerate spectrum.
            Ok((s, _)) if matches!(cfg.format, SoundFormat::Iq) && s.channels < 2 => {
                let msg = format!(
                    "Radio IQ input “{dev_label}” is mono — IQ needs a stereo (2-channel) \
                     input. Pick a stereo line-input device, or switch the sound format to \
                     Demod audio."
                );
                tracing::warn!("{msg}");
                (None, silent(), s.sample_rate, Some(msg))
            }
            Ok((s, c)) => {
                let rate = s.sample_rate;
                (Some(s), c, rate, None)
            }
            Err(e) => {
                let msg = format!(
                    "Radio input “{dev_label}” is unavailable ({e}) — no receive audio. \
                     The device may be in use by another program, unplugged, or held by \
                     the system audio server."
                );
                tracing::warn!("{msg}");
                (None, silent(), 48_000.0, Some(msg))
            }
        };

        // TX playback is best-effort: a missing device just means no TX audio.
        let out = match sdroxide_audio::start_output(audio_out, 48_000) {
            Ok((o, p)) => Some((o, p)),
            Err(e) => {
                tracing::warn!("radio TX audio device unavailable ({e}); RX only");
                None
            }
        };
        // `MonoResampler::new` returns None when the rates match.
        let tx_resampler =
            out.as_ref().and_then(|(o, _)| MonoResampler::new(48_000.0, o.sample_rate));

        let label =
            format!("CAT rig ({}) on {}", cfg.family.label(), sdroxide_cat::link_label(&cfg));
        let audio_bw = cfg.audio_bw_hz;
        let format = cfg.format;
        // Said out loud at open: a mirrored panadapter looks like a working one
        // until you notice signals are on the wrong side of the dial, so which
        // way round this rig is wired belongs in the log next to the rest of
        // what was assumed about it.
        if matches!(format, SoundFormat::Iq) && cfg.invert_spectrum {
            tracing::info!("radio I/Q inverted (Q negated) — spectrum mirrored about the dial");
        }
        let q_sign = if cfg.invert_spectrum { -1.0 } else { 1.0 };
        let shift = matches!(format, SoundFormat::Iq)
            .then_some(cfg.iq_offset_hz)
            .filter(|hz| hz.is_finite() && *hz != 0.0);
        // Said out loud for the same reason as the inversion above: a rig whose
        // I.F. has been shifted and a host that has not been told about it look
        // identical until you notice every signal is the offset away from where
        // it belongs, and this line is the only place the assumption is recorded.
        if let Some(hz) = shift {
            tracing::info!(
                "radio I/Q centre {:.0} Hz from the dial (e.g. Elecraft RX SHFT) — stream \
                 shifted back onto it; the rig's own dial is untouched",
                hz
            );
        }
        let iq_shift = shift.map(|hz| Nco::new(hz, in_rate));
        let iq_correct = build_correction(&cfg, format, in_rate);
        // Said out loud beside the rest of what was assumed at open: an image
        // 35 dB down is a plausible station, and an operator hunting one that
        // vanishes when this is switched on deserves to find the reason in the
        // log rather than in the settings dialog.
        if iq_correct.is_some() {
            tracing::info!(
                "radio I/Q corrected: {}{}",
                if cfg.iq_correction { "mirror image cancelled" } else { "DC only" },
                match cfg.iq_dc_block_hz {
                    hz if hz > 0.0 => format!(", centre high-passed at {hz:.0} Hz"),
                    _ => String::new(),
                }
            );
        }
        // Said out loud with the rest of what was assumed at open, and with the
        // same reasoning: on a quadrature rig the card's rate is the panadapter
        // width, so it belongs in the log beside which way round I and Q are.
        if matches!(format, SoundFormat::Iq) {
            tracing::info!(
                "radio I/Q at {:.0} Hz — {:.0} kHz of spectrum either side of the dial",
                in_rate,
                in_rate / 2000.0,
            );
        }
        // Read before the config goes to the CAT thread: how long an S-meter
        // reading stands in for the next one follows the rate that thread polls
        // at (see `sdroxide_cat::signal_max_age`).
        let signal_max_age = sdroxide_cat::signal_max_age(&cfg);
        // Either sound format: even an IQ-format rig transmits what arrives at
        // its sound card, so MCW rides a sideband there too.
        let cw_mcw = cfg.cw_keying == sdroxide_types::CwKeying::Audio;
        // The scope, asked for on a link its sweeps do not fit down, is
        // declined rather than allowed to bury the polls and the PTT — and
        // that has to be said on screen, because nothing else explains a
        // panadapter that stayed narrow with the box ticked.
        let status = if cfg.scope
            && cfg.family == sdroxide_types::CatFamily::Icom
            && !sdroxide_cat::scope_active(&cfg)
        {
            let note = format!(
                "The rig scope needs a {} baud CI-V link and this one is set to {}. On the \
                 radio set CI-V USB Baud Rate to 115200 and CI-V USB Port to \"Unlink from \
                 [REMOTE]\", then match the baud rate here.",
                sdroxide_types::CAT_SCOPE_MIN_BAUD,
                cfg.serial.baud
            );
            tracing::warn!("{note}");
            Some(match status {
                Some(s) => format!("{s}\n{note}"),
                None => note,
            })
        } else {
            status
        };
        // A rig that answered nothing on its control port. Said on screen
        // rather than only in the log, because it changes what the panadapter
        // does: with no dial to command, the span the radio is already sending
        // is the whole of the receiver (see [`IqSource::center_is_dial`]), and
        // an operator who thinks the link is up is left wondering why the
        // frequency readout no longer agrees with the radio.
        let status = if matches!(format, SoundFormat::Iq) && !dial_reachable {
            let note = format!(
                "No answer from the radio on {} — its dial cannot be commanded from here, so \
                 tuning stays inside the I/Q it is already sending. Set the band on the radio \
                 itself and type its dial frequency here to line the panadapter up. If you do \
                 have a control cable, check the port, baud rate and CAT family under \
                 Settings → Radio.",
                sdroxide_cat::link_label(&cfg),
            );
            tracing::warn!("{note}");
            Some(match status {
                Some(s) => format!("{s}\n{note}"),
                None => note,
            })
        } else {
            status
        };
        // Read off the model before the configuration is handed to the link,
        // which takes it by value.
        let scope_full_scale = f32::from(cfg.icom_model.scope_full_scale());
        let cat = sdroxide_cat::spawn(cfg);

        Ok(AudioCatSource {
            in_stream,
            in_consumer,
            in_rate,
            drops: DropWatch::started(std::time::Instant::now()),
            quad: QuadratureWatch::default(),
            format,
            q_sign,
            iq_shift,
            iq_correct,
            audio_bw,
            out,
            tx_resampler,
            tx_scratch: Vec::new(),
            cat,
            scope_full_scale,
            cw_mcw,
            dial: Dial::at(center),
            dial_reachable,
            label,
            status,
            last_telem: None,
            last_signal: None,
            signal_max_age,
            antenna: String::new(),
        })
    }

    /// The antenna sockets this rig can put its receiver on, for
    /// `DeviceCaps::antennas_rx`. Empty on every family but ELAD.
    pub fn antennas(&self) -> &'static [&'static str] {
        self.cat.antennas()
    }

    /// Report capture frames the sound card dropped since the last look.
    ///
    /// A dropped frame is a hole in the I/Q stream, and the two things reading
    /// it disagree completely about how much that matters. The panadapter does
    /// not care: every block it transforms is still real signal, so a stream
    /// with pieces missing paints a spectrum indistinguishable from an intact
    /// one. The demodulators do: audio has to be continuous, and each hole is a
    /// splice the operator hears as a click, a stutter, or — often enough — as
    /// speech that has simply stopped being intelligible.
    ///
    /// So the failure looks exactly like a DSP fault and is not one, which is
    /// why it is worth a line of its own. It is also the failure that arrives
    /// with the sample rate: the faster the card runs, the less wall-clock time
    /// the machine has to empty it between callbacks, and a rate that a given
    /// machine cannot sustain shows up here and nowhere else.
    ///
    /// What it must not report is an over. Nobody reads this stream while the
    /// rig transmits, so the ring overflows every time regardless of how fast
    /// the machine is — see [`Self::discard_pending_rx`], which is where those
    /// frames are excused.
    fn check_dropped(&mut self) {
        let Some(total) = self.in_stream.as_ref().map(|s| s.dropped_frames()) else { return };
        let Some((lost, window)) = self.drops.check(std::time::Instant::now(), total) else {
            return;
        };
        // Which rate to blame depends on which card this is. An I/Q card runs
        // at whatever rate the operator asked for, and that rate is the thing
        // to lower. Demod audio is opened at 48 kHz and at nothing else, so
        // sending its owner to a rate setting sends them somewhere that cannot
        // help them — and a remedy that does not apply is how an operator
        // learns to stop reading the warnings.
        let remedy = match self.format {
            SoundFormat::Iq => "Try a lower I/Q sample rate under Settings → Radio.",
            SoundFormat::DemodAudio => {
                "This card is fixed at 48 kHz, so there is no rate to lower — look instead at \
                 what else on this machine is keeping sdroxide off the CPU."
            }
        };
        // The window is measured, not assumed: reporting a fixed 5 s after a
        // stall claimed twelve seconds of signal lost in five, and an operator
        // who catches the arithmetic out has no reason to believe the rest.
        tracing::warn!(
            "radio audio: {lost} capture frames dropped in the last {:.1} s ({:.1} ms of signal) \
             — this machine is not emptying a {:.0} Hz card fast enough. The panadapter will \
             look fine and the demodulated audio will break up. {remedy}",
            window.as_secs_f64(),
            lost as f64 * 1000.0 / self.in_rate,
            self.in_rate,
        );
    }
}

/// The capture counter's bookkeeping: what it read last time and when.
///
/// Split out of the source for the same reason [`fill_iq`] is a free function —
/// the counter it watches lives behind a running sound card, and this is
/// arithmetic about wall-clock time and a monotonic total, which is exactly
/// where it went wrong before and exactly what should be checkable without a
/// radio plugged in.
struct DropWatch {
    /// The card's lifetime drop total as of the last look.
    seen: u64,
    /// When that look happened — the time it *did*, not the time the next one
    /// is due. The gap between two looks is not [`DROP_CHECK_INTERVAL`] and
    /// cannot be assumed to be: the check rides on `read`, and `read` is not
    /// called for the length of an over, so the first look after unkey spans
    /// the whole transmission.
    last_check: std::time::Instant,
}

impl DropWatch {
    fn started(now: std::time::Instant) -> Self {
        DropWatch { seen: 0, last_check: now }
    }

    /// Frames lost since the last look and the window they were lost in, or
    /// `None` when it is not yet time to look or nothing was lost.
    fn check(&mut self, now: std::time::Instant, total: u64) -> Option<(u64, std::time::Duration)> {
        let window = now.duration_since(self.last_check);
        if window < DROP_CHECK_INTERVAL {
            return None;
        }
        self.last_check = now;
        let lost = total.saturating_sub(self.seen);
        if lost == 0 {
            return None;
        }
        self.seen = total;
        Some((lost, window))
    }

    /// Forget what the counter accumulated and start the window again from
    /// `now` — for drops that happened while nobody was reading the stream and
    /// so say nothing about whether this machine can keep up with it.
    fn rebase(&mut self, now: std::time::Instant, total: u64) {
        self.seen = total;
        self.last_check = now;
    }
}

/// The soonest [`AudioCatSource::check_dropped`] looks again. Long enough that
/// a struggling machine is not also made to write a log line per block, short
/// enough that the operator sees it while still at the radio. A floor rather
/// than a period: nothing looks at all while the source is not being read.
const DROP_CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

/// How many pairs a [`QuadratureWatch`] weighs before it says anything. A
/// quarter of a second at 48 kHz: long enough that a moment of silence between
/// two overs is not read as a dead channel, short enough that the operator is
/// told while they are still looking at the picture that puzzled them.
const QUAD_WINDOW_PAIRS: usize = 12_000;

/// Coherence above which the two channels are carrying the same signal rather
/// than two quadrature halves of one.
///
/// Real I/Q from a rig sits near zero: the two channels are the same signal a
/// quarter-cycle apart at every frequency, so over a band of them the products
/// cancel. Even a badly unbalanced front end — the ±3 dB and several degrees
/// that leave a mirror image 20 dB down — stays well under a tenth. One signal
/// copied onto both wires reads 1.0 exactly, and this is set to catch that and
/// nothing else.
const QUAD_COHERENT: f32 = 0.9;

/// Below this, a channel is carrying nothing: the ratio of the quieter
/// channel's power to the louder one's. Four orders of magnitude down is
/// silence next to any signal, and still above the noise a live but unconnected
/// input picks up.
const QUAD_DEAD_RATIO: f32 = 1e-4;

/// Watches the two capture channels for the one arrangement that cannot work:
/// a stream declared as I/Q whose channels are not in quadrature.
///
/// The commonest way to get one is to point sdroxide at a radio's *demodulated*
/// audio and tell it the card carries I/Q. A demodulated channel is one real
/// signal, and a stereo cable carries it twice — so `I` and `Q` are the same
/// numbers, the complex stream they build is that real signal at 45°, and its
/// spectrum is symmetric about the dial. Every signal appears twice, once
/// either side, and neither *Invert spectrum* nor *I/Q correction* touches it:
/// there is no sideband to swap and no imbalance to trim, because there was
/// never a second sideband to begin with (issue #308).
///
/// It is worth saying out loud because the picture is *plausible*. The
/// panadapter is the right width, the noise floor is where it should be, the
/// signals are real signals — they are simply each drawn twice, and an operator
/// who has not seen it before reads it as a receiver fault rather than as a
/// setting.
///
/// A free struct with the arithmetic in it, for the reason [`DropWatch`] is
/// one: the thing it watches lives behind a sound card, and what actually goes
/// wrong is the sums.
#[derive(Default)]
struct QuadratureWatch {
    i: f64,
    q: f64,
    ii: f64,
    qq: f64,
    iq: f64,
    n: usize,
    /// What the last completed window concluded, so a verdict is announced when
    /// it changes and not once per window forever.
    said: Option<QuadVerdict>,
}

/// What a window of samples says about the two channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuadVerdict {
    /// Two channels a quarter-cycle apart: an I/Q stream.
    Quadrature,
    /// Both channels carrying one signal — demodulated audio, not I/Q.
    SameOnBoth,
    /// One channel silent. Half a cable, or a mono source opened as stereo.
    OneChannelDead,
}

impl QuadratureWatch {
    /// Fold one block in, and return a verdict on the window it completes when
    /// that verdict is not the one already announced.
    fn observe(&mut self, iq: &[Complex32]) -> Option<QuadVerdict> {
        for c in iq {
            let (i, q) = (f64::from(c.re), f64::from(c.im));
            self.i += i;
            self.q += q;
            self.ii += i * i;
            self.qq += q * q;
            self.iq += i * q;
        }
        self.n += iq.len();
        if self.n < QUAD_WINDOW_PAIRS {
            return None;
        }
        let verdict = self.verdict();
        self.i = 0.0;
        self.q = 0.0;
        self.ii = 0.0;
        self.qq = 0.0;
        self.iq = 0.0;
        self.n = 0;
        // A window with nothing in it at all — the rig is off, or this is the
        // silence of an over — says nothing about the wiring, so it is neither
        // reported nor allowed to clear a report already made.
        let verdict = verdict?;
        (self.said.replace(verdict) != Some(verdict)).then_some(verdict)
    }

    /// The completed window's verdict, or `None` when both channels are silent
    /// and there is nothing to compare.
    ///
    /// Measured about each channel's own mean rather than about zero, which is
    /// not a nicety: a sound card's DC offset is common to both channels, so on
    /// a quiet band the raw correlation between them is the *offset* correlating
    /// with itself and reads as a perfect 1.0 — a healthy front end accused of
    /// carrying one signal twice. The same subtraction stops an offset on one
    /// channel alone standing in for a signal on it.
    fn verdict(&self) -> Option<QuadVerdict> {
        let n = self.n as f64;
        if n <= 1.0 {
            return None;
        }
        let (mi, mq) = (self.i / n, self.q / n);
        let ii = (self.ii / n - mi * mi).max(0.0);
        let qq = (self.qq / n - mq * mq).max(0.0);
        let iq = self.iq / n - mi * mq;
        let loud = ii.max(qq);
        if loud <= 0.0 || !loud.is_finite() {
            return None;
        }
        // Absolute silence on both wires is a radio that is not sending, not a
        // wiring fault. The threshold is on the *quieter* channel relative to
        // the louder one, so this only fires when one of them is live.
        if (ii.min(qq) / loud) < f64::from(QUAD_DEAD_RATIO) {
            return Some(QuadVerdict::OneChannelDead);
        }
        let coherence = (iq.abs() / (ii * qq).sqrt()) as f32;
        Some(if coherence >= QUAD_COHERENT {
            QuadVerdict::SameOnBoth
        } else {
            QuadVerdict::Quadrature
        })
    }
}

/// What to tell the operator about a verdict, or `None` where there is nothing
/// wrong to tell them about.
fn quadrature_complaint(v: QuadVerdict) -> Option<&'static str> {
    match v {
        QuadVerdict::Quadrature => None,
        QuadVerdict::SameOnBoth => Some(
            "radio I/Q: both capture channels are carrying the same signal, so this is \
             demodulated audio and not I/Q. The spectrum will be mirrored about the dial \
             and every signal drawn twice, and neither Invert spectrum nor I/Q correction \
             can help — there is no second sideband to recover. Either set Sound format to \
             \"Demod audio\" under Settings \u{2192} Radio, or switch the radio\u{2019}s virtual \
             cable to its raw I/Q output (in PowerSDR, VAC \u{2192} Direct I/Q).",
        ),
        QuadVerdict::OneChannelDead => Some(
            "radio I/Q: one capture channel is silent. I/Q needs both — check that the \
             card is a stereo one and that the radio is feeding both channels of it.",
        ),
    }
}

/// The corrector a given configuration asks for, or `None` when it asks for
/// nothing — see [`sdroxide_types::CatConfig::iq_correction`] and
/// [`sdroxide_types::CatConfig::iq_dc_block_hz`].
///
/// The two settings are not independent in one direction: an uncorrected DC
/// offset multiplies straight into the correlation the imbalance loop measures,
/// so a rig being corrected always has its DC removed as well — at the operator's
/// corner if they set one, and at the ordinary front-end corner if they did not.
/// The reverse pairing is a real choice and is honoured: DC alone, with the
/// loop that a mirror-symmetric band can mislead left out of circuit.
fn build_correction(
    cfg: &sdroxide_types::CatConfig,
    format: SoundFormat,
    rate: f64,
) -> Option<IqCorrect> {
    // Demod audio is a real signal inside the rig's own filter: no quadrature,
    // so no image, and its DC is the demodulator's business.
    if !matches!(format, SoundFormat::Iq) {
        return None;
    }
    let notch = cfg.iq_dc_block_hz.clamp(0.0, sdroxide_types::CAT_IQ_DC_BLOCK_MAX_HZ);
    match (cfg.iq_correction, notch > 0.0) {
        (false, false) => None,
        (false, true) => Some(IqCorrect::dc_only(notch, rate)),
        (true, _) => Some(IqCorrect::new(notch.max(DC_BLOCK_HZ), rate)),
    }
}

/// Drain interleaved (I, Q) pairs from the capture ring into `buf`, with the
/// right channel scaled by `q_sign` — `-1.0` conjugates the stream, mirroring
/// the spectrum about the dial (see [`sdroxide_types::CatConfig::invert_spectrum`])
/// — the result run through `correct` to take out the front end's DC and mirror
/// image, and then mixed by `shift` when the rig's I.F. is not on its dial
/// (see [`sdroxide_types::CatConfig::iq_offset_hz`]).
///
/// The three are in that order because they undo the radio in the order the
/// radio applied it. Which wire carries Q decides what the stream *means*, and
/// only a stream that means the right thing can be moved the right way —
/// conjugating after the shift would mirror the shift along with everything
/// else and land the dial twice the offset away. The correction has to come
/// before the shift for a plainer reason: both of the defects it removes are
/// anchored to the *card's* centre, and the shift is what stops that being the
/// centre of what comes out. Correcting afterwards would look for the spike and
/// the mirror axis at the dial, where on a rig with `RX SHFT` set neither of
/// them is.
///
/// A free function rather than a method so all three can be exercised without a
/// sound card and a serial port behind them.
fn fill_iq(
    src: &mut rtrb::Consumer<f32>,
    buf: &mut [Complex32],
    q_sign: f32,
    quad: &mut QuadratureWatch,
    correct: Option<&mut IqCorrect>,
    shift: Option<&mut Nco>,
) -> usize {
    let mut n = 0;
    // Need pairs (I, Q); only consume when both are available.
    while n < buf.len() && src.slots() >= 2 {
        let i = src.pop().unwrap_or(0.0);
        let q = src.pop().unwrap_or(0.0);
        buf[n] = Complex32::new(i, q * q_sign);
        n += 1;
    }
    // Ahead of everything below, on the samples as the card handed them over.
    // The corrector adapts to what it is given and the oscillator moves it, and
    // this asks a question about the wiring rather than about the band: are
    // these two channels a quarter-cycle apart at all? Conjugating them does
    // not change the answer, so `q_sign` is allowed to have happened.
    if let Some(v) = quad.observe(&buf[..n])
        && let Some(complaint) = quadrature_complaint(v)
    {
        tracing::warn!("{complaint}");
    }
    // Ahead of the shift, and after the conjugation — which the estimator does
    // not mind either way, because a conjugated stream has its DC and its
    // mirror axis in the same place and the loop simply converges on the
    // mirrored coefficient.
    if let Some(iq) = correct {
        iq.process(&mut buf[..n]);
    }
    // Phase-continuous across blocks, which is the whole reason the oscillator
    // outlives the call: restarting it at every read would put a step in the
    // phase of every signal on the band, at whatever rate the sound card
    // happens to hand over blocks.
    if let Some(nco) = shift {
        nco.mix_in_place(&mut buf[..n]);
    }
    n
}

impl IqSource for AudioCatSource {
    fn sample_rate(&self) -> f64 {
        self.in_rate
    }
    fn center_hz(&self) -> f64 {
        self.dial.vfo
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        if let Some(f) = self.dial.set_vfo(hz) {
            self.cat.set_freq(f);
        }
        Ok(())
    }

    fn set_rit_hz(&mut self, hz: f64) {
        if let Some(f) = self.dial.set_rit(hz) {
            self.cat.set_freq(f);
        }
    }

    /// The rig's synthesiser is the centre of the I/Q it sends us: there is no
    /// second oscillator to park somewhere and tune away from — so the dial the
    /// operator asks for is commanded at the radio and the window follows it
    /// (`Engine::follow_dial`).
    ///
    /// Unless nothing is listening on the control port. A transceiver sending
    /// I/Q with no CAT cable on it is a receiver whose dial only its own knob
    /// can move, and commanding a dial that nothing hears is how the panadapter
    /// came to look **locked**: every click relabelled the span around a
    /// frequency the sound card was not sending, so the station clicked on
    /// walked out of the receiver instead of into it, and the only way left to
    /// change stations was the knob on the radio (issue #155, a Xiegu G90 on
    /// I/Q with no control cable — the regression arrived with the fix that
    /// stopped a click and the rig's readout parting company).
    ///
    /// With no dial to command, the span the rig *is* sending is the whole of
    /// the radio, and the engine tunes inside it with its own DDC exactly as it
    /// does on an SDR. Typing a frequency still moves the centre, which is how
    /// the operator tells sdroxide where they have left the radio's own dial.
    fn center_is_dial(&self) -> bool {
        self.dial_reachable
    }

    /// The transceiver in front of us owns its mode, in either sound format.
    ///
    /// Demod audio reaches this through `DeviceCaps::audio_mode` — there is no
    /// demodulator on this side at all — but a rig sending quadrature does not:
    /// its stream is ordinary complex baseband and sdroxide demodulates it, so
    /// the engine's "command the rig's mode" gate never fired and the mode
    /// control moved nothing on the radio. Mode travelled rig→app on the CAT
    /// poll and never the other way, and the two readouts sat there disagreeing
    /// until the next key-down, where `tx_begin` asserts the mode anyway — which
    /// is the wrong moment to discover that the radio was still on the other
    /// sideband.
    ///
    /// [`IqSource::commands_rx_mode`] rather than
    /// [`IqSource::tracks_rx_mode`]: nothing is imposed when the port opens.
    /// This source adopts the mode the rig is already sitting on (see
    /// `sdroxide_cat::query_once`), and rearranging somebody's radio out of a
    /// restored session is not what connecting to it should do.
    ///
    /// The *filter* is not pushed with it, and must not be: the engine guards
    /// that separately on whether the rig is the one doing the receiving, and
    /// here it is not — the width the operator sets belongs to sdroxide's own
    /// demodulator, and sending it would narrow the radio's IF against a
    /// passband it is not carrying.
    fn commands_rx_mode(&self) -> bool {
        true
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        self.check_dropped();
        match self.format {
            SoundFormat::DemodAudio => {
                let mut n = 0;
                while n < buf.len() {
                    match self.in_consumer.pop() {
                        Ok(s) => {
                            buf[n] = Complex32::new(s, 0.0);
                            n += 1;
                        }
                        Err(_) => break,
                    }
                }
                if n == 0 {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Ok(n)
            }
            SoundFormat::Iq => {
                let n = fill_iq(
                    &mut self.in_consumer,
                    buf,
                    self.q_sign,
                    &mut self.quad,
                    self.iq_correct.as_mut(),
                    self.iq_shift.as_mut(),
                );
                if n == 0 {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Ok(n)
            }
        }
    }

    /// The same drain as [`Self::read`] without the nap on an empty ring.
    ///
    /// The default falls through to `read`, whose 5 ms sleep is what keeps the
    /// engine loop from spinning on a sound card that has nothing yet. That is
    /// right when this is the only stream there is, and wrong wherever it is
    /// not: a radio with another radio attached as its panadapter pulls this
    /// rig's audio once per I/Q block, and a sleep there would pace the whole
    /// receiver off a sound card that is not driving it.
    fn read_available(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        // Checked on this path too, not only in `read`: a rig lent out as
        // another radio's panadapter is drained exclusively through here, and
        // that is the arrangement where a card the machine cannot keep up with
        // is *most* likely — two radios' streams on one machine — and the one
        // where the drops went entirely unreported.
        self.check_dropped();
        Ok(match self.format {
            SoundFormat::DemodAudio => {
                let mut n = 0;
                while n < buf.len() {
                    let Ok(s) = self.in_consumer.pop() else { break };
                    buf[n] = Complex32::new(s, 0.0);
                    n += 1;
                }
                n
            }
            SoundFormat::Iq => fill_iq(
                &mut self.in_consumer,
                buf,
                self.q_sign,
                &mut self.quad,
                self.iq_correct.as_mut(),
                self.iq_shift.as_mut(),
            ),
        })
    }

    fn describe(&self) -> String {
        self.label.clone()
    }

    fn open_status(&self) -> Option<String> {
        self.status.clone()
    }

    fn display_bandwidth(&self) -> Option<f64> {
        matches!(self.format, SoundFormat::DemodAudio).then_some(self.audio_bw)
    }

    /// The rig's own scope, when [`sdroxide_types::CatConfig::scope`] streams
    /// it over the serial link — the same `27 00` sweeps, and the same two
    /// lanes, as the Icom LAN backend: the full-band strip always, and on the
    /// demod-audio path the *main* panadapter too (the engine decides — see
    /// its `scope_main_window`). Finished magnitude bins on the radio's own
    /// scale, mapped to dB with the same uncalibrated slope the LAN backend
    /// uses; the engine's auto-levelling makes the picture right even where the
    /// absolute numbers are not.
    fn wide_spectrum_db(&mut self, out: &mut Vec<f32>) -> Option<(f64, f64)> {
        let sweep = self.cat.take_scope_sweep()?;
        if sweep.bins.is_empty() || sweep.span_hz <= 0.0 {
            return None;
        }
        out.clear();
        out.extend(sweep.bins.iter().map(|&b| (f32::from(b) - self.scope_full_scale) * 0.5));
        Some((sweep.center_hz, sweep.span_hz))
    }

    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        let mut out = Vec::new();
        // A tune the rig has not agreed to yet, asked for again. Cheap
        // insurance on a serial port, where a command is only lost to a cable
        // or a rig that was busy; the case it was written for is the networked
        // Icom sharing this `Dial` — see [`Dial::retry`] and issue #297.
        if let Some(f) = self.dial.retry() {
            self.cat.set_freq(f);
        }
        let updates = self.cat.poll();
        // Anything at all from the rig is also the answer to "is there a radio
        // on the control port": a link that only came up later — the operator
        // switched the radio on, or plugged the cable in, after sdroxide
        // started — hands its dial back here. Latched rather than tracked,
        // because a rig that has gone quiet for a poll or two is still a rig
        // whose dial we own, and flipping the panadapter's whole tuning
        // behaviour on a missed reply would be worse than either answer.
        self.dial_reachable |= !updates.is_empty();
        for u in updates {
            match u {
                // The dial is not the VFO — it carries RIT, and for the length
                // of an over it carries XIT/split instead — so a report has to
                // be folded back before the engine sees it as a dial move.
                sdroxide_cat::CatUpdate::Freq(hz) => {
                    if let Some(vfo) = self.dial.report(hz) {
                        out.push(ControlUpdate::Freq(vfo));
                    }
                }
                sdroxide_cat::CatUpdate::Mode(m) => out.push(ControlUpdate::Mode(m)),
                // Which antenna socket the rig is receiving on, read once when
                // the port opened — the same shape as the power below, and
                // adopted for the same reason: the operator set it on the radio
                // and the radio is where it survived a power cycle.
                sdroxide_cat::CatUpdate::Antenna(name) => {
                    self.antenna = name.to_string();
                    out.push(ControlUpdate::Antenna(name));
                }
                // The receiving antenna, from the same reply — read when the
                // port opened and again after every band change, because the
                // radio recalls it per band and this end never asserts it.
                sdroxide_cat::CatUpdate::RxAntenna(on) => {
                    out.push(ControlUpdate::RxAntenna(on));
                }
                // The power the rig came up on, read once when the port opened.
                // The engine adopts it into the Drive slider rather than
                // commanding the rig back — the radio's own setting is the
                // operator's, not a stale one to overwrite.
                sdroxide_cat::CatUpdate::Power(frac) => out.push(ControlUpdate::TxDrive(frac)),
                // And the squelch it came up on, read at the same moment and
                // adopted the same way. On demod audio this is the gate the
                // operator actually hears, so the rail has to start where the
                // radio's knob already is.
                sdroxide_cat::CatUpdate::Squelch(frac) => out.push(ControlUpdate::Squelch(frac)),
                // The operator keyed the radio itself — mic button, foot
                // switch, VOX, its own keyer. Passed up as the thing it is, not
                // as a request to transmit: see `ControlUpdate::RigTx`.
                sdroxide_cat::CatUpdate::Ptt(on) => {
                    // The SWR latched during that over belongs to that over.
                    // `tx_end` does this for one we keyed; nothing calls it for
                    // one we did not, and a stale ratio left on the meter would
                    // read as the *receiver's* — the needle sitting at 2:1 on a
                    // radio that is not transmitting at all.
                    if !on {
                        self.last_telem = None;
                    }
                    out.push(ControlUpdate::RigTx(on));
                }
                // The meters arrive on their own telemetry channels, not here.
                sdroxide_cat::CatUpdate::Swr(_)
                | sdroxide_cat::CatUpdate::Alc(_)
                | sdroxide_cat::CatUpdate::Po(_)
                | sdroxide_cat::CatUpdate::FwdW(_)
                | sdroxide_cat::CatUpdate::Signal(_) => {}
            }
        }
        out
    }

    fn set_control_mode(&mut self, mode: Mode) -> Result<()> {
        self.cat.set_mode(mode);
        Ok(())
    }

    /// The CAT link does: the mode goes through `digi_mode` there, on whichever
    /// sideband the dial says analog SSTV is riding — see `commanded_mode`.
    fn resolves_sstv_sideband(&self) -> bool {
        true
    }

    /// Put the receiver on one of the rig's antenna sockets — an ELAD FDM-DUO's
    /// `AN`, and nothing else in this family: every other rig here publishes an
    /// empty port list, so nothing ever asks.
    ///
    /// Receive only. The DUO transmits out of its RTX socket whichever socket
    /// it is *listening* on, so there is no transmit port to pick.
    fn set_antenna(&mut self, name: &str) -> Result<()> {
        // A name this rig does not have — the port of whatever front end was on
        // this radio before — is dropped by the handle rather than sent.
        if !self.cat.antennas().contains(&name) {
            return Ok(());
        }
        self.cat.set_antenna(name);
        self.antenna = name.to_string();
        Ok(())
    }

    fn current_antenna(&self) -> String {
        self.antenna.clone()
    }

    /// The sockets the rig has since said it has.
    ///
    /// An ELAD's list is settled from the family name and never changes here.
    /// A CI-V rig's is not: every Icom speaks one dialect and only some have an
    /// antenna selector, so the handle publishes nothing until the radio has
    /// answered the read the link sends when the port opens (issue #238). That
    /// answer lands a round trip after the capabilities went out, which is what
    /// this exists to catch up with.
    fn learned_antennas(&self) -> Option<&'static [&'static str]> {
        Some(self.cat.antennas())
    }

    /// The radio's own power switch, over the CAT link (issue #239).
    fn set_rig_power(&mut self, on: bool) -> Result<()> {
        self.cat.set_rig_power(on);
        Ok(())
    }

    fn commands_rig_power(&self) -> bool {
        self.cat.commands_rig_power()
    }

    /// The rig's separate receiving antenna, learned the same way the socket
    /// list is: the flag rides behind the socket in the antenna reply, and a
    /// radio without the connector answers the socket alone.
    fn rx_antenna(&self) -> Option<bool> {
        self.cat.rx_antenna()
    }

    fn set_rx_antenna(&mut self, on: bool) -> Result<()> {
        self.cat.set_rx_antenna(on);
        Ok(())
    }

    fn reread_rx_antenna(&mut self) {
        self.cat.reread_antenna();
    }

    /// The panel's width control, sent to the only filter in the path.
    ///
    /// There is no demodulator on this side of a CAT rig — the audio arrives
    /// already filtered, levelled by an AGC that has ridden the interference
    /// down, and narrowing it here would only cut what the radio had already
    /// let through. The rig's own filter is the one that does the work.
    fn set_control_filter(&mut self, mode: Mode, lo_hz: f64, hi_hz: f64) {
        self.cat.set_filter(mode, lo_hz as f32, hi_hz as f32);
    }

    // ── Squelch ──────────────────────────────────────────────────────────────
    /// The rig's own squelch, over CAT. On demod audio it is the only one there
    /// is: what reaches the sound card has already been through it, so a
    /// threshold applied on this side can close further on what got through but
    /// can never open what was shut out (issue #192).
    fn set_squelch(&mut self, frac: f32) {
        self.cat.set_squelch(frac);
    }

    /// True on a family whose squelch sdroxide can address — but only while the
    /// rig is the one doing the demodulating.
    ///
    /// On quadrature it is the wrong gate entirely: the stream is raw baseband,
    /// sdroxide demodulates it, and the radio's squelch decides nothing but
    /// what comes out of the radio's own speaker. There the engine's own
    /// threshold is the honest one, and handing the SQL rail to a control that
    /// does not reach the audio would be the same fault the other way round.
    fn commands_squelch(&self) -> bool {
        self.cat.commands_squelch() && matches!(self.format, SoundFormat::DemodAudio)
    }

    // ── Output power ─────────────────────────────────────────────────────────
    // The rig's own power control, over CAT. It is the only transmit level that
    // means anything in every mode: the audio we put into the sound card is not
    // one in CW (the rig keys itself from its own keyer and never looks at the
    // sound card) and not one under TUNE either, so before this the Drive and
    // Tune sliders moved nothing at all on a CAT rig.
    //
    // The engine asserts the level that applies — drive, or the tune level
    // while tuning — before every key-down, so an over cannot go out at the
    // previous one.
    fn set_tx_drive(&mut self, frac: f64) {
        self.cat.set_power(frac as f32);
    }

    /// Nothing to set: a rig has one power register, and the engine commands the
    /// tune level through [`Self::set_tx_drive`] for as long as TUNE holds the
    /// transmitter.
    fn set_tune_drive(&mut self, _frac: f64) {}

    fn commands_tx_power(&self) -> bool {
        self.cat.commands_power()
    }

    // ── CW from the rig's own keyer ──────────────────────────────────────────
    // A rig in CW does not modulate the audio we send it — it keys its own
    // transmitter — so the panel's keyer hands it text instead of sidetone.
    fn cw_text_keying(&self) -> Option<usize> {
        self.cat.cw_chunk_len()
    }
    // …and when the panel keys as audio instead, the CAT thread commands the
    // digi sideband rather than CW (see `commanded_mode`); this tells the
    // engine to expect the sideband back from the mode poll.
    fn cw_audio_keyed(&self) -> bool {
        self.cw_mcw
    }
    fn send_cw(&mut self, text: &str) {
        self.cat.send_cw(text.to_string());
    }
    fn abort_cw(&mut self) {
        self.cat.abort_cw();
    }
    fn set_cw_wpm(&mut self, wpm: f32) {
        self.cat.set_cw_wpm(wpm);
    }

    fn tx_begin(&mut self, center_hz: f64, _rate: f64) -> Result<f64> {
        // XIT and split have no DDC to ride on here — the rig's dial is the
        // whole of its frequency control — so an over that transmits away from
        // where we listen borrows the dial for its duration. The frequency is
        // queued before PTT and the CAT thread writes a pending frequency out
        // ahead of keying, so nothing goes on air at the receive frequency.
        if let Some(f) = self.dial.begin_tx(center_hz) {
            self.cat.set_freq(f);
        }
        self.cat.set_ptt(true);
        Ok(self.out.as_ref().map(|(o, _)| o.sample_rate).unwrap_or(self.in_rate))
    }

    fn tx_end(&mut self) -> Result<()> {
        self.cat.set_ptt(false);
        // Give the dial back, including any retune the operator asked for while
        // the over held it.
        if let Some(f) = self.dial.end_tx() {
            self.cat.set_freq(f);
        }
        self.last_telem = None; // drop the stale SWR reading on unkey
        Ok(())
    }

    fn discard_pending_rx(&mut self) {
        // The capture callback keeps filling this ring during TX too.
        while self.in_consumer.pop().is_ok() {}
        // And it overflowed it long before the over was out. The engine stops
        // reading a half-duplex source for the length of a transmission, so the
        // ring is full within a second and every frame the card delivers after
        // that is counted lost — 11.6 s of them behind one FT8 over. Those are
        // not a machine that cannot keep up. They are frames nobody was reading,
        // from a receiver nobody was listening to, and counting them told an
        // operator with a perfectly healthy card to go and fix it.
        //
        // Re-baselining belongs here because here is exactly the unkey: the
        // engine calls this at the end of an over and nowhere else, so the
        // frames excused are precisely the ones TX caused, and every drop that
        // happens while actually receiving still counts.
        //
        // The window goes with it: the next warning's starts at the unkey, not
        // at the last look before the over, or the report that follows a
        // transmission spans one.
        if let Some(total) = self.in_stream.as_ref().map(|s| s.dropped_frames()) {
            self.drops.rebase(std::time::Instant::now(), total);
        }
    }

    fn tx_telemetry(&mut self) -> Option<TxTelemetry> {
        // The CI-V thread polls SWR at ~5 Hz; latch its latest reading so the
        // engine's 100 ms meter tick always has a value to show.
        if let Some(t) = self.cat.poll_telemetry() {
            self.last_telem = Some(t);
        }
        self.last_telem
    }

    fn rx_signal_dbm(&mut self) -> Option<f32> {
        // Latched for the same reason as `tx_telemetry`, and against the same
        // ~5 Hz stream of readings. A rig whose family (or dialect) has no
        // S-meter read never sends any — and one that has stopped answering
        // stops counting once its last reading goes stale. Either way the
        // engine falls back to the level of the audio itself, which is still
        // arriving whatever the control link is doing.
        if let Some(dbm) = self.cat.poll_signal() {
            self.last_signal = Some((std::time::Instant::now(), dbm));
        }
        self.last_signal.filter(|(at, _)| at.elapsed() < self.signal_max_age).map(|(_, dbm)| dbm)
    }

    fn tx_write_audio(&mut self, audio: &[f32]) -> Result<()> {
        let Some((_, producer)) = self.out.as_mut() else {
            return Ok(()); // no TX audio device — PTT still keyed the rig
        };
        // Resample 48 kHz → card rate, then interleave to stereo.
        //
        // A *pair* per sample, whatever the card's own channel count is: the
        // ring `start_output` hands back is interleaved stereo by definition,
        // and the playback callback takes two out of it for every frame it
        // fills and mixes them down itself where the device opened mono. One
        // per sample on such a card is therefore not a quieter over — the
        // callback consumes the ring twice as fast as it is filled, so the
        // audio goes out at double speed with every pair averaged together and
        // silence spliced in wherever it ran dry. On a virtual cable, which is
        // where a mono output turns up (VB-Audio's CABLE-B opens as one
        // channel), that is an FT8 burst arriving as a smear across the whole
        // passband (issue #247).
        self.tx_scratch.clear();
        match self.tx_resampler.as_mut() {
            Some(rs) => rs.push(audio, &mut self.tx_scratch),
            None => self.tx_scratch.extend_from_slice(audio),
        }
        // Block until the card drains room, applying backpressure so the engine's
        // TX loop is paced to real time. Without this a long continuous burst
        // (e.g. a 110 s SSTV image) is generated at CPU speed and mostly dropped
        // on a full ring, so the radio only transmits the first buffer-full.
        for &s in &self.tx_scratch {
            for _ in 0..2 {
                let mut v = s;
                let mut tries = 0u32;
                while let Err(rtrb::PushError::Full(x)) = producer.push(v) {
                    v = x;
                    tries += 1;
                    if tries > 200 {
                        break; // output device stalled — drop rather than hang TX
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }
        }
        Ok(())
    }

    fn tx_drain(&mut self) {
        // The output ring holds ~1 s; wait for it to play out before PTT is
        // released so the tail of a burst (critical for FT8 decode) isn't cut.
        if let Some((_, producer)) = self.out.as_ref() {
            let cap = producer.buffer().capacity();
            for _ in 0..1000 {
                let buffered = cap.saturating_sub(producer.slots());
                if buffered <= cap / 40 {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    /// One FT8 over, as the capture counter sees it: the ring holds a second,
    /// nobody empties it for the length of the transmission, and every frame
    /// after that first second is counted lost.
    const OVER: std::time::Duration = std::time::Duration::from_millis(12_640);
    const OVER_DROPS: u64 = 606_597 - 48_000;

    /// The window a warning reports is the one that was measured, not the
    /// interval it hoped for.
    ///
    /// Nothing looks at the counter while the source is not being read, and a
    /// half-duplex rig is not read for the length of an over — so the first
    /// look afterwards covers the whole transmission. Reporting a fixed 5 s
    /// there announced 11.6 s of lost signal inside a 5 s window, which cannot
    /// happen, and an operator who catches the arithmetic out has no reason to
    /// believe the diagnosis attached to it.
    #[test]
    fn the_reported_window_is_the_one_that_elapsed() {
        let t0 = std::time::Instant::now();
        let mut w = DropWatch::started(t0);
        let (lost, window) = w.check(t0 + OVER, OVER_DROPS).expect("a stalled read must report");
        assert_eq!(lost, OVER_DROPS);
        assert!(
            window >= OVER,
            "{window:?} must span the whole stall, not {DROP_CHECK_INTERVAL:?}"
        );
        // And the loss has to fit inside the window it is reported in.
        assert!(lost as f64 / 48_000.0 <= window.as_secs_f64());
    }

    /// An unkey excuses the frames the transmission caused.
    ///
    /// The engine stops reading a half-duplex source while it transmits, so the
    /// ring overflows every over and the drops say nothing about this machine.
    /// Re-baselining at the unkey is what keeps them out of the log.
    #[test]
    fn an_over_leaves_nothing_to_report() {
        let t0 = std::time::Instant::now();
        let mut w = DropWatch::started(t0);
        let unkey = t0 + OVER;
        w.rebase(unkey, OVER_DROPS);
        assert_eq!(
            w.check(unkey + DROP_CHECK_INTERVAL, OVER_DROPS),
            None,
            "frames lost while nobody was reading the ring are not this machine's fault"
        );
    }

    /// A card that keeps losing frames while actually receiving is still
    /// reported, and against a window that starts at the unkey.
    ///
    /// The half the excusing could take with it: forgive the over and forgive
    /// everything, and the warning that matters never fires again. Nothing may
    /// run between the unkey and the measurement here — [`DropWatch::check`]
    /// restarts the window itself, so an intervening look would hide a `rebase`
    /// that failed to.
    #[test]
    fn a_real_drop_after_the_over_still_reports() {
        let t0 = std::time::Instant::now();
        let mut w = DropWatch::started(t0);
        let unkey = t0 + OVER;
        w.rebase(unkey, OVER_DROPS);

        let (lost, window) = w
            .check(unkey + DROP_CHECK_INTERVAL, OVER_DROPS + 900)
            .expect("a drop while receiving must report");
        assert_eq!(lost, 900, "only what was lost while receiving");
        assert_eq!(
            window, DROP_CHECK_INTERVAL,
            "measured from the unkey, not from before the over"
        );
    }

    /// A look that comes too soon reports nothing and — the part that matters —
    /// leaves the window running, so the frames it saw are still counted by
    /// whichever look does report.
    #[test]
    fn an_early_look_neither_reports_nor_forgets() {
        let t0 = std::time::Instant::now();
        let mut w = DropWatch::started(t0);
        assert_eq!(w.check(t0 + DROP_CHECK_INTERVAL / 2, 500), None);
        let (lost, window) = w.check(t0 + DROP_CHECK_INTERVAL, 700).expect("due now");
        assert_eq!(lost, 700, "the 500 the early look saw are still in the total");
        assert!(window >= DROP_CHECK_INTERVAL);
    }

    /// Push a complex tone at `hz` onto a ring as a sound card would deliver
    /// it: I on the left channel, Q on the right, interleaved.
    fn tone_ring(hz: f32, n: usize) -> rtrb::Consumer<f32> {
        let (mut p, c) = rtrb::RingBuffer::<f32>::new(n * 2);
        for k in 0..n {
            let phase = std::f32::consts::TAU * hz * k as f32 / RATE;
            p.push(phase.cos()).unwrap();
            p.push(phase.sin()).unwrap();
        }
        c
    }

    /// Which side of the dial the stream sits on, as the sign of its mean
    /// frequency: the argument of the average of each sample against the one
    /// before it. Positive is above the dial, negative below.
    fn mean_freq_hz(x: &[Complex32]) -> f32 {
        let acc: Complex32 = x.windows(2).map(|w| w[1] * w[0].conj()).sum();
        acc.arg() * RATE / std::f32::consts::TAU
    }

    /// The whole of the setting: a signal the radio puts 1 kHz *above* its dial
    /// has to read as 1 kHz above it on a normally wired rig, and 1 kHz below
    /// on one whose I and Q are the other way round — which is the mirrored
    /// waterfall the operator sees, and the wrong sideband SSB comes out on.
    #[test]
    fn inverting_iq_mirrors_the_signal_about_the_dial() {
        let mut buf = [Complex32::new(0.0, 0.0); 512];

        let n = fill_iq(
            &mut tone_ring(1000.0, 512),
            &mut buf,
            1.0,
            &mut QuadratureWatch::default(),
            None,
            None,
        );
        assert_eq!(n, 512, "every pair consumed");
        assert!((mean_freq_hz(&buf[..n]) - 1000.0).abs() < 1.0, "above the dial, unchanged");

        let n = fill_iq(
            &mut tone_ring(1000.0, 512),
            &mut buf,
            -1.0,
            &mut QuadratureWatch::default(),
            None,
            None,
        );
        assert_eq!(n, 512);
        assert!((mean_freq_hz(&buf[..n]) + 1000.0).abs() < 1.0, "below the dial once inverted");
    }

    /// A rig whose I.F. has been shifted — an Elecraft with `RX SHFT` at 8.0 —
    /// puts the station the operator is tuned to 8 kHz from the centre of its
    /// I/Q output, while still displaying and transmitting on the dial. Undoing
    /// that here is what keeps the dial the dial everywhere above.
    #[test]
    fn a_shifted_if_comes_back_onto_the_dial() {
        let mut buf = [Complex32::new(0.0, 0.0); 512];
        let mut nco = Nco::new(8_000.0, RATE as f64);

        // The rig's centre is 8 kHz above the dial, so the station on the dial
        // arrives 8 kHz below centre — and has to read as being on it.
        let n = fill_iq(
            &mut tone_ring(-8_000.0, 512),
            &mut buf,
            1.0,
            &mut QuadratureWatch::default(),
            None,
            Some(&mut nco),
        );
        assert_eq!(n, 512);
        assert!(mean_freq_hz(&buf[..n]).abs() < 1.0, "the dial is the dial again");

        // Everything else keeps its distance from it: a station 1 kHz up the
        // band is 1 kHz up the band, not 9.
        let mut nco = Nco::new(8_000.0, RATE as f64);
        let n = fill_iq(
            &mut tone_ring(-7_000.0, 512),
            &mut buf,
            1.0,
            &mut QuadratureWatch::default(),
            None,
            Some(&mut nco),
        );
        assert!((mean_freq_hz(&buf[..n]) - 1000.0).abs() < 1.0, "1 kHz above the dial");
    }

    /// The order the two corrections are applied in. A rig that is both
    /// miswired and shifted has to come out on the dial, not at twice the
    /// offset from it — which is what mirroring a stream that has already been
    /// moved would do.
    #[test]
    fn inversion_is_undone_before_the_shift() {
        let mut buf = [Complex32::new(0.0, 0.0); 512];
        let mut nco = Nco::new(8_000.0, RATE as f64);

        // Miswired, so the card delivers the conjugate: the station that sits
        // 8 kHz below the rig's centre arrives looking like +8 kHz.
        let n = fill_iq(
            &mut tone_ring(8_000.0, 512),
            &mut buf,
            -1.0,
            &mut QuadratureWatch::default(),
            None,
            Some(&mut nco),
        );
        assert!(mean_freq_hz(&buf[..n]).abs() < 1.0, "on the dial, both corrections applied");
    }

    /// The correction is anchored to the *card's* centre, which is why it runs
    /// before the shift. On a rig with `RX SHFT` set to 8.0 the mixer's offset
    /// sits on the centre of what the card records and the station the operator
    /// is tuned to sits 8 kHz below it; undo the two the other way round and
    /// the DC blocker eats the station — which the shift has just put on zero —
    /// while the spike survives as a tone 8 kHz up (issue #147).
    #[test]
    fn the_correction_runs_on_the_cards_centre_not_the_dial() {
        const N: usize = 32_768;
        let (mut p, mut c) = rtrb::RingBuffer::<f32>::new(2 * N);
        for k in 0..N {
            let phase = std::f32::consts::TAU * -8_000.0 * k as f32 / RATE;
            // The station on the dial, with the mixer's own offset on the
            // centre of the card alongside it.
            p.push(0.2 * phase.cos() + 0.5).unwrap();
            p.push(0.2 * phase.sin() - 0.3).unwrap();
        }

        let mut buf = vec![Complex32::new(0.0, 0.0); N];
        let mut corr = IqCorrect::new(DC_BLOCK_HZ, RATE as f64);
        let mut nco = Nco::new(8_000.0, RATE as f64);
        let n = fill_iq(
            &mut c,
            &mut buf,
            1.0,
            &mut QuadratureWatch::default(),
            Some(&mut corr),
            Some(&mut nco),
        );
        assert_eq!(n, N);

        // Judged on the settled tail: the blocker charges over a few hundred
        // samples at this corner.
        let tail = &buf[N - 8192..];
        assert!(
            mean_freq_hz(tail).abs() < 1.0,
            "the station should be on the dial, not {:.0} Hz from it",
            mean_freq_hz(tail)
        );
        // And it is still the station, not what is left of it: the shift has
        // put it on zero, so its amplitude is the mean of the output.
        let mean: Complex32 =
            tail.iter().sum::<Complex32>() / Complex32::new(tail.len() as f32, 0.0);
        assert!((mean.norm() - 0.2).abs() < 0.01, "the station came out at {:.3}", mean.norm());
    }

    /// Which corrector each combination of the two settings asks for. The one
    /// pairing that is not free: an uncorrected offset biases the correlation
    /// the imbalance loop measures, so a corrected rig always has DC removed —
    /// at the operator's corner where they set one, and at the front end's
    /// where they did not.
    #[test]
    fn the_two_settings_pick_the_corrector() {
        let cat = |correction: bool, notch: f64| sdroxide_types::CatConfig {
            iq_correction: correction,
            iq_dc_block_hz: notch,
            ..Default::default()
        };
        let build = |correction, notch, format| {
            build_correction(&cat(correction, notch), format, RATE as f64)
        };

        // Demod audio is one real signal: no quadrature, no image, nothing to do.
        assert!(build(true, 300.0, SoundFormat::DemodAudio).is_none());
        // Both off is genuinely off — no per-sample work at all.
        assert!(build(false, 0.0, SoundFormat::Iq).is_none());
        // Notch alone: DC, and the loop that a mirrored band can mislead left out.
        let mut dc = build(false, 300.0, SoundFormat::Iq).expect("a blocker");
        let mut spoiled = [Complex32::new(1.0, 0.5); 512];
        dc.process(&mut spoiled);
        assert_eq!(dc.estimate(), (0.0, 1.0), "the loop must not have run");
        // Correction alone still blocks DC, because the loop needs it gone.
        assert!(build(true, 0.0, SoundFormat::Iq).is_some());
        assert!(build(true, 300.0, SoundFormat::Iq).is_some());

        // A notch wound past the ceiling is clamped rather than honoured, which
        // is what keeps it a blocker: a corner at or above the sample rate
        // makes the running mean the sample itself and every output zero.
        let mut absurd = build(false, 1e9, SoundFormat::Iq).expect("a blocker");
        let mut tone: Vec<Complex32> = (0..4096)
            .map(|k| {
                let ph = std::f32::consts::TAU * 6_000.0 * k as f32 / RATE;
                Complex32::new(0.5 * ph.cos(), 0.5 * ph.sin())
            })
            .collect();
        absurd.process(&mut tone);
        let worst = tone[2048..].iter().map(|s| s.norm()).fold(f32::MAX, f32::min);
        assert!(worst > 0.4, "a 6 kHz tone came out at {worst:.3} — the notch swallowed the band");
    }

    /// The oscillator carries its phase from one block to the next: a sound
    /// card hands over a few hundred samples at a time, and an oscillator
    /// restarted at each boundary would put a phase step through every signal
    /// on the band at the block rate.
    #[test]
    fn the_shift_is_phase_continuous_across_blocks() {
        let mut nco = Nco::new(8_000.0, RATE as f64);
        let mut whole = [Complex32::new(0.0, 0.0); 512];
        let mut ring = tone_ring(-8_000.0, 512);

        // Same input, drained in two halves through the same oscillator.
        let mut halves = [Complex32::new(0.0, 0.0); 512];
        let a = fill_iq(
            &mut ring,
            &mut halves[..256],
            1.0,
            &mut QuadratureWatch::default(),
            None,
            Some(&mut nco),
        );
        let b = fill_iq(
            &mut ring,
            &mut halves[256..],
            1.0,
            &mut QuadratureWatch::default(),
            None,
            Some(&mut nco),
        );
        assert_eq!((a, b), (256, 256));

        let mut nco = Nco::new(8_000.0, RATE as f64);
        let n = fill_iq(
            &mut tone_ring(-8_000.0, 512),
            &mut whole,
            1.0,
            &mut QuadratureWatch::default(),
            None,
            Some(&mut nco),
        );
        assert_eq!(n, 512);
        for (k, (x, y)) in whole.iter().zip(halves.iter()).enumerate() {
            assert!((x - y).norm() < 1e-4, "sample {k} differs across the block boundary");
        }
    }

    /// The pairing is what makes the stream complex at all: a half-pair left in
    /// the ring has to stay there until its partner arrives, or every sample
    /// after it swaps I for Q and the spectrum is nonsense from then on.
    #[test]
    fn an_odd_sample_is_left_for_its_partner() {
        let (mut p, mut c) = rtrb::RingBuffer::<f32>::new(8);
        for v in [1.0f32, 2.0, 3.0] {
            p.push(v).unwrap();
        }
        let mut buf = [Complex32::new(0.0, 0.0); 4];
        assert_eq!(fill_iq(&mut c, &mut buf, 1.0, &mut QuadratureWatch::default(), None, None), 1);
        assert_eq!(buf[0], Complex32::new(1.0, 2.0));
        // The odd `3.0` is still waiting, and pairs with what arrives next.
        p.push(4.0).unwrap();
        assert_eq!(fill_iq(&mut c, &mut buf, -1.0, &mut QuadratureWatch::default(), None, None), 1);
        assert_eq!(buf[0], Complex32::new(3.0, -4.0));
    }

    /// A window of the stream a rig actually sends: one complex tone, which is
    /// a signal on one side of the dial and nothing on the other.
    fn quad_window(f_hz: f64, rate: f64, n: usize) -> Vec<Complex32> {
        (0..n)
            .map(|k| {
                let t = std::f64::consts::TAU * f_hz * k as f64 / rate;
                Complex32::new(t.cos() as f32, t.sin() as f32)
            })
            .collect()
    }

    /// The arrangement issue #308 arrived as: a radio sending demodulated audio
    /// down a stereo cable, so the same real signal is on both wires. It must
    /// be recognised, and it must be recognised for what it is rather than as a
    /// front end with a bad image — no amount of I/Q correction can help.
    #[test]
    fn one_signal_on_both_wires_is_not_iq() {
        let mut w = QuadratureWatch::default();
        // Three audio tones, as speech or a band of FT8 would be: what matters
        // is that both channels carry the identical numbers.
        let mono: Vec<Complex32> = (0..QUAD_WINDOW_PAIRS)
            .map(|k| {
                let t = k as f64 / 48_000.0;
                let a = (std::f64::consts::TAU * 700.0 * t).sin()
                    + 0.5 * (std::f64::consts::TAU * 1900.0 * t).sin()
                    + 0.3 * (std::f64::consts::TAU * 2600.0 * t).sin();
                Complex32::new(a as f32, a as f32)
            })
            .collect();
        assert_eq!(w.observe(&mono), Some(QuadVerdict::SameOnBoth));
        assert!(quadrature_complaint(QuadVerdict::SameOnBoth).is_some());
        // Said once, not once per window: the operator gets a sentence, not a
        // log full of the same sentence.
        assert_eq!(w.observe(&mono), None);
    }

    /// And a real I/Q stream must not be accused of it — including one with the
    /// imbalance a rig's quadrature mixer really has, which leaves a mirror
    /// image 20 dB down and is exactly what `IqCorrect` is for.
    #[test]
    fn a_real_iq_stream_is_left_alone() {
        let mut w = QuadratureWatch::default();
        assert_eq!(
            w.observe(&quad_window(1500.0, 48_000.0, QUAD_WINDOW_PAIRS)),
            Some(QuadVerdict::Quadrature)
        );

        // 3 dB of amplitude error and 10 degrees of phase error on Q, which is
        // a worse front end than any rig ships.
        let skewed: Vec<Complex32> = (0..QUAD_WINDOW_PAIRS)
            .map(|k| {
                let t = std::f64::consts::TAU * 1500.0 * k as f64 / 48_000.0;
                Complex32::new(t.cos() as f32, (1.41 * (t + 10.0f64.to_radians()).sin()) as f32)
            })
            .collect();
        let mut w = QuadratureWatch::default();
        assert_eq!(
            w.observe(&skewed),
            Some(QuadVerdict::Quadrature),
            "an unbalanced front end is still I/Q"
        );
        // And nothing is said about it: the verdict is the good one.
        assert!(quadrature_complaint(QuadVerdict::Quadrature).is_none());
    }

    /// Half a cable. Reported as its own thing: the remedy is not the same one.
    #[test]
    fn a_silent_channel_is_reported_as_a_silent_channel() {
        let mut w = QuadratureWatch::default();
        let one_sided: Vec<Complex32> = quad_window(1500.0, 48_000.0, QUAD_WINDOW_PAIRS)
            .into_iter()
            .map(|c| Complex32::new(c.re, 0.0))
            .collect();
        assert_eq!(w.observe(&one_sided), Some(QuadVerdict::OneChannelDead));
    }

    /// A sound card's DC offset is common to both channels, so before the means
    /// were taken out it correlated with itself and read as a perfect 1.0 —
    /// a healthy front end told it was carrying one signal twice, on any band
    /// quiet enough for the offset to be the largest thing there.
    #[test]
    fn a_dc_offset_is_not_a_signal_on_both_wires() {
        let mut w = QuadratureWatch::default();
        let offset: Vec<Complex32> = quad_window(1500.0, 48_000.0, QUAD_WINDOW_PAIRS)
            .into_iter()
            .map(|c| Complex32::new(0.02 * c.re + 0.5, 0.02 * c.im + 0.5))
            .collect();
        assert_eq!(w.observe(&offset), Some(QuadVerdict::Quadrature));
    }

    /// Silence on both wires is a radio that is not sending. It must neither be
    /// complained about nor allowed to cancel a complaint already made — the
    /// pause between two overs would otherwise re-arm the warning every time.
    #[test]
    fn silence_says_nothing_either_way() {
        let mut w = QuadratureWatch::default();
        let mono: Vec<Complex32> = (0..QUAD_WINDOW_PAIRS)
            .map(|k| Complex32::new(k as f32 % 3.0, k as f32 % 3.0))
            .collect();
        assert_eq!(w.observe(&mono), Some(QuadVerdict::SameOnBoth));
        let quiet = vec![Complex32::new(0.0, 0.0); QUAD_WINDOW_PAIRS];
        assert_eq!(w.observe(&quiet), None);
        // Still the same verdict standing, so the next live window is not a
        // second warning.
        assert_eq!(w.observe(&mono), None);
    }
}
