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
    fn olivia_new(
        tones: c_int,
        bandwidth: c_int,
        rate: c_double,
        centre: c_double,
        reverse: c_int,
    ) -> *mut c_void;
    fn olivia_process(
        handle: *mut c_void,
        audio: *const c_float,
        n: c_int,
        out: *mut c_char,
        outlen: c_int,
        snr: *mut c_double,
    ) -> c_int;
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
        let handle = unsafe {
            olivia_new(tones as c_int, bandwidth_hz as c_int, rate, centre_hz, reverse as c_int)
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

impl Drop for OliviaDecoder {
    fn drop(&mut self) {
        unsafe { olivia_free(self.handle) }
    }
}

#[cfg(test)]
mod tests {
    use crate::OliviaDecoder;

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
        let Ok(path) = std::env::var("SDROXIDE_OLIVIA_SAMPLE") else {
            eprintln!("SDROXIDE_OLIVIA_SAMPLE unset; skipping");
            return;
        };
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
        let hz = num("SDROXIDE_OLIVIA_HZ", 1500.0);

        let reverse = std::env::var("SDROXIDE_OLIVIA_REVERSE").is_ok();
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
}
