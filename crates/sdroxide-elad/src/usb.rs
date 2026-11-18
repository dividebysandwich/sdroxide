//! Enumeration, opening, and the vendor requests as control transfers.
//!
//! # The one invariant in this crate
//!
//! [`UsbDev`] is deliberately **not `Clone`**, and every control transfer runs
//! on the stream thread and nowhere else. `nusb`'s `Interface` is `Send + Sync`,
//! so a second thread poking the device would compile and would be wrong: the
//! FDM-DUO's CAT gateway is a read-the-busy-flag-then-write pair, and two of
//! those interleaved lose one of the commands with nothing failing. Everything
//! from outside arrives as a [`crate::handle::Ctrl`] message.

use nusb::MaybeFuture;
use nusb::transfer::{ControlIn, ControlOut, ControlType, Recipient};
use sdroxide_types::EladDevice;

use crate::error::{Error, Result};
use crate::protocol::{
    BULK_EP, CONFIGURATION, CTRL_TIMEOUT, INTERFACE, Model, Request, VID, serial_matches,
};
use crate::trace::Trace;

/// Enumerate the ELAD devices on the USB bus.
///
/// Non-invasive: no device is opened. That is also why the entries carry no
/// serial number — ELAD keeps it in the configuration EEPROM rather than in the
/// USB descriptor, so reading it needs the device claimed, which would disturb
/// one that is streaming. The bus address stands in.
pub fn list() -> Vec<EladDevice> {
    let devices = match nusb::list_devices().wait() {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("USB enumeration failed: {e}");
            return Vec::new();
        }
    };
    devices
        .filter(|d| d.vendor_id() == VID)
        .filter_map(|d| {
            let model = Model::from_pid(d.product_id())?;
            Some(EladDevice {
                serial: None,
                name: model.name().to_string(),
                pid: d.product_id(),
                path: bus_path(&d),
            })
        })
        .collect()
}

/// A stable-ish name for where a device is plugged in, for telling two of the
/// same model apart. `bus-port.port…` on the platforms that report a port
/// chain, the bus and address otherwise.
fn bus_path(d: &nusb::DeviceInfo) -> String {
    let ports = d.port_chain();
    if ports.is_empty() {
        format!("bus {}", d.bus_id())
    } else {
        let chain: Vec<String> = ports.iter().map(|p| p.to_string()).collect();
        format!("{}-{}", d.bus_id(), chain.join("."))
    }
}

/// An opened device: the claimed interface plus what we know about it.
///
/// Not `Clone` on purpose — see the module invariant.
pub struct UsbDev {
    iface: nusb::Interface,
    model: Model,
    label: String,
    speed: Option<nusb::Speed>,
    trace: Trace,
}

/// How long a refused open is retried, and how often. See
/// [`UsbDev::claim_patiently`].
const CLAIM_PATIENCE: std::time::Duration = std::time::Duration::from_secs(4);
const CLAIM_RETRY: std::time::Duration = std::time::Duration::from_millis(250);

impl UsbDev {
    /// Open the device with this serial, or the first one found when `serial`
    /// is empty.
    ///
    /// Matching by serial costs an open: the number is in EEPROM, so every
    /// candidate has to be claimed and asked before it can be rejected. With no
    /// serial configured — the ordinary single-device case — the first one is
    /// taken and nothing else is disturbed.
    pub fn open(serial: &str, trace: &Trace) -> Result<UsbDev> {
        let want = serial.trim();
        let devices: Vec<nusb::DeviceInfo> = nusb::list_devices().wait()?.collect();
        let candidates: Vec<nusb::DeviceInfo> = devices
            .iter()
            .filter(|d| d.vendor_id() == VID && Model::from_pid(d.product_id()).is_some())
            .cloned()
            .collect();

        if candidates.is_empty() {
            // Say what *is* there, so a report shows whether the radio is
            // missing, has another product id, or is held by another driver.
            let seen: Vec<String> =
                devices.iter().filter(|d| looks_elad(d)).map(describe).collect();
            trace.note(format!("{} USB devices visible", devices.len()));
            for line in &seen {
                trace.note(format!("ELAD-looking device: {line}"));
            }
            let msg = if seen.is_empty() {
                "no ELAD device found on USB: check the USB lead to the radio's I/Q port \
                 and that the radio is on"
                    .to_string()
            } else {
                format!("no ELAD I/Q device this driver knows on USB; seen: {}", seen.join("; "))
            };
            return Err(Error::NotFound(msg));
        }

        let mut last_err = None;
        for info in &candidates {
            let dev = match Self::claim_patiently(info, trace) {
                Ok(d) => d,
                Err(e) => {
                    trace.note(format!("skipping a device that would not open: {e}"));
                    last_err = Some(e);
                    continue;
                }
            };
            if want.is_empty() {
                return Ok(dev);
            }
            // Only now can the serial be compared: it lives in the EEPROM.
            let have = dev.read_serial();
            if serial_matches(want, have.as_deref()) {
                return Ok(dev);
            }
            trace.note(format!(
                "{} at {} has serial {:?}, not {want:?} — looking further",
                dev.model.name(),
                bus_path(info),
                have.as_deref().unwrap_or("(unreadable)")
            ));
        }

        if want.is_empty() {
            // Every candidate refused to open, so the last refusal is the
            // answer — it is the one with the actionable sentence in it.
            return Err(last_err
                .unwrap_or_else(|| Error::NotFound("no ELAD device found on USB".to_string())));
        }
        Err(Error::NotFound(format!(
            "no ELAD device with serial {want:?} is plugged in — pick another \
             device in Settings → Radio, or clear the serial to use the first \
             one found"
        )))
    }

