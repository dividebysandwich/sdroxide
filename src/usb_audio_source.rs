//! An [`IqSource`] for a radio that has no control port at all — a handheld, a
//! walkie, a toy, a USB-dongle rig: everything is sound cards. Receive arrives
//! from one of the computer's input devices as already-demodulated audio (the
//! radio's headphone socket), transmit leaves as audio into one of its output
//! devices (the radio's mic socket), and the radio keys itself — there is no
//! PTT to command by definition, so it is the rig's VOX firing on the audio it
//! hears at that mic line.
//!
//! The shape is the audio half of [`crate::audio_cat_source::AudioCatSource`]
//! minus the serial: mono 48 kHz demod audio in, real TX audio out, a narrow
//! audio-band panadapter, and no dial to command anywhere — the centre is a
//! label the operator keeps in the app, because nothing else owns a knob.

use sdroxide_dsp::MonoResampler;
use sdroxide_radio::rtrb;
use sdroxide_radio::{Complex32, IqSource, Result};

/// The audio band the engine shows this radio's panadapter over, Hz each side
/// of the dial. Demod audio arrives already inside the radio's own filter, so
/// this is just the width of the picture, not a filter of ours.
///
/// A handheld's speaker path is roughly 4 kHz of audio, the same figure the
/// demod-audio CAT path defaults to ([`sdroxide_types::CatConfig::audio_bw_hz`]).
const AUDIO_BW_HZ: f64 = 4000.0;

/// The first missing-box warning also says what the operator can expect; this
/// one says who must act now.
pub struct UsbAudioSource {
    // RX audio from the radio (mono demod). `None` when the capture device
    // could not be opened — the app still runs so the user can fix the device
    // in Settings; RX is just silent until then.
    in_stream: Option<sdroxide_audio::AudioInput>,
    in_consumer: rtrb::Consumer<f32>,
    in_rate: f64,
    /// Capture frames the sound card had to throw away, and when that was last
    /// looked at. Watched because dropping them is silent everywhere else — see
    /// [`Self::check_dropped`].
    drops: DropWatch,

    // TX audio to the radio (interleaved stereo playback ring, exactly as the
    // CAT source's).
    out: Option<(sdroxide_audio::AudioOutput, rtrb::Producer<f32>)>,
    tx_resampler: Option<MonoResampler>,
    tx_scratch: Vec<f32>,

    /// Where the operator says the radio is tuned. Nothing here can move the
    /// radio's own dial, so this is information the app keeps for the CB
    /// channel bookkeeping, the digi identity and the log; it never becomes a
    /// command to hardware ([`IqSource::center_is_dial`] is false).
    center: f64,
    /// Whether a mode that follows the band has to be translated to a plain
    /// sideband by the engine before this source sees it. False: there is no
    /// sideband to command, the radio sits wherever its own knob is, and
    /// whatever arrives at the mic line is what goes out.
    label: String,
    /// Warning captured at open time (RX device unavailable), surfaced to the
    /// UI. `None` when RX came up cleanly.
    status: Option<String>,
}

