//! USB layer: which dongles exist, how to take one, and how to poke its
//! registers.
//!
//! Transcribed from osmocom `librtlsdr` `src/librtlsdr.c` (the `known_devices`
//! table and `rtlsdr_read_array`/`rtlsdr_write_array`).
//!
//! # Invariant: register I/O happens on the streaming thread, and only there
//!
//! [`UsbDev`] is deliberately not `Clone`, and the stream thread owns the only
//! one. nusb's `Interface` *is* `Send + Sync`, so hitting registers from
//! another thread would compile — and would corrupt the tuner. The I2C repeater
//! is a global mode bit on the demodulator that wraps every tuner transaction;
//! a register access from a second thread lands inside someone else's repeater
//! window. All control from outside goes through `Ctrl` messages instead.

use std::time::Duration;

use nusb::MaybeFuture;
use nusb::transfer::{ControlIn, ControlOut, ControlType, Recipient};
use sdroxide_r82xx::Board;
use sdroxide_types::RtlSdrDevice;

use crate::error::{Error, Result};

/// librtlsdr's control-transfer timeout. Generous — a healthy dongle answers in
/// microseconds — but a wedged one must not hang the DSP thread forever.
const CTRL_TIMEOUT: Duration = Duration::from_millis(300);

/// The interface and alternate setting the RTL2832U streams on.
const INTERFACE: u8 = 0;

/// Register blocks, selected by the high byte of the control transfer's
/// `wIndex`. `Tun` reaches the tuner through the demod's I2C repeater; `Iic`
/// reaches the EEPROM on the same bus.
/// Not every block is reachable from this driver — `Tun`, `Rom` and `Ir` have
/// no use in SDR mode, and the demodulator is addressed through
/// [`UsbDev::read_array_raw`] rather than by block. They are listed anyway
/// because the numbering only makes sense as a whole, and a future direct-I2C
/// or IR-remote path would need them.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Block {
    Demod = 0,
    Usb = 1,
    Sys = 2,
    Tun = 3,
    Rom = 4,
    Ir = 5,
    Iic = 6,
}

impl Block {
    /// Short name for error messages.
    pub fn name(self) -> &'static str {
        match self {
            Block::Demod => "demod",
            Block::Usb => "usb",
            Block::Sys => "sys",
            Block::Tun => "tun",
            Block::Rom => "rom",
            Block::Ir => "ir",
            Block::Iic => "iic",
        }
    }
}

