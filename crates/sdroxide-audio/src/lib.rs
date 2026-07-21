//! Native audio I/O: a cpal output stream pulling mono samples from a
//! lock-free ring buffer. The DSP engine owns the producer side.

use std::collections::{HashMap, HashSet};
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
    /// Channels the capture stream actually runs with (1 = mono; IQ needs ≥2).
    pub channels: u16,
}

/// Extract the ALSA card id ("Device_1") from a cpal driver/pcm id like
/// "sysdefault:CARD=Device_1" or "hw:CARD=Device,DEV=0". `None` for virtual
/// devices ("default", "pipewire", …) and non-ALSA platforms.
fn alsa_card_id(pcm_id: &str) -> Option<String> {
    let rest = pcm_id.split("CARD=").nth(1)?;
    let end = rest.find([',', ':']).unwrap_or(rest.len());
    let id = rest[..end].trim();
    (!id.is_empty()).then(|| id.to_string())
}

/// Trim the "at usb-…, full speed" tail off an ALSA longname, leaving the
/// readable "manufacturer model" part.
fn prettify_longname(long: &str) -> String {
    long.split(" at ").next().unwrap_or(long).trim().to_string()
}

fn is_pseudo(name: &str) -> bool {
    name.starts_with("Rate Converter Plugin")
        || name.starts_with("Plugin ")
        || name.starts_with("Discard all samples")
}

/// One ALSA card's identity for building readable, unique device names.
#[derive(Clone)]
struct AlsaCard {
    /// Numeric card index ("5") — used to read /proc/asound/card5/*.
    index: String,
    /// Stable card id ("Device", "Device_1") — distinguishes identical models.
    id: String,
    /// Manufacturer text from the longname, e.g. "C-Media Electronics Inc.".
    vendor: String,
    /// USB "vid:pid", e.g. "0d8c:0012" — differentiates same-named dongles.
    usbid: String,
}

/// Linux: map every ALSA card *index* ("5") and *id* ("Device") to its identity,
/// so a pcm id using either form ("hw:CARD=5" / "sysdefault:CARD=Device")
/// resolves to the same card. Empty off-Linux.
fn alsa_cards() -> HashMap<String, AlsaCard> {
    #[allow(unused_mut)]
    let mut map = HashMap::new();
    #[cfg(target_os = "linux")]
    if let Ok(text) = std::fs::read_to_string("/proc/asound/cards") {
        // Records are two lines:
        //   " 5 [Device         ]: USB-Audio - USB Audio Device"
        //   "                      C-Media Electronics Inc. USB Audio Device at usb-..., full speed"
        let lines: Vec<&str> = text.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            let head = lines[i];
            if let (Some(lb), Some(rb)) = (head.find('['), head.find(']')) {
                if lb < rb {
                    let index = head[..lb].trim().to_string();
                    let id = head[lb + 1..rb].trim().to_string();
                    // Header tail after "]: <driver> - <shortname>".
                    let shortname = head[rb + 1..].split(" - ").nth(1).unwrap_or("").trim();
                    let pretty = prettify_longname(lines.get(i + 1).map(|s| s.trim()).unwrap_or(""));
                    // Vendor = longname with the model (shortname) trimmed off.
                    let vendor = pretty
                        .strip_suffix(shortname)
                        .unwrap_or(&pretty)
                        .trim()
                        .to_string();
                    let usbid = std::fs::read_to_string(format!("/proc/asound/card{index}/usbid"))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();
                    if !id.is_empty() {
                        let card = AlsaCard { index: index.clone(), id: id.clone(), vendor, usbid };
                        if !index.is_empty() {
                            map.insert(index, card.clone());
                        }
                        map.insert(id, card);
                    }
                    i += 2;
                    continue;
                }
            }
            i += 1;
        }
    }
    map
}

/// The ALSA card id token ("Device_1" / "5") a cpal device opens through.
fn device_card_id(device: &cpal::Device) -> Option<String> {
    alsa_card_id(device.description().ok()?.driver()?)
}

/// Linux: the true maximum hardware capture channel count for a card, read from
/// `/proc/asound/cardN/stream0`. This sees past ALSA's plug/dmix layer, which
/// upmixes a mono microphone to a fake stereo config — so it's the only
/// reliable way to tell that a "stereo" capture is really mono (no good for
/// I/Q). `None` off-Linux or when the file is absent.
fn hw_capture_channels(index: &str) -> Option<u16> {
    #[cfg(target_os = "linux")]
    {
        let text = std::fs::read_to_string(format!("/proc/asound/card{index}/stream0")).ok()?;
        let mut in_capture = false;
        let mut max = 0u16;
        for line in text.lines() {
            let t = line.trim();
            match t {
                "Capture:" => in_capture = true,
                "Playback:" => in_capture = false,
                _ if in_capture => {
                    if let Some(rest) = t.strip_prefix("Channels:") {
                        if let Ok(n) = rest.trim().parse::<u16>() {
                            max = max.max(n);
                        }
                    }
                }
                _ => {}
            }
        }
        return (max > 0).then_some(max);
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = index;
        None
    }
}

