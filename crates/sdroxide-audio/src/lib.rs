//! Native audio I/O: a cpal output stream pulling mono samples from a
//! lock-free ring buffer. The DSP engine owns the producer side.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat};
use tracing::{info, warn};

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("no audio device available")]
    NoDevice,
    #[error("the audio device reports no usable sample configuration")]
    NoConfig,
    #[error("cpal: {0}")]
    Build(String),
}

/// Sample formats our converters can read/write to/from f32.
fn supported_format(f: SampleFormat) -> bool {
    matches!(
        f,
        SampleFormat::F32
            | SampleFormat::I16
            | SampleFormat::I32
            | SampleFormat::U16
            | SampleFormat::U8
    )
}

/// Pick the best (config, format) from `ranges`: prefer f32 (no conversion) at
/// `preferred_rate`, then any supported format at that rate, then anything.
/// `want_channels` biases toward configs with at least that many channels.
fn choose_config(
    ranges: impl Iterator<Item = cpal::SupportedStreamConfigRange>,
    preferred_rate: u32,
    want_channels: u16,
) -> Option<(cpal::StreamConfig, SampleFormat)> {
    let mut best: Option<(i32, cpal::StreamConfig, SampleFormat)> = None;
    for range in ranges {
        let fmt = range.sample_format();
        if !supported_format(fmt) {
            continue;
        }
        let rate = preferred_rate.clamp(range.min_sample_rate(), range.max_sample_rate());
        let cfg = cpal::StreamConfig {
            channels: range.channels(),
            sample_rate: rate,
            buffer_size: cpal::BufferSize::Default,
        };
        let mut score = 0;
        if fmt == SampleFormat::F32 {
            score += 8;
        }
        if rate == preferred_rate {
            score += 4;
        }
        if range.channels() >= want_channels {
            score += 2;
        }
        if range.channels() <= 2 {
            score += 1; // avoid the 64-channel "default" ALSA device
        }
        if best.as_ref().map(|(s, _, _)| score > *s).unwrap_or(true) {
            best = Some((score, cfg, fmt));
        }
    }
    best.map(|(_, c, f)| (c, f))
}

/// Build a running input stream that converts any supported sample format to
/// f32 and pushes to `producer` (mono channel 0, or interleaved L/R if `stereo`).
fn spawn_input(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    fmt: SampleFormat,
    stereo: bool,
    mut producer: rtrb::Producer<f32>,
) -> Result<cpal::Stream, AudioError> {
    let channels = config.channels as usize;
    macro_rules! build {
        ($t:ty) => {
            device.build_input_stream(
                config.clone(),
                move |data: &[$t], _| {
                    for frame in data.chunks(channels) {
                        let l: f32 = frame[0].to_sample::<f32>();
                        let _ = producer.push(l);
                        if stereo {
                            let r = frame.get(1).copied().unwrap_or(frame[0]);
                            let _ = producer.push(r.to_sample::<f32>());
                        }
                    }
                },
                |e| warn!("input stream error: {e}"),
                None,
            )
        };
    }
    let stream = match fmt {
        SampleFormat::F32 => build!(f32),
        SampleFormat::I16 => build!(i16),
        SampleFormat::I32 => build!(i32),
        SampleFormat::U16 => build!(u16),
        SampleFormat::U8 => build!(u8),
        other => return Err(AudioError::Build(format!("unsupported input format {other:?}"))),
    }
    .map_err(|e| AudioError::Build(e.to_string()))?;
    stream.play().map_err(|e| AudioError::Build(e.to_string()))?;
    Ok(stream)
}

