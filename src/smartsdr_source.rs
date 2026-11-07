//! An [`IqSource`] for a SmartSDR radio (FlexRadio FLEX-6000 / FLEX-8000).
//!
//! Receive is wideband IQ over a DAX IQ stream → the engine's normal DDC/demod
//! path (`audio_mode = false`); transmit is 24 kHz audio (`tx_write_audio`)
//! which the radio modulates (`caps.tx_audio`). Control (frequency, mode, PTT,
//! power) rides the ASCII command protocol on TCP 4992.
//!
//! Shaped like [`crate::tci_source`], because the two rigs present the same way:
//! somebody else's transceiver, streaming us IQ and modulating audio we send
//! back. The differences are the ceiling — DAX IQ stops at 192 kHz — and that
//! this one has never been run against real hardware, which is why every
//! connection carries a diagnostic trace.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, ControlUpdate, IqSource, Result};
use sdroxide_smartsdr::{FlexHandle, FlexUpdate};
use sdroxide_types::TxTelemetry;

use crate::session_trace::TraceStore;

/// The last session's trace of each radio, kept after the source is dropped.
/// See [`crate::session_trace`] — including why there is one per radio rather
/// than one for the process.
static TRACES: TraceStore = TraceStore::new();

/// Radios that have thrown one of our sessions off for a duplicate GUI client
/// id, keyed the same way as [`TRACES`].
///
/// The engine reconnects a dead source on its own ([`IqSource::needs_reopen`]),
/// and reconnecting under the id we were just evicted over does not recover the
/// session — it evicts whoever took it, who reconnects and evicts us back. Two
/// sdroxide windows, or one that overlapped its own reconnect, then trade the
/// radio forever at a few seconds a turn.
///
/// So the eviction is remembered for the life of the process and the next
/// connection to that radio goes out under a transient identity, which nobody
/// can be holding. The cost is the radio's slice restore, which an operator who
/// wants it back can buy with an explicit `gui_client_id` in the config.
static EVICTED: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn evicted_here(key: &str) -> bool {
    EVICTED.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|k| k == key)
}

fn remember_eviction(key: &str) {
    let mut list = EVICTED.lock().unwrap_or_else(|e| e.into_inner());
    if !list.iter().any(|k| k == key) {
        list.push(key.to_string());
    }
}

/// Which FlexRadio a session was with: the address it was dialled at, the one
/// thing both the source and the tab asking for the report can spell.
fn session_key(cfg: &sdroxide_types::SmartSdrConfig) -> String {
    cfg.target().unwrap_or("").trim().to_ascii_lowercase()
}

fn record_trace(key: &str, trace: &sdroxide_smartsdr::Trace) {
    TRACES.record(key, trace.dump());
}

pub struct SmartSdrSource {
    handle: FlexHandle,
    /// Which radio this session is with, for [`TRACES`] — see the field of the
    /// same name on [`crate::icomnet_source::IcomNetSource`].
    key: String,
    center: f64,
    scratch: Vec<f32>,
    label: String,
    /// Last VFO-within-span offset pushed to the radio (dedup).
    if_offset: f64,
    /// Latest telemetry the radio reported while keyed, latched for the engine's
    /// meter poll. Cleared on unkey.
    last_telem: Option<TxTelemetry>,
    /// Kept so the trace survives the handle being dropped.
    trace: sdroxide_smartsdr::Trace,
    /// Warning captured while opening, surfaced in the UI.
    warning: Option<String>,
    /// Conjugate what the radio sends, mirroring the spectrum — see
    /// [`sdroxide_types::SmartSdrConfig::swap_iq`].
    swap_iq: bool,
    /// The rate asked for, which is what [`IqSource::sample_rate`] answers until
    /// the radio has sent a packet to say otherwise.
    requested_rate: f64,
    /// The radio's built-in antenna tuner as it last reported itself, or
    /// `None` where it has none (or has not said yet).
    atu: Option<sdroxide_types::AtuState>,
}