    /// [`Self::claim`], tried again for a few seconds while it is refused.
    ///
    /// A new rate reopens the device straight after the old stream let go, and
    /// Windows can still be closing the old handle: the open then fails with
    /// error 5, access denied (AJ3G, 25 Oct, on a SPAN change). Only a refusal
    /// is retried; a device that has gone is not.
    fn claim_patiently(info: &nusb::DeviceInfo, trace: &Trace) -> Result<UsbDev> {
        let deadline = std::time::Instant::now() + CLAIM_PATIENCE;
        let mut tries = 0;
        loop {
            match Self::claim(info, trace) {
                Err(Error::Access(e)) if std::time::Instant::now() < deadline => {
                    tries += 1;
                    if tries == 1 {
                        trace.note(format!("open refused, trying again: {e}"));
                    }
                    std::thread::sleep(CLAIM_RETRY);
                }
                r => {
                    if tries > 0 && r.is_ok() {
                        trace.note(format!("opened after {tries} refusals"));
                    }
                    return r;
                }
            }
        }
    }

    /// Open one enumerated device and claim its interface.
    fn claim(info: &nusb::DeviceInfo, trace: &Trace) -> Result<UsbDev> {
        // `from_pid` already succeeded during enumeration; this is the same
        // question asked of the device we are about to hold.
        let model = Model::from_pid(info.product_id())
            .ok_or_else(|| Error::NotFound("not an ELAD device".to_string()))?;
        let label = format!("{} at {}", model.name(), bus_path(info));
        let speed = info.speed();

        let device = info.open().wait().map_err(|e| Error::from_open(e, &label))?;

        // Select the configuration — and on a sampler, select it even when it is
        // already the active one.
        //
        // That last part is not a formality, it is the second half of issue
        // #178. A SET_CONFIGURATION is what makes the Cypress bridge run its own
        // re-initialisation, and on an FDM-S1/S2 that is where EP6 is put into
        // slave FIFO mode — the step the FDM-DUO has an explicit vendor command
        // for (`DuoSub::FifoInit`) and the samplers have none. Both programs
        // that are known to stream from a real FDM-S2 go through libusb, whose
        // `set_configuration` is documented to re-issue the request for the
        // configuration already in force ("a lightweight device reset:
        // altsetting reset to zero, endpoint halts cleared, toggles reset"), so
        // they get it without asking. nusb does exactly what it is told, and
        // skipping it left a correctly programmed FPGA with nowhere to put its
        // samples: sixteen queued transfers, four seconds, not one byte, no
        // error anywhere.
        //
        // The FDM-DUO keeps the old behaviour. It initialises its FIFO with a
        // command of its own and it is the one model anybody has actually
        // streamed from here, so it is left on the open sequence that is known
        // to work.
        let reselect = model != Model::Duo;
        match device.active_configuration() {
            Ok(c) if c.configuration_value() == CONFIGURATION && !reselect => {}
            Ok(c) => {
                if c.configuration_value() != CONFIGURATION {
                    trace.note(format!(
                        "device is on configuration {}, selecting {CONFIGURATION}",
                        c.configuration_value()
                    ));
                }
                // Not fatal: Windows cannot do this at all, and a device that
                // refuses it is still worth trying to stream from.
                match device.set_configuration(CONFIGURATION).wait() {
                    Ok(()) => trace.note(format!(
                        "selected configuration {CONFIGURATION} (re-initialises the bridge)"
                    )),
                    Err(e) => trace.note(format!("set_configuration failed ({e}); continuing")),
                }
            }
            Err(e) => trace.note(format!("no active configuration reported ({e}); continuing")),
        }

        let iface = device
            .detach_and_claim_interface(INTERFACE)
            .wait()
            .map_err(|e| Error::from_open(e, &label))?;

        let dev = UsbDev { iface, model, label, speed, trace: trace.clone() };
        dev.check_bulk_endpoint()?;
        trace.note(format!(
            "claimed interface {INTERFACE} on {} ({})",
            dev.label,
            dev.speed_name()
        ));
        Ok(dev)
    }

