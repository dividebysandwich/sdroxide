//! A radio's external transmit/receive switch, as its engine sees it.
//!
//! Each radio has a [`TrSwitch`] of its own, driving at most one relay module —
//! the box that grounds that radio's SDR antenna, keys its amplifier and
//! switches its band filters. A module belongs to exactly one radio (see
//! [`RelayClaim`]) and follows only that radio: its receive dial, its transmit
//! dial, its key-downs. Another radio keying does not touch it; a station whose
//! other receivers need protecting wires that from the owning radio's contacts.
//!
//! Inside, the hub still keeps its on-air state as one bit per radio and its
//! transmit bands by radio id — the arbitration it was built with when one
//! switch was shared by every radio — and with a single radio publishing into
//! it that collapses to exactly "this radio's over".
//!
//! The hardware itself lives behind `sdroxide_relay::RelayHandle`, which is a
//! thread and a port. This type is the arbitration in front of it, and it is
//! deliberately almost nothing: two atomics on the hot path, because
//! [`TrSwitch::publish`] is called on every engine tick and [`TrSwitch::key`]
//! sits in the key-down path at its most time-critical moment.
//!
//! # What the engine owes it
//!
//! * [`TrSwitch::key`] **before** RF, and the caller must wait the returned
//!   lead. That is the one guarantee the whole subsystem exists to make.
//! * [`TrSwitch::unkey`] after RF stops — the hold times are the driver's
//!   business, so this returns at once.
//! * [`TrSwitch::abort`] when a key-down was refused *after* the contacts were
//!   thrown. No RF appeared, so there is nothing to protect on the way out and
//!   the hold would only delay the receiver coming back.
//! * [`TrSwitch::publish`] every tick, for the overs sdroxide does not drive:
//!   a transceiver keyed at its own microphone, or a rig sending CW from its
//!   own keyer.
//! * [`TrSwitch::set_tx_band`] and [`TrSwitch::set_rx_band`] whenever its
//!   dials change band, for a relay bank that also switches external filters
//!   or a transverter by band (`sdroxide_types::RelayRole::BandDecoder`, issue
//!   #442). Bands, not output words: the worker resolves them against its
//!   own configuration and decides RX word or TX word from the on-air ramp,
//!   so a band-decoder contact moves inside its lead like every other one.
//!   [`TrSwitch::key`] sends the TX band ahead of the key-down itself.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use sdroxide_types::{Band, FailSafe, RelayConfig, RelayStatus};

/// See the module doc.
#[derive(Default)]
pub struct TrSwitch {
    /// One bit per radio id. Non-zero means the station is on the air.
    on_air: AtomicU64,
    /// The driver, installed by whichever engine owns the configuration.
    driver: std::sync::Mutex<Option<sdroxide_relay::RelayHandle>>,
    /// Whether a switch is configured at all, so the hot path can answer
    /// without taking the lock.
    configured: AtomicBool,
    /// Whether a switch that will not answer should refuse the over.
    refuse: AtomicBool,
    /// Why the switch could not be opened, when it could not. Separate from the
    /// driver's own status because there is no driver to ask.
    open_error: std::sync::Mutex<Option<String>>,
    /// Which radio the transmit-sense line belongs to.
    ///
    /// Here rather than read from each engine's own copy of the configuration,
    /// because those copies drift: the engine that was handed a
    /// `Command::SetRelayConfig` updates its own and the others keep what they
    /// loaded at startup. Every other stale field is cosmetic; this one decides
    /// which receiver gets muted and which key-down gets refused when a
    /// transmitter out in the shack comes up, so it lives with the driver it
    /// came in with.
    sense_radio: AtomicU32,
    /// What the band decoder has been told, kept here so a rebuilt driver can
    /// be told it again and so the TX band sent at key-down is the keying
    /// radio's.
    bands: std::sync::Mutex<Bands>,
}

/// See [`TrSwitch::bands`].
#[derive(Default)]
struct Bands {
    /// The owning radio's receive band.
    rx: Option<Band>,
    /// Each radio's transmit band, by radio id.
    tx: std::collections::HashMap<u32, Band>,
    /// The radio whose key-down started the current over. Its transmit band
    /// is the one the filters follow until the last radio unkeys.
    keyer: Option<u32>,
}

impl TrSwitch {
    pub fn new() -> TrSwitch {
        TrSwitch::default()
    }

