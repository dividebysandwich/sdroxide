//! What this machine has to offer a radio: buses, ports, sound cards, and the
//! addresses that answer from here.
//!
//! One function, because there are two callers and they must not drift apart.
//! The GUI asks it directly (the radio is on the same machine as the dialog),
//! and in server mode the same function is handed to `sdroxide-server` as its
//! [`sdroxide_server::ProbeFn`], so a remote or browser client's Rescan,
//! Discover and Test buttons describe *this* computer rather than the one the
//! operator happens to be sitting at. Every backend is reached from the binary,
//! which is why this lives here and not in a library crate.
//!
//! Everything here blocks — anywhere from a USB enumeration to a five-second
//! connection test — so callers run it off their hot path: the GUI after the
//! settings window closure, the server on its own probe thread.

use sdroxide_types::{DeviceProbe, ProbeAnswer, ProbeTest, ReportKind};

/// Answer one device question about this machine, asked by radio `radio` (its
/// roster id).
///
/// Which radio asked matters to exactly one question — the diagnostic report,
/// which is about *a* radio rather than about this computer. Everything else
/// enumerates a bus, a network or a sound stack, and answers the same whoever
/// is asking.
pub fn probe(req: DeviceProbe, radio: u32) -> ProbeAnswer {
    match req {
        DeviceProbe::Host => ProbeAnswer::Host { soapy: cfg!(feature = "soapy") },
        DeviceProbe::RadioAudio => ProbeAnswer::RadioAudio {
            inputs: sdroxide_audio::input_device_names(),
            outputs: sdroxide_audio::output_device_names(),
        },
        DeviceProbe::SerialPorts => ProbeAnswer::SerialPorts(sdroxide_cat::available_ports()),
        DeviceProbe::RtlSdr => ProbeAnswer::RtlSdr(sdroxide_rtlsdr::list()),
        DeviceProbe::Rx888 => ProbeAnswer::Rx888(sdroxide_rx888::list()),
        DeviceProbe::AirspyHf => ProbeAnswer::AirspyHf(sdroxide_airspyhf::list()),
        DeviceProbe::Airspy => ProbeAnswer::Airspy(sdroxide_airspy::list()),
        DeviceProbe::HackRf => ProbeAnswer::HackRf(sdroxide_hackrf::list()),
        DeviceProbe::HydraSdr => ProbeAnswer::HydraSdr(sdroxide_hydrasdr::list()),
        DeviceProbe::SdrPlay => ProbeAnswer::SdrPlay(sdroxide_sdrplay::list()),
        DeviceProbe::Elad => ProbeAnswer::Elad(sdroxide_elad::list()),
        DeviceProbe::Lime => ProbeAnswer::Lime(sdroxide_lime::list()),
        DeviceProbe::Soapy => ProbeAnswer::Soapy(soapy_devices()),
        DeviceProbe::Hpsdr => ProbeAnswer::Hpsdr(sdroxide_hpsdr::discover_default()),
        DeviceProbe::SmartSdr => ProbeAnswer::SmartSdr(crate::smartsdr_source::discover()),
        DeviceProbe::Pluto => ProbeAnswer::Pluto(sdroxide_pluto::discover_default()),
        // The one probe that asks about the internet rather than about this
        // machine. It still belongs here: this is the computer that will hold
        // the connection, so it is the one whose reachability decides whether a
        // receiver in the list is any use.
        DeviceProbe::PublicSdrs { refresh } => {
            ProbeAnswer::PublicSdrs(Box::new(sdroxide_config::public_sdr_directory(refresh)))
        }
        DeviceProbe::Fobos => ProbeAnswer::Fobos(fobos_devices()),
        DeviceProbe::Test(t) => ProbeAnswer::Test(t.kind(), test(&t)),
        DeviceProbe::Report(k) => ProbeAnswer::Report(k, report(k, radio)),
        DeviceProbe::Relays => ProbeAnswer::Relays(sdroxide_relay::list()),
        DeviceProbe::PublicSdrCheck { network, address } => {
            let result = public_sdr_check(network, &address);
            ProbeAnswer::PublicSdrCheck { network, address, result }
        }
    }
}