    /// Fail early, and name the real layout when we do.
    ///
    /// A bare "endpoint not found" from the first bulk submit says nothing;
    /// listing what the descriptor does have turns a dead end into a usable bug
    /// report — which matters more here than usual, because no one has checked
    /// these descriptors against a real device.
    fn check_bulk_endpoint(&self) -> Result<()> {
        let found: Vec<String> = self
            .iface
            .descriptors()
            .flat_map(|d| d.endpoints().collect::<Vec<_>>())
            .map(|e| {
                format!("0x{:02x} {:?} max {}", e.address(), e.transfer_type(), e.max_packet_size())
            })
            .collect();
        self.trace.note(format!("endpoints: [{}]", found.join(", ")));
        if found.iter().any(|e| e.starts_with(&format!("0x{BULK_EP:02x} "))) {
            return Ok(());
        }
        Err(Error::Descriptor(format!(
            "{} has no bulk IN endpoint 0x{BULK_EP:02x}; it offers [{}]. Please \
             report this with the device's model and firmware version.",
            self.label,
            found.join(", ")
        )))
    }

    /// The serial number out of the configuration EEPROM, if it reads.
    ///
    /// Best-effort by design: it is used for identification and for matching a
    /// pinned serial, and a device whose EEPROM string field will not read is
    /// still a perfectly good receiver.
    pub fn read_serial(&self) -> Option<String> {
        let (addr, len) = crate::protocol::eeprom::SERIAL;
        let bytes = self.eeprom(addr, len).ok()?;
        crate::protocol::eeprom_string(&bytes)
    }

    pub fn model(&self) -> Model {
        self.model
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn speed_name(&self) -> &'static str {
        match self.speed {
            Some(nusb::Speed::Low) => "low speed",
            Some(nusb::Speed::Full) => "full speed",
            Some(nusb::Speed::High) => "high speed",
            Some(nusb::Speed::Super) => "SuperSpeed",
            Some(nusb::Speed::SuperPlus) => "SuperSpeed+",
            _ => "unknown link speed",
        }
    }

    /// Whether the link can carry `rate_hz` at this device's sample width.
    ///
    /// A full-speed link is 12 Mb/s and the lowest rate here needs 12.3 Mb/s of
    /// payload, so an ELAD on a full-speed port cannot stream at all. Worth
    /// saying out loud rather than letting it present as a stuttering receiver.
    pub fn link_too_slow_for(&self, rate_hz: u32) -> Option<String> {
        let need_bps = rate_hz as f64 * crate::protocol::sample_bytes(rate_hz) as f64 * 8.0;
        let have_bps = match self.speed {
            Some(nusb::Speed::Low) => 1.5e6,
            Some(nusb::Speed::Full) => 12.0e6,
            Some(nusb::Speed::High) => 480.0e6,
            // SuperSpeed and above have room for anything this device does.
            _ => return None,
        };
        // Bus overhead and other devices mean the usable share is well under
        // the signalling rate; half is the conventional figure for bulk.
        (need_bps > have_bps * 0.5).then(|| {
            format!(
                "{} is on a {} port, which cannot carry {:.0} kHz ({:.1} Mb/s) — \
                 move it to a USB 2.0 or better port",
                self.label,
                self.speed_name(),
                rate_hz as f64 / 1000.0,
                need_bps / 1e6,
            )
        })
    }

    /// Borrow the interface so the streaming code can open the bulk endpoint.
    pub fn interface(&self) -> &nusb::Interface {
        &self.iface
    }

    pub fn trace(&self) -> &Trace {
        &self.trace
    }

    // ---- vendor requests -------------------------------------------------

    /// A vendor control-IN, with the reply length recorded whether or not it
    /// matched.
    pub fn control_in(
        &self,
        req: Request,
        name: &str,
        value: u16,
        index: u16,
        len: u16,
    ) -> Result<Vec<u8>> {
        self.control_in_traced(req, name, value, index, len, true)
    }

    /// [`Self::control_in`] for a request asked on a timer: a failure is
    /// traced like any other, an answer is not.
    ///
    /// The trace is a bounded ring, and a read four times a second fills it
    /// with identical lines — pushing the open sequence and the EEPROM
    /// calibration, the part of a report a developer reads first, out of it
    /// inside a quarter of an hour of an idle radio.
    pub fn control_in_polled(
        &self,
        req: Request,
        name: &str,
        value: u16,
        index: u16,
        len: u16,
    ) -> Result<Vec<u8>> {
        self.control_in_traced(req, name, value, index, len, false)
    }