impl SmartSdrSource {
    pub fn open(
        cfg: &sdroxide_types::SmartSdrConfig,
        center_hz: f64,
    ) -> anyhow::Result<SmartSdrSource> {
        let address = cfg
            .target()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "no FlexRadio selected — press Discover in Settings → Radio, or enter \
                     the radio's address"
                )
            })?
            .to_string();

        let key = session_key(cfg);
        let mut handle = FlexHandle::connect_with(&sdroxide_smartsdr::ConnectOptions {
            address: address.clone(),
            iq_rate_hz: cfg.iq_sample_rate_hz,
            iq_channel: cfg.iq_channel,
            station: cfg.station.clone(),
            client_id: cfg.gui_client_id.clone(),
            transient_identity: evicted_here(&key),
            network_mtu: cfg.network_mtu,
        })?;
        let trace = handle.trace.clone();

        let mut warning = None;
        let actual = wait_for_rate(&handle, cfg.iq_sample_rate_hz);
        // Whatever arrived before the rate change landed is 48 kHz I/Q, and the
        // engine is about to read the ring at the rate just settled on. Drawn at
        // the wrong rate it is a quarter of a second of band smeared across the
        // waterfall — gone, but not worth showing.
        handle.discard_pending_rx();

        // Nothing at all is a different fault from the wrong rate, and it is the
        // one an operator can act on: the control link is up (we got this far),
        // so what is missing is the radio's UDP reaching us. Naming the likely
        // cause is the difference between "it doesn't work" and a firewall rule.
        if wait_for_any_packet(&handle) {
            warning = Some(format!(
                "connected, but not one UDP packet has arrived from {address}. The radio \
                 reports the stream as running, so the control link is fine and its \
                 spectrum — which travels by UDP — is being blocked before it reaches \
                 here. Check this computer's firewall first: SmartSDR installs a rule for \
                 itself and sdroxide has none. Then a VPN, then the Network MTU setting."
            ));
            tracing::warn!(target: "smartsdr", "{}", warning.as_deref().unwrap_or(""));
        } else if (actual - cfg.iq_sample_rate_hz).abs() > 1.0 {
            // The radio creates every DAX IQ stream at 48 kHz and only moves on a
            // follow-up command; if it refused that, the engine would run its whole
            // DSP chain at a rate the samples are not arriving at, which sounds like
            // a badly detuned receiver rather than like an error. Say so instead.
            warning = Some(format!(
                "the radio is streaming {:.0} kHz IQ, not the {:.0} kHz requested — \
                 DAX IQ channel {} may be shared, or the radio declined the rate",
                actual / 1000.0,
                cfg.iq_sample_rate_hz / 1000.0,
                cfg.iq_channel
            ));
            tracing::warn!(target: "smartsdr", "{}", warning.as_deref().unwrap_or(""));
        }

        let label = format!(
            "{} @ {address} ({:.0} kHz DAX IQ)",
            handle.ident.display_name(),
            rate_or(actual, cfg.iq_sample_rate_hz) / 1000.0,
        );
        handle.set_center(center_hz);

        Ok(SmartSdrSource {
            center: center_hz,
            scratch: Vec::new(),
            label,
            handle,
            key,
            if_offset: 0.0,
            last_telem: None,
            trace,
            warning,
            swap_iq: cfg.swap_iq,
            requested_rate: cfg.iq_sample_rate_hz,
            atu: None,
        })
    }

    pub fn model(&self) -> &str {
        &self.handle.ident.model
    }
}

