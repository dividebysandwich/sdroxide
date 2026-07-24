//! Yaesu "new CAT" (ASCII, `;`-terminated) — SCAFFOLD, unverified against real
//! hardware. Frequency `FA<11 digits>;`, mode `MD0<x>;`, PTT `TX1;`/`TX0;`.
//! The response parser is intentionally minimal until a rig is available.

use crate::{CatUpdate, Protocol};
use sdroxide_types::Mode;

pub struct Yaesu {
    buf: String,
}

impl Yaesu {
    pub fn new() -> Self {
        Yaesu { buf: String::new() }
    }
}

fn mode_digit(m: Mode) -> char {
    // Yaesu MD map (FT-891/991 family): 1=LSB 2=USB 3=CW 4=FM 5=AM
    // 6=RTTY-L 7=CW-R 8=DATA-L 9=RTTY-U A=DATA-FM B=FM-N C=DATA-U
    match m {
        Mode::Lsb => '1',
        Mode::Cw => '3',
        Mode::Nfm | Mode::Wfm => '4',
        Mode::Am | Mode::Sam | Mode::Dsb => '5',
        Mode::Digl => '8',
        Mode::Digu
        | Mode::Ft8
        | Mode::Ft4
        | Mode::Psk
        | Mode::Rtty
        | Mode::Olivia
        | Mode::Thor
        | Mode::Fsq => 'C',
        Mode::Usb | Mode::Spec | Mode::Sstv | Mode::RfPaint => '2',
    }
}

impl Protocol for Yaesu {
    fn set_freq(&self, hz: f64) -> Vec<u8> {
        format!("FA{:09};", hz.round().max(0.0) as u64).into_bytes()
    }
    fn set_mode(&self, m: Mode) -> Vec<u8> {
        format!("MD0{};", mode_digit(m)).into_bytes()
    }
    fn ptt(&self, on: bool) -> Vec<u8> {
        if on { b"TX1;".to_vec() } else { b"TX0;".to_vec() }
    }
    fn poll_requests(&self) -> Vec<Vec<u8>> {
        vec![b"FA;".to_vec(), b"MD0;".to_vec()]
    }
    fn parse(&mut self, buf: &mut Vec<u8>) -> Vec<CatUpdate> {
        // Accumulate ASCII, split on ';'.
        self.buf.push_str(&String::from_utf8_lossy(buf));
        buf.clear();
        let mut out = Vec::new();
        while let Some(idx) = self.buf.find(';') {
            let msg: String = self.buf.drain(..=idx).collect();
            let msg = msg.trim_end_matches(';');
            if let Some(rest) = msg.strip_prefix("FA") {
                if let Ok(hz) = rest.trim().parse::<u64>() {
                    out.push(CatUpdate::Freq(hz as f64));
                }
            }
            // Mode reply parsing left for hardware verification.
        }
        out
    }
}
