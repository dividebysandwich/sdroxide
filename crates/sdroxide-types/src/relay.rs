//! The station's external transmit/receive switch: relay boards, sequencers,
//! and the contact closures they all come down to.
//!
//! # What this is for
//!
//! An SDR sharing an antenna system with a transceiver has to be got out of the
//! way before the transceiver's RF appears, and put back afterwards. What hams
//! use for that is a relay — a coax relay that grounds the receiver's input, a
//! T/R switch, the antenna port on an amplifier — and every one of them is
//! driven the same way: a contact closes while transmitting and opens while
//! receiving.
//!
//! So this is not a driver for a board. It is a description of *when* a contact
//! should be closed, which the `sdroxide-relay` crate then produces on whatever
//! hardware is in the shack: a USB relay board, a serial handshake line, a
//! sound-card GPIO pin, a Raspberry Pi header, or a program the operator wrote.
//!
//! # Why the vocabulary lives here
//!
//! Same reason [`crate::LimeRfeConfig`] does: the settings panel is compiled to
//! wasm for the browser client, and a remote operator has to be able to set up
//! the switch on the machine the antenna is actually attached to. The hardware
//! half is native-only and lives in `sdroxide-relay`; nothing in this module
//! touches a port.
//!
//! # The sequencer
//!
//! Ordering falls out of the per-channel timings and needs no ordering field.
//! Every channel is asserted at `key-down − lead_ms`, so the longest lead goes
//! first: the antenna relay throws, then the amplifier is keyed. Every channel
//! is released at `key-up + hold_ms`, so the shortest hold drops first: the
//! amplifier unkeys, then the antenna comes back. That is what a sequencer
//! does, expressed as one number per channel per direction instead of a
//! priority list nobody can reason about.
//!
//! # The one thing software cannot do
//!
//! When a *separate* transceiver keys itself — its own microphone button, foot
//! switch, VOX or keyer — sdroxide does not know until it next asks over CAT,
//! which is typically 300–600 ms into the over. The relay throws then, and not
//! before. See [`SenseConfig`] for the wire that fixes it, and the manual for
//! why a genuinely valuable front end wants an RF-sensed hardware switch
//! regardless.

use serde::{Deserialize, Serialize};

use crate::SerialConfig;

/// The largest channel number a board may be given. Eight covers every relay
/// board an amateur station has a use for — the dcttech family tops out there —
/// and a `u32` mask keeps the driver's state one word.
pub const MAX_CHANNEL: u8 = 32;

/// The default lead: long enough for a small coax relay to have thrown (5–15 ms
/// is the usual specification), short enough to disappear into the gap the
/// receive path is about to leave anyway.
pub const DEFAULT_LEAD_MS: u16 = 10;

/// The default hold. Longer than the lead on purpose: letting the antenna back
/// in a moment late costs nothing, and letting it back in early costs a front
/// end. A rig's own carrier decays for a few milliseconds after it is told to
/// stop, and an amplifier's for longer.
pub const DEFAULT_HOLD_MS: u16 = 20;

/// How the contact is produced.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelayLink {
    /// No external switch. The ordinary case.
    #[default]
    Off,
    /// A USB relay board that speaks over a serial port: LCUS (CH340),
    /// KMtronic, Numato. Which one is [`RelayConfig::family`].
    Serial,
    /// The handshake lines of any USB-serial adapter — RTS and DTR — driving an
    /// opto-isolator or a small relay board. What a DigiRig, a homebrew
    /// interface, and every hardware sequencer with a PTT input wants.
    SerialLines,
    /// A USB HID relay board: the dcttech / "free-driver USB control switch"
    /// family sold under a dozen names, 1 to 8 channels.
    Hid,
    /// The GPIO pins of a C-Media CM108/CM119 sound card — the most common ham
    /// PTT interface there is (DRA boards, RB-USB RIM, AIOC). Pin 3 by
    /// convention, because it is the one at the end of the package that a wire
    /// can be tacked to.
    Cm108,
    /// A Linux GPIO line: the Raspberry Pi header, or any board with a
    /// `/dev/gpiochip*`.
    Gpio,
    /// Run a program on key-down and another on key-up. The catch-all for
    /// everything with a command-line tool and no protocol worth embedding —
    /// Denkovi's FT245 boards, microHAM, `usbrelay`, a shell script.
    Command,
}