/// Every USB id librtlsdr knows about. Transcribed in full: a missing entry
/// means "sdroxide doesn't see my dongle" for whoever owns that stick.
///
/// The first two cover the overwhelming majority in practice, including every
/// RTL-SDR Blog V3 and V4.
pub const KNOWN: &[(u16, u16, &str)] = &[
    (0x0bda, 0x2832, "Generic RTL2832U"),
    (0x0bda, 0x2838, "Generic RTL2832U OEM"),
    (0x0413, 0x6680, "DigitalNow Quad DVB-T PCI-E card"),
    (0x0413, 0x6f0f, "Leadtek WinFast DTV Dongle mini D"),
    (0x0458, 0x707f, "Genius TVGo DVB-T03 USB dongle (Ver. B)"),
    (0x0ccd, 0x00a9, "Terratec Cinergy T Stick Black (rev 1)"),
    (0x0ccd, 0x00b3, "Terratec NOXON DAB/DAB+ USB dongle (rev 1)"),
    (0x0ccd, 0x00b4, "Terratec Deutschlandradio DAB Stick"),
    (0x0ccd, 0x00b5, "Terratec NOXON DAB Stick - Radio Energy"),
    (0x0ccd, 0x00b7, "Terratec Media Broadcast DAB Stick"),
    (0x0ccd, 0x00b8, "Terratec BR DAB Stick"),
    (0x0ccd, 0x00b9, "Terratec WDR DAB Stick"),
    (0x0ccd, 0x00c0, "Terratec MuellerVerlag DAB Stick"),
    (0x0ccd, 0x00c6, "Terratec Fraunhofer DAB Stick"),
    (0x0ccd, 0x00d3, "Terratec Cinergy T Stick RC (Rev.3)"),
    (0x0ccd, 0x00d7, "Terratec T Stick PLUS"),
    (0x0ccd, 0x00e0, "Terratec NOXON DAB/DAB+ USB dongle (rev 2)"),
    (0x1554, 0x5020, "PixelView PV-DT235U(RN)"),
    (0x15f4, 0x0131, "Astrometa DVB-T/DVB-T2"),
    (0x15f4, 0x0133, "HanfTek DAB+FM+DVB-T"),
    (0x185b, 0x0620, "Compro Videomate U620F"),
    (0x185b, 0x0650, "Compro Videomate U650F"),
    (0x185b, 0x0680, "Compro Videomate U680F"),
    (0x1b80, 0xd393, "GIGABYTE GT-U7300"),
    (0x1b80, 0xd394, "DIKOM USB-DVBT HD"),
    (0x1b80, 0xd395, "Peak 102569AGPK"),
    (0x1b80, 0xd397, "KWorld KW-UB450-T USB DVB-T Pico TV"),
    (0x1b80, 0xd398, "Zaapa ZT-MINDVBZP"),
    (0x1b80, 0xd39d, "SVEON STV20 DVB-T USB & FM"),
    (0x1b80, 0xd3a4, "Twintech UT-40"),
    (0x1b80, 0xd3a8, "ASUS U3100MINI_PLUS_V2"),
    (0x1b80, 0xd3af, "SVEON STV27 DVB-T USB & FM"),
    (0x1b80, 0xd3b0, "SVEON STV21 DVB-T USB & FM"),
    (0x1d19, 0x1101, "Dexatek DK DVB-T Dongle (Logilink VG0002A)"),
    (0x1d19, 0x1102, "Dexatek DK DVB-T Dongle (MSI DigiVox mini II V3.0)"),
    (0x1d19, 0x1103, "Dexatek Technology Ltd. DK 5217 DVB-T Dongle"),
    (0x1d19, 0x1104, "MSI DigiVox Micro HD"),
    (0x1f4d, 0xa803, "Sweex DVB-T USB"),
    (0x1f4d, 0xb803, "GTek T803"),
    (0x1f4d, 0xc803, "Lifeview LV5TDeluxe"),
    (0x1f4d, 0xd286, "MyGica TD312"),
    (0x1f4d, 0xd803, "PROlectrix DV107669"),
];

/// The table name for a USB id, if we know it.
fn known_name(vid: u16, pid: u16) -> Option<&'static str> {
    KNOWN.iter().find(|(v, p, _)| *v == vid && *p == pid).map(|(_, _, n)| *n)
}

/// Serial string that unprogrammed dongles ship with. Shared by thousands of
/// sticks, so it identifies nothing and must not be persisted as a selection.
pub const GENERIC_SERIAL: &str = "00000001";

/// The strings a Blog V4 and V4L are programmed with. `librtlsdr`'s own
/// `rtlsdr_check_dongle_model` matches on exactly these pairs, so a dongle that
/// the reference driver calls a V4 or V4L is one here too.
const BLOG_MANUFACTURER: &str = "RTLSDRBlog";
const BLOG_V4_PRODUCT: &str = "Blog V4";
const BLOG_V4L_PRODUCT: &str = "Blog V4L";

/// The Blog board a manufacturer/product pair names, if any. Whole-string
/// matches, as `librtlsdr`'s `strcmp`, so "Blog V4L" is never taken for a V4.
///
/// An absent manufacturer means "unknown", not "not a Blog" — see the note in
/// [`UsbDev::open`] on what Windows does and does not cache.
fn strings_name_board(manufacturer: &str, product: &str) -> Board {
    if !(manufacturer.is_empty() || manufacturer.eq_ignore_ascii_case(BLOG_MANUFACTURER)) {
        return Board::Plain;
    }
    if product.eq_ignore_ascii_case(BLOG_V4_PRODUCT) {
        Board::BlogV4
    } else if product.eq_ignore_ascii_case(BLOG_V4L_PRODUCT) {
        Board::BlogV4L
    } else {
        Board::Plain
    }
}