/// Whether a receiver from the public-SDR directories answers from here.
///
/// In: its network and endpoint (`host:port`). Out: `Ok` with one line on what
/// answered, `Err` with why nothing did, within [`CHECK_TIMEOUT`].
/// Effect: a KiwiSDR is asked for `/status` over HTTP (no channel is taken), a
/// SpyServer is greeted and hung up on, an sdr-list receiver gets a TCP
/// connect and nothing more, because nothing here speaks its protocol yet.
/// Fails: unresolvable host, refused or timed-out connection, a reply that is
/// not the protocol the directory promised.
// Why: HDZ 2026-11-04: "wenn man jetzt einen Server anklickt, wäre ein basic
//      Check nach 500 ms ein Connection-Test". The same tests the settings'
//      "Test connection" runs, so the map and the dialog agree about a receiver.
fn public_sdr_check(
    network: sdroxide_types::PublicSdrNetwork,
    address: &str,
) -> Result<String, String> {
    use sdroxide_types::PublicSdrNetwork as N;
    match network {
        // `/status` over HTTP: up, users, and whether non-browser apps may in.
        N::KiwiSdr => sdroxide_kiwisdr::test_connection(address, CHECK_TIMEOUT),
        // Hello, device info, hang up; no stream is started.
        N::SpyServer => sdroxide_spyserver::test_connection(address, CHECK_TIMEOUT),
        // Nothing here speaks these protocols yet, so the honest check is the
        // lowest one: does anything accept a connection on that port.
        N::SdrList => {
            use std::net::{TcpStream, ToSocketAddrs};
            // Every address the name resolves to (IPv4 and IPv6), tried in turn;
            // the first that accepts answers.
            let addrs: Vec<_> = address
                .to_socket_addrs()
                .map_err(|e| format!("{address}: {e}"))?
                .collect();
            // The reason reported if none accepts: the last error seen, or
            // that the name resolved to nothing at all.
            let mut last = format!("{address}: no address to connect to");
            for a in addrs {
                match TcpStream::connect_timeout(&a, CHECK_TIMEOUT) {
                    // Connected, and dropped again at once: nothing is sent.
                    Ok(_) => return Ok(format!("{address} accepts connections")),
                    Err(e) => last = format!("{address}: {e}"),
                }
            }
            Err(last)
        }
    }
}

/// How long [`public_sdr_check`] waits, as the settings' tests do: three
/// seconds per connection attempt (and, for a KiwiSDR, for the whole request).
const CHECK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

/// `fobos_rx_list_devices`, translated into the wasm-safe type. `libfobos`'s
/// own `DevInfo` also carries the index `fobos_rx_open` wants, which is not
/// here: `sdroxide-fobos::device::open` re-derives it by matching the serial
/// again, the same lookup-by-serial every other backend's config uses.
fn fobos_devices() -> Vec<sdroxide_types::FobosDevice> {
    sdroxide_fobos::list()
        .into_iter()
        .map(|d| sdroxide_types::FobosDevice { serial: d.serial })
        .collect()
}

/// The whole SoapySDR enumeration, pseudo-drivers included: this feeds a list
/// the operator reads, and a sound card that is being skipped is exactly what
/// they need to see named. The *automatic* pick filters it (see
/// `selectable_soapy_devices` in main.rs).
#[cfg(feature = "soapy")]
fn soapy_devices() -> Vec<sdroxide_types::SoapyDeviceInfo> {
    sdroxide_radio::enumerate_devices("")
        .unwrap_or_else(|e| {
            tracing::warn!("SoapySDR enumeration failed: {e}");
            Vec::new()
        })
        .into_iter()
        .map(|d| sdroxide_types::SoapyDeviceInfo { driver: d.driver, label: d.label, args: d.args })
        .collect()
}

/// Nothing to enumerate in a build without SoapySDR — and the interface is not
/// offered either, so nobody can be looking at this list.
#[cfg(not(feature = "soapy"))]
fn soapy_devices() -> Vec<sdroxide_types::SoapyDeviceInfo> {
    Vec::new()
}

/// Open, ask what is there, and hang up. Every one of these stops short of
/// taking the radio over: the engine keeps running whatever interface it had,
/// and a test that evicted somebody's running session would be a poor thing for
/// a button labelled "Test connection" to do.
fn test(t: &ProbeTest) -> Result<String, String> {
    use std::time::Duration;
    match t {
        ProbeTest::Tci(address) => sdroxide_tci::test_connection(address, Duration::from_secs(3)),
        ProbeTest::SpyServer(address) => {
            sdroxide_spyserver::test_connection(address, Duration::from_secs(3))
        }
        ProbeTest::SmartSdr(address) => {
            sdroxide_smartsdr::test_connection(address, Duration::from_secs(3))
        }
        ProbeTest::Pluto(address) => {
            sdroxide_pluto::test_connection(address, Duration::from_secs(3))
        }
        ProbeTest::Kiwi(address) => {
            sdroxide_kiwisdr::test_connection(address, Duration::from_secs(3))
        }
        ProbeTest::IcomNet(cfg) => {
            sdroxide_icomnet::test_connection(sdroxide_icomnet::IcomNetOptions {
                address: cfg.address.clone(),
                control_port: cfg.control_port,
                username: cfg.username.clone(),
                password: cfg.password.clone(),
                client_name: "sdroxide".into(),
                rx_sample_rate: cfg.sample_rate_hz,
                tx_sample_rate: cfg.sample_rate_hz,
                tx_buffer_ms: cfg.tx_latency_ms,
                civ_address_override: cfg.civ_address_override,
                timeout: Duration::from_secs(5),
            })
        }
    }
}

