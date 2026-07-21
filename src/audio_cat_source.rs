//! An [`IqSource`] for a CAT-controlled rig whose audio arrives over a USB
//! sound card: control (frequency/mode/PTT) goes over serial via
//! [`sdroxide_cat`], RX audio comes from the radio's capture device, and TX
//! audio goes to the radio's playback device. Two sound formats are supported:
//! stereo **IQ** (complex baseband → normal engine path) and mono **demod
//! audio** (real → the engine's audio-band bypass, `DeviceCaps.audio_mode`).

use sdroxide_dsp::MonoResampler;
use sdroxide_radio::rtrb;
use sdroxide_radio::{Complex32, ControlUpdate, IqSource, Result};
use sdroxide_types::{CatConfig, Mode, SoundFormat};

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
        let (in_stream, in_consumer, in_rate) = match opened {
            Ok((s, c)) => {
                let rate = s.sample_rate;
                (Some(s), c, rate)
            }
            Err(e) => {
                tracing::warn!("radio RX audio device unavailable ({e}); RX silent");
                // A dummy, always-empty ring keeps `read` returning silence.
                let (_p, c) = rtrb::RingBuffer::<f32>::new(1);
                (None, c, 48_000.0)
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

    fn display_bandwidth(&self) -> Option<f64> {
        matches!(self.format, SoundFormat::DemodAudio).then_some(self.audio_bw)
    }

    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        self.cat
            .poll()
            .into_iter()
            .map(|u| match u {
                sdroxide_cat::CatUpdate::Freq(hz) => ControlUpdate::Freq(hz),
                sdroxide_cat::CatUpdate::Mode(m) => ControlUpdate::Mode(m),
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
        Ok(())
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
        for &s in &self.tx_scratch {
            let _ = producer.push(s);
            let _ = producer.push(s);
        }
        Ok(())
    }
}
