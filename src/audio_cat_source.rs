//! An [`IqSource`] for a CAT-controlled rig whose audio arrives over a USB
//! sound card: control (frequency/mode/PTT) goes over serial via
//! [`sdroxide_cat`], RX audio comes from the radio's capture device, and TX
//! audio goes to the radio's playback device. Two sound formats are supported:
//! stereo **IQ** (complex baseband → normal engine path) and mono **demod
//! audio** (real → the engine's audio-band bypass, `DeviceCaps.audio_mode`).

use sdroxide_dsp::MonoResampler;
use sdroxide_radio::rtrb;
use sdroxide_radio::{Complex32, ControlUpdate, IqSource, Result};
use sdroxide_types::{CatConfig, Mode, SoundFormat, TxTelemetry};

pub struct AudioCatSource {
    // RX audio from the rig (mono for demod, interleaved L/R for IQ). `None`
    // when the capture device could not be opened — the app still runs so the
    // user can fix the device in Settings; RX is just silent until then.
    _in_stream: Option<sdroxide_audio::AudioInput>,
    in_consumer: rtrb::Consumer<f32>,
    in_rate: f64,
    format: SoundFormat,
    audio_bw: f64,

    // TX audio to the rig (interleaved stereo playback ring).
    out: Option<(sdroxide_audio::AudioOutput, rtrb::Producer<f32>)>,
    tx_resampler: Option<MonoResampler>,
    tx_scratch: Vec<f32>,

    cat: sdroxide_cat::CatHandle,
    center: f64,
    label: String,
    /// Warning captured at open time (RX device unavailable / mono-for-IQ),
    /// surfaced to the UI. `None` when RX came up cleanly.
    status: Option<String>,
    /// Latest SWR the rig reported while keyed (via CI-V meter reads), held so
    /// the engine's 100 ms meter poll sees the most recent value between the
    /// rig's ~5 Hz updates. Cleared on unkey.
    last_telem: Option<TxTelemetry>,
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
        let (init_freq, _init_mode) = sdroxide_cat::query_once(&cfg).unwrap_or((None, None));
        let center = init_freq.unwrap_or(14_074_000.0);

        // RX capture is best-effort: a missing/unsupported device leaves RX
        // silent but keeps the app (and its Settings dialog) alive.
        let opened = match cfg.format {
            SoundFormat::Iq => sdroxide_audio::start_input_stereo(audio_in, 48_000),
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
        let tx_resampler = out.as_ref().and_then(|(o, _)| MonoResampler::new(48_000.0, o.sample_rate));

        let label = format!("CAT rig ({}) on {}", cfg.family.label(), cfg.serial.path);
        let audio_bw = cfg.audio_bw_hz;
        let format = cfg.format;
        let cat = sdroxide_cat::spawn(cfg);

        Ok(AudioCatSource {
            _in_stream: in_stream,
            in_consumer,
            in_rate,
            format,
            audio_bw,
            out,
            tx_resampler,
            tx_scratch: Vec::new(),
            cat,
            center,
            label,
            status,
            last_telem: None,
        })
    }
}

impl IqSource for AudioCatSource {
    fn sample_rate(&self) -> f64 {
        self.in_rate
    }
    fn center_hz(&self) -> f64 {
        self.center
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        self.cat.set_freq(hz);
        Ok(())
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
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
                let mut n = 0;
                // Need pairs (I, Q); only consume when both are available.
                while n < buf.len() && self.in_consumer.slots() >= 2 {
                    let i = self.in_consumer.pop().unwrap_or(0.0);
                    let q = self.in_consumer.pop().unwrap_or(0.0);
                    buf[n] = Complex32::new(i, q);
                    n += 1;
                }
                if n == 0 {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Ok(n)
            }
        }
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

    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        self.cat
            .poll()
            .into_iter()
            .filter_map(|u| match u {
                sdroxide_cat::CatUpdate::Freq(hz) => Some(ControlUpdate::Freq(hz)),
                sdroxide_cat::CatUpdate::Mode(m) => Some(ControlUpdate::Mode(m)),
                // SWR arrives on the separate telemetry channel, not here.
                sdroxide_cat::CatUpdate::Swr(_) => None,
            })
            .collect()
    }

    fn set_control_mode(&mut self, mode: Mode) -> Result<()> {
        self.cat.set_mode(mode);
        Ok(())
    }

    fn tx_begin(&mut self, _center_hz: f64, _rate: f64) -> Result<f64> {
        self.cat.set_ptt(true);
        Ok(self.out.as_ref().map(|(o, _)| o.sample_rate).unwrap_or(self.in_rate))
    }

    fn tx_end(&mut self) -> Result<()> {
        self.cat.set_ptt(false);
        self.last_telem = None; // drop the stale SWR reading on unkey
        Ok(())
    }

    fn tx_telemetry(&mut self) -> Option<TxTelemetry> {
        // The CI-V thread polls SWR at ~5 Hz; latch its latest reading so the
        // engine's 100 ms meter tick always has a value to show.
        if let Some(t) = self.cat.poll_telemetry() {
            self.last_telem = Some(t);
        }
        self.last_telem
    }

    fn tx_write_audio(&mut self, audio: &[f32]) -> Result<()> {
        let Some((_, producer)) = self.out.as_mut() else {
            return Ok(()); // no TX audio device — PTT still keyed the rig
        };
        // Resample 48 kHz → card rate, then interleave to stereo (both channels).
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