/// Enumerate the RTL-SDR dongles present on the USB bus.
///
/// Non-invasive: no device is opened. The strings come from sysfs on Linux, the
/// registry on Windows and IOKit on macOS, so this is safe to call at any time
/// — including while another dongle is streaming, which is what makes the
/// settings dialog's Rescan button harmless.
pub fn list() -> Vec<RtlSdrDevice> {
    let devices = match nusb::list_devices().wait() {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("USB enumeration failed: {e}");
            return Vec::new();
        }
    };
    devices
        .filter_map(|d| {
            let (vid, pid) = (d.vendor_id(), d.product_id());
            let table = known_name(vid, pid)?;
            // Prefer what the dongle calls itself — an RTL-SDR Blog V4 reports
            // "Blog V4" here, which is far more use than "Generic RTL2832U OEM".
            let name = match (d.manufacturer_string(), d.product_string()) {
                (Some(m), Some(p)) => format!("{m} {p}"),
                (None, Some(p)) => p.to_string(),
                _ => table.to_string(),
            };
            Some(RtlSdrDevice {
                // Every unprogrammed dongle ships with the same serial, so it
                // identifies nothing; report it as absent so the settings UI
                // does not offer it as something that can be pinned.
                serial: d
                    .serial_number()
                    .filter(|s| !s.is_empty() && *s != GENERIC_SERIAL)
                    .map(str::to_string),
                name,
                vid,
                pid,
            })
        })
        .collect()
}

/// Reset the dongle at the USB level, recovering one that has been left
/// streaming by a process that died without closing it.
///
/// This is a real state to end up in, not a theoretical one: kill any SDR
/// program mid-capture — including this one — and the RTL2832U keeps its
/// endpoint armed. The next open succeeds, every register write is accepted,
/// and not one sample ever arrives. A port reset is the only way back short of
/// physically replugging, so [`UsbDev::open`] does this once before claiming.
///
/// Unsupported on Windows, where it is a no-op rather than an error.
///
/// # Never on macOS
///
/// A reset lowers to `USBDeviceReEnumerate` there, which is not a port reset at
/// all but a *synthetic unplug*: IOKit terminates the `IOUSBDevice` and every
/// interface nub hanging off it, then enumerates the stick again from scratch.
/// The handle we hold — and the interface services we are about to claim
/// through — are dead by the time the call returns, so the claim that follows
/// fails with `kIOReturnNoResources`, which reaches the user as the memorable
/// "failed to open interface (error 0xe00002be)". libusb reports this by
/// failing `libusb_reset_device` with `NOT_FOUND` so the caller knows it must
/// re-open; nusb returns `Ok`, so there is no failure to react to.
///
/// Skipped rather than followed by a re-open, because there is nothing left for
/// it to fix: the armed-endpoint state above is a usbfs quirk — IOKit closes
/// the user client when a process dies, which aborts the pipes — and
/// [`crate::rtl2832::Rtl2832::reset_buffer`] re-arms EPA at every stream start
/// on all platforms anyway. librtlsdr never resets here either.
fn reset_device(device: &nusb::Device, label: &str) {
    if cfg!(target_os = "macos") {
        return;
    }
    match device.reset().wait() {
        Ok(()) => tracing::debug!("reset {label} before claiming it"),
        // Not fatal: the device may well be in a perfectly good state, and the
        // open that follows will say so either way.
        Err(e) => tracing::debug!("could not reset {label} ({e}); continuing"),
    }
}

/// An opened dongle: the claimed interface plus the identity we opened it by.
///
/// Not `Clone` on purpose — see the module invariant.
pub struct UsbDev {
    iface: nusb::Interface,
    /// Human-readable identity, used in log lines and error messages.
    label: String,
    serial: Option<String>,
    board: Board,
    /// Whether the OS gave us a product string at all; without one the
    /// descriptors cannot rule a Blog board out.
    product_known: bool,
}