impl RelayLink {
    pub fn label(self) -> &'static str {
        match self {
            RelayLink::Off => "None",
            RelayLink::Serial => "USB relay board (serial)",
            RelayLink::SerialLines => "Serial RTS/DTR line",
            RelayLink::Hid => "USB relay board (HID)",
            RelayLink::Cm108 => "CM108/CM119 sound-card GPIO",
            RelayLink::Gpio => "Linux GPIO line",
            RelayLink::Command => "External command",
        }
    }

    /// Whether this link is configured by picking a serial port.
    pub fn uses_serial_port(self) -> bool {
        matches!(self, RelayLink::Serial | RelayLink::SerialLines)
    }

    /// Whether this link is configured by picking a device out of a list the
    /// engine's machine has to enumerate.
    pub fn uses_device_list(self) -> bool {
        matches!(self, RelayLink::Hid | RelayLink::Cm108)
    }

    /// Whether a contact's *number* means anything.
    ///
    /// On a relay board it is the silkscreen; on a GPIO chip it selects which
    /// of [`RelayConfig::gpio_lines`] the contact drives. It means nothing for
    /// RTS/DTR (two fixed lines), for a sound card (one brought-out pin) or for
    /// the command hook (whatever the operator's script decides).
    pub fn has_numbered_channels(self) -> bool {
        matches!(self, RelayLink::Serial | RelayLink::Hid | RelayLink::Gpio)
    }

    /// Whether each contact switches on its own. A sound card's one pin and
    /// the command hook's key-down/key-up pair are a single on/off whatever
    /// the contact table says — anything asserted is "transmit" — so a band
    /// decoder, whose receive word is asserted while receiving, would key
    /// them for as long as the dial sat on a band with a row.
    pub fn switches_contacts_separately(self) -> bool {
        !matches!(self, RelayLink::Cm108 | RelayLink::Command)
    }
}

/// Which serial relay board. They differ only in what a "close channel 2" looks
/// like on the wire, which is why one transport covers all three.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelayFamily {
    /// LCUS-1/2/4/8 and the many boards that copy them: a CH340 in front of a
    /// microcontroller, 9600 8N1, four bytes with a checksum. The cheapest
    /// board on the shelf and the most widely sold.
    #[default]
    Lcus,
    /// KMtronic's USB relay controllers: 9600 8N1, three bytes, and unlike the
    /// LCUS it will tell you what state it is in.
    KMtronic,
    /// Numato Lab's USB relay modules: a CDC ACM port speaking ASCII, so the
    /// baud rate is not real. Some models have digital inputs, which is what
    /// makes them worth the extra money for this job — see [`SenseConfig`].
    Numato,
}

impl RelayFamily {
    pub fn label(self) -> &'static str {
        match self {
            RelayFamily::Lcus => "LCUS / CH340",
            RelayFamily::KMtronic => "KMtronic",
            RelayFamily::Numato => "Numato Lab",
        }
    }

    /// What to open the port at. Numato's is a CDC ACM port and ignores this
    /// entirely; the other two are a real UART behind a bridge and do not.
    pub fn baud(self) -> u32 {
        match self {
            RelayFamily::Lcus | RelayFamily::KMtronic => 9600,
            RelayFamily::Numato => 19200,
        }
    }

    /// Whether the board can be asked what its relays are actually set to.
    /// Useful, but never load-bearing: a board that cannot answer is not a
    /// board that is failing.
    pub fn reads_back(self) -> bool {
        matches!(self, RelayFamily::KMtronic | RelayFamily::Numato)
    }
}

/// What a channel is wired to. Naming the job rather than the number is what
/// lets the timings default sensibly and the status line say something an
/// operator can act on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelayRole {
    /// Wired but not used yet, or used for something this program should not
    /// touch. Never driven.
    #[default]
    Unused,
    /// The reason this feature exists: disconnect and ground the SDR's antenna
    /// input. Wants the longest lead and the longest hold of anything here.
    SdrAntenna,
    /// An amplifier's key line, or an outboard T/R relay. Wants to be the last
    /// thing keyed and the first thing dropped.
    Amplifier,
    /// Anything else the operator wants switched with the over — a preamp
    /// bypass, a receive-antenna relay, a rotator inhibit.
    Aux,
    /// Part of a band-decoder output bank: which word from
    /// [`RelayConfig::band_table`] it follows is decided by the band, and
    /// whether that is the row's RX or TX word by the same on-air ramp every
    /// other role runs on — so a contact whose two words differ moves inside
    /// its own lead, before RF, and back only after its hold.
    ///
    /// The same idea as [`crate::HpsdrOcRow`]'s per-band output word,
    /// generalised from HPSDR's fixed seven-pin OC bus to whatever relay
    /// hardware this bank actually is — a USB relay board, GPIO header, HID
    /// relay or external command driving an outboard band-pass/low-pass
    /// filter bank or a transverter selector (issue #442).
    BandDecoder,
}

