use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    Rx,
    Tx,
}

/// What the numbers on a gain element actually are, which is what decides how
/// a control may label them.
///
/// Nearly every front end's stages are in decibels and this is not a question
/// worth asking. An RSP's front end is the exception the type exists for: its
/// RF gain is an *index into a table*, and which table depends on the band —
/// step 7 is 24 dB of reduction at 0–12 MHz and 25 dB at 420–1000 MHz. A
/// slider that appended "dB" to that index reported a number three times too
/// small and in the wrong unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GainUnit {
    /// Decibels, and safe to label as such.
    #[default]
    Db,
    /// A step index, whose dB value the hardware alone knows. Show the number
    /// bare: an unlabelled index reads as an index, where one labelled dB is a
    /// wrong measurement.
    Step,
}

/// One adjustable gain stage exposed by the device.
///
/// The `*_db` names are historical: they are the range and the increment of
/// whatever [`Self::unit`] says this element is counted in, decibels or
/// otherwise. Renaming them would touch every backend for no gain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GainElement {
    pub name: String,
    pub direction: Direction,
    pub min_db: f64,
    pub max_db: f64,
    pub step_db: f64,
    /// Decibels unless the front end says otherwise — see [`GainUnit`].
    pub unit: GainUnit,
}

impl GainElement {
    /// A stage counted in decibels, which is all but one of them.
    pub fn db(
        name: impl Into<String>,
        direction: Direction,
        min: f64,
        max: f64,
        step: f64,
    ) -> Self {
        GainElement {
            name: name.into(),
            direction,
            min_db: min,
            max_db: max,
            step_db: step,
            unit: GainUnit::Db,
        }
    }

    /// A stage counted in steps of a table the hardware owns.
    pub fn steps(
        name: impl Into<String>,
        direction: Direction,
        min: f64,
        max: f64,
        step: f64,
    ) -> Self {
        GainElement { unit: GainUnit::Step, ..GainElement::db(name, direction, min, max, step) }
    }

    /// The suffix a control should put after the number, if any.
    pub fn suffix(&self) -> &'static str {
        match self.unit {
            GainUnit::Db => " dB",
            GainUnit::Step => "",
        }
    }
}

/// What a driver setting holds, which is what decides the control drawn for it.
///
/// SoapySDR's own four argument types. `String` is the fallback in both
/// directions: a driver that declares nothing gets a text box, which can express
/// any of the others, rather than a control that quietly cannot say what the
/// driver wants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SettingKind {
    Bool,
    Int,
    Float,
    #[default]
    String,
}

/// One driver-specific setting, as the device describes itself.
///
/// Not a fixed list and deliberately not interpreted: a HackRF's `bias_tx`, an
/// RTL-SDR's `direct_samp` and an RSP's `rfnotch_ctrl` all arrive here the same
/// way, and sdroxide draws a control for each without knowing what any of them
/// means. That is the whole point of reaching a radio through SoapySDR, and it
/// is why this is carried as data rather than as per-driver code.
///
/// Values are strings in both directions because that is what
/// `readSetting`/`writeSetting` take; [`Self::kind`] says how to *render* one,
/// not how to store it.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DeviceSetting {
    /// The key `writeSetting` takes. Stable; this is what gets persisted.
    pub key: String,
    /// The driver's display name, or the key again when it gave none.
    pub name: String,
    /// One line of help from the driver, if it offered any.
    pub description: String,
    /// dB, Hz, and so on. Empty when unitless.
    pub units: String,
    pub kind: SettingKind,
    /// The value read back from the device at probe time.
    pub value: String,
    /// The values the driver will accept, when it restricts them. Empty means
    /// "anything of this kind" — a range check is the driver's job, not ours,
    /// because only it knows what it will refuse.
    pub options: Vec<String>,
}