    /// Install (or replace, or remove) the driver.
    ///
    /// The old handle is taken out *under* the lock and dropped *outside* it:
    /// dropping one joins a thread, and a key-down queueing behind that would
    /// be a key-down waiting on a serial port to close.
    pub fn install(&self, handle: Option<sdroxide_relay::RelayHandle>, cfg: &RelayConfig) {
        self.configured.store(cfg.enabled(), Ordering::Release);
        self.refuse.store(cfg.fail_safe == FailSafe::RefuseTx, Ordering::Release);
        self.sense_radio.store(cfg.sense.radio, Ordering::Release);
        // A new driver starts knowing no band. Only ever installed off the
        // air (see `Engine::sync_relay`), so the TX band to hand it is the
        // one a sensed over would transmit on.
        //
        // `bands` is held across the swap, taken before `driver` as `key`
        // takes them: a band another engine reports meanwhile then reaches
        // the new driver after this replay, rather than the old one on its
        // way out.
        let old = {
            let b = self.bands.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(h) = handle.as_ref() {
                if let Some(rx) = b.rx {
                    h.set_rx_band(rx);
                }
                if let Some(tx) = b.tx.get(&cfg.sense.radio).copied() {
                    h.set_tx_band(tx);
                }
            }
            let mut slot = self.driver.lock().unwrap_or_else(|e| e.into_inner());
            std::mem::replace(&mut *slot, handle)
        };
        drop(old);
    }

    /// Record why there is no driver, for the status line and the refusal.
    pub fn set_open_error(&self, why: Option<String>) {
        *self.open_error.lock().unwrap_or_else(|e| e.into_inner()) = why;
    }

    /// Whether any radio is on the air. A configuration must not be applied
    /// while this is true — rebuilding the driver throws every contact.
    pub fn busy(&self) -> bool {
        self.on_air.load(Ordering::Acquire) != 0
    }

    /// Claim the air for `radio` and throw the contacts. Returns how long the
    /// caller must wait before letting RF out.
    ///
    /// Zero when the station was already on the air: the contacts are closed
    /// already and a second radio joining an over it is sharing has nothing to
    /// wait for.
    pub fn key(&self, radio: u32) -> std::time::Duration {
        let was = self.on_air.fetch_or(bit(radio), Ordering::AcqRel);
        if was != 0 {
            return std::time::Duration::ZERO;
        }
        // This radio's transmit band, ahead of the key-down on the same
        // channel: the worker has it before the edge, and the edge's lead is
        // what switches the band-decoder contacts to it. `bands` is held until
        // both are sent, so no other radio's band can land between them — see
        // [`TrSwitch::set_tx_band`].
        let mut b = self.bands.lock().unwrap_or_else(|e| e.into_inner());
        b.keyer = Some(radio);
        let tx = b.tx.get(&radio).copied();
        if !self.configured.load(Ordering::Acquire) {
            return std::time::Duration::ZERO;
        }
        match self.driver.lock() {
            Ok(d) => d
                .as_ref()
                .map(|h| {
                    if let Some(tx) = tx {
                        h.set_tx_band(tx);
                    }
                    h.key()
                })
                .unwrap_or_default(),
            Err(_) => std::time::Duration::ZERO,
        }
    }

    /// Release `radio`'s claim. The contacts open only when the last one goes.
    pub fn unkey(&self, radio: u32) {
        let was = self.on_air.fetch_and(!bit(radio), Ordering::AcqRel);
        if was & bit(radio) == 0 || was & !bit(radio) != 0 {
            // Either this radio was not on the air, or somebody else still is.
            return;
        }
        let mut b = self.bands.lock().unwrap_or_else(|e| e.into_inner());
        let tx = self.over_ended(&mut b);
        if let Ok(d) = self.driver.lock()
            && let Some(h) = d.as_ref()
        {
            h.unkey();
            if let Some(tx) = tx {
                h.set_tx_band(tx);
            }
        }
    }

    /// Release `radio`'s claim with no hold — the key-down was refused after
    /// the contacts were thrown, so nothing ever reached the air.
    pub fn abort(&self, radio: u32) {
        let was = self.on_air.fetch_and(!bit(radio), Ordering::AcqRel);
        if was & !bit(radio) != 0 {
            return; // another radio is genuinely transmitting
        }
        let mut b = self.bands.lock().unwrap_or_else(|e| e.into_inner());
        let tx = self.over_ended(&mut b);
        if let Ok(d) = self.driver.lock()
            && let Some(h) = d.as_ref()
        {
            h.abort();
            if let Some(tx) = tx {
                h.set_tx_band(tx);
            }
        }
    }