impl RelayRole {
    pub fn label(self) -> &'static str {
        match self {
            RelayRole::Unused => "Not used",
            RelayRole::SdrAntenna => "Ground the SDR antenna",
            RelayRole::Amplifier => "Key an amplifier / T-R relay",
            RelayRole::Aux => "Auxiliary",
            RelayRole::BandDecoder => "Band decoder (filter / transverter)",
        }
    }

    /// The lead and hold a freshly-chosen role starts with. Not a constraint —
    /// the operator can set any number — but a set of numbers that sequence
    /// correctly without anyone having to work out why.
    pub fn default_timing(self) -> (u16, u16) {
        match self {
            // First in, last out.
            RelayRole::SdrAntenna => (DEFAULT_LEAD_MS, DEFAULT_HOLD_MS),
            // Last in, first out: the amplifier must never be keyed into an
            // antenna relay that is still moving, and must be off the air
            // before that relay comes back. Zero hold is a legitimate answer
            // here and is why these numbers are literal rather than a
            // sentinel — see [`RelayChannel::lead_ms`].
            RelayRole::Amplifier => (DEFAULT_LEAD_MS / 2, 0),
            RelayRole::Aux | RelayRole::Unused => (DEFAULT_LEAD_MS, DEFAULT_HOLD_MS),
            // The band picks the word, but the move from a row's RX word to
            // its TX word is an on-air edge like any other — a transmit-only
            // LPF bank switched in, a receive preamp bypassed — and must be
            // done before RF and undone only after it. A split across bands
            // makes that true of any band-decoder contact, not only one whose
            // row gives it different bits, so every one gets a real ramp.
            RelayRole::BandDecoder => (DEFAULT_LEAD_MS, DEFAULT_HOLD_MS),
        }
    }
}

/// What to do when the hardware will not answer at key-down.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailSafe {
    /// Refuse the over, the way the SWR guard does. The right default: a
    /// switch that exists to protect a receiver is not worth having if it
    /// silently stops protecting it.
    #[default]
    RefuseTx,
    /// Transmit anyway and say so. For an operator running a few watts into a
    /// preamp bypass, where a loose USB cable ending their contest is the worse
    /// outcome.
    WarnOnly,
}

impl FailSafe {
    pub fn label(self) -> &'static str {
        match self {
            FailSafe::RefuseTx => "Refuse to transmit",
            FailSafe::WarnOnly => "Transmit anyway, warn",
        }
    }
}

/// One switched contact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RelayChannel {
    /// Which contact on the board, numbered as the silkscreen numbers it —
    /// from 1. The boards disagree about this among themselves (Numato counts
    /// from zero on the wire) and the drivers convert; nothing above them has
    /// to know.
    pub index: u8,
    pub role: RelayRole,
    /// What the operator called it. Shown in the status line and the test
    /// button, so "IC-7300 antenna" beats "channel 1".
    pub label: String,
    /// Whether *transmit* is the energised state.
    ///
    /// This is the only fail-safe that survives the program dying, the USB
    /// cable being pulled, or the computer being off — so it is a wiring
    /// decision before it is a setting. Choose it so the de-energised contact
    /// is the state you want when nothing is running. See the manual.
    pub active_high: bool,
    /// Asserted this long before RF.
    ///
    /// Literal, with no sentinel: zero means zero. An amplifier that should
    /// drop the instant the RF stops has a hold of zero and must be able to say
    /// so — which a "zero means inherit" rule would quietly turn into the
    /// longest hold on the station.
    pub lead_ms: u16,
    /// Released this long after RF stops. Literal, as [`Self::lead_ms`] is.
    pub hold_ms: u16,
}