impl UsbDev {
    /// Open the dongle with this serial, or the first one found when `serial`
    /// is `None`.
    pub fn open(serial: Option<&str>) -> Result<UsbDev> {
        // Matching the shared factory-default serial would pick an arbitrary
        // dongle out of several, so treat it the same way `list` does.
        let want = serial.map(str::trim).filter(|s| !s.is_empty() && *s != GENERIC_SERIAL);
        let devices = nusb::list_devices().wait()?;
        let mut candidates =
            devices.filter(|d| known_name(d.vendor_id(), d.product_id()).is_some());

        let info = match want {
            Some(s) => candidates.find(|d| d.serial_number() == Some(s)).ok_or_else(|| {
                Error::NotFound(format!(
                    "no RTL-SDR with serial {s:?} is plugged in — pick another \
                     dongle in Settings → Radio, or clear the serial to use the \
                     first one found"
                ))
            })?,
            None => candidates
                .next()
                .ok_or_else(|| Error::NotFound("no RTL-SDR dongle found on USB".to_string()))?,
        };

        let table = known_name(info.vendor_id(), info.product_id()).unwrap_or("RTL2832U");
        let manufacturer = info.manufacturer_string().unwrap_or_default().to_string();
        let product = info.product_string().unwrap_or_default().to_string();
        let serial = info
            .serial_number()
            .filter(|s| !s.is_empty() && *s != GENERIC_SERIAL)
            .map(str::to_string);

        // The Blog V4 has no silicon-level marker; the strings are exactly how
        // the blog fork identifies it too. Getting this wrong costs HF: a V4
        // needs the upconverter path, a V3 needs direct sampling.
        //
        // An absent manufacturer must read as "unknown", not as "not a V4":
        // `nusb` reports `None` for it on Windows unconditionally, because
        // Windows caches only the bus-reported *product* description. Requiring
        // both strings therefore made every V4 on Windows fall through to
        // direct sampling, which that board does not implement — the operator
        // gets a spectrum full of noise and no signals at all.
        let board = strings_name_board(&manufacturer, &product);
        let product_known = !product.is_empty();

        let label = if product.is_empty() {
            table.to_string()
        } else if manufacturer.is_empty() {
            product.clone()
        } else {
            format!("{manufacturer} {product}")
        };

        let device = info.open().wait().map_err(|e| Error::from_open(e, &label))?;
        // Clear any state left behind by a program that died mid-stream.
        reset_device(&device, &label);
        // Detaching is the whole answer to `dvb_usb_rtl28xxu` claiming the
        // dongle on Linux; it is a no-op on Windows and macOS. The kernel
        // driver rebinds by itself when the device is unplugged.
        let iface = device
            .detach_and_claim_interface(INTERFACE)
            .wait()
            .map_err(|e| Error::from_open(e, &label))?;

        tracing::info!(
            "opened RTL-SDR {label} (usb {:04x}:{:04x}, serial {})",
            info.vendor_id(),
            info.product_id(),
            serial.as_deref().unwrap_or("none")
        );

        Ok(UsbDev { iface, label, serial, board, product_known })
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn serial(&self) -> Option<&str> {
        self.serial.as_deref()
    }

    /// The Blog board the USB descriptors name: a V4 (R828D) or V4L (R828S,
    /// which reads as an R820T), both with an on-board upconverter.
    ///
    /// Trustworthy when it names one — the descriptors are served straight
    /// out of the EEPROM read by [`UsbDev::reads_board`] — but `Plain` can
    /// mean "the OS did not give us the strings", so that is the fallback.
    pub fn board(&self) -> Board {
        self.board
    }

    /// Whether the OS reported a product string. A V4L can only be missed
    /// when it did not, so only then is the EEPROM worth a read for one.
    pub fn product_known(&self) -> bool {
        self.product_known
    }

    /// Ask the dongle itself whether it is a Blog V4 or V4L, by reading the
    /// strings out of its configuration EEPROM.
    ///
    /// This is how `librtlsdr` identifies both: `rtlsdr_check_dongle_model`
    /// reads the EEPROM and never consults the descriptors at all. It is the
    /// only route that survives an OS which did not cache them — a Windows box
    /// with the dongle bound to WinUSB through Zadig can report it as "Bulk-In,
    /// Interface (Interface 0)", losing the product string as well as the
    /// manufacturer one.
    ///
    /// Reads only as far as the product string. One control transfer per byte
    /// at about 2 ms each, so the whole 256-byte image would cost half a second
    /// of open time to answer a question the first 47 bytes settle.
    ///
    /// The caller must have the I2C repeater closed: the EEPROM sits on the
    /// demodulator's own bus, and an open repeater routes to the tuner instead.
    ///
    /// Any failure reads as `Plain`. Plenty of dongles have no EEPROM at
    /// all, and an unreadable one must not stop the radio from opening.
    pub fn reads_board(&self) -> Board {
        // Header, then the manufacturer and product descriptors: 9 + 22 + 16 =
        // 47 bytes on a V4 (49 on a V4L), with slack for a longer manufacturer
        // string.
        const NEEDED: usize = 64;
        let raw = match self.read_eeprom(0, NEEDED) {
            Ok(raw) => raw,
            Err(e) => {
                tracing::debug!("EEPROM unreadable ({e}); treating as not a Blog board");
                return Board::Plain;
            }
        };
        let Some(eeprom) = parse_eeprom(&raw) else {
            tracing::debug!("no valid EEPROM image; treating as not a Blog board");
            return Board::Plain;
        };
        let board = strings_name_board(&eeprom.manufacturer, &eeprom.product);
        tracing::debug!(
            "EEPROM reports manufacturer {:?}, product {:?} — {}",
            eeprom.manufacturer,
            eeprom.product,
            board.name().unwrap_or("not a Blog board")
        );
        board
    }

    /// Borrow the interface so the streaming code can open the bulk endpoint.
    pub fn interface(&self) -> &nusb::Interface {
        &self.iface
    }

    // ---- raw register access -------------------------------------------

    /// A vendor control-IN with the `wValue`/`wIndex` given verbatim.
    ///
    /// The demodulator block is addressed differently from every other block
    /// (register number in the high byte of `wValue`, page in `wIndex`), so it
    /// bypasses [`UsbDev::read_array`] and comes through here.
    pub fn read_array_raw(&self, value: u16, index: u16, len: u16) -> Result<Vec<u8>> {
        let data = self
            .iface
            .control_in(
                ControlIn {
                    control_type: ControlType::Vendor,
                    recipient: Recipient::Device,
                    request: 0,
                    value,
                    index,
                    length: len,
                },
                CTRL_TIMEOUT,
            )
            .wait()
            .map_err(|source| Error::Transfer { op: "control read", source })?;
        tracing::trace!("ctrl in  value={value:#06x} index={index:#06x} -> {data:02x?}");
        if data.len() < len as usize {
            return Err(Error::ShortRead {
                block: "raw",
                addr: value,
                want: len as usize,
                got: data.len(),
            });
        }
        Ok(data)
    }

    /// A vendor control-OUT with the `wValue`/`wIndex` given verbatim.
    pub fn write_array_raw(&self, value: u16, index: u16, data: &[u8]) -> Result<()> {
        tracing::trace!("ctrl out value={value:#06x} index={index:#06x} <- {data:02x?}");
        self.iface
            .control_out(
                ControlOut {
                    control_type: ControlType::Vendor,
                    recipient: Recipient::Device,
                    request: 0,
                    value,
                    index,
                    data,
                },
                CTRL_TIMEOUT,
            )
            .wait()
            .map_err(|source| Error::Transfer { op: "control write", source })
    }

    pub fn read_array(&self, block: Block, addr: u16, len: u16) -> Result<Vec<u8>> {
        self.read_array_raw(addr, crate::regs::read_index(block), len).map_err(|e| match e {
            // Re-label a short read with the block it came from; "raw" is no
            // help when the message reaches an operator.
            Error::ShortRead { want, got, .. } => {
                Error::ShortRead { block: block.name(), addr, want, got }
            }
            other => other,
        })
    }

    pub fn write_array(&self, block: Block, addr: u16, data: &[u8]) -> Result<()> {
        self.write_array_raw(addr, crate::regs::write_index(block), data)
    }

    /// Read a 1- or 2-byte register. Note the assembly order: 16-bit reads come
    /// back little-endian even though writes go out big-endian — see
    /// [`crate::regs`].
    pub fn read_reg(&self, block: Block, addr: u16, len: u16) -> Result<u16> {
        let d = self.read_array(block, addr, len)?;
        Ok(crate::regs::decode_reg(&d))
    }

    pub fn write_reg(&self, block: Block, addr: u16, val: u16, len: u16) -> Result<()> {
        let buf = crate::regs::encode_reg(val, len);
        self.write_array(block, addr, &buf[..len as usize])
    }

    /// Read-modify-write: only the bits in `mask` are taken from `val`.
    pub fn write_reg_mask(&self, block: Block, addr: u16, val: u8, mask: u8) -> Result<()> {
        let cur = self.read_reg(block, addr, 1)? as u8;
        let new = (cur & !mask) | (val & mask);
        if new == cur {
            return Ok(());
        }
        self.write_reg(block, addr, new as u16, 1)
    }

    // ---- I2C, for the tuner and the EEPROM ------------------------------

    pub fn i2c_write(&self, i2c_addr: u8, data: &[u8]) -> Result<()> {
        self.write_array(Block::Iic, i2c_addr as u16, data)
    }

    pub fn i2c_read(&self, i2c_addr: u8, len: u16) -> Result<Vec<u8>> {
        self.read_array(Block::Iic, i2c_addr as u16, len)
    }

    /// Read `len` bytes from the configuration EEPROM starting at `offset`.
    ///
    /// One byte per transfer: the RTL2832U's I2C block does not do a
    /// sequential multi-byte read here, and asking for one returns the same
    /// byte repeated. Slow, but it runs once at open time at most.
    pub fn read_eeprom(&self, offset: u8, len: usize) -> Result<Vec<u8>> {
        self.i2c_write(EEPROM_ADDR, &[offset])?;
        let mut out = Vec::with_capacity(len);
        for _ in 0..len {
            out.extend_from_slice(&self.i2c_read(EEPROM_ADDR, 1)?);
        }
        Ok(out)
    }
}

/// I2C address of the configuration EEPROM.
pub const EEPROM_ADDR: u8 = 0xa0;

/// The first bytes of a valid RTL2832U EEPROM image.
const EEPROM_MAGIC: [u8; 2] = [0x28, 0x32];

/// What the EEPROM says about a dongle: the USB ids and strings the device
/// reports, held in flash rather than derived from the descriptors.
///
/// Only needed as a fallback — the USB descriptor strings say the same thing
/// and cost nothing to read. It earns its place in `examples/probe.rs`, where
/// being able to cross-check against `rtl_eeprom -d 0` is worth a lot during
/// bring-up.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Eeprom {
    pub vid: u16,
    pub pid: u16,
    pub manufacturer: String,
    pub product: String,
    pub serial: String,
    pub have_serial: bool,
    pub remote_wakeup: bool,
}