/// Wait for packets at the requested rate, then return the rate the radio is
/// really sending — `0.0` if it has sent nothing at all.
///
/// This is the rate the engine builds its whole receive chain for, once, as it
/// adopts the source, so settling early is not a small error. The radio starts
/// every DAX IQ stream at 48 kHz and moves only on a follow-up command — 260 ms
/// later on a FLEX-8400 on the same LAN, longer across a VPN — and a source
/// that settled in between ran a 192 kHz stream through a 48 kHz chain for the
/// rest of the session: every signal a quarter of its width, and every click
/// tuning to somewhere else (issue #687).
///
/// Bounded rather than open-ended: a radio that never streams, or refuses the
/// rate, is a fault to report, not a reason to hang the whole startup.
fn wait_for_rate(handle: &FlexHandle, requested: f64) -> f64 {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        let rate = handle.actual_rate_hz();
        if (rate - requested).abs() < 1.0 {
            return rate;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    handle.actual_rate_hz()
}

/// Whether the radio has sent nothing at all, after giving it long enough to be
/// sure.
///
/// Long enough matters: the client's registration datagrams go out over the
/// first two seconds of the session, so a verdict passed before that is a
/// verdict on a question still being asked.
fn wait_for_any_packet(handle: &FlexHandle) -> bool {
    let deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < deadline {
        if !handle.trace.stream_counters().is_empty() {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    handle.trace.stream_counters().is_empty()
}

impl Drop for SmartSdrSource {
    fn drop(&mut self) {
        record_trace(&self.key, &self.trace);
    }
}

/// `actual`, or `fallback` while no packet has said what the rate is.
fn rate_or(actual: f64, fallback: f64) -> f64 {
    if actual > 0.0 { actual } else { fallback }
}

impl IqSource for SmartSdrSource {
    fn sample_rate(&self) -> f64 {
        rate_or(self.handle.actual_rate_hz(), self.requested_rate)
    }

    fn center_hz(&self) -> f64 {
        self.center
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        self.handle.set_center(hz);
        Ok(())
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let need = buf.len() * 2;
        if self.scratch.len() < need {
            self.scratch.resize(need, 0.0);
        }
        let n = self.handle.rx_read(&mut self.scratch[..need]);
        let pairs = n / 2;
        if pairs == 0 {
            std::thread::sleep(Duration::from_millis(2));
            return Ok(0);
        }
        // Negating Q mirrors the spectrum about the panadapter centre. Receive
        // only: transmit is audio the radio modulates itself, so there is no
        // second direction here to keep in step (issue #368).
        let q = if self.swap_iq { -1.0 } else { 1.0 };
        for p in 0..pairs {
            buf[p] = Complex32::new(self.scratch[2 * p], q * self.scratch[2 * p + 1]);
        }
        Ok(pairs)
    }

    fn describe(&self) -> String {
        self.label.clone()
    }

    fn open_status(&self) -> Option<String> {
        self.warning.clone()
    }

    fn set_control_mode(&mut self, mode: sdroxide_types::Mode) -> Result<()> {
        self.handle.set_mode(mode);
        Ok(())
    }

    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        let mut out = Vec::new();
        for u in self.handle.poll_updates() {
            match u {
                FlexUpdate::Freq(hz) => out.push(ControlUpdate::Freq(hz)),
                FlexUpdate::Mode(m) => out.push(ControlUpdate::Mode(m)),
                FlexUpdate::TxDrive(f) => out.push(ControlUpdate::TxDrive(f)),
                FlexUpdate::TuneDrive(f) => out.push(ControlUpdate::TuneDrive(f)),
                // Held rather than passed on: the engine asks for it through
                // `atu`, the way it asks after a receiving antenna.
                FlexUpdate::Atu(a) => self.atu = a,
                // An ATU tune, or SmartSDR or the front panel keying the
                // radio: the engine mutes the receiver and stays off the air
                // for the length of it, as for any over the radio keys itself.
                FlexUpdate::RigTx(on) => out.push(ControlUpdate::RigTx(on)),
            }
        }
        out
    }

    fn atu(&self) -> Option<sdroxide_types::AtuState> {
        self.atu
    }

    fn atu_tune(&mut self) -> Result<()> {
        self.handle.atu_start();
        Ok(())
    }

    fn atu_bypass(&mut self) -> Result<()> {
        self.handle.atu_bypass();
        Ok(())
    }

    fn tx_begin(&mut self, center_hz: f64, _rate: f64) -> Result<f64> {
        Ok(self.handle.tx_begin(center_hz))
    }

    fn tx_write_audio(&mut self, audio: &[f32]) -> Result<()> {
        self.handle.tx_write(audio);
        Ok(())
    }

    /// Let the queued audio reach the radio before PTT drops. The engine hands
    /// us a burst faster than real time and the radio plays it out at 24 kHz, so
    /// unkeying immediately would cut the tail (FT8 needs every symbol).
    fn tx_drain(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while self.handle.tx_pending() > 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        // The last packet handed over is still playing out in the radio.
        std::thread::sleep(Duration::from_millis(40));
    }

    fn tx_end(&mut self) -> Result<()> {
        self.handle.tx_end();
        self.last_telem = None; // drop the stale SWR reading on unkey
        Ok(())
    }

    fn discard_pending_rx(&mut self) {
        self.handle.discard_pending_rx();
    }

    fn tx_telemetry(&mut self) -> Option<TxTelemetry> {
        if let Some(t) = self.handle.poll_telemetry() {
            self.last_telem = Some(t);
        }
        self.last_telem
    }

    fn set_tx_drive(&mut self, frac: f64) {
        self.handle.set_drive(frac);
    }

    fn set_tune_drive(&mut self, frac: f64) {
        self.handle.set_tune_drive(frac);
    }

    /// `transmit set rfpower=` is the radio's real power control, so the TX
    /// audio we stream must stay at full scale.
    fn commands_tx_power(&self) -> bool {
        true
    }

    /// The control thread stops when the radio closes the TCP session (powered
    /// off, or another client evicted ours); the engine then reconnects on its
    /// own.
    ///
    /// An eviction is filed on the way past. The engine's retry has no idea why
    /// the session ended, and reconnecting under an id we were just thrown off
    /// for would only start the trade again — see [`EVICTED`].
    fn needs_reopen(&self) -> bool {
        if self.handle.is_alive() {
            return false;
        }
        if self.handle.evicted_for_duplicate_id() {
            remember_eviction(&self.key);
        }
        true
    }

    fn set_if_offset(&mut self, hz: f64) {
        if (hz - self.if_offset).abs() > 0.5 {
            self.if_offset = hz;
            self.handle.set_if_offset(hz);
        }
    }

    /// Hand the radio's streams back before the engine builds a replacement.
    ///
    /// Not strictly required — a FLEX reaps a client's streams when its TCP
    /// session ends — but the DAX IQ channel is a limited resource, and a
    /// reconnect that raced its own predecessor for channel 1 would fail with
    /// "channel in use" against a stream we ourselves had just abandoned.
    fn release(&mut self) {
        record_trace(&self.key, &self.trace);
        self.handle.tx_end();
    }
}

/// Discover FlexRadios and map them to the settings UI's device type.
pub fn discover() -> Vec<sdroxide_types::SmartSdrDevice> {
    sdroxide_smartsdr::discover_default()
        .into_iter()
        .map(|r| sdroxide_types::SmartSdrDevice {
            joinable: r.joinable(),
            ip: r.ip,
            port: r.port,
            model: r.model,
            serial: r.serial,
            nickname: r.nickname,
            version: r.version,
            gui_clients: r.gui_clients,
        })
        .collect()
}

/// Shared with the settings UI's "Copy diagnostic report" button: the trace of
/// the radio *this* configuration names, never another one's.
pub fn diagnostics_or_hint(cfg: &sdroxide_types::SmartSdrConfig) -> String {
    match TRACES.get(&session_key(cfg)) {
        Some(t) => format!("{t}\n{}\n", sdroxide_smartsdr::FIELD_REPORT_HINT),
        None => format!(
            "No SmartSDR session has run yet for {} — connect to that radio first.\n\n{}\n",
            cfg.target().unwrap_or("this radio"),
            sdroxide_smartsdr::FIELD_REPORT_HINT
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_smartsdr::sim::{SimConfig, SimRadio};

    /// The source settles on the rate the radio *ends up* streaming, not on the
    /// 48 kHz every DAX IQ stream starts at. The engine reads `sample_rate` once
    /// as it adopts the source and builds its whole receive chain for it, so a
    /// source that answered during the change ran a 192 kHz stream through a
    /// 48 kHz chain for the rest of the session — issue #687, reproduced on a
    /// FLEX-8400 whose rate change takes 260 ms.
    #[test]
    fn settles_on_the_rate_after_the_radio_changes_it() {
        let sim = SimRadio::start(SimConfig {
            rate_change_delay: Duration::from_millis(600),
            ..SimConfig::default()
        })
        .expect("start simulator");
        let cfg = sdroxide_types::SmartSdrConfig {
            address: sim.address(),
            iq_sample_rate_hz: 192_000.0,
            station: "test".into(),
            ..Default::default()
        };
        let src = SmartSdrSource::open(&cfg, 14_074_000.0).expect("open");

        assert_eq!(src.sample_rate(), 192_000.0);
        assert!(src.describe().contains("192 kHz"), "{}", src.describe());
        assert!(src.open_status().is_none(), "{:?}", src.open_status());
    }
}
