//! Native audio I/O: a cpal output stream pulling mono samples from a
//! lock-free ring buffer. The DSP engine owns the producer side.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

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

/// How long one refused open may take before the remaining candidate
/// configurations are abandoned.
///
/// A device that merely won't run a configuration says so in milliseconds, so
/// working down the list costs nothing. A device the sound stack is *stuck* on
/// is a different animal: ALSA's PulseAudio plugin asked to open a PipeWire
/// monitor of a card sdroxide already holds waits its full 30-second timeout
/// before failing, and every further candidate spends that timeout again. Once
/// an open has been that slow, the configuration was never the problem.
const SLOW_REFUSAL: Duration = Duration::from_secs(5);

/// Configs to try opening, in order: the best pick from the advertised
/// ranges, then the device's own default. The default matters on Windows:
/// WASAPI in shared mode only promises the mix format it reports as the
/// default, and a config assembled from the advertised ranges can still come
/// back "Stream configuration is not supported in shared mode".
fn config_candidates(
    picked: Option<(cpal::StreamConfig, SampleFormat)>,
    default: Result<cpal::SupportedStreamConfig, cpal::Error>,
) -> Vec<(cpal::StreamConfig, SampleFormat)> {
    let mut tries: Vec<(cpal::StreamConfig, SampleFormat)> = picked.into_iter().collect();
    // A config carrying an explicit buffer size gets a second go with the
    // driver's own, tried before falling back to the device default. The two
    // fallbacks are not interchangeable: this one keeps the rate and channel
    // count that were chosen and gives up only the buffering, whereas the
    // device default gives up the rate as well — and for an I/Q card the rate
    // *is* the setting the operator asked for.
    if let Some((cfg, fmt)) = tries.first().cloned()
        && cfg.buffer_size != cpal::BufferSize::Default
    {
        tries.push((cpal::StreamConfig { buffer_size: cpal::BufferSize::Default, ..cfg }, fmt));
    }
    if let Ok(def) = default {
        let pair = (def.config(), def.sample_format());
        if supported_format(pair.1) && !tries.contains(&pair) {
            tries.push(pair);
        }
    }
    tries
}

/// How much capture the sound stack is asked to buffer ahead of us, in
/// milliseconds, when we size it ourselves.
///
/// cpal's `BufferSize::Default` hands the period choice to the driver and then
/// pins the ring to two of them, and a driver's period is a *frame* count — so
/// the same setting is worth four times less time at 192 kHz than at 48 kHz,
/// and the margin the capture thread has to be scheduled inside shrinks exactly
/// as the rate the operator picked goes up. Anything that has to buffer
/// steadily wants a fixed number of milliseconds instead, whatever the rate:
/// samples the device drops because nobody emptied it are gone, and a gap in a
/// quadrature stream is a splice in the demodulated audio.
///
/// Receive only, and deliberately generous: nothing downstream of an I/Q card
/// is latency-critical (the panadapter and the demodulators both read a stream
/// that is already tens of milliseconds old), whereas a microphone is, which is
/// why [`start_input`] leaves the driver's own choice alone.
const CAPTURE_BUFFER_MS: u32 = 50;