/// User-facing name for one device, or `None` to exclude it (pseudo-plugins,
/// raw usb-stream nodes). ALSA cards get "vendor model [card-id · vid:pid]" so
/// two identically-named dongles (e.g. a pair of C-Media adapters) stay
/// distinct while separate sub-devices (HDMI 0 vs HDMI 2) are kept apart by the
/// base name. Virtual devices ("default", "pipewire", …) keep their name.
fn device_display(device: &cpal::Device, cards: &HashMap<String, AlsaCard>) -> Option<String> {
    let desc = device.description().ok()?;
    let pcm = desc.driver().unwrap_or("");
    if pcm.starts_with("usbstream:") {
        return None; // raw USB stream node, not a normal capture/playback PCM
    }
    let base = desc.name().to_string();
    if base.is_empty() || is_pseudo(&base) {
        return None;
    }
    match alsa_card_id(pcm) {
        Some(raw) => {
            let card = cards.get(&raw);
            let vendor = card.map(|c| c.vendor.as_str()).unwrap_or("");
            let id = card.map(|c| c.id.as_str()).unwrap_or(raw.as_str());
            let usbid = card.map(|c| c.usbid.as_str()).unwrap_or("");
            let mut name = String::new();
            if !vendor.is_empty() && !base.contains(vendor) {
                name.push_str(vendor);
                name.push(' ');
            }
            name.push_str(&base);
            name.push_str(" [");
            name.push_str(id);
            if !usbid.is_empty() {
                name.push_str(" · ");
                name.push_str(usbid);
            }
            name.push(']');
            Some(name)
        }
        None => Some(base), // virtual/default device
    }
}

/// Enumerate devices for one direction as (device, unique display name),
/// deduping identical display names (several sub-PCMs / index-vs-id forms of one
/// card collapse to a single entry).
fn enumerate_named(host: &cpal::Host, output: bool) -> Vec<(cpal::Device, String)> {
    let cards = alsa_cards();
    let devs = if output { host.output_devices().ok() } else { host.input_devices().ok() };
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    if let Some(devs) = devs {
        for d in devs {
            let Some(name) = device_display(&d, &cards) else { continue };
            if seen.insert(name.clone()) {
                out.push((d, name));
            }
        }
    }
    out
}

/// Names of the available output devices (for a device-selection UI).
pub fn output_device_names() -> Vec<String> {
    enumerate_named(&cpal::default_host(), true).into_iter().map(|(_, n)| n).collect()
}

/// Names of the available input devices (for a device-selection UI).
pub fn input_device_names() -> Vec<String> {
    enumerate_named(&cpal::default_host(), false).into_iter().map(|(_, n)| n).collect()
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

/// Find a device by its enumerated (enriched) name; falls back to the default
/// device (with a warning) when the name is gone — e.g. the device was
/// unplugged. Matches the enriched display name first, then the plain cpal name
/// for configs saved before names carried the manufacturer/card id. Among
/// matches, prefers one that actually reports a usable config.
fn pick_device(
    host: &cpal::Host,
    name: Option<&str>,
    output: bool,
) -> Result<cpal::Device, AudioError> {
    if let Some(want) = name {
        let cards = alsa_cards();
        let all: Vec<cpal::Device> =
            if output { host.output_devices().ok() } else { host.input_devices().ok() }
                .map(|d| d.collect())
                .unwrap_or_default();
        // 1) enriched-name match (all sub-PCMs of the card); 2) legacy plain
        // cpal-name match for configs saved before names carried the vendor/id.
        let mut idxs: Vec<usize> = all
            .iter()
            .enumerate()
            .filter(|(_, d)| device_display(d, &cards).as_deref() == Some(want))
            .map(|(i, _)| i)
            .collect();
        if idxs.is_empty() {
            idxs = all
                .iter()
                .enumerate()
                .filter(|(_, d)| {
                    // Skip excluded nodes (usbstream/pseudo) even in legacy mode.
                    device_display(d, &cards).is_some()
                        && d.description().ok().map(|x| x.name() == want).unwrap_or(false)
                })
                .map(|(i, _)| i)
                .collect();
        }
        if !idxs.is_empty() {
            let best = idxs
                .iter()
                .copied()
                .find(|&i| has_usable_config(&all[i], output))
                .unwrap_or(idxs[0]);
            return Ok(all.into_iter().nth(best).unwrap());
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
    let channels = config.channels;

    let (producer, consumer) = rtrb::RingBuffer::<f32>::new(rate as usize);
    let stream = spawn_input(&device, &config, fmt, false, producer)?;

    info!(
        rate,
        format = ?fmt,
        device = device.description().map(|d| d.to_string()).unwrap_or_default(),
        "mic input running"
    );
    Ok((AudioInput { _stream: stream, sample_rate: rate as f64, channels }, consumer))
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
    // Report the TRUE hardware channel count, not cpal's — the ALSA plug layer
    // upmixes a mono mic to a fake 2-channel config, which would otherwise slip
    // past the caller's mono-for-IQ guard. Fall back to cpal's count when the
    // hardware count is unknown (non-Linux, or a virtual device).
    let hw_channels = alsa_cards()
        .get(&device_card_id(&device).unwrap_or_default())
        .and_then(|c| hw_capture_channels(&c.index));
    let channels = hw_channels.unwrap_or(config.channels);

    let (producer, consumer) = rtrb::RingBuffer::<f32>::new(rate as usize * 2);
    let stream = spawn_input(&device, &config, fmt, true, producer)?;
    info!(rate, stream_channels = config.channels, hw_channels = channels, format = ?fmt, "radio IQ input running");
    Ok((AudioInput { _stream: stream, sample_rate: rate as f64, channels }, consumer))
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