/// The last session's trace from a backend that has not been verified against
/// hardware, so a fault can be reported without asking anyone to reproduce it
/// under a log filter. Where nothing has run yet, the answer says what to press
/// first rather than coming back empty.
///
/// `radio` is the roster id of the radio whose tab pressed the button, and the
/// networked backends answer about *that* radio: a station can have two Icoms
/// on the LAN, and handing the IC-9700's tab the IC-7300's trace is worse than
/// handing it nothing — it reads as this radio's own conversation. The
/// bus-attached backends still keep one trace apiece; two of a kind on one
/// station is a rarity there, and the trace names the serial it opened.
fn report(kind: ReportKind, radio: u32) -> String {
    let scoped = || sdroxide_config::Store::radio(radio).load_radio_config();
    match kind {
        ReportKind::IcomNet => crate::icomnet_source::diagnostics_or_hint(&scoped().icomnet),
        ReportKind::SmartSdr => crate::smartsdr_source::diagnostics_or_hint(&scoped().smartsdr),
        ReportKind::Pluto => sdroxide_pluto::diagnostics().unwrap_or_else(|| {
            "No PlutoSDR session has run yet — press Test connection or \
             Apply / reconnect first."
                .to_string()
        }),
        ReportKind::AirspyHf => sdroxide_airspyhf::diagnostics().unwrap_or_else(|| {
            "No Airspy HF+ session has run yet — press Apply / reconnect \
             first, or run `cargo run -p sdroxide-airspyhf --example probe`."
                .to_string()
        }),
        ReportKind::Airspy => sdroxide_airspy::diagnostics().unwrap_or_else(|| {
            "No Airspy R2/Mini session has run yet — press Apply / reconnect \
             first, or run `cargo run -p sdroxide-airspy --example probe`."
                .to_string()
        }),
        ReportKind::HackRf => sdroxide_hackrf::diagnostics().unwrap_or_else(|| {
            "No HackRF session has run yet — press Apply / reconnect first.".to_string()
        }),
        ReportKind::HydraSdr => sdroxide_hydrasdr::diagnostics().unwrap_or_else(|| {
            "No HydraSDR RFOne session has run yet — press Apply / reconnect \
             first, or run `cargo run -p sdroxide-hydrasdr --example probe`."
                .to_string()
        }),
        ReportKind::Elad => sdroxide_elad::diagnostics().unwrap_or_else(|| {
            "No ELAD session has run yet — press Apply / reconnect first, or \
             run `cargo run -p sdroxide-elad --example probe`."
                .to_string()
        }),
        ReportKind::Lime => lime_report(),
        ReportKind::Relay => sdroxide_relay::diagnostics().unwrap_or_else(|| {
            "No T/R switch has been opened yet — set one up in Settings \u{2192} T/R switch and \
             press APPLY first."
                .to_string()
        }),
    }
}

/// The LimeSDR's report, and the LimeRFE's beside it.
///
/// Two crates because they are two devices: on the front end's own USB cable
/// nothing it does passes through LimeSuite, so a report built from the radio's
/// trace alone would contain the whole session and not one word about the
/// amplifier — which is the half of the path that decides whether anything
/// reaches the antenna. Either section may be absent; the hint is what an
/// operator who has pressed the button too early needs.
fn lime_report() -> String {
    let mut out = String::new();
    if let Some(radio) = sdroxide_lime::diagnostics() {
        out.push_str(&radio);
    }
    if let Some(rfe) = sdroxide_limerfe::diagnostics() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str("### LimeRFE front end\n");
        out.push_str(&rfe);
    }
    if out.is_empty() {
        return "No LimeSDR session has run yet — press Apply / reconnect first, or \
                run `cargo run -p sdroxide-lime --example probe`."
            .to_string();
    }
    format!("{out}\n{}\n", sdroxide_lime::trace::FIELD_REPORT_HINT)
}