/// Device capabilities probed once at open time. Drives all UI adaptation
/// (e.g. `tx_channels == 0` hides every TX control).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DeviceCaps {
    pub driver: String,
    pub label: String,

    pub rx_channels: usize,
    pub tx_channels: usize,
    /// Whether RX keeps running during TX. Conservative default: false.
    pub full_duplex: bool,
    /// The source delivers already-demodulated real audio (a CAT rig on a
    /// sound card), so the engine bypasses the DDC/demod chain and shows a
    /// narrow audio-band panadapter. Sound-card *IQ* leaves this `false` and
    /// runs the normal wideband path.
    pub audio_mode: bool,
    /// Transmit by sending raw 48 kHz audio (`IqSource::tx_write_audio`) — the
    /// device modulates it — instead of modulated IQ.
    ///
    /// Set by every backend that transmits this way, whatever its *receive*
    /// stream is carrying: a CAT rig on a sound card, an Icom over LAN, an
    /// ELAD, a FLEX over DAX, and TCI. [`Self::audio_mode`] is not a substitute
    /// for it — that flag is about the receive path, and a backend which sets
    /// it happens to satisfy the engine's `audio_mode || tx_audio` test by
    /// accident. A rig whose receive stream is wideband I/Q while its
    /// transmitter still takes audio has nothing but this flag standing between
    /// it and the modulated-I/Q path, where the first block asks an `IqSource`
    /// with no I/Q transmitter to write one.
    pub tx_audio: bool,

    /// Tunable ranges in Hz: (min, max).
    pub freq_ranges_rx: Vec<(f64, f64)>,
    pub freq_ranges_tx: Vec<(f64, f64)>,

    /// Discrete supported rates, if the device reports any.
    pub sample_rates: Vec<f64>,
    /// Continuous rate ranges: (min, max).
    pub rate_ranges: Vec<(f64, f64)>,

    pub gains: Vec<GainElement>,
    pub antennas_rx: Vec<String>,
    pub antennas_tx: Vec<String>,

    /// Sensor names from the SoapySDR sensor API (device- and channel-level).
    pub sensors: Vec<String>,
    pub has_swr_sensor: bool,
    pub has_fwd_power_sensor: bool,
    /// This front end's receive LO is shared with sibling streams on the same
    /// physical device (the AD9361's two receive chains have one
    /// synthesiser): retuning this radio moves the others, and theirs moves
    /// this one — the engine adopts such moves as centre changes. Appended
    /// last (postcard layout; `PROTO_VERSION` bumped with it).
    #[serde(default)]
    pub shared_lo_rx: bool,
    /// Receive audio arrives from a *separate* transceiver through
    /// [`IqSource::rx_audio`](../../sdroxide_radio/trait.IqSource.html), rather
    /// than from demodulating this stream: a radio with another radio attached
    /// as its panadapter, listening to the transceiver rather than to the
    /// receiver painting the picture.
    ///
    /// The stream itself is still ordinary wideband I/Q — this is not
    /// [`Self::audio_mode`], which says there is no I/Q at all. Appended last,
    /// for the same reason as `shared_lo_rx`.
    #[serde(default)]
    pub rx_audio_external: bool,
    /// This radio has no front end of its own because another radio in the
    /// station has borrowed its receiver as a panadapter: the id that radio is
    /// known by, on the station both belong to.
    ///
    /// Reported by the *lent* radio, and reported here rather than read from
    /// its configuration because this is about what is actually open. A
    /// pairing chosen in the settings dialog is not in force until Apply, and a
    /// tab that vanished the moment the combo was touched would be a tab the
    /// operator had not yet agreed to lose. Capabilities are announced when the
    /// source is established and at no other time, which is exactly the
    /// lifetime wanted.
    #[serde(default)]
    pub lent_to: Option<u32>,
    /// The receive baseband filter widths this device will take, in Hz:
    /// discrete values first, then any continuous ranges as (min, max).
    ///
    /// Separate from [`Self::sample_rates`] because they are separate controls
    /// on the hardware — a device is perfectly entitled to run a 2 Msps stream
    /// through a 1.75 MHz filter — and because plenty of drivers publish one
    /// and not the other. Appended last, for the same reason as `shared_lo_rx`.
    #[serde(default)]
    pub bandwidths: Vec<f64>,
    #[serde(default)]
    pub bandwidth_ranges: Vec<(f64, f64)>,
    /// The driver's own settings, as it describes them. See [`DeviceSetting`].
    #[serde(default)]
    pub settings: Vec<DeviceSetting>,
    /// This front end's centre *is* the dial: a transceiver whose I/Q output
    /// feeds a sound card, an Icom sending its 12 kHz IF. There is one
    /// synthesiser behind both, so tuning already moves the captured window and
    /// nothing may ask for a centre of its own — a second command to the same
    /// place is a second CAT write per frame while the panadapter is dragged.
    ///
    /// False on an SDR, where the window is a resource the dial moves inside
    /// and the centre can be commanded on its own. That is what lets a pan
    /// which has run off the end of the window carry the window with it (issue
    /// #133); see `IqSource::center_is_dial`, which the engine reports here.
    ///
    /// False too on a transceiver whose control port never answered: there is
    /// one synthesiser, but nothing this end can say to it, so the I/Q the
    /// radio is already sending is the whole receiver (issue #155). Re-sent
    /// whenever that changes, so this is not fixed for the life of a session.
    ///
    /// Appended last, for the same reason as `shared_lo_rx`.
    #[serde(default)]
    pub center_is_dial: bool,
    /// A diversity filter is running on this stream: two coherent aerials —
    /// a LimeSDR's two receive chains, an RSPduo's two tuners — combined into
    /// the one span this radio shows.
    ///
    /// What it is for is the main window: the filter has controls an operator
    /// works with *while listening* (which way it combines, and holding it the
    /// moment a null appears), and those belong on the strip rather than three
    /// clicks into a settings dialog. Every backend that has one drives it
    /// through the same pseudo-gain element names, so this one flag is all the
    /// strip needs to know.
    ///
    /// Reported by the source, not read from the configuration: a setting the
    /// hardware refused (no second tuner, a chain another radio has taken)
    /// must not put controls on screen that do nothing. Appended last, for the
    /// same reason as `shared_lo_rx`.
    #[serde(default)]
    pub diversity: bool,
    /// CW goes out as *audio* on this radio, so the digital transmit-audio
    /// level reaches it.
    ///
    /// False where the rig sends from its own keyer: there CW leaves as text
    /// over the control port and the sound card is not in the path at all, so
    /// a level control for it would be a control that does nothing. That is the
    /// distinction `Mode::takes_digi_tx_audio` deliberately cannot make —
    /// whether the audio is heard is a property of the radio, not of the mode.
    ///
    /// Reported by the source (`IqSource::cw_audio_keyed`) rather than read
    /// from the CAT configuration, because a backend that is not a CAT rig has
    /// no `cw_keying` field to read and still has an answer.
    ///
    /// Appended last, for the same reason as `shared_lo_rx`.
    #[serde(default)]
    pub cw_audio_keyed: bool,
    /// The radio has a squelch of its own that sdroxide can set, so the SQL
    /// control drives *that* rather than the engine's own gate
    /// ([`crate::RadioState::rig_squelch`] rather than
    /// [`crate::RxState::squelch_db`]).
    ///
    /// True on a transceiver that hands us audio it has already gated, where
    /// the rig's squelch is the only one that can open — a threshold on this
    /// side never hears what the radio muted (issue #192). False on every I/Q
    /// front end, where the engine has the whole passband and its own gate is
    /// the honest one.
    ///
    /// Reported by the source (`IqSource::commands_squelch`) rather than read
    /// from the CAT configuration, for the same reason [`Self::cw_audio_keyed`]
    /// is: a backend that is not a CAT rig has no such field and still has an
    /// answer. Appended last, for the same reason as `shared_lo_rx`.
    #[serde(default)]
    pub commands_squelch: bool,
    /// How wide the front end's full-band lane is, in Hz, or `0.0` where it has
    /// none — see `IqSource::wide_spectrum_db`.
    ///
    /// The client needs this *before* the first full-band frame arrives, which
    /// is why it rides the capabilities and not the frame. It is what bounds
    /// the panadapter's zoom-out on a receiver whose I/Q is a narrow window
    /// onto a wide band, and a client that had to wait for a picture to learn
    /// it would spend the first frames of every session believing the passband
    /// was the limit — and would shrink a restored window to it before the lane
    /// had said otherwise.
    ///
    /// A span only: *where* it sits moves with the receiver and is carried on
    /// each frame, where it belongs. Appended last, so every field already on
    /// the wire keeps its number.
    #[serde(default)]
    pub wide_span_hz: f64,
    /// The radio can be switched off — and back on — from here.
    ///
    /// A transceiver with a remote-control port, not an SDR: an Icom answers
    /// `18 00` / `18 01` on CI-V whether that link is a serial cable or the
    /// radio's own network socket, and keeps the control end of it alive while
    /// the set is off so that the second of those can reach it (issue #239).
    /// False everywhere else, including on every I/Q front end, where the only
    /// power switch is the USB cable.
    ///
    /// Reported by the source (`IqSource::commands_rig_power`), for the reason
    /// [`Self::commands_squelch`] is: a backend that is not a CAT rig has no
    /// such field and still has an answer. Appended last, so every field
    /// already on the wire keeps its number.
    #[serde(default)]
    pub commands_rig_power: bool,
}