/// Parse an EEPROM image (at least 256 bytes).
///
/// Layout: `[0..2]` magic, `[2..4]` VID little-endian, `[4..6]` PID, `[6]`
/// have-serial, `[7]` flags, then three USB string descriptors back to back,
/// each `[len, 0x03, UTF-16LE…]` where `len` counts the whole descriptor.
pub fn parse_eeprom(data: &[u8]) -> Option<Eeprom> {
    if data.len() < 10 || data[0..2] != EEPROM_MAGIC {
        return None;
    }
    let mut e = Eeprom {
        vid: u16::from_le_bytes([data[2], data[3]]),
        pid: u16::from_le_bytes([data[4], data[5]]),
        have_serial: data[6] == 0xa5,
        remote_wakeup: data[7] & 0x01 != 0,
        ..Eeprom::default()
    };

    let mut pos = 0x09;
    for field in [&mut e.manufacturer, &mut e.product, &mut e.serial] {
        let Some(&len) = data.get(pos) else { break };
        // A descriptor is a length byte, the 0x03 string-descriptor tag, then
        // UTF-16LE. Anything else means we have walked off the end of the
        // programmed area into erased flash.
        if len < 4 || data.get(pos + 1) != Some(&0x03) || pos + len as usize > data.len() {
            break;
        }
        let units: Vec<u16> = data[pos + 2..pos + len as usize]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        *field = String::from_utf16_lossy(&units);
        pos += len as usize;
    }
    Some(e)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build an EEPROM image the way a real dongle is programmed, so the
    /// parser is tested against the layout rather than against itself.
    fn image(vid: u16, pid: u16, strings: &[&str]) -> Vec<u8> {
        let mut d = vec![0u8; 9];
        d[0..2].copy_from_slice(&EEPROM_MAGIC);
        d[2..4].copy_from_slice(&vid.to_le_bytes());
        d[4..6].copy_from_slice(&pid.to_le_bytes());
        d[6] = 0xa5;
        for s in strings {
            let utf16: Vec<u8> = s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
            d.push(utf16.len() as u8 + 2);
            d.push(0x03);
            d.extend_from_slice(&utf16);
        }
        d.resize(256, 0);
        d
    }

    #[test]
    fn parses_a_blog_v4_image() {
        let d = image(0x0bda, 0x2838, &["RTLSDRBlog", "Blog V4", "00000001"]);
        let e = parse_eeprom(&d).expect("valid image");
        assert_eq!(e.vid, 0x0bda);
        assert_eq!(e.pid, 0x2838);
        assert_eq!(e.manufacturer, "RTLSDRBlog");
        assert_eq!(e.product, "Blog V4");
        assert_eq!(e.serial, "00000001");
        assert!(e.have_serial);
    }

    #[test]
    fn rejects_a_blank_or_foreign_image() {
        assert!(parse_eeprom(&[0u8; 256]).is_none());
        assert!(parse_eeprom(&[0xff; 256]).is_none());
        assert!(parse_eeprom(&[0x28]).is_none(), "too short to hold the header");
    }

    /// The regression this pair guards: Windows never reports a manufacturer
    /// string, so demanding one identified every V4 there as an ordinary
    /// R828D, sent it down the direct-sampling path it has no wiring for, and
    /// left the operator with a waterfall of pure noise.
    #[test]
    fn identifies_a_v4_without_a_manufacturer_string() {
        for (manufacturer, product, board, why) in [
            ("RTLSDRBlog", "Blog V4", Board::BlogV4, "both strings, as Linux reports"),
            ("", "Blog V4", Board::BlogV4, "product only, as Windows reports"),
            ("RTLSDRBlog", "Blog V4L", Board::BlogV4L, "the V4L, both strings"),
            ("", "Blog V4L", Board::BlogV4L, "the V4L, product only"),
        ] {
            assert_eq!(strings_name_board(manufacturer, product), board, "{why}");
        }
    }

    #[test]
    fn does_not_mistake_other_dongles_for_a_v4() {
        for (manufacturer, product, why) in [
            ("Realtek", "RTL2838UHIDIR", "generic R820T2 stick"),
            ("RTLSDRBlog", "Blog V3", "the V3 direct-samples instead"),
            ("", "", "nothing known either way"),
            ("Nooelec", "Blog V4", "product alone is not enough"),
            ("Nooelec", "Blog V4L", "nor for a V4L"),
            ("RTLSDRBlog", "Blog V4LX", "whole strings only, as librtlsdr's strcmp"),
        ] {
            assert_eq!(strings_name_board(manufacturer, product), Board::Plain, "{why}");
        }
    }

    /// The detection read stops after the product string, so the parser must
    /// return those two fields from a buffer that ends mid-serial.
    #[test]
    fn detects_a_v4_from_a_truncated_read() {
        let mut d = image(0x0bda, 0x2838, &["RTLSDRBlog", "Blog V4", "00000001"]);
        d.truncate(64);
        let e = parse_eeprom(&d).expect("valid image");
        assert_eq!(strings_name_board(&e.manufacturer, &e.product), Board::BlogV4);
    }

    /// The V4L's longer product string still fits the 64-byte detection read,
    /// with the lsusb strings from the report (sdroxide #602).
    #[test]
    fn detects_a_v4l_from_a_truncated_read() {
        let mut d = image(0x0bda, 0x2838, &["RTLSDRBlog", "Blog V4L", "00000001"]);
        d.truncate(64);
        let e = parse_eeprom(&d).expect("valid image");
        assert_eq!(strings_name_board(&e.manufacturer, &e.product), Board::BlogV4L);
    }

    /// Erased flash after the last programmed string must stop the walk rather
    /// than produce mojibake — dongles are shipped with fewer than three
    /// strings surprisingly often.
    #[test]
    fn stops_at_unprogrammed_strings() {
        let d = image(0x0bda, 0x2832, &["Realtek"]);
        let e = parse_eeprom(&d).expect("valid image");
        assert_eq!(e.manufacturer, "Realtek");
        assert_eq!(e.product, "");
        assert_eq!(e.serial, "");
    }
}