impl Default for RelayChannel {
    fn default() -> Self {
        RelayChannel {
            index: 1,
            role: RelayRole::Unused,
            label: String::new(),
            active_high: true,
            lead_ms: DEFAULT_LEAD_MS,
            hold_ms: DEFAULT_HOLD_MS,
        }
    }
}

impl RelayChannel {
    /// A name for logs and the status line, never empty.
    pub fn name(&self) -> String {
        let l = self.label.trim();
        if l.is_empty() { format!("channel {}", self.index) } else { l.to_string() }
    }
}

/// Which line to watch for a transmitter keying itself.
///
/// The problem it solves: sdroxide learns about an over the operator started at
/// the radio by *asking over CAT*, and that question rides the meter poll, so
/// the answer lands a few hundred milliseconds in. A relay that throws then has
/// already let the first part of the over into the receiver.
///
/// Wire the transceiver's SEND / PTT / accessory key line into a handshake
/// input on the same adapter that drives the relay — through an opto-isolator,
/// not directly — and the edge is seen in a few milliseconds instead. The same
/// edge is handed to the engine, so the meter, the transmit interlock and the
/// relay all stop being late together.
///
/// It does not make the switching instantaneous: the relay still has to throw.
/// Nothing driven from a computer can. It moves the delay from "most of a
/// syllable" to "about what the relay itself costs".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SenseLine {
    /// Not wired. The CAT poll remains the only source, with its latency.
    #[default]
    Off,
    Cts,
    Dsr,
    Dcd,
}

impl SenseLine {
    pub fn label(self) -> &'static str {
        match self {
            SenseLine::Off => "Not wired",
            SenseLine::Cts => "CTS",
            SenseLine::Dsr => "DSR",
            SenseLine::Dcd => "DCD (carrier detect)",
        }
    }
}

/// The transmit-sense input. See [`SenseLine`] for why it is worth wiring.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SenseConfig {
    pub line: SenseLine,
    /// Whether a *high* line means transmitting. Most opto-isolated interfaces
    /// pull the line down when the rig keys, so this is often false.
    pub active_high: bool,
    /// Retained for compatibility with configurations saved when one switch
    /// was shared by every radio; ignored now. The switch belongs to one
    /// radio, so the sensed transmitter is always that radio's.
    pub radio: u32,
}

/// One row of a band-decoder output table — the same idea as
/// [`crate::HpsdrOcRow`], generalised from HPSDR's fixed seven-pin OC bus to
/// whatever [`RelayChannel`]s this bank has given [`RelayRole::BandDecoder`].
///
/// `rx_mask`/`tx_mask` read the same way the HPSDR OC table's do: bit 0 is
/// channel 1, bit `N` is channel `N + 1` (matching `RelayChannel::index`,
/// 1-based). A bit set in `rx_mask` means that channel is asserted while
/// receiving on this band; `tx_mask` while transmitting. Give a channel the
/// same bit in both for a filter that does not care which way the RF is
/// going, and different bits for a receive preamp bypass or a transmit-only
/// LPF bank that must not be in circuit on receive.
///
/// The move between the two is sequenced exactly like any other contact's:
/// the channel's own lead before RF, its own hold after — the relay worker
/// makes it, from the owning radio's on-air state, never the engine. The RX
/// word is the owning radio's receive band; the TX word its transmit band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayBandRow {
    pub band: crate::Band,
    pub rx_mask: u32,
    pub tx_mask: u32,
}