    /// The last radio is off the air: nobody is the keyer any more, and the
    /// band decoder's idea of "the next over" goes back to the sense radio's
    /// band — the one over that can start without a [`TrSwitch::key`] to
    /// bring its own. The worker holds it back until the hold is done.
    fn over_ended(&self, b: &mut Bands) -> Option<Band> {
        b.keyer = None;
        b.tx.get(&self.sense_radio.load(Ordering::Acquire)).copied()
    }

    /// Reconcile `radio`'s bit with what it is actually doing. Called every
    /// engine tick, so it is one atomic in the common case.
    ///
    /// This is the path for the overs sdroxide does not drive and cannot lead:
    /// a transceiver keyed at its own microphone, or a rig sending CW from its
    /// own keyer. The relay follows as soon as the engine notices, which is the
    /// best software can do — see `sdroxide_types::SenseConfig` for the wire
    /// that makes "as soon as the engine notices" mean milliseconds.
    pub fn publish(&self, radio: u32, on_air: bool) {
        let held = self.on_air.load(Ordering::Acquire) & bit(radio) != 0;
        if held == on_air {
            return;
        }
        if on_air {
            let _ = self.key(radio);
        } else {
            self.unkey(radio);
        }
    }

    /// Why a key-down should be refused, if it should.
    ///
    /// Only ever `Some` when the operator asked for [`FailSafe::RefuseTx`] and
    /// the switch is not in a state to protect anything. A switch that is
    /// simply not configured refuses nothing.
    pub fn refusal(&self) -> Option<String> {
        if !self.configured.load(Ordering::Acquire) || !self.refuse.load(Ordering::Acquire) {
            return None;
        }
        if let Some(why) = self.open_error.lock().unwrap_or_else(|e| e.into_inner()).clone() {
            return Some(why);
        }
        let d = self.driver.lock().ok()?;
        let h = d.as_ref()?;
        let st = h.status();
        (!st.present)
            .then(|| st.error.unwrap_or_else(|| "the T/R switch is not answering".to_string()))
    }

    /// What to show the operator.
    pub fn status(&self) -> RelayStatus {
        if !self.configured.load(Ordering::Acquire) {
            return RelayStatus::default();
        }
        if let Some(why) = self.open_error.lock().unwrap_or_else(|e| e.into_inner()).clone() {
            return RelayStatus {
                configured: true,
                present: false,
                describe: String::new(),
                error: Some(why),
                keyed: false,
            };
        }
        match self.driver.lock() {
            Ok(d) => d.as_ref().map(|h| h.status()).unwrap_or_default(),
            Err(_) => RelayStatus::default(),
        }
    }

    /// The most recent sense-line edge nobody has acted on, taken — but only
    /// by the radio the wire belongs to.
    ///
    /// Gated here rather than at the call site so that exactly one engine can
    /// consume an edge: a second one asking would take the edge away from the
    /// radio that was supposed to act on it, which is worse than not asking.
    pub fn take_sense_edge(&self, radio: u32) -> Option<bool> {
        if radio != self.sense_radio.load(Ordering::Acquire) {
            return None;
        }
        let d = self.driver.lock().ok()?;
        d.as_ref()?.take_sense_edge()
    }

    /// The owning radio's receive band. See `sdroxide_relay::Sequencer::set_rx_band`.
    pub fn set_rx_band(&self, band: Band) {
        self.bands.lock().unwrap_or_else(|e| e.into_inner()).rx = Some(band);
        if let Ok(d) = self.driver.lock()
            && let Some(h) = d.as_ref()
        {
            h.set_rx_band(band);
        }
    }

    /// The band `radio` would transmit on. Every engine reports its own, so
    /// whichever radio keys brings its band with it — see [`TrSwitch::key`].
    ///
    /// Passed through at once only when it is the band the filters should
    /// already be on: the keyer's mid-over (the worker holds it back until
    /// the over is done), or the sense radio's between overs. Anyone else's
    /// waits for their own key-down.
    ///
    /// The decision and the send happen under one hold of `bands`, the same
    /// lock [`TrSwitch::key`] sends under: otherwise another radio's key-down
    /// could slip in between, and this band would reach the worker after that
    /// over's edge, as if it were the keyer's.
    pub fn set_tx_band(&self, radio: u32, band: Band) {
        let mut b = self.bands.lock().unwrap_or_else(|e| e.into_inner());
        b.tx.insert(radio, band);
        let forward = match b.keyer {
            Some(k) => k == radio,
            None => {
                self.on_air.load(Ordering::Acquire) == 0
                    && radio == self.sense_radio.load(Ordering::Acquire)
            }
        };
        if forward
            && let Ok(d) = self.driver.lock()
            && let Some(h) = d.as_ref()
        {
            h.set_tx_band(band);
        }
    }