impl DeviceCaps {
    pub fn is_transmit_capable(&self) -> bool {
        self.tx_channels > 0
    }

    /// Whether a *published* receive range covers `hz`. A device that publishes
    /// none can reach nothing by this answer, so anything deciding whether to
    /// allow or offer a frequency wants [`Self::may_rx_hz`] instead.
    pub fn can_rx_hz(&self, hz: f64) -> bool {
        self.freq_ranges_rx.iter().any(|&(lo, hi)| hz >= lo && hz <= hi)
    }

    /// Whether a *published* transmit range covers `hz`. As with
    /// [`Self::can_rx_hz`], "no ranges" reads as "nothing" here; the gate that
    /// decides whether a key-down is allowed is [`Self::may_tx_hz`].
    pub fn can_tx_hz(&self, hz: f64) -> bool {
        self.freq_ranges_tx.iter().any(|&(lo, hi)| hz >= lo && hz <= hi)
    }

    /// Whether receiving here is permitted: inside a published range, or
    /// anywhere at all on a device that publishes no ranges.
    ///
    /// Publishing a tuning range is optional in SoapySDR — `getFrequencyRange`
    /// has no meaningful default and plenty of drivers never implement it — so
    /// an empty list means "this driver didn't say", not "this radio tunes
    /// nowhere". Taking silence as a prohibition would leave such a device
    /// unable to do the thing it is demonstrably doing.
    pub fn may_rx_hz(&self, hz: f64) -> bool {
        self.freq_ranges_rx.is_empty() || self.can_rx_hz(hz)
    }