/// Build a running output stream that pulls interleaved-stereo f32 from
/// `consumer`, down/up-mixes to the device's channel count, and converts to the
/// device's native sample format.
fn spawn_output(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    fmt: SampleFormat,
    mut consumer: rtrb::Consumer<f32>,
    underruns: Arc<AtomicU64>,
) -> Result<cpal::Stream, AudioError> {
    let channels = config.channels as usize;
    macro_rules! build {
        ($t:ty) => {
            device.build_output_stream(
                config.clone(),
                move |data: &mut [$t], _| {
                    let mut short = false;
                    for frame in data.chunks_mut(channels) {
                        let (l, r) = match (consumer.pop(), consumer.pop()) {
                            (Ok(l), Ok(r)) => (l, r),
                            _ => {
                                short = true;
                                (0.0f32, 0.0f32)
                            }
                        };
                        match frame.len() {
                            1 => frame[0] = (0.5 * (l + r)).to_sample::<$t>(),
                            _ => {
                                frame[0] = l.to_sample::<$t>();
                                frame[1] = r.to_sample::<$t>();
                                for x in &mut frame[2..] {
                                    *x = 0.0f32.to_sample::<$t>();
                                }
                            }
                        }
                    }
                    if short {
                        underruns.fetch_add(1, Ordering::Relaxed);
                    }
                },
                |e| warn!("output stream error: {e}"),
                None,
            )
        };
    }
    let stream = match fmt {
        SampleFormat::F32 => build!(f32),
        SampleFormat::I16 => build!(i16),
        SampleFormat::I32 => build!(i32),
        SampleFormat::U16 => build!(u16),
        SampleFormat::U8 => build!(u8),
        other => return Err(AudioError::Build(format!("unsupported output format {other:?}"))),
    }
    .map_err(|e| AudioError::Build(e.to_string()))?;
    stream.play().map_err(|e| AudioError::Build(e.to_string()))?;
    Ok(stream)
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

/// True if the device reports at least one sample configuration we can use.
fn has_usable_config(device: &cpal::Device, output: bool) -> bool {
    if output {
        device
            .supported_output_configs()
            .map(|mut cs| cs.any(|c| supported_format(c.sample_format())))
            .unwrap_or(false)
    } else {
        device
            .supported_input_configs()
            .map(|mut cs| cs.any(|c| supported_format(c.sample_format())))
            .unwrap_or(false)
    }
}

/// Find a device by its enumerated name; falls back to the default device
/// (with a warning) when the name is gone — e.g. the device was unplugged.
/// ALSA lists several sub-PCMs under one description, and some report no usable
/// config, so among same-named devices prefer one that actually has one.
fn pick_device(
    host: &cpal::Host,
    name: Option<&str>,
    output: bool,
) -> Result<cpal::Device, AudioError> {
    if let Some(want) = name {
        let matching: Vec<cpal::Device> =
            if output { host.output_devices().ok() } else { host.input_devices().ok() }
                .map(|devs| {
                    devs.filter(|d| {
                        d.description().map(|n| n.to_string() == want).unwrap_or(false)
                    })
                    .collect()
                })
                .unwrap_or_default();
        if let Some(i) = matching.iter().position(|d| has_usable_config(d, output)) {
            return Ok(matching.into_iter().nth(i).unwrap());
        }
        if let Some(d) = matching.into_iter().next() {
            return Ok(d);
        }
        warn!("audio device {want:?} not found; using default");
    }
    if output { host.default_output_device() } else { host.default_input_device() }
        .ok_or(AudioError::NoDevice)
}

/// Open an input device (microphone) by name (`None` = system default) and
/// stream mono f32 samples into the returned consumer's ring (channel 0).
/// Accepts any native sample format (i16/i32/u16/u8/f32), converting to f32.
pub fn start_input(
    device_name: Option<&str>,
    preferred_rate: u32,
) -> Result<(AudioInput, rtrb::Consumer<f32>), AudioError> {
    let host = cpal::default_host();
    let device = pick_device(&host, device_name, false)?;

    let (config, fmt) = device
        .supported_input_configs()
        .ok()
        .and_then(|configs| choose_config(configs, preferred_rate, 1))
        .ok_or(AudioError::NoConfig)?;
    let rate = config.sample_rate;

    let (producer, consumer) = rtrb::RingBuffer::<f32>::new(rate as usize);
    let stream = spawn_input(&device, &config, fmt, false, producer)?;

    info!(
        rate,
        format = ?fmt,
        device = device.description().map(|d| d.to_string()).unwrap_or_default(),
        "mic input running"
    );
    Ok((AudioInput { _stream: stream, sample_rate: rate as f64 }, consumer))
}

/// Like [`start_input`] but keeps the first TWO channels interleaved (L, R) —
/// used to read complex I/Q from a radio's stereo sound card. A mono device
/// degrades to duplicated samples. Accepts any native sample format.
pub fn start_input_stereo(
    device_name: Option<&str>,
    preferred_rate: u32,
) -> Result<(AudioInput, rtrb::Consumer<f32>), AudioError> {
    let host = cpal::default_host();
    let device = pick_device(&host, device_name, false)?;

    let (config, fmt) = device
        .supported_input_configs()
        .ok()
        .and_then(|configs| choose_config(configs, preferred_rate, 2))
        .ok_or(AudioError::NoConfig)?;
    let rate = config.sample_rate;
    let channels = config.channels;

    let (producer, consumer) = rtrb::RingBuffer::<f32>::new(rate as usize * 2);
    let stream = spawn_input(&device, &config, fmt, true, producer)?;
    info!(rate, channels, format = ?fmt, "radio IQ input running");
    Ok((AudioInput { _stream: stream, sample_rate: rate as f64 }, consumer))
}

/// Open an output device by name (`None` = system default), preferring
/// `preferred_rate` (48 kHz), f32, ≤2 channels. Returns the running stream and
/// the producer to feed with **interleaved stereo** (L, R) frames. Ring
/// capacity is one second. Accepts any native sample format, converting from
/// f32.
pub fn start_output(
    device_name: Option<&str>,
    preferred_rate: u32,
) -> Result<(AudioOutput, rtrb::Producer<f32>), AudioError> {
    let host = cpal::default_host();
    let device = pick_device(&host, device_name, true)?;

    let (config, fmt) = device
        .supported_output_configs()
        .ok()
        .and_then(|configs| choose_config(configs, preferred_rate, 2))
        .ok_or(AudioError::NoConfig)?;
    let rate = config.sample_rate;
    let channels = config.channels;

    let (producer, consumer) = rtrb::RingBuffer::<f32>::new(rate as usize * 2);
    let underruns = Arc::new(AtomicU64::new(0));

    let stream = spawn_output(&device, &config, fmt, consumer, underruns.clone())?;

    info!(
        rate,
        channels,
        format = ?fmt,
        device = device.description().map(|d| d.to_string()).unwrap_or_default(),
        "audio output running"
    );

    Ok((
        AudioOutput {
            _stream: stream,
            sample_rate: rate as f64,
            channels,
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