    /// Which radio's transmit band the filters follow right now — the one
    /// whose key-down started the over, `None` off the air.
    #[cfg(test)]
    fn keyer(&self) -> Option<u32> {
        self.bands.lock().unwrap_or_else(|e| e.into_inner()).keyer
    }

    /// Pulse one contact so the operator can check their wiring. Refused by the
    /// driver while anything is on the air.
    pub fn test(&self, channel: u8) {
        if let Ok(d) = self.driver.lock()
            && let Some(h) = d.as_ref()
        {
            h.test(channel);
        }
    }
}

/// One bit per radio id. Ids above 63 share the top bit, which on a station
/// with sixty-four radios is not the problem anybody has.
fn bit(radio: u32) -> u64 {
    1u64 << (radio.min(63))
}

/// Which radio owns which relay module, process-wide.
///
/// Every radio has a switch of its own, and a module belongs to exactly one of
/// them: two radios driving the same board would each throw the other's
/// contacts and put the other's band filters in line. The hardware would
/// usually refuse the second open anyway — a serial port is exclusive — but a
/// HID device or a GPIO chip often is not, and "the band filters follow
/// whichever tab moved last" is a much worse failure than an error that names
/// the radio that already has it.
static RELAY_CLAIMS: std::sync::Mutex<Vec<(String, u32)>> = std::sync::Mutex::new(Vec::new());

/// What identifies the hardware `cfg` would open. Empty for a configuration
/// that opens nothing exclusive: no switch, or a command hook (whose script is
/// the operator's to arbitrate). A GPIO chip is claimed line by line, since
/// two radios can each have their own pins on one header.
pub fn relay_module_keys(cfg: &RelayConfig) -> Vec<String> {
    use sdroxide_types::RelayLink;
    let norm = |s: &str| {
        let s = s.trim();
        if cfg!(windows) { s.to_ascii_lowercase() } else { s.to_string() }
    };
    match cfg.link {
        RelayLink::Off | RelayLink::Command => Vec::new(),
        RelayLink::Serial | RelayLink::SerialLines => {
            vec![format!("serial:{}", norm(&cfg.serial.path))]
        }
        RelayLink::Hid => vec![format!("hid:{}", norm(&cfg.device))],
        RelayLink::Cm108 => vec![format!("cm108:{}:{}", norm(&cfg.device), cfg.cm108_pin)],
        RelayLink::Gpio => {
            let chip = norm(&cfg.device);
            cfg.gpio_lines.iter().map(|l| format!("gpio:{chip}:{l}")).collect()
        }
    }
}

/// A radio's hold on its relay module. Released when dropped — when the radio
/// is reconfigured, switched to another module, or closed.
#[derive(Debug)]
pub struct RelayClaim {
    radio: u32,
    keys: Vec<String>,
}

impl RelayClaim {
    /// Claim the module `cfg` names for `radio`. Refused, with the id of the
    /// radio that already has it, when another radio owns any part of it.
    pub fn take(cfg: &RelayConfig, radio: u32) -> Result<RelayClaim, u32> {
        let keys = relay_module_keys(cfg);
        let mut claims = RELAY_CLAIMS.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((_, owner)) =
            claims.iter().find(|(k, owner)| *owner != radio && keys.contains(k))
        {
            return Err(*owner);
        }
        for k in &keys {
            if !claims.iter().any(|(c, o)| c == k && *o == radio) {
                claims.push((k.clone(), radio));
            }
        }
        Ok(RelayClaim { radio, keys })
    }
}