/// The period to ask for so the capture ring comes to [`CAPTURE_BUFFER_MS`] at
/// `rate`. cpal turns `BufferSize::Fixed(n)` into a two-period ring of `2n`
/// frames, so the period is half the buffer wanted.
fn capture_period_frames(rate: u32) -> u32 {
    (rate * CAPTURE_BUFFER_MS / 2_000).max(64)
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

/// How often a stream that keeps reporting glitches may say so.
///
/// The first one is logged as it happens, because the first one is news. After
/// that they are counted and summarised: a virtual audio cable can raise one
/// every few seconds for as long as it is running, and a line per glitch turns
/// the diagnostics window into a wall of identical warnings with the rest of
/// the session's log scrolled off the top of it (issues #338, #343).
const GLITCH_REPORT_EVERY: Duration = Duration::from_secs(60);

/// The running count of capture glitches on one stream, and when it last said
/// anything about them.
///
/// A glitch here is the host telling us the captured audio is not continuous —
/// on WASAPI, the data-discontinuity flag; on ALSA, an overrun. Samples between
/// two callbacks were lost and nothing downstream can tell, because what
/// arrives is spliced end to end: the panadapter looks perfectly healthy and
/// the audio has a hole in it. That matters most to the modes that align audio
/// to a clock — an FT8 cycle with a gap in the middle of it decodes nothing —
/// which is why it is counted rather than merely survived.
struct Glitches {
    n: Arc<AtomicU64>,
    said: Mutex<(u64, Instant)>,
}

impl Glitches {
    fn new(n: Arc<AtomicU64>) -> Glitches {
        Glitches { n, said: Mutex::new((0, Instant::now())) }
    }

    /// Record one and log it, at most once per [`GLITCH_REPORT_EVERY`] after
    /// the first.
    fn on_glitch(&self, what: &str, device: &str) {
        let total = self.n.fetch_add(1, Ordering::Relaxed) + 1;
        let Ok(mut said) = self.said.lock() else { return };
        let (last_total, at) = *said;
        if total > 1 && at.elapsed() < GLITCH_REPORT_EVERY {
            return;
        }
        *said = (total, Instant::now());
        if total == 1 {
            warn!(
                "{what}: the audio stream from \"{device}\" glitched — the host says samples \
                 were lost between two callbacks, so what reaches the decoders has a hole in \
                 it spliced out of it. A virtual audio cable (VB-Audio, VAC, Flex DAX) does \
                 this routinely when the program feeding it is not keeping exact pace; a real \
                 sound card doing it means this machine is not keeping up. Further glitches on \
                 this stream are counted and summarised rather than logged one by one."
            );
        } else {
            let since = total - last_total;
            warn!(
                "{what}: {since} more audio glitch(es) from \"{device}\" in the last {:.0} s \
                 ({total} since the stream opened)",
                at.elapsed().as_secs_f64()
            );
        }
    }
}

/// Build a running input stream that converts any supported sample format to
/// f32 and pushes to `producer` (mono channel 0, or interleaved L/R if `stereo`).
///
/// `what` and `device` name the stream in anything it has to report, because
/// a station runs several at once — a microphone, a receiver's audio, a
/// panadapter's — and "input stream error" said nothing about which.
#[allow(clippy::too_many_arguments)]
fn spawn_input(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    fmt: SampleFormat,
    stereo: bool,
    mut producer: rtrb::Producer<f32>,
    dropped: Arc<AtomicU64>,
    glitches: Arc<AtomicU64>,
    what: &'static str,
    label: String,
) -> Result<cpal::Stream, AudioError> {
    let channels = config.channels as usize;
    let log = Glitches::new(glitches);
    macro_rules! build {
        ($t:ty) => {
            device.build_input_stream(
                config.clone(),
                move |data: &[$t], _| {
                    let mut lost = 0u64;
                    for frame in data.chunks(channels) {
                        // Room for the whole frame or none of it. Half a frame
                        // pushed into a full ring would leave an odd sample
                        // behind, and from there every (I, Q) pair the reader
                        // takes is one sample out — I and Q swapped for the rest
                        // of the session, which is a mirrored waterfall and SSB
                        // on the wrong sideband rather than the passing gap the
                        // overflow actually was.
                        let want = if stereo { 2 } else { 1 };
                        if producer.slots() < want {
                            lost += 1;
                            continue;
                        }
                        let l: f32 = frame[0].to_sample::<f32>();
                        let _ = producer.push(l);
                        if stereo {
                            let r = frame.get(1).copied().unwrap_or(frame[0]);
                            let _ = producer.push(r.to_sample::<f32>());
                        }
                    }
                    if lost > 0 {
                        dropped.fetch_add(lost, Ordering::Relaxed);
                    }
                },
                move |e| {
                    // A glitch is not a broken stream: the host is telling us
                    // it lost samples and carried on, and so do we. Everything
                    // else is a fault worth one line each.
                    if e.kind() == cpal::ErrorKind::Xrun {
                        log.on_glitch(what, &label);
                    } else {
                        warn!("{what}: audio input from \"{label}\" failed: {e}");
                    }
                },
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
    what: &'static str,
    label: String,
) -> Result<cpal::Stream, AudioError> {
    let channels = config.channels as usize;
    // The same rate limit the capture side runs on, and for the same reason: a
    // virtual cable on the playback end glitches just as freely as one on the
    // capture end. Its count is not published — `underruns` above is the
    // figure that matters for playback, and it is ours rather than the host's.
    let log = Glitches::new(Arc::new(AtomicU64::new(0)));
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
                move |e| {
                    if e.kind() == cpal::ErrorKind::Xrun {
                        log.on_glitch(what, &label);
                    } else {
                        warn!("{what}: audio output to \"{label}\" failed: {e}");
                    }
                },
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
    stream: cpal::Stream,
    /// The rate the stream actually runs at — resample to this.
    pub sample_rate: f64,
    /// Channels the *card* opened with. Not the shape of what to write: the
    /// ring is interleaved stereo either way — see [`start_output`].
    pub channels: u16,
    underruns: Arc<AtomicU64>,
}

impl AudioOutput {
    /// Total output callbacks that ran short of samples.
    pub fn underruns(&self) -> u64 {
        self.underruns.load(Ordering::Relaxed)
    }

    /// Stop the device callback without tearing the stream down.
    ///
    /// Not a mute: the producer side keeps filling its ring, so un-pausing
    /// replays whatever accumulated. Callers that want silence use the
    /// engine-side mute, which keeps the stream running and zeroes what goes
    /// into it. Best-effort — some raw ALSA configurations cannot pause.
    pub fn pause(&self) {
        if let Err(e) = self.stream.pause() {
            tracing::warn!("audio output pause not supported here: {e}");
        }
    }

    pub fn resume(&self) {
        if let Err(e) = self.stream.play() {
            tracing::warn!("audio output resume failed: {e}");
        }
    }
}

pub struct AudioInput {
    _stream: cpal::Stream,
    pub sample_rate: f64,
    /// Channels the capture stream actually runs with (1 = mono; IQ needs ≥2).
    pub channels: u16,
    dropped: Arc<AtomicU64>,
    glitches: Arc<AtomicU64>,
}

impl AudioInput {
    /// Frames the capture callback had to throw away because the reader had not
    /// emptied the ring — a whole second of samples behind, at any rate.
    ///
    /// Counted rather than merely survived because there is nothing else to see
    /// it by: dropped frames leave the panadapter looking perfectly healthy (an
    /// FFT of a spliced stream is still an FFT of real signals) while the
    /// demodulated audio, which has to be continuous, breaks up. A rising count
    /// here is the difference between "the radio is set up wrong" and "this
    /// machine cannot keep up with the rate it was asked for".
    pub fn dropped_frames(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Times the *host* said the captured audio was not continuous — see
    /// [`Glitches`].
    ///
    /// The other half of [`Self::dropped_frames`], and a different fault
    /// wearing the same face. Frames dropped here mean this program could not
    /// keep up; glitches mean the audio never arrived, and no amount of
    /// headroom on this side would have caught them. An operator whose FT8 will
    /// not decode needs to know which of the two they have, because the answers
    /// are "close something" and "fix the audio device".
    pub fn glitches(&self) -> u64 {
        self.glitches.load(Ordering::Relaxed)
    }
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
                    let pretty =
                        prettify_longname(lines.get(i + 1).map(|s| s.trim()).unwrap_or(""));
                    // Vendor = longname with the model (shortname) trimmed off.
                    let vendor =
                        pretty.strip_suffix(shortname).unwrap_or(&pretty).trim().to_string();
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

/// What makes two enumerated entries *the same piece of hardware*.
///
/// The two directions this has to get right pull opposite ways. On ALSA one
/// card is reached through several PCMs — `front:CARD=X`, `sysdefault:CARD=X`,
/// `hw:CARD=X,DEV=0` — and those are one device with several doors, so the card
/// is the identity and the opener may use whichever door will run. Everywhere
/// else each entry is its own endpoint, and the *name* is no identity at all:
/// two of the same USB codec — which is exactly what a station running an
/// IC-7300 and an IC-R8600 has — come back as one string twice, and only the
/// host's own device id tells them apart (a WASAPI endpoint id, a CoreAudio
/// device UID; both stay put across restarts).
fn device_identity(device: &cpal::Device, cards: &HashMap<String, AlsaCard>) -> Option<String> {
    if let Some(raw) = device_card_id(device) {
        // ALSA names the same card both ways — the hints say `CARD=Generic`,
        // the traditional `hw:`/`plughw:` entries say `CARD=0` — so the token is
        // resolved to the card's stable id before it stands for anything.
        let id = cards.get(&raw).map(|c| c.id.clone()).unwrap_or(raw);
        return Some(format!("card:{id}"));
    }
    device.id().ok().map(|id| format!("dev:{}", id.id()))
}

/// 16 bits of FNV-1a, rendered as four hex digits. Only ever used to tell two
/// identically-named devices apart, so what matters is that the same device
/// gets the same tag every run — not that the tag means anything.
fn short_tag(s: &str) -> String {
    let mut h: u32 = 0x811c_9dc5;
    for b in s.as_bytes() {
        h ^= *b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    format!("{:04x}", (h ^ (h >> 16)) & 0xffff)
}

/// Hands out the name the operator picks a device by, one device at a time.
///
/// Entries that are the same hardware *under the same name* share that name —
/// ALSA's several PCMs per card. Entries that are not never do: the second
/// device to want a name already taken is given a suffix derived from its
/// identity, so it stays the same name across restarts and the operator's
/// choice keeps pointing at the radio they chose.
///
/// Same-card-different-name is deliberately two entries, not one: the several
/// outputs of one HDMI card are one piece of hardware and still different
/// sockets, and an operator has to be able to pick between them.
#[derive(Default)]
struct NameAssigner {
    by_device: HashMap<(String, String), String>,
    taken: HashSet<String>,
}

impl NameAssigner {
    fn name_for(&mut self, base: &str, identity: &str) -> String {
        let key = (identity.to_string(), base.to_string());
        if let Some(name) = self.by_device.get(&key) {
            return name.clone();
        }
        let name = if self.taken.insert(base.to_string()) {
            base.to_string()
        } else {
            let tag = short_tag(identity);
            let mut name = format!("{base} [#{tag}]");
            let mut n = 2;
            while !self.taken.insert(name.clone()) {
                name = format!("{base} [#{tag}-{n}]");
                n += 1;
            }
            name
        };
        self.by_device.insert(key, name.clone());
        name
    }
}

/// Every device for one direction, paired with the name it is picked by. One
/// card's several PCMs appear once per PCM under a shared name — the opener
/// wants them all, so it can fall back to whichever one will run — while two
/// separate devices always get two names (see [`NameAssigner`]).
fn enumerate_devices(host: &cpal::Host, output: bool) -> Vec<(cpal::Device, String)> {
    let cards = alsa_cards();
    let devs = if output { host.output_devices().ok() } else { host.input_devices().ok() };
    let mut names = NameAssigner::default();
    let mut out = Vec::new();
    if let Some(devs) = devs {
        for d in devs {
            let Some(base) = device_display(&d, &cards) else { continue };
            // A device that won't say who it is falls back to its name, which
            // is what this used to key on throughout.
            let identity = device_identity(&d, &cards).unwrap_or_else(|| base.clone());
            let name = names.name_for(&base, &identity);
            out.push((d, name));
        }
    }
    out
}

/// The distinct names of the devices for one direction, in enumeration order.
fn device_names(host: &cpal::Host, output: bool) -> Vec<String> {
    let mut seen = HashSet::new();
    enumerate_devices(host, output)
        .into_iter()
        .map(|(_, n)| n)
        .filter(|n| seen.insert(n.clone()))
        .collect()
}

/// Names of the available output devices (for a device-selection UI).
pub fn output_device_names() -> Vec<String> {
    device_names(&cpal::default_host(), true)
}

/// Names of the available input devices (for a device-selection UI).
pub fn input_device_names() -> Vec<String> {
    device_names(&cpal::default_host(), false)
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

/// Find a device by the name it was picked by; falls back to the default device
/// (with a warning) when the name is gone — e.g. the device was unplugged.
/// Matches the enumerated name first, then the plain cpal name for configs saved
/// before names carried the manufacturer/card id (or the suffix that separates
/// two identical devices). Among matches — one card's several PCMs — prefers one
/// that actually reports a usable config.
///
/// Returns the device *and* the name it ended up under, which is what the "audio
/// output running" line reports: with two of the same sound card in the station
/// the plain cpal name says nothing about which one this is, and that line is
/// where an operator checks that each radio got its own.
fn pick_device(
    host: &cpal::Host,
    name: Option<&str>,
    output: bool,
) -> Result<(cpal::Device, String), AudioError> {
    if let Some(want) = name {
        let all = enumerate_devices(host, output);
        // 1) enumerated-name match (all sub-PCMs of the card); 2) legacy plain
        // cpal-name match for configs saved before names carried the vendor/id.
        // The legacy form cannot tell two identical devices apart — nothing
        // recorded then could — so it takes the first, exactly as before, and
        // the operator re-picks from a list that now shows both.
        let mut idxs: Vec<usize> =
            all.iter().enumerate().filter(|(_, (_, n))| n == want).map(|(i, _)| i).collect();
        if idxs.is_empty() {
            idxs = all
                .iter()
                .enumerate()
                .filter(|(_, (d, _))| {
                    d.description().ok().map(|x| x.name() == want).unwrap_or(false)
                })
                .map(|(i, _)| i)
                .collect();
        }
        if !idxs.is_empty() {
            let best = idxs
                .iter()
                .copied()
                .find(|&i| has_usable_config(&all[i].0, output))
                .unwrap_or(idxs[0]);
            return Ok(all.into_iter().nth(best).unwrap());
        }
        warn!("audio device {want:?} not found; using default");
    }
    let device = if output { host.default_output_device() } else { host.default_input_device() }
        .ok_or(AudioError::NoDevice)?;
    let cards = alsa_cards();
    let label = device_display(&device, &cards).unwrap_or_else(|| "system default".into());
    Ok((device, label))
}

/// Open an input device (microphone) by name (`None` = system default) and
/// stream mono f32 samples into the returned consumer's ring (channel 0).
/// Accepts any native sample format (i16/i32/u16/u8/f32), converting to f32.
pub fn start_input(
    device_name: Option<&str>,
    preferred_rate: u32,
) -> Result<(AudioInput, rtrb::Consumer<f32>), AudioError> {
    let host = cpal::default_host();
    let (device, label) = pick_device(&host, device_name, false)?;

    let picked = device
        .supported_input_configs()
        .ok()
        .and_then(|configs| choose_config(configs, preferred_rate, 1));
    let mut last = AudioError::NoConfig;
    for (config, fmt) in config_candidates(picked, device.default_input_config()) {
        let rate = config.sample_rate;
        let channels = config.channels;
        let (producer, consumer) = rtrb::RingBuffer::<f32>::new(rate as usize);
        let dropped = Arc::new(AtomicU64::new(0));
        let glitches = Arc::new(AtomicU64::new(0));
        let started = Instant::now();
        match spawn_input(
            &device,
            &config,
            fmt,
            false,
            producer,
            dropped.clone(),
            glitches.clone(),
            "mic input",
            label.clone(),
        ) {
            Ok(stream) => {
                info!(rate, format = ?fmt, device = %label, "mic input running");
                return Ok((
                    AudioInput {
                        _stream: stream,
                        sample_rate: rate as f64,
                        channels,
                        dropped,
                        glitches,
                    },
                    consumer,
                ));
            }
            Err(e) => {
                warn!("mic input {channels}ch {rate} Hz {fmt:?} refused: {e}");
                last = e;
                if slow_refusal(&label, started.elapsed()) {
                    break;
                }
            }
        }
    }
    Err(last)
}

/// A microphone open running on a thread of its own, handed back so the caller
/// can get on with bringing the radio up.
///
/// Opening a capture device is normally instant, but it is a blocking call into
/// the platform's sound stack and that stack can sit on it for a long time —
/// see [`SLOW_REFUSAL`]. Startup must not be that call's hostage: the radio
/// comes up, and the microphone joins it whenever it arrives.
///
/// Dropping this abandons the open. The thread runs to completion regardless —
/// nothing can cancel a call already inside the sound stack — but its result
/// goes nowhere, so the stream is closed again the moment it exists and the
/// device is released.
pub struct PendingInput {
    rx: std::sync::mpsc::Receiver<Result<(AudioInput, rtrb::Consumer<f32>), AudioError>>,
    done: bool,
}

impl PendingInput {
    /// The finished open, or `None` while it is still running. Yields its
    /// result once; every later call is `None`.
    pub fn take(&mut self) -> Option<Result<(AudioInput, rtrb::Consumer<f32>), AudioError>> {
        if self.done {
            return None;
        }
        match self.rx.try_recv() {
            Ok(result) => {
                self.done = true;
                Some(result)
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            // The opener died without answering; nothing more is coming.
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.done = true;
                Some(Err(AudioError::Build("the microphone open did not finish".into())))
            }
        }
    }
}

/// [`start_input`] on a background thread — poll the returned handle for the
/// stream instead of waiting for it.
pub fn start_input_background(device_name: Option<String>, preferred_rate: u32) -> PendingInput {
    let (tx, rx) = std::sync::mpsc::channel();
    let fallback = tx.clone();
    let spawned = std::thread::Builder::new()
        .name("mic-open".into())
        .spawn(move || {
            let _ = tx.send(start_input(device_name.as_deref(), preferred_rate));
        })
        .is_ok();
    if !spawned {
        let _ = fallback.send(Err(AudioError::Build("could not spawn the mic opener".into())));
    }
    PendingInput { rx, done: false }
}

/// Whether an open took long enough that the sound stack, not the configuration,
/// is what refused it — in which case the remaining candidates are not worth the
/// same wait. Logs the reason it is giving up.
fn slow_refusal(label: &str, took: Duration) -> bool {
    if took < SLOW_REFUSAL {
        return false;
    }
    warn!(
        device = %label,
        "the sound stack sat on that open for {took:.0?}; not spending it again on the \
         remaining configurations"
    );
    true
}

/// Like [`start_input`] but keeps the first TWO channels interleaved (L, R) —
/// used to read complex I/Q from a radio's stereo sound card. A mono device
/// degrades to duplicated samples. Accepts any native sample format.
pub fn start_input_stereo(
    device_name: Option<&str>,
    preferred_rate: u32,
) -> Result<(AudioInput, rtrb::Consumer<f32>), AudioError> {
    let host = cpal::default_host();
    let (device, label) = pick_device(&host, device_name, false)?;

    let picked = device
        .supported_input_configs()
        .ok()
        .and_then(|configs| choose_config(configs, preferred_rate, 2))
        // Sized here rather than in `choose_config` because only this path wants
        // it: see [`CAPTURE_BUFFER_MS`]. `config_candidates` retries without it
        // for a device that will not take the period asked for.
        .map(|(cfg, fmt)| {
            let period = capture_period_frames(cfg.sample_rate);
            (cpal::StreamConfig { buffer_size: cpal::BufferSize::Fixed(period), ..cfg }, fmt)
        });
    // Report the TRUE hardware channel count, not cpal's — the ALSA plug layer
    // upmixes a mono mic to a fake 2-channel config, which would otherwise slip
    // past the caller's mono-for-IQ guard. Fall back to cpal's count when the
    // hardware count is unknown (non-Linux, or a virtual device).
    let hw_channels = alsa_cards()
        .get(&device_card_id(&device).unwrap_or_default())
        .and_then(|c| hw_capture_channels(&c.index));
    let mut last = AudioError::NoConfig;
    for (config, fmt) in config_candidates(picked, device.default_input_config()) {
        let rate = config.sample_rate;
        let channels = hw_channels.unwrap_or(config.channels);
        let (producer, consumer) = rtrb::RingBuffer::<f32>::new(rate as usize * 2);
        let dropped = Arc::new(AtomicU64::new(0));
        let glitches = Arc::new(AtomicU64::new(0));
        let started = Instant::now();
        match spawn_input(
            &device,
            &config,
            fmt,
            true,
            producer,
            dropped.clone(),
            glitches.clone(),
            "radio IQ input",
            label.clone(),
        ) {
            Ok(stream) => {
                info!(rate, buffer = ?config.buffer_size, stream_channels = config.channels, hw_channels = channels, format = ?fmt, device = %label, "radio IQ input running");
                // The card is the one that decides. A panadapter half the width
                // that was asked for looks exactly like one the operator
                // mis-set, so which of the two it is goes in the log.
                if rate != preferred_rate {
                    warn!(
                        "radio IQ input “{label}” will not run at {preferred_rate} Hz — opened at \
                         {rate} Hz instead, so the panadapter is {:.0} kHz wide rather than {:.0}. \
                         Pick a rate this card offers under Settings → Radio → I/Q sample rate.",
                        rate as f64 / 1000.0,
                        preferred_rate as f64 / 1000.0,
                    );
                }
                return Ok((
                    AudioInput {
                        _stream: stream,
                        sample_rate: rate as f64,
                        channels,
                        dropped,
                        glitches,
                    },
                    consumer,
                ));
            }
            Err(e) => {
                warn!("radio IQ input {}ch {rate} Hz {fmt:?} refused: {e}", config.channels);
                last = e;
                if slow_refusal(&label, started.elapsed()) {
                    break;
                }
            }
        }
    }
    Err(last)
}

/// Open an output device by name (`None` = system default), preferring
/// `preferred_rate` (48 kHz), f32, ≤2 channels. Returns the running stream and
/// the producer to feed with **interleaved stereo** (L, R) frames. Ring
/// capacity is one second. Accepts any native sample format, converting from
/// f32.
///
/// The stereo pair is the ring's format and not the device's:
/// [`AudioOutput::channels`] says what the card opened with, and the callback
/// takes an (L, R) out of the ring for every frame it fills whatever that is,
/// mixing the two down itself on a mono card. A caller that writes one sample
/// per frame to a mono device therefore does not get a quieter stream, it gets
/// one played at double speed with each pair averaged together — see
/// `AudioCatSource::tx_write_audio` for the over that went out that way
/// (issue #247).
pub fn start_output(
    device_name: Option<&str>,
    preferred_rate: u32,
) -> Result<(AudioOutput, rtrb::Producer<f32>), AudioError> {
    let host = cpal::default_host();
    let (device, label) = pick_device(&host, device_name, true)?;

    let picked = device
        .supported_output_configs()
        .ok()
        .and_then(|configs| choose_config(configs, preferred_rate, 2));
    let mut last = AudioError::NoConfig;
    for (config, fmt) in config_candidates(picked, device.default_output_config()) {
        let rate = config.sample_rate;
        let channels = config.channels;
        let (producer, consumer) = rtrb::RingBuffer::<f32>::new(rate as usize * 2);
        let underruns = Arc::new(AtomicU64::new(0));
        let started = Instant::now();
        match spawn_output(
            &device,
            &config,
            fmt,
            consumer,
            underruns.clone(),
            "audio output",
            label.clone(),
        ) {
            Ok(stream) => {
                info!(rate, channels, format = ?fmt, device = %label, "audio output running");
                return Ok((
                    AudioOutput { stream, sample_rate: rate as f64, channels, underruns },
                    producer,
                ));
            }
            Err(e) => {
                warn!("audio output {channels}ch {rate} Hz {fmt:?} refused: {e}");
                last = e;
                if slow_refusal(&label, started.elapsed()) {
                    break;
                }
            }
        }
    }
    Err(last)
}

#[cfg(test)]
mod tests {
    use super::{
        CAPTURE_BUFFER_MS, NameAssigner, PendingInput, capture_period_frames, config_candidates,
    };
    use std::time::{Duration, Instant};

    /// The reason the period is computed rather than left to the driver: what
    /// has to stay constant as the operator moves an I/Q card up the rates is
    /// the buffer's depth in *time*. A period fixed in frames — which is what
    /// leaving it alone gets — is worth a quarter as long at 192 kHz as at 48,
    /// and the margin the capture thread has to be scheduled inside goes with
    /// it.
    #[test]
    fn the_capture_buffer_is_the_same_length_at_every_rate() {
        for rate in [48_000u32, 96_000, 192_000, 384_000] {
            // cpal turns `Fixed(n)` into a ring of two periods.
            let buffered_ms = 2.0 * capture_period_frames(rate) as f64 * 1000.0 / rate as f64;
            assert!(
                (buffered_ms - CAPTURE_BUFFER_MS as f64).abs() < 1.0,
                "{rate} Hz buffers {buffered_ms:.1} ms, wanted {CAPTURE_BUFFER_MS}"
            );
        }
    }

    /// A card that will not take the period asked for must lose the *period*,
    /// not the rate: the rate is the setting the operator chose, and silently
    /// dropping to the device default would narrow the panadapter to fix a
    /// problem they never had.
    #[test]
    fn refusing_the_buffer_size_does_not_give_up_the_rate() {
        let picked = cpal::StreamConfig {
            channels: 2,
            sample_rate: 192_000,
            buffer_size: cpal::BufferSize::Fixed(capture_period_frames(192_000)),
        };
        // A device with no default config to fall back on, so the only
        // candidates are the ones this function derives.
        let no_default = Err(cpal::Error::new(cpal::ErrorKind::DeviceNotAvailable));
        let tries = config_candidates(Some((picked, cpal::SampleFormat::I16)), no_default);
        assert_eq!(tries.len(), 2, "the chosen config, then the same without the buffer size");
        assert_eq!(tries[1].0.sample_rate, 192_000, "still the rate that was asked for");
        assert_eq!(
            tries[1].0.buffer_size,
            cpal::BufferSize::Default,
            "only the buffering given up"
        );
    }

    /// The whole point of the background open: whatever the sound stack does
    /// with the request, asking is instant. A default input that PipeWire
    /// cannot deliver — a monitor of a card sdroxide already holds — sat in the
    /// ALSA PulseAudio plugin for 30 seconds per candidate configuration, and
    /// that used to be startup's wait, not a worker thread's.
    #[test]
    fn asking_for_the_microphone_does_not_block_the_caller() {
        let started = Instant::now();
        let pending = super::start_input_background(None, 48_000);
        let asked = started.elapsed();
        drop(pending); // abandons the open; the worker closes whatever it got
        assert!(asked < Duration::from_secs(1), "starting the open took {asked:?}");
    }

    /// An opener that dies without answering has to say so, or the caller polls
    /// a handle that will never land for as long as the radio is up.
    #[test]
    fn an_opener_that_never_answers_is_reported_not_awaited() {
        let (tx, rx) = std::sync::mpsc::channel();
        drop(tx);
        let mut pending = PendingInput { rx, done: false };
        assert!(matches!(pending.take(), Some(Err(_))));
        assert!(pending.take().is_none(), "the answer is handed over once");
    }

    /// A refusal that took real time is the sound stack's, not the
    /// configuration's — the remaining candidates would each spend the same
    /// wait, which is how one wedged device turned into a minute of startup.
    #[test]
    fn only_a_slow_refusal_abandons_the_other_configurations() {
        assert!(!super::slow_refusal("a device", Duration::from_millis(3)));
        assert!(super::slow_refusal("a wedged device", Duration::from_secs(30)));
    }

    #[test]
    fn device_enumeration_works() {
        // Must not panic, even on systems without audio; prints what it found.
        let outs = super::output_device_names();
        let ins = super::input_device_names();
        eprintln!("outputs: {outs:?}");
        eprintln!("inputs:  {ins:?}");
    }

    /// ALSA reaches one card through several PCMs. They are one device and have
    /// to stay one entry, or the operator picks between doors instead of radios
    /// and the opener loses the fallback to whichever PCM will actually run.
    #[test]
    fn one_card_reached_several_ways_is_one_device() {
        let mut n = NameAssigner::default();
        assert_eq!(n.name_for("USB Audio CODEC [CODEC]", "card:CODEC"), "USB Audio CODEC [CODEC]");
        assert_eq!(n.name_for("USB Audio CODEC [CODEC]", "card:CODEC"), "USB Audio CODEC [CODEC]");
    }

    /// Two of the same USB codec — an IC-7300 and an IC-R8600 on one machine —
    /// come back from Windows and macOS as one name twice. Collapsing them left
    /// a single entry in the picker and both radios opening the first card, so
    /// whichever radio came up first took the audio and the other was silent.
    #[test]
    fn two_of_the_same_sound_card_are_two_devices() {
        let mut n = NameAssigner::default();
        let first = n.name_for("USB Audio CODEC", "dev:{0.0.1.00000000}.{aaaa}");
        let second = n.name_for("USB Audio CODEC", "dev:{0.0.1.00000000}.{bbbb}");
        assert_eq!(first, "USB Audio CODEC");
        assert_ne!(first, second, "two devices must not share one name");
        assert!(second.starts_with("USB Audio CODEC ["), "the suffix hangs off the real name");
        // And each keeps the name it was given, however often it is asked.
        assert_eq!(n.name_for("USB Audio CODEC", "dev:{0.0.1.00000000}.{bbbb}"), second);
    }

    /// One card, several sockets: an HDMI card's outputs are one piece of
    /// hardware under several names, and the operator picks between the names.
    #[test]
    fn one_card_with_several_outputs_keeps_them_apart() {
        let mut n = NameAssigner::default();
        assert_eq!(n.name_for("HDMI 0 [Generic]", "card:Generic"), "HDMI 0 [Generic]");
        assert_eq!(n.name_for("HDMI 2 [Generic]", "card:Generic"), "HDMI 2 [Generic]");
    }

    /// The suffix is derived from the device's own identity, not from the order
    /// the host happened to enumerate in: a name that moved between radios
    /// across a restart would point each one at the other's transmitter.
    #[test]
    fn the_suffix_does_not_depend_on_enumeration_order() {
        let name = |a: &str, b: &str| {
            let mut n = NameAssigner::default();
            n.name_for("USB Audio CODEC", a);
            n.name_for("USB Audio CODEC", b)
        };
        // B named second in both runs, so B keeps its suffix whichever order
        // the two were seen in.
        assert_eq!(name("dev:aaaa", "dev:bbbb"), name("dev:cccc", "dev:bbbb"));
    }
}
