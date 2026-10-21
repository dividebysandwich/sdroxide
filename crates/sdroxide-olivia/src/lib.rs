//! Olivia, decoded by the engine that reads real signals.
//!
//! This crate exists because three implementations written here all failed on a
//! recording that fldigi decoded perfectly. Rather than keep guessing at the
//! Walsh, interleave and scrambler conventions, the working engine — Pawel
//! Jalocha's `MFSK_Receiver`, the same one behind fldigi and SDR-J — is vendored
//! under `cxx/` and wrapped here. See README.md for provenance and licence.
//!
//! What this crate does is set the parameters the reference drivers set, feed
//! audio in the batches they feed, and hand back the characters.

use std::ffi::c_void;
use std::os::raw::{c_char, c_double, c_float, c_int};

unsafe extern "C" {
    fn olivia_process(
        handle: *mut c_void,
        audio: *const c_float,
        n: c_int,
        out: *mut c_char,
        outlen: c_int,
        snr: *mut c_double,
    ) -> c_int;
    fn olivia_new_ex(
        tones: c_int,
        bandwidth: c_int,
        rate: c_double,
        centre: c_double,
        reverse: c_int,
        contestia: c_int,
    ) -> *mut c_void;
    fn olivia_free(handle: *mut c_void);
}

/// One Olivia receiver.
///
/// `centre_hz` is the audio frequency the tone bank is centred on, which is the
/// carrier the operator tunes to — 1500 Hz on fldigi's sweet spot. `tones` and
/// `bandwidth_hz` describe the mode: 16 tones over 500 Hz, 32 over 1000, and so
/// on.
pub struct OliviaDecoder {
    handle: *mut c_void,
    scratch: Vec<u8>,
    snr: f64,
}

impl OliviaDecoder {
    /// `reverse` inverts the tone order. A capture taken on the wrong sideband
    /// is spectrally inverted and reads as nonsense until this is set — which is
    /// exactly what happened to the first bench recordings: extracted as USB
    /// from a rig that was in LSB.
    pub fn new(
        rate: f64,
        centre_hz: f64,
        tones: usize,
        bandwidth_hz: u32,
        reverse: bool,
    ) -> Option<Self> {
        Self::make(rate, centre_hz, tones, bandwidth_hz, reverse, false)
    }

    /// The same engine, set to Contestia: a different scrambling code
    /// (`0xEDB88320`) and a five-bit shift instead of thirteen. Contestia's own
    /// mode settings name `n` where the engine wants `2 << n` carriers, so the
    /// caller passes the carrier count.
    pub fn contestia(
        rate: f64,
        centre_hz: f64,
        tones: usize,
        bandwidth_hz: u32,
        reverse: bool,
    ) -> Option<Self> {
        Self::make(rate, centre_hz, tones, bandwidth_hz, reverse, true)
    }

    fn make(
        rate: f64,
        centre_hz: f64,
        tones: usize,
        bandwidth_hz: u32,
        reverse: bool,
        contestia: bool,
    ) -> Option<Self> {
        let handle = unsafe {
            olivia_new_ex(
                tones as c_int,
                bandwidth_hz as c_int,
                rate,
                centre_hz,
                reverse as c_int,
                contestia as c_int,
            )
        };
        if handle.is_null() {
            return None;
        }
        Some(OliviaDecoder { handle, scratch: vec![0u8; 4096], snr: 0.0 })
    }

    /// Decode a batch of audio, returning whatever text came out of it.
    ///
    /// Control characters are dropped rather than turned into spaces: Olivia's
    /// idle character is NUL and a listener does not want a stream of them
    /// pushing the text off the end of the box.
    pub fn process(&mut self, audio: &[f32]) -> String {
        let n = unsafe {
            olivia_process(
                self.handle,
                audio.as_ptr(),
                audio.len() as c_int,
                self.scratch.as_mut_ptr() as *mut c_char,
                self.scratch.len() as c_int,
                &mut self.snr,
            )
        };
        let n = n.max(0) as usize;
        self.scratch[..n].iter().map(|&b| b as char).filter(|c| !c.is_control()).collect()
    }

    /// The engine's own signal-to-noise estimate for the last batch.
    pub fn snr(&self) -> f64 {
        self.snr
    }
}