    fn control_in_traced(
        &self,
        req: Request,
        name: &str,
        value: u16,
        index: u16,
        len: u16,
        trace_ok: bool,
    ) -> Result<Vec<u8>> {
        let r = self
            .iface
            .control_in(
                ControlIn {
                    control_type: ControlType::Vendor,
                    recipient: Recipient::Device,
                    request: req.code(),
                    value,
                    index,
                    length: len,
                },
                CTRL_TIMEOUT,
            )
            .wait();
        match r {
            Ok(data) if !trace_ok => Ok(data),
            Ok(data) => {
                self.trace.ctrl(
                    req.code(),
                    name,
                    value,
                    index,
                    len as usize,
                    Some(data.len()),
                    "ok",
                );
                Ok(data)
            }
            Err(source) => {
                self.trace.ctrl(
                    req.code(),
                    name,
                    value,
                    index,
                    len as usize,
                    None,
                    &format!("FAILED: {source}"),
                );
                Err(Error::Transfer { op: "control read", source })
            }
        }
    }

    /// A control-IN whose reply must be at least `len` bytes.
    pub fn control_in_exact(
        &self,
        req: Request,
        name: &str,
        value: u16,
        index: u16,
        len: u16,
    ) -> Result<Vec<u8>> {
        let data = self.control_in(req, name, value, index, len)?;
        if data.len() < len as usize {
            return Err(Error::ShortRead {
                request: req.code(),
                want: len as usize,
                got: data.len(),
            });
        }
        Ok(data)
    }

    /// A vendor control-OUT.
    pub fn control_out(
        &self,
        req: Request,
        name: &str,
        value: u16,
        index: u16,
        data: &[u8],
    ) -> Result<()> {
        let r = self
            .iface
            .control_out(
                ControlOut {
                    control_type: ControlType::Vendor,
                    recipient: Recipient::Device,
                    request: req.code(),
                    value,
                    index,
                    data,
                },
                CTRL_TIMEOUT,
            )
            .wait();
        match r {
            Ok(()) => {
                self.trace.ctrl(req.code(), name, value, index, data.len(), None, "ok");
                Ok(())
            }
            Err(source) => {
                self.trace.ctrl(
                    req.code(),
                    name,
                    value,
                    index,
                    data.len(),
                    None,
                    &format!("FAILED: {source}"),
                );
                Err(Error::Transfer { op: "control write", source })
            }
        }
    }

    /// A vendor control-OUT traced only if it fails: twelve thousand of these
    /// make up one FDM-DUO image load, and the trace is a bounded ring.
    pub fn control_out_quiet(
        &self,
        req: Request,
        value: u16,
        index: u16,
        data: &[u8],
    ) -> Result<()> {
        let r = self
            .iface
            .control_out(
                ControlOut {
                    control_type: ControlType::Vendor,
                    recipient: Recipient::Device,
                    request: req.code(),
                    value,
                    index,
                    data,
                },
                CTRL_TIMEOUT,
            )
            .wait();
        r.map_err(|source| {
            self.trace.ctrl(
                req.code(),
                "load data",
                value,
                index,
                data.len(),
                None,
                &format!("FAILED: {source}"),
            );
            Error::Transfer { op: "control write", source }
        })
    }

    /// Read `len` bytes from the configuration EEPROM at `addr`.
    pub fn eeprom(&self, addr: u16, len: u16) -> Result<Vec<u8>> {
        self.control_in_exact(
            Request::Eeprom,
            &format!("eeprom 0x{addr:04X}"),
            addr,
            crate::protocol::EEPROM_INDEX,
            len,
        )
    }
}

/// An ELAD vendor id, or a name that says ELAD or FDM.
fn looks_elad(d: &nusb::DeviceInfo) -> bool {
    let named = |x: Option<&str>| {
        x.is_some_and(|x| {
            let x = x.to_ascii_uppercase();
            x.contains("ELAD") || x.contains("FDM")
        })
    };
    d.vendor_id() == VID || named(d.manufacturer_string()) || named(d.product_string())
}

/// One line about a device, for a bug report.
fn describe(d: &nusb::DeviceInfo) -> String {
    #[cfg(target_os = "windows")]
    let driver = format!(", driver {}", d.driver().unwrap_or("none"));
    #[cfg(not(target_os = "windows"))]
    let driver = String::new();
    format!(
        "{:04x}:{:04x} {} {}{driver}",
        d.vendor_id(),
        d.product_id(),
        d.manufacturer_string().unwrap_or("?"),
        d.product_string().unwrap_or("?"),
    )
}