/// Everything one radio's T/R switch needs to know. Persisted as `relay.json`
/// in the radio's own configuration scope and announced in
/// [`crate::StationConfig`], because it is a fact about the machine the antenna
/// is attached to and a remote client has no way to guess it. Each radio has
/// its own, and a relay module may be assigned to only one radio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RelayConfig {
    pub link: RelayLink,
    /// Which serial relay board, when [`RelayLink::Serial`].
    pub family: RelayFamily,
    /// The port, when the link uses one. The same type the CAT panel edits, so
    /// the port picker is the identical widget and the forced-line settings
    /// mean the same thing.
    pub serial: SerialConfig,
    /// The device, when the link picks one out of a list: a HID relay's key, or
    /// a `/dev/gpiochip0`.
    pub device: String,
    /// Which GPIO offsets to drive, for [`RelayLink::Gpio`] — one per channel,
    /// in channel order. Ignored by every other link.
    pub gpio_lines: Vec<u32>,
    /// Which sound-card GPIO pin, for [`RelayLink::Cm108`]. Three by
    /// convention; the boards that break with it are rare enough to be worth a
    /// setting rather than a fork.
    pub cm108_pin: u8,
    pub channels: Vec<RelayChannel>,
    pub fail_safe: FailSafe,
    /// What to run on key-down and key-up for [`RelayLink::Command`]. Split on
    /// whitespace; the first word is the program.
    pub tx_cmd: String,
    pub rx_cmd: String,
    pub sense: SenseConfig,
    /// Per-band output state for this bank's [`RelayRole::BandDecoder`]
    /// channels — see [`RelayBandRow`]. Empty means none of this bank's
    /// contacts switch by band; the existing on-air-driven channels are
    /// unaffected either way, and the two kinds of channel live on the same
    /// hardware together exactly as HPSDR's OC byte already combines a band
    /// decoder with everything else on the bus.
    #[serde(default)]
    pub band_table: Vec<RelayBandRow>,
}

impl Default for RelayConfig {
    fn default() -> Self {
        RelayConfig {
            link: RelayLink::Off,
            family: RelayFamily::Lcus,
            serial: SerialConfig { baud: 9600, ..SerialConfig::default() },
            device: String::new(),
            gpio_lines: Vec::new(),
            cm108_pin: 3,
            // One channel doing the job this feature is named for, so a fresh
            // configuration needs a port and a polarity and nothing else.
            channels: vec![RelayChannel {
                index: 1,
                role: RelayRole::SdrAntenna,
                label: String::new(),
                active_high: true,
                lead_ms: DEFAULT_LEAD_MS,
                hold_ms: DEFAULT_HOLD_MS,
            }],
            fail_safe: FailSafe::default(),
            tx_cmd: String::new(),
            rx_cmd: String::new(),
            sense: SenseConfig::default(),
            band_table: Vec::new(),
        }
    }
}

impl RelayConfig {
    /// Whether anything should be opened at all.
    pub fn enabled(&self) -> bool {
        self.link != RelayLink::Off
    }

    /// The channels that actually get driven. A channel with no role is one the
    /// operator has told us to leave alone.
    pub fn active_channels(&self) -> impl Iterator<Item = &RelayChannel> {
        self.channels.iter().filter(|c| c.role != RelayRole::Unused)
    }

    /// The [`RelayRole::BandDecoder`] channels that should be asserted right
    /// now, given the dial's band and whether the station is transmitting —
    /// see [`RelayBandRow`].
    ///
    /// Restricted to `BandDecoder`-role channels by construction: a row's
    /// mask is intersected with the *actual* band-decoder channels on this
    /// bank, so a stale or mis-edited table bit can never assert an
    /// `Amplifier` or `SdrAntenna` channel it happens to share a bit position
    /// with. An unconfigured band (no row, or a band the table has no entry
    /// for — `Band::Gen` covers everywhere outside the amateur allocations,
    /// the way it does for [`crate::HpsdrOcPlan`]) asserts nothing.
    pub fn band_mask(&self, band: crate::Band, tx: bool) -> u32 {
        let decoder: u32 = self
            .channels
            .iter()
            .filter(|c| c.role == RelayRole::BandDecoder)
            .fold(0u32, |m, c| m | (1u32 << (c.index.clamp(1, 32) - 1)));
        let row = self
            .band_table
            .iter()
            .find(|r| r.band == band)
            .map(|r| if tx { r.tx_mask } else { r.rx_mask })
            .unwrap_or(0);
        row & decoder
    }

    /// This channel's lead. A method rather than a field access because every
    /// call site reads better for saying whose lead it is, and because this is
    /// the one place a future rule about them would go.
    pub fn lead_for(&self, ch: &RelayChannel) -> u16 {
        ch.lead_ms
    }

    /// This channel's hold.
    pub fn hold_for(&self, ch: &RelayChannel) -> u16 {
        ch.hold_ms
    }

    /// The longest lead any driven channel asks for — what the engine has to
    /// wait before it may let RF out. Zero when nothing is driven.
    ///
    /// Deliberately the maximum and not the sum: the channels are asserted
    /// against one deadline, staggered by their own leads, not one after
    /// another.
    pub fn max_lead_ms(&self) -> u16 {
        self.active_channels().map(|c| self.lead_for(c)).max().unwrap_or(0)
    }