impl Drop for RelayClaim {
    fn drop(&mut self) {
        let mut claims = RELAY_CLAIMS.lock().unwrap_or_else(|e| e.into_inner());
        claims.retain(|(k, o)| !(*o == self.radio && self.keys.contains(k)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A module belongs to one radio. The second radio to ask is told whose it
    /// is, and gets it once the first lets go.
    #[test]
    fn a_relay_module_is_owned_by_one_radio_at_a_time() {
        use sdroxide_types::RelayLink;
        let cfg = RelayConfig {
            link: RelayLink::Hid,
            device: "test-claim-hid-0001".into(),
            ..RelayConfig::default()
        };
        let first = RelayClaim::take(&cfg, 100).expect("the first radio is refused");
        assert_eq!(RelayClaim::take(&cfg, 101).unwrap_err(), 100);
        // The owner re-applying its own configuration is not a conflict.
        let again = RelayClaim::take(&cfg, 100).expect("the owner is refused its own module");
        drop(first);
        drop(again);
        let second = RelayClaim::take(&cfg, 101).expect("a released module stayed claimed");
        // A different board is somebody else's business entirely.
        let other = RelayConfig { device: "test-claim-hid-0002".into(), ..cfg.clone() };
        let _third = RelayClaim::take(&other, 100).expect("an unrelated board was refused");
        drop(second);
    }

    /// The case this type exists for. Two radios, one antenna relay: it must
    /// not open when the first one unkeys while the second is still on the air.
    #[test]
    fn the_last_radio_to_unkey_is_what_releases_the_contacts() {
        let s = TrSwitch::new();
        assert!(!s.busy());
        let _ = s.key(0);
        assert!(s.busy());
        let _ = s.key(1);
        s.unkey(0);
        assert!(s.busy(), "radio 1 is still transmitting");
        s.unkey(1);
        assert!(!s.busy());
    }

    #[test]
    fn an_unkey_from_a_radio_that_never_keyed_releases_nothing() {
        let s = TrSwitch::new();
        let _ = s.key(0);
        s.unkey(3);
        assert!(s.busy(), "radio 0 is still on the air");
    }

    #[test]
    fn publish_is_idempotent_and_follows_both_edges() {
        let s = TrSwitch::new();
        s.publish(2, false);
        assert!(!s.busy());
        s.publish(2, true);
        s.publish(2, true);
        assert!(s.busy());
        s.publish(2, false);
        assert!(!s.busy());
    }

    #[test]
    fn radio_id_zero_is_a_real_claim() {
        let s = TrSwitch::new();
        let _ = s.key(0);
        assert!(s.busy(), "id 0 must not read back as \"nobody\"");
    }

    /// Only the radio the wire is in takes the edge — and the others asking
    /// must not consume it out from under it.
    #[test]
    fn a_sense_edge_belongs_to_one_radio() {
        let s = TrSwitch::new();
        let cfg = RelayConfig {
            link: sdroxide_types::RelayLink::Serial,
            sense: sdroxide_types::SenseConfig {
                line: sdroxide_types::SenseLine::Cts,
                active_high: false,
                radio: 1,
            },
            ..RelayConfig::default()
        };
        s.install(None, &cfg);
        // No driver, so nobody gets an edge — but radio 0 must be turned away
        // before the driver is even consulted, which is what the gate is for.
        assert_eq!(s.take_sense_edge(0), None);
        assert_eq!(s.take_sense_edge(1), None);
    }

    #[test]
    fn a_switch_that_is_not_configured_refuses_nothing() {
        let s = TrSwitch::new();
        assert_eq!(s.refusal(), None);
        assert_eq!(s.status(), RelayStatus::default());
    }

    /// With no driver installed, the bands have nothing to forward to — and
    /// must not panic finding that out.
    #[test]
    fn bands_with_no_driver_do_nothing() {
        let s = TrSwitch::new();
        s.set_rx_band(Band::M20);
        s.set_tx_band(0, Band::M20);
    }

    /// On a multi-radio station the filters follow the radio that keyed, not
    /// the primary, and keep following it until the last radio is off the air.
    #[test]
    fn the_band_decoder_follows_the_radio_that_keyed() {
        let s = TrSwitch::new();
        s.set_tx_band(0, Band::M20);
        s.set_tx_band(1, Band::M40);
        let _ = s.key(1);
        assert_eq!(s.keyer(), Some(1));
        let _ = s.key(0);
        assert_eq!(s.keyer(), Some(1), "radio 0 joined an over radio 1 started");
        s.unkey(1);
        assert_eq!(s.keyer(), Some(1), "radio 0 is still on the air");
        s.unkey(0);
        assert_eq!(s.keyer(), None);
    }

    #[test]
    fn a_switch_that_would_not_open_refuses_the_over_when_asked_to() {
        let s = TrSwitch::new();
        let cfg = RelayConfig {
            link: sdroxide_types::RelayLink::Serial,
            fail_safe: FailSafe::RefuseTx,
            ..RelayConfig::default()
        };
        s.install(None, &cfg);
        s.set_open_error(Some("cannot open the T/R switch on /dev/ttyUSB9".into()));
        assert!(s.refusal().is_some_and(|r| r.contains("ttyUSB9")));

        // ...and does not, when the operator chose otherwise.
        let cfg = RelayConfig { fail_safe: FailSafe::WarnOnly, ..cfg };
        s.install(None, &cfg);
        assert_eq!(s.refusal(), None, "WarnOnly transmits and says so instead");
    }
}
