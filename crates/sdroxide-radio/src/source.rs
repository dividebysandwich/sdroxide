use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::{Duration, Instant};

use crate::{Complex32, Result};

/// Anything that produces a stream of complex baseband samples: a live
/// SoapySDR RX stream, a recorded IQ file, or a signal generator.
///
/// This is the seam that lets the whole DSP stack run in CI and without
/// hardware attached.
pub trait IqSource: Send {
    fn sample_rate(&self) -> f64;
    fn center_hz(&self) -> f64;
    fn set_center_hz(&mut self, hz: f64) -> Result<()>;

    /// Blocking read. Returns the number of samples written to `buf`;
    /// 0 means a timeout (caller should just retry).
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize>;

    /// Human-readable description for logs/UI.
    fn describe(&self) -> String;

    // Hardware controls — meaningful only for real devices; default no-ops.
    fn set_gain_element(&mut self, _name: &str, _db: f64) -> Result<()> {
        Ok(())
    }
    fn set_antenna(&mut self, _name: &str) -> Result<()> {
        Ok(())
    }
    fn current_gains(&self) -> Vec<(String, f64)> {
        Vec::new()
    }
    fn current_antenna(&self) -> String {
        String::new()
    }

    // Transmit path — implemented by transmit-capable devices only.
    // Half-duplex sequencing (pausing RX) is the implementation's job.

    /// Start transmitting: tune the TX LO, apply TX gains, activate the TX
    /// stream. Returns the actual TX sample rate.
    fn tx_begin(&mut self, _center_hz: f64, _rate: f64) -> Result<f64> {
        Err(crate::RadioError::Msg("device is not transmit capable".into()))
    }
    /// Blocking write of complex baseband at the TX rate.
    fn tx_write(&mut self, _samples: &[Complex32]) -> Result<()> {
        Err(crate::RadioError::Msg("device is not transmit capable".into()))
    }
    /// Stop transmitting and restore RX.
    fn tx_end(&mut self) -> Result<()> {
        Ok(())
    }
    fn set_tx_gain_element(&mut self, _name: &str, _db: f64) -> Result<()> {
        Ok(())
    }
    fn current_tx_gains(&self) -> Vec<(String, f64)> {
        Vec::new()
    }
}

/// Paces reads so a non-hardware source delivers samples in real time.
struct Throttle {
    start: Instant,
    emitted: u64,
    rate: f64,
}

impl Throttle {
    fn new(rate: f64) -> Self {
        Throttle { start: Instant::now(), emitted: 0, rate }
    }

    fn pace(&mut self, n: usize) {
        self.emitted += n as u64;
        let due = self.start + Duration::from_secs_f64(self.emitted as f64 / self.rate);
        let now = Instant::now();
        if due > now {
            std::thread::sleep(due - now);
        }
    }
}

/// Multi-tone signal generator with a noise floor. Real-time paced.
pub struct SigGenSource {
    sample_rate: f64,
    center_hz: f64,
    /// (offset from center in Hz, linear amplitude)
    tones: Vec<(f64, f32)>,
    phases: Vec<f64>,
    noise_amp: f32,
    rng: u64,
    throttle: Throttle,
}

impl SigGenSource {
    pub fn new(sample_rate: f64, center_hz: f64, tones: Vec<(f64, f32)>, noise_amp: f32) -> Self {
        let phases = vec![0.0; tones.len()];
        SigGenSource {
            sample_rate,
            center_hz,
            tones,
            phases,
            noise_amp,
            rng: 0x9e3779b97f4a7c15,
            throttle: Throttle::new(sample_rate),
        }
    }

    /// A default test scene: carriers at various offsets over a noise floor.
    /// One tone sits 700 Hz above center so the default USB tune is audible
    /// immediately.
    pub fn demo(sample_rate: f64, center_hz: f64) -> Self {
        Self::new(
            sample_rate,
            center_hz,
            vec![
                (-sample_rate * 0.30, 0.02),
                (-sample_rate * 0.11, 0.10),
                (700.0, 0.05),
                (sample_rate * 0.07, 0.30),
                (sample_rate * 0.23, 0.05),
            ],
            0.001,
        )
    }

    fn white(&mut self) -> f32 {
        // xorshift64* — cheap, deterministic, good enough for a noise floor.
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        let v = (self.rng.wrapping_mul(0x2545F4914F6CDD1D) >> 40) as i32;
        v as f32 / (1 << 23) as f32 - 1.0
    }
}

impl IqSource for SigGenSource {
    fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    fn center_hz(&self) -> f64 {
        self.center_hz
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center_hz = hz;
        Ok(())
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        use std::f64::consts::TAU;
        for s in buf.iter_mut() {
            let mut acc = Complex32::new(self.noise_amp * self.white(), self.noise_amp * self.white());
            for (i, &(offset, amp)) in self.tones.iter().enumerate() {
                let ph = self.phases[i];
                acc += Complex32::new((ph.cos() * amp as f64) as f32, (ph.sin() * amp as f64) as f32);
                self.phases[i] = (ph + TAU * offset / self.sample_rate) % TAU;
            }
            *s = acc;
        }
        self.throttle.pace(buf.len());
        Ok(buf.len())
    }

    fn describe(&self) -> String {
        format!(
            "signal generator ({} tones, {:.3} Msps)",
            self.tones.len(),
            self.sample_rate / 1e6
        )
    }
}

/// Raw interleaved CF32 (little-endian f32 I,Q) file playback, looped,
/// real-time paced.
pub struct FileSource {
    reader: BufReader<File>,
    path: String,
    sample_rate: f64,
    center_hz: f64,
    throttle: Throttle,
}

impl FileSource {
    pub fn open(path: impl AsRef<Path>, sample_rate: f64, center_hz: f64) -> Result<Self> {
        let path_str = path.as_ref().display().to_string();
        let reader = BufReader::new(File::open(path)?);
        Ok(FileSource {
            reader,
            path: path_str,
            sample_rate,
            center_hz,
            throttle: Throttle::new(sample_rate),
        })
    }
}

impl IqSource for FileSource {
    fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    fn center_hz(&self) -> f64 {
        self.center_hz
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center_hz = hz;
        Ok(())
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let mut raw = vec![0u8; buf.len() * 8];
        let mut filled = 0;
        while filled < raw.len() {
            let n = self.reader.read(&mut raw[filled..])?;
            if n == 0 {
                self.reader.seek(SeekFrom::Start(0))?; // loop
                continue;
            }
            filled += n;
        }
        for (s, chunk) in buf.iter_mut().zip(raw.chunks_exact(8)) {
            let i = f32::from_le_bytes(chunk[0..4].try_into().unwrap());
            let q = f32::from_le_bytes(chunk[4..8].try_into().unwrap());
            *s = Complex32::new(i, q);
        }
        self.throttle.pace(buf.len());
        Ok(buf.len())
    }

    fn describe(&self) -> String {
        format!("IQ file {} ({:.3} Msps)", self.path, self.sample_rate / 1e6)
    }
}