// The engine is a self-contained C++ object: it holds no thread-local state, no
// file descriptors and no Rust references. The digi controller is *moved*
// between threads but never used from two at once, which is what `Send` asks
// for; it stays `!Sync`, because two threads driving it at once would be a data
// race in the engine and nothing here prevents that.
unsafe impl Send for OliviaDecoder {}

/// The receiver, shaped the way the digi engine expects: `process`, `magnitude`
/// and `set_params`, exactly as `sdroxide_dsp::OliviaRx` offers them.
///
/// The engine does not report a received level, so the S-meter is fed by the
/// peak of the audio as it arrives — the same decayed peak the DSP receiver
/// kept.
pub struct OliviaReceiver {
    engine: Option<OliviaDecoder>,
    rate: f64,
    centre: f64,
    tones: usize,
    bandwidth: u32,
    reverse: bool,
    /// Contestia is the same engine with a different scrambling code, so the
    /// flag has to survive a rebuild — `set_params` and `set_reverse` both
    /// reconstruct the decoder, and dropping it here would silently turn a
    /// Contestia station back into an Olivia one on the first retune.
    contestia: bool,
    mag: f32,
}

impl OliviaReceiver {
    pub fn new(rate: f64, centre: f64, tones: usize, bandwidth: u32, reverse: bool) -> Self {
        let engine = OliviaDecoder::new(rate, centre, tones, bandwidth, reverse);
        OliviaReceiver {
            engine,
            rate,
            centre,
            tones,
            bandwidth,
            reverse,
            contestia: false,
            mag: 0.0,
        }
    }

    /// The same receiver set to Contestia.
    pub fn contestia(rate: f64, centre: f64, tones: usize, bandwidth: u32, reverse: bool) -> Self {
        let engine = OliviaDecoder::contestia(rate, centre, tones, bandwidth, reverse);
        OliviaReceiver {
            engine,
            rate,
            centre,
            tones,
            bandwidth,
            reverse,
            contestia: true,
            mag: 0.0,
        }
    }

    fn build(
        &self,
        centre: f64,
        tones: usize,
        bandwidth: u32,
        reverse: bool,
    ) -> Option<OliviaDecoder> {
        if self.contestia {
            OliviaDecoder::contestia(self.rate, centre, tones, bandwidth, reverse)
        } else {
            OliviaDecoder::new(self.rate, centre, tones, bandwidth, reverse)
        }
    }

    /// Retune or change mode. The engine is rebuilt, which is what fldigi's own
    /// `restart()` does when the tone count or bandwidth changes.
    pub fn set_params(&mut self, centre: f64, tones: usize, bandwidth: u32) {
        if (centre - self.centre).abs() < 0.5 && tones == self.tones && bandwidth == self.bandwidth
        {
            return;
        }
        self.centre = centre;
        self.tones = tones;
        self.bandwidth = bandwidth;
        self.engine = self.build(centre, tones, bandwidth, self.reverse);
        self.mag = 0.0;
    }

    /// Invert the tone order. Real stations are sometimes reversed, and a
    /// capture taken on the wrong sideband always is.
    pub fn set_reverse(&mut self, reverse: bool) {
        if reverse == self.reverse {
            return;
        }
        self.reverse = reverse;
        self.engine = self.build(self.centre, self.tones, self.bandwidth, reverse);
    }

    pub fn reverse(&self) -> bool {
        self.reverse
    }

    /// Whether this receiver is running Contestia's scrambling code rather than
    /// Olivia's. Read by the tests: `set_params` and `set_reverse` rebuild the
    /// engine, and a flag that is dropped there would turn a Contestia station
    /// into an Olivia one on the first retune — silently, because both engines
    /// decode *something*.
    pub fn is_contestia(&self) -> bool {
        self.contestia
    }

    pub fn process(&mut self, audio: &[f32]) -> String {
        let peak = audio.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        self.mag += 0.02 * (peak - self.mag);
        match self.engine.as_mut() {
            Some(e) => e.process(audio),
            None => String::new(),
        }
    }

    pub fn magnitude(&self) -> f32 {
        self.mag
    }

    /// The engine's signal-to-noise estimate, for the panel's readout.
    pub fn snr(&self) -> f64 {
        self.engine.as_ref().map(|e| e.snr()).unwrap_or(0.0)
    }
}