    /// Whether any part of `lo..=hi` is receivable — inside a published range,
    /// or anywhere at all on a device that publishes none.
    ///
    /// The band-button rule (issue #272). Asking whether the band's *edges* are
    /// reachable is not the same question and gets a band wrong whenever the
    /// radio reaches into it without reaching either end of it: a receiver
    /// published as 50.1–51 MHz covers most of 6 m and answers "no" to both
    /// 50.000 and 52.000. What matters for offering a band is whether there is
    /// anything in it to tune to.
    pub fn may_rx_span(&self, lo: f64, hi: f64) -> bool {
        let (lo, hi) = (lo.min(hi), lo.max(hi));
        self.freq_ranges_rx.is_empty()
            || self.freq_ranges_rx.iter().any(|&(a, b)| b >= lo && a <= hi)
    }

    /// Whether transmitting here is permitted, by the same rule as
    /// [`Self::may_rx_hz`]: a driver that publishes no transmit range is taken
    /// at its word rather than silenced.
    ///
    /// This is not the only thing between an operator and the antenna —
    /// [`Self::is_transmit_capable`] has already established there is a
    /// transmitter, the amateur-band gate still applies, and the driver may
    /// still refuse the tune. An operator who wants a firmer limit than the
    /// driver gives can state one: see `RadioConfig::freq_ranges_tx`.
    pub fn may_tx_hz(&self, hz: f64) -> bool {
        self.freq_ranges_tx.is_empty() || self.can_tx_hz(hz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The case this rule exists for: a driver (SoapySX among others) that
    /// implements neither frequency-range call. Its ranges arrive empty, and
    /// every empty list has to read as "unknown", never as "nothing" — the
    /// device receives and transmits perfectly well.
    #[test]
    fn a_device_that_publishes_no_ranges_may_still_receive_and_transmit() {
        let silent = DeviceCaps { rx_channels: 1, tx_channels: 1, ..Default::default() };
        for hz in [1_800_000.0, 145_500_000.0, 435_000_000.0] {
            assert!(silent.may_rx_hz(hz), "receive at {hz} Hz");
            assert!(silent.may_tx_hz(hz), "transmit at {hz} Hz");
            // The strict form still answers "not in any published range", which
            // is what it is for.
            assert!(!silent.can_rx_hz(hz));
            assert!(!silent.can_tx_hz(hz));
        }
    }

    /// A band is offered when the radio reaches *into* it, which is not the
    /// same question as whether either of its edges is reachable — issue #272.
    #[test]
    fn a_band_is_reachable_when_it_overlaps_the_range() {
        let (m6_lo, m6_hi) = (50_000_000.0, 52_000_000.0);
        // Well inside the band and touching neither edge: both edge tests say
        // no, and the radio plainly has the band.
        let sliver = DeviceCaps {
            rx_channels: 1,
            freq_ranges_rx: vec![(50_100_000.0, 51_000_000.0)],
            ..Default::default()
        };
        assert!(!sliver.may_rx_hz(m6_lo) && !sliver.may_rx_hz(m6_hi), "neither edge");
        assert!(sliver.may_rx_span(m6_lo, m6_hi), "but the band is there");

        // An HF-only receive range, which is the fault as reported: 6 m really
        // is out of reach and the button really should be dead.
        let hf = DeviceCaps {
            rx_channels: 1,
            freq_ranges_rx: vec![(0.0, 30_000_000.0)],
            ..Default::default()
        };
        assert!(!hf.may_rx_span(m6_lo, m6_hi));
        assert!(hf.may_rx_span(14_000_000.0, 14_350_000.0), "20 m still works");

        // Silence is still "the driver didn't say", here as everywhere.
        assert!(DeviceCaps::default().may_rx_span(m6_lo, m6_hi));
    }

    /// A device that does publish its ranges is held to them.
    #[test]
    fn published_ranges_are_still_enforced() {
        let caps = DeviceCaps {
            rx_channels: 1,
            tx_channels: 1,
            freq_ranges_rx: vec![(100_000.0, 148_000_000.0)],
            freq_ranges_tx: vec![(1_800_000.0, 54_000_000.0)],
            ..Default::default()
        };
        assert!(caps.may_rx_hz(145_000_000.0));
        assert!(!caps.may_rx_hz(435_000_000.0));
        assert!(caps.may_tx_hz(14_200_000.0));
        assert!(!caps.may_tx_hz(145_000_000.0), "outside the transmit range and inside the RX one");
        // Edges are inclusive, both ends.
        assert!(caps.may_tx_hz(1_800_000.0) && caps.may_tx_hz(54_000_000.0));
    }

    /// The default matters more than it looks: every backend but one builds
    /// its stages with [`GainElement::db`], and a stage that arrived without
    /// an opinion must be labelled decibels rather than left bare.
    #[test]
    fn a_stage_is_decibels_unless_it_says_otherwise() {
        assert_eq!(GainUnit::default(), GainUnit::Db);
        let g = GainElement::db("LNA", Direction::Rx, 0.0, 40.0, 1.0);
        assert_eq!(g.unit, GainUnit::Db);
        assert_eq!(g.suffix(), " dB");
    }

    /// A step index carries no unit, because the one it would be given is
    /// wrong: an RSP's state 7 is 24 dB at 0–12 MHz and 25 dB at 420 MHz, so
    /// "7 dB" is neither the number nor the unit.
    #[test]
    fn a_step_index_is_never_labelled_decibels() {
        let g = GainElement::steps("LNA", Direction::Rx, -18.0, 0.0, 1.0);
        assert_eq!(g.unit, GainUnit::Step);
        assert_eq!(g.suffix(), "");
        // The range still describes the same element, just not in dB.
        assert_eq!((g.min_db, g.max_db, g.step_db), (-18.0, 0.0, 1.0));
    }
}
