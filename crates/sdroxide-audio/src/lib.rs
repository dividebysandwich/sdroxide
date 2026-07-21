//! Native audio I/O: a cpal output stream pulling mono samples from a
//! lock-free ring buffer. The DSP engine owns the producer side.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tracing::{info, warn};

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("no audio output device available")]
    NoDevice,
    #[error("no supported f32 output configuration")]
    NoConfig,
    #[error("cpal: {0}")]
    Build(String),
}

pub struct AudioOutput {
    _stream: cpal::Stream,
    /// The rate the stream actually runs at — resample to this.
    pub sample_rate: f64,
    pub channels: u16,
    underruns: Arc<AtomicU64>,
}

impl AudioOutput {
    /// Total output callbacks that ran short of samples.
    pub fn underruns(&self) -> u64 {
        self.underruns.load(Ordering::Relaxed)
    }
}

pub struct AudioInput {
    _stream: cpal::Stream,
    pub sample_rate: f64,
}

/// Dedupe (ALSA lists one entry per sub-PCM with identical descriptions —
/// unpickable by name anyway) and drop pseudo-devices that aren't endpoints.
fn clean_names(raw: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    raw.filter(|n| {
        !(n.starts_with("Rate Converter Plugin")
            || n.starts_with("Plugin ")
            || n.starts_with("Discard all samples"))
    })
    .filter(|n| seen.insert(n.clone()))
    .collect()
}

/// Names of the available output devices (for a device-selection UI).
pub fn output_device_names() -> Vec<String> {
    let host = cpal::default_host();
    host.output_devices()
        .map(|devs| clean_names(devs.filter_map(|d| d.description().ok().map(|n| n.to_string()))))
        .unwrap_or_default()
}

/// Names of the available input devices (for a device-selection UI).
pub fn input_device_names() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|devs| clean_names(devs.filter_map(|d| d.description().ok().map(|n| n.to_string()))))
        .unwrap_or_default()
}

/// Find a device by its enumerated name; falls back to the default device
/// (with a warning) when the name is gone — e.g. the device was unplugged.
fn pick_device(
    host: &cpal::Host,
    name: Option<&str>,
    output: bool,
) -> Result<cpal::Device, AudioError> {
    if let Some(want) = name {
        let found = if output { host.output_devices().ok() } else { host.input_devices().ok() }
            .and_then(|mut devs| {
                devs.find(|d| d.description().map(|n| n.to_string() == want).unwrap_or(false))
            });
        match found {
            Some(d) => return Ok(d),
            None => warn!("audio device {want:?} not found; using default"),
        }
    }
    if output { host.default_output_device() } else { host.default_input_device() }
        .ok_or(AudioError::NoDevice)
}

/// Open an input device (microphone) by name (`None` = system default) and
/// stream mono f32 samples into the returned consumer's ring (channel 0).
pub fn start_input(
    device_name: Option<&str>,
    preferred_rate: u32,
) -> Result<(AudioInput, rtrb::Consumer<f32>), AudioError> {
    let host = cpal::default_host();
    let device = pick_device(&host, device_name, false)?;

    let mut chosen: Option<cpal::StreamConfig> = None;
    if let Ok(configs) = device.supported_input_configs() {
        for range in configs {
            if range.sample_format() != cpal::SampleFormat::F32 {
                continue;
            }
            let rate = preferred_rate.clamp(range.min_sample_rate(), range.max_sample_rate());
            let cfg = cpal::StreamConfig {
                channels: range.channels(),
                sample_rate: rate,
                buffer_size: cpal::BufferSize::Default,
            };
            let exact = rate == preferred_rate;
            match &chosen {
                None => chosen = Some(cfg),
                Some(_) if exact => {
                    chosen = Some(cfg);
                    break;
                }
                Some(_) => {}
            }
        }
    }
    let config = chosen.ok_or(AudioError::NoConfig)?;
    let channels = config.channels as usize;
    let rate = config.sample_rate;

    let (mut producer, consumer) = rtrb::RingBuffer::<f32>::new(rate as usize);
    let stream = device
        .build_input_stream(
            config.clone(),
            move |data: &[f32], _| {
                for frame in data.chunks(channels) {
                    let _ = producer.push(frame[0]);
                }
            },
            |e| warn!("mic stream error: {e}"),
            None,
        )
        .map_err(|e| AudioError::Build(e.to_string()))?;
    stream.play().map_err(|e| AudioError::Build(e.to_string()))?;

    info!(
        rate,
        device = device.description().map(|d| d.to_string()).unwrap_or_default(),
        "mic input running"
    );
    Ok((AudioInput { _stream: stream, sample_rate: rate as f64 }, consumer))
}