impl Drop for OliviaDecoder {
    fn drop(&mut self) {
        unsafe { olivia_free(self.handle) }
    }
}

#[cfg(test)]
mod tests {
    use crate::{OliviaDecoder, OliviaReceiver};

    /// A real transmission, decoded by the vendored engine.
    ///
    /// `SDROXIDE_OLIVIA_SAMPLE` is a mono WAV of an Olivia signal;
    /// `_TONES`, `_BW`, `_HZ` and `_TEXT` describe it. The capture this was
    /// written against is `~/Downloads/sounds/our-captures/olivia_16-500_cb_27415.wav`
    /// — transmitted by fldigi through a rig's VOX and received on an RSP1, so
    /// the text is known exactly.
    #[test]
    #[ignore]
    fn an_off_air_capture_decodes() {
        // Default to the fixture that lives in the repository, so this runs
        // without any environment set up. `SDROXIDE_OLIVIA_SAMPLE` overrides it.
        let path = std::env::var("SDROXIDE_OLIVIA_SAMPLE").unwrap_or_else(|_| {
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/bench/olivia_16-500_cb_27415.wav")
                .to_string()
        });
        if !std::path::Path::new(&path).exists() {
            eprintln!("no sample at {path}; skipping");
            return;
        }
        let mut reader = hound::WavReader::open(&path).expect("open the sample WAV");
        let spec = reader.spec();
        let audio: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => {
                reader.samples::<f32>().map(|s| s.expect("sample")).collect()
            }
            hound::SampleFormat::Int => {
                reader.samples::<i16>().map(|s| s.expect("sample") as f32 / 32_768.0).collect()
            }
        };
        let num = |k: &str, d: f64| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
        let tones = num("SDROXIDE_OLIVIA_TONES", 16.0) as usize;
        let bw = num("SDROXIDE_OLIVIA_BW", 500.0) as u32;
        // The fixture in the repository is the bench capture: 16/500, centred at
        // 1705 Hz of audio, and taken while the rig was on LSB, so it needs the
        // tone order reversed. All three are what the manifest says.
        let builtin = std::env::var("SDROXIDE_OLIVIA_SAMPLE").is_err();
        let hz = num("SDROXIDE_OLIVIA_HZ", if builtin { 1705.0 } else { 1500.0 });