    /// The longest hold any driven channel asks for.
    pub fn max_hold_ms(&self) -> u16 {
        self.active_channels().map(|c| self.hold_for(c)).max().unwrap_or(0)
    }

    /// Why this configuration cannot be opened, if it cannot. Checked before
    /// anything is opened so the operator gets the reason in the settings
    /// panel rather than a silent nothing at the next key-down.
    pub fn refusal(&self) -> Option<String> {
        if !self.enabled() {
            return None;
        }
        if self.link.uses_serial_port() && self.serial.path.trim().is_empty() {
            return Some("no serial port chosen for the T/R switch".to_string());
        }
        if self.link.uses_device_list() && self.device.trim().is_empty() {
            return Some("no device chosen for the T/R switch".to_string());
        }
        if self.link == RelayLink::Gpio && self.device.trim().is_empty() {
            return Some("no GPIO chip chosen for the T/R switch".to_string());
        }
        if self.link == RelayLink::Command && self.tx_cmd.trim().is_empty() {
            return Some("no transmit command set for the T/R switch".to_string());
        }
        if self.active_channels().next().is_none() {
            return Some("no T/R switch channel has been given a job".to_string());
        }
        if !self.link.switches_contacts_separately()
            && self.channels.iter().any(|c| c.role == RelayRole::BandDecoder)
        {
            return Some(format!(
                "a band decoder needs contacts that switch one at a time, and the \"{}\" \
                 link is a single on/off",
                self.link.label()
            ));
        }
        if let Some(c) = self.channels.iter().find(|c| c.index == 0 || c.index > MAX_CHANNEL) {
            return Some(format!(
                "T/R switch channel number {} is out of range (1–{MAX_CHANNEL})",
                c.index
            ));
        }
        // The driver keeps one bit per contact number, so a band decoder
        // sharing its number with the antenna relay or the amplifier line
        // would put that contact under the band table: its receive word would
        // key the amplifier while receiving, and the over would no longer
        // close it.
        if let Some(c) = self.active_channels().find(|c| {
            c.role == RelayRole::BandDecoder
                && self
                    .active_channels()
                    .any(|o| o.index == c.index && o.role != RelayRole::BandDecoder)
        }) {
            return Some(format!(
                "T/R switch contact {} is a band decoder and has another job as well",
                c.index
            ));
        }
        if self.link == RelayLink::Gpio {
            // Against the highest contact *number*, not the count: the GPIO
            // list is indexed by number, so a table using contacts 1 and 5
            // needs five entries however few of them are driven.
            let highest = self.active_channels().map(|c| usize::from(c.index)).max().unwrap_or(0);
            if self.gpio_lines.len() < highest {
                return Some(format!(
                    "the T/R switch drives contact {highest} but only {} GPIO line(s) are listed",
                    self.gpio_lines.len()
                ));
            }
        }
        None
    }

    /// One line for the settings panel saying what will happen at key-down, in
    /// the order it will happen. Worth showing because the ordering is derived
    /// from the timings rather than stated, and an operator who has just typed
    /// the numbers should be able to see what they bought.
    pub fn sequence_note(&self) -> String {
        let mut chans: Vec<&RelayChannel> = self.active_channels().collect();
        if chans.is_empty() {
            return "Nothing is switched.".to_string();
        }
        chans.sort_by_key(|c| std::cmp::Reverse(self.lead_for(c)));
        let on: Vec<String> =
            chans.iter().map(|c| format!("{} at −{} ms", c.name(), self.lead_for(c))).collect();
        chans.sort_by_key(|c| self.hold_for(c));
        let off: Vec<String> =
            chans.iter().map(|c| format!("{} at +{} ms", c.name(), self.hold_for(c))).collect();
        format!("Key-down: {}. Key-up: {}.", on.join(", then "), off.join(", then "))
    }
}

/// A switching device the engine's machine can see, for the settings panel's
/// picker. Answered by [`crate::DeviceProbe::Relays`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayDevice {
    /// What to put in [`RelayConfig::device`] to open this one. Opaque: a
    /// hidraw path on Linux, an interface path on Windows, a registry entry id
    /// on macOS.
    pub key: String,
    /// What to show the operator.
    pub label: String,
    /// Which link this device is for, so the picker can offer HID relays and
    /// CM108 cards in the right place.
    pub link: RelayLink,
    /// How many contacts, where the device says. Zero when it will not.
    pub channels: u8,
}