impl UsbAudioSource {
    /// Open the radio's sound-card streams. `audio_in` / `audio_out` are cpal
    /// device names (`None` = system default). `center_hz` is the initial dial,
    /// taken from the same place every non-CAT front end takes its own.
    pub fn open(
        audio_in: Option<&str>,
        audio_out: Option<&str>,
        center_hz: f64,
    ) -> anyhow::Result<Self> {
        // A radio with no sound card named falls back to the machine's default
        // input, which is almost never the radio — it is the operator's
        // headset, or, at a station with two rigs on two identical USB codecs,
        // the *other* radio's card. Worth saying out loud, exactly as the CAT
        // path does.
        if audio_in.is_none() || audio_out.is_none() {
            tracing::warn!(
                "no sound card chosen for the USB audio radio ({}) — falling back to the system \
                 default, which is not this radio unless it happens to be the default. Pick its \
                 devices under Settings → General → Radio audio.",
                match (audio_in.is_none(), audio_out.is_none()) {
                    (true, true) => "receive and transmit",
                    (true, false) => "receive",
                    _ => "transmit",
                },
            );
        }

        // RX capture is best-effort: a missing/unsupported device leaves RX
        // silent but keeps the app (and its Settings dialog) alive. Demod audio
        // is fixed at 48 kHz, the same as the CAT path: it arrives already
        // inside the radio's own filter.
        let opened = sdroxide_audio::start_input_buffered(audio_in, 48_000);
        let dev_label = audio_in.unwrap_or("system default");
        // A dummy, always-empty ring keeps `read` returning silence when RX is
        // unavailable or guarded off.
        let silent = || {
            let (_p, c) = rtrb::RingBuffer::<f32>::new(1);
            c
        };
        let (in_stream, in_consumer, in_rate, in_status) = match opened {
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

        // TX playback is best-effort: a missing device just means no TX audio
        // reaching the radio, which on a VOX-keyed radio is the same as never
        // keying.
        let (out, out_status) = match sdroxide_audio::start_output(audio_out, 48_000) {
            Ok((o, p)) => (Some((o, p)), None),
            Err(e) => {
                let msg = format!(
                    "Radio transmit device unavailable ({e}) — nothing will key the radio. \
                     It keys off VOX, so no audio here means no over."
                );
                tracing::warn!("{msg}");
                (None, Some(msg))
            }
        };
        let tx_resampler =
            out.as_ref().and_then(|(o, _)| MonoResampler::new(48_000.0, o.sample_rate));

        let status = match (in_status, out_status) {
            (Some(a), Some(b)) => Some(format!("{a}\n{b}")),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        Ok(UsbAudioSource {
            in_stream,
            in_consumer,
            in_rate,
            drops: DropWatch::started(std::time::Instant::now()),
            out,
            tx_resampler,
            tx_scratch: Vec::new(),
            center: center_hz,
            label: format!("USB audio radio on {dev_label}"),
            status,
        })
    }

    /// Report capture frames the sound card dropped since the last look.
    ///
    /// A dropped frame is a hole in the audio stream, and the only symptom is
    /// audio that clicks, stutters or stops being intelligible. The panadapter
    /// does not care — every block it transforms is still real signal, so a
    /// stream with pieces missing paints a spectrum indistinguishable from an
    /// intact one — which is why the failure is worth a line of its own and a
    /// second, separate strike: the drop counter is the only thing that says
    /// the splice was the card's, not the DSP's.
    ///
    /// What it must not report is an over. Nobody reads this stream while the
    /// radio transmits, so the ring overflows every time regardless of how fast
    /// the machine is — see [`Self::discard_pending_rx`], which is where those
    /// frames are excused.
    fn check_dropped(&mut self) {
        let Some(total) = self.in_stream.as_ref().map(|s| s.dropped_frames()) else { return };
        let Some((lost, window)) = self.drops.check(std::time::Instant::now(), total) else {
            return;
        };
        // This card is fixed at 48 kHz and nothing else, so a remedy that tells
        // its owner to lower a rate that does not exist is how an operator
        // learns to stop reading the warnings.
        tracing::warn!(
            "radio audio: {lost} capture frames dropped in the last {:.1} s ({:.1} ms of signal) \
             — this machine is not emptying a {:.0} Hz card fast enough. The panadapter will \
             look fine and the demodulated audio will break up. This card is fixed at 48 kHz, \
             so there is no rate to lower — look instead at what else on this machine is \
             keeping sdroxide off the CPU.",
            window.as_secs_f64(),
            lost as f64 * 1000.0 / self.in_rate,
            self.in_rate,
        );
    }
}

/// The capture counter's bookkeeping: what it read last time and when.
///
/// Split out the same way the CAT source does — this is arithmetic about
/// wall-clock time and a monotonic total, which is exactly where it went wrong
/// before and exactly what should be checkable without a radio plugged in.
struct DropWatch {
    /// The card's lifetime drop total as of the last look.
    seen: u64,
    /// When that look happened — the time it *did*, not the time the next one
    /// is due.
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

/// The soonest [`UsbAudioSource::check_dropped`] looks again — the same window
/// the CAT source uses, for the same reason.
const DROP_CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

impl IqSource for UsbAudioSource {
    fn sample_rate(&self) -> f64 {
        self.in_rate
    }
    fn center_hz(&self) -> f64 {
        self.center
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        // Kept in the app only — there is no knob on this radio to move.
        self.center = hz;
        Ok(())
    }

    /// False on purpose: nothing here can move the radio's dial, so the engine
    /// tunes within whatever the radio is already sending and the operator's
    /// entry just relabels the centre (see [`IqSource::center_is_dial`]).
    fn center_is_dial(&self) -> bool {
        false
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        self.check_dropped();
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

    /// The same drain as [`Self::read`] without the nap on an empty ring, for
    /// the same reason the CAT source overrides it: a radio lent out as another
    /// radio's panadapter is drained exclusively through here.
    fn read_available(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        self.check_dropped();
        let mut n = 0;
        while n < buf.len() {
            let Ok(s) = self.in_consumer.pop() else { break };
            buf[n] = Complex32::new(s, 0.0);
            n += 1;
        }
        Ok(n)
    }

    fn describe(&self) -> String {
        self.label.clone()
    }

    fn open_status(&self) -> Option<String> {
        self.status.clone()
    }

    fn display_bandwidth(&self) -> Option<f64> {
        Some(AUDIO_BW_HZ)
    }

    /// CW typed at the panel goes out as keyed audio through the sound card
    /// (MCW), where the radio's VOX keys it — the only way this radio can be
    /// keyed at all. That needs the radio left wherever it is; it is a
    /// sideband rig, so there is no CW position to be put in and out of.
    fn cw_audio_keyed(&self) -> bool {
        true
    }

    fn tx_begin(&mut self, _center_hz: f64, _rate: f64) -> Result<f64> {
        // The radio keys itself: its VOX fires on the first audio that reaches
        // the mic line, so there is nothing to command here. Return the card
        // rate the engine feeds TX audio at.
        Ok(self.out.as_ref().map(|(o, _)| o.sample_rate).unwrap_or(self.in_rate))
    }

    fn tx_end(&mut self) -> Result<()> {
        Ok(())
    }

    fn discard_pending_rx(&mut self) {
        // The capture callback keeps filling this ring during TX too, and the
        // engine stops reading a half-duplex source for the length of an over —
        // the same backlog and the same overrun accounting as the CAT source,
        // and the same excusing, right here at the unkey.
        while self.in_consumer.pop().is_ok() {}
        if let Some(total) = self.in_stream.as_ref().map(|s| s.dropped_frames()) {
            self.drops.rebase(std::time::Instant::now(), total);
        }
    }

    fn tx_write_audio(&mut self, audio: &[f32]) -> Result<()> {
        let Some((_, producer)) = self.out.as_mut() else {
            return Ok(()); // no TX device — nothing keyed, and nothing to send
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
        // silence spliced in wherever it ran dry (issue #247).
        self.tx_scratch.clear();
        match self.tx_resampler.as_mut() {
            Some(rs) => rs.push(audio, &mut self.tx_scratch),
            None => self.tx_scratch.extend_from_slice(audio),
        }
        // Block until the card drains room, applying backpressure so the engine's
        // TX loop is paced to real time. Without this a long continuous burst
        // is generated at CPU speed and mostly dropped on a full ring, so the
        // radio only transmits the first buffer-full — and on a VOX radio the
        // gap that follows is a key that drops out mid-sentence.
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
        // The output ring holds ~1 s; wait for it to play out so the VOX tail
        // of a burst (critical for FT8 decode) isn't cut before the radio
        // unkeys. `tx_end` (which releases the "PTT", such as it is on a VOX
        // radio) comes after this.
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