        let reverse = std::env::var("SDROXIDE_OLIVIA_REVERSE").is_ok() || builtin;
        let mut rx = OliviaDecoder::new(spec.sample_rate as f64, hz, tones, bw, reverse)
            .expect("the engine allocated");
        // The engine's pipeline runs about a second behind the audio it is
        // given, so a capture that ends on the last character loses that
        // character's block. Feed a second of silence to flush it.
        let pad = vec![0.0f32; (spec.sample_rate as usize).max(8000) * 2];
        let mut out = String::new();
        for chunk in audio.iter().chain(pad.iter()).copied().collect::<Vec<_>>().chunks(1024) {
            out.push_str(&rx.process(chunk));
        }
        eprintln!("decoded {} chars at {tones}/{bw}, centre {hz}: {out:?}", out.len());
        if let Ok(want) = std::env::var("SDROXIDE_OLIVIA_TEXT") {
            for w in want.split_whitespace() {
                assert!(out.contains(w), "did not recover {w:?}: {out:?}");
            }
        }
    }

    /// The Contestia sample from the corpus. Contestia is the same engine with a
    /// different scrambling code and a five-bit shift, so the test is really
    /// "did the flag get set and the bank placed where fldigi places it".
    ///
    /// The mode settings are the ones the sample was sent with; the manifest
    /// records `CONTESTIA` without its variant, so the grid is searched and the
    /// best is printed. An override pins it.
    #[test]
    #[ignore]
    fn the_corpus_contestia_sample_decodes() {
        let path = std::env::var("SDROXIDE_CONTESTIA_SAMPLE").unwrap_or_else(|_| {
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/fldigi/CONTESTIA.wav").to_string()
        });
        let mut reader = hound::WavReader::open(&path).expect("open the sample WAV");
        let spec = reader.spec();
        let audio: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => {
                reader.samples::<f32>().map(|s| s.expect("sample")).collect()
            }
            hound::SampleFormat::Int => {
                reader.samples::<i16>().map(|s| s.expect("sample") as f32 / 32_768.0).collect()
            }
        };
        let want = "EAGLE LANDED SDRBROWN";
        let words: Vec<&str> = want.split_whitespace().collect();
        let pad = vec![0.0f32; (spec.sample_rate as usize).max(8000) * 2];
        let mut best = (0usize, 0usize, 0u32, 0.0f64, false, String::new());
        // Contestia's own settings are independent: `tones` n means
        // `2 << n` carriers and `bw` m means `125 << m` Hz. The paired grid is
        // the one fldigi's presets use; the cross combinations follow if needed.
        for n in 0..5u32 {
            let tones = (4usize) << n;
            let bw = 125u32 << n;
            if tones > 64 || bw > 2000 {
                continue;
            }
            for reverse in [false, true] {
                for hz in [1718.0f64, 1727.0, 1736.0] {
                    let mut rx =
                        OliviaDecoder::contestia(spec.sample_rate as f64, hz, tones, bw, reverse)
                            .expect("allocated");
                    let mut out = String::new();
                    for chunk in
                        audio.iter().chain(pad.iter()).copied().collect::<Vec<_>>().chunks(1024)
                    {
                        out.push_str(&rx.process(chunk));
                    }
                    let hits = words.iter().filter(|w| out.contains(**w)).count();
                    eprintln!(
                        "{tones}/{bw} at {hz} reverse {reverse}: {hits}/{} {:?}",
                        words.len(),
                        out
                    );
                    if hits > best.0 {
                        best = (hits, tones, bw, hz, reverse, out);
                    }
                }
            }
        }
        eprintln!(
            "best: {}/{} at {}/{}, {hz} Hz reverse {}: {:?}",
            best.0,
            words.len(),
            best.1,
            best.2,
            best.4,
            best.5,
            hz = best.3
        );
        // The corpus sample is Contestia 4/125 (fldigi's `tones` setting 1
        // means `2 << 1` carriers), sent on the rig's LSB path, hence reverse.
        assert_eq!(best.1, 4, "the tone count moved");
        assert_eq!(best.2, 125, "the bandwidth moved");
        assert!(best.4, "the sample no longer needs the reversal");
        assert_eq!(best.0, words.len(), "Contestia did not read its sample: {:?}", best.5);
    }

    /// Every Olivia variant in the corpus, with its own tones and bandwidth
    /// read from the name and a small centre search. The four that do not
    /// decode are named; the rest are the engine's own proof.
    #[test]
    #[ignore]
    fn the_corpus_olivia_variants_are_measured() {
        use std::path::Path;
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/fldigi");
        let table: &[(&str, usize, u32)] = &[
            ("OLIVIA-4-125", 4, 125),
            ("OLIVIA-4-250", 4, 250),
            ("OLIVIA-4-500", 4, 500),
            ("OLIVIA-4-1K", 4, 1000),
            ("OLIVIA-4-2K", 4, 2000),
            ("OLIVIA-8-125", 8, 125),
            ("OLIVIA-8-250", 8, 250),
            ("OLIVIA-8-500", 8, 500),
            ("OLIVIA-8-1K", 8, 1000),
            ("OLIVIA-8-2K", 8, 2000),
            ("OLIVIA-16-500", 16, 500),
            ("OLIVIA-16-1K", 16, 1000),
            ("OLIVIA-16-2K", 16, 2000),
            ("OLIVIA-32-1K", 32, 1000),
            ("OLIVIA-32-2K", 32, 2000),
            ("OLIVIA-64-500", 64, 500),
            ("OLIVIA-64-1K", 64, 1000),
            ("OLIVIA-64-2K", 64, 2000),
        ];
        let mut ok = 0;
        for &(name, tones, bw) in table {
            let path = format!("{root}/{name}.wav");
            if !Path::new(&path).exists() {
                eprintln!("{name}: no sample");
                continue;
            }
            let mut reader = hound::WavReader::open(&path).expect("open");
            let spec = reader.spec();
            let audio: Vec<f32> =
                reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect();
            let pad = vec![0.0f32; 16000];
            let mut best = String::new();
            for hz in [1720.0f64, 1740.0, 1760.0, 1780.0] {
                let mut rx = OliviaDecoder::new(spec.sample_rate as f64, hz, tones, bw, true)
                    .expect("allocated");
                let mut out = String::new();
                for chunk in
                    audio.iter().chain(pad.iter()).copied().collect::<Vec<_>>().chunks(1024)
                {
                    out.push_str(&rx.process(chunk));
                }
                if out.contains("SDRBROWN") {
                    best = format!("{hz}: {out}");
                    break;
                }
            }
            if best.is_empty() {
                eprintln!("{name} ({tones}/{bw}): no decode");
            } else {
                ok += 1;
                eprintln!("{name} ({tones}/{bw}): {best}");
            }
        }
        eprintln!("{ok} of {} decode", table.len());
        assert_eq!(ok, 14, "the Olivia variant count moved");
    }

    /// Every Contestia variant in the corpus, with the parameters its name
    /// gives (`Cont-16/1K` is 16 carriers over 1000 Hz) and a small centre
    /// search. CONTESTIA.wav proved the engine; this says which variants it
    /// reads.
    #[test]
    #[ignore]
    fn the_corpus_contestia_variants_are_measured() {
        use std::path::Path;
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/fldigi");
        let table: &[(&str, usize, u32)] = &[
            ("Cont-4-125", 4, 125),
            ("Cont-4-250", 4, 250),
            ("Cont-4-500", 4, 500),
            ("Cont-4-1K", 4, 1000),
            ("Cont-4-2K", 4, 2000),
            ("Cont-8-125", 8, 125),
            ("Cont-8-250", 8, 250),
            ("Cont-8-500", 8, 500),
            ("Cont-8-1K", 8, 1000),
            ("Cont-8-2K", 8, 2000),
            ("Cont-16-250", 16, 250),
            ("Cont-16-500", 16, 500),
            ("Cont-16-1K", 16, 1000),
            ("Cont-16-2K", 16, 2000),
            ("Cont-32-1K", 32, 1000),
            ("Cont-32-2K", 32, 2000),
            ("Cont-64-500", 64, 500),
            ("Cont-64-1K", 64, 1000),
            ("Cont-64-2K", 64, 2000),
        ];
        let mut ok = 0;
        for &(name, tones, bw) in table {
            let path = format!("{root}/{name}.wav");
            if !Path::new(&path).exists() {
                eprintln!("{name}: no sample");
                continue;
            }
            let mut reader = hound::WavReader::open(&path).expect("open");
            let spec = reader.spec();
            let audio: Vec<f32> =
                reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect();
            let pad = vec![0.0f32; 16000];
            let mut best = String::new();
            for hz in [1720.0f64, 1740.0, 1760.0, 1780.0] {
                let mut rx = OliviaDecoder::contestia(spec.sample_rate as f64, hz, tones, bw, true)
                    .expect("allocated");
                let mut out = String::new();
                for chunk in
                    audio.iter().chain(pad.iter()).copied().collect::<Vec<_>>().chunks(1024)
                {
                    out.push_str(&rx.process(chunk));
                }
                if out.contains("SDRBROWN") || out.contains("EAGLE") {
                    best = format!("{hz}: {out}");
                    break;
                }
            }
            if best.is_empty() {
                eprintln!("{name} ({tones}/{bw}): no decode");
            } else {
                ok += 1;
                eprintln!("{name} ({tones}/{bw}): {best}");
            }
        }
        eprintln!("{ok} of {} decode", table.len());
        assert_eq!(ok, 18, "the Contestia variant count moved");
    }

    /// Contestia is a flag on one engine, and every retune rebuilds that engine.
    /// The flag has to survive the rebuild: both scramblers decode *something*,
    /// so dropping it makes a Contestia station read as noise with no symptom
    /// that points at the cause.
    #[test]
    fn the_contestia_flag_survives_a_retune_and_a_reversal() {
        let mut rx = OliviaReceiver::contestia(8000.0, 1500.0, 8, 500, false);
        assert!(rx.is_contestia());
        rx.set_params(1700.0, 16, 1000);
        assert!(rx.is_contestia(), "a retune dropped the scrambler");
        rx.set_reverse(true);
        assert!(rx.is_contestia(), "the polarity change dropped the scrambler");

        let mut plain = OliviaReceiver::new(8000.0, 1500.0, 16, 500, false);
        plain.set_params(1700.0, 8, 250);
        assert!(!plain.is_contestia(), "an Olivia receiver became Contestia");
    }
}
