//! Asking a machine what radios it has.
//!
//! Every other setting in `radio.json` *describes* a device — a sample rate, a
//! gain, an address — and so means the same thing wherever it is read. These
//! requests are the opposite: each one asks a question about a particular
//! computer. Which dongles are on its USB bus, which serial ports it has, what
//! a discovery broadcast finds on its network, whether an address answers from
//! where it stands, and which sound cards the rig could be plugged into.
//!
//! They used to be answered by whichever machine the settings dialog was
//! running on, which is right in the shack and wrong everywhere else: a remote
//! client would have offered a laptop's built-in microphone as the transceiver's
//! transmit path and listed a dongle that was on the wrong bus. So the question
//! became a message. A [`DeviceProbe`] is asked *of the machine the radio is
//! attached to* — answered in-process by the native app, and over the session
//! by a remote or browser client — and comes back as a [`ProbeAnswer`].
//!
//! That is what makes an interface change reachable from away: choosing a
//! device is picking one out of a list, and until the list could travel there
//! was nothing to pick from.

use serde::{Deserialize, Serialize};

use crate::{
    AirspyDevice, AirspyHfDevice, EladDevice, FobosDevice, HackRfDevice, HpsdrDevice,
    HydraSdrDevice, IcomNetConfig, LimeDevice, PlutoDevice, PublicSdrDirectory, PublicSdrNetwork,
    RtlSdrDevice,
    Rx888Device, SdrPlayDevice, SmartSdrDevice, SoapyDeviceInfo,
};