/// What the T/R switch is doing, for the settings panel and the status line.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayStatus {
    /// Whether a switch is configured at all. False means every other field is
    /// meaningless, not that something is wrong.
    pub configured: bool,
    /// Whether the hardware is answering.
    pub present: bool,
    /// The link, as the driver describes it: the port or device it opened.
    pub describe: String,
    /// A standing condition worth showing. `None` when all is well.
    pub error: Option<String>,
    /// Whether the contacts are in their transmit state right now.
    pub keyed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ordering claim in the module docs, checked rather than asserted.
    #[test]
    fn the_longest_lead_goes_first_and_the_longest_hold_comes_back_last() {
        let cfg = RelayConfig {
            link: RelayLink::Serial,
            channels: vec![
                RelayChannel {
                    index: 1,
                    role: RelayRole::SdrAntenna,
                    label: "SDR".into(),
                    lead_ms: 25,
                    hold_ms: 40,
                    ..RelayChannel::default()
                },
                RelayChannel {
                    index: 2,
                    role: RelayRole::Amplifier,
                    label: "PA".into(),
                    lead_ms: 5,
                    hold_ms: 5,
                    ..RelayChannel::default()
                },
            ],
            ..RelayConfig::default()
        };
        assert_eq!(cfg.max_lead_ms(), 25, "the engine waits for the slowest channel");
        assert_eq!(cfg.max_hold_ms(), 40);
        let note = cfg.sequence_note();
        let sdr_on = note.find("SDR at −25").expect("{note}");
        let pa_on = note.find("PA at −5").expect("{note}");
        assert!(sdr_on < pa_on, "the antenna must throw before the amplifier is keyed: {note}");
        let pa_off = note.find("PA at +5").expect("{note}");
        let sdr_off = note.find("SDR at +40").expect("{note}");
        assert!(pa_off < sdr_off, "the amplifier must unkey before the antenna returns: {note}");
    }

    fn band_decoder_cfg() -> RelayConfig {
        RelayConfig {
            link: RelayLink::Gpio,
            channels: vec![
                RelayChannel { index: 1, role: RelayRole::BandDecoder, ..RelayChannel::default() },
                RelayChannel { index: 2, role: RelayRole::BandDecoder, ..RelayChannel::default() },
                // An ordinary on-air-driven channel sharing the same bank —
                // the two kinds of channel must not interfere.
                RelayChannel { index: 3, role: RelayRole::Amplifier, ..RelayChannel::default() },
            ],
            band_table: vec![
                RelayBandRow { band: crate::Band::M20, rx_mask: 0b01, tx_mask: 0b01 },
                RelayBandRow { band: crate::Band::M40, rx_mask: 0b10, tx_mask: 0b11 },
            ],
            ..RelayConfig::default()
        }
    }

    #[test]
    fn a_configured_band_asserts_its_row() {
        let cfg = band_decoder_cfg();
        assert_eq!(cfg.band_mask(crate::Band::M20, false), 0b01);
        assert_eq!(cfg.band_mask(crate::Band::M20, true), 0b01);
        assert_eq!(cfg.band_mask(crate::Band::M40, false), 0b10);
        assert_eq!(cfg.band_mask(crate::Band::M40, true), 0b11);
    }

    #[test]
    fn a_band_with_no_row_asserts_nothing() {
        let cfg = band_decoder_cfg();
        assert_eq!(cfg.band_mask(crate::Band::M15, false), 0);
        assert_eq!(cfg.band_mask(crate::Band::M15, true), 0);
    }

    /// The safety property the whole design rests on: a band row can never
    /// assert a channel that is not actually a band-decoder channel, however
    /// its bits happen to line up.
    #[test]
    fn a_band_row_cannot_assert_a_non_decoder_channel() {
        let mut cfg = band_decoder_cfg();
        // Channel 3 (the amplifier) is bit index 2 — put it in a row's mask
        // as if by a copy-paste mistake in the table.
        cfg.band_table.push(RelayBandRow { band: crate::Band::M15, rx_mask: 0b100, tx_mask: 0 });
        assert_eq!(cfg.band_mask(crate::Band::M15, false), 0, "the amplifier bit leaked through");
    }

    #[test]
    fn an_empty_band_table_asserts_nothing_on_any_band() {
        let cfg = RelayConfig {
            channels: vec![RelayChannel {
                index: 1,
                role: RelayRole::BandDecoder,
                ..RelayChannel::default()
            }],
            ..RelayConfig::default()
        };
        assert_eq!(cfg.band_mask(crate::Band::M20, false), 0);
        assert_eq!(cfg.band_mask(crate::Band::M20, true), 0);
    }

    /// A CM108 pin or a command hook turns any asserted contact into
    /// "transmit", so a band row's receive word would key the PTT line — or
    /// run the transmit command — for as long as the dial sat on that band.
    #[test]
    fn a_band_decoder_on_a_single_on_off_link_is_refused() {
        for link in [RelayLink::Cm108, RelayLink::Command] {
            let cfg = RelayConfig {
                link,
                device: "card".into(),
                tx_cmd: "key".into(),
                ..band_decoder_cfg()
            };
            let why = cfg.refusal();
            assert!(
                why.as_deref().is_some_and(|w| w.contains("band decoder")),
                "{link:?} took a band decoder: {why:?}"
            );
        }
        // Each RTS/DTR line switches on its own, so two outputs work there.
        let lines = RelayConfig {
            link: RelayLink::SerialLines,
            serial: SerialConfig { path: "/dev/ttyUSB0".into(), ..SerialConfig::default() },
            ..band_decoder_cfg()
        };
        assert_eq!(lines.refusal(), None);
    }

    /// `band_mask` clips a row to the band-decoder contacts by number, so a
    /// number that is also the amplifier's would let the row through to the
    /// amplifier line — keyed on receive whenever the band's RX word has it.
    #[test]
    fn a_band_decoder_cannot_share_its_contact_with_another_job() {
        let mut cfg = RelayConfig {
            link: RelayLink::Serial,
            serial: SerialConfig { path: "/dev/ttyUSB0".into(), ..SerialConfig::default() },
            ..band_decoder_cfg()
        };
        assert_eq!(cfg.refusal(), None);
        // The amplifier renumbered onto the first filter's contact.
        cfg.channels[2].index = 1;
        let why = cfg.refusal();
        assert!(
            why.as_deref().is_some_and(|w| w.contains("contact 1")),
            "a shared contact was accepted: {why:?}"
        );
    }

    #[test]
    fn a_channel_with_no_job_is_never_driven() {
        let mut cfg = RelayConfig { link: RelayLink::Serial, ..RelayConfig::default() };
        cfg.channels[0].role = RelayRole::Unused;
        assert_eq!(cfg.active_channels().count(), 0);
        assert_eq!(cfg.max_lead_ms(), 0);
        assert!(
            cfg.refusal().is_some(),
            "and the configuration says why rather than doing nothing"
        );
    }

    /// The trap this design walked into once: an amplifier wants to drop the
    /// instant the RF stops, and under a "zero means inherit the default" rule
    /// it could not say so — a typed 0 became the longest hold on the station,
    /// holding the amplifier keyed past the antenna relay it was sequenced
    /// against.
    #[test]
    fn a_zero_hold_means_zero_and_not_the_default() {
        let cfg = RelayConfig {
            link: RelayLink::Serial,
            channels: vec![RelayChannel {
                index: 1,
                role: RelayRole::Amplifier,
                hold_ms: 0,
                lead_ms: 5,
                ..RelayChannel::default()
            }],
            ..RelayConfig::default()
        };
        assert_eq!(cfg.hold_for(&cfg.channels[0]), 0);
        assert_eq!(cfg.max_hold_ms(), 0);
        // And the shipped amplifier timing is that zero, not a sentinel.
        assert_eq!(RelayRole::Amplifier.default_timing(), (DEFAULT_LEAD_MS / 2, 0));
    }

    #[test]
    fn a_fresh_contact_starts_with_usable_timings() {
        let cfg = RelayConfig::default();
        let ch = &cfg.channels[0];
        assert_eq!(cfg.lead_for(ch), DEFAULT_LEAD_MS);
        assert_eq!(cfg.hold_for(ch), DEFAULT_HOLD_MS);
    }

    #[test]
    fn a_switch_that_is_off_is_not_a_switch_that_is_broken() {
        assert_eq!(RelayConfig::default().refusal(), None);
    }
}
