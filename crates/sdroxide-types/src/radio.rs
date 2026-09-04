//! Persisted radio-backend configuration (`radio.json`): choose between a
//! SoapySDR device and a CAT-controlled rig whose audio arrives over a USB
//! sound card. Serde-only — no I/O, safe in the wasm client (the settings UI
//! is shared, even though the CAT machinery is native-only).

use serde::{Deserialize, Serialize};

use crate::Band;
use crate::limerfe::LimeRfeConfig;

/// Which radio backend to drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Backend {
    /// Legacy "SoapySDR if present, else CAT" auto-detect. No longer offered in
    /// the UI, but kept so older `radio.json` files still deserialize.
    Auto,
    #[default]
    Soapy,
    Cat,
    /// OpenHPSDR ethernet SDR (Protocol 2), discovered/reached over the LAN.
    Hpsdr,
    /// TCI (Transceiver Control Interface) over WebSocket — ExpertSDR3, Thetis, …
    Tci,
    /// RTL2832U dongle driven directly over USB by the native driver — no
    /// SoapySDR, no libusb, nothing to install.
    RtlSdr,
    /// RX-888 Mk2 direct-sampling HF receiver, driven over USB by the native
    /// driver. Uploads its own firmware, so nothing needs installing.
    Rx888,
    /// SmartSDR (FlexRadio FLEX-6000 / FLEX-8000) over the LAN. Receive is a DAX
    /// IQ stream, transmit is DAX audio the radio modulates.
    ///
    /// Appended last on purpose: this enum is serde-serialised into `radio.json`
    /// by variant name, but `ALL` fixes the order the UI offers.
    SmartSdr,
    /// ADALM-Pluto (AD9361/AD9363) over the IIOD protocol — reached over the
    /// network, which the USB cable provides as an Ethernet gadget. Appended
    /// last, for the same reason as `SmartSdr` above.
    Pluto,
    /// SDRplay RSP family (RSP1/1A/1B/2/duo/dx), driven through the vendor's
    /// `sdrplay_api` service — the one RSP protocol there is; no open USB
    /// protocol exists for anything after the original RSP1. Appended last,
    /// for the same reason as `SmartSdr` above.
    SdrPlay,
    /// No interface chosen yet. The seeded state of a freshly created radio
    /// tab: it must open *nothing* until the operator picks a device, because
    /// the defaults above would grab the first device found — which is
    /// whatever the station's first radio is already running. Appended last,
    /// for the same reason as `SmartSdr` above; not offered in the picker
    /// (`ALL`), only ever written by the multi-radio seeding.
    None,
    /// Airspy HF+ (Dual / Discovery / Ranger), driven directly over USB by the
    /// native pure-Rust driver — no libairspyhf, no SoapySDR. Appended last,
    /// for the same reason as `SmartSdr` above.
    AirspyHf,
    /// An Icom over its LAN or WiFi port, speaking the IP-remote protocol
    /// RS-BA1 uses: IC-7300MK2, IC-705, IC-7610, IC-7760, IC-7851, IC-9700,
    /// IC-905, IC-R8600.
    /// Control, audio and the radio's own spectrum scope all arrive over the
    /// network; there is no I/Q, because no Icom offers any. Appended last,
    /// for the same reason as `SmartSdr` above.
    IcomNet,
    /// An RTL-SDR dongle plugged into another machine and published with
    /// `rtl_tcp` — the same hardware as [`Backend::RtlSdr`], reached over the
    /// network instead of over USB. Kept a separate interface rather than a
    /// mode of the USB one because what the operator picks is *where the
    /// radio is*, and because the two configurations have nothing in common
    /// but the tuner: a serial number identifies nothing on the far end, and
    /// an address identifies nothing locally. Appended last, for the same
    /// reason as `SmartSdr` above.
    RtlTcp,
    /// Airspy R2 or Mini, driven directly over USB by the native pure-Rust
    /// driver — no libairspy, no SoapySDR. A **different radio** from the
    /// Airspy HF+ above: different silicon, different USB id, different
    /// protocol. Appended last, for the same reason as `SmartSdr` above.
    Airspy,
    /// HackRF One or HackRF Pro (or a Jawbreaker / rad1o), driven directly over
    /// USB by the native pure-Rust driver — no libhackrf, no SoapySDR. The only
    /// native USB backend here that transmits, and half duplex: receive stops
    /// for the length of an over. Appended last, for the same reason as
    /// `SmartSdr` above.
    HackRf,
    /// A receiver published by a SpyServer — Airspy's own server and the
    /// several re-implementations that speak its protocol — delivering
    /// wideband I/Q the way every other SDR here does. The far end owns the
    /// hardware; this end asks it for a decimation stage and receives that
    /// slice of its ADC. Appended last, for the same reason as `SmartSdr`
    /// above.
    SpyServer,
    /// The same servers, in their low-bandwidth mode: a *narrow* I/Q stream
    /// that follows the dial, plus a separate low-rate FFT of the whole band
    /// for the full-band strip.
    ///
    /// A separate interface rather than a mode of [`Backend::SpyServer`]
    /// because what it delivers is a different shape — a receiver whose
    /// panadapter is a few tens of kHz wide, and whose band view is a picture
    /// the server drew rather than anything this end can demodulate — and
    /// because the two offer different sample-rate ladders. It is the
    /// interface for a link that cannot carry megabits: WiFi at the far end of
    /// a property, or a cellular modem. Appended last, for the same reason as
    /// `SmartSdr` above.
    SpyServerVfo,
    /// An ELAD FDM-DUO, FDM-S2 or FDM-S1, driven directly over USB by the
    /// native pure-Rust driver — no libusb, no gr-elad, no SoapySDR module.
    ///
    /// The FDM-DUO is three USB devices in one radio and this interface drives
    /// all three at once: wideband I/Q from ELAD's own vendor interface, rig
    /// control over the CAT serial port ([`CatFamily::Elad`], configured in the
    /// same place every other CAT rig is), and transmit audio out through the
    /// radio's USB sound card. The S1 and S2 have only the first of the three
    /// and come up receive-only. Appended last, for the same reason as
    /// `SmartSdr` above.
    Elad,
    /// The LimeSDR family (LimeSDR-USB, LimeSDR Mini v1/v2, LimeNET-Micro,
    /// LimeSDR-PCIe) driven through `libLimeSuite` — full-duplex wideband I/Q
    /// both ways, plus LimeRFE front-end control that no other path offers.
    ///
    /// The one backend here that is neither pure Rust nor SoapySDR. LimeSuite
    /// is found with dlopen at *runtime*, exactly as the SDRplay backend finds
    /// `sdrplay_api`, so nothing is linked at build time and this ships in
    /// every build variant; on a machine without it the device list is empty
    /// and opening explains what to install. Unlike the SDRplay case the
    /// library is open source (Apache-2.0), so this is a shortcut past ~10k
    /// lines of LMS7002M register, PLL and calibration work rather than the
    /// only door there is.
    ///
    /// A LimeSDR reaches sdroxide through SoapySDR too, and always has. What
    /// this interface adds is the **LimeRFE**: SoapyLMS7 exposes none of it, so
    /// band filters, the LNA, the PA and the transmit/receive relay are
    /// unreachable from that side. Appended last, for the same reason as
    /// `SmartSdr` above.
    Lime,
    /// HydraSDR RFOne, driven directly over USB by the native pure-Rust driver
    /// — no libhydrasdr, no SoapySDR.
    ///
    /// A *fork* of the Airspy R2 rather than a relative: vendor requests 0–26
    /// line up number for number, the gain curves are byte-for-byte the same,
    /// and libhydrasdr still carries libairspy's copyright header. Its own
    /// interface all the same, because the two cannot drive each other's
    /// hardware — `SET_FREQ` is eight bytes wide here and four there, this
    /// radio has three RF sockets and seven sample rates, and a prototype board
    /// shares the Airspy's USB id. Appended last, for the same reason as
    /// `SmartSdr` above.
    HydraSdr,
    /// A KiwiSDR or Web-888 (the board its listing calls "KiwiSDR 2") reached
    /// over the network — the ~900 receivers published on `rx.kiwisdr.com`, and
    /// any private one on the same firmware.
    ///
    /// The shape is [`Backend::SpyServerVfo`]'s: a *narrow* I/Q window that
    /// follows the dial — the receiver's own audio channel, taken as I/Q at
    /// about 12 kHz — plus the receiver's full 0-30 MHz waterfall as the band
    /// view. Two WebSocket streams from one session, and the only two the
    /// protocol offers; there is no wideband I/Q to ask for.
    ///
    /// Receive only, and not because of a missing feature: these are other
    /// people's antennas. Appended after `HydraSdr` above, for the same
    /// reason as `SmartSdr` there.
    KiwiSdr,
    /// RigExpert Fobos SDR, driven through the vendor's `libfobos` — the one
    /// interface this radio speaks; no separately documented raw USB
    /// protocol exists to reimplement instead. `libfobos` is LGPL-2.1 and
    /// open source, but found with dlopen at *runtime* all the same, for the
    /// same build-portability reason the SDRplay and LimeSDR backends do:
    /// nothing is linked at build time, so this ships in every build variant
    /// and a machine without it enumerates nothing and opening explains what
    /// to install. Appended last, for the same reason as `SmartSdr` above.
    Fobos,
}

impl Backend {
    pub const ALL: [Backend; 22] = [
        Backend::Auto,
        Backend::Soapy,
        Backend::Cat,
        Backend::Hpsdr,
        Backend::Tci,
        Backend::IcomNet,
        Backend::SmartSdr,
        Backend::Pluto,
        Backend::RtlSdr,
        Backend::RtlTcp,
        Backend::SpyServer,
        Backend::SpyServerVfo,
        Backend::KiwiSdr,
        Backend::Rx888,
        Backend::AirspyHf,
        Backend::Airspy,
        Backend::HackRf,
        Backend::SdrPlay,
        Backend::Elad,
        Backend::Lime,
        Backend::HydraSdr,
        Backend::Fobos,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Backend::Auto => "Auto-detect (SoapySDR / CAT)",
            Backend::Soapy => "SoapySDR",
            Backend::Cat => "CAT / Audio",
            Backend::Hpsdr => "HPSDR (network)",
            Backend::Tci => "TCI (network)",
            Backend::IcomNet => "Icom LAN (network)",
            Backend::SmartSdr => "SmartSDR / FlexRadio (network)",
            Backend::Pluto => "PlutoSDR (network)",
            Backend::RtlSdr => "RTL-SDR (USB)",
            Backend::RtlTcp => "RTL-SDR over rtl_tcp (network)",
            Backend::SpyServer => "SpyServer (network)",
            Backend::SpyServerVfo => "SpyServer VFO+FFT, low bandwidth (network)",
            Backend::KiwiSdr => "KiwiSDR / Web-888 (network)",
            Backend::Rx888 => "RX-888 (USB)",
            Backend::AirspyHf => "Airspy HF+ (USB)",
            Backend::Airspy => "Airspy R2 / Mini (USB)",
            Backend::HackRf => "HackRF One / Pro (USB)",
            Backend::SdrPlay => "SDRplay RSP (USB)",
            Backend::Elad => "ELAD FDM-DUO / FDM-S (USB)",
            Backend::Lime => "LimeSDR + LimeRFE (LimeSuite)",
            Backend::HydraSdr => "HydraSDR RFOne (USB)",
            Backend::Fobos => "RigExpert Fobos SDR (USB)",
            Backend::None => "Not configured",
        }
    }
}

/// One device from a SoapySDR enumeration. Wasm-safe so the list can cross the
/// `RadioController` trait to the settings UI, like [`SdrPlayDevice`] — the
/// SoapySDR types themselves live behind the native `soapy` feature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoapyDeviceInfo {
    /// The driver key, as SoapySDR spells it. Case is *not* dependable: an
    /// enumeration reports `audio` where the opened device's `driver_key()`
    /// says `Audio`, so every comparison here folds case.
    pub driver: String,
    /// The human label the module publishes ("Audio (Audio)", "SDRplay
    /// Dev0 RSP1A 2405001234").
    pub label: String,
    /// The full args string that opens exactly this device.
    pub args: String,
}

impl SoapyDeviceInfo {
    /// SoapySDR modules that are not receivers at all.
    ///
    /// `audio` is SoapyAudio, which presents any sound card as an SDR: it
    /// accepts every tuning request, ignores them all, and returns the sound
    /// card's input. On a bundle install (PothosSDR ships every module) it
    /// enumerates ahead of the real hardware, so "the first device found" can
    /// silently be the machine's line input — a spectrum that looks like a
    /// receiver with a dead antenna. `null` is SoapySDR's own test stub.
    ///
    /// These are never what an operator means by "my SDR", so they are only
    /// ever opened when named explicitly.
    pub fn driver_is_pseudo(driver: &str) -> bool {
        matches!(driver.trim().to_ascii_lowercase().as_str(), "audio" | "null")
    }

    /// The native sdroxide interface that drives this hardware directly, for
    /// drivers that have one. The native backends carry the model-specific
    /// controls a generic SoapySDR device cannot express — per-band LNA state
    /// and notches on an RSP, the bias tee and direct sampling on an RTL-SDR —
    /// so an operator reaching them through SoapySDR is losing most of the
    /// radio.
    pub fn native_backend_for(driver: &str) -> Option<Backend> {
        match driver.trim().to_ascii_lowercase().as_str() {
            "sdrplay" => Some(Backend::SdrPlay),
            "rtlsdr" => Some(Backend::RtlSdr),
            "plutosdr" => Some(Backend::Pluto),
            // Two different radios behind two different SoapySDR modules, and
            // each now has its own native backend. Steering `airspy` at the HF+
            // interface (or the other way round) would open the wrong driver
            // against the wrong silicon.
            "airspyhf" => Some(Backend::AirspyHf),
            "airspy" => Some(Backend::Airspy),
            // SoapyHydraSDR is HydraSDR's own module and reaches the same
            // receiver, but it cannot select the RF port and it stops at the
            // three sample rates the firmware admits to.
            "hydrasdr" => Some(Backend::HydraSdr),
            // Worth steering even harder than the rest: SoapyHackRF drops the
            // receive amp on the first transmit and never applies the transmit
            // one at all, which the native driver does not do.
            "hackrf" => Some(Backend::HackRf),
            // Both sides of this one end up in LimeSuite — SoapyLMS7 is a thin
            // wrapper over it — so the steer is not about the I/Q path at all.
            // It is about the LimeRFE, which SoapySDR cannot reach.
            "lime" => Some(Backend::Lime),
            _ => None,
        }
    }

    pub fn is_pseudo(&self) -> bool {
        Self::driver_is_pseudo(&self.driver)
    }

    pub fn native_backend(&self) -> Option<Backend> {
        Self::native_backend_for(&self.driver)
    }

    /// One-line label for a device list.
    pub fn label(&self) -> String {
        format!("{}  (driver {})", self.label, self.driver)
    }

    /// Operator-facing warning for a device that is not a radio, `None` for
    /// real hardware. One composer so the running-source notice, the settings
    /// list and `--probe` all say the same thing.
    pub fn pseudo_warning(driver: &str, label: &str) -> Option<String> {
        if !Self::driver_is_pseudo(driver) {
            return None;
        }
        Some(format!(
            "SoapySDR opened \"{label}\" (driver {driver}) — a sound card, not a radio. \
             It ignores the dial, so what you see is the sound card's input, not the \
             band. Pick a real device with --device / device_args, or choose a native \
             interface in Settings → Radio."
        ))
    }
}

/// CAT protocol family. Only `Xiegu` is hardware-verified so far.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CatFamily {
    #[default]
    Xiegu,
    Icom,
    Yaesu,
    Kenwood,
    /// K3, K3S, KX3, KX2 — and a K4, which answers the same command set.
    ///
    /// A dialect of Kenwood's rather than a family of its own, but not one the
    /// Kenwood profile can drive: DATA is a mode here instead of a flag, and
    /// the two disagree about what keys the transmitter.
    Elecraft,
    /// FDM-DUO and FDM-DUOr.
    ///
    /// Another Kenwood dialect, and a thinner one than Elecraft's: ELAD
    /// describes it as "proprietary commands and also a subset of the TS-480
    /// set", and past the dial and the mode the subset runs out. The meters,
    /// the filter, the power and the split are all ELAD's own commands on
    /// ELAD's own scales, so the Kenwood profile would drive the frequency and
    /// then read every needle wrong.
    ///
    /// This is the *control* half of an FDM-DUO. Its wideband I/Q arrives on a
    /// separate USB interface that this family knows nothing about — see
    /// [`Backend::Elad`], which drives both at once.
    Elad,
    /// Not a radio at all: an already-running Hamlib `rigctld`, over TCP.
    ///
    /// The catch-all. Every profile above drives one manufacturer's rigs
    /// natively and does things a translation layer cannot — reading the model
    /// off the radio to scale its meters, pinning a data sub-mode, keying from
    /// the rig's own text buffer — but between them they still cover only five
    /// makes. Hamlib covers a couple of hundred, and where it is already
    /// installed and working there is no reason to be locked out of it.
    ///
    /// What is given up is real: the daemon exposes frequency, mode, PTT,
    /// power, the S-meter and the SWR, and nothing at all of the keyer chunking
    /// or the per-model filter tables. Prefer a native profile where one fits
    /// the radio.
    Rigctld,
    /// Not a radio either: an already-running flrig, over its XML-RPC port.
    ///
    /// The other daemon, and the other catch-all. Like [`CatFamily::Rigctld`]
    /// this drives a program that is already driving the rig — but through
    /// flrig's own per-model driver, and on a number of radios flrig's handling
    /// of the transmit power and the receive bandwidth is the more faithful of
    /// the two. It also shares the rig: flrig's panel and everything else
    /// pointed at it stay live while this end is connected, which no serial
    /// family can offer.
    ///
    /// Reaches the frequency, mode, PTT, transmit power (in whole watts, scaled
    /// by what the rig says its maximum is), the receive bandwidth (flrig snaps
    /// to the nearest filter its driver has), the S-meter (flrig hands it over
    /// already in dBm), and the SWR and power-out meters while transmitting.
    /// CW goes through flrig's own `cwio` keyer — a DTR/RTS line on a port
    /// configured inside flrig, not the rig's internal keyer — so it keys
    /// nothing until that port is set up there. No RIT/XIT clear exists in
    /// flrig's interface (only split), and no antenna switching.
    ///
    /// Appended last on purpose: `CatFamily` is postcard-encoded by declaration
    /// index inside [`RadioConfig`], so a new variant may only go at the end.
    /// Where it appears in the picker is [`CatFamily::ALL`]'s business.
    Flrig,
    /// QMX, QMX+ and the QDX-series radios that share their command set.
    ///
    /// A third Kenwood dialect — QRP Labs describe it as "a subset of the
    /// Kenwood TS-480/TS-440 CAT command set" — and the one that diverges
    /// furthest. `PC` is the power *meter* here rather than the power control,
    /// so a radio driven as a Kenwood would be told to transmit at whatever
    /// wattage the slider asked for and would answer with a meter reading;
    /// `MD8` is SWR Tune rather than a mode; and the USB sound card is either
    /// demodulated audio or raw I/Q depending on a setting (`Q9`) that only
    /// this profile knows to assert.
    ///
    /// Appended after [`CatFamily::Flrig`] for the reason given there.
    QrpLabs,
}

impl CatFamily {
    pub const ALL: [CatFamily; 9] = [
        CatFamily::Xiegu,
        CatFamily::Icom,
        CatFamily::Yaesu,
        CatFamily::Kenwood,
        CatFamily::Elecraft,
        CatFamily::Elad,
        CatFamily::QrpLabs,
        CatFamily::Rigctld,
        CatFamily::Flrig,
    ];

    /// Whether this family reaches the radio over the network rather than a
    /// serial port, in which case the serial settings mean nothing.
    pub fn is_network(self) -> bool {
        matches!(self, CatFamily::Rigctld | CatFamily::Flrig)
    }
    pub fn label(self) -> &'static str {
        match self {
            CatFamily::Xiegu => "Xiegu",
            CatFamily::Icom => "Icom",
            CatFamily::Yaesu => "Yaesu",
            CatFamily::Kenwood => "Kenwood",
            CatFamily::Elecraft => "Elecraft",
            CatFamily::Elad => "ELAD",
            CatFamily::QrpLabs => "QRP Labs",
            CatFamily::Rigctld => "Hamlib rigctld (network)",
            CatFamily::Flrig => "flrig (network)",
        }
    }
}

/// Which Icom is on the other end of the CI-V link.
///
/// CI-V is one protocol across the whole range, so almost nothing here depends
/// on the model — the frequency, the mode, the meters and the power all work
/// the same on every one of them. Two things do not, and both are the sort that
/// fail quietly:
///
/// * **The transceiver address.** Every model ships with a different one, and a
///   frame addressed to the wrong number is simply ignored — a radio that
///   answers nothing at all, with no error anywhere to say why. Picking the
///   model fills it in.
/// * **DATA mode.** On CI-V, USB and USB-DATA are the *same* mode byte; what
///   separates them is a second command (`1A 06`) that most but not all models
///   have. Without it a digital-mode over goes out through the microphone
///   input, with the rig's speech processing and SSB filter in the path.
///
/// [`IcomModel::Other`] leaves both to the operator: the address is typed by
/// hand and no DATA-mode command is sent, which is the only safe answer for a
/// radio this list has never been told about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum IcomModel {
    #[default]
    Ic7300,
    Ic7300Mk2,
    Ic705,
    Ic7760,
    Ic905,
    Ic9700,
    Ic7610,
    Ic7100,
    Ic7410,
    Ic7600,
    Ic7700,
    Ic7800,
    Ic9100,
    Ic7200,
    Ic7000,
    Other,
}

impl IcomModel {
    pub const ALL: [IcomModel; 16] = [
        IcomModel::Ic7300,
        IcomModel::Ic7300Mk2,
        IcomModel::Ic705,
        IcomModel::Ic7760,
        IcomModel::Ic905,
        IcomModel::Ic9700,
        IcomModel::Ic7610,
        IcomModel::Ic7100,
        IcomModel::Ic7410,
        IcomModel::Ic7600,
        IcomModel::Ic7700,
        IcomModel::Ic7800,
        IcomModel::Ic9100,
        IcomModel::Ic7200,
        IcomModel::Ic7000,
        IcomModel::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            IcomModel::Ic7300 => "IC-7300",
            IcomModel::Ic7300Mk2 => "IC-7300MK2",
            IcomModel::Ic705 => "IC-705",
            IcomModel::Ic7760 => "IC-7760",
            IcomModel::Ic905 => "IC-905",
            IcomModel::Ic9700 => "IC-9700",
            IcomModel::Ic7610 => "IC-7610",
            IcomModel::Ic7100 => "IC-7100",
            IcomModel::Ic7410 => "IC-7410",
            IcomModel::Ic7600 => "IC-7600",
            IcomModel::Ic7700 => "IC-7700",
            IcomModel::Ic7800 => "IC-7800",
            IcomModel::Ic9100 => "IC-9100",
            IcomModel::Ic7200 => "IC-7200",
            IcomModel::Ic7000 => "IC-7000",
            IcomModel::Other => "Other (set the address by hand)",
        }
    }

    /// The address this model leaves the factory with. `None` for
    /// [`IcomModel::Other`], where the operator types it.
    pub fn civ_addr(self) -> Option<u8> {
        Some(match self {
            IcomModel::Ic7300 => 0x94,
            IcomModel::Ic7300Mk2 => 0xB6,
            IcomModel::Ic705 => 0xA4,
            IcomModel::Ic7760 => 0xB2,
            IcomModel::Ic905 => 0xAC,
            IcomModel::Ic9700 => 0xA2,
            IcomModel::Ic7610 => 0x98,
            IcomModel::Ic7100 => 0x88,
            IcomModel::Ic7410 => 0x80,
            IcomModel::Ic7600 => 0x7A,
            IcomModel::Ic7700 => 0x74,
            IcomModel::Ic7800 => 0x6A,
            IcomModel::Ic9100 => 0x7C,
            IcomModel::Ic7200 => 0x76,
            IcomModel::Ic7000 => 0x70,
            IcomModel::Other => return None,
        })
    }

    /// Top of this model's `27 00` scope amplitude scale.
    ///
    /// `160` across the IC-7300 generation; the IC-7610 and the IC-7760 sweep
    /// a taller `0 ~ 200`, in 689 points rather than 475. Icom documents no dB
    /// per step for any of them, so this only says what "full scale" means —
    /// reading the wrong one puts the whole trace 20 dB out, which the
    /// panadapter's auto-levelling then hides.
    pub fn scope_full_scale(self) -> u8 {
        match self {
            IcomModel::Ic7610 | IcomModel::Ic7760 => 200,
            _ => 160,
        }
    }

    /// The `1A` sub-command that switches DATA mode on this model, or `None`
    /// where there is none.
    ///
    /// `06` on everything current. The IC-7200 is the odd one — it does the
    /// same job from `04` — and the IC-7000 has no such command at all: its
    /// data input is selected at the radio, not over CI-V.
    pub fn data_mode_sub(self) -> Option<u8> {
        match self {
            IcomModel::Ic7200 => Some(0x04),
            IcomModel::Ic7000 | IcomModel::Other => None,
            _ => Some(0x06),
        }
    }
}

/// Which transceiver generation's `TX` command keys the rig.
///
/// Really a question about the rig, not about sdroxide: the two generations
/// disagree about what the `TX` parameter *means*, and there is no value that
/// is right on both. On a TS-590 and later, `TX1;` is DATA SEND — key with the
/// ACC2/USB input live — while the plain send selects the microphone input and
/// puts a digital-mode station on the air with no audio at all. On a TS-2000,
/// `TX1;` instead means transmit on the **sub-band**, which is a different band
/// entirely. Nothing on the wire distinguishes the two, so the operator says.
///
/// The default is the generation whose command cannot transmit somewhere
/// unintended: a TS-590 set wrong is silent, a TS-2000 set wrong is on the air
/// in the wrong place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum KenwoodSend {
    /// TS-2000 and earlier (TS-480, TS-570, TS-870, TS-2000): plain `TX;` — the
    /// ordinary send, on the main band. Also the right answer for any Kenwood
    /// with no separate data input to key.
    ///
    /// The alias is the name this variant shipped under before it was renamed
    /// by model. It is not cosmetic: `radio.json` is stored by variant *name*,
    /// so without it every config written under the old name fails to
    /// deserialize — and because serde fails the whole `RadioConfig`, that
    /// takes the operator's interface selection with it and resets them to the
    /// default backend. One renamed word cost a working RX-888 setup exactly
    /// that way.
    #[default]
    #[serde(alias = "Standard")]
    Ts2000,
    /// TS-590 and later (TS-590S/SG, TS-890, TS-990): `TX1;` — DATA SEND, which
    /// keys with the ACC2/USB audio input live rather than the microphone.
    #[serde(alias = "Data")]
    Ts590,
}

impl KenwoodSend {
    pub const ALL: [KenwoodSend; 2] = [KenwoodSend::Ts2000, KenwoodSend::Ts590];
    pub fn label(self) -> &'static str {
        match self {
            KenwoodSend::Ts2000 => "TS-2000 style (TX;)",
            KenwoodSend::Ts590 => "TS-590 style (TX1;)",
        }
    }
}

/// Where an ELAD FDM-DUO takes its transmit audio from — the rig's `TI`
/// command, and menu 32 `TX IN` at the front panel.
///
/// A setting rather than an assumption because the radio remembers it across
/// power cycles and both mistakes are silent. A DUO left on `MIC` transmits the
/// microphone no matter what sdroxide puts into its sound card — a digital-mode
/// over that goes out as room noise. One forced to `USB` behind an operator who
/// wanted the microphone takes their voice away with nothing on screen to say
/// where it went.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EladTxInput {
    /// `TI1;` — the USB audio port, which is where sdroxide's transmit audio
    /// goes. The default, because it is the setting that makes this interface
    /// work at all.
    #[default]
    UsbAudio,
    /// `TI0;` — the microphone jack. For an operator who talks into the radio
    /// and uses sdroxide for everything else.
    Mic,
    /// `TI2;` — the rig decides: the microphone for a PTT press on the
    /// microphone, the USB port for a CAT or RTS key-down.
    Auto,
    /// Send no `TI` at all and leave whatever the radio was set to.
    Leave,
}

impl EladTxInput {
    pub const ALL: [EladTxInput; 4] =
        [EladTxInput::UsbAudio, EladTxInput::Mic, EladTxInput::Auto, EladTxInput::Leave];
    pub fn label(self) -> &'static str {
        match self {
            EladTxInput::UsbAudio => "USB audio",
            EladTxInput::Mic => "Microphone",
            EladTxInput::Auto => "Auto",
            EladTxInput::Leave => "Leave as set on the radio",
        }
    }
}

/// How the radio's audio is carried over the sound card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SoundFormat {
    /// Stereo L=I, R=Q complex baseband → normal wideband engine path.
    Iq,
    /// Mono already-demodulated audio → audio-band panadapter (engine bypass).
    #[default]
    DemodAudio,
}

impl SoundFormat {
    pub const ALL: [SoundFormat; 2] = [SoundFormat::DemodAudio, SoundFormat::Iq];
    pub fn label(self) -> &'static str {
        match self {
            SoundFormat::Iq => "IQ (stereo)",
            SoundFormat::DemodAudio => "Demod audio",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Parity {
    #[default]
    None,
    Even,
    Odd,
}

impl Parity {
    pub const ALL: [Parity; 3] = [Parity::None, Parity::Even, Parity::Odd];
    pub fn label(self) -> &'static str {
        match self {
            Parity::None => "None",
            Parity::Even => "Even",
            Parity::Odd => "Odd",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StopBits {
    #[default]
    One,
    Two,
}

impl StopBits {
    pub const ALL: [StopBits; 2] = [StopBits::One, StopBits::Two];
    pub fn label(self) -> &'static str {
        match self {
            StopBits::One => "1",
            StopBits::Two => "2",
        }
    }
}

/// A serial control line forced to a fixed level while the port is open (some
/// rigs need DTR/RTS held high to enable CAT). `None` = leave as-is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LineState {
    #[default]
    None,
    High,
    Low,
}

impl LineState {
    pub const ALL: [LineState; 3] = [LineState::None, LineState::High, LineState::Low];
    pub fn label(self) -> &'static str {
        match self {
            LineState::None => "None",
            LineState::High => "High",
            LineState::Low => "Low",
        }
    }
}

/// How to key the transmitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PttMethod {
    /// Rig keys itself from TX audio; software just routes audio.
    Vox,
    Dtr,
    Rts,
    /// A CAT command keys the rig.
    #[default]
    Cat,
}

impl PttMethod {
    pub const ALL: [PttMethod; 4] =
        [PttMethod::Cat, PttMethod::Dtr, PttMethod::Rts, PttMethod::Vox];
    pub fn label(self) -> &'static str {
        match self {
            PttMethod::Vox => "VOX",
            PttMethod::Dtr => "DTR",
            PttMethod::Rts => "RTS",
            PttMethod::Cat => "CAT",
        }
    }
}

/// Who drives the rig's mode for ordinary modes (USB/LSB/CW/AM/FM/DIGU/DIGL).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ModeControl {
    /// The app commands the rig's mode over CAT to match the selected mode.
    #[default]
    Cat,
    /// The operator sets the mode on the radio; the app just follows it.
    Radio,
}

impl ModeControl {
    pub const ALL: [ModeControl; 2] = [ModeControl::Cat, ModeControl::Radio];
    pub fn label(self) -> &'static str {
        match self {
            ModeControl::Cat => "CAT",
            ModeControl::Radio => "Radio controlled",
        }
    }
}

/// What mode the rig should be in for the FT8/FT4 digital engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DigiMode {
    /// Force the rig to USB.
    #[default]
    Usb,
    /// Force the rig to its DATA/PKT (USB-D) mode.
    Data,
    /// Leave the rig's mode as the operator set it.
    Radio,
}

impl DigiMode {
    pub const ALL: [DigiMode; 3] = [DigiMode::Usb, DigiMode::Data, DigiMode::Radio];
    pub fn label(self) -> &'static str {
        match self {
            DigiMode::Usb => "USB",
            DigiMode::Data => "DIGI",
            DigiMode::Radio => "Radio controlled",
        }
    }
}

/// Where a CAT rig's CW comes from when the panel's keyer sends.
///
/// A transceiver in CW mode does not modulate what arrives at its sound card —
/// the transmitter is keyed, by a key line or by its own memory keyer, and
/// nothing else reaches the air. So sidetone written to the rig's playback
/// device (which is all an SDR-side keyer can produce) is silently discarded,
/// and the operator hears nothing go out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CwKeying {
    /// Hand the text to the rig and let its own keyer send it (Yaesu keyer
    /// memory playback, Icom CI-V "send CW"). The rig keys itself, so this is
    /// the only route that puts CW on the air from a rig that is *in* CW.
    #[default]
    Cat,
    /// Send the keyer's sidetone through the rig's sound card as audio, a tone
    /// on the sideband (MCW) at dial + pitch rather than CW on the dial
    /// frequency. Because it only reaches the air from a voice/data mode,
    /// selecting CW then keeps the rig on the digital modes' sideband
    /// (`digi_mode`; plain USB on the LAN backend) instead of switching it to
    /// CW — the route for rigs whose keyer sdroxide cannot drive, like a Xiegu
    /// G90 (issue #119).
    Audio,
}

impl CwKeying {
    pub const ALL: [CwKeying; 2] = [CwKeying::Cat, CwKeying::Audio];
    pub fn label(self) -> &'static str {
        match self {
            CwKeying::Cat => "Rig keyer (CAT)",
            CwKeying::Audio => "Sound card (MCW)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SerialConfig {
    /// Serial device path (Linux/mac `/dev/tty…`, Windows `COMx`).
    pub path: String,
    pub baud: u32,
    pub data_bits: u8,
    pub parity: Parity,
    pub stop_bits: StopBits,
    pub force_rts: LineState,
    pub force_dtr: LineState,
}

impl Default for SerialConfig {
    fn default() -> Self {
        SerialConfig {
            path: String::new(),
            baud: 19200,
            data_bits: 8,
            parity: Parity::None,
            stop_bits: StopBits::One,
            force_rts: LineState::None,
            force_dtr: LineState::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CatConfig {
    pub family: CatFamily,
    pub serial: SerialConfig,
    pub ptt: PttMethod,
    /// How often to poll the rig for its dial, mode and meters (Hz).
    ///
    /// The whole of the control traffic this end generates, and on a fair
    /// number of radios that is not free. A modern Icom is a USB hub with the
    /// CI-V port and the sound card behind it — an IC-7300 enumerates a TI
    /// 4-port hub carrying a CP2102 and a PCM2901, both at full speed — so
    /// every frame asked for on the control port is bus time the audio does not
    /// get, inside the radio, whatever the cable outside it is like. The
    /// symptom is dropouts in the received audio that look like a DSP fault and
    /// are not one.
    ///
    /// Two hertz by default, which is half a second behind the rig's own VFO
    /// knob and quiet enough not to be the thing breaking the audio. Raise it
    /// on a radio whose control port is its own device — a separate USB-serial
    /// adapter, a network `rigctld` — where the traffic competes with nothing.
    pub poll_hz: f32,
    /// Who controls the rig's mode for ordinary modes.
    pub mode_control: ModeControl,
    /// What mode the rig uses for the FT8/FT4 engine.
    pub digi_mode: DigiMode,
    /// Where CW the operator sends comes from — the rig's own keyer, or the
    /// keyer's sidetone over the sound card.
    pub cw_keying: CwKeying,
    /// Icom CI-V transceiver address (hex byte), e.g. 0x70 for many rigs.
    pub icom_radio_id: u8,
    /// Which Icom, for the handful of things CI-V does not do the same way on
    /// all of them. Defaulted so a config written before this existed still
    /// loads — see the note on [`KenwoodSend::Ts2000`] for what a failed
    /// deserialise costs.
    #[serde(default)]
    pub icom_model: IcomModel,
    /// `host:port` of the Hamlib `rigctld` to drive, for
    /// [`CatFamily::Rigctld`]. Defaulted so a config written before this
    /// existed still loads.
    #[serde(default = "default_rigctld_addr")]
    pub rigctld_addr: String,
    /// `host:port` of the flrig to drive, for [`CatFamily::Flrig`]. Defaulted
    /// so a config written before this existed still loads.
    #[serde(default = "default_flrig_addr")]
    pub flrig_addr: String,
    /// Which `TX` command keys a Kenwood.
    pub kenwood_send: KenwoodSend,
    /// Where an ELAD FDM-DUO takes its transmit audio from, asserted when the
    /// port opens. Defaulted so a config written before this existed still
    /// loads — see the note on [`KenwoodSend::Ts2000`] for what a failed
    /// deserialise costs.
    #[serde(default)]
    pub elad_tx_input: EladTxInput,
    pub format: SoundFormat,
    /// Conjugate the sound card's I/Q, mirroring the panadapter about the
    /// tuned frequency. Only read for [`SoundFormat::Iq`]: demod audio is real
    /// and has no sideband to swap.
    ///
    /// Which of the two channels a quadrature rig calls I is a wiring
    /// convention, and the ones that disagree with this end are indistinguishable
    /// from correct until you look at what is on the waterfall — the band is
    /// mirrored about the dial, so signals sit on the wrong side of it and SSB
    /// comes out on the opposite sideband. Reversing the two cables at the sound
    /// card would fix it just as well; this is the same fix without the soldering
    /// iron, and it is why the setting exists per rig rather than per family.
    ///
    /// **Off by default** — the convention this end assumes (I on the left
    /// channel, Q on the right) is the common one, and every rig already working
    /// has to keep working.
    ///
    /// Receive only. The transmit side of this interface is not quadrature: it
    /// hands the rig one real audio signal, which the radio modulates, and a
    /// real signal has no sideband to invert.
    #[serde(default)]
    pub invert_spectrum: bool,
    /// How far the rig's I/Q output is centred *above* its own dial, in Hz —
    /// the Elecraft KX3's `RX SHFT`, and anything else that moves a quadrature
    /// rig's receive I.F. off zero. Only read for [`SoundFormat::Iq`]; demod
    /// audio has already been mixed down by the radio.
    ///
    /// A quadrature rig normally puts its local oscillator on the dial, which
    /// piles the mixer's own DC offset, LO leakage and the sound card's zero-Hz
    /// junk exactly on the signal being listened to. The cure, on rigs that
    /// offer it, is to move the I.F. off zero: the KX3's `RX SHFT` menu entry
    /// set to `8.0` puts the LO 8 kHz from the dial, so the dial is no longer
    /// on the spike — and, per Elecraft's own note, also stops a nearby
    /// high-power SSB/AM station being AM-detected in the receiver.
    ///
    /// The rig keeps displaying — and transmitting on — the real frequency, so
    /// this must **not** be entered as a converter offset: a converter retunes
    /// the radio, which is precisely what a shifted I.F. does not do. This
    /// number never reaches the rig. It says where the samples on the sound
    /// card actually sit, and the stream is translated by it on the way in, so
    /// the dial stays the dial for the panadapter, the demodulators, the
    /// skimmer, the logbook and the rig's own display alike.
    ///
    /// **Sign**: positive means the I/Q centre is above the dial, so the signal
    /// tuned appears *below* centre in the stream. `0.0` (the default) is a rig
    /// with its LO on the dial, which is every rig until its owner says
    /// otherwise. See [`CAT_IQ_OFFSET_MAX_HZ`] for the range.
    ///
    /// Receive only, for the same reason as [`Self::invert_spectrum`]: transmit
    /// hands the radio one real audio signal and the rig's own dial decides
    /// where it lands.
    #[serde(default)]
    pub iq_offset_hz: f64,
    /// What sample rate the rig's I/Q sound card is opened at, in Hz — and so
    /// how wide the panadapter is, since a quadrature stream spans its whole
    /// sample rate. Only read for [`SoundFormat::Iq`]: demod audio is a real
    /// signal already inside the rig's filter, and widening the card would only
    /// digitise more silence.
    ///
    /// This is a request, not a guarantee. The card decides: one that cannot do
    /// the rate asked for is opened at the nearest it does, and everything
    /// downstream — the span, the demodulators, the I.F. shift — follows the
    /// rate actually achieved rather than this number. A mismatch is logged at
    /// open, because a panadapter half the width the operator asked for is not
    /// otherwise distinguishable from one they mis-set.
    ///
    /// Defaulted to 48 kHz so a config written before this existed loads
    /// unchanged and comes up exactly as it did.
    #[serde(default = "default_iq_rate_hz")]
    pub iq_rate_hz: u32,
    /// Correct the sound card's quadrature: cancel the **mirror image** every
    /// signal casts on the other side of the centre, and remove the DC offset
    /// that piles a permanent spike on it. Only read for [`SoundFormat::Iq`];
    /// demod audio is a real signal, with neither defect to fix.
    ///
    /// A rig's I/Q output is two analogue paths — the receiver's own quadrature
    /// mixer, then two channels of a sound card — and they are never quite
    /// equal in gain, nor exactly 90° apart. What that leaves is a copy of
    /// every signal reflected about the tuned frequency, typically 30–40 dB
    /// down: strong enough to look like a station that is not there, and to
    /// decode as one on a waterfall full of FT8. Some radios have a pair of
    /// front-panel trimmers for it; the ones that do not are why this exists.
    /// The correction is adaptive and needs no adjustment — see
    /// `sdroxide_dsp::IqCorrect` for what it can and cannot measure.
    ///
    /// **On by default**, and defaulted so a config written before this existed
    /// gets it: it is what every other quadrature front end here already does,
    /// and an image left in place is not something an operator can be expected
    /// to recognise for what it is.
    ///
    /// Receive only, like the two settings above.
    #[serde(default = "default_cat_iq_correction")]
    pub iq_correction: bool,
    /// How much of the middle of the span to high-pass away, in Hz — 0 for the
    /// ordinary DC blocker alone. Only read for [`SoundFormat::Iq`].
    ///
    /// The spike in the centre of the waterfall is DC: the mixer's own offset
    /// and its LO leaking back into itself, sitting on the sound card's zero
    /// hertz. [`Self::iq_correction`] removes the offset, which is a corner of
    /// a few tens of hertz — enough for the level, not always enough for the
    /// look of it, because what is left is the *near*-DC noise either side.
    /// This widens that corner, so an operator who wants the bottom couple of
    /// hundred hertz gone can have it.
    ///
    /// It costs what it says: a first-order high-pass centred on the rig's I/Q
    /// centre, −3 dB at the figure set and falling further in, which is the
    /// dial itself unless [`Self::iq_offset_hz`] has moved it. Anything tuned
    /// there goes with the spike — a CW note at 600 Hz is inside a 600 Hz
    /// setting, and an AM carrier on the dial is DC by definition. Hence 0 by
    /// default, and [`CAT_IQ_DC_BLOCK_MAX_HZ`] as far as it goes.
    #[serde(default)]
    pub iq_dc_block_hz: f64,
    /// Displayed panadapter bandwidth for demod-audio mode (Hz).
    pub audio_bw_hz: f64,
    /// Stream the rig's own spectrum scope over the serial CI-V link and draw
    /// it as the panadapter — the same `27 00` sweeps the LAN backend uses
    /// (see [`IcomNetConfig::scope`]), on the transport the IC-7300 generation
    /// actually has. Icom family only; every other family ignores it.
    ///
    /// **Off by default**, deliberately, where the LAN default is on: over USB
    /// the sweeps share one full-speed bus with the rig's own sound card, and
    /// measured experience with this generation (see [`Self::poll_hz`]) is
    /// that control traffic there is paid for in received-audio dropouts. An
    /// operator who turns it on is choosing the picture over that risk — and
    /// must also set the radio's CI-V USB port to 115200 baud and "Unlink from
    /// [REMOTE]", or the sweeps do not fit down the link at all.
    #[serde(default)]
    pub scope: bool,
    /// How wide to sweep that scope — shared with the LAN backend, and like
    /// there it also puts the scope into centre mode so it follows the dial.
    #[serde(default)]
    pub scope_span: IcomScopeSpan,
}

/// The slowest CI-V link the scope sweeps fit down. A sweep is ~500 bytes of
/// frame ten-ish times a second — roughly 60% of a 115200 line and several
/// times more than a 19200 one carries at all — so asking a slower link for it
/// would bury every poll and PTT under sweep fragments.
pub const CAT_SCOPE_MIN_BAUD: u32 = 115_200;

/// The rates a rig's I/Q sound card may be opened at, in Hz — what the operator
/// picks between, and so what panadapter widths are on offer.
///
/// Every one of these is an ordinary sound-card rate; which of them a given
/// card will actually give is the card's business, and [`CatConfig::iq_rate_hz`]
/// says what happens when it declines.
pub const CAT_IQ_RATES: [u32; 4] = [48_000, 96_000, 192_000, 384_000];

/// The rate a rig's I/Q card is opened at unless the operator says otherwise —
/// the one every such rig was opened at before the setting existed.
fn default_iq_rate_hz() -> u32 {
    48_000
}

/// Where a QMX's I/Q sits relative to its dial, in Hz — the value
/// [`CatConfig::iq_offset_hz`] is prefilled with when [`CatFamily::QrpLabs`] is
/// chosen with [`SoundFormat::Iq`].
///
/// Negative, and the sign is the whole of the point. A QMX is a superhet with a
/// 12 kHz I.F.: the synthesiser sits 12 kHz *below* the operating frequency, so
/// the signal the operator tuned appears 12 kHz *above* the middle of the
/// stream. `iq_offset_hz` is measured the other way round — positive means the
/// centre is above the dial — hence −12000.
///
/// It is not a guess at the sign. QRP Labs' own operating manual pins it: "the
/// image response is 24kHz down the band", which only holds for a local
/// oscillator 12 kHz below the signal, and the receiver's image sweep is run
/// from −30.5 kHz to +7 kHz for the same reason.
///
/// ⚠️ In CW the radio adds a further ~700 Hz to that offset (the pitch, so that
/// zero-beat stays zero-beat), which this single number cannot follow. An
/// operator running the panadapter with the radio in CW can add it by hand; in
/// Digi — where a radio being used as an I/Q front end normally sits — there is
/// nothing to add.
pub const QMX_IQ_OFFSET_HZ: f64 = -12_000.0;

/// What a QMX's USB codec runs at: "24-bit 48 ksps USB sound card", and the ADC
/// behind it "digitizes the I and Q channels at 48 ksps". There is no other
/// rate to ask for, so the panadapter is 48 kHz wide and that is the whole of
/// the band this radio can show at once.
pub const QMX_IQ_RATE_HZ: u32 = 48_000;

/// Whether a rig's I/Q is corrected unless the operator says otherwise. On:
/// see [`CatConfig::iq_correction`].
fn default_cat_iq_correction() -> bool {
    true
}

/// How far [`CatConfig::iq_dc_block_hz`] may be wound up, in Hz.
///
/// Half a kilohertz, which covers the "first two or three hundred hertz" an
/// operator actually asks for with room to spare, and stops short of a setting
/// that would swallow a CW note. Past this the answer is not a wider notch but
/// a shifted I.F. ([`CatConfig::iq_offset_hz`]), which moves the signal away
/// from the spike instead of digging a hole around it.
pub const CAT_IQ_DC_BLOCK_MAX_HZ: f64 = 500.0;

/// How far either way [`CatConfig::iq_offset_hz`] may be set for a card running
/// at `iq_rate_hz`, in Hz.
///
/// Half the sample rate: a quadrature stream spans its whole rate about the
/// centre, so an offset past the halfway mark puts the dial outside the window
/// the card digitises at all — which is not a shifted I.F. but a receiver
/// pointed at nothing. The 8 kHz an Elecraft asks for is comfortably inside
/// even the narrowest of [`CAT_IQ_RATES`].
pub fn cat_iq_offset_max_hz(iq_rate_hz: u32) -> f64 {
    iq_rate_hz as f64 / 2.0
}

/// Where a `rigctld` listens unless told otherwise — the daemon's own default
/// port, on this machine.
fn default_rigctld_addr() -> String {
    "127.0.0.1:4532".to_string()
}

/// Where an flrig serves XML-RPC unless told otherwise — its own default port,
/// on this machine.
fn default_flrig_addr() -> String {
    "127.0.0.1:12345".to_string()
}

impl Default for CatConfig {
    fn default() -> Self {
        CatConfig {
            family: CatFamily::default(),
            serial: SerialConfig::default(),
            ptt: PttMethod::default(),
            poll_hz: 2.0,
            mode_control: ModeControl::default(),
            digi_mode: DigiMode::default(),
            cw_keying: CwKeying::default(),
            icom_radio_id: 0x70,
            icom_model: IcomModel::default(),
            rigctld_addr: default_rigctld_addr(),
            flrig_addr: default_flrig_addr(),
            kenwood_send: KenwoodSend::default(),
            elad_tx_input: EladTxInput::default(),
            format: SoundFormat::default(),
            invert_spectrum: false,
            iq_offset_hz: 0.0,
            iq_rate_hz: default_iq_rate_hz(),
            iq_correction: default_cat_iq_correction(),
            iq_dc_block_hz: 0.0,
            audio_bw_hz: 4000.0,
            scope: false,
            scope_span: IcomScopeSpan::default(),
        }
    }
}

/// Where an N2ADR HL2IOBoard takes its receive signal from — the board's
/// `REG_RF_INPUTS`, which its documentation asks host software to expose.
///
/// The IO board carries two SMA jacks of its own: J9, which can replace the
/// radio's receive input, and J10, a PureSignal (transmit-sample) input. Only an
/// operator who has wired one of them should move this off the default: selecting
/// J9 with nothing on it leaves the receiver deaf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum HpsdrIoRxInput {
    /// Mode 0 — the radio's own receive input, with J10 mixed into it.
    #[default]
    Radio,
    /// Mode 1 — J9 on the IO board is the receive input; no PureSignal.
    IoBoard,
    /// Mode 2 — J9 receives, and on transmit J10's PureSignal sample is passed
    /// to the radio in place of a receive signal.
    IoBoardPureSignal,
}

impl HpsdrIoRxInput {
    pub const ALL: [HpsdrIoRxInput; 3] =
        [HpsdrIoRxInput::Radio, HpsdrIoRxInput::IoBoard, HpsdrIoRxInput::IoBoardPureSignal];

    /// The value written to the board's `REG_RF_INPUTS`.
    pub fn code(self) -> u8 {
        match self {
            HpsdrIoRxInput::Radio => 0,
            HpsdrIoRxInput::IoBoard => 1,
            HpsdrIoRxInput::IoBoardPureSignal => 2,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            HpsdrIoRxInput::Radio => "Radio's own input",
            HpsdrIoRxInput::IoBoard => "IO board J9",
            HpsdrIoRxInput::IoBoardPureSignal => "IO board J9, PureSignal on transmit",
        }
    }
}

/// Which accessory filter board is wired to a Hermes-Lite 2's J16 header, and
/// therefore how its seven open-collector outputs should be driven.
///
/// Those pins are general-purpose openHPSDR outputs, not filter-only: operators
/// also wire them to amplifier PTT, antenna relays and transverter switching.
/// Driving them from band data would start operating that hardware, so the
/// default leaves every one of them off and the operator says what is attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum HpsdrFilterBoard {
    /// Leave all seven outputs off — the safe default, and correct for a bare
    /// board with nothing on J16.
    #[default]
    None,
    /// N2ADR filter board: one-hot relay select, forwarded by the gateware over
    /// I2C to the board's MCP23008.
    N2adr,
    /// Alex-style band code: the band as a four-bit number on outputs 1–4,
    /// which is what a Hermes/ANAN, a Zeus SDR, a HiQSDR and Quisk's own
    /// filter switching all expect (issue #196).
    ///
    /// Outputs 5–7 stay off. They are not part of the band code — on the
    /// boards that use this mapping they are spare pins operators wire to a
    /// preamplifier, an attenuator or a transverter, and this backend has no
    /// way to know which.
    Alex,
    /// Neither convention: the operator states the seven outputs themselves,
    /// band by band, in [`HpsdrConfig::oc_table`] (issue #296).
    ///
    /// The two presets above are the boards this program can name. An operator
    /// with anything else on those pins — an antenna switch, an amplifier's
    /// band decoder, a transverter sequencer, a filter board neither preset
    /// fits — knows what their hardware wants and nothing here can guess it, so
    /// the table is theirs to fill in. Each preset can be *poured into* that
    /// table as a starting point, which is how a board that is nearly an N2ADR
    /// gets configured.
    Custom,
}

/// Open-collector byte for the N2ADR filter board when tuned to `freq_hz`.
///
/// The board selects one-hot from its own documentation: bit 0 = 160 m LPF,
/// 1 = 80 m, 2 = 60/40 m, 3 = 30/20 m, 4 = 17/15 m, 5 = 12/10 m, and bit 6 is a
/// 3 MHz high-pass used on receive to keep broadcast AM out of the front end.
/// The board switches that high-pass out itself while transmitting, so it can be
/// asserted unconditionally above 3 MHz. Frequencies between the ham bands pick
/// the lowest filter that still passes them, so short-wave listening is not
/// filtered into silence.
pub fn hpsdr_n2adr_oc(freq_hz: f64) -> u8 {
    let lpf = match freq_hz {
        f if f <= 2_000_000.0 => 0,
        f if f <= 4_000_000.0 => 1,
        f if f <= 7_300_000.0 => 2,
        f if f <= 14_350_000.0 => 3,
        f if f <= 21_450_000.0 => 4,
        _ => 5,
    };
    let hpf = if freq_hz >= 3_000_000.0 { 1 << 6 } else { 0 };
    (1 << lpf) | hpf
}

/// Open-collector byte for an Alex-style filter board when tuned to `freq_hz`.
///
/// Not one-hot like the N2ADR board above: this is the band as a four-bit
/// number on outputs 1–4, which is the mapping a Hermes/ANAN's Alex board, a
/// Zeus SDR, a HiQSDR and Quisk's own filter switching all share (issue #196).
/// 160 m is 1 and the code counts upwards by band — except 60 m, which is 0,
/// the same byte as "nothing selected", because that is what the boards
/// expect.
///
/// Outputs 5–7 are left off: they carry no part of the band code, and on the
/// boards that use this mapping they are the spare pins an operator wires to a
/// preamplifier, an attenuator or a transverter. An operator who wants those
/// pins driven pours this preset into [`HpsdrConfig::oc_table`] and adds them.
///
/// Between the bands the boundary sits in the middle of the gap, so a
/// short-wave listener gets the nearer of the two filters rather than silence,
/// and everything below 160 m and above 10 m is carried by the band at that
/// end.
pub fn hpsdr_alex_oc(freq_hz: f64) -> u8 {
    match freq_hz {
        f if f < 2_750_000.0 => 0x01,  // 160 m
        f if f < 4_650_000.0 => 0x02,  // 80 m
        f if f < 6_200_000.0 => 0x00,  // 60 m — no pins, by the board's table
        f if f < 8_700_000.0 => 0x03,  // 40 m
        f if f < 12_100_000.0 => 0x04, // 30 m
        f if f < 16_200_000.0 => 0x05, // 20 m
        f if f < 19_600_000.0 => 0x06, // 17 m
        f if f < 23_200_000.0 => 0x07, // 15 m
        f if f < 26_500_000.0 => 0x08, // 12 m
        f if f < 39_900_000.0 => 0x09, // 10 m
        _ => 0x0A,                     // 6 m
    }
}

/// One band's open-collector control words: what the seven outputs are while
/// receiving on that band, and what they are while transmitting on it (issue
/// #296).
///
/// Two words rather than one because the outputs are not only filters. A
/// low-pass filter has to be the same either way, but an amplifier's key line,
/// a transverter's sequencer and a receive preamplifier's bypass are all things
/// an operator wants asserted on exactly one side of the changeover — and
/// Thetis's table, which this is modelled on, has offered both since the
/// beginning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HpsdrOcRow {
    /// The band these words apply to, matched against the frequency on the
    /// *dial*: an accessory board switches for the signal on the air, not for
    /// the intermediate frequency a transverter left the radio on.
    pub band: Band,
    /// Outputs 1–7 as a bit mask (bit 0 = output 1) while receiving. Bit 7 is
    /// not an output and is ignored.
    pub rx: u8,
    /// The same while the transmitter is keyed.
    pub tx: u8,
}

impl HpsdrOcRow {
    /// One of the presets poured into a table, so it can be edited from
    /// something that works rather than from seven zeros.
    ///
    /// Each band is asked for at its own default entry frequency, which is
    /// inside the band in every region. Receive and transmit come out the same:
    /// both presets describe filters, which have to be in circuit in both
    /// directions — the N2ADR board's receive-only 3 MHz high-pass is switched
    /// out by the board itself, not by us. What an operator wants on one side
    /// only is exactly what they add here afterwards.
    pub fn from_preset(board: HpsdrFilterBoard) -> Vec<HpsdrOcRow> {
        let Some(f) = (match board {
            HpsdrFilterBoard::N2adr => Some(hpsdr_n2adr_oc as fn(f64) -> u8),
            HpsdrFilterBoard::Alex => Some(hpsdr_alex_oc as fn(f64) -> u8),
            HpsdrFilterBoard::None | HpsdrFilterBoard::Custom => None,
        }) else {
            return Vec::new();
        };
        Band::ALL
            .iter()
            .filter(|b| **b != Band::Gen)
            .map(|&band| {
                let w = f(band.default_entry().0) & 0x7F;
                HpsdrOcRow { band, rx: w, tx: w }
            })
            .collect()
    }
}

/// The open-collector outputs resolved for a whole connection: which convention
/// is in force, and — for [`HpsdrFilterBoard::Custom`] — the operator's words
/// for every band, ready to be indexed rather than searched.
///
/// Flattened out of [`HpsdrConfig`] at open time and `Copy`, because the
/// protocol threads carry it inside a register snapshot that is copied around
/// several hundred times a second; a `Vec` there would be a heap allocation per
/// datagram and would cost those structs their `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HpsdrOcPlan {
    board: HpsdrFilterBoard,
    /// Indexed by [`Band::index`]; only read under `Custom`.
    rx: [u8; Band::ALL.len()],
    tx: [u8; Band::ALL.len()],
}

impl Default for HpsdrOcPlan {
    fn default() -> Self {
        HpsdrOcPlan {
            board: HpsdrFilterBoard::None,
            rx: [0; Band::ALL.len()],
            tx: [0; Band::ALL.len()],
        }
    }
}

impl HpsdrOcPlan {
    /// Nothing attached: every output stays off, whatever the dial does.
    pub fn none() -> HpsdrOcPlan {
        HpsdrOcPlan::default()
    }

    /// The plan a preset alone describes, for a caller that has no table.
    pub fn preset(board: HpsdrFilterBoard) -> HpsdrOcPlan {
        HpsdrOcPlan { board, ..HpsdrOcPlan::default() }
    }

    /// Which convention this plan drives.
    pub fn board(&self) -> HpsdrFilterBoard {
        self.board
    }

    /// What the log line at open calls this — the operator's only warning that
    /// seven general-purpose outputs are about to start moving, so it names
    /// what will move them rather than repeating a menu entry.
    pub fn describe(&self) -> String {
        match self.board {
            HpsdrFilterBoard::None => "nothing (all outputs off)".into(),
            HpsdrFilterBoard::N2adr => "an N2ADR filter board".into(),
            HpsdrFilterBoard::Alex => "an Alex / Hermes band code".into(),
            HpsdrFilterBoard::Custom => {
                let bands = Band::ALL
                    .iter()
                    .filter(|b| {
                        let i = b.index();
                        self.rx[i] != 0 || self.tx[i] != 0
                    })
                    .count();
                format!("a custom table ({bands} band(s) with outputs asserted)")
            }
        }
    }

    /// Whether anything at all is driven. A plan that asserts nothing on any
    /// band is the same as no accessory board, and is not announced as one.
    pub fn drives_anything(&self) -> bool {
        match self.board {
            HpsdrFilterBoard::None => false,
            HpsdrFilterBoard::Custom => self.rx.iter().chain(&self.tx).any(|&w| w != 0),
            _ => true,
        }
    }

    /// The seven outputs for a radio on `freq_hz`, keyed or not — bit 0 is
    /// output 1, and bit 7 is never set (there is no eighth output).
    ///
    /// The two presets are frequency-driven and answer the same word either
    /// way: they describe filters, and the caller has already handed us the
    /// transmit frequency if that is what is on the air. Only a custom table
    /// distinguishes the two directions, which is the point of having one.
    pub fn word(&self, freq_hz: f64, keyed: bool) -> u8 {
        match self.board {
            HpsdrFilterBoard::None => 0,
            HpsdrFilterBoard::N2adr => hpsdr_n2adr_oc(freq_hz),
            HpsdrFilterBoard::Alex => hpsdr_alex_oc(freq_hz),
            HpsdrFilterBoard::Custom => {
                let i = Band::containing(freq_hz).index();
                if keyed { self.tx[i] } else { self.rx[i] }
            }
        }
    }
}

impl HpsdrFilterBoard {
    pub const ALL: [HpsdrFilterBoard; 4] = [
        HpsdrFilterBoard::None,
        HpsdrFilterBoard::N2adr,
        HpsdrFilterBoard::Alex,
        HpsdrFilterBoard::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            HpsdrFilterBoard::None => "None — outputs stay off",
            HpsdrFilterBoard::N2adr => "N2ADR filter board",
            HpsdrFilterBoard::Alex => "Alex / Hermes band code (Zeus, HiQSDR, Quisk)",
            HpsdrFilterBoard::Custom => "Custom — my own table below",
        }
    }
}

/// OpenHPSDR (ethernet SDR) backend configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HpsdrConfig {
    /// Explicit target IP (e.g. "192.168.1.50"). When set, connect directly and
    /// skip discovery/selection.
    pub manual_ip: Option<String>,
    /// IP of the device picked from a discovery scan (persisted selection).
    pub selected_ip: Option<String>,
    /// DDC sample rate in Hz (48k, 96k, 192k, 384k, 768k, 1536k).
    pub sample_rate_hz: f64,
    /// Front-end LNA gain in dB applied when the radio is opened, on boards
    /// that have one (Hermes-Lite 2: −12…+48 dB). Adjust it live in
    /// Settings → Device; this is the value the rig starts at.
    #[serde(default = "HpsdrConfig::default_lna_gain_db")]
    pub lna_gain_db: f64,
    /// Accessory board on the Hermes-Lite 2's J16 header. Defaults to `None`,
    /// which leaves the open-collector outputs untouched.
    #[serde(default)]
    pub filter_board: HpsdrFilterBoard,
    /// Conjugate the board's I/Q, mirroring the spectrum about the tuned
    /// frequency, on transmit as well as receive so the two directions cannot
    /// disagree about which sideband they are on.
    ///
    /// **On by default**: a Hermes-Lite 2 needs it — verified on air, where
    /// without it FT8 produces no decodes at all and SSB comes out on the wrong
    /// sideband. A board that turns out not to need it can turn it off.
    ///
    /// Deliberately *not* named `swap_iq`, which is what the one release that
    /// defaulted it to off called it. Ignoring that older key is the migration:
    /// whether an operator had found the setting and switched it on, or had it
    /// saved as off without ever knowing it existed, they all land on the value
    /// that works.
    #[serde(default = "HpsdrConfig::default_invert_spectrum")]
    pub invert_spectrum: bool,
    /// Switch on the Hermes-Lite 2's onboard power amplifier (register `0x09`
    /// bit 19). Ignored on every other board — the bit is a Hermes-Lite
    /// repurposing of an Apollo/Alex field.
    ///
    /// **On by default**, because with it off the board keys — the T/R relay
    /// throws, the PTT line and any accessory board follow — and puts out no
    /// power at all at the antenna jack. Turn it off only to drive an external
    /// amplifier from the low-power RF1 output, which also parks the T/R relay
    /// in receive (register `0x09` bit 18) so the antenna connector stays on
    /// the receiver.
    #[serde(default = "HpsdrConfig::default_pa_enable")]
    pub pa_enable: bool,
    /// Where an HL2IOBoard on the accessory bus takes its receive signal from.
    /// Only meaningful when such a board is fitted — it is found automatically —
    /// and only worth moving off the default by an operator who has wired the
    /// board's own SMA jacks. See [`HpsdrIoRxInput`].
    #[serde(default)]
    pub io_rx_input: HpsdrIoRxInput,
    /// Crystal/TCXO error in ppm, applied to RX/TX frequency before it's sent
    /// to the board's NCO.
    #[serde(default)]
    pub ppm: f64,
    /// Which of the board's DDCs (receivers) this radio runs, 0-based as the
    /// wire counts them. A Protocol 2 board carries several independently
    /// tunable DDCs on one connection, so two radios on the same address can
    /// each take one; the transmitter (DUC) belongs to DDC 0's radio, and
    /// Protocol 1 boards have only DDC 0 here. Defaults keep every existing
    /// `radio.json` on DDC 0, exactly as before.
    #[serde(default)]
    pub ddc: u8,
    /// How far ahead of real time the engine may fill this board's transmit
    /// ring before it paces production back down, in ms. Unlike
    /// [`IcomNetConfig::tx_latency_ms`], the board holds no such buffer itself
    /// — OpenHPSDR has no such setting — so this only widens the cushion the
    /// engine leaves in the ring on *this* side of the network. Higher
    /// survives a worse link (WiFi, a VPN) at the cost of transmit-audio and
    /// PTT latency; the low default is right for a direct wired connection,
    /// where the link's own jitter is negligible. See
    /// [`HpsdrConfig::default_tx_latency_ms`].
    #[serde(default = "HpsdrConfig::default_tx_latency_ms")]
    pub tx_latency_ms: f64,

    // ── PureSignal: adaptive predistortion from the transmit sample ──
    /// Linearise the transmitter from a sample of what it actually emitted —
    /// what openHPSDR calls PureSignal (issue #283).
    ///
    /// The receiver is the feedback path. The board is commanded in duplex, so
    /// its DDC keeps running through an over; give that DDC a coupled sample of
    /// the amplifier's output and the transmitter can compare what came back
    /// with what it meant to send, and bend the next block by the inverse of
    /// the difference. Twenty-odd decibels of intermodulation improvement is
    /// the usual figure, and the amplifier keeps its power.
    ///
    /// **It needs the sample to reach the receiver**, which on a Hermes-Lite 2
    /// means a directional coupler and an attenuator into an input the T/R
    /// switch does not take away on transmit — the IO board's PureSignal jack,
    /// which is [`HpsdrIoRxInput::IoBoardPureSignal`]. With nothing coupled in
    /// the loop simply never locks and the transmitter is left exactly as it
    /// would have been: the table starts at unity and only measurement moves
    /// it.
    ///
    /// Off by default, and only ever on the radio that owns the transmitter
    /// (DDC 0).
    #[serde(default)]
    pub puresignal: bool,
    /// Steps in the predistortion table — how finely the amplifier's curve is
    /// modelled. Clamped to [`LimeAuxConfig::PS_MIN_BINS`] ..=
    /// [`LimeAuxConfig::PS_MAX_BINS`], as the same processor's other caller is.
    #[serde(default = "HpsdrConfig::default_ps_bins")]
    pub ps_bins: u8,
    /// How fast the table follows what the coupler reports, 0..1. Slow is
    /// right: this is averaging an amplifier's curve, which does not move, out
    /// of a feedback path that has noise in it.
    #[serde(default = "HpsdrConfig::default_ps_rate")]
    pub ps_rate: f32,
    /// Hold the table where it is instead of adapting — for measuring, and for
    /// an operator who has a correction they are happy with.
    #[serde(default)]
    pub ps_frozen: bool,
    /// The operator's own open-collector words, one row per band, used when
    /// [`Self::filter_board`] is [`HpsdrFilterBoard::Custom`] — see
    /// [`HpsdrOcRow`] (issue #296).
    ///
    /// Kept even while a preset is selected: switching to a preset to compare
    /// and back again must not throw the table away. A band with no row
    /// asserts nothing, which is what a fresh table is made of.
    #[serde(default)]
    pub oc_table: Vec<HpsdrOcRow>,
}

impl Default for HpsdrConfig {
    fn default() -> Self {
        HpsdrConfig {
            manual_ip: None,
            selected_ip: None,
            sample_rate_hz: 1_536_000.0,
            lna_gain_db: Self::default_lna_gain_db(),
            filter_board: HpsdrFilterBoard::None,
            invert_spectrum: Self::default_invert_spectrum(),
            pa_enable: Self::default_pa_enable(),
            io_rx_input: HpsdrIoRxInput::Radio,
            ppm: 0.0,
            ddc: 0,
            tx_latency_ms: Self::default_tx_latency_ms(),
            puresignal: false,
            ps_bins: Self::default_ps_bins(),
            ps_rate: Self::default_ps_rate(),
            ps_frozen: false,
            oc_table: Vec::new(),
        }
    }
}

impl HpsdrConfig {
    /// The open-collector outputs this configuration asks for, flattened for
    /// the protocol threads — see [`HpsdrOcPlan`] (issue #296).
    pub fn oc_plan(&self) -> HpsdrOcPlan {
        let mut plan = HpsdrOcPlan::preset(self.filter_board);
        if self.filter_board == HpsdrFilterBoard::Custom {
            for row in &self.oc_table {
                let i = row.band.index();
                plan.rx[i] = row.rx & 0x7F;
                plan.tx[i] = row.tx & 0x7F;
            }
        }
        plan
    }

    /// See [`Self::ps_bins`] — the same 32 steps the other caller of this
    /// processor starts at.
    pub fn default_ps_bins() -> u8 {
        32
    }

    /// See [`Self::ps_rate`].
    pub fn default_ps_rate() -> f32 {
        0.5
    }

    /// Range of the Hermes-Lite 2 front-end gain, in dB.
    pub const LNA_GAIN_MIN_DB: f64 = -12.0;
    pub const LNA_GAIN_MAX_DB: f64 = 48.0;
    /// Name of the RX gain element the backend exposes for that gain. Lives here
    /// rather than in `sdroxide-hpsdr` so the (wasm-safe) settings UI can address
    /// the same element without depending on the native backend crate.
    pub const LNA_GAIN_ELEMENT: &'static str = "LNA";
    /// Ppm correction, riding `SetGain` like [`RtlSdrConfig::PPM_ELEMENT`].
    pub const PPM_ELEMENT: &'static str = "PPM";

    /// Mid-scale default: sensitive enough on a quiet band without clipping the
    /// ADC on a real antenna.
    pub fn default_lna_gain_db() -> f64 {
        20.0
    }

    /// Hermes-Lite 2 boards deliver a conjugated stream, so inversion is the
    /// working default. See [`HpsdrConfig::invert_spectrum`].
    pub fn default_invert_spectrum() -> bool {
        true
    }

    /// A Hermes-Lite 2 with its PA switched off transmits nothing at the
    /// antenna jack, so the amplifier is on unless the operator says otherwise.
    /// See [`HpsdrConfig::pa_enable`].
    pub fn default_pa_enable() -> bool {
        true
    }

    /// Matches the cushion every other backend gets: enough to absorb ordinary
    /// scheduling jitter on a direct wired connection, short enough to keep
    /// transmit-audio and PTT latency unnoticeable. See
    /// [`HpsdrConfig::tx_latency_ms`].
    pub fn default_tx_latency_ms() -> f64 {
        30.0
    }

    /// Range offered in the UI. The floor keeps the round trip to the board
    /// from dominating.
    ///
    /// The ceiling is set by the ring this cushion actually lives in, which is
    /// the host's: `sdroxide-hpsdr` sizes it at `48000 * 2 * 0.5` floats
    /// rounded up to a power of two, so ~683 ms at the fixed 48 kHz transmit
    /// rate both protocols use. Asking for more than the ring can hold does
    /// not buy headroom — the ring simply saturates, `HpsdrRx::tx_write` spins
    /// waiting for room and, after 200 ms of that, drops the sample pair,
    /// which is the very glitch this setting exists to prevent. 500 ms leaves
    /// room under that limit while still covering a link far worse than the
    /// default assumes. Deliberately lower than Icom LAN's identically-named
    /// ceiling: that one sizes a buffer inside the radio, this one has to fit
    /// a buffer here.
    pub const TX_LATENCY_MS_RANGE: std::ops::RangeInclusive<f64> = 10.0..=500.0;

    /// Supported DDC sample rates (Hz) for Protocol 2 boards.
    pub const SAMPLE_RATES: [f64; 6] =
        [48_000.0, 96_000.0, 192_000.0, 384_000.0, 768_000.0, 1_536_000.0];

    /// Protocol 1 (Metis) boards top out at 384 kHz.
    pub const P1_SAMPLE_RATES: [f64; 4] = [48_000.0, 96_000.0, 192_000.0, 384_000.0];

    /// The sample rates valid for a given protocol (1 or 2).
    pub fn rates_for(protocol: u8) -> &'static [f64] {
        if protocol == 1 { &Self::P1_SAMPLE_RATES } else { &Self::SAMPLE_RATES }
    }

    /// Resolve the IP to connect to: manual override, else the persisted pick.
    /// `None` means "discover and use the first responder".
    pub fn target_ip(&self) -> Option<&str> {
        self.manual_ip.as_deref().filter(|s| !s.trim().is_empty()).or(self.selected_ip.as_deref())
    }

    /// Scale `hz` by a ppm correction.
    pub fn apply_ppm(hz: f64, ppm: f64) -> f64 {
        hz * (1.0 + ppm / 1e6)
    }
}

/// One HPSDR device found by a discovery scan. Wasm-safe so it can cross the
/// `RadioController` trait to the settings UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HpsdrDevice {
    pub ip: String,
    pub mac: String,
    /// Board name, e.g. "Hermes", "Saturn", "Hermes-Lite 2".
    pub board: String,
    /// OpenHPSDR protocol the board speaks (1 or 2).
    pub protocol: u8,
    /// Whether the board reports it is already in use by another host.
    pub in_use: bool,
}

impl HpsdrDevice {
    /// One-line label for the selection UI.
    pub fn label(&self) -> String {
        let mut s = format!("{}  {}  (P{})", self.board, self.ip, self.protocol);
        if self.in_use {
            s.push_str("  [in use]");
        }
        if !self.supported() {
            s.push_str("  [unsupported protocol]");
        }
        s
    }

    /// Whether this device can be driven by the current implementation
    /// (Protocol 1 and Protocol 2 are both supported).
    pub fn supported(&self) -> bool {
        matches!(self.protocol, 1 | 2)
    }
}

/// TCI (Transceiver Control Interface, WebSocket) backend configuration.
/// Receive is wideband IQ (sdroxide demodulates); transmit is audio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TciConfig {
    /// TCI server `host:port` (default `127.0.0.1:50001`, the ExpertSDR3 port).
    pub address: String,
    /// IQ stream sample rate in Hz (48k / 96k / 192k).
    pub iq_sample_rate_hz: f64,
    /// Which of the rig's receivers this radio runs, 0-based as the wire
    /// counts them (a SunSDR2DX has 0 and 1). Two radios on the same address
    /// share one connection, each with its own receiver; the transmitter
    /// belongs to receiver 0's radio. `#[serde(default)]` on the struct keeps
    /// every existing `radio.json` on receiver 0, exactly as before.
    pub rx: u32,
    /// How long after a `dds:` command the IQ actually arrives on the new
    /// centre, in milliseconds — what the panadapter's axis has to be held back
    /// by so a pan draws the band where it really is. See
    /// `IqSource::stream_delay_s` for what goes wrong without it.
    ///
    /// Configurable because it belongs to the rig and the link rather than to
    /// this protocol: nothing in TCI reports it (the `dds:` echo is a command
    /// acknowledgement, returning in 0.4 ms), so it can only be declared. The
    /// default is [`TciConfig::DEFAULT_STREAM_DELAY_MS`]; set it to 0 to switch
    /// the compensation off entirely.
    pub stream_delay_ms: f64,
}

impl Default for TciConfig {
    fn default() -> Self {
        TciConfig {
            address: "127.0.0.1:50001".into(),
            iq_sample_rate_hz: 192_000.0,
            rx: 0,
            stream_delay_ms: TciConfig::DEFAULT_STREAM_DELAY_MS,
        }
    }
}

impl TciConfig {
    /// IQ sample rates offered in the UI.
    pub const IQ_RATES: [f64; 3] = [48_000.0, 96_000.0, 192_000.0];

    /// Measured on a SunSDR2DX through ExpertSDR3 over the loopback interface:
    /// a `dds:` step took 109–131 ms to reach the samples at 192 kHz, 129 ms at
    /// 96 kHz and 169 ms at 48 kHz — near enough one number rather than a fixed
    /// count of samples, which is what a rig-side pipeline measured in
    /// milliseconds looks like. Only 21 ms of it was sdroxide's own ring.
    ///
    /// One rig on one host, so it is a starting point and not a constant of
    /// nature; a TCI server across a network will be slower. The cost of having
    /// it wrong is bounded either way — the axis is off by the drag rate times
    /// the *error*, where before it was off by the drag rate times the whole
    /// delay.
    ///
    /// A second ExpertSDR3 setup on another SunSDR2DX, also on loopback,
    /// measured **159 ms** at 192 kHz (2026-08-31) — 47 ms to first movement
    /// and 159 ms to settle, so the retune is a ramp rather than a step and no
    /// single delay is exact across it. That is a 50 ms spread on the same rig
    /// *model*, which is why this is a slider on the settings tab and not a
    /// constant: it has to be calibrated per setup with
    /// `cargo run --release -p sdroxide-tci --example retune_latency`, and
    /// confirmed by eye on a drag.
    pub const DEFAULT_STREAM_DELAY_MS: f64 = 130.0;
}

/// What the Icom's LAN audio stream is carrying.
///
/// The radio decides this — `SET > Connectors > LAN AF/IF Output → Output
/// Select` — and sdroxide can write the setting on a model it knows. The choice
/// is between letting the radio demodulate and doing it here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum IcomRxSource {
    /// Demodulated audio. The rig's filters, AGC and demodulator do the work
    /// and sdroxide shows a narrow audio-band panadapter beside the rig's own
    /// scope. Always available, on every model.
    #[default]
    Af,
    /// The 12 kHz IF — Icom's DRM output. sdroxide mixes it to baseband and
    /// demodulates it, which brings its own notch, noise reduction, digital
    /// modes and skimmer to bear over roughly ±12 kHz of real spectrum.
    ///
    /// Needs a 48 kHz stream; at any lower rate there is no room for the IF.
    /// How much of that ±12 kHz is genuinely there is not documented by Icom
    /// and has not been measured on hardware.
    If12k,
}

impl IcomRxSource {
    pub const ALL: [IcomRxSource; 2] = [IcomRxSource::Af, IcomRxSource::If12k];
    pub fn label(self) -> &'static str {
        match self {
            IcomRxSource::Af => "AF — the radio demodulates",
            IcomRxSource::If12k => "12 kHz IF — sdroxide demodulates",
        }
    }
    /// The value `1A 05 <lan_afif_select>` takes for this choice.
    pub fn menu_value(self) -> u8 {
        match self {
            IcomRxSource::Af => 0x00,
            IcomRxSource::If12k => 0x01,
        }
    }
}

/// How wide the radio's own scope should be swept for the full-band waterfall.
///
/// The scope is the only wide view an Icom has — there is no I/Q on any of
/// them — and the radio keeps whatever span the operator last chose on its own
/// screen, which is routinely ±5 kHz. Left at that, the full-band strip is
/// barely wider than the panadapter it sits above, which is exactly the
/// complaint RS-BA1 does not get. So sdroxide asks for a span of its own.
///
/// The values are Icom's, as its menu labels them: half widths. A radio that
/// does not have one answers `FA` and keeps the span it had.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum IcomScopeSpan {
    /// Leave the radio's own setting alone — for an operator who is watching
    /// the radio's screen as well and does not want it moved.
    Radio,
    Khz2_5,
    Khz5,
    Khz10,
    Khz25,
    Khz50,
    /// 200 kHz across. Wide enough to hold a whole HF sub-band at about 400 Hz
    /// a bin, which is why it is the default.
    #[default]
    Khz100,
    Khz250,
    Khz500,
}

impl IcomScopeSpan {
    pub const ALL: [IcomScopeSpan; 9] = [
        IcomScopeSpan::Radio,
        IcomScopeSpan::Khz2_5,
        IcomScopeSpan::Khz5,
        IcomScopeSpan::Khz10,
        IcomScopeSpan::Khz25,
        IcomScopeSpan::Khz50,
        IcomScopeSpan::Khz100,
        IcomScopeSpan::Khz250,
        IcomScopeSpan::Khz500,
    ];

    /// The half width to command, or `None` to leave the radio as it is.
    pub fn half_span_hz(self) -> Option<f64> {
        Some(match self {
            IcomScopeSpan::Radio => return None,
            IcomScopeSpan::Khz2_5 => 2_500.0,
            IcomScopeSpan::Khz5 => 5_000.0,
            IcomScopeSpan::Khz10 => 10_000.0,
            IcomScopeSpan::Khz25 => 25_000.0,
            IcomScopeSpan::Khz50 => 50_000.0,
            IcomScopeSpan::Khz100 => 100_000.0,
            IcomScopeSpan::Khz250 => 250_000.0,
            IcomScopeSpan::Khz500 => 500_000.0,
        })
    }

    /// Labelled by the width the operator actually sees, with Icom's own ±
    /// figure after it — the menu on the radio uses the second form.
    pub fn label(self) -> &'static str {
        match self {
            IcomScopeSpan::Radio => "As set on the radio",
            IcomScopeSpan::Khz2_5 => "5 kHz (±2.5k)",
            IcomScopeSpan::Khz5 => "10 kHz (±5k)",
            IcomScopeSpan::Khz10 => "20 kHz (±10k)",
            IcomScopeSpan::Khz25 => "50 kHz (±25k)",
            IcomScopeSpan::Khz50 => "100 kHz (±50k)",
            IcomScopeSpan::Khz100 => "200 kHz (±100k)",
            IcomScopeSpan::Khz250 => "500 kHz (±250k)",
            IcomScopeSpan::Khz500 => "1 MHz (±500k)",
        }
    }
}

/// Icom LAN backend configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct IcomNetConfig {
    /// Hostname or IP of the radio. There is no discovery — an Icom does not
    /// announce itself — so this is always typed in.
    pub address: String,
    /// Control port; the CI-V and audio ports are negotiated, not configured.
    pub control_port: u16,
    /// The network user set on the radio, under `SET > Network`.
    pub username: String,
    /// Stored in the clear in `radio.json`, as every other service credential
    /// in this program is: the radio requires a reversible obfuscation of it on
    /// the wire, so a secret store here would protect nothing.
    pub password: String,
    pub rx_source: IcomRxSource,
    /// Where CW the operator sends comes from. Same choice as a serial CAT rig
    /// and for the same reason: a rig *in* CW ignores the audio we send it and
    /// keys its own transmitter, so CW has to go as text to its keyer.
    pub cw_keying: CwKeying,
    /// Audio sample rate to ask the radio for.
    pub sample_rate_hz: u32,
    /// Displayed panadapter bandwidth in AF mode (Hz), as for a CAT rig.
    ///
    /// Only reaches the display where the audio FFT is the panadapter: the
    /// digital modes, and a session with no scope. Otherwise the window is the
    /// radio's own sweep and [`Self::scope_span`] sets its width.
    pub audio_bw_hz: f64,
    /// How much audio the radio should buffer before modulating, in ms. Higher
    /// survives a worse network at the cost of transmit latency.
    pub tx_latency_ms: u32,
    /// Pick a radio by CI-V address rather than taking the first offered. Only
    /// an RS-BA1 server PC ever presents more than one.
    pub civ_address_override: Option<u8>,
    /// Switch the radio's modulation input to LAN when the session opens, so
    /// transmit audio is heard. Off for a model whose menu numbering is not in
    /// the table — see `sdroxide_icomnet::protocol::MODELS`.
    pub set_mod_input_on_open: bool,
    /// Ask the radio to stream its spectrum scope.
    ///
    /// Drawn in the full-band strip always, and on the AF path in the main
    /// panadapter as well — there is no I/Q there for the main lane to show, so
    /// the scope is the only picture of the band the session has.
    pub scope: bool,
    /// How wide to sweep that scope. The radio's own setting is usually far
    /// narrower than the band view this lane exists to give.
    pub scope_span: IcomScopeSpan,
    /// Whether to mirror the 12 kHz IF about its centre on the way in, or
    /// `None` to follow whatever the model is known to do
    /// (`sdroxide_icomnet::protocol::Model::if_inverted`).
    ///
    /// Only reaches the IF path — the AF path is audio the radio has already
    /// demodulated, and there is nothing there to mirror. It exists because the
    /// consequence is unmistakable and the cause is not: a mirrored IF puts SSB
    /// on the opposite sideband, so the operator ends up selecting USB on 40 m
    /// and LSB on 20 m to make anyone intelligible, with the radio's own mode
    /// display agreeing with sdroxide the whole time.
    ///
    /// Left as an override rather than a plain switch because the model table
    /// already knows the answer for the radios anybody has reported, and a
    /// setting that has to be found before the audio works is not a fix.
    pub invert_if: Option<bool>,
}

impl Default for IcomNetConfig {
    fn default() -> Self {
        IcomNetConfig {
            address: String::new(),
            control_port: 50_001,
            username: String::new(),
            password: String::new(),
            rx_source: IcomRxSource::default(),
            cw_keying: CwKeying::default(),
            sample_rate_hz: 48_000,
            audio_bw_hz: 4000.0,
            tx_latency_ms: 150,
            civ_address_override: None,
            set_mod_input_on_open: true,
            scope: true,
            scope_span: IcomScopeSpan::default(),
            invert_if: None,
        }
    }
}

impl IcomNetConfig {
    /// Audio rates an Icom offers over the network.
    pub const SAMPLE_RATES: [u32; 4] = [8_000, 16_000, 24_000, 48_000];

    /// Whether the 12 kHz IF can be used at the configured rate. A 12 kHz IF
    /// needs the whole of a 48 kHz stream; below that its centre is above
    /// Nyquist and there is nothing to recover.
    pub fn if_mode_usable(&self) -> bool {
        self.sample_rate_hz >= 48_000
    }

    /// What the source actually does, given what the operator asked for and
    /// what the rate allows.
    pub fn effective_rx_source(&self) -> IcomRxSource {
        if self.rx_source == IcomRxSource::If12k && !self.if_mode_usable() {
            IcomRxSource::Af
        } else {
            self.rx_source
        }
    }
}

/// SmartSDR (FlexRadio) backend configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SmartSdrConfig {
    /// Radio address as `host[:port]`. Empty means "use the discovered radio",
    /// which is the normal case on a LAN — a FlexRadio announces itself.
    pub address: String,
    /// IP of the radio picked from a discovery scan (persisted selection).
    pub selected_ip: Option<String>,
    /// DAX IQ stream rate in Hz. 192 kHz is the radio's maximum, and so this
    /// backend's widest span.
    pub iq_sample_rate_hz: f64,
    /// Which of the radio's four DAX IQ channels to claim. Change it only when
    /// something else on the network is already using channel 1.
    pub iq_channel: u32,
    /// Station name reported to the radio and shown against our session in
    /// SmartSDR's client list.
    pub station: String,
    /// GUI client id to register under, if the operator has fixed one.
    ///
    /// The radio remembers a GUI client by this id and restores its slices and
    /// panadapters to it, so a stable one is worth having. Empty — the default —
    /// derives it from [`Self::station`], which is stable but *not* unique:
    /// every sdroxide that never renamed its station derives the same id, and a
    /// radio resolves a duplicate id by evicting whoever had it first. The
    /// backend detects that on the wire and falls back to a per-session id;
    /// setting a UUID of your own here is how to keep the session restore as
    /// well. Any string the radio accepts will do, but SmartSDR expects a UUID.
    pub gui_client_id: String,
    /// Largest datagram the radio may send, in bytes. FlexLib's default is 1450.
    /// Lower it on a path with a smaller MTU — a VPN or a tunnel — where
    /// fragmented VITA-49 packets are dropped and the spectrum simply never
    /// arrives.
    pub network_mtu: u32,
}

impl Default for SmartSdrConfig {
    fn default() -> Self {
        SmartSdrConfig {
            address: String::new(),
            selected_ip: None,
            iq_sample_rate_hz: 192_000.0,
            iq_channel: 1,
            station: "sdroxide".into(),
            gui_client_id: String::new(),
            network_mtu: 1450,
        }
    }
}

impl SmartSdrConfig {
    /// IQ sample rates a FLEX will deliver over DAX.
    pub const IQ_RATES: [f64; 4] = [24_000.0, 48_000.0, 96_000.0, 192_000.0];
    /// DAX IQ channels the radio provides.
    pub const IQ_CHANNELS: [u32; 4] = [1, 2, 3, 4];

    /// The address to connect to: the manual entry, else the discovered
    /// selection, else nothing.
    pub fn target(&self) -> Option<&str> {
        let manual = self.address.trim();
        if !manual.is_empty() {
            return Some(manual);
        }
        self.selected_ip.as_deref().map(str::trim).filter(|s| !s.is_empty())
    }
}

/// A FlexRadio found by a discovery scan, for the selection UI.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SmartSdrDevice {
    pub ip: String,
    pub port: u16,
    pub model: String,
    pub serial: String,
    pub nickname: String,
    pub version: String,
    /// Whether a GUI client can join: nobody else has it, or multiFLEX is on.
    pub joinable: bool,
    /// Station names of GUI clients already connected.
    pub gui_clients: Vec<String>,
}

impl SmartSdrDevice {
    /// One-line label for the selection UI.
    pub fn label(&self) -> String {
        let name = match (self.nickname.is_empty(), self.model.is_empty()) {
            (false, false) => format!("{} ({})", self.nickname, self.model),
            (false, true) => self.nickname.clone(),
            (true, false) => self.model.clone(),
            (true, true) => "FlexRadio".to_string(),
        };
        let mut s = format!("{name}  {}", self.ip);
        if !self.version.is_empty() {
            s.push_str(&format!("  v{}", self.version));
        }
        if !self.gui_clients.is_empty() {
            s.push_str(&format!("  [in use: {}]", self.gui_clients.join(", ")));
        }
        if !self.joinable {
            s.push_str("  [multiFLEX off]");
        }
        s
    }
}

/// How an RTL-SDR reaches HF. The R82xx tuner itself starts at 24 MHz, so
/// anything below that needs help from the dongle's hardware.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RtlSdrHfMode {
    /// Tuner only — nothing below 24 MHz.
    Off,
    /// Use whatever this dongle has: the V4's built-in upconverter, or
    /// direct sampling on a V3. Switched automatically at the crossover.
    #[default]
    Auto,
    /// Force direct sampling on the ADC's Q branch (the V3's HF port). Has no
    /// meaning on a Blog V4, which upconverts instead.
    DirectQ,
}

impl RtlSdrHfMode {
    pub const ALL: [RtlSdrHfMode; 3] =
        [RtlSdrHfMode::Auto, RtlSdrHfMode::Off, RtlSdrHfMode::DirectQ];

    /// Paired with [`RtlSdrHfMode::from_code`] so the mode can ride the
    /// `HFMODE` pseudo-element; keep the two in step.
    pub fn code(self) -> u8 {
        match self {
            RtlSdrHfMode::Off => 0,
            RtlSdrHfMode::Auto => 1,
            RtlSdrHfMode::DirectQ => 2,
        }
    }

    pub fn from_code(code: u8) -> RtlSdrHfMode {
        match code {
            0 => RtlSdrHfMode::Off,
            2 => RtlSdrHfMode::DirectQ,
            _ => RtlSdrHfMode::Auto,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            RtlSdrHfMode::Off => "Off (tuner only, 24 MHz up)",
            RtlSdrHfMode::Auto => "Automatic",
            RtlSdrHfMode::DirectQ => "Direct sampling (Q branch)",
        }
    }
}

/// Which automatic gain loops to enable. The tuner AGC lives in the R82xx; the
/// RTL AGC is the demod's digital one. They are independent and can both run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RtlSdrAgc {
    /// Manual tuner gain, no automatic loops — the setting for measurement and
    /// for weak-signal digital modes.
    #[default]
    Manual,
    Tuner,
    Rtl,
    Both,
}

impl RtlSdrAgc {
    pub const ALL: [RtlSdrAgc; 4] =
        [RtlSdrAgc::Manual, RtlSdrAgc::Tuner, RtlSdrAgc::Rtl, RtlSdrAgc::Both];
    pub fn label(self) -> &'static str {
        match self {
            RtlSdrAgc::Manual => "Manual (no AGC)",
            RtlSdrAgc::Tuner => "Tuner AGC",
            RtlSdrAgc::Rtl => "RTL digital AGC",
            RtlSdrAgc::Both => "Tuner + RTL AGC",
        }
    }

    /// Whether the R82xx runs its own LNA/mixer gain loop.
    pub fn tuner_auto(self) -> bool {
        matches!(self, RtlSdrAgc::Tuner | RtlSdrAgc::Both)
    }

    /// Whether the demod's digital AGC runs.
    pub fn rtl_auto(self) -> bool {
        matches!(self, RtlSdrAgc::Rtl | RtlSdrAgc::Both)
    }

    /// AGC mode as a number, so it can ride the existing `SetGain` command on
    /// the `AGC` pseudo-element instead of needing a new `Command` variant.
    /// Paired with [`RtlSdrAgc::from_code`]; keep the two in step.
    pub fn code(self) -> u8 {
        match self {
            RtlSdrAgc::Manual => 0,
            RtlSdrAgc::Tuner => 1,
            RtlSdrAgc::Rtl => 2,
            RtlSdrAgc::Both => 3,
        }
    }

    pub fn from_code(code: u8) -> RtlSdrAgc {
        match code {
            1 => RtlSdrAgc::Tuner,
            2 => RtlSdrAgc::Rtl,
            3 => RtlSdrAgc::Both,
            _ => RtlSdrAgc::Manual,
        }
    }
}

/// RTL-SDR (RTL2832U over USB) backend configuration. Receive only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RtlSdrConfig {
    /// USB serial of the dongle to open. `None` = the first one found. Serial
    /// rather than an index because bus position changes on every replug, and
    /// a persisted index would attach to the wrong dongle.
    pub serial: Option<String>,
    /// Sample rate in Hz. The resampler only reaches 225–300 kHz and
    /// 900 kHz–3.2 MHz; everything between is rejected by the hardware.
    pub sample_rate_hz: f64,
    /// Crystal error in parts per million. Read it off the `clock error`
    /// line that `RUST_LOG=sdroxide_rtlsdr=debug` prints once the stream runs.
    pub ppm: i32,
    /// Tuner gain in dB when AGC is off. Snapped to the nearest step the
    /// hardware can actually produce.
    pub tuner_gain_db: f64,
    pub agc: RtlSdrAgc,
    pub hf_mode: RtlSdrHfMode,
    /// Bias tee: ~4.5 V DC on the antenna coax for a remote LNA. Off by
    /// default, and turned off again on a clean shutdown — it will damage a
    /// transceiver or anything DC-shorted on the other end of the cable.
    pub bias_tee: bool,
    /// Remove the DC spike on the centre of the span and the mirror image, in
    /// DSP. On by default: both are artefacts of the dongle rather than
    /// anything on the antenna, and the R820T has no offset-tuning mode to
    /// move the LO out of the way instead.
    pub iq_correction: bool,
    /// Bulk transfers kept in flight (advanced). The default gives ~53 ms of
    /// hardware-side buffering at 2.4 Msps, twice the worst-case retune stall.
    pub transfers: u8,
    /// Size of each bulk transfer in KiB (advanced). Must stay a multiple of
    /// the endpoint's 512-byte packet.
    pub transfer_kib: u16,
}

impl Default for RtlSdrConfig {
    fn default() -> Self {
        RtlSdrConfig {
            serial: None,
            sample_rate_hz: 2_400_000.0,
            ppm: 0,
            tuner_gain_db: 30.0,
            agc: RtlSdrAgc::Manual,
            hf_mode: RtlSdrHfMode::Auto,
            bias_tee: false,
            iq_correction: true,
            transfers: 16,
            transfer_kib: 16,
        }
    }
}

impl RtlSdrConfig {
    /// Gain element names the backend exposes. They live here rather than in
    /// `sdroxide-rtlsdr` so the (wasm-safe) settings UI can address them
    /// without depending on the native backend crate — same reason as
    /// [`HpsdrConfig::LNA_GAIN_ELEMENT`].
    pub const TUNER_GAIN_ELEMENT: &'static str = "TUNER";
    pub const IF_GAIN_ELEMENT: &'static str = "IF";
    /// Pseudo-elements carrying settings that are not gains at all.
    ///
    /// These ride the existing `SetGain` command so that adding this backend
    /// needs no new `Command` variant, no `DeviceCaps` field and no engine
    /// change for five settings only one backend has. They are deliberately
    /// absent from `DeviceCaps::gains`, so nothing renders them as sliders —
    /// the RTL-SDR settings panel drives them directly. The encodings live
    /// beside the enums they carry ([`RtlSdrAgc::code`], `HfMode as u8`) so
    /// the two ends cannot drift.
    pub const AGC_ELEMENT: &'static str = "AGC";
    pub const PPM_ELEMENT: &'static str = "PPM";
    pub const HF_MODE_ELEMENT: &'static str = "HFMODE";
    pub const BIAS_TEE_ELEMENT: &'static str = "BIASTEE";
    pub const IQ_CORRECTION_ELEMENT: &'static str = "IQCORR";

    /// Sample rates offered in the UI. All lie inside the resampler's upper
    /// window except 250 kHz, which is in the lower one. 3.2 Msps is offered
    /// but drops samples on most hosts.
    pub const SAMPLE_RATES: [f64; 9] = [
        250_000.0,
        960_000.0,
        1_024_000.0,
        1_200_000.0,
        1_536_000.0,
        1_800_000.0,
        2_048_000.0,
        2_400_000.0,
        3_200_000.0,
    ];

    /// Maximum R82xx tuner gain, in dB (the last entry of the gain table).
    pub const GAIN_MAX_DB: f64 = 49.6;
}

/// An RTL-SDR published over the network by `rtl_tcp` (osmocom's, the
/// rtl-sdr-blog fork's, or any of the several servers that speak the same
/// protocol). Receive only.
///
/// The knobs are deliberately the same ones as [`RtlSdrConfig`], and they ride
/// the same pseudo-elements, because it is the same radio — the difference is
/// only which side of the link the register writes happen on. What is missing
/// here is everything that describes *this* machine's USB bus: there is no
/// serial to pin (the server chose the dongle when it started) and no transfer
/// geometry (the server owns the transfers; TCP does its own buffering).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RtlTcpConfig {
    /// `host` or `host:port` of the `rtl_tcp` server. The port may be left off
    /// and defaults to 1234, which is what `rtl_tcp` listens on unless told
    /// otherwise — see [`Self::endpoint`].
    pub address: String,
    /// Sample rate in Hz, requested of the server. Same resampler limits as
    /// the USB backend, since it is the same silicon on the far end.
    pub sample_rate_hz: f64,
    /// Crystal error in parts per million, applied by the *server* to its
    /// dongle. This is a property of the far-end hardware, not of the link.
    pub ppm: i32,
    /// Tuner gain in dB when AGC is off. Sent in tenths of a dB, which is the
    /// unit the protocol carries; the server snaps it to the nearest step its
    /// tuner can produce and tells us nothing about the result.
    pub tuner_gain_db: f64,
    pub agc: RtlSdrAgc,
    /// How the far end reaches HF. `Auto` and `DirectQ` send the protocol's
    /// direct-sampling command; a Blog V4 on the far end needs neither,
    /// because the blog fork's server upconverts inside its own tuning call.
    pub hf_mode: RtlSdrHfMode,
    /// Bias tee on the *remote* dongle: ~4.5 V DC on an antenna coax that is
    /// wherever the server is, which may be a mast a hundred metres away and
    /// out of sight. Off by default, and switched off again when the stream
    /// closes cleanly.
    ///
    /// Older servers do not implement the command at all and answer it with
    /// silence rather than an error — the protocol has no replies — so a
    /// bias tee that does not come on is not necessarily this end's fault.
    pub bias_tee: bool,
    /// Remove the DC spike and mirror image in DSP, exactly as on the USB
    /// backend: these are artefacts of the dongle, so they arrive over the
    /// network along with everything else.
    pub iq_correction: bool,

    // --- rsp_tcp only ----------------------------------------------------
    //
    // An SDRplay server greets exactly like a dongle, so these are only
    // reachable when it was started with `-E` and sent a capability block. The
    // settings tab hides them until one arrives, and each is gated on the bit
    // the server set for it — an RSP1A has no antenna switch, and offering one
    // would be a control that silently does nothing.
    /// Antenna input: 0 = A, 1 = B, 2 = high impedance.
    pub rsp_antenna: u8,
    /// LNA state — an index, not a dB figure. Which one is "least gain" depends
    /// on the model *and* the band, which is why the native SDRplay interface
    /// exposes a step control rather than a slider and this does the same.
    pub rsp_lna_state: u8,
    /// IF gain **reduction** in dB, so a bigger number is less signal. The
    /// server reports the legal range in its capability block.
    pub rsp_if_gain_reduction: u8,
    /// The RSP's own AGC, and the level it aims for in dBfs (negative).
    pub rsp_agc: bool,
    pub rsp_agc_setpoint: i32,
    /// Notch filters, as the protocol's bitmask: AM 1, broadcast 2, DAB 4,
    /// RF 8.
    pub rsp_notch: u8,
    /// Reference clock output on the RSP2/duo.
    pub rsp_ref_out: bool,
}

impl Default for RtlTcpConfig {
    fn default() -> Self {
        RtlTcpConfig {
            address: format!("127.0.0.1:{}", RtlTcpConfig::DEFAULT_PORT),
            // Deliberately lower than the USB backend's 2.4 Msps default: this
            // one has to fit down a network link, and 2.4 Msps is 38 Mbit/s of
            // uncompressed 8-bit I/Q. 1.024 Msps is 16 Mbit/s, which survives
            // WiFi, and the operator can raise it on a wired link.
            sample_rate_hz: 1_024_000.0,
            ppm: 0,
            tuner_gain_db: 30.0,
            rsp_antenna: 0,
            rsp_lna_state: 0,
            // Mid-scale on every model's range, and roughly where SDRplay's own
            // tools open.
            rsp_if_gain_reduction: 40,
            rsp_agc: true,
            rsp_agc_setpoint: -30,
            rsp_notch: 0,
            rsp_ref_out: false,
            agc: RtlSdrAgc::Manual,
            hf_mode: RtlSdrHfMode::Auto,
            bias_tee: false,
            iq_correction: true,
        }
    }
}

impl RtlTcpConfig {
    /// The port `rtl_tcp` listens on when it is not given `-p`.
    pub const DEFAULT_PORT: u16 = 1234;

    /// Pseudo-elements for the `rsp_tcp` controls, riding `SetGain` the way the
    /// RTL-SDR's own switches do — see [`RtlSdrConfig::AGC_ELEMENT`]. Only
    /// reachable against an SDRplay server in extended mode; a plain `rtl_tcp`
    /// server ignores the opcodes, which is harmless because the protocol has
    /// no replies and never did.
    pub const RSP_ANTENNA_ELEMENT: &'static str = "RSPANT";
    pub const RSP_LNA_STATE_ELEMENT: &'static str = "RSPLNA";
    pub const RSP_IFGR_ELEMENT: &'static str = "RSPIFGR";
    pub const RSP_AGC_ELEMENT: &'static str = "RSPAGC";
    pub const RSP_AGC_SETPOINT_ELEMENT: &'static str = "RSPAGCSP";
    pub const RSP_NOTCH_ELEMENT: &'static str = "RSPNOTCH";
    pub const RSP_REF_OUT_ELEMENT: &'static str = "RSPREFOUT";

    /// Notch-filter bits, as the `rsp_tcp` protocol packs them into one
    /// argument. Here rather than in the driver because the settings tab builds
    /// the mask and is shared with the wasm client, which cannot see
    /// `sdroxide-rtlsdr`; the driver passes the mask through without looking at
    /// the bits.
    pub const RSP_NOTCH_AM: u8 = 1 << 0;
    pub const RSP_NOTCH_BROADCAST: u8 = 1 << 1;
    pub const RSP_NOTCH_DAB: u8 = 1 << 2;
    pub const RSP_NOTCH_RF: u8 = 1 << 3;

    /// The configured address as `host:port`, supplying the default port when
    /// the operator typed only a host.
    ///
    /// "No colon means no port" is not quite enough on its own: an IPv6
    /// literal is all colons, and is written in brackets exactly so that a
    /// port can be told apart from an address word. So a bracketed literal
    /// with nothing after the bracket needs the port too.
    pub fn endpoint(&self) -> String {
        let a = self.address.trim();
        let has_port = match a.rfind(']') {
            // `[::1]:1234` — a port only if something follows the bracket.
            Some(close) => a[close + 1..].starts_with(':'),
            None => a.contains(':'),
        };
        if has_port { a.to_string() } else { format!("{a}:{}", Self::DEFAULT_PORT) }
    }

    /// Sample rates offered in the UI, and how much of a link each one asks
    /// for. Same list as [`RtlSdrConfig::SAMPLE_RATES`] — it is the same
    /// resampler — but here the number that decides is the second one.
    pub fn link_mbit(rate_hz: f64) -> f64 {
        // Two bytes per complex sample, eight bits to the byte.
        rate_hz * 2.0 * 8.0 / 1e6
    }
}

/// Sample format the server sends I/Q in.
///
/// The protocol also defines a 24-bit integer format, which is deliberately
/// absent: no open client decodes it, the servers that offer it are rare, and
/// a format guessed at rather than implemented would arrive as noise. A server
/// that *forces* it is refused at connect, with the reason said out loud,
/// rather than being fed to a decoder that would misread it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SpyServerFormat {
    /// 8-bit unsigned, two bytes a complex sample. Half the bandwidth of the
    /// 16-bit format and the reason SpyServer works over a domestic uplink at
    /// all.
    #[default]
    Uint8,
    /// 16-bit signed. Worth it on a receiver whose ADC has range to keep — an
    /// Airspy HF+ is 18-bit internally — and only on a link that can carry it.
    Int16,
    /// 32-bit float. Twice the bandwidth of 16-bit for no more information
    /// than the ADC had; offered because some servers force it.
    Float32,
}

impl SpyServerFormat {
    pub const ALL: [SpyServerFormat; 3] =
        [SpyServerFormat::Uint8, SpyServerFormat::Int16, SpyServerFormat::Float32];

    pub fn label(self) -> &'static str {
        match self {
            SpyServerFormat::Uint8 => "8-bit (lowest bandwidth)",
            SpyServerFormat::Int16 => "16-bit",
            SpyServerFormat::Float32 => "32-bit float",
        }
    }

    /// Bytes per complex sample on the wire — what decides the link cost.
    pub fn bytes_per_sample(self) -> usize {
        match self {
            SpyServerFormat::Uint8 => 2,
            SpyServerFormat::Int16 => 4,
            SpyServerFormat::Float32 => 8,
        }
    }

    /// The wire value for `IQ_FORMAT`. Paired with [`Self::from_wire`]; keep
    /// the two in step.
    pub fn wire(self) -> u32 {
        match self {
            SpyServerFormat::Uint8 => 1,
            SpyServerFormat::Int16 => 2,
            SpyServerFormat::Float32 => 4,
        }
    }

    /// A server's `ForcedIQFormat`, when it is one this end can decode.
    /// `None` covers "not forced" (0) and the two formats deliberately absent
    /// above, which the caller reports rather than substitutes.
    pub fn from_wire(v: u32) -> Option<SpyServerFormat> {
        match v {
            1 => Some(SpyServerFormat::Uint8),
            2 => Some(SpyServerFormat::Int16),
            4 => Some(SpyServerFormat::Float32),
            _ => None,
        }
    }

    /// Format as a number, so it can ride the existing `SetGain` command on a
    /// pseudo-element. Paired with [`Self::from_code`].
    pub fn code(self) -> u8 {
        match self {
            SpyServerFormat::Uint8 => 0,
            SpyServerFormat::Int16 => 1,
            SpyServerFormat::Float32 => 2,
        }
    }

    pub fn from_code(code: u8) -> SpyServerFormat {
        match code {
            1 => SpyServerFormat::Int16,
            2 => SpyServerFormat::Float32,
            _ => SpyServerFormat::Uint8,
        }
    }
}

/// A receiver published by a SpyServer, in either of the two interfaces that
/// reach one ([`Backend::SpyServer`] and [`Backend::SpyServerVfo`]).
///
/// One type for both, because the server, the handshake and every control
/// below are the same; only which streams are asked for differs, and that is
/// the interface the operator picked rather than anything stored here.
///
/// Almost nothing is a frequency or a rate in Hz. The server publishes its own
/// ladder — `MaximumSampleRate / 2ⁿ` — which is a property of the far end and
/// of the receiver behind it, and which nothing on this side can know before
/// connecting. So what is stored is the *stage*, which is also what the wire
/// carries, and which stays meaningful when the same settings are pointed at a
/// different server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SpyServerConfig {
    /// `host` or `host:port` of the server. The port may be left off and
    /// defaults to 5555, which is what `spyserver` listens on unless its
    /// config says otherwise — see [`Self::endpoint`].
    pub address: String,
    /// I/Q decimation stage, or [`Self::AUTO_DECIMATION`] to let the program
    /// pick the stage nearest the interface's target rate once the server has
    /// said what it offers.
    pub iq_decimation: i32,
    pub iq_format: SpyServerFormat,
    /// The server's gain stage, as an **index**, not a dB figure. What each
    /// index means is the far-end receiver's business and the protocol never
    /// says; see [`Self::GAIN_ELEMENT`].
    pub gain_index: u32,
    /// Let the program compute the digital gain from the device type, the gain
    /// index and the decimation stage, the way every other client does.
    /// Switching this off exposes [`Self::digital_gain_db`].
    pub auto_digital_gain: bool,
    pub digital_gain_db: f64,
    /// Ask the server for its FFT stream, which feeds the full-band strip.
    ///
    /// On by default in both interfaces: a 2048-bin frame is 2 KB and arrives
    /// a dozen or so times a second, which is a rounding error beside the I/Q
    /// even at 8-bit, and it buys a view of the whole band rather than of the
    /// slice being received. It is the *only* band view in the VFO interface.
    pub fft_enabled: bool,
    /// FFT decimation stage. `0` is the device's full bandwidth, which is the
    /// widest view there is and the default.
    pub fft_decimation: u32,
    /// Top of the FFT's dB window, and how far down from it the scale reaches.
    /// The server quantises its bins into this window before sending them, so
    /// these decide the resolution of what arrives, not just how it is drawn.
    pub fft_db_offset: f64,
    pub fft_db_range: f64,
    /// Remove the DC spike and mirror image in DSP. Whether the receiver on
    /// the far end needs it depends on what it is — an Airspy HF+ does not, an
    /// RTL-SDR does — and the protocol does not say, so it is left to the
    /// operator with the usual default of on.
    pub iq_correction: bool,
}

impl Default for SpyServerConfig {
    fn default() -> Self {
        SpyServerConfig {
            address: format!("127.0.0.1:{}", SpyServerConfig::DEFAULT_PORT),
            iq_decimation: SpyServerConfig::AUTO_DECIMATION,
            iq_format: SpyServerFormat::Uint8,
            gain_index: 0,
            auto_digital_gain: true,
            digital_gain_db: 20.0,
            fft_enabled: true,
            fft_decimation: 0,
            // The protocol's own widest window, and what every reference client
            // opens with. Auto-levelling in the engine sets what is actually
            // *drawn*; this only decides how finely the server quantises.
            fft_db_offset: 0.0,
            fft_db_range: SpyServerConfig::FFT_DB_RANGE_MAX,
            iq_correction: true,
        }
    }
}

impl SpyServerConfig {
    /// The port `spyserver` listens on out of the box.
    pub const DEFAULT_PORT: u16 = 5555;

    /// [`Self::iq_decimation`] meaning "whichever stage is nearest the target
    /// rate for this interface". Negative so it cannot collide with a stage.
    pub const AUTO_DECIMATION: i32 = -1;

    /// What `AUTO_DECIMATION` aims for in the wideband interface: about a
    /// megasample, which is 16 Mbit/s at 8-bit — a wired LAN or a good WiFi
    /// hop — and wide enough to be worth calling a panadapter.
    pub const WIDEBAND_TARGET_RATE_HZ: f64 = 1_000_000.0;

    /// The same for the VFO interface. 96 kHz is 1.5 Mbit/s at 8-bit, carries
    /// every mode this program demodulates including the wide FM broadcast
    /// skirts, and fits down a cellular uplink with room to spare.
    pub const VFO_TARGET_RATE_HZ: f64 = 96_000.0;

    /// Bounds the protocol states for the FFT window and the bin count. Here
    /// rather than in the driver because the settings tab clamps against them
    /// and is shared with the wasm client, which cannot see
    /// `sdroxide-spyserver`.
    pub const FFT_DB_RANGE_MIN: f64 = 10.0;
    pub const FFT_DB_RANGE_MAX: f64 = 150.0;
    pub const FFT_DB_OFFSET_MIN: f64 = -100.0;
    pub const FFT_DB_OFFSET_MAX: f64 = 100.0;
    pub const DISPLAY_PIXELS_MIN: u32 = 100;
    pub const DISPLAY_PIXELS_MAX: u32 = 1 << 15;

    /// Pseudo-elements, riding `SetGain` the way the RTL-SDR's switches do —
    /// see [`RtlSdrConfig::AGC_ELEMENT`].
    ///
    /// `SPYGAIN` is the odd one: it carries an **index** into the far-end
    /// receiver's gain table, not a number of dB, and `GainElement` has no
    /// other field to put it in. The same is already true of the SDRplay
    /// backend's LNA state. What an index means depends on the model and on
    /// the band, so no dB mapping is invented here — the settings tab says so
    /// instead.
    pub const GAIN_ELEMENT: &'static str = "SPYGAIN";
    pub const AUTO_DIGITAL_GAIN_ELEMENT: &'static str = "SPYAUTOG";
    pub const DIGITAL_GAIN_ELEMENT: &'static str = "SPYDGAIN";
    pub const FFT_ENABLED_ELEMENT: &'static str = "SPYFFT";
    pub const FFT_DECIMATION_ELEMENT: &'static str = "SPYFFTDEC";
    pub const FFT_DB_OFFSET_ELEMENT: &'static str = "SPYFFTOFF";
    pub const FFT_DB_RANGE_ELEMENT: &'static str = "SPYFFTRNG";
    pub const IQ_CORRECTION_ELEMENT: &'static str = "IQCORR";

    /// The configured address as `host:port`, supplying the default port when
    /// the operator typed only a host. Same rule as
    /// [`RtlTcpConfig::endpoint`], including the bracketed-IPv6 case.
    pub fn endpoint(&self) -> String {
        let a = self.address.trim();
        let has_port = match a.rfind(']') {
            Some(close) => a[close + 1..].starts_with(':'),
            None => a.contains(':'),
        };
        if has_port { a.to_string() } else { format!("{a}:{}", Self::DEFAULT_PORT) }
    }
}

/// A KiwiSDR or Web-888, reached over its web port. Receive only.
///
/// The receiver runs the whole front end: it decides the sample rate, it owns
/// the AGC, and what arrives is one of its eight (or four) user channels taken
/// as I/Q rather than as audio. So there is very little to configure here
/// compared with a radio on a bus — an address, who to say we are, and how much
/// of the link to spend on the waterfall.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct KiwiConfig {
    /// `host` or `host:port`. The port may be left off and defaults to 8073,
    /// which is what a Kiwi listens on out of the box — but note that the
    /// project's own reverse proxy (`*.proxy.kiwisdr.com`, which is how nearly
    /// half the public receivers are reached) answers on 80 instead, so an
    /// address picked from the directory always carries its port explicitly.
    pub address: String,
    /// The *user* password, where the operator has set one. Not the admin
    /// password, and usually empty: most public receivers ask for nothing.
    pub password: String,
    /// The name this end announces with `SET ident_user`, which the receiver's
    /// owner and its other listeners can see.
    ///
    /// Empty means "the station callsign, or `sdroxide` when none is set" —
    /// resolved where the source is opened, because this crate cannot reach the
    /// configuration. Identifying is the network's convention rather than a
    /// requirement, and some operators do block clients that will not.
    pub ident: String,
    /// Ask for the receiver's waterfall as well as its I/Q, to fill the
    /// full-band strip.
    ///
    /// On by default. It is a second WebSocket carrying about 20 kB/s at the
    /// default speed, against the I/Q's 44 kB/s — and without it the only band
    /// view is the ~12 kHz the I/Q covers, which is not enough to tune by.
    pub wide_lane: bool,
    /// Waterfall frame rate, 1 (slowest) to 4. The receiver caps this at its
    /// own `wf_fps_max`, which was 23 fps on the one this was measured against.
    pub wf_speed: u8,
    /// The *receiver's* AGC, which sits ahead of the I/Q and so ahead of
    /// everything this program does.
    ///
    /// On by default, which is unlike every bus-attached backend here and is
    /// deliberate. Measured on a live receiver, the manual gain was not
    /// monotonic across its range and its top end clipped the I/Q at full
    /// scale, while the AGC held about -24 dBFS RMS with 7 dB of headroom. An
    /// AGC on the far side of the link is the lesser evil.
    ///
    /// It does mean the I/Q amplitude is not a signal level, which is why the
    /// S-meter is read from the receiver's own per-frame figure instead.
    pub agc: bool,
    /// Fixed gain when [`Self::agc`] is off, on the receiver's own 0-90 scale.
    /// Not decibels of anything stated: the protocol calls it `manGain` and
    /// says nothing more.
    pub man_gain: u8,
}

impl Default for KiwiConfig {
    fn default() -> Self {
        KiwiConfig {
            address: String::new(),
            password: String::new(),
            ident: String::new(),
            wide_lane: true,
            wf_speed: 3,
            agc: true,
            man_gain: 50,
        }
    }
}

impl KiwiConfig {
    /// The port a KiwiSDR serves its web UI on unless its operator moved it.
    pub const DEFAULT_PORT: u16 = 8073;

    /// Pseudo-elements, riding `SetGain` the way the SpyServer's do — see
    /// [`SpyServerConfig::GAIN_ELEMENT`]. Here rather than in the driver
    /// because the settings tab builds them and is shared with the wasm client,
    /// which cannot see `sdroxide-kiwisdr`.
    pub const AGC_ELEMENT: &'static str = "KIWIAGC";
    pub const MAN_GAIN_ELEMENT: &'static str = "KIWIGAIN";
    pub const WIDE_LANE_ELEMENT: &'static str = "KIWIWF";
    pub const WF_SPEED_ELEMENT: &'static str = "KIWIWFSPD";

    /// Slowest and fastest waterfall speeds the protocol accepts.
    pub const WF_SPEED_MIN: u8 = 1;
    pub const WF_SPEED_MAX: u8 = 4;

    /// The configured address as `host:port`, supplying the default port when
    /// the operator typed only a host. Same rule as [`RtlTcpConfig::endpoint`],
    /// including the bracketed-IPv6 case.
    pub fn endpoint(&self) -> String {
        let a = self.address.trim();
        let has_port = match a.rfind(']') {
            Some(close) => a[close + 1..].starts_with(':'),
            None => a.contains(':'),
        };
        if has_port { a.to_string() } else { format!("{a}:{}", Self::DEFAULT_PORT) }
    }

    /// What to announce as, given the station callsign. Empty callsign and
    /// empty override both fall back to the program name, because announcing
    /// nothing at all is what some operators block.
    pub fn ident_or(&self, callsign: &str) -> String {
        let explicit = self.ident.trim();
        if !explicit.is_empty() {
            return explicit.to_string();
        }
        let call = callsign.trim();
        if call.is_empty() { "sdroxide".to_string() } else { format!("{call} (sdroxide)") }
    }

    /// Roughly what one connected receiver costs, in kilobytes a second, so the
    /// settings tab can say it before the operator finds out.
    ///
    /// Measured, not derived: 44.3 kB/s of I/Q and 20.4 kB/s of waterfall at
    /// `wf_speed` 4 against a KiwiSDR 1 running v1.902.
    pub fn link_kbytes_s(&self) -> f64 {
        44.3 + if self.wide_lane { 20.4 * f64::from(self.wf_speed.clamp(1, 4)) / 4.0 } else { 0.0 }
    }
}

/// One RTL-SDR dongle found on the USB bus. Wasm-safe so it can cross the
/// `RadioController` trait to the settings UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RtlSdrDevice {
    /// USB serial string, when the dongle has one programmed.
    pub serial: Option<String>,
    /// Best available name: the USB product string, else the VID/PID table.
    pub name: String,
    pub vid: u16,
    pub pid: u16,
}

impl RtlSdrDevice {
    /// One-line label for the selection UI.
    pub fn label(&self) -> String {
        match &self.serial {
            Some(s) => format!("{}  (serial {s})", self.name),
            // Without a serial we can only ever open "the first one", so say so
            // rather than implying this entry can be pinned.
            None => format!("{}  [no serial — first match only]", self.name),
        }
    }
}

/// An RX-888 seen on the USB bus.
///
/// Wasm-safe so it can cross the `RadioController` trait to the settings UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rx888Device {
    /// USB serial. The boot ROM and the running firmware report *different*
    /// serials, so a pinned value only matches the state the device is in.
    pub serial: Option<String>,
    /// Product string, or a generic name while it is still in its boot ROM.
    pub name: String,
    /// True while the device is still in the Cypress boot ROM. Not a fault:
    /// every RX-888 looks like this until something programs it, and sdroxide
    /// does that on open.
    pub needs_firmware: bool,
    /// Whether the link negotiated SuperSpeed. Only meaningful once the device
    /// is programmed — the boot ROM always enumerates at USB 2.0, even on a
    /// perfectly good USB 3 cable and port.
    pub superspeed: bool,
}

impl Rx888Device {
    /// One-line label for the selection UI.
    pub fn label(&self) -> String {
        let mut s = self.name.clone();
        if let Some(serial) = &self.serial {
            s.push_str(&format!("  (serial {serial})"));
        }
        if self.needs_firmware {
            s.push_str("  [firmware will be uploaded]");
        }
        s
    }
}

/// RX-888 settings (`radio.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Rx888Config {
    /// Pin a particular receiver; empty means "the first one found".
    pub serial: String,
    /// ADC clock in Hz, which is also the real-sample rate on the wire.
    pub adc_rate_hz: f64,
    /// LTC2208 dither: costs a little noise floor, buys spurious-free dynamic
    /// range.
    pub dither: bool,
    /// LTC2208 output randomiser. On by default — it stops the digital bus
    /// radiating into the front end, and undoing it costs one XOR per sample.
    pub randomize: bool,
    /// DC on the HF antenna port. Off by default: putting phantom power on
    /// someone's feedline uninvited is not a good default.
    pub bias_tee_hf: bool,
    /// Select the ADC's wider 2.25 Vp-p input range. Named for the GPIO bit,
    /// which is not actually a preamplifier — see the driver's `gpio::PGA_EN`.
    pub pga: bool,
    /// Step attenuator as a gain, i.e. -31.5..=0 dB.
    pub attenuator_db: f64,
    /// AD8370 VGA gain in dB.
    pub vga_db: f64,
    /// Reference trim, parts per million.
    pub ppm: f64,
    /// Override the bundled FX3 firmware image. Empty uses the built-in one.
    pub firmware_path: String,
    /// Power the VHF antenna port.
    #[serde(default)]
    pub bias_tee_vhf: bool,
    /// R828D RF gain in dB, used above the automatic HF/VHF crossover.
    #[serde(default = "default_tuner_gain_db")]
    pub tuner_gain_db: f64,
    /// Let the tuner run its own gain loops instead of the fixed ladder.
    #[serde(default)]
    pub tuner_agc: bool,
    /// Bins the downconverter keeps of its 8192-bin analysis: the panadapter
    /// width is `adc_rate · bins / 8192`, so 256 is the classic 1/32 and 4096
    /// puts the whole half-spectrum in the panadapter. Zero (an older config)
    /// means the default.
    #[serde(default = "default_ddc_bins")]
    pub ddc_bins: u32,
}

fn default_tuner_gain_db() -> f64 {
    30.0
}

fn default_ddc_bins() -> u32 {
    256
}

impl Default for Rx888Config {
    fn default() -> Self {
        Rx888Config {
            serial: String::new(),
            adc_rate_hz: 64_800_000.0,
            dither: false,
            randomize: true,
            bias_tee_hf: false,
            pga: true,
            attenuator_db: 0.0,
            vga_db: 12.0,
            ppm: 0.0,
            firmware_path: String::new(),
            bias_tee_vhf: false,
            tuner_gain_db: 30.0,
            tuner_agc: false,
            ddc_bins: 256,
        }
    }
}

impl Rx888Config {
    /// Pseudo gain-element names, riding `Command::SetGain` so this backend
    /// needs no new `Command` variant, no `DeviceCaps` field and no engine
    /// change for settings only it has. They live here rather than in
    /// `sdroxide-rx888` so the wasm-safe settings UI can address them without
    /// depending on the native backend crate.
    pub const VGA_ELEMENT: &'static str = "VGA";
    pub const ATT_ELEMENT: &'static str = "ATT";
    pub const DITHER_ELEMENT: &'static str = "DITHER";
    pub const BIAS_TEE_ELEMENT: &'static str = "BIASTEE";
    pub const PGA_ELEMENT: &'static str = "PGA";
    pub const TUNER_GAIN_ELEMENT: &'static str = "TUNER";
    pub const TUNER_AGC_ELEMENT: &'static str = "TUNERAGC";
    pub const BIAS_TEE_VHF_ELEMENT: &'static str = "BIASTEEVHF";

    /// Top of the R828D's 29-step gain ladder — the same figure the RTL-SDR
    /// publishes, because it is the same table in the same chip family.
    pub const TUNER_GAIN_MAX_DB: f64 = 49.6;

    /// ADC clocks offered in the UI. The Si5351 will synthesise nearly
    /// anything between 4 and 130 MHz — the panel has a free-entry field for
    /// that — so this is the set worth a click, not a limit. The native driver
    /// re-exports this same list as `sdroxide_rx888::ADC_RATES`.
    pub const ADC_RATES: [f64; 7] = [
        8_100_000.0,
        16_200_000.0,
        32_400_000.0,
        48_600_000.0,
        64_800_000.0,
        96_000_000.0,
        129_600_000.0,
    ];

    /// What the ADC clock may be trimmed to by hand: the LTC2208's specified
    /// ceiling, and the floor below which the Si5351 and the ADC both
    /// misbehave. The native driver clamps to the same figures.
    pub const MIN_ADC_HZ: f64 = 4_000_000.0;
    pub const MAX_ADC_HZ: f64 = 130_000_000.0;

    /// Panadapter widths on offer, as downconverter bin counts out of
    /// [`Self::DDC_BLOCK`]: 1/32 of the ADC clock up to the full half-spectrum.
    pub const DDC_BIN_CHOICES: [u32; 5] = [256, 512, 1024, 2048, 4096];

    /// The downconverter's analysis size, fixed in the native driver
    /// (`sdroxide_rx888::stream::DDC_BLOCK`); repeated here so the wasm-safe
    /// settings UI can label a bin count with the width it produces.
    pub const DDC_BLOCK: u32 = 8192;

    /// The complex output rate — and so the panadapter width — this clock and
    /// bin count produce.
    pub fn ddc_out_rate_hz(adc_rate_hz: f64, ddc_bins: u32) -> f64 {
        let bins = if ddc_bins == 0 { 256 } else { ddc_bins };
        adc_rate_hz * f64::from(bins) / f64::from(Self::DDC_BLOCK)
    }

    /// The R828D's IF carrier with its 8 MHz filter selected
    /// (`sdroxide_rx888::band::IF_CENTER_HZ`), repeated here for the reason
    /// [`Self::DDC_BLOCK`] is: the settings UI has to be able to label a width
    /// without linking the driver.
    pub const VHF_IF_CENTER_HZ: f64 = 4_570_000.0;

    /// Where the receiver hands over to its tuner
    /// (`sdroxide_rx888::band::crossover_hz`): the ADC's own Nyquist limit,
    /// unless that lands below the tuner's floor.
    pub fn vhf_crossover_hz(adc_rate_hz: f64) -> f64 {
        (adc_rate_hz / 2.0).max(24_000_000.0)
    }

    /// Whether a panadapter width is usable *above* that crossover.
    ///
    /// The downconverter reads a **real** spectrum, so its window must lie
    /// inside DC..Nyquist and its centre clamps up to `out/2` at the bottom.
    /// Above the crossover the wanted signal is no longer the antenna — it is
    /// the tuner's IF, parked at [`Self::VHF_IF_CENTER_HZ`] — so a window wider
    /// than twice that cannot be centred on the IF at all. It pins at `out/2`,
    /// the tuner's 8 MHz of IF fills only the low part of it, and the VHF
    /// path's spectrum inversion then puts that on the *right* of the
    /// panadapter with dead spectrum to its left.
    ///
    /// Nothing is broken when this is false — every width is delivered, and the
    /// tuned signal is received and reported off-centre. There is simply no
    /// more signal to show: the tuner has 8 MHz and no more, however wide the
    /// window asking for it.
    pub fn width_works_on_vhf(adc_rate_hz: f64, ddc_bins: u32) -> bool {
        Self::ddc_out_rate_hz(adc_rate_hz, ddc_bins) <= 2.0 * Self::VHF_IF_CENTER_HZ
    }
}

/// Which Airspy HF+ is on the other end.
///
/// Decoded from the `part_id` word of `GET_SERIALNO_BOARDID` (vendor request
/// 7), because that is the *only* answer: every model — Dual, Discovery,
/// Ranger — enumerates as the same `03eb:800c`, with the same product string on
/// some firmwares. A device list therefore cannot know which one it is looking
/// at; only an opened receiver can say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AirspyHfModel {
    /// HF+ Dual (rev A) — two front ends, HF from 9 kHz.
    Dual,
    /// HF+ Discovery (rev A) — one front end, and the model that reaches
    /// furthest down.
    Discovery,
    /// HF+ Ranger (rev A).
    Ranger,
    #[default]
    Unknown,
}

impl AirspyHfModel {
    pub fn from_part_id(part_id: u32) -> AirspyHfModel {
        match part_id {
            1 => AirspyHfModel::Dual,
            2 => AirspyHfModel::Discovery,
            3 => AirspyHfModel::Ranger,
            _ => AirspyHfModel::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AirspyHfModel::Dual => "Airspy HF+ Dual",
            AirspyHfModel::Discovery => "Airspy HF+ Discovery",
            AirspyHfModel::Ranger => "Airspy HF+ Ranger",
            AirspyHfModel::Unknown => "Airspy HF+ (unknown model)",
        }
    }

    /// Published receive ranges, in Hz.
    ///
    /// The bottom end is reached by the host's own oscillator below the
    /// synthesiser's floor — 180 kHz on a zero-IF rate, 84 kHz on a low-IF one
    /// — which is how VLF tuning works on this hardware at all. The gap
    /// between 31 and 60 MHz is real: there is no front end there.
    ///
    /// An unknown model gets the Dual's ranges, which are the conservative
    /// ones. Publishing too little is a dial that refuses a frequency the
    /// radio can hear; publishing too much only costs a failed tune.
    pub fn freq_ranges(self) -> &'static [(f64, f64)] {
        const DISCOVERY: [(f64, f64); 2] = [(500.0, 31_000_000.0), (60_000_000.0, 260_000_000.0)];
        const DUAL: [(f64, f64); 2] = [(9_000.0, 31_000_000.0), (60_000_000.0, 260_000_000.0)];
        match self {
            AirspyHfModel::Discovery | AirspyHfModel::Ranger => &DISCOVERY,
            AirspyHfModel::Dual | AirspyHfModel::Unknown => &DUAL,
        }
    }
}

/// An Airspy HF+ seen on the USB bus. Wasm-safe so it can cross the
/// `RadioController` trait to the settings UI.
///
/// The model is deliberately absent — see [`AirspyHfModel`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AirspyHfDevice {
    /// The 16 hex digits from the `AIRSPYHF SN:…` descriptor, when it parses.
    pub serial: Option<String>,
    /// The USB product string, else a generic name.
    pub name: String,
}

impl AirspyHfDevice {
    /// One-line label for the selection UI.
    pub fn label(&self) -> String {
        match &self.serial {
            Some(s) => format!("{}  (serial {s})", self.name),
            // Without a serial we can only ever open "the first one", so say so
            // rather than implying this entry can be pinned.
            None => format!("{}  [no serial — first match only]", self.name),
        }
    }
}

/// One ELAD receiver from a USB enumeration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EladDevice {
    /// The serial number read out of the device's EEPROM, when one could be
    /// read. Not the USB `iSerial` string: ELAD's devices do not carry one, so
    /// this is `None` from an enumeration and filled in only once the device
    /// has been opened.
    pub serial: Option<String>,
    /// Which model, from the USB product id.
    pub name: String,
    /// The USB product id, so the selection UI can say what it is holding
    /// before anything has been opened.
    pub pid: u16,
    /// The address on the bus (`bus-port.port…`), which is the only thing that
    /// tells two of the same model apart without opening them.
    pub path: String,
}

impl EladDevice {
    /// One-line label for the selection UI.
    pub fn label(&self) -> String {
        match &self.serial {
            Some(s) => format!("{}  (serial {s})", self.name),
            // Not a defect: ELAD keeps the serial in EEPROM rather than in the
            // USB descriptor, so an unopened device genuinely has nothing to
            // pin. The bus address is what is left, and it changes when the
            // cable moves.
            None => format!("{}  (at {})", self.name, self.path),
        }
    }
}

/// The complex sample rates an ELAD DDC delivers, in Hz.
///
/// **Not commandable.** Nothing sdroxide knows how to send changes this — ELAD's
/// own `gr-elad` never sets it either, and the FDM-DUO has no front-panel menu
/// for it — so this list is what the *stream can be*, not what it can be told to
/// be. See [`EladConfig::sample_rate_hz`].
pub const ELAD_SAMPLE_RATES: [u32; 6] =
    [192_000, 384_000, 768_000, 1_536_000, 3_072_000, 6_144_000];

/// The rate an ELAD device is assumed to be in until told otherwise: the one
/// `gr-elad` defaults to, and the one the FDM-DUO's own specification quotes for
/// its I/Q channel.
pub const ELAD_DEFAULT_RATE_HZ: u32 = 192_000;

/// The attenuator's depth in dB. One pad, in or out.
pub const ELAD_ATTENUATOR_DB: f64 = 12.0;

/// The baud rates an FDM-DUO's CAT port can be set to, from menu 70 `CAT BAUD`.
///
/// The whole list, and nothing outside it works: the port is asynchronous 8N1
/// at one of these four (FDM-DUO manual v2.6 §6.1), so a link opened at any
/// other rate is silent in both directions — no command lands, no answer comes
/// back, and the radio sits there ignoring the dial and refusing to key.
///
/// It is worth being explicit about which rate that is in practice.
/// [`SerialConfig`]'s own default is **19200**, which is not one of these, and
/// [`RadioConfig::cat`] is shared with the CAT / Audio interface — so a `cat`
/// block whose baud has never been touched describes a link an FDM-DUO cannot
/// hear a word of. That is not an exotic mistake; it is what a fresh
/// configuration looks like, and it is why [`elad_cat_baud`] exists rather than
/// this being left to the operator to notice.
pub const ELAD_CAT_BAUDS: [u32; 4] = [9_600, 38_400, 57_600, 115_200];

/// The rate an FDM-DUO ships on, and what a rate it has no setting for falls
/// back to.
pub const ELAD_DEFAULT_CAT_BAUD: u32 = 38_400;

/// The rate an FDM-DUO's CAT port will actually be opened at, given what the
/// configuration asks for.
///
/// A rate the radio has is used as it stands — the operator may well have moved
/// menu 70 off the factory setting, and this must not drag them back. Anything
/// else is a rate no FDM-DUO answers at, so it becomes [`ELAD_DEFAULT_CAT_BAUD`]:
/// a link that might work beats one that certainly cannot, and the caller says
/// on screen that the substitution happened.
pub fn elad_cat_baud(configured: u32) -> u32 {
    if ELAD_CAT_BAUDS.contains(&configured) { configured } else { ELAD_DEFAULT_CAT_BAUD }
}

/// ELAD FDM-DUO / FDM-S1 / FDM-S2 (USB) backend configuration.
///
/// The DUO's rig control is *not* here: it reuses [`RadioConfig::cat`] with
/// [`CatFamily::Elad`], so the serial port, PTT method, poll rate and mode
/// control are configured in the one place every other CAT radio's are. Leaving
/// that serial path empty is how an S1 or S2 — which has no CAT port — comes up
/// receive-only. Transmit audio likewise goes out through
/// [`RadioConfig::radio_audio_out`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EladConfig {
    /// Pin a device by its EEPROM serial; empty means "the first one found".
    pub serial: String,
    /// The DDC's rate in Hz — one of [`ELAD_SAMPLE_RATES`].
    ///
    /// What this *is* depends on the model, because the six rates are six
    /// different FPGA images rather than a register — which is why nothing in
    /// ELAD's vendor protocol selects between them.
    ///
    /// On an FDM-S1 or FDM-S2 the host loads that image at every power-up, so
    /// this is a command: it says which one to load. On an FDM-DUO, which boots
    /// its own, it is only how the stream is *read* — the radio arrives at
    /// whatever rate it powered up in or was last left in by ELAD's own
    /// software, and getting this wrong leaves the samples still samples, with
    /// the panadapter simply the wrong width and every frequency inside it
    /// scaled. Either way the driver measures the throughput once the stream is
    /// running and says so if the two disagree.
    ///
    /// It also selects how the samples are *shaped*: every rate up to 3072 kHz
    /// delivers 32-bit words, and 6144 kHz delivers 16-bit ones. That half is a
    /// real decode difference, not a scale factor, so a wrong guess there is
    /// noise rather than a wrong number.
    pub sample_rate_hz: u32,
    /// The 12 dB input attenuator.
    pub attenuator: bool,
    /// Whether the pre-selection filters are in circuit. Off bypasses them,
    /// which is the wider view and the worse one for strong out-of-band
    /// signals.
    pub preselector: bool,
}

impl Default for EladConfig {
    fn default() -> Self {
        EladConfig {
            serial: String::new(),
            sample_rate_hz: ELAD_DEFAULT_RATE_HZ,
            attenuator: false,
            preselector: true,
        }
    }
}

/// Which socket an ELAD FDM-DUO receives on — the rig's `AN` command, and menu
/// 31 `ANTENNAS` at the front panel.
///
/// The radio has two M-type sockets on the back: `RTX`, which is the transmit
/// output *and* the receive input when only one antenna is in use, and `RX`,
/// which is a receive-only input. `AN` is published as "the number of antennas
/// used" rather than as a port selector, and that is exactly what it switches:
/// one antenna means everything on the RTX socket, two means receiving on the
/// RX socket while transmitting still leaves by RTX.
///
/// So this is a *receive* choice and only a receive choice — there is no
/// transmit port to pick, which is why nothing here ever reaches
/// `DeviceCaps::antennas_tx`. It moves the whole receiver with it: the audio the
/// rig demodulates for itself and the wideband I/Q its DDC puts on the USB
/// interface both come from the socket this selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EladAntenna {
    /// `AN1;` — one antenna, on the RTX socket. The rig's own default.
    #[default]
    Rtx,
    /// `AN2;` — two antennas: receive on the RX socket, transmit out of RTX.
    RxOnly,
}

impl EladAntenna {
    pub const ALL: [EladAntenna; 2] = [EladAntenna::Rtx, EladAntenna::RxOnly];

    /// Both ports' names, in the order [`Self::ALL`] has them — the list a
    /// front end publishes as `DeviceCaps::antennas_rx`.
    pub const LABELS: [&'static str; 2] = ["RTX", "RX only"];

    /// The name this port is known by everywhere outside this enum: in
    /// `DeviceCaps::antennas_rx`, in the `SetAntenna` command, in the combo box
    /// and in `session.json`.
    ///
    /// Named for the socket on the back of the radio rather than for the
    /// command's own "1" and "2", which say nothing about where to plug a
    /// cable — and spelled far enough apart that `RTX` and `RX` cannot be
    /// misread for each other at a glance in a two-line list.
    pub fn label(self) -> &'static str {
        match self {
            EladAntenna::Rtx => EladAntenna::LABELS[0],
            EladAntenna::RxOnly => EladAntenna::LABELS[1],
        }
    }

    /// The `AN` parameter, which is a count of antennas and not an index.
    pub fn digit(self) -> char {
        match self {
            EladAntenna::Rtx => '1',
            EladAntenna::RxOnly => '2',
        }
    }

    /// The port a [`Self::label`] names, or `None` for a name from some other
    /// radio — which is what a `session.json` carried over from another
    /// interface holds.
    pub fn from_label(name: &str) -> Option<EladAntenna> {
        EladAntenna::ALL.into_iter().find(|a| a.label().eq_ignore_ascii_case(name.trim()))
    }

    /// The port an `AN` answer reports.
    pub fn from_digit(c: char) -> Option<EladAntenna> {
        EladAntenna::ALL.into_iter().find(|a| a.digit() == c)
    }

    /// Both ports, as `DeviceCaps::antennas_rx` wants them.
    pub fn names() -> Vec<String> {
        EladAntenna::LABELS.iter().map(|a| a.to_string()).collect()
    }
}

impl EladConfig {
    /// Gain-element name for the attenuator — the one real gain this hardware
    /// has, so it is what the main window's Gain slider drives.
    pub const ATT_ELEMENT: &'static str = "ATT";
    /// Pseudo-element for the pre-selection filters. Deliberately absent from
    /// `DeviceCaps::gains`, so only this backend's own settings tab draws it.
    pub const LPF_ELEMENT: &'static str = "LPF";
}

/// One board from a LimeSuite enumeration.
///
/// LimeSuite reports each device as a 256-byte `key=value` string, and that
/// string — not any field parsed out of it — is what reopens the device. It is
/// carried verbatim in [`Self::info`] for exactly that reason; everything else
/// here is only for the picker to draw a row with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LimeDevice {
    /// The device string LimeSuite handed back, passed straight back to
    /// `LMS_Open`. Never reconstructed from the parsed fields: the format is
    /// LimeSuite's and a rebuilt string is not guaranteed to match.
    pub info: String,
    /// The board name (`LimeSDR-USB`, `LimeSDR-Mini_v2`, `LimeNET-Micro`, …).
    pub name: String,
    /// The board serial, when the string carried one.
    pub serial: String,
    /// How it is attached (`USB 3.0`, `PCIe`, …), for the row's second line.
    pub media: String,
}

impl LimeDevice {
    /// The board names this backend will open.
    ///
    /// An allow-list rather than a deny-list, and that is the whole point.
    /// LimeSuite claims the bare Cypress FX3 id that an unprogrammed RX-888
    /// also presents — the fault recorded on [`Rx888Config`] — so an
    /// enumeration on a machine with one plugged in offers a "Lime" device
    /// that is nothing of the kind. Opening it would flood the log with
    /// transfer errors and hand back a receiver that hears nothing.
    pub const KNOWN_BOARDS: [&'static str; 8] = [
        "LimeSDR-USB",
        "LimeSDR-Mini",
        "LimeSDR-Mini_v2",
        "LimeNET-Micro",
        "LimeSDR-PCIe",
        "LimeSDR-QPCIe",
        "LimeSDR-Core",
        "LimeSDR_Core",
    ];

    /// Whether a device string names a board this backend recognises.
    ///
    /// Matched on a prefix and case-folded, because LimeSuite spells the same
    /// board differently across versions (`LimeSDR-USB` and `LimeSDR-USB_SP`
    /// are the same family) and the trailing variant is not worth a new entry
    /// every time one appears.
    pub fn name_is_known(name: &str) -> bool {
        let name = name.trim().to_ascii_lowercase();
        Self::KNOWN_BOARDS.iter().any(|b| name.starts_with(&b.to_ascii_lowercase()))
    }

    /// Whether `want` selects this board. Empty selects the first one found;
    /// otherwise the serial is matched on its **suffix**, case-folded — the
    /// same rule [`hackrf_serial_matches`] uses, and for the same reason: every
    /// instruction anyone writes down quotes only the last few digits.
    pub fn matches(&self, want: &str) -> bool {
        let want = want.trim();
        if want.is_empty() {
            return true;
        }
        // The whole device string is accepted too, so a config written from a
        // picker row keeps working even if the serial could not be parsed out.
        self.info.eq_ignore_ascii_case(want)
            || (!self.serial.is_empty()
                && self.serial.to_ascii_lowercase().ends_with(&want.to_ascii_lowercase()))
    }

    /// One-line label for the selection UI.
    pub fn label(&self) -> String {
        let mut s = self.name.clone();
        if !self.serial.is_empty() {
            s.push_str(&format!("  (serial {})", self.serial));
        }
        if !self.media.is_empty() {
            s.push_str(&format!("  [{}]", self.media));
        }
        s
    }

    /// How many receive chains this board has, from its name.
    ///
    /// Derived rather than read, because the enumeration never opens a device:
    /// `LMS_GetDeviceList` is the only call made against a board that may
    /// belong to another program, and `LMS_GetNumChannels` needs an open one.
    /// The answer only decides what the picker offers — [`LimeConfig::channel`]
    /// is checked against the real count when the board opens, and a board that
    /// turns out to have fewer chains than this says refuses with the number it
    /// actually has.
    ///
    /// Two on the boards that bring out a second SMA pair (`RX2_H`, `RX2_L`,
    /// `RX2_W`), one everywhere else. The Mini has a single chain; the
    /// LimeNET-Micro's LMS7002M has two but only one is wired to a connector.
    pub fn rx_channels(&self) -> usize {
        let name = self.name.trim().to_ascii_lowercase();
        let two = ["limesdr-usb", "limesdr-pcie", "limesdr-qpcie", "limesdr-core", "limesdr_core"];
        if two.iter().any(|b| name.starts_with(b)) { 2 } else { 1 }
    }

    /// Parse one `key=value, key=value` device string.
    ///
    /// The leading element carries no `=` and is the board name; everything
    /// after it is a pair. Unknown keys are ignored rather than rejected —
    /// LimeSuite adds them between versions and none of them is load-bearing
    /// here, because [`Self::info`] keeps the original.
    pub fn parse(info: &str) -> LimeDevice {
        let mut dev = LimeDevice {
            info: info.to_string(),
            name: String::new(),
            serial: String::new(),
            media: String::new(),
        };
        for (i, part) in info.split(',').map(str::trim).enumerate() {
            match part.split_once('=') {
                Some((k, v)) => match k.trim().to_ascii_lowercase().as_str() {
                    "serial" => dev.serial = v.trim().to_string(),
                    "media" => dev.media = v.trim().to_string(),
                    // Some builds put the board name in a `name=` pair instead
                    // of leading with it bare.
                    "name" if dev.name.is_empty() => dev.name = v.trim().to_string(),
                    _ => {}
                },
                None if i == 0 && !part.is_empty() => dev.name = part.to_string(),
                None => {}
            }
        }
        dev
    }
}

/// What the board's **second** receive chain is for.
///
/// A LimeSDR-USB has two, they share one synthesiser — so they cannot be tuned
/// apart — and they sample from one clock, which makes the pair *coherent*.
/// That is the property everything here is built on: two streams of the same
/// span whose relative phase is fixed by the aerials and the feedlines rather
/// than by chance. What the second one is worth depends entirely on what is
/// plugged into it, which is why this is a choice and not a switch.
///
/// Appended-to rather than reordered: this is serde-serialised into
/// `radio.json` by name, but it also rides the wire, where the variant *index*
/// is what is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LimeAuxRole {
    /// Nothing. The chain is left disabled and no second stream is created,
    /// which is the only setting that costs nothing at all.
    #[default]
    Off,
    /// A second aerial, combined with the first — either to null a local noise
    /// source or to ride out fading. See [`DiversityMode`].
    Diversity,
    /// A directional coupler on the transmitter's output, so the amplifier can
    /// be linearised from a sample of what it actually emitted — the technique
    /// openHPSDR calls PureSignal. Nothing is received on it: it listens only
    /// while transmitting, and what it hears never reaches the demodulator.
    PureSignal,
}

impl LimeAuxRole {
    pub const ALL: [LimeAuxRole; 3] =
        [LimeAuxRole::Off, LimeAuxRole::Diversity, LimeAuxRole::PureSignal];

    pub fn label(self) -> &'static str {
        match self {
            LimeAuxRole::Off => "Not used",
            LimeAuxRole::Diversity => "A second aerial (diversity / QRM suppression)",
            LimeAuxRole::PureSignal => "Transmit feedback (PureSignal predistortion)",
        }
    }
}

/// What to do with two coherent receive channels.
///
/// The mirror of `sdroxide_dsp::DiversityMode`, kept here because this crate is
/// the one the configuration and the wire format live in and it must not depend
/// on the DSP crate. Shared by every backend that has a second receiver to
/// spare: a LimeSDR's other chain ([`LimeAuxConfig`]) and an RSPduo's other
/// tuner ([`SdrPlayDuo`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DiversityMode {
    /// Subtract what the second aerial hears from what the first does: the DSP
    /// form of a noise-cancelling phaser, and the answer to a local QRM source.
    /// The second aerial wants to hear the interference and as little of the
    /// band as possible.
    #[default]
    Cancel,
    /// Add the two in the phase and proportion that maximise signal to noise:
    /// diversity reception, which fills in fades. Both aerials want to hear the
    /// same station, and their noise floors want to be set to about the same
    /// level.
    Combine,
}

impl DiversityMode {
    pub const ALL: [DiversityMode; 2] = [DiversityMode::Cancel, DiversityMode::Combine];

    pub fn label(self) -> &'static str {
        match self {
            DiversityMode::Cancel => "Cancel — null a noise source",
            DiversityMode::Combine => "Combine — diversity reception",
        }
    }
}

/// The second receive chain, and what is done with it.
///
/// Which chain it is is not a setting: there are two, and this is the one
/// [`LimeConfig::channel`] is not. What socket on it is
/// [`Self::antenna`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LimeAuxConfig {
    /// What the chain is for. [`LimeAuxRole::Off`] creates no second stream.
    ///
    /// Changing this rebuilds the session, because a LimeSuite stream is bound
    /// to its channel when it is created.
    pub role: LimeAuxRole,
    /// The port on the second chain: `LNAH`, `LNAL`, `LNAW`, or empty to
    /// follow the main chain's choice. `LNAL` on chain 1 is the socket the
    /// silkscreen calls `RX2_L` — the one issue #98 asks for by name.
    pub antenna: String,
    /// Combined receive gain for the second chain, in dB.
    ///
    /// A real setting, not a convenience. In [`DiversityMode::Combine`]
    /// the branch weighting assumes the two noise floors are comparable; in
    /// [`DiversityMode::Cancel`] a second chain running into compression
    /// hands the canceller a distorted copy of the interference, and a
    /// distorted copy cannot be subtracted from an undistorted one.
    pub gain_db: f64,
    /// Cancel or combine.
    pub mode: DiversityMode,
    /// How many taps the adaptive filter has, 1 to 64.
    ///
    /// One tap is a gain and a phase — a null at one frequency that gets worse
    /// either side of it, which is all an analogue phaser can do. Each further
    /// tap buys one sample period of path difference the filter can equalise,
    /// which is what turns the notch into a band that is quiet all the way
    /// across. They cost arithmetic on the sample path at the full device
    /// rate; see the note in the settings panel.
    pub taps: u8,
    /// How fast the filter adapts, 0 to 1 — slow and steady at the bottom,
    /// converging in a fraction of a second and visibly hunting at the top.
    pub rate: f32,
    /// Hold the filter where it is. The control to reach for the moment a null
    /// has appeared: a converged filter left adapting will re-aim itself at
    /// whatever becomes loudest.
    pub frozen: bool,

    /// How many amplitude steps the predistortion table has, 4 to 256.
    ///
    /// The table is a complex gain against drive level: more entries follow a
    /// sharper knee, but each has to be learned from the samples that landed
    /// in it, and the top of a speech amplitude histogram is thin. Thirty-two
    /// is enough for the smooth curve an HF amplifier actually has.
    pub ps_bins: u8,
    /// How fast the predistortion table adapts, 0 to 1. Separate from
    /// [`Self::rate`] because it is a different loop with a different time
    /// scale — this one only runs while transmitting.
    pub ps_rate: f32,
    /// Hold the predistortion table. A correction learned on a clean over is
    /// worth keeping, and an amplifier's curve does not change between overs.
    pub ps_frozen: bool,
}

/// The longest adaptive filter a settings panel offers, matching
/// `sdroxide_dsp::Diversity::MAX_TAPS`.
pub const DIVERSITY_MAX_TAPS: u8 = 64;

/// The pseudo-gain elements a diversity filter is driven through, whichever
/// backend is running one.
///
/// One set of names rather than one per backend, because the main window's
/// controls are one set too: the strip knows a filter is running from
/// [`crate::DeviceCaps::diversity`] and drives it through these, without
/// having to know whether it is a LimeSDR's second chain or an RSPduo's second
/// tuner behind them. `DIV_MODE_ELEMENT` carries [`DiversityMode`]'s index;
/// `DIV_RESET_ELEMENT` is momentary.
pub const DIV_MODE_ELEMENT: &str = "DIVMODE";
pub const DIV_RATE_ELEMENT: &str = "DIVRATE";
pub const DIV_TAPS_ELEMENT: &str = "DIVTAPS";
pub const DIV_FREEZE_ELEMENT: &str = "DIVFREEZE";
pub const DIV_RESET_ELEMENT: &str = "DIVRESET";

/// What a filter of this length costs on the sample path, for a panel that
/// would otherwise let someone ask for 64 taps at 40 Msps and wonder why the
/// waterfall stopped.
///
/// Three complex multiply-accumulates per tap per sample: the output sum, the
/// weight update, and the conjugate product inside it. It runs at the
/// **device** rate, before any decimation, because that is what makes the
/// interference disappear from the whole panadapter rather than only from the
/// channel being demodulated.
pub fn diversity_cost_note(taps: u8, rate_hz: f64) -> String {
    let mmac = 3.0 * f64::from(taps) * rate_hz / 1e6;
    format!("about {mmac:.0} million complex multiply-accumulates a second")
}

impl LimeAuxConfig {
    /// See [`DIVERSITY_MAX_TAPS`].
    pub const MAX_TAPS: u8 = DIVERSITY_MAX_TAPS;

    /// The predistortion table's bounds, matching what `sdroxide_dsp`'s
    /// `PureSignal::new` clamps to.
    pub const PS_MIN_BINS: u8 = 4;
    pub const PS_MAX_BINS: u8 = 128;

    /// See [`diversity_cost_note`].
    pub fn cost_note(taps: u8, rate_hz: f64) -> String {
        diversity_cost_note(taps, rate_hz)
    }
}

impl Default for LimeAuxConfig {
    fn default() -> Self {
        LimeAuxConfig {
            role: LimeAuxRole::Off,
            antenna: String::new(),
            gain_db: 40.0,
            mode: DiversityMode::Cancel,
            // Enough to equalise a couple of microseconds of path difference,
            // which covers two aerials on one site, without asking for a
            // hundred million multiplies a second at the rates people use.
            taps: 8,
            // Fast enough to watch converge, which is what an operator does
            // with it the first time.
            rate: 0.7,
            frozen: false,
            ps_bins: 32,
            // Slower than the diversity filter's: this one is averaging an
            // amplifier's curve, which does not move, out of a feedback path
            // that has noise in it.
            ps_rate: 0.5,
            ps_frozen: false,
        }
    }
}

/// LimeSDR family (LimeSuite) backend configuration.
///
/// The LimeRFE in front of the radio is [`Self::rfe`]; it is part of this block
/// rather than a peer of it because the board's second control path runs
/// through the LimeSDR's own GPIO, so the two are only separable on paper.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LimeConfig {
    /// Pin a board by its serial suffix, or carry a whole device string; empty
    /// means "the first Lime board found".
    pub device: String,
    /// Which RX/TX chain to receive and transmit on, counted from zero.
    ///
    /// A LimeSDR-USB has two of each and brings both out to their own SMA
    /// pairs; a Mini has one. The two receive chains share the SXR
    /// synthesiser, so they cannot be tuned apart — but they are otherwise
    /// separate front ends on separate sockets, and that is what choosing
    /// between them is for: the `LNAL` on chain 0 is the socket the silkscreen
    /// calls `RX1_L` and the one on chain 1 is `RX2_L`, so an operator who has
    /// done the HF matching modification to one of them can name which
    /// (issue #98).
    ///
    /// Changing this rebuilds the session: a LimeSuite stream is bound to its
    /// channel at `LMS_SetupStream` and cannot be moved.
    pub channel: u8,
    /// Complex sample rate in Hz. See [`Self::SAMPLE_RATES`].
    pub sample_rate_hz: f64,
    /// RF oversampling ratio — 1, 2, 4, 8, 16, 32, or 0 for the device default.
    /// Higher moves the ADC further from the band of interest at the cost of
    /// nothing on the host, so the default is the device's own choice.
    pub oversample: u8,
    /// Combined receive gain in dB, 0–73. LimeSuite takes an integer here, so
    /// this is rounded on the way to the hardware and what comes back from
    /// `current_gains` is what the chip got, not what was asked for.
    pub rx_gain_db: f64,
    /// Combined transmit gain in dB, 0–73. Only reachable while
    /// [`Self::tx_enabled`].
    pub tx_gain_db: f64,
    /// Arm the transmitter.
    ///
    /// Off by default, and the default is the point — the same reasoning as
    /// [`HackRfConfig::tx_enabled`]. With this off the backend publishes no
    /// transmit channel at all, so the engine's own capability check refuses to
    /// key whatever else is configured.
    pub tx_enabled: bool,
    /// Receive port: `LNAH`, `LNAL`, `LNAW`, or empty for "let the driver pick
    /// from the frequency". Which names exist is read from the board.
    pub antenna_rx: String,
    /// Transmit port: `BAND1`, `BAND2`, or empty to let the driver pick.
    pub antenna_tx: String,
    /// Analog low-pass filter bandwidth in Hz; `0.0` derives it from the sample
    /// rate.
    ///
    /// Worth leaving alone, and worth leaving *wide* if not: a filter narrower
    /// than a quarter of the span silently withdraws the zero-IF LO offset
    /// rather than merely softening the band edges — see
    /// `sdroxide_radio::lo_offset_for`.
    pub lpf_rx_hz: f64,
    /// The same on the transmit side; `0.0` follows the rate.
    pub lpf_tx_hz: f64,
    /// Run LimeSuite's own DC-offset and IQ-imbalance calibration when the
    /// device is opened, **and again once the dial settles on a new band or a
    /// different socket**. Costs about a second each time and is worth it: this
    /// is a zero-IF radio, the uncalibrated image sits across the band, and the
    /// numbers are measured at one LO frequency — carried to another they are
    /// not merely absent but wrong, which is what a carrier parked in the
    /// middle of the span usually is (issue #94). The retune half waits for the
    /// operator to stop moving, so dragging a panadapter never stalls on it.
    pub calibrate: bool,
    /// Adaptive IQ image correction and DC removal on the host, on top of the
    /// chip's own calibration. On by default for the same reason the HackRF
    /// has it.
    pub iq_correction: bool,
    /// LimeSuite's own FIFO, in thousands of samples. Its streaming layer
    /// already buffers, so this is the only depth that matters; the ring on
    /// this side only decouples the engine's block cadence.
    pub fifo_ksamples: u32,
    /// LimeSuite's `throughputVsLatency`, 0.0–1.0. Low favours latency.
    pub throughput_vs_latency: f32,
    /// The LimeRFE front end, if one is attached.
    pub rfe: LimeRfeConfig,
    /// The board's second receive chain, and what it is for (issue #98).
    pub aux: LimeAuxConfig,
}

impl Default for LimeConfig {
    fn default() -> Self {
        LimeConfig {
            device: String::new(),
            channel: 0,
            // Wide enough to be a useful panadapter, narrow enough that the
            // LO offset applies and any USB 3 port keeps up.
            sample_rate_hz: 5_000_000.0,
            oversample: 0,
            rx_gain_db: 40.0,
            // Minimum drive, transmitter disarmed: the radio comes up unable
            // to emit anything meaningful even if it is keyed.
            tx_gain_db: 0.0,
            tx_enabled: false,
            antenna_rx: String::new(),
            antenna_tx: String::new(),
            lpf_rx_hz: 0.0,
            lpf_tx_hz: 0.0,
            calibrate: true,
            iq_correction: true,
            fifo_ksamples: 256,
            throughput_vs_latency: 0.5,
            rfe: LimeRfeConfig::default(),
            aux: LimeAuxConfig::default(),
        }
    }
}

impl LimeConfig {
    /// The real gain elements, published in `DeviceCaps::gains` so the generic
    /// sliders drive them and the engine remembers them across a reopen.
    ///
    /// Two, and only two, because `LMS_SetGaindB` is the only gain the C API
    /// exposes: it distributes a single number across the LNA, TIA and PGA
    /// itself. Reaching those stages individually needs `LMS_WriteParam`, and
    /// three sliders that silently fight the combined one would be worse than
    /// the one that works.
    pub const RX_GAIN_ELEMENT: &'static str = "RX";
    pub const TX_GAIN_ELEMENT: &'static str = "TX";

    /// The gain range LimeSuite accepts, in dB. It takes an `unsigned`, so the
    /// step is 1 dB and anything finer is truncated by the library.
    pub const GAIN_MIN_DB: f64 = 0.0;
    pub const GAIN_MAX_DB: f64 = 73.0;

    /// Below this much transmit gain, an over is worth saying something about.
    ///
    /// Not a threshold with a datasheet behind it: [`Self::tx_gain_db`]
    /// defaults to the bottom of the range on purpose, so that arming the
    /// transmitter cannot by itself put anything on the air, and left there it
    /// produces a radio that keys, reports no error and emits microwatts.
    /// Downstream of the antenna that is indistinguishable from a transmitter
    /// that does not work — which is the report it exists to answer.
    ///
    /// One number rather than two because the settings panel and the driver
    /// both apply it, and a panel that stayed quiet while the log complained
    /// would be worse than either alone.
    pub const LOW_DRIVE_DB: f64 = 10.0;

    /// Pseudo-elements carrying settings that are not gains at all.
    ///
    /// They ride the existing `SetGain` command so this backend needs no new
    /// `Command` variant, no `DeviceCaps` field and no engine change. They are
    /// deliberately absent from `DeviceCaps::gains`, so nothing renders them as
    /// sliders — the LimeSDR settings panel drives them directly.
    pub const LPF_RX_ELEMENT: &'static str = "LPFBW";
    pub const LPF_TX_ELEMENT: &'static str = "TXLPFBW";
    pub const IQ_CORRECTION_ELEMENT: &'static str = "IQCORR";
    /// Momentary: any value at or above 0.5 runs a calibration now.
    pub const CALIBRATE_ELEMENT: &'static str = "CAL";
    /// The second chain and the diversity filter, through the same door. The
    /// filter's own names are the shared ones ([`DIV_MODE_ELEMENT`] and
    /// friends), which is what lets the main strip drive it without knowing
    /// which board it is.
    pub const AUX_GAIN_ELEMENT: &'static str = "AUXGAIN";
    pub const DIV_MODE_ELEMENT: &'static str = DIV_MODE_ELEMENT;
    pub const DIV_RATE_ELEMENT: &'static str = DIV_RATE_ELEMENT;
    pub const DIV_TAPS_ELEMENT: &'static str = DIV_TAPS_ELEMENT;
    pub const DIV_FREEZE_ELEMENT: &'static str = DIV_FREEZE_ELEMENT;
    pub const DIV_RESET_ELEMENT: &'static str = DIV_RESET_ELEMENT;
    /// The predistortion loop, likewise. `PSRESET` is momentary and forgets
    /// the table as well as the alignment.
    pub const PS_BINS_ELEMENT: &'static str = "PSBINS";
    pub const PS_RATE_ELEMENT: &'static str = "PSRATE";
    pub const PS_FREEZE_ELEMENT: &'static str = "PSFREEZE";
    pub const PS_RESET_ELEMENT: &'static str = "PSRESET";

    /// The device setting that names the second chain's port. A name rather
    /// than a number, so it goes through `SetDeviceSetting` rather than riding
    /// a pseudo-gain like everything else here.
    pub const AUX_ANTENNA_SETTING: &'static str = "aux_antenna";

    /// The LimeRFE's whole configuration, as
    /// [`LimeRfeConfig::to_setting`](crate::LimeRfeConfig::to_setting) writes
    /// it, through the same door.
    ///
    /// One setting rather than a pseudo-gain per control. These fields only
    /// mean anything together — which channel a dial resolves to depends on
    /// the connectors, and which relay modes the board will accept depends on
    /// both — so pushing them one at a time would put states on the wire that
    /// no configuration ever asked for. It also means a control added to the
    /// panel reaches a running board without a new element to plumb, which is
    /// what the connectors, the band and the relay mode were missing (issue
    /// #94: changing them wrote the file and did nothing until a restart).
    pub const RFE_SETTING: &'static str = "limerfe";

    /// The chain the auxiliary stream runs on: the one the main stream is not.
    /// There are two, so this is arithmetic rather than a setting.
    pub fn aux_channel(&self) -> u8 {
        1 - self.channel.min(1)
    }

    /// The board socket a receive port name reaches on a given chain.
    ///
    /// LimeSuite's port names (`LNAH`, `LNAL`, `LNAW`) are the chip's, and are
    /// the same on every chain — which is exactly what makes them useless for
    /// saying *which connector*. The board silkscreen numbers the chain, so
    /// chain 0's low-band input is `RX1_L` and chain 1's is `RX2_L`. Both the
    /// picker and the logs say the socket, because that is the end an aerial
    /// goes into.
    ///
    /// `None` for a name that is not one of the three, so a board reporting
    /// something unexpected is shown LimeSuite's name unadorned rather than a
    /// guess.
    pub fn rx_socket(channel: u8, port: &str) -> Option<String> {
        let suffix = match port.trim().to_ascii_uppercase().as_str() {
            "LNAH" => "H",
            "LNAL" => "L",
            "LNAW" => "W",
            _ => return None,
        };
        Some(format!("RX{}_{suffix}", channel + 1))
    }

    /// The band each transmit port is matched for, straight out of LimeSuite's
    /// own `LMS7_Device::GetTxPathBand` (23.11):
    ///
    /// ```text
    /// case LMS_PATH_TX1: return Range(30e6, 1.9e9);
    /// case LMS_PATH_TX2: return Range(2e9, 2.6e9);
    /// ```
    ///
    /// These are not alternatives. They are two different matching networks on
    /// the board, and BAND2 — the microwave one — passes essentially nothing on
    /// any band this program is normally used for. A LimeSDR keyed on 145 MHz
    /// out of BAND2 answers every command with `ok`, reports full drive,
    /// streams its samples, and puts nothing on a power meter (issue #94).
    ///
    /// `None` for a name this table does not describe, so a board reporting
    /// something unexpected is never second-guessed on figures that do not
    /// describe it. Deliberately not mirrored for receive either: LimeSuite's
    /// receive table stops LNAW at 700 MHz and LNAL at 900, and the boards
    /// plainly do better than that — 2 m and even 20 m are received on `RX1_W`
    /// — so vetting a receive port against it would refuse the cabling that
    /// works.
    pub fn tx_port_band(port: &str) -> Option<(f64, f64)> {
        match port.trim().to_ascii_uppercase().as_str() {
            "BAND1" => Some((30.0e6, 1.9e9)),
            "BAND2" => Some((2.0e9, 2.6e9)),
            _ => None,
        }
    }

    /// Whether `port` is matched for `hz`. A port [`Self::tx_port_band`] does
    /// not know gets the benefit of the doubt.
    pub fn tx_port_covers(port: &str, hz: f64) -> bool {
        Self::tx_port_band(port).is_none_or(|(lo, hi)| hz >= lo && hz <= hi)
    }

    /// A transmit port with its socket *and* the band it is matched for:
    /// `BAND1 — TX1_1, 30 MHz–1.9 GHz`. The band is the part that matters —
    /// see [`Self::tx_port_band`] — and leaving it off a picker is how an
    /// operator ends up transmitting 2 m into a 13 cm matching network.
    pub fn tx_port_label(channel: u8, port: &str) -> String {
        let label = Self::port_label(channel, port, true);
        match Self::tx_port_band(port) {
            Some((lo, hi)) => format!("{label}, {} – {}", Self::mhz(lo), Self::mhz(hi)),
            None => label,
        }
    }

    /// A frequency in whichever of MHz or GHz reads better.
    fn mhz(hz: f64) -> String {
        if hz >= 1.0e9 {
            format!("{:.1} GHz", hz / 1.0e9)
        } else {
            format!("{:.0} MHz", hz / 1.0e6)
        }
    }

    /// The same for a transmit port: `BAND1` on chain 0 is `TX1_1`.
    pub fn tx_socket(channel: u8, port: &str) -> Option<String> {
        let suffix = match port.trim().to_ascii_uppercase().as_str() {
            "BAND1" => "1",
            "BAND2" => "2",
            _ => return None,
        };
        Some(format!("TX{}_{suffix}", channel + 1))
    }

    /// A port name with its socket beside it, for a combo row or a log line:
    /// `LNAL — RX2_L`. Just the name where the socket is not known.
    pub fn port_label(channel: u8, port: &str, tx: bool) -> String {
        let socket =
            if tx { Self::tx_socket(channel, port) } else { Self::rx_socket(channel, port) };
        match socket {
            Some(s) => format!("{port} — {s}"),
            None => port.to_string(),
        }
    }

    /// The rates offered in the settings combo.
    ///
    /// The hardware synthesises anything inside the range it reports, so this
    /// is a useful subset rather than the whole menu — and the range is read
    /// from the board at open, which is what actually bounds it. The top two
    /// are past what a USB 3 port will hold up over a long session; see
    /// [`Self::rate_note`].
    pub const SAMPLE_RATES: [f64; 9] =
        [1.0e6, 2.0e6, 2.5e6, 5.0e6, 10.0e6, 15.36e6, 20.0e6, 30.72e6, 40.0e6];

    /// What to say beside a rate in the combo, when there is something to say.
    ///
    /// The numbers are the host link's load at 12 bits per sample per
    /// component, which is what the board actually sends — the ADC is 12-bit,
    /// so nothing is lost by not asking for 16.
    pub fn rate_note(rate_hz: f64) -> Option<&'static str> {
        match rate_hz {
            r if r >= 40.0e6 => Some("beyond USB 3 — PCIe boards only"),
            r if r >= 30.0e6 => Some("needs a good USB 3 port"),
            r if r <= 1.0e6 => Some("no LO offset below 1 Msps"),
            _ => None,
        }
    }
}

/// Airspy HF+ (USB) backend configuration. Receive only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AirspyHfConfig {
    /// Pin a receiver by the 16 hex digits of its USB serial; empty means "the
    /// first one found". The descriptor spells it
    /// `AIRSPYHF SN:0123456789ABCDEF`; only the digits are stored.
    pub serial: String,
    /// Complex sample rate in Hz. Which rates exist depends on the model *and*
    /// the firmware, so this is snapped to the nearest one the receiver
    /// actually reports at open — see [`Self::SAMPLE_RATES`].
    pub sample_rate_hz: f64,
    /// The receiver's own AGC.
    pub agc: bool,
    /// AGC threshold: `false` = low, `true` = high. Only meaningful while
    /// [`Self::agc`] is on.
    pub agc_threshold_high: bool,
    /// Front-end attenuator, carried as a *gain* so more slider is more
    /// signal: `0.0` down to `-48.0` dB. Snapped to the steps the receiver
    /// reports, six dB apiece on every firmware seen so far. Only obeyed with
    /// the AGC off.
    pub attenuator_db: f64,
    /// The HF preamplifier. Buys sensitivity at the cost of intermodulation,
    /// so off by default — which is the right setting on a real antenna.
    pub lna: bool,
    /// Bias tee on the antenna port. Off by default: putting phantom power on
    /// someone's feedline uninvited is not a good default.
    pub bias_tee: bool,
    /// Frequency calibration in parts per *billion* — the receiver's own unit,
    /// not ppm. `None` (the default) uses the value stored in the receiver's
    /// flash, which is what the vendor tool wrote and is normally right.
    /// Setting a value overrides it for this session; nothing here ever writes
    /// flash.
    pub calibration_ppb: Option<i32>,
    /// Run the host-side DSP: the adaptive IQ image balancer, the 5 kHz
    /// zero-IF offset and the fine-tuning oscillator that brings the signal
    /// back to baseband. On by default — without it a zero-IF rate puts the
    /// synthesiser's own leakage on the operator's signal and the mirror image
    /// sits tens of dB higher. Turn it off only to see raw hardware output.
    pub lib_dsp: bool,
}

impl Default for AirspyHfConfig {
    fn default() -> Self {
        AirspyHfConfig {
            serial: String::new(),
            // Present on every model and every firmware, and the receiver's own
            // power-on default since R2.8.1.
            sample_rate_hz: 768_000.0,
            agc: true,
            agc_threshold_high: false,
            attenuator_db: 0.0,
            lna: false,
            bias_tee: false,
            calibration_ppb: None,
            lib_dsp: true,
        }
    }
}

impl AirspyHfConfig {
    /// The one real gain element, carried *negative* so more slider is more
    /// signal — like the RX-888's attenuator. It lives here rather than in
    /// `sdroxide-airspyhf` so the wasm-safe settings UI can address it without
    /// depending on the native backend crate, the same reason as
    /// [`HpsdrConfig::LNA_GAIN_ELEMENT`].
    pub const ATT_ELEMENT: &'static str = "ATT";

    /// Pseudo-elements carrying settings that are not gains at all.
    ///
    /// They ride the existing `SetGain` command so this backend needs no new
    /// `Command` variant, no `DeviceCaps` field and no engine change for six
    /// settings only it has. They are deliberately absent from
    /// `DeviceCaps::gains`, so nothing renders them as sliders — the Airspy
    /// HF+ settings panel drives them directly.
    pub const AGC_ELEMENT: &'static str = "AGC";
    pub const AGC_THRESHOLD_ELEMENT: &'static str = "AGCTHR";
    pub const LNA_ELEMENT: &'static str = "LNA";
    pub const BIAS_TEE_ELEMENT: &'static str = "BIASTEE";
    /// Parts per *billion*, not million. Named `PPB` rather than `PPM` on
    /// purpose: this receiver calibrates in ppb, and a value copied out of the
    /// RTL-SDR's ppm field would be a thousand times too small.
    pub const PPB_ELEMENT: &'static str = "PPB";
    pub const LIB_DSP_ELEMENT: &'static str = "LIBDSP";

    /// Every rate any model and firmware combination is known to offer — the
    /// list the settings combo shows *before* a receiver has been opened. It
    /// is a menu, not a promise: the real list is queried from the device and
    /// published in `DeviceCaps::sample_rates`, and the settings tab prefers
    /// that whenever one is connected.
    pub const SAMPLE_RATES: [f64; 8] =
        [192_000.0, 228_000.0, 256_000.0, 384_000.0, 456_000.0, 650_000.0, 768_000.0, 912_000.0];

    /// Attenuator steps assumed when the receiver's own table cannot be read
    /// (firmware before R3.0.7 does not answer): nine 6 dB steps, 0 to 48 dB.
    pub const ATT_STEP_DB: f64 = 6.0;
    pub const ATT_MAX_DB: f64 = 48.0;

    /// A short note on a rate, for the settings combo. Which rates a given
    /// receiver has depends on the model and the firmware together, so the
    /// pre-open list has to say who each one belongs to.
    pub fn rate_note(rate_hz: f64) -> &'static str {
        match rate_hz as u32 {
            912_000 => "Discovery, R3.0.7+ — zero-IF",
            768_000 => "every model — zero-IF",
            650_000 => "R4.0.4+ — low-IF",
            456_000 => "Discovery, R3.0.7+ — low-IF",
            228_000 => "R4.0.0+ — low-IF",
            256_000 => "before R4.0.0 — low-IF",
            _ => "low-IF",
        }
    }
}

/// A HackRF seen on the USB bus. Wasm-safe so it can cross the
/// `RadioController` trait to the settings UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HackRfDevice {
    /// The 32 hex digits of the USB serial descriptor, when it has one.
    pub serial: Option<String>,
    /// The USB product string, else the name the product id implies.
    pub name: String,
    /// The USB product id. Kept because it is the only thing that separates a
    /// HackRF One from a Jawbreaker or a rad1o without opening the device, and
    /// the three do not tune the same range — see `BoardKind::freq_range` in
    /// `sdroxide-hackrf`.
    ///
    /// It does **not** separate a HackRF One from a HackRF Pro: those two share
    /// `0x6089`. That is what [`Self::is_pro`] is for.
    pub pid: u16,
}

/// Whether an operator-supplied serial names a radio whose USB serial is
/// `found`.
///
/// **Suffix** matching, which is libhackrf's behaviour and the reason every
/// HackRF instruction on the internet quotes only the last few digits: the
/// serial is 32 hex characters of which the leading half is usually zeroes, so
/// nobody types the whole thing. Case-insensitive and whitespace-tolerant where
/// libhackrf is neither, because the value in `radio.json` was typed or pasted
/// by a human and the descriptor's case is the firmware's choice.
///
/// An empty `want` matches everything — that is "no serial configured", not "a
/// serial that happens to be blank".
///
/// Lives here rather than in `sdroxide-hackrf` because the settings UI has to
/// apply the same rule as the driver — it decides which device's capabilities
/// to draw the panel from — and the settings UI is shared with the wasm client,
/// which cannot depend on a USB crate. `sdroxide_hackrf::protocol::serial_matches`
/// delegates here, so there is one rule and one set of tests for it.
pub fn hackrf_serial_matches(want: &str, found: Option<&str>) -> bool {
    let want = want.trim();
    if want.is_empty() {
        return true;
    }
    match found {
        Some(f) => {
            let f = f.trim();
            f.len() >= want.len() && f[f.len() - want.len()..].eq_ignore_ascii_case(want)
        }
        None => false,
    }
}

impl HackRfDevice {
    /// Whether a configured serial names this radio. See
    /// [`hackrf_serial_matches`] — suffix, not equality.
    pub fn matches_serial(&self, want: &str) -> bool {
        hackrf_serial_matches(want, self.serial.as_deref())
    }

    /// Whether this is a HackRF Pro rather than a HackRF One.
    ///
    /// Read off the USB product string, because the product id cannot tell:
    /// GSG's firmware ships one device descriptor for both boards and only the
    /// string differs. The board id would be definitive but needs a control
    /// transfer, and this has to answer in the settings dialog — which lists
    /// radios without opening any of them, so that pressing Rescan while one is
    /// transmitting stays harmless.
    ///
    /// The settings UI needs the answer because the two boards do not accept
    /// the same sample rates: the Pro decimates in its FPGA and takes rates a
    /// decade below anything a MAX5864 board can usefully run.
    pub fn is_pro(&self) -> bool {
        self.pid == 0x6089 && self.name.trim().eq_ignore_ascii_case("HackRF Pro")
    }

    /// One-line label for the selection UI.
    pub fn label(&self) -> String {
        match &self.serial {
            // The tail is what identifies a radio in practice: the leading half
            // of a HackRF serial is zeroes on every unit, so showing all 32
            // digits is 16 characters of noise. The full value is still what
            // gets stored.
            Some(s) => {
                let tail = &s[s.len().saturating_sub(8)..];
                format!("{}  (serial …{tail})", self.name)
            }
            // Without a serial we can only ever open "the first one", so say so
            // rather than implying this entry can be pinned.
            None => format!("{}  [no serial — first match only]", self.name),
        }
    }
}

/// HackRF (USB) backend configuration — One, Pro, Jawbreaker or rad1o. Receive,
/// and transmit once it is armed — see [`Self::tx_enabled`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HackRfConfig {
    /// Pin a radio by its USB serial; empty means "the first one found".
    /// Matched on the **suffix**, which is libhackrf's behaviour and the reason
    /// every HackRF instruction quotes only the last eight digits.
    pub serial: String,
    /// Complex sample rate in Hz. See [`Self::SAMPLE_RATES`].
    pub sample_rate_hz: f64,
    /// Front-end LNA, 0–40 dB in 8 dB steps. Truncated to a step by the
    /// hardware, so what is stored here is not always what is applied; the
    /// settings tab shows back what the radio really did.
    pub lna_db: f64,
    /// Baseband VGA, 0–62 dB in 2 dB steps.
    pub vga_db: f64,
    /// The 14 dB RF amplifier, on receive.
    ///
    /// Off by default. It is one switch shared with [`Self::tx_amp`] rather
    /// than two independent stages, and on a real antenna a HackRF front end
    /// overloads with it in circuit — which is exactly why the two directions
    /// are separate settings here even though the hardware has one control.
    pub amp: bool,
    /// Transmit VGA, 0–47 dB in 1 dB steps.
    pub txvga_db: f64,
    /// The 14 dB RF amplifier, on transmit. The same switch as [`Self::amp`],
    /// applied when the radio changes direction.
    pub tx_amp: bool,
    /// Arm the transmitter.
    ///
    /// Off by default, and the default is the point. A HackRF is a wideband
    /// transmitter with poor harmonic suppression that wants an external
    /// low-pass filter for any real use; somebody who plugged one in to listen
    /// should not be one PTT away from radiating. With this off the backend
    /// publishes no transmit channel at all, so the engine's own capability
    /// check refuses to key.
    pub tx_enabled: bool,
    /// Bias tee on the antenna port — about 3 V at 50 mA. Off by default:
    /// putting phantom power on someone's feedline uninvited is not a good
    /// default. Absent on the Jawbreaker and the rad1o, which have no such
    /// circuit.
    pub bias_tee: bool,
    /// Baseband filter bandwidth in Hz; `0.0` means "follow the sample rate",
    /// which is what almost everyone wants.
    ///
    /// Worth leaving alone: the filter is coupled to the zero-IF LO offset, and
    /// choosing one too narrow silently withdraws the offset rather than merely
    /// softening the band edges. `sdroxide-hackrf`'s `auto_filter_bw` has the
    /// arithmetic and a test that pins it.
    ///
    /// Ignored on a HackRF Pro, whose firmware derives the filter from the
    /// sample rate and discards what the host asks for — see
    /// `BoardKind::sets_own_filter`. The driver does not send the request on
    /// that board, and the open status says so if this is set anyway.
    pub filter_bw_hz: f64,
    /// Crystal error in parts per million.
    pub ppm: f64,
    /// Adaptive IQ image correction and DC removal on the host. On by default:
    /// this is a zero-IF radio, so its own LO leakage sits at the centre of the
    /// span and the MAX2837's quadrature error puts a mirror image across it.
    /// Turn it off to see raw hardware output, which is also the one-click way
    /// to tell a driver problem from a DSP one.
    pub iq_correction: bool,
    /// Bulk transfers in flight, and the size of each in KiB. Defaults suit
    /// every rate; they exist because 20 Msps on a marginal USB 3 port is the
    /// one case where the geometry has to be tuned by hand.
    pub transfers: u8,
    pub transfer_kib: u16,
}

impl Default for HackRfConfig {
    fn default() -> Self {
        HackRfConfig {
            serial: String::new(),
            // The rate the LO-offset policy was measured at, and the one that
            // asks least of the host.
            sample_rate_hz: 2_000_000.0,
            lna_db: 16.0,
            vga_db: 16.0,
            amp: false,
            // Minimum drive and no amplifier: the transmitter comes up unable
            // to emit anything meaningful even if it is armed and keyed.
            txvga_db: 0.0,
            tx_amp: false,
            tx_enabled: false,
            bias_tee: false,
            filter_bw_hz: 0.0,
            ppm: 0.0,
            iq_correction: true,
            transfers: 16,
            transfer_kib: 64,
        }
    }
}

impl HackRfConfig {
    /// The real gain elements, published in `DeviceCaps::gains` so the generic
    /// sliders drive them and the engine remembers them across a reopen.
    ///
    /// `LNA` is first on purpose: `gains[0]` is what the main window's Gain
    /// slider reaches, and the LNA is the stage that actually changes
    /// sensitivity — and the stage that overloads.
    pub const LNA_ELEMENT: &'static str = "LNA";
    pub const VGA_ELEMENT: &'static str = "VGA";
    pub const TXVGA_ELEMENT: &'static str = "TXVGA";

    /// Pseudo-elements carrying settings that are not gains at all.
    ///
    /// They ride the existing `SetGain` command so this backend needs no new
    /// `Command` variant, no `DeviceCaps` field and no engine change. They are
    /// deliberately absent from `DeviceCaps::gains`, so nothing renders them as
    /// sliders — the HackRF settings panel drives them directly.
    ///
    /// `AMP` and `TXAMP` are two names for one hardware switch. That is not an
    /// oversight: the radio applies whichever one belongs to the direction it
    /// is entering, which is how an operator can run the preamp bypassed on
    /// receive and in circuit on transmit.
    pub const AMP_ELEMENT: &'static str = "AMP";
    pub const TXAMP_ELEMENT: &'static str = "TXAMP";
    pub const BIAS_TEE_ELEMENT: &'static str = "BIASTEE";
    /// Carried in Hz, not as a table index, so a stored value survives a change
    /// to the filter table.
    pub const FILTER_ELEMENT: &'static str = "BBFILT";
    pub const PPM_ELEMENT: &'static str = "PPM";
    pub const IQ_CORRECTION_ELEMENT: &'static str = "IQCORR";

    /// The 14 dB RF amplifier's step, for the settings label.
    pub const AMP_DB: f64 = 14.0;

    /// The rates offered in the settings combo.
    ///
    /// The hardware takes anything from 2 to 20 Msps; this is the useful subset.
    /// Unlike the Airspy HF+ list next door this is not queried from the device
    /// — a HackRF has no rate table, it synthesises whatever it is asked for —
    /// so this list is the whole menu.
    pub const SAMPLE_RATES: [f64; 7] = [2.0e6, 4.0e6, 8.0e6, 10.0e6, 12.5e6, 16.0e6, 20.0e6];

    /// The extra rates a **HackRF Pro** offers, below everything the other
    /// boards can do. Prepended to [`Self::SAMPLE_RATES`] by
    /// [`Self::rates_for`].
    ///
    /// Not offered on a HackRF One, and the distinction is not pedantry. On the
    /// MAX5864 boards the converter simply runs slower while the narrowest
    /// analog filter stays 1.75 MHz wide, so a 500 ksps stream comes back with
    /// three-quarters of a megahertz of everything else folded into it. The Pro
    /// runs its front end fast and decimates in the FPGA instead, so its narrow
    /// rates are narrow all the way through.
    ///
    /// Worth having rather than decimating on the host: at 500 ksps a Pro is
    /// sending 1 MB/s over the USB link instead of 4, and the whole
    /// panadapter-and-decoders chain runs on a fortieth of the samples.
    pub const PRO_SAMPLE_RATES: [f64; 4] = [0.25e6, 0.5e6, 1.0e6, 1.5e6];

    /// The rates to offer for a given board — the Pro's extra low rates first,
    /// then the ones every HackRF shares.
    pub fn rates_for(is_pro: bool) -> Vec<f64> {
        let mut out = Vec::with_capacity(Self::PRO_SAMPLE_RATES.len() + Self::SAMPLE_RATES.len());
        if is_pro {
            out.extend_from_slice(&Self::PRO_SAMPLE_RATES);
        }
        out.extend_from_slice(&Self::SAMPLE_RATES);
        out
    }

    /// A short note on a rate, for the settings combo. The interesting facts
    /// are at the two ends: what the datasheet actually covers, and what the
    /// host and the USB link have to keep up with.
    pub fn rate_note(rate_hz: f64) -> &'static str {
        match rate_hz as u32 {
            250_000 => "HackRF Pro only — 0.5 MB/s, and narrow in the FPGA rather than aliased",
            500_000 => "HackRF Pro only — 1 MB/s",
            1_000_000 => "HackRF Pro only — 2 MB/s",
            1_500_000 => "HackRF Pro only — 3 MB/s",
            2_000_000 => "below the MAX5864's spec — but the usual choice, and 4 MB/s",
            4_000_000 => "below the MAX5864's spec — 8 MB/s",
            8_000_000 => "lowest specified rate — 16 MB/s",
            10_000_000 => "20 MB/s",
            12_500_000 => "25 MB/s",
            16_000_000 => "32 MB/s",
            20_000_000 => "40 MB/s — wants a real USB 3 port and a modern CPU",
            _ => "",
        }
    }
}

/// Which of the R820T2's two curated gain curves to drive the front end from.
///
/// The tuner has an LNA, a mixer and a VGA, and setting the three
/// independently is a good way to build a receiver that either overloads or
/// hisses. Airspy publishes two curves through them and every Airspy program
/// offers the choice rather than three sliders; this does the same, because the
/// numbers were tuned as curves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AirspyGain {
    /// Least intermodulation for a given sensitivity — the right default on an
    /// antenna with broadcast nearby.
    #[default]
    Linearity,
    /// More sensitivity, less overload margin.
    Sensitivity,
}

impl AirspyGain {
    pub fn code(self) -> u8 {
        match self {
            AirspyGain::Linearity => 0,
            AirspyGain::Sensitivity => 1,
        }
    }

    pub fn from_code(code: u8) -> AirspyGain {
        match code {
            1 => AirspyGain::Sensitivity,
            _ => AirspyGain::Linearity,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AirspyGain::Linearity => "Linearity (best strong-signal handling)",
            AirspyGain::Sensitivity => "Sensitivity (best weak-signal)",
        }
    }

    pub const ALL: [AirspyGain; 2] = [AirspyGain::Linearity, AirspyGain::Sensitivity];
}

/// An Airspy R2 or Mini seen on the USB bus. Wasm-safe so it can cross the
/// `RadioController` trait to the settings UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AirspyDevice {
    /// The USB serial descriptor, when it has one.
    pub serial: Option<String>,
    /// The USB product string, else a generic name.
    ///
    /// An R2 and a Mini are indistinguishable here — same product id, same
    /// product string. Only the rate list separates them, and that needs the
    /// device open, so this does not pretend to know.
    pub name: String,
}

impl AirspyDevice {
    /// One-line label for the selection UI.
    pub fn label(&self) -> String {
        match &self.serial {
            Some(s) => {
                let tail = &s[s.len().saturating_sub(8)..];
                format!("{}  (serial …{tail})", self.name)
            }
            // Without a serial we can only ever open "the first one", so say so
            // rather than implying this entry can be pinned.
            None => format!("{}  [no serial — first match only]", self.name),
        }
    }
}

/// Airspy R2 / Mini (USB) backend configuration. Receive only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AirspyConfig {
    /// Pin a receiver by its USB serial; empty means "the first one found".
    /// Matched on the **suffix**, so the last eight digits are enough.
    pub serial: String,
    /// **Complex** sample rate in Hz — what you get, not what the ADC runs at.
    /// The receiver is programmed at twice this, because its ADC is real and
    /// the host makes complex baseband from it. Snapped to a rate the receiver
    /// actually offers at open; see [`Self::SAMPLE_RATES`].
    pub sample_rate_hz: f64,
    /// Which curated gain curve the three tuner stages follow.
    pub gain_curve: AirspyGain,
    /// Step along that curve, 0 (least gain) to 21 (most).
    pub gain_step: u8,
    /// The tuner's own AGC loops, one for the LNA and one for the mixer. Off by
    /// default: they fight a manual gain step, and the curves are what this
    /// receiver is usually driven by.
    pub lna_agc: bool,
    pub mixer_agc: bool,
    /// Bias tee on the antenna port. Off by default: putting phantom power on
    /// someone's feedline uninvited is not a good default.
    pub bias_tee: bool,
    /// 12-bit packing on the USB link.
    ///
    /// On by default, and it matters more here than it looks: at the top rate
    /// the ADC produces 20 Msps of real samples, which is 40 MB/s unpacked
    /// against 30 MB/s packed — and this is a USB 2.0 device, so the link has
    /// no headroom to spare. Firmware too old to have the request falls back to
    /// unpacked and says so.
    pub packing: bool,
    /// Remove the ADC's DC offset on the host.
    ///
    /// On by default. Worth knowing where the spur goes if you turn it off: the
    /// offset lands at the *edge* of the output span rather than its centre,
    /// because the signal is translated by a quarter of the sample rate on the
    /// way through. Turn it off to see raw hardware output.
    pub dc_block: bool,
    /// Bulk transfers in flight, and the size of each in KiB.
    pub transfers: u8,
    pub transfer_kib: u16,
}

impl Default for AirspyConfig {
    fn default() -> Self {
        AirspyConfig {
            serial: String::new(),
            // The R2's slower rate: present on every R2, and the one that asks
            // least of both the USB link and the host.
            sample_rate_hz: 2_500_000.0,
            gain_curve: AirspyGain::Linearity,
            // Mid-curve. High enough to hear something on a first plug-in, low
            // enough not to overload on a real antenna.
            gain_step: 11,
            lna_agc: false,
            mixer_agc: false,
            bias_tee: false,
            packing: true,
            dc_block: true,
            transfers: 16,
            transfer_kib: 128,
        }
    }
}

impl AirspyConfig {
    /// The one real gain element: a step along the selected curve. It is a
    /// *step*, not a dB figure — how much each one is worth depends on the
    /// curve and the band — so the settings tab shows it as a step control and
    /// this is what carries it.
    pub const GAIN_ELEMENT: &'static str = "GAIN";

    /// Pseudo-elements carrying settings that are not gains at all. They ride
    /// the existing `SetGain` command so this backend needs no new `Command`
    /// variant, no `DeviceCaps` field and no engine change; they are
    /// deliberately absent from `DeviceCaps::gains`, so nothing renders them as
    /// sliders.
    pub const CURVE_ELEMENT: &'static str = "CURVE";
    pub const LNA_AGC_ELEMENT: &'static str = "LNAAGC";
    pub const MIXER_AGC_ELEMENT: &'static str = "MIXAGC";
    pub const BIAS_TEE_ELEMENT: &'static str = "BIASTEE";
    pub const PACKING_ELEMENT: &'static str = "PACKING";
    pub const DC_BLOCK_ELEMENT: &'static str = "DCBLOCK";

    /// Steps along a gain curve.
    pub const GAIN_STEPS: u8 = 22;

    /// The **complex** rates any R2 or Mini is known to offer — the list the
    /// settings combo shows *before* a receiver has been opened. It is a menu,
    /// not a promise: the real list is queried from the device and published in
    /// `DeviceCaps::sample_rates`, and the settings tab prefers that whenever
    /// one is connected.
    pub const SAMPLE_RATES: [f64; 4] = [2.5e6, 3.0e6, 6.0e6, 10.0e6];

    /// Which model a rate belongs to, for the pre-open combo. An R2 and a Mini
    /// cannot be told apart on the bus, so the list has to cover both and say
    /// which is which.
    pub fn rate_note(rate_hz: f64) -> &'static str {
        match rate_hz as u32 {
            10_000_000 => "R2 — 40 MB/s over USB, 30 packed",
            6_000_000 => "Mini — 24 MB/s, 18 packed",
            3_000_000 => "Mini — 12 MB/s, 9 packed",
            2_500_000 => "R2 — 10 MB/s, 7.5 packed",
            _ => "",
        }
    }

    /// Tuning range, in Hz. Fixed by the R820T2 and the same on both models.
    pub const FREQ_RANGE: (f64, f64) = (24.0e6, 1_800.0e6);
}

/// Which of the R828D's two curated gain curves to drive the front end from.
///
/// The same two curves the Airspy R2 publishes, and the same numbers: HydraSDR
/// forked libairspy's tables unchanged, because the tuner change from R820T2 to
/// R828D left the three stages' ranges alone. Its own type rather than a shared
/// one all the same — these are HydraSDR's tables now, and a firmware that
/// retunes them should not have to move the Airspy's with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum HydraSdrGain {
    /// Least intermodulation for a given sensitivity — the right default on an
    /// antenna with broadcast nearby.
    #[default]
    Linearity,
    /// More sensitivity, less overload margin.
    Sensitivity,
}

impl HydraSdrGain {
    pub fn code(self) -> u8 {
        match self {
            HydraSdrGain::Linearity => 0,
            HydraSdrGain::Sensitivity => 1,
        }
    }

    pub fn from_code(code: u8) -> HydraSdrGain {
        match code {
            1 => HydraSdrGain::Sensitivity,
            _ => HydraSdrGain::Linearity,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            HydraSdrGain::Linearity => "Linearity (best strong-signal handling)",
            HydraSdrGain::Sensitivity => "Sensitivity (best weak-signal)",
        }
    }

    pub const ALL: [HydraSdrGain; 2] = [HydraSdrGain::Linearity, HydraSdrGain::Sensitivity];
}

/// Which of the RFOne's three RF input sockets the tuner is connected to.
///
/// Nothing the Airspy this driver was forked from has. The firmware names them
/// `ANT`, `CABLE1` and `CABLE2`, and publishes the bias tee on the first alone
/// — so the port and the bias tee are one decision, not two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum HydraSdrPort {
    /// The antenna SMA, and the only socket with a bias tee behind it.
    #[default]
    Ant,
    Cable1,
    Cable2,
}

impl HydraSdrPort {
    pub fn code(self) -> u8 {
        match self {
            HydraSdrPort::Ant => 0,
            HydraSdrPort::Cable1 => 1,
            HydraSdrPort::Cable2 => 2,
        }
    }

    pub fn from_code(code: u8) -> HydraSdrPort {
        match code {
            1 => HydraSdrPort::Cable1,
            2 => HydraSdrPort::Cable2,
            _ => HydraSdrPort::Ant,
        }
    }

    /// The name silk-screened on the board, which is also what the firmware
    /// calls it.
    pub fn name(self) -> &'static str {
        match self {
            HydraSdrPort::Ant => "ANT",
            HydraSdrPort::Cable1 => "CABLE1",
            HydraSdrPort::Cable2 => "CABLE2",
        }
    }

    /// Whether this socket can carry the bias tee. Only `ANT` can — asking for
    /// DC on either cable port is a request the firmware takes and the hardware
    /// ignores, which is worse than a control that says no.
    pub fn has_bias_tee(self) -> bool {
        matches!(self, HydraSdrPort::Ant)
    }

    pub const ALL: [HydraSdrPort; 3] =
        [HydraSdrPort::Ant, HydraSdrPort::Cable1, HydraSdrPort::Cable2];
}

/// A HydraSDR RFOne seen on the USB bus. Wasm-safe so it can cross the
/// `RadioController` trait to the settings UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HydraSdrDevice {
    /// The USB serial descriptor with its `HYDRASDR SN:` prefix stripped, when
    /// it has one — which is what is printed on the board and what an operator
    /// would type.
    pub serial: Option<String>,
    /// The USB product string, else a generic name.
    pub name: String,
    /// Whether this board came up on `1d50:60a1`, the pair it shares with the
    /// Airspy R2 and Mini.
    ///
    /// Only the prototypes do — production boards have HydraSDR's own
    /// `38af:0001`. Worth saying out loud in the device list all the same: it
    /// is the one fact about this receiver that can send somebody to the wrong
    /// interface, in either direction.
    pub legacy_usb_id: bool,
}

impl HydraSdrDevice {
    /// One-line label for the selection UI.
    pub fn label(&self) -> String {
        let legacy =
            if self.legacy_usb_id { "  [legacy USB id, shared with Airspy R2]" } else { "" };
        match &self.serial {
            Some(s) => {
                let tail = &s[s.len().saturating_sub(8)..];
                format!("{}  (serial …{tail}){legacy}", self.name)
            }
            // Without a serial we can only ever open "the first one", so say so
            // rather than implying this entry can be pinned.
            None => format!("{}  [no serial — first match only]{legacy}", self.name),
        }
    }
}

/// HydraSDR RFOne (USB) backend configuration. Receive only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HydraSdrConfig {
    /// Pin a receiver by its USB serial; empty means "the first one found".
    /// Matched on the **suffix**, so the last eight digits are enough — and the
    /// `HYDRASDR SN:` prefix may be left on or taken off either way.
    pub serial: String,
    /// **Complex** sample rate in Hz — what you get, not what the ADC runs at.
    /// The receiver is programmed at twice this, because its ADC is real and
    /// the host makes complex baseband from it. Snapped to a rate the receiver
    /// actually has at open; see [`Self::SAMPLE_RATES`].
    pub sample_rate_hz: f64,
    /// Which curated gain curve the three tuner stages follow.
    pub gain_curve: HydraSdrGain,
    /// Step along that curve, 0 (least gain) to 21 (most).
    pub gain_step: u8,
    /// The tuner's own AGC loops, one for the LNA and one for the mixer. Off by
    /// default: they fight a manual gain step, and the curves are what this
    /// receiver is usually driven by.
    pub lna_agc: bool,
    pub mixer_agc: bool,
    /// Which of the three RF sockets the tuner sees.
    pub rf_port: HydraSdrPort,
    /// Bias tee on the antenna port. Off by default: putting phantom power on
    /// someone's feedline uninvited is not a good default. Has no effect while
    /// the tuner is on a cable port, which is where the hardware puts it.
    pub bias_tee: bool,
    /// 12-bit packing on the USB link.
    ///
    /// On by default, and it matters here: at the top rate the ADC produces 24
    /// Msps of real samples, which is 48 MB/s unpacked against 36 MB/s packed —
    /// and this is a USB 2.0 device, so the link has no headroom to spare.
    /// Firmware too old to have the request falls back to unpacked and says so.
    pub packing: bool,
    /// Remove the ADC's DC offset on the host.
    ///
    /// On by default. Worth knowing where the spur goes if you turn it off: the
    /// offset lands at the *edge* of the output span rather than its centre,
    /// because the signal is translated by a quarter of the sample rate on the
    /// way through. Turn it off to see raw hardware output.
    pub dc_block: bool,
    /// Bulk transfers in flight, and the size of each in KiB.
    pub transfers: u8,
    pub transfer_kib: u16,
}

impl Default for HydraSdrConfig {
    fn default() -> Self {
        HydraSdrConfig {
            serial: String::new(),
            // The middle of the three rates every RFOne lists: a real 10 MHz of
            // span without asking the USB link or the host for everything they
            // have on a first plug-in.
            sample_rate_hz: 5_000_000.0,
            gain_curve: HydraSdrGain::Linearity,
            // `rfone_gain_defs`' own starting point.
            gain_step: 10,
            lna_agc: false,
            mixer_agc: false,
            rf_port: HydraSdrPort::Ant,
            bias_tee: false,
            packing: true,
            dc_block: true,
            transfers: 16,
            transfer_kib: 128,
        }
    }
}

impl HydraSdrConfig {
    /// The one real gain element: a step along the selected curve. It is a
    /// *step*, not a dB figure — how much each one is worth depends on the
    /// curve and the band — so the settings tab shows it as a step control and
    /// this is what carries it.
    pub const GAIN_ELEMENT: &'static str = "GAIN";

    /// Pseudo-elements carrying settings that are not gains at all. They ride
    /// the existing `SetGain` command so this backend needs no new `Command`
    /// variant, no `DeviceCaps` field and no engine change; they are
    /// deliberately absent from `DeviceCaps::gains`, so nothing renders them as
    /// sliders.
    pub const CURVE_ELEMENT: &'static str = "CURVE";
    pub const LNA_AGC_ELEMENT: &'static str = "LNAAGC";
    pub const MIXER_AGC_ELEMENT: &'static str = "MIXAGC";
    pub const RF_PORT_ELEMENT: &'static str = "RFPORT";
    pub const BIAS_TEE_ELEMENT: &'static str = "BIASTEE";
    pub const PACKING_ELEMENT: &'static str = "PACKING";
    pub const DC_BLOCK_ELEMENT: &'static str = "DCBLOCK";

    /// Steps along a gain curve (`RFONE_GAIN_TABLE_SIZE`).
    pub const GAIN_STEPS: u8 = 22;

    /// The **complex** rates an RFOne has, widest first — the list the settings
    /// combo shows *before* a receiver has been opened.
    ///
    /// Only three of these are ones the receiver will admit to. `GET_SAMPLERATES`
    /// reports the firmware's primary configurations — 10, 5 and 2.5 Msps — and
    /// says nothing about the alternate table behind them, which carries 12, 8,
    /// 6 and 4.096. Those are reached by naming the ADC rate in kilohertz, and
    /// a driver that offered only what was listed would leave the top of this
    /// radio's range unreachable.
    pub const SAMPLE_RATES: [f64; 7] = [12.0e6, 10.0e6, 8.0e6, 6.0e6, 5.0e6, 4.096e6, 2.5e6];

    /// What a rate costs on the USB link, and whether the receiver lists it.
    pub fn rate_note(rate_hz: f64) -> &'static str {
        match rate_hz as u32 {
            12_000_000 => "alternate — 48 MB/s over USB, 36 packed",
            10_000_000 => "listed — 40 MB/s, 30 packed",
            8_000_000 => "alternate — 32 MB/s, 24 packed",
            6_000_000 => "alternate — 24 MB/s, 18 packed",
            5_000_000 => "listed — 20 MB/s, 15 packed",
            4_096_000 => "alternate — 16.4 MB/s, 12.3 packed",
            2_500_000 => "listed — 10 MB/s, 7.5 packed",
            _ => "",
        }
    }

    /// Tuning range, in Hz: `RFONE_MIN_FREQ_HZ`..`RFONE_MAX_FREQ_HZ`.
    pub const FREQ_RANGE: (f64, f64) = (24.0e6, 1_800.0e6);
}

/// AD9361 receive AGC mode. The names are the IIO `gain_control_mode` values,
/// which is what actually goes on the wire.
///
/// SoapySDR can only say "AGC on" or "AGC off"; the part itself has four modes
/// and they behave very differently on the air, which is one of the reasons
/// this backend is native rather than a SoapySDR device string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PlutoAgc {
    /// The gain slider is in charge.
    Manual,
    /// Rides slowly over a signal — the right default for SSB and CW, where a
    /// fast AGC pumps on every syllable.
    #[default]
    SlowAttack,
    /// Reacts within a burst. Wanted where signals appear suddenly and at very
    /// different strengths.
    FastAttack,
    /// Digital AGC with an analog fast-attack safety net.
    Hybrid,
}

impl PlutoAgc {
    pub const ALL: [PlutoAgc; 4] =
        [PlutoAgc::Manual, PlutoAgc::SlowAttack, PlutoAgc::FastAttack, PlutoAgc::Hybrid];

    /// What the IIO attribute is set to.
    pub fn iio_name(self) -> &'static str {
        match self {
            PlutoAgc::Manual => "manual",
            PlutoAgc::SlowAttack => "slow_attack",
            PlutoAgc::FastAttack => "fast_attack",
            PlutoAgc::Hybrid => "hybrid",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PlutoAgc::Manual => "Manual",
            PlutoAgc::SlowAttack => "Slow attack",
            PlutoAgc::FastAttack => "Fast attack",
            PlutoAgc::Hybrid => "Hybrid",
        }
    }

    /// Numeric code carried on [`PlutoConfig::AGC_ELEMENT`], so the mode rides
    /// the existing `SetGain` command instead of needing one of its own.
    pub fn code(self) -> f64 {
        match self {
            PlutoAgc::Manual => 0.0,
            PlutoAgc::SlowAttack => 1.0,
            PlutoAgc::FastAttack => 2.0,
            PlutoAgc::Hybrid => 3.0,
        }
    }

    pub fn from_code(v: f64) -> PlutoAgc {
        match v.round() as i32 {
            0 => PlutoAgc::Manual,
            2 => PlutoAgc::FastAttack,
            3 => PlutoAgc::Hybrid,
            _ => PlutoAgc::SlowAttack,
        }
    }
}

/// Which duplex the AD9361's enable state machine runs in.
///
/// A Pluto arrives in FDD, where receive and transmit are enabled together and
/// each has a synthesiser of its own. That is the right mode for anything that
/// listens while it talks — a QO-100 station hearing its own downlink — and it
/// is what this backend has always left the board in.
///
/// TDD enables one direction at a time, which is what the part's GPO pins are
/// slaved to: [`PlutoPtt`] can only key an external amplifier in TDD, because
/// in FDD the transmit-enable line the GPO follows is asserted the whole time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PlutoDuplex {
    /// Leave the board as it boots: both directions enabled at once.
    #[default]
    Fdd,
    /// One direction at a time, with the enable state machine driven from
    /// here — the mode the GPO PTT lines need.
    Tdd,
}

impl PlutoDuplex {
    pub const ALL: [PlutoDuplex; 2] = [PlutoDuplex::Fdd, PlutoDuplex::Tdd];

    pub fn label(self) -> &'static str {
        match self {
            PlutoDuplex::Fdd => "FDD (both at once)",
            PlutoDuplex::Tdd => "TDD (one at a time)",
        }
    }

    /// What `adi,frequency-division-duplex-mode-enable` is set to.
    pub fn fdd_enable(self) -> &'static str {
        match self {
            PlutoDuplex::Fdd => "1",
            PlutoDuplex::Tdd => "0",
        }
    }
}

/// Which pair of the Pluto's four GPO pins follows the transmit/receive state,
/// for keying an external power amplifier, LNA or transmit-receive switch.
///
/// The AD9361 can slave any GPO to its receive-enable or transmit-enable line,
/// so a pair gives a complementary drive: one pin high on receive, the other
/// high on transmit — exactly what a T/R relay and a PA's key line want, with
/// no host software in the loop once it is set up.
///
/// It only works in [`PlutoDuplex::Tdd`], and the pins are 1.3 V at a few
/// milliamps: they drive a transistor or an opto-isolator, never a relay coil
/// directly. Analog Devices' own note uses GPO0/GPO1 for an external LNA, which
/// is why GPO2/GPO3 are offered as well — a board already wired for eLNA
/// control keeps those two and puts PTT on the other pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PlutoPtt {
    /// Leave every GPO alone.
    #[default]
    Off,
    /// GPO0 high on receive, GPO1 high on transmit.
    Gpo01,
    /// GPO2 high on receive, GPO3 high on transmit — the pair to use when
    /// GPO0/GPO1 already drive an external LNA.
    Gpo23,
}

impl PlutoPtt {
    pub const ALL: [PlutoPtt; 3] = [PlutoPtt::Off, PlutoPtt::Gpo01, PlutoPtt::Gpo23];

    pub fn label(self) -> &'static str {
        match self {
            PlutoPtt::Off => "Off",
            PlutoPtt::Gpo01 => "GPO0 = RX, GPO1 = TX",
            PlutoPtt::Gpo23 => "GPO2 = RX, GPO3 = TX",
        }
    }

    /// The (receive, transmit) GPO numbers, or `None` when this is off.
    pub fn pins(self) -> Option<(u8, u8)> {
        match self {
            PlutoPtt::Off => None,
            PlutoPtt::Gpo01 => Some((0, 1)),
            PlutoPtt::Gpo23 => Some((2, 3)),
        }
    }
}

/// ADALM-Pluto (PlutoSDR) backend configuration.
///
/// The device is reached over the network — which the USB cable already
/// provides, as an Ethernet gadget — so this is an address, not a serial.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlutoConfig {
    /// `host[:port]`, defaulting to the USB gadget's device end. Blank falls
    /// back to [`Self::selected_ip`], then to the default address.
    pub address: String,
    /// IP of the Pluto picked from a discovery scan (persisted selection).
    pub selected_ip: Option<String>,
    /// Sample rate in Hz. The AD9361 reaches 61.44 Msps; a USB 2.0 Ethernet
    /// gadget does not, which is what [`Self::SAMPLE_RATES`] is scaled to.
    pub sample_rate_hz: f64,
    /// Analog filter bandwidth in Hz, or `0.0` for automatic (0.9 × the sample
    /// rate). Automatic is deliberately wide: the engine parks the LO a quarter
    /// of a span off the dial to keep the signal clear of a zero-IF part's DC
    /// spike, and a narrow analog filter is what makes it give that up.
    pub rf_bandwidth_hz: f64,
    /// Receive gain in dB when the AGC is in manual.
    pub rx_gain_db: f64,
    pub agc: PlutoAgc,
    /// Transmit gain in dB — negative, because the AD9361 expresses it as
    /// attenuation. `0` is full output. The default is well down: this is a
    /// transmitter, and a first key-up should not be a surprise.
    pub tx_gain_db: f64,
    /// `rf_port_select` for receive; empty leaves the device's own choice. A
    /// Pluto wires only `A_BALANCED`, but the AD9361 has nine and a custom
    /// board may use another.
    pub rx_port: String,
    /// `rf_port_select` for transmit; empty leaves the device's own choice.
    pub tx_port: String,
    /// Reference error in parts per million. Applied in software to every
    /// requested LO — the device's own `xo_correction` is persistent, and
    /// writing it would outlive the session.
    pub ppm: f64,
    /// Device-side buffer length in complex samples (advanced). 32768 is ~16 ms
    /// at 2 Msps: long enough that the per-buffer round trip is not the
    /// bottleneck, short enough that a retune is not visibly late.
    pub buffer_samples: usize,
    /// Which of the device's receive chains this radio runs, 0-based. A 2R2T
    /// firmware (a Pluto+) streams two; two radios on the same address share
    /// one connection, each with its own chain — **and the one LO**: the
    /// AD9361's chains share a synthesiser, so retuning either radio moves
    /// both, and the second chain is a second antenna, not a second
    /// frequency. The transmitter belongs to chain 0's radio. Defaults keep
    /// every existing `radio.json` on chain 0, exactly as before.
    #[serde(default)]
    pub rx: u8,
    /// Keep receiving through an over, instead of standing receive down for
    /// its length.
    ///
    /// The AD9361 is a full-duplex part in FDD, with a synthesiser for each
    /// direction, so the limit is never the silicon — it is the link. Both
    /// streams together are twice the sample rate in 16-bit I/Q: 2.5 Msps is
    /// 10 MB/s each way, which a Pluto's USB 2.0 Ethernet gadget cannot carry
    /// and 100BASE-TX cannot either. On a board with real Ethernet (a
    /// LibreSDR, a Pluto on a gigabit adapter) there is room, and then the
    /// operator hears the receiver through their own over — which is how a
    /// QO-100 station listens to its own downlink.
    ///
    /// Off by default because the failure is not a refusal but a bad signal:
    /// a link that cannot carry both directions starves the transmit buffer,
    /// and what goes on the air is chopped.
    #[serde(default)]
    pub full_duplex: bool,
    /// Which duplex to put the AD9361's enable state machine in. FDD is how a
    /// Pluto boots and what every earlier version of this backend left alone;
    /// TDD is what [`Self::ptt_gpo`] needs, and it rules out
    /// [`Self::full_duplex`] — the part is only ever doing one of the two.
    #[serde(default)]
    pub duplex: PlutoDuplex,
    /// Which GPO pair keys an external PA, LNA or T/R switch. Needs
    /// [`PlutoDuplex::Tdd`]: the lines these pins follow are both asserted the
    /// whole time in FDD, so nothing would ever toggle.
    #[serde(default)]
    pub ptt_gpo: PlutoPtt,
}

impl Default for PlutoConfig {
    fn default() -> Self {
        PlutoConfig {
            address: PlutoConfig::DEFAULT_ADDRESS.into(),
            selected_ip: None,
            // Above `NO_FIR_FLOOR_HZ`, so a stock Pluto can actually produce
            // it. The 2 Msps this used to be could not be, and an out-of-the-box
            // radio refused to open at its own default settings.
            sample_rate_hz: 2_500_000.0,
            rf_bandwidth_hz: 0.0,
            rx_gain_db: 40.0,
            agc: PlutoAgc::SlowAttack,
            tx_gain_db: -20.0,
            rx_port: String::new(),
            tx_port: String::new(),
            ppm: 0.0,
            buffer_samples: 32768,
            rx: 0,
            full_duplex: false,
            duplex: PlutoDuplex::default(),
            ptt_gpo: PlutoPtt::default(),
        }
    }
}

impl PlutoConfig {
    /// Where an out-of-the-box Pluto lives: the device end of the USB Ethernet
    /// gadget (the host takes 192.168.2.10 on the same link).
    pub const DEFAULT_ADDRESS: &'static str = "192.168.2.1";

    /// Gain elements this backend exposes. They live here rather than in
    /// `sdroxide-pluto` so the (wasm-safe) settings UI can address them without
    /// depending on the native backend crate — same reason as
    /// [`HpsdrConfig::LNA_GAIN_ELEMENT`].
    pub const RF_GAIN_ELEMENT: &'static str = "RF";
    pub const TX_GAIN_ELEMENT: &'static str = "TXATT";
    /// Pseudo-elements carrying settings that are not gains at all. They ride
    /// the existing `SetGain` command so this backend needs no new `Command`
    /// variant, and are deliberately absent from `DeviceCaps::gains` so nothing
    /// renders them as sliders — the Pluto settings panel drives them directly.
    pub const AGC_ELEMENT: &'static str = "AGC";
    pub const PPM_ELEMENT: &'static str = "PPM";

    /// Sample rates offered in the UI.
    ///
    /// The floor is the AD9361's own (521 ksps, through its internal FIR
    /// decimator). The ceiling is not the part's 61.44 Msps but what a USB 2.0
    /// Ethernet gadget will actually carry: 2 Msps of 16-bit I/Q is 64 Mbit/s
    /// before framing, which is already most of the link.
    ///
    /// The entries below [`Self::NO_FIR_FLOOR_HZ`] need a FIR configuration
    /// loaded into the part, which sdroxide does not do — a stock Pluto rounds
    /// them all up to that floor and says so on connect. They are still offered
    /// because a board someone else has configured, or an IIO device that is
    /// not a Pluto at all, can honour them.
    pub const SAMPLE_RATES: [f64; 6] =
        [521_000.0, 1_000_000.0, 2_000_000.0, 2_500_000.0, 3_840_000.0, 5_000_000.0];

    /// The lowest rate an AD936x can produce with its FIR decimator bypassed,
    /// which is how a Pluto arrives and how sdroxide leaves it.
    ///
    /// The part's clock-chain solver accepts a rate only if `rate × 12` clears
    /// the ADC's 25 MHz minimum, so the true floor is 25 MHz / 12 = 2083333.33
    /// Hz — and the driver publishes that range through integer division, so it
    /// advertises 2083333 and then refuses it. This is the first integer that
    /// actually works.
    pub const NO_FIR_FLOOR_HZ: f64 = 2_083_334.0;

    /// The address to open: the typed one, else a discovered selection, else
    /// the USB gadget's default.
    pub fn target(&self) -> String {
        let typed = self.address.trim();
        if !typed.is_empty() {
            return typed.to_string();
        }
        match self.selected_ip.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(ip) => ip.to_string(),
            None => PlutoConfig::DEFAULT_ADDRESS.to_string(),
        }
    }

    /// Apply the reference trim to a frequency, the same way
    /// [`HpsdrConfig::apply_ppm`] does.
    pub fn apply_ppm(hz: f64, ppm: f64) -> f64 {
        hz * (1.0 + ppm / 1e6)
    }
}

/// A Pluto found on the network (or confirmed at a typed address).
///
/// Wasm-safe so it can cross the `RadioController` trait to the settings UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlutoDevice {
    pub ip: String,
    /// mDNS instance or host name, when discovery supplied one.
    pub hostname: String,
    /// The `hw_model` context attribute, e.g.
    /// "Analog Devices PlutoSDR Rev.B (Z7010-AD9364)".
    pub model: String,
    pub firmware: String,
    pub serial: String,
    /// libiio version the device's `iiod` reports.
    pub iiod_version: String,
}

impl PlutoDevice {
    /// One-line label for the selection UI.
    pub fn label(&self) -> String {
        let what = if self.model.is_empty() { "PlutoSDR" } else { self.model.as_str() };
        let mut s = format!("{what}  ({})", self.ip);
        if !self.firmware.is_empty() {
            s.push_str(&format!("  firmware {}", self.firmware));
        }
        s
    }

    /// Whether the model string names the AD9364 — the 70 MHz–6 GHz part an
    /// unlocked Pluto reports. Only a hint for the label; the real limits are
    /// read off the device when it is opened.
    pub fn is_ad9364(&self) -> bool {
        self.model.contains("AD9364")
    }
}

/// Which RSP the `sdrplay_api` service says a device is, from the `hwVer`
/// byte it reports. The numbering is the API's, not sequential — RSP1A is 255
/// because it was added after the RSP2 had already taken 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SdrPlayModel {
    Rsp1,
    Rsp1a,
    Rsp1b,
    Rsp2,
    RspDuo,
    RspDx,
    RspDxR2,
    Unknown,
}

impl SdrPlayModel {
    pub fn from_hw_ver(hw_ver: u8) -> SdrPlayModel {
        match hw_ver {
            1 => SdrPlayModel::Rsp1,
            2 => SdrPlayModel::Rsp2,
            3 => SdrPlayModel::RspDuo,
            4 => SdrPlayModel::RspDx,
            6 => SdrPlayModel::Rsp1b,
            7 => SdrPlayModel::RspDxR2,
            255 => SdrPlayModel::Rsp1a,
            _ => SdrPlayModel::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SdrPlayModel::Rsp1 => "RSP1",
            SdrPlayModel::Rsp1a => "RSP1A",
            SdrPlayModel::Rsp1b => "RSP1B",
            SdrPlayModel::Rsp2 => "RSP2",
            SdrPlayModel::RspDuo => "RSPduo",
            SdrPlayModel::RspDx => "RSPdx",
            SdrPlayModel::RspDxR2 => "RSPdx R2",
            SdrPlayModel::Unknown => "RSP (unknown model)",
        }
    }

    /// Highest LNA state the model has in *any* band.
    ///
    /// A ceiling, not an offer: on an RSPdx it is 27, and only the 250–420 MHz
    /// band has all of those — below 12 MHz there are 19. Use it where the
    /// band is not known (a device that is not open yet); anywhere the dial is
    /// known, [`Self::max_lna_state_on`] is the honest answer and is what the
    /// sliders are built from.
    pub fn max_lna_state(self) -> u8 {
        match self {
            SdrPlayModel::Rsp1 => 3,
            SdrPlayModel::Rsp1a | SdrPlayModel::Rsp1b => 9,
            SdrPlayModel::Rsp2 => 8,
            SdrPlayModel::RspDuo => 9,
            SdrPlayModel::RspDx | SdrPlayModel::RspDxR2 => 27,
            // An unknown model still has the API-guaranteed minimum.
            SdrPlayModel::Unknown => 3,
        }
    }

    /// Highest LNA state this model has **on this band**, which is the number
    /// a control should offer.
    ///
    /// State 0 is maximum gain; each step up switches more attenuation in
    /// front of the tuner. How many steps exist is a property of the band, not
    /// of the box: the counts here are the API headers' `*_NUM_LNA_STATES_*`
    /// defines, and the band edges are the rows of the gain tables in the API
    /// specification (v3.15, pp. 38–39). A state past this is refused by the
    /// service with `OutOfRange`, so the driver clamps to it — and a slider
    /// built from [`Self::max_lna_state`] instead spends its top third moving
    /// nothing and snapping back.
    ///
    /// `hiz` says the Hi-Z input is routed (RSP2 and RSPduo tuner 1 only),
    /// which has fewer front-end states of its own. `hdr` is the RSPdx's
    /// high-dynamic-range path, which has its own table below 2 MHz.
    ///
    /// Lives here rather than beside the driver so the settings UI — which is
    /// wasm-safe and cannot link the backend — builds its slider from the same
    /// table the clamp uses. Two copies of this drifting apart is exactly the
    /// bug it prevents.
    pub fn max_lna_state_on(self, freq_hz: f64, hiz: bool, hdr: bool) -> u8 {
        let f = freq_hz;
        match self {
            SdrPlayModel::Rsp1 => {
                if f >= 1_000e6 {
                    2 // 3 states in L-band
                } else {
                    3 // 4 states everywhere else
                }
            }
            SdrPlayModel::Rsp1a | SdrPlayModel::Rsp1b => {
                if f < 60e6 {
                    6 // RSPIA_NUM_LNA_STATES_AM = 7
                } else if f >= 1_000e6 {
                    8 // RSPIA_NUM_LNA_STATES_LBAND = 9
                } else {
                    9 // RSPIA_NUM_LNA_STATES = 10
                }
            }
            SdrPlayModel::Rsp2 => {
                if hiz {
                    4 // RSPII_NUM_LNA_STATES_AMPORT = 5
                } else if f >= 420e6 {
                    5 // RSPII_NUM_LNA_STATES_420MHZ = 6
                } else {
                    8 // RSPII_NUM_LNA_STATES = 9
                }
            }
            SdrPlayModel::RspDuo => {
                if hiz {
                    4 // RSPDUO_NUM_LNA_STATES_AMPORT = 5
                } else if f < 60e6 {
                    6 // RSPDUO_NUM_LNA_STATES_AM = 7
                } else if f >= 1_000e6 {
                    8 // RSPDUO_NUM_LNA_STATES_LBAND = 9
                } else {
                    9 // RSPDUO_NUM_LNA_STATES = 10
                }
            }
            SdrPlayModel::RspDx | SdrPlayModel::RspDxR2 => {
                if hdr && f < 2e6 {
                    21 // RSPDX_NUM_LNA_STATES_DX = 22
                } else if f < 12e6 {
                    18 // RSPDX_NUM_LNA_STATES_AMPORT2_0_12 = 19
                } else if f < 50e6 {
                    19 // RSPDX_NUM_LNA_STATES_AMPORT2_12_50 = 20
                } else if f < 60e6 {
                    24 // RSPDX_NUM_LNA_STATES_AMPORT2_50_60 = 25
                } else if f < 250e6 {
                    26 // RSPDX_NUM_LNA_STATES_VHF_BAND3 = 27
                } else if f < 420e6 {
                    27 // RSPDX_NUM_LNA_STATES = 28
                } else if f < 1_000e6 {
                    20 // RSPDX_NUM_LNA_STATES_420MHZ = 21
                } else {
                    18 // RSPDX_NUM_LNA_STATES_LBAND = 19
                }
            }
            // The API guarantees nothing about a model these bindings do not
            // know; every RSP has at least the RSP1's four states.
            SdrPlayModel::Unknown => 3,
        }
    }

    /// Whether an antenna choice routes a Hi-Z input, for the LNA clamp.
    ///
    /// Beside the table above because it is an argument to it, and the UI
    /// needs both for the same reason.
    pub fn antenna_is_hiz(self, antenna: &str) -> bool {
        match self {
            SdrPlayModel::Rsp2 => antenna == "Hi-Z",
            SdrPlayModel::RspDuo => antenna == "Hi-Z port",
            _ => false,
        }
    }

    /// Whether the model has a switchable bias tee. The original RSP1 is the
    /// only one without.
    pub fn has_bias_tee(self) -> bool {
        !matches!(self, SdrPlayModel::Rsp1 | SdrPlayModel::Unknown)
    }

    /// Whether the model has the FM-broadcast notch filter.
    pub fn has_rf_notch(self) -> bool {
        !matches!(self, SdrPlayModel::Rsp1 | SdrPlayModel::Unknown)
    }

    /// Whether the model has the separate DAB notch filter.
    pub fn has_dab_notch(self) -> bool {
        matches!(
            self,
            SdrPlayModel::Rsp1a
                | SdrPlayModel::Rsp1b
                | SdrPlayModel::RspDuo
                | SdrPlayModel::RspDx
                | SdrPlayModel::RspDxR2
        )
    }

    /// Whether the model has the RSPdx HDR mode (a second, higher-linearity
    /// signal path below 2 MHz).
    pub fn has_hdr(self) -> bool {
        matches!(self, SdrPlayModel::RspDx | SdrPlayModel::RspDxR2)
    }

    /// Antenna ports the operator can choose between, for `DeviceCaps`. Empty
    /// means one fixed port — the selector stays hidden, like every other
    /// single-port backend. The RSPduo's choice depends on which tuner is in
    /// use: tuner 1 has both a 50 Ω and a Hi-Z port, tuner 2 only its own.
    pub fn antennas(self, duo_tuner: SdrPlayDuoTuner) -> &'static [&'static str] {
        match self {
            SdrPlayModel::Rsp2 => &["Antenna A", "Antenna B", "Hi-Z"],
            SdrPlayModel::RspDx | SdrPlayModel::RspDxR2 => &["Antenna A", "Antenna B", "Antenna C"],
            SdrPlayModel::RspDuo => match duo_tuner {
                SdrPlayDuoTuner::Tuner1 => &["50 Ohm port", "Hi-Z port"],
                SdrPlayDuoTuner::Tuner2 => &[],
            },
            _ => &[],
        }
    }
}

/// RSP hardware AGC loop rate. The loop runs in the tuner's IF stage, driven
/// by the API service; `Off` hands the IF gain slider back to the operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SdrPlayAgc {
    Off,
    Hz5,
    #[default]
    Hz50,
    Hz100,
}

impl SdrPlayAgc {
    pub const ALL: [SdrPlayAgc; 4] =
        [SdrPlayAgc::Off, SdrPlayAgc::Hz5, SdrPlayAgc::Hz50, SdrPlayAgc::Hz100];

    pub fn label(self) -> &'static str {
        match self {
            SdrPlayAgc::Off => "Off (manual IF gain)",
            SdrPlayAgc::Hz5 => "5 Hz",
            SdrPlayAgc::Hz50 => "50 Hz",
            SdrPlayAgc::Hz100 => "100 Hz",
        }
    }

    /// Numeric code carried on [`SdrPlayConfig::AGC_ELEMENT`], so the mode
    /// rides the existing `SetGain` command instead of needing one of its own.
    /// The values are the API's own `sdrplay_api_AgcControlT` numbers — note
    /// they are not in speed order — so the two ends cannot drift.
    pub fn code(self) -> f64 {
        match self {
            SdrPlayAgc::Off => 0.0,
            SdrPlayAgc::Hz100 => 1.0,
            SdrPlayAgc::Hz50 => 2.0,
            SdrPlayAgc::Hz5 => 3.0,
        }
    }

    pub fn from_code(v: f64) -> SdrPlayAgc {
        match v.round() as i32 {
            0 => SdrPlayAgc::Off,
            1 => SdrPlayAgc::Hz100,
            3 => SdrPlayAgc::Hz5,
            // Anything unrecognised lands on the safe default rather than
            // manual, which on an unknown band would be a deaf or overloaded
            // receiver.
            _ => SdrPlayAgc::Hz50,
        }
    }
}

/// Which RSPduo tuner to run (single-tuner mode; the second tuner idles).
/// Changing it reopens the device — the choice is fixed when the tuner is
/// selected, before streaming starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SdrPlayDuoTuner {
    #[default]
    Tuner1,
    Tuner2,
}

impl SdrPlayDuoTuner {
    pub const ALL: [SdrPlayDuoTuner; 2] = [SdrPlayDuoTuner::Tuner1, SdrPlayDuoTuner::Tuner2];

    pub fn label(self) -> &'static str {
        match self {
            SdrPlayDuoTuner::Tuner1 => "Tuner 1 (50 Ohm / Hi-Z)",
            SdrPlayDuoTuner::Tuner2 => "Tuner 2 (50 Ohm)",
        }
    }

    /// The other one. There are two, so which tuner carries the second aerial
    /// is arithmetic rather than a setting — see [`SdrPlayDuo`].
    pub fn other(self) -> SdrPlayDuoTuner {
        match self {
            SdrPlayDuoTuner::Tuner1 => SdrPlayDuoTuner::Tuner2,
            SdrPlayDuoTuner::Tuner2 => SdrPlayDuoTuner::Tuner1,
        }
    }

    /// A short name for logs and labels: "tuner 1" / "tuner 2".
    pub fn short_label(self) -> &'static str {
        match self {
            SdrPlayDuoTuner::Tuner1 => "tuner 1",
            SdrPlayDuoTuner::Tuner2 => "tuner 2",
        }
    }
}

/// What an RSPduo's **second** tuner is for (issue #153, #165).
///
/// Appended-to rather than reordered: this is serde-serialised into
/// `radio.json` by name, but it also rides the wire, where the variant
/// *index* is what is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SdrPlayDuoRole {
    /// Combined with the first: a second aerial, to null a noise source or to
    /// ride out a fade. See [`DiversityMode`].
    #[default]
    Diversity,
    /// A radio of its own, on its own frequency — HF on one tuner and VHF on
    /// the other, in two tabs, from one board.
    ///
    /// Both radios must be configured for it: whichever opens the device
    /// first is what puts it in dual-tuner mode, and a tuner is only ever
    /// handed to a second radio by a session that was opened expecting one.
    SecondRadio,
}

impl SdrPlayDuoRole {
    pub const ALL: [SdrPlayDuoRole; 2] = [SdrPlayDuoRole::Diversity, SdrPlayDuoRole::SecondRadio];

    pub fn label(self) -> &'static str {
        match self {
            SdrPlayDuoRole::Diversity => "A second aerial (diversity / QRM suppression)",
            SdrPlayDuoRole::SecondRadio => "A second radio, on its own frequency",
        }
    }
}

/// The RSPduo's **second** tuner, and what is done with it (issue #153).
///
/// An RSPduo is two complete tuners on one board sharing one reference clock
/// and one ADC clock. Run both and they hear their spans at the same instant
/// from the same clock, which is what makes two aerials on them *coherent* —
/// a relative phase set by the aerials and the feedlines rather than by
/// chance. Two aerials like that can be subtracted to null a local noise
/// source, or added to ride out a fade: the same two jobs the LimeSDR's second
/// chain does, through the same [`DiversityMode`] and the same adaptive
/// filter.
///
/// They do not have to be combined, though. The tuners are separately
/// tunable, so the other one can instead be a **second radio** — HF in one tab
/// and VHF in another, off one board ([`SdrPlayDuoRole::SecondRadio`]).
///
/// The tuner this runs on is the one [`SdrPlayConfig::duo_tuner`] is not.
///
/// Dual-tuner operation is not free of constraints: the API fixes the ADC at
/// 6 MHz and hands back a 2 Msps stream from a low IF, so the widest span
/// available with both tuners running is 2 MHz (1.536 MHz of it inside the
/// analog filter) — and that span is the *session's*, so a second radio
/// adopts whatever the first one opened at. The driver clamps the configured
/// rate rather than refusing to open.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SdrPlayDuo {
    /// Run both tuners. Off leaves the RSPduo in single-tuner mode, which is
    /// the only setting that costs nothing.
    ///
    /// Changing this reopens the device: the mode is fixed at selection time,
    /// before the tuners are configured.
    pub enabled: bool,
    /// What the second tuner is for. Defaults to combining, which is what
    /// [`Self::enabled`] meant on its own before there was a choice — so a
    /// configuration written by an older build keeps doing what it did.
    pub role: SdrPlayDuoRole,
    /// Cancel or combine.
    pub mode: DiversityMode,
    /// LNA state for the second tuner, its own because the two aerials are
    /// not the same aerial. In [`DiversityMode::Combine`] the branch
    /// weighting assumes comparable noise floors; in [`DiversityMode::Cancel`]
    /// a second front end driven into overload hands the canceller a
    /// distorted copy of the interference, and a distorted copy cannot be
    /// subtracted from an undistorted one.
    pub lna_state: u8,
    /// IF gain *reduction* for the second tuner, in dB (20 = maximum gain).
    /// Only obeyed while the AGC is off.
    pub if_gr_db: i32,
    /// How many taps the adaptive filter has, 1 to [`DIVERSITY_MAX_TAPS`].
    pub taps: u8,
    /// How fast the filter adapts, 0 to 1.
    pub rate: f32,
    /// Hold the filter where it is.
    pub frozen: bool,
}

impl Default for SdrPlayDuo {
    fn default() -> Self {
        SdrPlayDuo {
            enabled: false,
            role: SdrPlayDuoRole::Diversity,
            mode: DiversityMode::Cancel,
            // The same mid-ladder default the main tuner has, and for the same
            // reason: state 0 on a real antenna overloads the ADC.
            lna_state: 4,
            if_gr_db: 40,
            taps: 8,
            rate: 0.7,
            frozen: false,
        }
    }
}

/// SDRplay RSP settings (`radio.json`). Receive only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SdrPlayConfig {
    /// Pin a particular receiver by its API serial; empty means "the first one
    /// found".
    pub serial: String,
    /// Effective complex sample rate in Hz. At and above 2 Msps this is the
    /// ADC rate; below, the ADC runs at 2 Msps and the service decimates.
    pub sample_rate_hz: f64,
    /// Analog IF bandwidth in kHz, or 0 for automatic — the widest filter that
    /// fits inside the sample rate.
    pub bw_khz: u32,
    /// IF gain *reduction* in dB, 20..=59 — the RSP's native unit, where 20 is
    /// maximum gain. Only obeyed while the AGC is off.
    pub if_gr_db: i32,
    /// LNA state, 0..=model max. 0 is maximum gain; each step switches more
    /// front-end attenuation in. The default is deliberately mid-table, not
    /// 0: state 0 on a real antenna drives the ADC straight into overload,
    /// and the IF AGC cannot rescue that — its whole 20..59 dB range sits
    /// *after* the front end. 4 is also what SoapySDRPlay3 defaults to.
    pub lna_state: u8,
    pub agc: SdrPlayAgc,
    /// AGC target level in dBFS.
    pub agc_setpoint_dbfs: i32,
    /// Reference trim, parts per million, applied by the device itself.
    pub ppm: f64,
    /// Bias tee: ~4.7 V DC on the antenna port for a remote LNA. Off by
    /// default — putting phantom power on someone's feedline uninvited is not
    /// a good default.
    pub bias_tee: bool,
    /// FM broadcast-band notch filter.
    pub rf_notch: bool,
    /// DAB-band notch filter.
    pub dab_notch: bool,
    /// Chosen antenna port, by the names [`SdrPlayModel::antennas`] publishes.
    /// Empty leaves the device's default.
    pub antenna: String,
    /// RSPduo only: which tuner to run — and, with
    /// [`SdrPlayDuo::enabled`], which of the two this radio listens on.
    pub duo_tuner: SdrPlayDuoTuner,
    /// RSPdx only: HDR mode below 2 MHz.
    ///
    /// **Does not work yet.** On the one RSPdx it has been tried against,
    /// switching the path on silences the receiver — at every centre, filter,
    /// antenna port, LNA state, level, offset, rate, IF and LO setting tried,
    /// enabled both before `Init` and at runtime. The struct layouts match
    /// `sdrplay_api_RspDx.h` and SoapySDRPlay3 drives it the same way, so
    /// whatever the mode needs is not on the documented API surface. What is
    /// known about it is in [`SdrPlayHdrBw`], and the panel says so where the
    /// operator can see it.
    pub hdr: bool,
    /// RSPdx only: the HDR path's analog filter. Only meaningful with
    /// [`Self::hdr`], and only at the centre frequencies that filter is
    /// implemented at — see [`SdrPlayHdrBw::works_at`]. Appended last, like
    /// every config field before it.
    #[serde(default)]
    pub hdr_bw: SdrPlayHdrBw,
    /// Let the IF gain reduction go below 20 dB, down to 0 — the last 20 dB of
    /// gain the hardware has.
    ///
    /// Off by default, which is the API's own default (`NORMAL_MIN_GR`) and
    /// the right one for ordinary listening: the bottom of the range is where
    /// an RSP is easiest to overload. Worth having on for weak-signal work
    /// with the LNA already at 0, where the AGC note calls IF gain free to
    /// use. Turning it on also lifts the AGC set-point ceiling to 0 dBFS.
    #[serde(default)]
    pub extended_if_gr: bool,
    /// RSPduo only: run the other tuner too — combined with this one, or as a
    /// radio of its own. Read from `radio.json` under either name: the block
    /// was called `diversity` when combining was all it could do.
    #[serde(alias = "diversity")]
    pub duo: SdrPlayDuo,
}

impl Default for SdrPlayConfig {
    fn default() -> Self {
        SdrPlayConfig {
            serial: String::new(),
            sample_rate_hz: 2_000_000.0,
            bw_khz: 0,
            if_gr_db: 40,
            lna_state: 4,
            agc: SdrPlayAgc::Hz50,
            agc_setpoint_dbfs: -60,
            ppm: 0.0,
            bias_tee: false,
            rf_notch: false,
            dab_notch: false,
            antenna: String::new(),
            duo_tuner: SdrPlayDuoTuner::Tuner1,
            hdr: false,
            hdr_bw: SdrPlayHdrBw::default(),
            extended_if_gr: false,
            duo: SdrPlayDuo::default(),
        }
    }
}

/// The RSPdx's HDR-path analog filter, and the only frequencies each of its
/// settings could work at.
///
/// HDR is not a mode that follows the dial. The path has a fixed analog filter
/// and the hardware only implements it centred on a short list of frequencies:
/// tuned anywhere else the switch is accepted and does nothing useful, which
/// reads as HDR being broken rather than as the dial being in the wrong place.
/// The lists are from the API specification's RSPdx section.
///
/// *Could*, because being on one of them is necessary and not sufficient — see
/// [`SdrPlayConfig::hdr`]. Nothing here has been seen to work on hardware.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SdrPlayHdrBw {
    Khz200,
    Khz500,
    Mhz1_2,
    /// The API's own default, and what an RSPdx runs with when nobody sets one.
    #[default]
    Mhz1_7,
}

impl SdrPlayHdrBw {
    pub const ALL: [SdrPlayHdrBw; 4] =
        [SdrPlayHdrBw::Khz200, SdrPlayHdrBw::Khz500, SdrPlayHdrBw::Mhz1_2, SdrPlayHdrBw::Mhz1_7];

    pub fn label(self) -> &'static str {
        match self {
            SdrPlayHdrBw::Khz200 => "200 kHz",
            SdrPlayHdrBw::Khz500 => "500 kHz",
            SdrPlayHdrBw::Mhz1_2 => "1.2 MHz",
            SdrPlayHdrBw::Mhz1_7 => "1.7 MHz",
        }
    }

    /// Back from a [`Self::code`] carried on the pseudo-element. Anything
    /// unrecognised lands on the API's own default rather than on a filter the
    /// operator did not pick.
    pub fn from_code(v: f64) -> SdrPlayHdrBw {
        match v.round() as i32 {
            0 => SdrPlayHdrBw::Khz200,
            1 => SdrPlayHdrBw::Khz500,
            2 => SdrPlayHdrBw::Mhz1_2,
            _ => SdrPlayHdrBw::Mhz1_7,
        }
    }

    /// The API's `sdrplay_api_RspDx_HdrModeBwT` code.
    pub fn code(self) -> i32 {
        match self {
            SdrPlayHdrBw::Khz200 => 0,
            SdrPlayHdrBw::Khz500 => 1,
            SdrPlayHdrBw::Mhz1_2 => 2,
            SdrPlayHdrBw::Mhz1_7 => 3,
        }
    }

    /// The centre frequencies, in Hz, this filter is actually implemented at.
    ///
    /// Two lists, not four: the narrow filters share the 500 kHz set and the
    /// wide ones share the 2 MHz set.
    pub fn centres_hz(self) -> &'static [f64] {
        match self {
            SdrPlayHdrBw::Khz200 | SdrPlayHdrBw::Khz500 => {
                &[135_000.0, 175_000.0, 220_000.0, 250_000.0, 340_000.0, 475_000.0]
            }
            SdrPlayHdrBw::Mhz1_2 | SdrPlayHdrBw::Mhz1_7 => {
                &[516_000.0, 875_000.0, 1_125_000.0, 1_900_000.0]
            }
        }
    }

    /// Whether HDR does anything at this centre. A hertz of slack either way,
    /// because a dial arrives here through a converter offset and a ppm trim
    /// and need not land on an exact integer.
    pub fn works_at(self, centre_hz: f64) -> bool {
        self.centres_hz().iter().any(|c| (c - centre_hz).abs() < 1.0)
    }

    /// The list an operator can be pointed at when their dial is not on one.
    pub fn centres_label(self) -> String {
        let parts: Vec<String> = self
            .centres_hz()
            .iter()
            .map(|hz| {
                if *hz < 1e6 {
                    format!("{:.0} kHz", hz / 1e3)
                } else {
                    format!("{:.3} MHz", hz / 1e6)
                }
            })
            .collect();
        parts.join(", ")
    }
}

impl SdrPlayConfig {
    /// Gain elements this backend exposes. They live here rather than in
    /// `sdroxide-sdrplay` so the (wasm-safe) settings UI can address them
    /// without depending on the native backend crate — same reason as
    /// [`HpsdrConfig::LNA_GAIN_ELEMENT`]. Both are carried as *negative*
    /// values (like the RX-888 attenuator) so more slider is more gain:
    /// `IF` is −(gain reduction dB), `LNA` is −(LNA state).
    pub const IF_GAIN_ELEMENT: &'static str = "IF";
    pub const LNA_ELEMENT: &'static str = "LNA";
    /// Pseudo-elements carrying settings that are not gains at all. They ride
    /// the existing `SetGain` command so this backend needs no new `Command`
    /// variant, and are deliberately absent from `DeviceCaps::gains` so
    /// nothing renders them as sliders — the SDRplay settings panel drives
    /// them directly. The AGC encoding lives beside the enum it carries
    /// ([`SdrPlayAgc::code`]) so the two ends cannot drift.
    pub const AGC_ELEMENT: &'static str = "AGC";
    pub const AGC_SETPOINT_ELEMENT: &'static str = "AGCSP";
    pub const PPM_ELEMENT: &'static str = "PPM";
    pub const BIAS_TEE_ELEMENT: &'static str = "BIASTEE";
    pub const RF_NOTCH_ELEMENT: &'static str = "RFNOTCH";
    pub const DAB_NOTCH_ELEMENT: &'static str = "DABNOTCH";
    pub const HDR_ELEMENT: &'static str = "HDR";
    /// The HDR path's analog filter, as a [`SdrPlayHdrBw::code`].
    pub const HDR_BW_ELEMENT: &'static str = "HDRBW";
    /// Whether the IF gain reduction may go below the API's 20 dB floor.
    pub const EXTENDED_IF_GR_ELEMENT: &'static str = "EXTGR";
    /// The RSPduo's second tuner and the diversity filter, through the same
    /// door — the filter's names being the shared ones ([`DIV_MODE_ELEMENT`]
    /// and friends). The two gains are carried negated, like the main
    /// tuner's.
    pub const AUX_LNA_ELEMENT: &'static str = "AUXLNA";
    pub const AUX_IF_GAIN_ELEMENT: &'static str = "AUXIF";
    pub const DIV_MODE_ELEMENT: &'static str = DIV_MODE_ELEMENT;
    pub const DIV_RATE_ELEMENT: &'static str = DIV_RATE_ELEMENT;
    pub const DIV_TAPS_ELEMENT: &'static str = DIV_TAPS_ELEMENT;
    pub const DIV_FREEZE_ELEMENT: &'static str = DIV_FREEZE_ELEMENT;
    pub const DIV_RESET_ELEMENT: &'static str = DIV_RESET_ELEMENT;

    /// IF gain reduction limits, in dB, from the API (`NORMAL_MIN_GR` and
    /// `MAX_BB_GR`).
    pub const IF_GR_MIN: i32 = 20;
    pub const IF_GR_MAX: i32 = 59;
    /// The floor with [`Self::extended_if_gr`] on (`EXTENDED_MIN_GR`).
    pub const IF_GR_MIN_EXTENDED: i32 = 0;

    /// The lowest IF gain reduction this configuration may ask for.
    pub fn if_gr_min(&self) -> i32 {
        if self.extended_if_gr { Self::IF_GR_MIN_EXTENDED } else { Self::IF_GR_MIN }
    }

    /// The AGC set-point range, in dBFS, that the API will accept at a given
    /// sample rate.
    ///
    /// Not one range: the converter's headroom shrinks as it trades resolution
    /// for speed, and the API refuses a set point below what the rate can
    /// reach. The thresholds are the specification's — under 8.064 MSPS the
    /// full −72 dB is available, to 9.216 it is −60, and above that −48. The
    /// ceiling is −20 unless the extended gain floor is in use, which lifts it
    /// to 0.
    ///
    /// This backend offers 10 MSPS, so the narrow end is reachable in normal
    /// use rather than being a theoretical edge.
    pub fn agc_setpoint_range(&self) -> std::ops::RangeInclusive<i32> {
        Self::agc_setpoint_range_at(self.sample_rate_hz, self.extended_if_gr)
    }

    /// [`Self::agc_setpoint_range`] over the two things it actually depends
    /// on, so the driver's live path can ask the same question.
    ///
    /// The rate is fixed for a session but the gain floor is a switch the
    /// operator throws mid-session, and the range the settings panel offers
    /// has to be the range the driver will accept — otherwise a set point the
    /// panel let the operator reach is one the driver quietly clamps away.
    pub fn agc_setpoint_range_at(
        sample_rate_hz: f64,
        extended_if_gr: bool,
    ) -> std::ops::RangeInclusive<i32> {
        let floor = if sample_rate_hz < 8_064_000.0 {
            -72
        } else if sample_rate_hz < 9_216_000.0 {
            -60
        } else {
            -48
        };
        let ceiling = if extended_if_gr { 0 } else { -20 };
        floor..=ceiling
    }

    /// Sample rates offered in the UI. Below 2 Msps the ADC still runs at
    /// 2 Msps and the API decimates; above 6.048 Msps the ADC trades
    /// resolution for speed (12 bits up to 6.048, 10 to 8.064, 8 beyond).
    pub const SAMPLE_RATES: [f64; 10] = [
        250_000.0,
        500_000.0,
        1_000_000.0,
        2_000_000.0,
        3_000_000.0,
        4_000_000.0,
        5_000_000.0,
        6_000_000.0,
        8_000_000.0,
        10_000_000.0,
    ];

    /// Analog IF bandwidths the tuner has, in kHz — the values of
    /// `sdrplay_api_Bw_MHzT`.
    pub const BANDWIDTHS_KHZ: [u32; 8] = [200, 300, 600, 1536, 5000, 6000, 7000, 8000];

    /// Sample rates an RSPduo offers with **both** tuners running.
    ///
    /// A much shorter list, and not a policy choice: in dual-tuner mode the
    /// API fixes the ADC at 6 MHz and both tuners hand it a 1.620 MHz low IF,
    /// from which its downconverter delivers 2 Msps. Everything narrower is
    /// that 2 Msps decimated by a power of two.
    pub const DUAL_SAMPLE_RATES: [f64; 6] =
        [62_500.0, 125_000.0, 250_000.0, 500_000.0, 1_000_000.0, 2_000_000.0];

    /// The widest analog filter the tuners have in dual-tuner mode: the low IF
    /// leaves no room for the wider ones.
    pub const DUAL_MAX_BW_KHZ: u32 = 1536;

    /// Whether this configuration asks for both of an RSPduo's tuners.
    ///
    /// Model-blind on purpose — the caller knows which RSP it has, and a
    /// setting left over from an RSPduo must not put an RSP1A into a mode it
    /// does not have.
    pub fn wants_dual_tuner(&self) -> bool {
        self.duo.enabled
    }

    /// The tuner the second aerial is on: the one [`Self::duo_tuner`] is not.
    pub fn aux_tuner(&self) -> SdrPlayDuoTuner {
        self.duo_tuner.other()
    }
}

/// An RSP the `sdrplay_api` service reports. Wasm-safe so it can cross the
/// `RadioController` trait to the settings UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdrPlayDevice {
    /// The API's serial string — what [`SdrPlayConfig::serial`] pins.
    pub serial: String,
    /// The raw `hwVer` byte; [`Self::model`] decodes it.
    pub hw_ver: u8,
}

impl SdrPlayDevice {
    pub fn model(&self) -> SdrPlayModel {
        SdrPlayModel::from_hw_ver(self.hw_ver)
    }

    /// One-line label for the selection UI.
    pub fn label(&self) -> String {
        format!("{}  (serial {})", self.model().label(), self.serial)
    }

    /// Whether the service reported this receiver without a usable identity:
    /// no serial number, or a hardware version these bindings do not know.
    ///
    /// A missing serial is SDRplay's own documented signature of a USB
    /// communication problem — a brownout, a bad cable, or an API service
    /// holding a stale session after the device re-enumerated under it. Such
    /// a receiver still lists, selects and streams, but often deaf, and
    /// nothing else about the session looks wrong; hence a dedicated check
    /// every surface can warn from.
    pub fn identity_missing(&self) -> bool {
        Self::degraded_identity(&self.serial, self.model())
    }

    /// The predicate behind [`Self::identity_missing`], for callers that hold
    /// the serial and model without a device row (the running source).
    pub fn degraded_identity(serial: &str, model: SdrPlayModel) -> bool {
        serial.trim().is_empty() || model == SdrPlayModel::Unknown
    }

    /// Operator-facing warning for a degraded enumeration, `None` when
    /// healthy. One composer so the settings picker, the standing notice and
    /// the log all say the same thing.
    pub fn degraded_warning(serial: &str, model: SdrPlayModel) -> Option<String> {
        if !Self::degraded_identity(serial, model) {
            return None;
        }
        let what = if serial.trim().is_empty() {
            "no serial number"
        } else {
            "an unrecognised hardware version"
        };
        Some(format!(
            "The SDRplay service reports this RSP with {what} — usually a USB \
             communication problem, and such a receiver often runs deaf. Restart the \
             SDRplay API service, then unplug and replug the receiver."
        ))
    }

    /// [`Self::degraded_warning`] for a listed device.
    pub fn identity_warning(&self) -> Option<String> {
        Self::degraded_warning(&self.serial, self.model())
    }
}

/// Named converters for [`RadioConfig::converter_offset_hz`], with the offset
/// each one puts on the dial in Hz.
///
/// Signs follow the one rule the whole feature is built on: the hardware is
/// tuned to `dial + offset`. An *up*-converter therefore has a positive offset
/// — a Ham It Up presents 10.1 MHz to the receiver as 135.1 MHz — and a
/// *down*-converter a negative one: a universal Ku-band LNB with a 9750 MHz
/// local oscillator hands a 10.489 GHz downlink to the receiver at 739 MHz.
///
/// Anything else is typed in directly; the settings dialog calls that Manual,
/// and shows it whenever the offset matches nothing here.
pub const CONVERTER_PRESETS: [(&str, f64); 5] = [
    ("None", 0.0),
    ("Ham It Up (+125 MHz)", 125_000_000.0),
    ("SpyVerter (+120 MHz)", 120_000_000.0),
    ("LNB, Ku low (−9750 MHz)", -9_750_000_000.0),
    ("LNB, Ku high (−10600 MHz)", -10_600_000_000.0),
];

/// How far a converter offset may be set either way, in Hz.
///
/// Wide enough for a Ku-band LNB, which is the largest offset anyone puts in
/// front of a receiver; an HF upconverter is two orders of magnitude inside it.
pub const CONVERTER_OFFSET_MAX_HZ: f64 = 12_000_000_000.0;

/// What the transmit path does while a receive converter is set — see
/// [`RadioConfig::converter_tx`].
///
/// A converter is a receive accessory: an LNB, a Ham It Up, anything in the
/// antenna line ahead of the receiver's input. What sits in the *transmit* line
/// is a separate question with three real answers, and the operator is the only
/// one who can give it, so it is asked rather than assumed.
/// Externally tagged (serde's default shape) rather than something prettier in
/// `radio.json`: this config crosses the remote link as postcard, which is not
/// self-describing and refuses an internally or adjacently tagged enum outright
/// — see `roundtrip_radio_config` in `sdroxide-proto`.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConverterTx {
    /// Nothing is transmitted while the converter is set. The safe default, and
    /// right for the receive-only accessory the offset usually describes: a
    /// dongle behind a Ham It Up has no transmitter, and a transceiver behind
    /// one would key up 125 MHz away from where the dial says.
    #[default]
    Off,
    /// The same box converts both ways — a transverter. The transmit offset is
    /// the receive one, and follows it when it is trimmed.
    Transverter,
    /// The transmit path has an offset of its own, on the same sign rule as
    /// [`RadioConfig::converter_offset_hz`]: the hardware transmits at
    /// `dial + offset`.
    ///
    /// `0.0` — the common case — is a receive-only converter with the
    /// transmitter wired straight to its own antenna. That is the QO-100
    /// station: the downlink arrives through a 10 GHz LNB and the 2.4 GHz
    /// uplink leaves the radio directly, so receive is offset and transmit is
    /// not.
    Own(f64),
}

impl ConverterTx {
    /// The offset the transmit path takes given the receive converter's, or
    /// `None` when transmit is withdrawn.
    pub fn offset_hz(self, rx_offset_hz: f64) -> Option<f64> {
        match self {
            ConverterTx::Off => None,
            ConverterTx::Transverter => Some(rx_offset_hz),
            ConverterTx::Own(hz) => Some(hz),
        }
    }

    /// What the settings dialog calls this.
    pub fn label(self) -> &'static str {
        match self {
            ConverterTx::Off => "Off while converting",
            ConverterTx::Transverter => "Through the same converter",
            ConverterTx::Own(_) => "Its own offset",
        }
    }
}

/// One transverter: a band it works, the offset it works it at, and what it can
/// take on transmit.
///
/// A station with more than one of these has more than one answer, and which
/// one applies is decided by where the dial is — that is the whole difference
/// between this and [`RadioConfig::converter_offset_hz`], which is one answer
/// for every frequency (issue #278). A 2 m transverter on a 28 MHz I.F. and a
/// 6 m one on the same I.F. sit in the same table, each with its own offset,
/// and the radio follows the dial from one to the other.
///
/// The offset carries the sign rule the rest of the feature uses: **the
/// hardware is tuned to `dial + offset_hz`**. A transverter that brings a band
/// *down* to an I.F. therefore has a negative offset — 2 m into 28 MHz is
/// −116 MHz, because 144 − 116 = 28.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transverter {
    /// What the operator calls it. Shown in the dialog and in the log line that
    /// says which one the dial has selected; not otherwise interpreted.
    #[serde(default)]
    pub name: String,
    /// Unticked leaves the row in the table and takes the transverter out of
    /// the line — the box is switched off or unplugged, and the band goes back
    /// to whatever the radio reaches on its own.
    #[serde(default = "default_xvtr_enabled")]
    pub enabled: bool,
    /// The band this transverter works, on the *dial*, in Hz.
    #[serde(default)]
    pub rf_lo_hz: f64,
    #[serde(default)]
    pub rf_hi_hz: f64,
    /// `hardware = dial + offset_hz`.
    #[serde(default)]
    pub offset_hz: f64,
    /// What the transmit line does while this transverter is the selected one.
    #[serde(default)]
    pub tx: ConverterTx,
    /// A ceiling on transmit drive while it is selected, 0.0–1.0 of full.
    ///
    /// The reason this is per transverter rather than per station: a
    /// transverter's I.F. input takes milliwatts, and the drive that is right
    /// for the radio's own bands will destroy it. 0.05 is 5 % of full drive,
    /// which is the order most transverters want from a 100 W radio through
    /// their built-in attenuator.
    #[serde(default = "default_xvtr_drive")]
    pub tx_drive: f32,
}

fn default_xvtr_enabled() -> bool {
    true
}

fn default_xvtr_drive() -> f32 {
    0.05
}

impl Default for Transverter {
    fn default() -> Self {
        Transverter {
            name: String::new(),
            enabled: true,
            rf_lo_hz: 0.0,
            rf_hi_hz: 0.0,
            offset_hz: 0.0,
            tx: ConverterTx::Off,
            tx_drive: default_xvtr_drive(),
        }
    }
}

impl Transverter {
    /// Whether this row describes a band at all. A pair that is not a range —
    /// unset, inverted, not finite — selects nothing rather than everything:
    /// a half-typed row must not take the whole dial with it.
    pub fn is_band(&self) -> bool {
        self.rf_lo_hz.is_finite()
            && self.rf_hi_hz.is_finite()
            && self.rf_lo_hz >= 0.0
            && self.rf_hi_hz > self.rf_lo_hz
    }

    /// Whether the dial is inside this transverter's band.
    pub fn covers(&self, dial_hz: f64) -> bool {
        self.enabled && self.is_band() && (self.rf_lo_hz..=self.rf_hi_hz).contains(&dial_hz)
    }

    /// What to call it in a log line or a dialog row.
    pub fn describe(&self) -> String {
        if !self.name.trim().is_empty() {
            return self.name.trim().to_string();
        }
        format!("{:.3}–{:.3} MHz", self.rf_lo_hz / 1e6, self.rf_hi_hz / 1e6)
    }
}

/// How many transverters a station may state. Ten is more bands than any
/// amateur station has boxes for, and the table is drawn in full.
pub const MAX_TRANSVERTERS: usize = 10;

/// One band's transmit drive calibration: how far the drive that reaches the
/// air has to be moved on that band for the operator's Drive setting to mean
/// the same output power everywhere (issue #295).
///
/// Every amplifier has a different gain on every band — a 10 m stage typically
/// wants several decibels more drive than the same radio's 40 m one — so a
/// single Drive number produces a different power on each. This table takes
/// that out: the operator sets Drive for the band that needs the most, then
/// trims every other band down by what they measure.
///
/// `db` is decibels of **RF output power**, negative to take power off, which
/// is the direction a calibration runs in: the slider is already the maximum,
/// so the bands the amplifier is happier on come *down* to meet the one it is
/// not. A little the other way is allowed for a station whose worst band is
/// calibrated below full — see [`BandDriveTrim::DB_RANGE`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BandDriveTrim {
    /// The band this row calibrates, matched against the *transmit* frequency
    /// on the **dial** — where the station radiates, which is the band whose
    /// amplifier is being calibrated. Behind a transverter that is the
    /// converted band, not the I.F. the radio is really on; what the
    /// transverter's own I.F. input can take is its row's drive ceiling
    /// ([`Transverter::tx_drive`]), which still wins over anything here.
    pub band: Band,
    /// Decibels of output power to add on this band. `0.0` — the default, and
    /// what an absent row means — leaves the band exactly as it was.
    pub db: f32,
}

impl BandDriveTrim {
    /// How far a row may trim, in dB of output power.
    ///
    /// Twenty decibels down is a hundredth of the power, which is more than the
    /// spread between any two bands of one amplifier; the six the other way are
    /// for a station that calibrated its worst band below full drive and has
    /// somewhere to go. Drive is still a fraction of full scale afterwards, so
    /// a positive trim runs out at the top rather than overdriving anything.
    pub const DB_RANGE: std::ops::RangeInclusive<f32> = -20.0..=6.0;

    /// The trim, held inside [`Self::DB_RANGE`] and never NaN — what a
    /// hand-edited `radio.json` gets held to before it reaches the modulator.
    pub fn db(&self) -> f32 {
        if self.db.is_finite() {
            self.db.clamp(*Self::DB_RANGE.start(), *Self::DB_RANGE.end())
        } else {
            0.0
        }
    }

    /// The multiplier `db` decibels of **output power** puts on a drive
    /// control.
    ///
    /// Which conversion applies depends on what drive *is* on the radio
    /// underneath, and the two differ by a factor of two in the exponent:
    ///
    /// - `drive_is_power`: the control is a fraction of the radio's rated
    ///   power — 0.5 is half its watts — the way a CAT, LAN or TCI transceiver
    ///   takes it. A decibel of output is then a decibel of drive:
    ///   `10^(dB/10)`.
    /// - Otherwise drive scales the modulated I/Q, which is an *amplitude*,
    ///   and power goes as its square: `10^(dB/20)`.
    ///
    /// So a row means the same change at the antenna whichever kind of radio
    /// the operator points this configuration at, which is the whole point of
    /// stating the table in decibels of output rather than in "drive units".
    pub fn factor_for(db: f32, drive_is_power: bool) -> f32 {
        if db == 0.0 || !db.is_finite() {
            return 1.0;
        }
        10f32.powf(db / if drive_is_power { 10.0 } else { 20.0 })
    }
}

#[cfg(test)]
mod oc_table_tests {
    use super::*;

    /// The operator's own words reach the wire, and receive and transmit are
    /// genuinely separate — which is what an amplifier key line or a receive
    /// preamplifier bypass on one of those pins needs (issue #296).
    #[test]
    fn a_custom_table_answers_per_band_and_per_direction() {
        let cfg = HpsdrConfig {
            filter_board: HpsdrFilterBoard::Custom,
            oc_table: vec![
                HpsdrOcRow { band: Band::M40, rx: 0x01, tx: 0x41 },
                HpsdrOcRow { band: Band::M20, rx: 0x02, tx: 0x02 },
            ],
            ..HpsdrConfig::default()
        };
        let plan = cfg.oc_plan();
        assert_eq!(plan.word(7_074_000.0, false), 0x01);
        assert_eq!(plan.word(7_074_000.0, true), 0x41, "the amplifier line is transmit-only");
        assert_eq!(plan.word(14_074_000.0, false), 0x02);
        assert_eq!(plan.word(14_074_000.0, true), 0x02, "a filter is the same either way");
        // A band with no row asserts nothing, and so does a dial outside every
        // band: an operator who has said nothing about 10 m has said nothing.
        assert_eq!(plan.word(28_074_000.0, false), 0);
        assert_eq!(plan.word(11_000_000.0, false), 0);
        assert!(plan.drives_anything());
    }

    /// Selecting a preset ignores the table, and selecting nothing ignores
    /// both — the table stays on disk either way, so switching back and forth
    /// to compare does not throw it away.
    #[test]
    fn a_preset_outranks_the_table_without_destroying_it() {
        let mut cfg = HpsdrConfig {
            filter_board: HpsdrFilterBoard::Alex,
            oc_table: vec![HpsdrOcRow { band: Band::M40, rx: 0x7F, tx: 0x7F }],
            ..HpsdrConfig::default()
        };
        assert_eq!(cfg.oc_plan().word(7_074_000.0, false), hpsdr_alex_oc(7_074_000.0));
        cfg.filter_board = HpsdrFilterBoard::None;
        assert_eq!(cfg.oc_plan().word(7_074_000.0, false), 0);
        assert!(!cfg.oc_plan().drives_anything());
        cfg.filter_board = HpsdrFilterBoard::Custom;
        assert_eq!(cfg.oc_plan().word(7_074_000.0, false), 0x7F, "the table was still there");
    }

    /// Pouring a preset into the table reproduces it band for band, which is
    /// what makes "start from the N2ADR mapping and change one pin" possible.
    #[test]
    fn a_poured_preset_reproduces_itself() {
        for board in [HpsdrFilterBoard::N2adr, HpsdrFilterBoard::Alex] {
            let cfg = HpsdrConfig {
                filter_board: HpsdrFilterBoard::Custom,
                oc_table: HpsdrOcRow::from_preset(board),
                ..HpsdrConfig::default()
            };
            let poured = cfg.oc_plan();
            let preset = HpsdrOcPlan::preset(board);
            for band in Band::ALL {
                if band == Band::Gen || band.edges().is_none() {
                    continue;
                }
                let hz = band.default_entry().0;
                assert_eq!(
                    poured.word(hz, false),
                    preset.word(hz, false) & 0x7F,
                    "{board:?} on {} ({hz} Hz)",
                    band.label()
                );
            }
        }
        // "None" and "Custom" have nothing to pour.
        assert!(HpsdrOcRow::from_preset(HpsdrFilterBoard::None).is_empty());
        assert!(HpsdrOcRow::from_preset(HpsdrFilterBoard::Custom).is_empty());
    }

    /// A table of zeros is not an accessory board, and must not be announced as
    /// one — the log line at open is the operator's only warning that seven
    /// general-purpose outputs are about to start moving.
    #[test]
    fn an_empty_table_drives_nothing() {
        let cfg = HpsdrConfig { filter_board: HpsdrFilterBoard::Custom, ..HpsdrConfig::default() };
        assert!(!cfg.oc_plan().drives_anything());
        assert_eq!(cfg.oc_plan().word(7_074_000.0, true), 0);
    }
}

#[cfg(test)]
mod drive_trim_tests {
    use super::*;

    /// A row applies to its own band and to nothing else, and a dial outside
    /// every band falls on the `Gen` row if there is one.
    #[test]
    fn the_trim_follows_the_transmit_band() {
        let cfg = RadioConfig {
            tx_drive_trim: vec![
                BandDriveTrim { band: Band::M10, db: 0.0 },
                BandDriveTrim { band: Band::M40, db: -6.0 },
                BandDriveTrim { band: Band::Gen, db: -12.0 },
            ],
            ..RadioConfig::default()
        };
        assert_eq!(cfg.drive_trim_db(7_074_000.0), -6.0);
        assert_eq!(cfg.drive_trim_db(28_074_000.0), 0.0, "a row set to zero is no trim at all");
        assert_eq!(cfg.drive_trim_db(14_074_000.0), 0.0, "a band with no row is untouched");
        assert_eq!(cfg.drive_trim_db(11_000_000.0), -12.0, "outside every ham band");
        // The station that has never opened the table, which is nearly all of
        // them: nothing is looked up and nothing is changed.
        assert_eq!(RadioConfig::default().drive_trim_db(7_074_000.0), 0.0);
    }

    /// A hand-edited `radio.json` cannot drive the transmitter through the
    /// roof — or produce a NaN that would silence it.
    #[test]
    fn an_impossible_row_is_held_to_the_range() {
        assert_eq!(BandDriveTrim { band: Band::M20, db: 40.0 }.db(), 6.0);
        assert_eq!(BandDriveTrim { band: Band::M20, db: -100.0 }.db(), -20.0);
        assert_eq!(BandDriveTrim { band: Band::M20, db: f32::NAN }.db(), 0.0);
    }

    /// The same decibels of output, on the two kinds of drive control.
    #[test]
    fn a_decibel_means_the_same_at_the_antenna_either_way() {
        // Six decibels down is a quarter of the power, which is half the
        // amplitude — and a quarter of a rig's power setting.
        let amplitude = BandDriveTrim::factor_for(-6.0, false);
        let power = BandDriveTrim::factor_for(-6.0, true);
        assert!((amplitude - 0.5011872).abs() < 1e-4, "{amplitude}");
        assert!((power - 0.2511886).abs() < 1e-4, "{power}");
        // …and squaring the amplitude gets back to the power fraction, which is
        // the identity the pair exists to keep.
        assert!((amplitude * amplitude - power).abs() < 1e-4);
        // No trim is exactly no change, not a rounding of one.
        assert_eq!(BandDriveTrim::factor_for(0.0, false), 1.0);
        assert_eq!(BandDriveTrim::factor_for(0.0, true), 1.0);
    }
}

/// The preset name for an offset, or `"Manual"` when it is not one of them.
pub fn converter_preset_name(offset_hz: f64) -> &'static str {
    CONVERTER_PRESETS
        .iter()
        .find(|(_, hz)| (hz - offset_hz).abs() < 0.5)
        .map(|(name, _)| *name)
        .unwrap_or("Manual")
}

/// The highest frequency an operator-supplied tuning range may name, in Hz.
///
/// 300 GHz is the top of the highest amateur allocation, and well past any
/// front end this program will meet — a number above it is a typo (a range
/// entered in Hz where megahertz was asked for, most likely) rather than a
/// microwave station.
pub const FREQ_RANGE_MAX_HZ: f64 = 300_000_000_000.0;

/// Parse an operator-typed list of tuning ranges — `"144-146, 430-440"` — into
/// (low, high) pairs in Hz.
///
/// The numbers are megahertz, because that is how an operator says which bands
/// a radio covers and how every band plan is written; the ranges themselves are
/// kept in Hz to match [`crate::DeviceCaps`]. Ranges are separated by commas,
/// semicolons or newlines and their edges by `-`, `–` or `..`, so a list
/// pasted back out of [`format_freq_ranges`] — or copied from a band plan —
/// reads straight in.
///
/// Empty input is not an error: it parses to no ranges, which is how an
/// operator says "use whatever the device publishes".
pub fn parse_freq_ranges(text: &str) -> Result<Vec<(f64, f64)>, String> {
    let mut out = Vec::new();
    for item in text.split([',', ';', '\n']) {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let normalised = item.replace(['\u{2013}', '\u{2014}'], "-").replace("..", "-");
        let Some((lo, hi)) = normalised.split_once('-') else {
            return Err(format!("\"{item}\" is not a range — write it as low-high, e.g. 430-440"));
        };
        let lo = parse_range_edge(lo, item)?;
        let hi = parse_range_edge(hi, item)?;
        if hi <= lo {
            return Err(format!("\"{item}\" has its top at or below its bottom"));
        }
        out.push((lo, hi));
    }
    Ok(out)
}

/// One edge of a range, in MHz, to Hz. `whole` names the range it came from so
/// the message points at what was typed rather than at a bare number.
fn parse_range_edge(edge: &str, whole: &str) -> Result<f64, String> {
    let text = edge.trim();
    // A unit is optional and only ever the one the field is in; accepting it
    // costs nothing and refusing it would look like the number was wrong.
    let text = text.strip_suffix("MHz").or_else(|| text.strip_suffix("mhz")).unwrap_or(text).trim();
    let mhz: f64 = text
        .parse()
        .map_err(|_| format!("\"{text}\" in \"{whole}\" is not a number of megahertz"))?;
    if !mhz.is_finite() || mhz < 0.0 {
        return Err(format!("\"{text}\" in \"{whole}\" is not a frequency"));
    }
    let hz = mhz * 1e6;
    if hz > FREQ_RANGE_MAX_HZ {
        return Err(format!(
            "\"{text}\" in \"{whole}\" is above {} GHz — these are megahertz",
            FREQ_RANGE_MAX_HZ / 1e9
        ));
    }
    Ok(hz)
}

/// Ranges in Hz back to the megahertz list an operator typed, ready to be
/// parsed again by [`parse_freq_ranges`].
pub fn format_freq_ranges(ranges: &[(f64, f64)]) -> String {
    fn mhz(hz: f64) -> String {
        // Six decimals is one hertz, and trailing zeros are trimmed so a band
        // edge reads as "430" rather than "430.000000".
        let mut s = format!("{:.6}", hz / 1e6);
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
        s
    }
    ranges.iter().map(|&(lo, hi)| format!("{}-{}", mhz(lo), mhz(hi))).collect::<Vec<_>>().join(", ")
}

/// Where the attached receiver is connected.
///
/// Not cosmetic: it decides whether an offset is expected at all, and whether
/// the per-mode table below has anything to say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PanadapterTap {
    /// The same antenna, through a splitter or the transceiver's RX-out loop.
    /// The receiver tunes to the dial and the offset is normally zero.
    #[default]
    Antenna,
    /// The transceiver's I.F. output. The receiver stays parked on the rig's
    /// intermediate frequency, so what looks like a dial move is really the
    /// rig's own first oscillator moving underneath a receiver that never
    /// retunes — and the offset is the I.F., which on many rigs depends on the
    /// mode.
    IfOutput,
}

impl PanadapterTap {
    pub const ALL: [PanadapterTap; 2] = [PanadapterTap::Antenna, PanadapterTap::IfOutput];
    pub fn label(self) -> &'static str {
        match self {
            PanadapterTap::Antenna => "Antenna (shared with the radio)",
            PanadapterTap::IfOutput => "The radio's I.F. output",
        }
    }
}

/// Which of the two radios you listen to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PanadapterAudio {
    /// The attached receiver: sdroxide demodulates its I/Q, so its filter, AGC,
    /// noise reduction and notch are the ones in the panel.
    #[default]
    Attached,
    /// The transceiver's own demodulated audio, over its sound card. The rig's
    /// receiver does the work and the attached one supplies only the picture.
    Transceiver,
}

impl PanadapterAudio {
    pub const ALL: [PanadapterAudio; 2] = [PanadapterAudio::Attached, PanadapterAudio::Transceiver];
    pub fn label(self) -> &'static str {
        match self {
            PanadapterAudio::Attached => "The attached receiver",
            PanadapterAudio::Transceiver => "The transceiver",
        }
    }
}

/// The classes a rig's I.F. offset can differ between.
///
/// Deliberately *not* [`crate::Mode`]: there are 29 of those and a rig has at
/// most a handful of carrier positions. And deliberately not the engine's
/// `rig_mode_class` either, which folds the data modes onto the sideband they
/// ride on — the whole point of a DATA entry here is that a rig commonly puts
/// its data mode at a different carrier offset from plain SSB.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IfModeClass {
    Lsb,
    Usb,
    Cw,
    Am,
    Fm,
    Data,
}

impl IfModeClass {
    pub const ALL: [IfModeClass; 6] = [
        IfModeClass::Lsb,
        IfModeClass::Usb,
        IfModeClass::Cw,
        IfModeClass::Am,
        IfModeClass::Fm,
        IfModeClass::Data,
    ];
    pub fn label(self) -> &'static str {
        match self {
            IfModeClass::Lsb => "LSB",
            IfModeClass::Usb => "USB",
            IfModeClass::Cw => "CW",
            IfModeClass::Am => "AM",
            IfModeClass::Fm => "FM",
            IfModeClass::Data => "DATA",
        }
    }
}

/// How far a panadapter offset may be set either way, in Hz.
///
/// Sized for an I.F. rather than for a sound card: 70.455 MHz is an ordinary
/// first I.F., and a receiver watching one is doing exactly what this feature
/// is for. Well inside [`CONVERTER_OFFSET_MAX_HZ`], which has a satellite LNB
/// to reach.
pub const PANADAPTER_OFFSET_MAX_HZ: f64 = 1_000_000_000.0;

/// Using another radio in the roster as this one's receiver: the panadapter,
/// the waterfall and everything sdroxide reads off them come from that radio's
/// front end, while the dial, the mode, the filter and the transmitter stay
/// with this one.
///
/// The pairing is recorded on the radio that *uses* the receiver, not on the
/// receiver, because that is the radio the operator is looking at and the one
/// whose engine opens both devices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PanadapterConfig {
    /// Roster id of the radio whose receiver supplies the spectrum. `None` —
    /// the default — is an ordinary radio with nothing attached.
    pub source_radio: Option<u32>,
    pub tap: PanadapterTap,
    /// How far above this radio's dial the attached receiver sits, in Hz.
    /// Zero for a receiver on the same antenna; the intermediate frequency for
    /// an I.F. tap.
    ///
    /// The sign rule is the converter's: the receiver is tuned to
    /// `dial + offset`. Nothing here is ever sent to the transceiver.
    pub offset_hz: f64,
    /// Overrides of [`Self::offset_hz`] for a rig whose I.F. moves with the
    /// mode. A class with no entry uses the plain offset.
    ///
    /// A list rather than a map so the file stays diffable and the wire format
    /// stays positional.
    pub mode_offsets: Vec<(IfModeClass, f64)>,
    /// The tap is spectrally inverted — a high-side-injection I.F., where the
    /// band arrives mirrored about the receiver's centre.
    pub invert: bool,
    /// Follow the transceiver's dial. Off parks the receiver where it is, which
    /// is what a fixed watch on one segment wants.
    pub track: bool,
    pub audio: PanadapterAudio,
    /// Silence receive audio while the transceiver is transmitting. On by
    /// default: with the receiver on the same antenna (or on the rig's I.F.)
    /// what it hears during an over is your own transmitter.
    pub mute_on_tx: bool,
    /// Stop the panadapter and waterfall while the transceiver is
    /// transmitting. On by default, for the same reason and because a
    /// transmitter painted across the whole span erases the band behind it.
    pub blank_on_tx: bool,
}

impl Default for PanadapterConfig {
    fn default() -> Self {
        PanadapterConfig {
            source_radio: None,
            tap: PanadapterTap::default(),
            offset_hz: 0.0,
            mode_offsets: Vec::new(),
            invert: false,
            track: true,
            audio: PanadapterAudio::default(),
            mute_on_tx: true,
            blank_on_tx: true,
        }
    }
}

impl PanadapterConfig {
    /// Whether a receiver is attached at all.
    pub fn is_attached(&self) -> bool {
        self.source_radio.is_some()
    }

    /// The offset to use in `mode`: the class override where there is one,
    /// otherwise the plain offset.
    pub fn offset_for(&self, mode: crate::Mode) -> f64 {
        let class = mode.if_class();
        self.mode_offsets
            .iter()
            .find(|(c, _)| *c == class)
            .map(|(_, hz)| *hz)
            .unwrap_or(self.offset_hz)
    }

    /// Record (or clear) a class override. Clearing removes the entry rather
    /// than storing the plain offset, so a later change to
    /// [`Self::offset_hz`] still reaches every class that never had one.
    pub fn set_mode_offset(&mut self, class: IfModeClass, hz: Option<f64>) {
        self.mode_offsets.retain(|(c, _)| *c != class);
        if let Some(hz) = hz {
            self.mode_offsets.push((class, hz));
        }
    }

    /// The class override, if this class has one.
    pub fn mode_offset(&self, class: IfModeClass) -> Option<f64> {
        self.mode_offsets.iter().find(|(c, _)| *c == class).map(|(_, hz)| *hz)
    }
}

/// What the operator has chosen for a SoapySDR device.
///
/// Every field's default means **"leave it alone"** — zero for the two rates,
/// empty for the settings — so a configuration written before this existed
/// opens its radio exactly as it always did. That matters more here than in a
/// native backend's block: this one covers every driver SoapySDR has, most of
/// which nobody here has ever run, and a default that asserted something would
/// be asserting it blind on all of them.
///
/// The settings are a key/value list rather than named fields on purpose. They
/// are whatever the driver said it had ([`DeviceSetting`]), so naming them here
/// would mean naming every driver's — which is exactly the per-driver knowledge
/// that reaching a radio through SoapySDR is supposed to avoid. Keys the device
/// no longer reports are kept rather than pruned: the operator may be moving one
/// configuration between two machines, and a setting silently dropped because
/// the module was missing that day is worse than one that goes unused.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SoapyConfig {
    /// Complex sample rate in Hz, or `0.0` to take the app-wide rate as before.
    /// Snapped to what the device says it accepts, so a value that no longer
    /// fits a swapped-in radio degrades to the nearest one rather than failing.
    pub sample_rate_hz: f64,
    /// Receive baseband filter in Hz, or `0.0` to leave the driver's own choice
    /// alone — which is usually derived from the sample rate and usually right.
    pub bandwidth_hz: f64,
    /// Driver settings, as `(key, value)`. See the note above on why these are
    /// not named fields.
    pub settings: Vec<(String, String)>,
}

impl SoapyConfig {
    /// The value the operator has chosen for `key`, if any.
    pub fn setting(&self, key: &str) -> Option<&str> {
        self.settings.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    /// Record a choice, replacing any earlier one for the same key.
    pub fn set_setting(&mut self, key: &str, value: &str) {
        match self.settings.iter_mut().find(|(k, _)| k == key) {
            Some((_, v)) => *v = value.to_string(),
            None => self.settings.push((key.to_string(), value.to_string())),
        }
    }
}

/// One Fobos SDR from `fobos_rx_list_devices` — free to ask for (no device is
/// opened), unlike most of the vendor-library-backed enumerations here.
/// Always has a serial: `libfobos`'s own enumeration is serial-based, unlike
/// a bare USB descriptor that may have none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FobosDevice {
    pub serial: String,
}

impl FobosDevice {
    pub fn label(&self) -> String {
        format!("Fobos SDR  (serial {})", self.serial)
    }
}

/// Which of the Fobos SDR's inputs to stream. `Rf` goes through the tuner
/// (mixer, LNA, VGA); `Hf1`/`Hf2` bypass it for direct sampling of one real
/// ADC channel apiece, turned into complex baseband entirely in software
/// (`sdroxide_fobos::stream`'s own `WbDdc` path) — the front end LNA/VGA gain
/// switches is powered down while either runs, so those two controls have no
/// effect here. `HfDual` runs both real channels at once, through their own
/// `WbDdc`s, combined by [`FobosConfig`]'s own diversity filter — the two
/// channels come from one ADC and one clock, so unlike an RSPduo's two
/// independent tuners the relative phase between them is reproducible across
/// a restart (measured directly on the radio, not assumed: under a degree
/// of inter-channel phase drift across repeated stop/start cycles).
/// Changing the port reopens the device: `Rf` and the HF ports use
/// different hardware paths
/// (`fobos_rx_set_direct_sampling`) and the HF ports build their
/// downconverter(s) at open time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FobosPort {
    #[default]
    Rf,
    Hf1,
    Hf2,
    HfDual,
}

impl FobosPort {
    pub fn name(self) -> &'static str {
        match self {
            FobosPort::Rf => "RF",
            FobosPort::Hf1 => "HF1",
            FobosPort::Hf2 => "HF2",
            FobosPort::HfDual => "HF1 + HF2 (diversity)",
        }
    }

    pub const ALL: [FobosPort; 4] =
        [FobosPort::Rf, FobosPort::Hf1, FobosPort::Hf2, FobosPort::HfDual];
}

/// RigExpert Fobos SDR (USB) backend configuration. Receive only, through
/// `libfobos` — LGPL-2.1 and open source (github.com/rigexpert/libfobos),
/// found with dlopen at runtime the way the SDRplay and LimeSDR backends find
/// their own vendor libraries, for the same build-portability reason: this
/// crate ships in every build variant without every contributor needing
/// `libfobos` installed to compile sdroxide.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FobosConfig {
    /// Pin a receiver by its serial; empty means "the first one found".
    pub serial: String,
    pub port: FobosPort,
    /// The ADC's own I/Q rate on [`FobosPort::Rf`]; the *target* complex
    /// output rate of the software downconverter on the HF ports — the
    /// achieved rate can differ from either (both a real hardware quirk
    /// below ~8 Msps on `Rf`, and the HF path's own power-of-two bin
    /// quantization), reported back rather than assumed.
    ///
    /// On the HF ports this also sets a floor on which centre frequencies
    /// are reachable at all: the downconverter selects a band directly out
    /// of a real FFT's non-negative bins, so the band can never straddle DC,
    /// and the lowest centre it can select is half this rate. A wider
    /// output rate here means a wider panadapter view but a higher floor —
    /// see `FobosConfig::default()`'s own comment for why the default is as
    /// narrow as it is.
    pub sample_rate_hz: f64,
    /// 0..3. [`FobosPort::Rf`] only — see [`FobosPort`]'s own doc comment.
    pub lna_gain: u8,
    /// 0..31. Same caveat as `lna_gain`.
    pub vga_gain: u8,
    pub clk_external: bool,

    /// Diversity filter settings — meaningful only on [`FobosPort::HfDual`],
    /// which is what actually turns the filter on. Unlike an RSPduo or a
    /// LimeSDR's second chain there is no separate `enabled`/`role` field
    /// here: running both real channels *is* asking for diversity, so the
    /// port selection already says so.
    ///
    /// The same adaptive [`sdroxide_dsp::Diversity`] filter the RSPduo and
    /// the LimeSDR's second chain already use: Cancel or Combine mode,
    /// configurable taps and adaptation rate, with a freeze/hold control.
    pub div_mode: DiversityMode,
    /// How many taps the adaptive filter has, 1 to [`DIVERSITY_MAX_TAPS`].
    pub div_taps: u8,
    /// How fast the filter adapts, 0 to 1.
    pub div_rate: f32,
    /// Hold the filter where it is.
    pub div_frozen: bool,
}

impl Default for FobosConfig {
    fn default() -> Self {
        FobosConfig {
            serial: String::new(),
            port: FobosPort::default(),
            // Narrow enough that the HF ports' near-DC floor (half this
            // rate — see `sample_rate_hz`'s own doc comment) sits at
            // 312.5 kHz, under every AM/mediumwave broadcast frequency, not
            // above most of them: a wider figure such as 2 Msps (this
            // backend's first default) puts the floor at 1.25 MHz, which
            // silently makes the entire AM broadcast band untunable on
            // Hf1/Hf2/HfDual — the dial accepts a lower frequency, the
            // downconverter clamps it back up to the floor without saying
            // so, and whatever is actually at the floor gets decoded and
            // labelled with the frequency the operator asked for instead.
            // 625 kHz is also this crate's own achievable minimum: the
            // downconverter's smallest bin count already lands here, so
            // nothing narrower is being left on the table by picking this
            // as the default. On `Rf` this may snap to a different rate —
            // see `sample_rate_hz`'s own doc comment — which is reported
            // back rather than hidden.
            sample_rate_hz: 625_000.0,
            // The values this backend was verified against real hardware
            // with, not a documented "recommended" figure — a reasonable,
            // moderate starting point either way.
            lna_gain: 2,
            vga_gain: 10,
            clk_external: false,
            div_mode: DiversityMode::default(),
            div_taps: 8,
            div_rate: 0.7,
            div_frozen: false,
        }
    }
}

impl FobosConfig {
    /// A real hardware gain stage, 0..3 — see [`FobosPort`]'s own doc
    /// comment for why it only does anything on [`FobosPort::Rf`].
    pub const LNA_GAIN_ELEMENT: &'static str = "LNA";
    pub const LNA_GAIN_MAX: u8 = 3;
    /// A real hardware gain stage, 0..31, same [`FobosPort::Rf`]-only
    /// caveat as `LNA_GAIN_ELEMENT`.
    pub const VGA_GAIN_ELEMENT: &'static str = "VGA";
    pub const VGA_GAIN_MAX: u8 = 31;
    /// Pseudo-element carrying the clock-source switch: internal (default)
    /// or external reference. Rides the existing `SetGain` command so this
    /// backend needs no new `Command` variant, and is deliberately absent
    /// from `DeviceCaps::gains` so nothing renders it as a slider.
    pub const CLK_EXTERNAL_ELEMENT: &'static str = "CLKEXT";
    /// See [`DIVERSITY_MAX_TAPS`] — the settings panel's own bound on
    /// `div_taps`, same constant every other diversity-capable backend here
    /// uses.
    pub const DIV_TAPS_MAX: u8 = DIVERSITY_MAX_TAPS;

    /// Sample rates to offer before a receiver has answered — the settings
    /// tab shows the device's own reported list once `DeviceCaps` has one,
    /// same as [`HydraSdrConfig::SAMPLE_RATES`]. These are not from a
    /// datasheet: they are the 13-entry list `fobos_rx_get_samplerates`
    /// reported on the real unit this backend was verified against (which
    /// may not be every Fobos's own list), plus the 625 kHz entry
    /// `sdroxide-fobos::device::samplerates` adds to that reported list for
    /// the HF ports — see its own doc comment.
    ///
    /// [`FobosConfig::default`]'s `sample_rate_hz` must always appear here
    /// and in that list both, or the settings dropdown renders it as
    /// selected text with no entry behind it and the first touch of that
    /// control loses it for good — which is exactly what happened to 625 kHz,
    /// the rate the default was chosen as in the first place.
    pub const SAMPLE_RATES: [f64; 14] = [
        0.625e6, 1.25e6, 2.5e6, 5.0e6, 8.0e6, 10.0e6, 12.5e6, 16.0e6, 20.0e6, 25.0e6, 32.0e6,
        40.0e6, 50.0e6, 80.0e6,
    ];
}

/// Where the antenna this radio listens on actually is.
///
/// The station's locator says where the *operator* is, and for a radio in the
/// shack that is also where the antenna is. It is not where a public KiwiSDR or
/// SpyServer is: issue #284, where an online receiver in Australia taken in a
/// tab in Europe reported 2 m FT8 decodes as an intercontinental opening,
/// because everything heard was still posted from the operator's own square.
///
/// So a radio says which. Reception reports — PSK Reporter, WSPRnet, FreeDV
/// Reporter — and the ADS-B receiver position are taken from here rather than
/// from [`crate::DigiConfig::my_grid`].
///
/// Externally tagged (serde's default shape), like [`ConverterTx`] above and
/// for the same reason: this config crosses the remote link as postcard, which
/// is not self-describing and refuses an internally or adjacently tagged enum
/// outright.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RxSite {
    /// The antenna is the station's own, so the operator's locator describes
    /// it. The default, and what every radio configured before this existed
    /// gets — which is right, because until public receivers could be taken in
    /// a tab it was the only possibility.
    #[default]
    Station,
    /// Somewhere else, at this Maidenhead locator: an online receiver, or the
    /// operator's own set on a hilltop.
    ///
    /// **Empty means somewhere else and nobody knows where** — a directory
    /// entry that published no position. Nothing is reported then, because a
    /// report has to say where it was heard and there is no honest answer.
    Elsewhere(String),
}

impl RxSite {
    /// What the settings dialog calls this.
    pub fn label(&self) -> &'static str {
        match self {
            RxSite::Station => "At the station",
            RxSite::Elsewhere(_) => "Somewhere else",
        }
    }
}

/// Tuning ranges an operator stated for an interface this radio is not on at
/// the moment.
///
/// [`RadioConfig::freq_ranges_rx`] and its transmit twin describe *a device*,
/// and a tab can be moved from one device to another — by the interface picker
/// in Settings, or wholesale by "Public SDRs". Leaving the numbers where they
/// were makes them a statement about hardware that is no longer there: issue
/// #254, where taking a public SpyServer in the tab an IC-9700 was in left the
/// receiver's published 200-350 MHz clamped over the Icom when the operator
/// switched back, with no way to see where it had come from.
///
/// So the ranges travel with the interface they were stated for.
/// [`RadioConfig::set_backend`] parks the pair being left here and takes back
/// whatever this radio last stated for the one being taken up. One entry per
/// interface at most, which bounds the list at the number of variants
/// [`Backend`] has.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ParkedRanges {
    pub backend: Backend,
    pub rx: Vec<(f64, f64)>,
    pub tx: Vec<(f64, f64)>,
}

/// Persisted backend configuration (`radio.json`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RadioConfig {
    pub backend: Backend,
    /// Sound-card device (cpal name) carrying the radio's RX audio → PC.
    pub radio_audio_in: Option<String>,
    /// Sound-card device (cpal name) carrying the TX audio PC → radio.
    pub radio_audio_out: Option<String>,
    /// External frequency converter in the antenna line: the hardware is tuned
    /// this far from the operator's dial, in Hz. So `+125_000_000` is a Ham It
    /// Up HF upconverter and the dial reads the real on-air frequency, and a
    /// negative value is a down-converter such as a satellite LNB. `0.0` (the
    /// default) is no converter and leaves tuning exactly as it was.
    ///
    /// Hz rather than MHz because that is the unit every converter's
    /// documentation and every other SDR program states it in, and a number
    /// copied from one of those has to mean the same thing here.
    ///
    /// Receive by default — a converter is a receive accessory, so transmit is
    /// withdrawn while this is set unless [`Self::converter_tx`] says what is
    /// in the transmit line.
    pub converter_offset_hz: f64,
    /// What the transmit path does while [`Self::converter_offset_hz`] is set:
    /// nothing (the default), the same conversion (a transverter), or an offset
    /// of its own — including none at all, which is the QO-100 station whose
    /// downlink comes through an LNB and whose uplink leaves the radio direct.
    ///
    /// Ignored when there is no converter: with the offset at zero the transmit
    /// path was never touched to begin with.
    pub converter_tx: ConverterTx,
    /// The station's transverters, each with its own band and its own offset —
    /// see [`Transverter`].
    ///
    /// Where [`Self::converter_offset_hz`] is one answer for the whole dial
    /// (an upconverter, an LNB: a box in front of *everything*), this is the
    /// station whose 2 m, 6 m and 23 cm come from three different boxes on one
    /// I.F. The dial decides which applies; a frequency no row covers falls
    /// through to the single offset above, and to the bare radio when that is
    /// zero too (issue #278).
    ///
    /// Appended last, and `#[serde(default)]`, so a `radio.json` written before
    /// this existed loads with an empty table and behaves exactly as it did.
    #[serde(default)]
    pub transverters: Vec<Transverter>,
    /// Tuning ranges the operator states for this radio, in Hz, replacing what
    /// the device publishes about itself. Empty (the default) leaves the
    /// device's own answer alone.
    ///
    /// Two things need this. A driver may publish no range at all — it is an
    /// optional call in SoapySDR, and SoapySX among others does not implement
    /// it — which leaves the program with nothing to check a frequency
    /// against. Or the range it publishes may be the silicon's rather than the
    /// radio's: a transceiver whose filters and PA cover one band still reports
    /// whatever its tuner chip can synthesise, and an operator who wants the
    /// dial and the transmit gate held to the real hardware has to say so.
    ///
    /// These describe the *device*, on the hardware side of any converter
    /// offset, which is where its own published ranges come from.
    pub freq_ranges_rx: Vec<(f64, f64)>,
    /// Transmit ranges the operator states, by the same rule as
    /// [`Self::freq_ranges_rx`] — and the licence gate still applies on top,
    /// so naming a range here is not a way around `tx_ham_only`.
    ///
    /// This cannot conjure a transmitter: a receive-only device has no TX
    /// channel and stays receive-only whatever is written here.
    pub freq_ranges_tx: Vec<(f64, f64)>,
    pub cat: CatConfig,
    pub hpsdr: HpsdrConfig,
    pub tci: TciConfig,
    pub icomnet: IcomNetConfig,
    pub smartsdr: SmartSdrConfig,
    pub rtlsdr: RtlSdrConfig,
    pub rtltcp: RtlTcpConfig,
    pub spyserver: SpyServerConfig,
    /// The VFO+FFT interface's own block. Same type as `spyserver` above, kept
    /// apart because the two are different interfaces in the picker and an
    /// operator who has both configured means two different servers as often
    /// as one — and because the decimation stage that suits a wideband stream
    /// is nowhere near the one that suits a narrow one.
    pub spyserver_vfo: SpyServerConfig,
    pub rx888: Rx888Config,
    pub airspyhf: AirspyHfConfig,
    pub airspy: AirspyConfig,
    pub hackrf: HackRfConfig,
    pub pluto: PlutoConfig,
    pub sdrplay: SdrPlayConfig,
    /// Another roster radio used as this one's receiver. Appended last, like
    /// every other field here: the layout is positional.
    pub panadapter: PanadapterConfig,
    /// ELAD FDM-DUO / FDM-S. Appended after `panadapter` because the layout is
    /// positional and every field is only ever added at the end.
    pub elad: EladConfig,
    /// LimeSDR family through LimeSuite, and the LimeRFE in front of it.
    /// Appended after `elad` for the same reason.
    pub lime: LimeConfig,
    /// The SoapySDR interface's own block. Appended after `lime`, for the same
    /// reason as every field above it: the layout is positional.
    pub soapy: SoapyConfig,
    /// HydraSDR RFOne. Appended after `soapy`, for the same reason as every
    /// field above it.
    pub hydrasdr: HydraSdrConfig,
    /// KiwiSDR / Web-888. Appended after `hydrasdr`, for the same reason as
    /// every field above it: the layout is positional.
    pub kiwi: KiwiConfig,
    /// Stated tuning ranges belonging to interfaces this radio is not on —
    /// see [`ParkedRanges`]. Appended after `kiwi`, for the same reason as
    /// every field above it: the layout is positional.
    ///
    /// Empty in a configuration written before this existed, which is exactly
    /// right: such a radio has only ever stated ranges for the interface it is
    /// on, and those are still in the two fields above.
    pub freq_ranges_parked: Vec<ParkedRanges>,
    /// RigExpert Fobos SDR. Appended after `freq_ranges_parked`, for the same
    /// reason as every field above it: the layout is positional, so a new
    /// block goes on the end and nowhere else — and, appended or not, it is
    /// what made `PROTO_VERSION` 118 (see that constant's own history entry:
    /// this struct rides `ServerMsg::RadioConfig` and
    /// `Command::SetRadioConfig` whole, so a peer one field short reads the
    /// tail of every one of them out of step).
    pub fobos: FobosConfig,
    /// Where this radio's antenna is — see [`RxSite`]. Appended after `fobos`,
    /// for the same reason as every field above it: the layout is positional,
    /// so a new block goes on the end and nowhere else, and `RadioConfig` rides
    /// `ServerMsg::RadioConfig` and `Command::SetRadioConfig` whole (issue
    /// #284).
    pub rx_site: RxSite,
    /// Per-band transmit drive calibration — see [`BandDriveTrim`]. Appended
    /// after `rx_site`, for the same reason as every field above it: the
    /// layout is positional, so a new block goes on the end and nowhere else,
    /// and `RadioConfig` rides `ServerMsg::RadioConfig` and
    /// `Command::SetRadioConfig` whole (issue #295).
    ///
    /// Empty in a configuration written before this existed, and empty is
    /// exactly right: no row means no trim, and the radio transmits at the
    /// operator's Drive setting on every band as it always did.
    pub tx_drive_trim: Vec<BandDriveTrim>,
}

impl RadioConfig {
    /// The converter offset that is actually in force at `dial_hz`, to be
    /// changed in place.
    ///
    /// A measurement made *through* a converter — the QO-100 beacon's, above
    /// all — corrects the box the dial is currently looking through, and on a
    /// station with a transverter table that is not necessarily
    /// [`Self::converter_offset_hz`]. The engine resolves the same precedence
    /// when it tunes (`ConverterPlan`: a band-limited row first, the single
    /// offset behind it), so a correction written anywhere else is written
    /// into a number nothing reads, the beacon does not move, and a closed
    /// loop watching for it to move never stops correcting.
    ///
    /// Falls through to [`Self::converter_offset_hz`] when no row covers the
    /// dial, which is the bare radio and the single-converter station both.
    pub fn converter_offset_at_mut(&mut self, dial_hz: f64) -> &mut f64 {
        match self.transverters.iter_mut().find(|x| x.covers(dial_hz)) {
            Some(x) => &mut x.offset_hz,
            None => &mut self.converter_offset_hz,
        }
    }

    /// The drive calibration in force at `tx_dial_hz`, in dB of output power,
    /// or `0.0` where the operator has set none (issue #295).
    ///
    /// The *dial* transmit frequency, which behind a transverter is the
    /// converted band rather than the I.F. the radio is really on: the table
    /// calibrates the amplifier that puts the signal on the air, and on a
    /// transverter station that is the transverter. What its I.F. input can
    /// take is a separate and harder limit — [`Transverter::tx_drive`] — and
    /// the engine applies that after this.
    ///
    /// A band with more than one row takes the first: rows are the operator's
    /// and a duplicate is a mistake, not an instruction to add them up.
    pub fn drive_trim_db(&self, tx_dial_hz: f64) -> f32 {
        if self.tx_drive_trim.is_empty() {
            return 0.0;
        }
        let band = Band::containing(tx_dial_hz);
        self.tx_drive_trim.iter().find(|t| t.band == band).map(BandDriveTrim::db).unwrap_or(0.0)
    }

    /// The converter offset in force at `dial_hz`, read-only.
    pub fn converter_offset_at(&self, dial_hz: f64) -> f64 {
        self.transverters
            .iter()
            .find(|x| x.covers(dial_hz))
            .map(|x| x.offset_hz)
            .unwrap_or(self.converter_offset_hz)
    }

    /// Point this radio at another interface, carrying the stated tuning
    /// ranges with the interface they were stated for.
    ///
    /// Every path that changes [`Self::backend`] on a configured radio goes
    /// through here rather than assigning the field: the ranges are the
    /// operator's word about a *device*, and a tab that has been moved to a
    /// different one must not go on enforcing the last one's limits. See
    /// [`ParkedRanges`] for what that cost in issue #254.
    ///
    /// Nothing else in the configuration moves. Every interface already has a
    /// block of its own, so the address, the sound cards and the converter in
    /// front of them are all still there when a radio comes back to where it
    /// was.
    pub fn set_backend(&mut self, backend: Backend) {
        if backend == self.backend {
            return;
        }
        // Park what the interface being left had stated, under its own name,
        // so that coming back to it is not a retype. Nothing is parked for an
        // interface that stated nothing — an empty pair means "take the
        // device's own answer", which is also what a missing entry means.
        let (rx, tx) =
            (std::mem::take(&mut self.freq_ranges_rx), std::mem::take(&mut self.freq_ranges_tx));
        let left = self.backend;
        self.freq_ranges_parked.retain(|p| p.backend != left);
        if !rx.is_empty() || !tx.is_empty() {
            self.freq_ranges_parked.push(ParkedRanges { backend: left, rx, tx });
        }
        // ...and take back whatever was stated for the one being taken up.
        if let Some(i) = self.freq_ranges_parked.iter().position(|p| p.backend == backend) {
            let p = self.freq_ranges_parked.remove(i);
            self.freq_ranges_rx = p.rx;
            self.freq_ranges_tx = p.tx;
        }
        // The antenna's whereabouts belong to the receiver being left, not to
        // the tab: a KiwiSDR in Alberta swapped for the transceiver in the
        // shack must not go on posting the shack's decodes from Alberta. Back
        // to the station, which is the only thing that can be assumed about a
        // device that has just been plugged in. Picking a public receiver from
        // the directory sets it again afterwards — `PublicSdrEntry::radio_config`
        // calls this first and states the site second, in that order.
        self.rx_site = RxSite::Station;
        self.backend = backend;
    }

    /// The Maidenhead locator receptions through this radio are reported under,
    /// given the station's own — or `None` when the antenna is somebody else's
    /// and the directory did not say where it is.
    ///
    /// `None` is not "fall back on the operator's square": that is exactly the
    /// wrong answer, and the one issue #284 was about. A caller with nothing to
    /// report from reports nothing.
    pub fn report_grid<'a>(&'a self, station_grid: &'a str) -> Option<&'a str> {
        match &self.rx_site {
            RxSite::Station => Some(station_grid.trim()),
            RxSite::Elsewhere(g) if !g.trim().is_empty() => Some(g.trim()),
            RxSite::Elsewhere(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The SoapySDR block's defaults must mean "change nothing", because it
    /// covers every driver SoapySDR has and almost none of them has ever been
    /// run here. A configuration written before this existed opens its radio
    /// exactly as it always did.
    #[test]
    fn an_untouched_soapy_block_asserts_nothing() {
        let cfg = SoapyConfig::default();
        assert_eq!(cfg.sample_rate_hz, 0.0, "a rate would be forced on every driver");
        assert_eq!(cfg.bandwidth_hz, 0.0, "a filter would override the driver's own choice");
        assert!(cfg.settings.is_empty());
        assert_eq!(cfg.setting("bias_tx"), None);
    }

    /// A key is remembered once and updated in place: a second choice for the
    /// same setting replaces the first rather than queuing behind it, or the
    /// list would grow without bound and the wrong one would be applied last.
    #[test]
    fn a_soapy_setting_is_remembered_once_and_updated_in_place() {
        let mut cfg = SoapyConfig::default();
        cfg.set_setting("bias_tx", "true");
        cfg.set_setting("direct_samp", "2");
        cfg.set_setting("bias_tx", "false");
        assert_eq!(cfg.settings.len(), 2, "{:?}", cfg.settings);
        assert_eq!(cfg.setting("bias_tx"), Some("false"));
        assert_eq!(cfg.setting("direct_samp"), Some("2"));
        // Order is insertion order, so what the operator set first is applied
        // first — some drivers care (a direct-sampling branch before a gain).
        assert_eq!(cfg.settings[0].0, "bias_tx");
    }

    /// A config written before the rate was selectable has to come up at the
    /// rate it was written for. Every such rig was opened at 48 kHz, and a
    /// panadapter that silently changed width on upgrade — or, worse, a config
    /// that failed to deserialise and took every other radio setting with it —
    /// is the cost of getting this wrong.
    #[test]
    fn a_config_from_before_the_setting_still_opens_at_48_khz() {
        let before = serde_json::to_value(CatConfig::default()).unwrap();
        let mut before = before.as_object().unwrap().clone();
        assert!(before.remove("iq_rate_hz").is_some(), "the field is in the written form");

        let loaded: CatConfig = serde_json::from_value(before.into()).unwrap();
        assert_eq!(loaded.iq_rate_hz, 48_000);
        assert_eq!(loaded, CatConfig::default(), "and nothing else moved");
    }

    /// Appending a family-specific field must not cost every existing operator
    /// their interface selection. serde fails a `RadioConfig` whole, and the
    /// default backend would then go and grab whatever hardware it found first
    /// — see the note on [`KenwoodSend::Ts2000`].
    #[test]
    fn a_config_from_before_the_elad_family_still_loads() {
        let before = serde_json::to_value(CatConfig::default()).unwrap();
        let mut before = before.as_object().unwrap().clone();
        assert!(before.remove("elad_tx_input").is_some(), "the field is in the written form");

        let loaded: CatConfig = serde_json::from_value(before.into()).unwrap();
        assert_eq!(loaded.elad_tx_input, EladTxInput::UsbAudio);
        assert_eq!(loaded, CatConfig::default(), "and nothing else moved");
    }

    /// Same contract for the flrig address: a config written before the family
    /// existed must still load, and come up pointing at flrig's own default
    /// port.
    #[test]
    fn a_config_from_before_the_flrig_family_still_loads() {
        let before = serde_json::to_value(CatConfig::default()).unwrap();
        let mut before = before.as_object().unwrap().clone();
        assert!(before.remove("flrig_addr").is_some(), "the field is in the written form");

        let loaded: CatConfig = serde_json::from_value(before.into()).unwrap();
        assert_eq!(loaded.flrig_addr, "127.0.0.1:12345");
        assert_eq!(loaded, CatConfig::default(), "and nothing else moved");
    }

    /// Every family the combo box offers has to reach a profile. The array
    /// length is hard-coded, so a variant added without touching it silently
    /// disappears from the dialog instead of failing to build.
    #[test]
    fn every_cat_family_is_offered_and_labelled() {
        assert_eq!(CatFamily::ALL.len(), 9);
        for f in CatFamily::ALL {
            assert!(!f.label().is_empty(), "{f:?}");
        }
        assert!(CatFamily::ALL.contains(&CatFamily::Elad));
        assert!(CatFamily::ALL.contains(&CatFamily::QrpLabs));
        // ELAD is a serial family: the FDM-DUO's CAT port is an FTDI bridge,
        // not a socket.
        assert!(!CatFamily::Elad.is_network());
        // A QMX is one too — its control port is a virtual COM port the radio
        // itself serves over USB.
        assert!(!CatFamily::QrpLabs.is_network());
        // flrig is the other daemon: a socket, never a serial port.
        assert!(CatFamily::Flrig.is_network());
    }

    /// sdroxide's own serial default is a rate no FDM-DUO has, and the `cat`
    /// block it lives in is shared with the CAT / Audio interface — so the
    /// configuration an ELAD owner starts from describes a control port the
    /// radio cannot hear a word of. That was issue #146: a DUO receiving
    /// perfectly and refusing to transmit on every port its owner tried.
    #[test]
    fn the_default_baud_is_one_no_fdm_duo_has() {
        let default = SerialConfig::default().baud;
        assert!(
            !ELAD_CAT_BAUDS.contains(&default),
            "if the shared default ever becomes an ELAD rate, the substitution below stops \
             being the thing that makes a fresh configuration work"
        );
        assert_eq!(elad_cat_baud(default), ELAD_DEFAULT_CAT_BAUD);
    }

    /// A rate the radio *does* have is left exactly as it is: menu 70 is the
    /// operator's own setting, and an owner who moved it off the factory 38400
    /// must not be dragged back to it.
    #[test]
    fn a_rate_the_radio_has_is_left_alone() {
        for b in ELAD_CAT_BAUDS {
            assert_eq!(elad_cat_baud(b), b, "{b} is one of the radio's own");
        }
        assert!(ELAD_CAT_BAUDS.contains(&ELAD_DEFAULT_CAT_BAUD), "the fallback must be reachable");
        // The rates the manual does not list, including the ones every other
        // family here uses.
        for b in [1200, 4800, 19_200, 230_400] {
            assert_eq!(elad_cat_baud(b), ELAD_DEFAULT_CAT_BAUD, "{b} is not an FDM-DUO rate");
        }
    }

    /// The offset is a position inside the digitised window, so what may be
    /// asked for grows with the window. Half the rate either way, and the 8 kHz
    /// an Elecraft's `RX SHFT` asks for fits in the narrowest of them.
    #[test]
    fn the_iq_offset_ceiling_follows_the_sample_rate() {
        assert_eq!(cat_iq_offset_max_hz(48_000), 24_000.0);
        assert_eq!(cat_iq_offset_max_hz(192_000), 96_000.0);
        for rate in CAT_IQ_RATES {
            assert!(cat_iq_offset_max_hz(rate) >= 8_000.0, "{rate} Hz cannot express RX SHFT");
        }
    }

    fn hackrf(name: &str, pid: u16) -> HackRfDevice {
        HackRfDevice {
            serial: Some("0000000000000000457863c8267a765f".into()),
            name: name.into(),
            pid,
        }
    }

    /// A HackRF Pro and a HackRF One share `0x6089`, so the settings UI has to
    /// tell them apart by the USB product string — and it has to, because the
    /// two do not accept the same sample rates.
    #[test]
    fn a_hackrf_pro_is_recognised_from_its_product_string() {
        assert!(hackrf("HackRF Pro", 0x6089).is_pro());
        // The descriptor's case and padding are the firmware's choice.
        assert!(hackrf(" hackrf pro ", 0x6089).is_pro());

        assert!(!hackrf("HackRF One", 0x6089).is_pro());
        assert!(!hackrf("", 0x6089).is_pro());
        // The string never overrides an id that already names another board.
        assert!(!hackrf("HackRF Pro", 0xcc15).is_pro());
        assert!(!hackrf("HackRF Pro", 0x604b).is_pro());
    }

    /// Suffix matching, because nobody types 32 hex digits. The driver opens by
    /// this rule, so the settings panel has to select by it too or the two
    /// disagree about which radio is being configured.
    #[test]
    fn a_configured_serial_selects_a_device_by_its_suffix() {
        let d = hackrf("HackRF Pro", 0x6089);
        assert!(d.matches_serial("267a765f"));
        assert!(d.matches_serial("267A765F"), "case is the firmware's choice");
        assert!(d.matches_serial(" 267a765f "), "pasted values carry whitespace");
        assert!(d.matches_serial("0000000000000000457863c8267a765f"));
        // No serial configured means the first radio listed will do.
        assert!(d.matches_serial(""));
        assert!(d.matches_serial("  "));
        // A prefix is not a suffix, a wrong tail is not a match, and something
        // longer than the real serial must not panic.
        assert!(!d.matches_serial("00000000"));
        assert!(!d.matches_serial("267a7650"));
        assert!(!d.matches_serial(&"f".repeat(64)));
        // A radio with no serial descriptor can only ever satisfy "any".
        let anon = HackRfDevice { serial: None, ..d };
        assert!(anon.matches_serial(""));
        assert!(!anon.matches_serial("267a765f"));
    }

    /// The Pro's extra rates are the Pro's alone: offering them on a HackRF One
    /// would hand somebody a stream with three quarters of a megahertz of the
    /// rest of the band folded into it.
    #[test]
    fn only_a_pro_is_offered_the_narrow_rates() {
        let one = HackRfConfig::rates_for(false);
        assert_eq!(one, HackRfConfig::SAMPLE_RATES.to_vec());

        let pro = HackRfConfig::rates_for(true);
        assert_eq!(pro.len(), one.len() + HackRfConfig::PRO_SAMPLE_RATES.len());
        assert_eq!(pro[0], 250_000.0, "narrowest first, so the menu reads in order");
        assert!(pro.ends_with(&one), "every shared rate is still there, unchanged");
        assert!(pro.windows(2).all(|w| w[0] < w[1]), "the menu must be sorted: {pro:?}");

        // Every offered rate carries a note, or the combo shows a bare dash.
        for r in pro {
            assert!(!HackRfConfig::rate_note(r).is_empty(), "{r} has no note");
        }
    }

    #[test]
    fn the_if_source_falls_back_when_the_rate_cannot_carry_it() {
        let mut c = IcomNetConfig { rx_source: IcomRxSource::If12k, ..Default::default() };
        assert!(c.if_mode_usable());
        assert_eq!(c.effective_rx_source(), IcomRxSource::If12k);
        // A 12 kHz IF has nowhere to live in a 24 kHz stream.
        c.sample_rate_hz = 24_000;
        assert!(!c.if_mode_usable());
        assert_eq!(c.effective_rx_source(), IcomRxSource::Af);
    }

    #[test]
    fn every_offered_backend_has_a_label_and_icom_lan_is_offered() {
        assert!(Backend::ALL.contains(&Backend::IcomNet));
        for b in Backend::ALL {
            assert!(!b.label().is_empty());
        }
        // Serde writes the variant name, so an old radio.json must still load
        // and a new one must name this backend recognisably.
        let json = serde_json::to_string(&Backend::IcomNet).unwrap();
        assert_eq!(json, "\"IcomNet\"");
    }

    /// The network RTL-SDR is its own interface, and an older `radio.json`
    /// that predates it still loads — with the USB entry it was written with.
    #[test]
    fn the_rtl_tcp_interface_is_offered_and_named_on_the_wire() {
        assert!(Backend::ALL.contains(&Backend::RtlTcp));
        assert_eq!(serde_json::to_string(&Backend::RtlTcp).unwrap(), "\"RtlTcp\"");
        assert_ne!(Backend::RtlTcp.label(), Backend::RtlSdr.label());

        let cfg: RadioConfig = serde_json::from_str(r#"{"backend": "RtlSdr"}"#).expect("parses");
        assert_eq!(cfg.backend, Backend::RtlSdr);
        assert_eq!(cfg.rtltcp, RtlTcpConfig::default(), "a config with no rtl_tcp block");
    }

    /// The two SpyServer interfaces are separate entries with separate blocks,
    /// and an older `radio.json` that predates both still loads.
    #[test]
    fn both_spyserver_interfaces_are_offered_and_named_on_the_wire() {
        assert!(Backend::ALL.contains(&Backend::SpyServer));
        assert!(Backend::ALL.contains(&Backend::SpyServerVfo));
        assert_eq!(serde_json::to_string(&Backend::SpyServer).unwrap(), "\"SpyServer\"");
        assert_eq!(serde_json::to_string(&Backend::SpyServerVfo).unwrap(), "\"SpyServerVfo\"");
        assert_ne!(Backend::SpyServer.label(), Backend::SpyServerVfo.label());

        let cfg: RadioConfig = serde_json::from_str(r#"{"backend": "RtlTcp"}"#).expect("parses");
        assert_eq!(cfg.spyserver, SpyServerConfig::default(), "a config with no spyserver block");
        assert_eq!(cfg.spyserver_vfo, SpyServerConfig::default());

        // Two blocks, and they really are independent — an operator with a
        // wideband receiver on the LAN and a narrowband one over cellular is
        // the case this split exists for.
        let cfg: RadioConfig = serde_json::from_str(
            r#"{"backend":"SpyServerVfo",
                "spyserver":{"address":"pi.local"},
                "spyserver_vfo":{"address":"remote.example:5556","iq_decimation":7}}"#,
        )
        .expect("parses");
        assert_eq!(cfg.spyserver.endpoint(), "pi.local:5555");
        assert_eq!(cfg.spyserver_vfo.endpoint(), "remote.example:5556");
        assert_eq!(cfg.spyserver_vfo.iq_decimation, 7);
        assert_eq!(cfg.spyserver.iq_decimation, SpyServerConfig::AUTO_DECIMATION);
    }

    /// The format decides what a rate costs on the link, which is the number
    /// that decides whether a remote receiver works at all.
    #[test]
    fn the_sample_format_round_trips_and_states_its_wire_cost() {
        for f in SpyServerFormat::ALL {
            assert_eq!(SpyServerFormat::from_code(f.code()), f);
            assert_eq!(SpyServerFormat::from_wire(f.wire()), Some(f));
            assert!(!f.label().is_empty());
        }
        assert_eq!(SpyServerFormat::Uint8.bytes_per_sample(), 2);
        assert_eq!(SpyServerFormat::Int16.bytes_per_sample(), 4);
        assert_eq!(SpyServerFormat::Float32.bytes_per_sample(), 8);
        // 24-bit and the 4-bit FFT coding are deliberately absent, and must not
        // be silently mapped onto something this program would then misread.
        assert_eq!(SpyServerFormat::from_wire(3), None, "24-bit");
        assert_eq!(SpyServerFormat::from_wire(5), None, "the 4-bit differential coding");
        assert_eq!(SpyServerFormat::from_wire(0), None, "the protocol's 'invalid'");
    }

    /// The port may be left off, and an IPv6 literal is all colons — so the
    /// brackets, not the colons, are what say whether a port is present.
    #[test]
    fn an_address_without_a_port_gets_the_protocol_default() {
        let at = |a: &str| RtlTcpConfig { address: a.into(), ..Default::default() }.endpoint();
        assert_eq!(at("192.168.1.5"), "192.168.1.5:1234");
        assert_eq!(at("192.168.1.5:5678"), "192.168.1.5:5678");
        assert_eq!(at("raspberrypi.local"), "raspberrypi.local:1234");
        assert_eq!(at("[fe80::1]"), "[fe80::1]:1234");
        assert_eq!(at("[fe80::1]:5678"), "[fe80::1]:5678");
        // Typed with a stray space, as a pasted address arrives.
        assert_eq!(at("  10.0.0.9  "), "10.0.0.9:1234");
    }

    /// Every way an existing `radio.json` can arrive has to land on the working
    /// sideband. The one release that shipped this setting called it `swap_iq`
    /// and defaulted it to off, which is the broken value — so that key is
    /// deliberately not read any more, and neither an operator who found the
    /// checkbox nor one who never knew it existed ends up inverted the wrong way.
    #[test]
    fn spectrum_inversion_survives_every_old_config_shape() {
        let cases = [
            // Written before the setting existed at all.
            r#"{"sample_rate_hz": 384000.0}"#,
            // The old key, left at its (broken) default by someone who never
            // opened the HPSDR settings.
            r#"{"sample_rate_hz": 384000.0, "swap_iq": false}"#,
            // The old key, switched on by an operator who diagnosed it.
            r#"{"sample_rate_hz": 384000.0, "swap_iq": true}"#,
            // A completely empty object.
            r#"{}"#,
        ];
        for json in cases {
            let cfg: HpsdrConfig = serde_json::from_str(json).expect("parses");
            assert!(cfg.invert_spectrum, "inverted after loading {json}");
        }
        // A fresh install gets it too.
        assert!(HpsdrConfig::default().invert_spectrum);
        // And an operator who turns it off is still obeyed on the next load.
        let off: HpsdrConfig =
            serde_json::from_str(r#"{"invert_spectrum": false}"#).expect("parses");
        assert!(!off.invert_spectrum);
    }

    /// The sound-card rig's copy of the same setting goes the other way: every
    /// CAT rig already working is on the convention this end assumes, so the
    /// only safe value for a config that predates the checkbox is off.
    #[test]
    fn a_cat_rigs_iq_is_not_inverted_unless_asked() {
        for json in [
            // Written before the setting existed.
            r#"{"format": "Iq"}"#,
            r#"{}"#,
        ] {
            let cfg: CatConfig = serde_json::from_str(json).expect("parses");
            assert!(!cfg.invert_spectrum, "left alone after loading {json}");
        }
        assert!(!CatConfig::default().invert_spectrum);
        // And an operator who ticks it is still inverted on the next load.
        let on: CatConfig = serde_json::from_str(r#"{"invert_spectrum": true}"#).expect("parses");
        assert!(on.invert_spectrum);
        // Round trips through the file it is written to.
        let back: CatConfig =
            serde_json::from_str(&serde_json::to_string(&on).expect("serialises")).expect("parses");
        assert_eq!(back, on);
    }

    /// Correction goes on for everyone, including the configs written before
    /// it existed: an operator cannot be expected to recognise a mirror image
    /// for what it is, and the notch stays at zero because that one *is* an
    /// operator's choice — it takes signal with it (issue #147).
    #[test]
    fn a_cat_rigs_iq_is_corrected_unless_turned_off() {
        for json in [r#"{"format": "Iq"}"#, r#"{}"#] {
            let cfg: CatConfig = serde_json::from_str(json).expect("parses");
            assert!(cfg.iq_correction, "correction after loading {json}");
            assert_eq!(cfg.iq_dc_block_hz, 0.0, "notch after loading {json}");
        }
        assert!(CatConfig::default().iq_correction);
        let off: CatConfig =
            serde_json::from_str(r#"{"iq_correction": false, "iq_dc_block_hz": 300.0}"#)
                .expect("parses");
        assert!(!off.iq_correction);
        assert_eq!(off.iq_dc_block_hz, 300.0);
        let back: CatConfig =
            serde_json::from_str(&serde_json::to_string(&off).expect("serialises"))
                .expect("parses");
        assert_eq!(back, off);
    }

    /// Every `radio.json` written before the converter existed has to keep
    /// tuning the radio exactly where it did — which means the offset must read
    /// back as zero, the one value that takes the whole feature out of circuit.
    #[test]
    fn converter_offset_defaults_to_none() {
        for json in [r#"{}"#, r#"{"backend": "RtlSdr"}"#, r#"{"rx888": {"ppm": 1.5}}"#] {
            let cfg: RadioConfig = serde_json::from_str(json).expect("parses");
            assert_eq!(cfg.converter_offset_hz, 0.0, "converter offset after loading {json}");
        }
        assert_eq!(RadioConfig::default().converter_offset_hz, 0.0);
    }

    /// A correction measured through a converter has to reach the offset the
    /// dial is actually looking through. On a station with a transverter row
    /// covering the band, that is the row — the single offset is behind it and
    /// nothing reads it there (issue #291).
    #[test]
    fn the_offset_in_force_is_the_row_covering_the_dial_then_the_single_one() {
        const QO100: f64 = 10_489_750_000.0;
        let mut cfg = RadioConfig { converter_offset_hz: -9_750_000_000.0, ..Default::default() };

        // No table: the single offset, as it has always been.
        assert_eq!(cfg.converter_offset_at(QO100), -9_750_000_000.0);
        *cfg.converter_offset_at_mut(QO100) += 3_000.0;
        assert_eq!(cfg.converter_offset_hz, -9_749_997_000.0);

        // A row covering the beacon takes precedence, and takes the correction.
        cfg.transverters.push(Transverter {
            name: "LNB".into(),
            rf_lo_hz: 10_489_000_000.0,
            rf_hi_hz: 10_500_000_000.0,
            offset_hz: -9_750_000_000.0,
            ..Default::default()
        });
        assert_eq!(cfg.converter_offset_at(QO100), -9_750_000_000.0);
        *cfg.converter_offset_at_mut(QO100) += 3_000.0;
        assert_eq!(cfg.transverters[0].offset_hz, -9_749_997_000.0, "the row is corrected");
        assert_eq!(cfg.converter_offset_hz, -9_749_997_000.0, "the single offset is left alone");

        // A row for another band does not: 2 m through a transverter must not
        // catch a correction measured on 3 cm.
        cfg.transverters[0].rf_lo_hz = 144_000_000.0;
        cfg.transverters[0].rf_hi_hz = 146_000_000.0;
        assert_eq!(cfg.converter_offset_at(QO100), cfg.converter_offset_hz);
        let up: RadioConfig =
            serde_json::from_str(r#"{"converter_offset_hz": 125000000.0}"#).expect("parses");
        assert_eq!(up.converter_offset_hz, 125_000_000.0);
    }

    /// Every `radio.json` written before this backend existed has to keep
    /// working, and has to land on a Pluto configuration that would actually
    /// open — the USB gadget's address, not an empty string.
    #[test]
    fn pluto_settings_default_for_every_older_config() {
        for json in [r#"{}"#, r#"{"backend": "Tci"}"#, r#"{"rx888": {"ppm": 1.5}}"#] {
            let cfg: RadioConfig = serde_json::from_str(json).expect("parses");
            assert_eq!(cfg.pluto.target(), PlutoConfig::DEFAULT_ADDRESS, "after loading {json}");
            assert_eq!(cfg.pluto.agc, PlutoAgc::SlowAttack);
        }
        // And the new variant round-trips by name, which is how `Backend` is
        // stored — appending it must not have renumbered anything.
        let pluto: RadioConfig = serde_json::from_str(r#"{"backend": "Pluto"}"#).expect("parses");
        assert_eq!(pluto.backend, Backend::Pluto);
        for b in Backend::ALL {
            let json = serde_json::to_string(&b).expect("serialises");
            assert_eq!(serde_json::from_str::<Backend>(&json).expect("round trip"), b);
        }
    }

    /// Every `radio.json` written before this backend existed has to keep
    /// working, and has to land on an SDRplay configuration that would
    /// actually open and hear something.
    #[test]
    fn sdrplay_settings_default_for_every_older_config() {
        for json in [r#"{}"#, r#"{"backend": "Pluto"}"#, r#"{"rx888": {"ppm": 1.5}}"#] {
            let cfg: RadioConfig = serde_json::from_str(json).expect("parses");
            assert_eq!(cfg.sdrplay.sample_rate_hz, 2_000_000.0, "after loading {json}");
            assert_eq!(cfg.sdrplay.agc, SdrPlayAgc::Hz50);
            assert!(!cfg.sdrplay.bias_tee, "no uninvited DC on the antenna after {json}");
            // And one tuner, which is the only setting that costs nothing: a
            // config written before dual-tuner support must not come back
            // asking an RSPduo for a mode the operator never chose.
            assert!(!cfg.sdrplay.duo.enabled, "after loading {json}");
            assert!(!cfg.sdrplay.wants_dual_tuner());
        }
        // A configuration that *does* ask for it keeps the rest of the block's
        // defaults rather than zeroing them — a filter with no taps and no
        // adaptation rate would combine nothing at all. Written under the name
        // the block had in 1.5.x, when combining was all the second tuner
        // could do: that file must keep working, and keep meaning diversity.
        let dual: RadioConfig =
            serde_json::from_str(r#"{"sdrplay": {"diversity": {"enabled": true}}}"#)
                .expect("parses");
        assert!(dual.sdrplay.wants_dual_tuner());
        assert_eq!(dual.sdrplay.duo.role, SdrPlayDuoRole::Diversity);
        assert_eq!(dual.sdrplay.duo.taps, SdrPlayDuo::default().taps);
        assert_eq!(dual.sdrplay.duo.mode, DiversityMode::Cancel);
        // The second aerial is on the tuner the first one is not.
        assert_eq!(dual.sdrplay.duo_tuner, SdrPlayDuoTuner::Tuner1);
        assert_eq!(dual.sdrplay.aux_tuner(), SdrPlayDuoTuner::Tuner2);
        // The name it is written under now reads the same way, and the second
        // tuner can be a radio of its own instead.
        let split: RadioConfig = serde_json::from_str(
            r#"{"sdrplay": {"duo": {"enabled": true, "role": "SecondRadio"},
                            "duo_tuner": "Tuner2"}}"#,
        )
        .expect("parses");
        assert!(split.sdrplay.wants_dual_tuner());
        assert_eq!(split.sdrplay.duo.role, SdrPlayDuoRole::SecondRadio);
        assert_eq!(split.sdrplay.aux_tuner(), SdrPlayDuoTuner::Tuner1);
        // And the new variant round-trips by name, which is how `Backend` is
        // stored — appending it must not have renumbered anything.
        let sdrplay: RadioConfig =
            serde_json::from_str(r#"{"backend": "SdrPlay"}"#).expect("parses");
        assert_eq!(sdrplay.backend, Backend::SdrPlay);
        for b in Backend::ALL {
            let json = serde_json::to_string(&b).expect("serialises");
            assert_eq!(serde_json::from_str::<Backend>(&json).expect("round trip"), b);
        }
    }

    /// The AGC mode rides `SetGain` as the API's own numeric values, which are
    /// not in speed order — a hand-rolled "obvious" mapping here would set a
    /// different loop rate than the label says.
    #[test]
    fn sdrplay_agc_modes_survive_the_pseudo_gain_element_encoding() {
        for mode in SdrPlayAgc::ALL {
            assert_eq!(SdrPlayAgc::from_code(mode.code()), mode, "{}", mode.label());
        }
        // The API's numbering: 0 disable, 1 = 100 Hz, 2 = 50 Hz, 3 = 5 Hz.
        assert_eq!(SdrPlayAgc::Off.code(), 0.0);
        assert_eq!(SdrPlayAgc::Hz100.code(), 1.0);
        assert_eq!(SdrPlayAgc::Hz50.code(), 2.0);
        assert_eq!(SdrPlayAgc::Hz5.code(), 3.0);
        assert_eq!(SdrPlayAgc::from_code(99.0), SdrPlayAgc::Hz50);
    }

    /// A renamed variant must keep reading under its old name.
    ///
    /// Field-reported, and the cost was out of all proportion to the change.
    /// `KenwoodSend::Standard` was renamed `Ts2000`; serde stores this enum by
    /// *name*, so every `radio.json` holding the old one stopped deserializing
    /// — and serde fails the whole `RadioConfig`, not the one field. The loader
    /// quarantined the file and returned the defaults, so an operator running a
    /// native RX-888 was silently switched to the default SoapySDR backend,
    /// which then opened whatever SoapySDR offered first. On that machine it
    /// was LimeSuite, which claims the RX-888's bare Cypress FX3 id — so the
    /// program flooded the console with LimeSuite transfer errors and never
    /// started, and nothing on screen connected any of it to a Kenwood setting
    /// on a rig that was not even plugged in.
    #[test]
    fn a_renamed_kenwood_variant_still_reads_under_its_old_name() {
        for (old, want) in [("Standard", KenwoodSend::Ts2000), ("Data", KenwoodSend::Ts590)] {
            let json = format!(r#"{{"backend": "Rx888", "cat": {{"kenwood_send": "{old}"}}}}"#);
            let cfg: RadioConfig = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("{old:?} no longer parses: {e}"));
            assert_eq!(cfg.cat.kenwood_send, want);
            // The point of the test: the *interface* survives. A config that
            // fails to parse takes this with it and lands on the default
            // backend, which goes looking for hardware.
            assert_eq!(cfg.backend, Backend::Rx888, "the interface selection was lost");
        }
        // The current names round-trip, so the aliases have not displaced them.
        for v in KenwoodSend::ALL {
            let json = serde_json::to_string(&v).expect("serialises");
            assert_eq!(serde_json::from_str::<KenwoodSend>(&json).expect("round trip"), v);
        }
    }

    /// Every `radio.json` written before this backend existed has to keep
    /// working, and has to land on an Airspy HF+ configuration that would
    /// actually open and hear something.
    #[test]
    fn airspyhf_settings_default_for_every_older_config() {
        for json in [r#"{}"#, r#"{"backend": "SdrPlay"}"#, r#"{"rx888": {"ppm": 1.5}}"#] {
            let cfg: RadioConfig = serde_json::from_str(json).expect("parses");
            assert_eq!(cfg.airspyhf.sample_rate_hz, 768_000.0, "after loading {json}");
            assert!(cfg.airspyhf.agc, "a fresh install must not be deaf after {json}");
            assert!(cfg.airspyhf.lib_dsp, "the image balancer is on by default after {json}");
            assert!(!cfg.airspyhf.bias_tee, "no uninvited DC on the antenna after {json}");
            // `None` means "whatever the receiver's own flash says", which is
            // the only value that cannot be wrong on a device we have not seen.
            assert_eq!(cfg.airspyhf.calibration_ppb, None, "after loading {json}");
        }
        // And the new variant round-trips by name, which is how `Backend` is
        // stored — appending it must not have renumbered anything.
        let a: RadioConfig = serde_json::from_str(r#"{"backend": "AirspyHf"}"#).expect("parses");
        assert_eq!(a.backend, Backend::AirspyHf);
        for b in Backend::ALL {
            let json = serde_json::to_string(&b).expect("serialises");
            assert_eq!(serde_json::from_str::<Backend>(&json).expect("round trip"), b);
        }
    }

    /// Every `radio.json` written before this backend existed has to keep
    /// working, and has to land on a HydraSDR configuration that would actually
    /// open and hear something.
    #[test]
    fn hydrasdr_settings_default_for_every_older_config() {
        for json in [r#"{}"#, r#"{"backend": "Airspy"}"#, r#"{"rx888": {"ppm": 1.5}}"#] {
            let cfg: RadioConfig = serde_json::from_str(json).expect("parses");
            // One of the three rates the receiver itself lists, so a fresh
            // install cannot land on an alternate configuration an older
            // firmware might not carry.
            assert_eq!(cfg.hydrasdr.sample_rate_hz, 5_000_000.0, "after loading {json}");
            assert!(
                HydraSdrConfig::rate_note(cfg.hydrasdr.sample_rate_hz).starts_with("listed"),
                "the default rate must be one the receiver reports, after {json}"
            );
            assert_eq!(cfg.hydrasdr.rf_port, HydraSdrPort::Ant, "after loading {json}");
            assert!(!cfg.hydrasdr.bias_tee, "no uninvited DC on the antenna after {json}");
            assert!(cfg.hydrasdr.packing, "a USB 2.0 link needs it after {json}");
            assert!(cfg.hydrasdr.gain_step < HydraSdrConfig::GAIN_STEPS, "after {json}");
        }
        let h: RadioConfig = serde_json::from_str(r#"{"backend": "HydraSdr"}"#).expect("parses");
        assert_eq!(h.backend, Backend::HydraSdr);
        assert!(Backend::ALL.contains(&Backend::HydraSdr), "the picker has to offer it");
    }

    /// The bias tee lives on one socket, so the port and the switch are one
    /// decision. A UI that let them disagree would claim DC on a socket that
    /// has none.
    #[test]
    fn only_the_hydrasdr_antenna_socket_has_a_bias_tee() {
        assert!(HydraSdrPort::Ant.has_bias_tee());
        assert!(!HydraSdrPort::Cable1.has_bias_tee());
        assert!(!HydraSdrPort::Cable2.has_bias_tee());
        assert_eq!(HydraSdrPort::ALL.iter().filter(|p| p.has_bias_tee()).count(), 1);
        // The wire codes are the firmware's `HYDRASDR_RF_PORT_RX*`, and the
        // names are what it publishes for them.
        assert_eq!(HydraSdrPort::Ant.code(), 0);
        assert_eq!(HydraSdrPort::Cable2.name(), "CABLE2");
        for p in HydraSdrPort::ALL {
            assert_eq!(HydraSdrPort::from_code(p.code()), p);
            let json = serde_json::to_string(&p).expect("serialises");
            assert_eq!(serde_json::from_str::<HydraSdrPort>(&json).expect("round trip"), p);
        }
        for g in HydraSdrGain::ALL {
            assert_eq!(HydraSdrGain::from_code(g.code()), g);
        }
    }

    /// Three of this radio's seven rates are ones the receiver reports and four
    /// are not, and the menu has to say which is which — an operator who picks
    /// an alternate on an older firmware gets snapped to a listed rate, and
    /// wants to know that was possible before it happens.
    #[test]
    fn the_hydrasdr_rate_menu_marks_the_ones_the_receiver_does_not_report() {
        let listed: Vec<f64> = HydraSdrConfig::SAMPLE_RATES
            .iter()
            .copied()
            .filter(|r| HydraSdrConfig::rate_note(*r).starts_with("listed"))
            .collect();
        assert_eq!(listed, vec![10.0e6, 5.0e6, 2.5e6], "the firmware's primary configurations");
        let alternate: Vec<f64> = HydraSdrConfig::SAMPLE_RATES
            .iter()
            .copied()
            .filter(|r| HydraSdrConfig::rate_note(*r).starts_with("alternate"))
            .collect();
        assert_eq!(alternate, vec![12.0e6, 8.0e6, 6.0e6, 4.096e6], "the alternate table");
        // Every rate on the menu is annotated, and the menu is widest first.
        for r in HydraSdrConfig::SAMPLE_RATES {
            assert!(!HydraSdrConfig::rate_note(r).is_empty(), "{r} is unannotated");
        }
        assert!(HydraSdrConfig::SAMPLE_RATES.windows(2).all(|w| w[0] > w[1]));
    }

    /// A default the operator can never get back to is worse than no default:
    /// the settings dropdown builds its entries from this list and renders
    /// `sample_rate_hz` as the *selected text* whether or not an entry
    /// matches, so a default missing from the list looks selected right up
    /// until the first click and then cannot be chosen again. 625 kHz was
    /// exactly that — and it is not an arbitrary rate to lose: it is the one
    /// the default was chosen as, because half of it is the HF ports'
    /// reachable floor and anything wider puts that floor above the bottom of
    /// the AM broadcast band (see `FobosConfig::default`'s own comment, and
    /// `sdroxide-fobos::stream`'s
    /// `the_default_hf_rate_keeps_the_whole_am_broadcast_band_reachable`).
    #[test]
    fn the_default_fobos_rate_is_one_the_settings_menu_actually_offers() {
        let default = FobosConfig::default().sample_rate_hz;
        assert!(
            FobosConfig::SAMPLE_RATES.iter().any(|r| (r - default).abs() < 1.0),
            "the default {default} Hz is not in SAMPLE_RATES {:?}",
            FobosConfig::SAMPLE_RATES,
        );
        // Narrowest first, the way the menu reads.
        assert!(FobosConfig::SAMPLE_RATES.windows(2).all(|w| w[0] < w[1]));
    }

    /// The `part_id` word is the only thing that says which HF+ is on the other
    /// end — every model enumerates as the same `03eb:800c`.
    #[test]
    fn airspyhf_models_decode_from_the_board_part_id() {
        assert_eq!(AirspyHfModel::from_part_id(1), AirspyHfModel::Dual);
        assert_eq!(AirspyHfModel::from_part_id(2), AirspyHfModel::Discovery);
        assert_eq!(AirspyHfModel::from_part_id(3), AirspyHfModel::Ranger);
        // Anything else is a model this driver predates. It still has to open
        // and hear, so it gets the conservative ranges rather than none.
        for unknown in [0, 4, 99, u32::MAX] {
            assert_eq!(AirspyHfModel::from_part_id(unknown), AirspyHfModel::Unknown);
        }
        assert_eq!(AirspyHfModel::Unknown.freq_ranges(), AirspyHfModel::Dual.freq_ranges());
        // The Discovery reaches further down than the Dual; nothing reaches
        // above 260 MHz, and nothing covers 31–60 MHz.
        assert!(
            AirspyHfModel::Discovery.freq_ranges()[0].0 < AirspyHfModel::Dual.freq_ranges()[0].0
        );
        for m in [AirspyHfModel::Dual, AirspyHfModel::Discovery, AirspyHfModel::Ranger] {
            let r = m.freq_ranges();
            assert_eq!(r.len(), 2, "{}", m.label());
            assert!(r[0].1 < r[1].0, "the HF and VHF windows must not touch: {}", m.label());
        }
    }

    /// The `hwVer` byte is the only thing that says which RSP is on the other
    /// end, and its numbering is historical rather than sequential — RSP1A is
    /// 255, RSP1B is 6, and 5 does not exist.
    #[test]
    fn sdrplay_models_decode_from_the_api_hw_ver() {
        assert_eq!(SdrPlayModel::from_hw_ver(1), SdrPlayModel::Rsp1);
        assert_eq!(SdrPlayModel::from_hw_ver(2), SdrPlayModel::Rsp2);
        assert_eq!(SdrPlayModel::from_hw_ver(3), SdrPlayModel::RspDuo);
        assert_eq!(SdrPlayModel::from_hw_ver(4), SdrPlayModel::RspDx);
        assert_eq!(SdrPlayModel::from_hw_ver(6), SdrPlayModel::Rsp1b);
        assert_eq!(SdrPlayModel::from_hw_ver(7), SdrPlayModel::RspDxR2);
        assert_eq!(SdrPlayModel::from_hw_ver(255), SdrPlayModel::Rsp1a);
        assert_eq!(SdrPlayModel::from_hw_ver(5), SdrPlayModel::Unknown);
        // Model-gated UI depends on these staying honest.
        assert!(!SdrPlayModel::Rsp1.has_bias_tee());
        assert!(SdrPlayModel::Rsp1b.has_dab_notch());
        assert!(!SdrPlayModel::Rsp1b.has_hdr());
        assert!(SdrPlayModel::RspDx.has_hdr());
    }

    /// The RSPdx's ladder, band by band, against the gain tables in the API
    /// specification (v3.15 p. 39). The model-wide ceiling is 27 and only
    /// 250–420 MHz has all of it — everywhere else a slider built from the
    /// ceiling offers states the service refuses.
    #[test]
    fn the_rspdx_ladder_follows_the_band() {
        let dx = SdrPlayModel::RspDx;
        let at = |hz: f64| dx.max_lna_state_on(hz, false, false);
        assert_eq!(at(7.295e6), 18, "0-12 MHz has 19 states");
        assert_eq!(at(14.1e6), 19, "12-50 MHz has 20");
        assert_eq!(at(52e6), 24, "50-60 MHz has 25");
        assert_eq!(at(145e6), 26, "60-250 MHz has 27");
        assert_eq!(at(300e6), 27, "250-420 MHz is the only band with all 28");
        assert_eq!(at(435e6), 20, "420-1000 MHz has 21");
        assert_eq!(at(1296e6), 18, "L-band has 19");
        // The HDR path has a table of its own, and only below 2 MHz.
        assert_eq!(dx.max_lna_state_on(1e6, false, true), 21);
        assert_eq!(dx.max_lna_state_on(3e6, false, true), 18, "HDR is a sub-2 MHz path");
        // The ceiling is a ceiling: no band exceeds it, and one reaches it.
        for mhz in [0.5, 7.0, 14.0, 52.0, 145.0, 300.0, 435.0, 1296.0] {
            assert!(at(mhz * 1e6) <= dx.max_lna_state());
        }
    }

    /// HDR is not a mode that follows the dial: each filter is built at a
    /// fixed set of centres and does nothing elsewhere. The lists are from the
    /// API specification's RSPdx section, and the two narrow filters share one
    /// while the two wide ones share the other.
    #[test]
    fn the_hdr_path_only_works_at_its_own_centres() {
        use SdrPlayHdrBw::*;
        // 500 kHz set.
        assert!(Khz500.works_at(135_000.0));
        assert!(Khz500.works_at(475_000.0));
        assert!(Khz200.works_at(220_000.0), "the narrow filters share a set");
        // 2 MHz set.
        assert!(Mhz1_7.works_at(516_000.0));
        assert!(Mhz1_7.works_at(1_900_000.0));
        assert!(Mhz1_2.works_at(875_000.0), "the wide filters share a set");
        // The sets are not interchangeable.
        assert!(!Khz500.works_at(516_000.0));
        assert!(!Mhz1_7.works_at(135_000.0));
        // And nothing else works at all — including the LF/MF frequencies an
        // operator would most reasonably expect to.
        for hz in [0.0, 153_000.0, 198_000.0, 600_000.0, 1_000_000.0, 1_395_000.0] {
            assert!(!Mhz1_7.works_at(hz), "{hz} Hz is not an HDR centre");
        }
        // A hertz of slack, because a dial arrives through a ppm trim.
        assert!(Mhz1_7.works_at(1_900_000.4));
        assert!(!Mhz1_7.works_at(1_900_002.0));
    }

    /// The AGC set point's floor is a property of the sample rate: the
    /// converter trades headroom for speed, and this backend offers rates on
    /// both sides of each threshold.
    #[test]
    fn the_agc_set_point_range_follows_the_rate() {
        let at = |hz: f64| {
            let c = SdrPlayConfig { sample_rate_hz: hz, ..SdrPlayConfig::default() };
            let r = c.agc_setpoint_range();
            (*r.start(), *r.end())
        };
        assert_eq!(at(2_000_000.0), (-72, -20));
        assert_eq!(at(8_000_000.0), (-72, -20), "8 Msps is still under the 8.064 threshold");
        assert_eq!(at(8_064_000.0), (-60, -20));
        assert_eq!(at(9_216_000.0), (-48, -20), "above 9.216 the floor is -48");
        assert_eq!(at(10_000_000.0), (-48, -20), "a rate this backend actually offers");
        // The extended gain floor lifts the ceiling to 0.
        let ext = SdrPlayConfig {
            sample_rate_hz: 2_000_000.0,
            extended_if_gr: true,
            ..SdrPlayConfig::default()
        };
        assert_eq!(*ext.agc_setpoint_range().end(), 0);
    }

    /// The IF gain floor is the API's 20 dB unless the operator opens the last
    /// of the range.
    #[test]
    fn the_extended_range_opens_the_last_twenty_db() {
        let normal = SdrPlayConfig::default();
        assert_eq!(normal.if_gr_min(), SdrPlayConfig::IF_GR_MIN);
        assert_eq!(normal.if_gr_min(), 20);
        let ext = SdrPlayConfig { extended_if_gr: true, ..SdrPlayConfig::default() };
        assert_eq!(ext.if_gr_min(), 0);
    }

    /// A Hi-Z input has fewer front-end states than the 50 Ω socket beside it,
    /// and only two models have one at all.
    #[test]
    fn a_hi_z_port_has_its_own_ladder() {
        let duo = SdrPlayModel::RspDuo;
        assert!(duo.antenna_is_hiz("Hi-Z port"));
        assert!(!duo.antenna_is_hiz("50 Ohm port"));
        assert_eq!(duo.max_lna_state_on(7e6, true, false), 4);
        assert_eq!(duo.max_lna_state_on(7e6, false, false), 6);
        assert!(SdrPlayModel::Rsp2.antenna_is_hiz("Hi-Z"));
        // Every other model routes no Hi-Z, whatever the port is called.
        assert!(!SdrPlayModel::RspDx.antenna_is_hiz("Antenna C"));
        assert!(!SdrPlayModel::Rsp1b.antenna_is_hiz("Hi-Z"));
        // Antenna lists: single-port models hide the selector entirely.
        assert!(SdrPlayModel::Rsp1b.antennas(SdrPlayDuoTuner::Tuner1).is_empty());
        assert_eq!(SdrPlayModel::Rsp2.antennas(SdrPlayDuoTuner::Tuner1).len(), 3);
        assert_eq!(SdrPlayModel::RspDuo.antennas(SdrPlayDuoTuner::Tuner1).len(), 2);
        assert!(SdrPlayModel::RspDuo.antennas(SdrPlayDuoTuner::Tuner2).is_empty());
    }

    /// An RSP enumerating with no serial (or a hwVer nothing decodes) is
    /// SDRplay's signature of a USB brownout or a wedged API service — a
    /// device that lists and streams but hears nothing. Field-reported on an
    /// RSP1B after broadband interference; every surface warns from this one
    /// predicate.
    #[test]
    fn an_rsp_without_an_identity_is_flagged_as_degraded() {
        let healthy = SdrPlayDevice { serial: "2405001234".into(), hw_ver: 6 };
        assert!(!healthy.identity_missing());
        assert!(healthy.identity_warning().is_none());

        let no_serial = SdrPlayDevice { serial: "  ".into(), hw_ver: 6 };
        assert!(no_serial.identity_missing());
        assert!(no_serial.identity_warning().unwrap().contains("no serial number"));

        let no_model = SdrPlayDevice { serial: "2405001234".into(), hw_ver: 0 };
        assert!(no_model.identity_missing());
        assert!(no_model.identity_warning().unwrap().contains("hardware version"));

        // Both point the operator at the same remedy.
        for d in [&no_serial, &no_model] {
            assert!(d.identity_warning().unwrap().contains("Restart the SDRplay API service"));
        }
    }

    /// SoapyAudio presents any sound card as an SDR that accepts every tune and
    /// ignores it. Field-reported: an RSP1A owner on a bundle install spent a
    /// session watching their line input, because "the first device found" was
    /// the sound card. Case folding is the part that is easy to get wrong — an
    /// enumeration says `audio` where the opened device says `Audio`.
    #[test]
    fn soapy_pseudo_drivers_are_recognised_whatever_their_case() {
        for d in ["audio", "Audio", "AUDIO", " audio ", "null", "Null"] {
            assert!(SoapyDeviceInfo::driver_is_pseudo(d), "{d} is not a radio");
            assert!(SoapyDeviceInfo::pseudo_warning(d, "Audio (Audio)").is_some());
        }
        for d in ["sdrplay", "rtlsdr", "hackrf", "lime", "uhd", "remote", ""] {
            assert!(!SoapyDeviceInfo::driver_is_pseudo(d), "{d} is real hardware");
            assert!(SoapyDeviceInfo::pseudo_warning(d, "x").is_none());
        }
        // The warning names the device and points somewhere useful.
        let w = SoapyDeviceInfo::pseudo_warning("Audio", "Audio (Audio)").unwrap();
        assert!(w.contains("Audio (Audio)") && w.contains("ignores the dial"));

        // Drivers with a native interface steer there; the rest stay on SoapySDR.
        assert_eq!(SoapyDeviceInfo::native_backend_for("sdrplay"), Some(Backend::SdrPlay));
        assert_eq!(SoapyDeviceInfo::native_backend_for("SDRplay"), Some(Backend::SdrPlay));
        assert_eq!(SoapyDeviceInfo::native_backend_for("rtlsdr"), Some(Backend::RtlSdr));
        assert_eq!(SoapyDeviceInfo::native_backend_for("plutosdr"), Some(Backend::Pluto));
        assert_eq!(SoapyDeviceInfo::native_backend_for("airspyhf"), Some(Backend::AirspyHf));
        assert_eq!(SoapyDeviceInfo::native_backend_for("AirspyHF"), Some(Backend::AirspyHf));
        // The R2/Mini are a different radio behind a different SoapySDR module,
        // and now have a native backend of their own. The pair below is the
        // thing worth guarding: `airspy` and `airspyhf` must never steer at
        // each other's driver, because each would open the wrong silicon.
        assert_eq!(SoapyDeviceInfo::native_backend_for("airspy"), Some(Backend::Airspy));
        assert_eq!(SoapyDeviceInfo::native_backend_for("Airspy"), Some(Backend::Airspy));
        assert_ne!(
            SoapyDeviceInfo::native_backend_for("airspy"),
            SoapyDeviceInfo::native_backend_for("airspyhf"),
            "two different radios must not share a backend"
        );
        // And SoapyHydraSDR is a third module for a third radio. It reaches the
        // RFOne, but it cannot select the RF port and it stops at the three
        // sample rates the firmware admits to — and steering it at the Airspy
        // driver would be worse than either, because the two program their
        // tuners differently.
        assert_eq!(SoapyDeviceInfo::native_backend_for("hydrasdr"), Some(Backend::HydraSdr));
        assert_eq!(SoapyDeviceInfo::native_backend_for("HydraSDR"), Some(Backend::HydraSdr));
        assert_ne!(
            SoapyDeviceInfo::native_backend_for("hydrasdr"),
            SoapyDeviceInfo::native_backend_for("airspy"),
            "a fork is not the same radio"
        );
        // A HackRF, on the other hand, has a native backend now, and steering
        // one there matters more than for the receivers: SoapyHackRF loses the
        // receive amp on the first transmit and never applies the transmit one.
        assert_eq!(SoapyDeviceInfo::native_backend_for("hackrf"), Some(Backend::HackRf));
        assert_eq!(SoapyDeviceInfo::native_backend_for("HackRF"), Some(Backend::HackRf));
        assert_eq!(SoapyDeviceInfo::native_backend_for("audio"), None);
    }

    /// A discovered radio and a typed address are two different things, and the
    /// typed one has to win — that is the whole reason both fields exist.
    #[test]
    fn a_typed_pluto_address_beats_a_discovered_one() {
        let mut cfg = PlutoConfig { address: String::new(), ..PlutoConfig::default() };
        assert_eq!(cfg.target(), PlutoConfig::DEFAULT_ADDRESS);
        cfg.selected_ip = Some("10.0.0.9".into());
        assert_eq!(cfg.target(), "10.0.0.9");
        cfg.address = "  pluto.local  ".into();
        assert_eq!(cfg.target(), "pluto.local");
    }

    /// The AGC mode rides `SetGain` as a number, so the encoding has to survive
    /// the round trip or the radio ends up in a mode nobody chose.
    #[test]
    fn agc_modes_survive_the_pseudo_gain_element_encoding() {
        for mode in PlutoAgc::ALL {
            assert_eq!(PlutoAgc::from_code(mode.code()), mode, "{}", mode.label());
        }
        // The IIO spellings are what goes on the wire; a typo here is a mode
        // the device rejects.
        assert_eq!(PlutoAgc::Manual.iio_name(), "manual");
        assert_eq!(PlutoAgc::SlowAttack.iio_name(), "slow_attack");
        assert_eq!(PlutoAgc::FastAttack.iio_name(), "fast_attack");
        assert_eq!(PlutoAgc::Hybrid.iio_name(), "hybrid");
        // Anything unrecognised lands on the safe default rather than manual,
        // which on an unknown band would be a deaf or overloaded receiver.
        assert_eq!(PlutoAgc::from_code(99.0), PlutoAgc::SlowAttack);
    }

    /// The sign is the whole feature. An upconverter moves the hardware *up*
    /// from the dial and a down-converter moves it down, and getting either
    /// backwards points the receiver twice the offset away from the signal.
    #[test]
    fn converter_presets_have_the_right_sign_and_size() {
        for (name, hz) in CONVERTER_PRESETS {
            assert!(hz.abs() <= CONVERTER_OFFSET_MAX_HZ, "{name} is outside the allowed range");
            assert_eq!(converter_preset_name(hz), name, "{name} should name itself");
        }
        // A Ham It Up presents 10.1008 MHz to the receiver as 135.1008 MHz.
        let ham = CONVERTER_PRESETS[1].1;
        assert_eq!(10_100_800.0 + ham, 135_100_800.0);
        // A universal LNB hands a 10.489 GHz downlink over at 739 MHz.
        let lnb = CONVERTER_PRESETS[3].1;
        assert_eq!(10_489_000_000.0 + lnb, 739_000_000.0);
        assert_eq!(converter_preset_name(0.0), "None");
        assert_eq!(converter_preset_name(28_000_000.0), "Manual");
    }

    /// The three shapes a station's transmit line takes behind a receive
    /// converter, in the numbers each one actually produces.
    #[test]
    fn the_transmit_converter_answers_for_the_transmit_line_only() {
        // A universal LNB on receive, the transmitter wired to its own dish
        // feed: the QO-100 station. Receive is offset, transmit is not, so
        // 2400.050 MHz on the dial is 2400.050 MHz out of the radio.
        let lnb = -9_750_000_000.0;
        assert_eq!(ConverterTx::Own(0.0).offset_hz(lnb), Some(0.0));
        // One box both ways — a 23 cm transverter with a 144 MHz IF: the
        // hardware works 1152 MHz below the dial in both directions.
        assert_eq!(ConverterTx::Transverter.offset_hz(-1_152_000_000.0), Some(-1_152_000_000.0));
        // The default withdraws transmit, which is what every `radio.json`
        // written before this existed has to keep meaning.
        assert_eq!(ConverterTx::default(), ConverterTx::Off);
        assert_eq!(ConverterTx::Off.offset_hz(lnb), None);
        let old: RadioConfig =
            serde_json::from_str(r#"{"converter_offset_hz": 125000000.0}"#).expect("parses");
        assert_eq!(old.converter_tx, ConverterTx::Off);
        assert_eq!(old.converter_tx.offset_hz(old.converter_offset_hz), None);
    }

    /// The transmit converter is written to `radio.json` and read back by the
    /// remote client, so its wire form has to survive the round trip — and be
    /// legible to whoever opens the file.
    #[test]
    fn the_transmit_converter_round_trips_through_json() {
        for tx in [ConverterTx::Off, ConverterTx::Transverter, ConverterTx::Own(0.0)] {
            let cfg = RadioConfig { converter_tx: tx, ..RadioConfig::default() };
            let json = serde_json::to_string(&cfg).expect("serializes");
            let back: RadioConfig = serde_json::from_str(&json).expect("parses");
            assert_eq!(back.converter_tx, tx);
        }
        let own = serde_json::to_string(&ConverterTx::Own(-2_256_000_000.0)).expect("serializes");
        assert_eq!(own, r#"{"own":-2256000000.0}"#);
        assert_eq!(serde_json::to_string(&ConverterTx::Off).expect("serializes"), r#""off""#);
    }

    /// The forms an operator will actually type, including one copied straight
    /// back out of the box below it.
    #[test]
    fn tuning_ranges_parse_the_way_they_are_written() {
        let two = parse_freq_ranges("144-146, 430-440").expect("parses");
        assert_eq!(two, vec![(144_000_000.0, 146_000_000.0), (430_000_000.0, 440_000_000.0)]);
        // Spaces, semicolons, en dashes, `..` and a unit are all the same list.
        for text in [
            " 144 - 146 ; 430 .. 440 ",
            "144\u{2013}146\n430-440",
            "144MHz-146MHz, 430 mhz - 440 mhz",
        ] {
            assert_eq!(parse_freq_ranges(text).expect(text), two, "parsing {text:?}");
        }
        // What the field shows is what the field accepts.
        assert_eq!(format_freq_ranges(&two), "144-146, 430-440");
        assert_eq!(parse_freq_ranges(&format_freq_ranges(&two)).unwrap(), two);
        // Down to the hertz, without trailing zeros on the round numbers.
        assert_eq!(format_freq_ranges(&[(10_100_805.0, 10_150_000.0)]), "10.100805-10.15");
        // Blank means "whatever the device says", not an error.
        assert_eq!(parse_freq_ranges("   ").unwrap(), vec![]);
        assert_eq!(format_freq_ranges(&[]), "");
    }

    /// Every rejection has to name what was typed: this is a field where a
    /// silent misreading would either hide bands or open ones the radio can't
    /// reach.
    #[test]
    fn nonsense_tuning_ranges_are_refused() {
        for bad in [
            "430",                   // not a range
            "430-",                  // half a range
            "440-430",               // backwards
            "430-430",               // empty
            "seven-eight",           // not numbers
            "430000000-44000000000", // Hz where megahertz was asked for
        ] {
            assert!(parse_freq_ranges(bad).is_err(), "{bad:?} should be refused");
        }
        // A good range in a bad list fails the whole list rather than being
        // quietly kept: half an entered limit is not a limit.
        assert!(parse_freq_ranges("144-146, oops").is_err());
    }

    /// A `radio.json` from before this setting existed has to keep behaving as
    /// it did, which means no ranges at all — the device's own answer stands.
    #[test]
    fn tuning_range_overrides_default_to_empty() {
        for json in [r#"{}"#, r#"{"backend": "Soapy"}"#, r#"{"converter_offset_hz": 0.0}"#] {
            let cfg: RadioConfig = serde_json::from_str(json).expect("parses");
            assert!(cfg.freq_ranges_rx.is_empty(), "rx ranges after loading {json}");
            assert!(cfg.freq_ranges_tx.is_empty(), "tx ranges after loading {json}");
        }
        let cfg: RadioConfig =
            serde_json::from_str(r#"{"freq_ranges_tx": [[430000000.0, 440000000.0]]}"#)
                .expect("parses");
        assert_eq!(cfg.freq_ranges_tx, vec![(430_000_000.0, 440_000_000.0)]);
    }

    /// Issue #254. The stated ranges describe a device, so moving a tab to
    /// another interface has to leave them behind with the one they were
    /// stated for — and hand them back when the operator comes home.
    #[test]
    fn stated_ranges_travel_with_the_interface_they_were_stated_for() {
        let mut cfg = RadioConfig { backend: Backend::IcomNet, ..RadioConfig::default() };
        cfg.freq_ranges_rx = vec![(144e6, 148e6)];
        cfg.freq_ranges_tx = vec![(144e6, 146e6)];

        cfg.set_backend(Backend::SpyServer);
        assert_eq!(cfg.backend, Backend::SpyServer);
        assert!(cfg.freq_ranges_rx.is_empty(), "the Icom's receive range came along");
        assert!(cfg.freq_ranges_tx.is_empty(), "the Icom's transmit range came along");

        // The receiver states its own, the way "Public SDRs" does.
        cfg.freq_ranges_rx = vec![(200e6, 350e6)];

        cfg.set_backend(Backend::IcomNet);
        assert_eq!(cfg.freq_ranges_rx, vec![(144e6, 148e6)], "the Icom's own, back again");
        assert_eq!(cfg.freq_ranges_tx, vec![(144e6, 146e6)]);

        cfg.set_backend(Backend::SpyServer);
        assert_eq!(cfg.freq_ranges_rx, vec![(200e6, 350e6)], "and the SpyServer's, back again");
        assert!(cfg.freq_ranges_tx.is_empty());
    }

    /// The parked list is keyed by interface, so a tab shuffled around the
    /// picker cannot grow one without bound — and an interface that stated
    /// nothing parks nothing, because an empty pair and a missing entry both
    /// mean "take the device's own answer".
    #[test]
    fn parking_ranges_keeps_one_entry_per_interface_and_none_for_an_empty_one() {
        let mut cfg = RadioConfig { backend: Backend::IcomNet, ..RadioConfig::default() };
        cfg.freq_ranges_rx = vec![(144e6, 148e6)];
        for _ in 0..4 {
            cfg.set_backend(Backend::SpyServer);
            cfg.set_backend(Backend::IcomNet);
        }
        assert_eq!(cfg.freq_ranges_rx, vec![(144e6, 148e6)]);
        assert!(cfg.freq_ranges_parked.is_empty(), "{:?}", cfg.freq_ranges_parked);

        // Setting the interface it is already on is not a move.
        cfg.set_backend(Backend::IcomNet);
        assert_eq!(cfg.freq_ranges_rx, vec![(144e6, 148e6)]);
        assert!(cfg.freq_ranges_parked.is_empty());
    }

    /// A `radio.json` from before the parked list existed loads with an empty
    /// one, which is the truth about it: such a radio has only ever stated
    /// ranges for the interface it is on, and those are still where they were.
    #[test]
    fn a_config_from_before_ranges_were_parked_still_loads() {
        let cfg: RadioConfig = serde_json::from_str(
            r#"{"backend": "IcomNet", "freq_ranges_rx": [[144000000.0, 148000000.0]]}"#,
        )
        .expect("parses");
        assert_eq!(cfg.freq_ranges_rx, vec![(144_000_000.0, 148_000_000.0)]);
        assert!(cfg.freq_ranges_parked.is_empty());
    }

    #[test]
    fn hpsdr_defaults_round_trip() {
        let cfg = HpsdrConfig::default();
        let back: HpsdrConfig =
            serde_json::from_str(&serde_json::to_string(&cfg).unwrap()).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn ppm_scales_frequency_proportionally() {
        assert_eq!(HpsdrConfig::apply_ppm(14_000_000.0, 0.0), 14_000_000.0);
        // +1 ppm at 14 MHz is +14 Hz.
        assert!((HpsdrConfig::apply_ppm(14_000_000.0, 1.0) - 14_000_014.0).abs() < 1e-6);
        assert!((HpsdrConfig::apply_ppm(14_000_000.0, -1.0) - 13_999_986.0).abs() < 1e-6);
    }

    /// The whole point of issue #98's first half: `LNAL` names the same chip
    /// port on both chains, and only the socket says which connector.
    #[test]
    fn a_port_name_resolves_to_the_chains_own_socket() {
        assert_eq!(LimeConfig::rx_socket(0, "LNAL").as_deref(), Some("RX1_L"));
        assert_eq!(LimeConfig::rx_socket(1, "LNAL").as_deref(), Some("RX2_L"));
        assert_eq!(LimeConfig::rx_socket(1, "lnaw").as_deref(), Some("RX2_W"));
        assert_eq!(LimeConfig::tx_socket(1, "BAND2").as_deref(), Some("TX2_2"));
        assert_eq!(LimeConfig::port_label(1, "LNAL", false), "LNAL — RX2_L");
        // A name the board reported that is not one of the three is shown as
        // it came rather than decorated with a guess.
        assert_eq!(LimeConfig::rx_socket(0, "AUTO"), None);
        assert_eq!(LimeConfig::port_label(0, "AUTO", false), "AUTO");
    }

    /// Which boards have a second front end to choose, from the name alone —
    /// the enumeration never opens one.
    #[test]
    fn only_the_two_chain_boards_offer_a_second_chain() {
        let chains = |name: &str| LimeDevice::parse(name).rx_channels();
        assert_eq!(chains("LimeSDR-USB, media=USB 3.0, serial=1234"), 2);
        assert_eq!(chains("LimeSDR-PCIe, media=PCIe"), 2);
        assert_eq!(chains("LimeSDR-Mini_v2, media=USB 3.0"), 1);
        assert_eq!(chains("LimeNET-Micro, media=USB 2.0"), 1);
    }

    /// Which RX-888 panadapter widths the tuner's IF can actually fill.
    ///
    /// The reported case, at the 129.6 MHz clock: 1/32 and 1/16 look right and
    /// everything above them puts the whole of the live spectrum on the right
    /// of the waterfall. That is not a width the receiver failed to deliver —
    /// it is the R828D's 8 MHz IF, parked at 4.57 MHz, in a window too wide to
    /// centre on it, mirrored by the VHF path's high-side injection.
    #[test]
    fn a_vhf_width_has_to_fit_around_the_tuners_if() {
        const CLOCK: f64 = 129_600_000.0;
        let ok = |bins| Rx888Config::width_works_on_vhf(CLOCK, bins);
        assert!(ok(256), "1/32 — 4.05 MHz, centred on the IF with room to spare");
        assert!(ok(512), "1/16 — 8.1 MHz, the widest that still centres");
        assert!(!ok(1024), "1/8 — 16.2 MHz, the width in the report");
        assert!(!ok(2048));
        assert!(!ok(4096));

        // The boundary is twice the IF carrier, whatever the clock: at half
        // this one the same 8.1 MHz is 1/8 rather than 1/16, and still fits.
        assert!(Rx888Config::width_works_on_vhf(64_800_000.0, 1024));
        assert!(!Rx888Config::width_works_on_vhf(64_800_000.0, 2048));

        // And the crossover the warning is keyed on: Nyquist, until that falls
        // below the tuner's own floor.
        assert_eq!(Rx888Config::vhf_crossover_hz(CLOCK), 64_800_000.0);
        assert_eq!(Rx888Config::vhf_crossover_hz(32_400_000.0), 24_000_000.0);
    }
}