/// Open an output device by name (`None` = system default), preferring
/// `preferred_rate` (48 kHz), f32, ≤2 channels. Returns the running stream and
/// the producer to feed with **interleaved stereo** (L, R) frames. Ring
/// capacity is one second.
pub fn start_output(
    device_name: Option<&str>,
    preferred_rate: u32,
) -> Result<(AudioOutput, rtrb::Producer<f32>), AudioError> {
    let host = cpal::default_host();
    let device = pick_device(&host, device_name, true)?;

    let mut chosen: Option<(cpal::StreamConfig, u32)> = None;
    if let Ok(configs) = device.supported_output_configs() {
        for range in configs {
            if range.sample_format() != cpal::SampleFormat::F32 {
                continue;
            }
            let rate = preferred_rate.clamp(range.min_sample_rate(), range.max_sample_rate());
            let channels = range.channels();
            let cfg = cpal::StreamConfig {
                channels,
                sample_rate: rate,
                buffer_size: cpal::BufferSize::Default,
            };
            let exact = rate == preferred_rate && channels <= 2;
            match &chosen {
                None => chosen = Some((cfg, rate)),
                Some(_) if exact => {
                    chosen = Some((cfg, rate));
                    break;
                }
                Some(_) => {}
            }
        }
    }
    let (config, rate) = chosen.ok_or(AudioError::NoConfig)?;
    let channels = config.channels as usize;

    let (producer, mut consumer) = rtrb::RingBuffer::<f32>::new(rate as usize * 2);
    let underruns = Arc::new(AtomicU64::new(0));
    let cb_underruns = underruns.clone();

    let stream = device
        .build_output_stream(
            config.clone(),
            move |data: &mut [f32], _| {
                let mut short = false;
                for frame in data.chunks_mut(channels) {
                    let (l, r) = match (consumer.pop(), consumer.pop()) {
                        (Ok(l), Ok(r)) => (l, r),
                        _ => {
                            short = true;
                            (0.0, 0.0)
                        }
                    };
                    match frame.len() {
                        1 => frame[0] = 0.5 * (l + r),
                        _ => {
                            frame[0] = l;
                            frame[1] = r;
                            frame[2..].fill(0.0);
                        }
                    }
                }
                if short {
                    cb_underruns.fetch_add(1, Ordering::Relaxed);
                }
            },
            |e| warn!("audio stream error: {e}"),
            None,
        )
        .map_err(|e| AudioError::Build(e.to_string()))?;
    stream.play().map_err(|e| AudioError::Build(e.to_string()))?;

    info!(
        rate,
        channels,
        device = device.description().map(|d| d.to_string()).unwrap_or_default(),
        "audio output running"
    );

    Ok((
        AudioOutput {
            _stream: stream,
            sample_rate: rate as f64,
            channels: channels as u16,
            underruns,
        },
        producer,
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn device_enumeration_works() {
        // Must not panic, even on systems without audio; prints what it found.
        let outs = super::output_device_names();
        let ins = super::input_device_names();
        eprintln!("outputs: {outs:?}");
        eprintln!("inputs:  {ins:?}");
    }
}