/// A question about the machine the radio is attached to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceProbe {
    /// What that machine's build can do at all — currently only whether it has
    /// SoapySDR compiled in, which decides whether that interface is offered.
    /// Asked when the settings dialog opens, because the answer shapes the
    /// interface list before anything is chosen from it.
    Host,
    /// The sound cards there, for the CAT / Audio interface: which one carries
    /// the rig's receive audio into the computer, and which one carries
    /// transmit audio back out. Nothing to do with the operator's own speaker
    /// and microphone, which belong to the screen and are enumerated locally.
    RadioAudio,
    /// Serial ports there, for CAT control.
    SerialPorts,
    /// RTL-SDR dongles on the USB bus. Fast and non-invasive — no device is
    /// opened — so it is safe to ask at any time, including while one is
    /// streaming.
    RtlSdr,
    /// RX-888 receivers on the USB bus, same contract as [`DeviceProbe::RtlSdr`].
    /// Devices still sitting in their boot ROM are included and flagged,
    /// because that is the normal state of one that has just been plugged in.
    Rx888,
    /// Airspy HF+ receivers on the USB bus, same contract as
    /// [`DeviceProbe::RtlSdr`]. Which model each one is cannot be told from the
    /// bus — every HF+ shares a product id — so the entries name the family.
    AirspyHf,
    /// Airspy R2 / Mini receivers on the USB bus, same contract as
    /// [`DeviceProbe::RtlSdr`]. An R2 and a Mini share a product id *and* a
    /// product string, so the entries name neither.
    Airspy,
    /// HackRFs on the USB bus, same contract as [`DeviceProbe::RtlSdr`]. The
    /// board revision needs a control transfer and so is not known here.
    HackRf,
    /// HydraSDR RFOne receivers on the USB bus, same contract as
    /// [`DeviceProbe::RtlSdr`]. A prototype board shares its USB id with the
    /// Airspy R2, so the answer includes only the ones whose descriptors say
    /// HydraSDR — being told to use the other interface is a far better outcome
    /// than a receiver programmed by the wrong driver.
    HydraSdr,
    /// The RSPs the SDRplay API service reports. Brief — the service answers
    /// from its own device table — and safe while one is streaming.
    SdrPlay,
    /// ELAD FDM-DUO / FDM-S1 / FDM-S2 on the USB bus, same contract as
    /// [`DeviceProbe::RtlSdr`]. These carry no USB serial string — ELAD keeps
    /// the serial in EEPROM, which needs the device opened — so the entries are
    /// named by model and bus address.
    Elad,
    /// The LimeSDR boards LimeSuite reports.
    ///
    /// Unlike the USB lists above this one is **not** free: `LMS_GetDeviceList`
    /// opens each candidate to read its identity, so it disturbs a device
    /// another program is holding. Asked on demand only, like
    /// [`DeviceProbe::Soapy`].
    Lime,
    /// The SoapySDR enumeration. Unlike the USB lists this one is not free: it
    /// loads every installed SoapySDR module and asks each to scan, which on a
    /// bundle install means probing several buses. Asked on demand only.
    Soapy,
    /// A LAN scan for OpenHPSDR devices (~1.5 s).
    Hpsdr,
    /// A passive listen for FlexRadio discovery broadcasts (~2.5 s). Radios
    /// announce themselves unprompted, so nothing is sent and nothing on the
    /// network is disturbed.
    SmartSdr,
    /// A scan for PlutoSDRs (~1.5 s): mDNS *and* the USB gadget's default
    /// address, because a Pluto on the end of a USB cable often has no
    /// reachable mDNS responder.
    Pluto,
    /// The public-SDR directories — the KiwiSDR and SpyServer listings —
    /// fetched by the machine the radio is attached to.
    ///
    /// Here rather than on a route of its own because a browser client has no
    /// HTTP client and, if it had, could not read either directory across
    /// origins. This lane already means "ask the machine with the radio on it",
    /// which is also the machine that will hold the connection.
    ///
    /// `refresh` forces the network; without it a list fetched in the last
    /// quarter of an hour is served from disk, which is what makes opening the
    /// window instant.
    PublicSdrs { refresh: bool },
    /// Try an address and report what answered, without starting a stream.
    Test(ProbeTest),
    /// The last session's trace from one of the backends that has not been
    /// verified against hardware, so a fault can be reported without asking
    /// anyone to reproduce it under a log filter.
    Report(ReportKind),
    /// The switching devices on that machine — USB HID relay boards and the
    /// sound cards with usable GPIO pins — for the T/R switch's picker.
    ///
    /// Serial relay boards are not here: they are serial ports, and
    /// [`DeviceProbe::SerialPorts`] already answers for those. Non-invasive,
    /// like the dongle probes: nothing is opened.
    ///
    /// Appended last, for the usual reason.
    Relays,
    /// Fobos SDRs `fobos_rx_list_devices` reports, same contract as
    /// [`DeviceProbe::RtlSdr`] — no device is opened.
    ///
    /// After `Relays` rather than beside the other dongle probes, for the
    /// usual reason: appending is what leaves every surviving discriminant
    /// where a peer already expects it.
    Fobos,
    /// Whether one receiver from the public-SDR directories answers right now:
    /// the map's quick check of a receiver a click picked.
    ///
    /// `address` is the endpoint, `host:port`, and comes back in the answer, so
    /// several checks in flight at once each find their row. Same costs as the
    /// settings' "Test connection" for that network ([`ProbeTest::Kiwi`] reads
    /// `/status` and takes no channel, [`ProbeTest::SpyServer`] says hello and
    /// hangs up); an sdr-list receiver, which nothing here speaks to yet, gets
    /// a TCP connect. Appended last, for the usual reason.
    // Why: HDZ 2026-11-04: "wenn man jetzt einen Server anklickt, wäre ein
    //      basic Check nach 500 ms ein Connection-Test … ein Feld für
    //      online/silent". `Test` answers by kind only, into the one slot the
    //      settings dialog shows, so it cannot carry six receivers at once.
    PublicSdrCheck { network: PublicSdrNetwork, address: String },
}

/// A connection test: open, ask what is there, and hang up again.
///
/// The address is the one *typed in the dialog* rather than the one currently
/// applied — a test that ignored an edit would answer a question nobody asked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProbeTest {
    /// A TCI server, `host:port`.
    Tci(String),
    /// A SpyServer, `host:port`. The reply names the receiver on the far end,
    /// the rates it offers and whether this client would be allowed to tune it.
    SpyServer(String),
    /// An Icom over its LAN protocol. Takes the whole block: the answer depends
    /// on the credentials and the CI-V address as much as on the host.
    IcomNet(Box<IcomNetConfig>),
    /// A FlexRadio, `host:port`. Implementations must stop short of registering
    /// as a GUI client: on a radio without multiFLEX that would evict whatever
    /// client the operator is actually using, which is a poor thing for a "Test
    /// connection" button to do.
    SmartSdr(String),
    /// A PlutoSDR, `host[:port]`. Reads the front-end limits as well as the
    /// identity, so the answer states the tuning range *this* board has — a
    /// stock AD9363 and one unlocked to AD9364 differ by an octave and a half.
    Pluto(String),
    /// A KiwiSDR, `host[:port]`. Answered over HTTP rather than by opening a
    /// session: a receiver has only four or eight channels, and taking one to
    /// find out whether it is worth taking would, on a busy receiver, be the
    /// reason it was full.
    Kiwi(String),
}

/// Which "Test connection" button an answer belongs under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestKind {
    Tci,
    SpyServer,
    IcomNet,
    SmartSdr,
    Pluto,
    Kiwi,
}

impl ProbeTest {
    /// Which button asked. Carried in the answer so a reply that crossed a
    /// network still knows where it belongs.
    pub fn kind(&self) -> TestKind {
        match self {
            ProbeTest::Tci(_) => TestKind::Tci,
            ProbeTest::SpyServer(_) => TestKind::SpyServer,
            ProbeTest::IcomNet(_) => TestKind::IcomNet,
            ProbeTest::SmartSdr(_) => TestKind::SmartSdr,
            ProbeTest::Pluto(_) => TestKind::Pluto,
            ProbeTest::Kiwi(_) => TestKind::Kiwi,
        }
    }
}

/// Which backend's diagnostic trace is being asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportKind {
    IcomNet,
    SmartSdr,
    AirspyHf,
    Airspy,
    HackRf,
    Pluto,
    Elad,
    Lime,
    HydraSdr,
    /// The external T/R switch. Not "unverified against hardware" like the rest
    /// of this list so much as *unverifiable from here*: the relay either
    /// clicked or it did not, and nothing in the program can tell which — so
    /// the trace of what it was told is the whole of the evidence.
    ///
    /// Appended last, for the usual reason.
    Relay,
}

/// What the machine with the radio on it answered.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProbeAnswer {
    Host {
        /// Whether that build can drive SoapySDR devices.
        soapy: bool,
    },
    RadioAudio {
        inputs: Vec<String>,
        outputs: Vec<String>,
    },
    SerialPorts(Vec<String>),
    RtlSdr(Vec<RtlSdrDevice>),
    Rx888(Vec<Rx888Device>),
    AirspyHf(Vec<AirspyHfDevice>),
    Airspy(Vec<AirspyDevice>),
    HackRf(Vec<HackRfDevice>),
    HydraSdr(Vec<HydraSdrDevice>),
    SdrPlay(Vec<SdrPlayDevice>),
    Elad(Vec<EladDevice>),
    Lime(Vec<LimeDevice>),
    Soapy(Vec<SoapyDeviceInfo>),
    Hpsdr(Vec<HpsdrDevice>),
    SmartSdr(Vec<SmartSdrDevice>),
    Pluto(Vec<PlutoDevice>),
    /// Both public-SDR directories, however fresh they turned out to be.
    ///
    /// Boxed because it is by far the largest answer here — around eleven
    /// hundred receivers — and every other variant would otherwise be padded
    /// to its size.
    PublicSdrs(Box<PublicSdrDirectory>),
    /// A connection test, and which button asked for it.
    Test(TestKind, Result<String, String>),
    Report(ReportKind, String),
    /// Nobody here can answer device questions — a server built or started
    /// without them. The controls that ask are greyed out rather than left to
    /// look broken, so this is an answer and not a silence.
    Unsupported,
    /// The switching devices that machine can see. Appended last, for the usual
    /// reason.
    Relays(Vec<crate::RelayDevice>),
    /// The Fobos SDRs that machine can see. After `Relays` for the same
    /// reason [`DeviceProbe::Fobos`] is.
    Fobos(Vec<FobosDevice>),
    /// The answer to [`DeviceProbe::PublicSdrCheck`]: the receiver asked about,
    /// and `Ok` with what answered or `Err` with why nothing did.
    PublicSdrCheck { network: PublicSdrNetwork, address: String, result: Result<String, String> },
}
