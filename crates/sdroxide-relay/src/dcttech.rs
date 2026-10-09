//! The dcttech USB HID relay boards — the "free-driver USB control switch"
//! family, sold under MagiDeal and a dozen other names, 1 to 8 channels.
//!
//! This is the board the feature request named. It is the cheapest way to get a
//! computer-controlled contact closure, it needs no driver on any platform, and
//! it is what a great many stations already have in a drawer.
//!
//! # The shared vendor id
//!
//! Every one of these carries `16c0:05df`, which is a *V-USB hobby id* — a
//! range Objective Development hands out to anyone using their software USB
//! stack. Other people's home-made keyboards, LED controllers and thermometers
//! share it. So the product string is checked as well, and it is the only thing
//! that makes the enumeration safe to run on a stranger's bus.
//!
//! Noyito modules use the separate `5131:2007` id and are also offered without
//! requiring the V-USB family's product string.

use std::time::Duration;

use sdroxide_types::{RelayDevice, RelayLink};

use crate::error::{Error, Result};
use crate::frame::{self, ChannelMask, dcttech};
use crate::hid::{self, HidDev};
use crate::transport::RelayTransport;

pub struct DcttechTransport {
    dev: Box<dyn HidDev>,
    key: String,
    name: String,
    managed: ChannelMask,
    last: Option<ChannelMask>,
}

impl DcttechTransport {
    pub fn open(key: &str, managed: ChannelMask) -> Result<DcttechTransport> {
        if managed & !0xFF != 0 {
            return Err(Error::Config("a dcttech relay board has at most eight contacts".into()));
        }
        let name = hid::enumerate(dcttech::USB_IDS)
            .into_iter()
            .find(|e| e.key == key)
            .map(|e| e.name)
            .unwrap_or_default();
        let dev = hid::open(key)?;
        Ok(DcttechTransport { dev, key: key.to_string(), name, managed, last: None })
    }
}

impl RelayTransport for DcttechTransport {
    fn apply(&mut self, want: ChannelMask) -> Result<()> {
        let want = want & self.managed;
        let changed = match self.last {
            Some(had) => (had ^ want) & self.managed,
            None => self.managed,
        };
        if changed == 0 {
            return Ok(());
        }
        for ch in 1..=8u8 {
            let b = frame::bit(ch);
            if changed & b == 0 {
                continue;
            }
            if let Err(e) = self.dev.set_feature(0, &dcttech::set(ch, want & b != 0)) {
                self.last = None;
                return Err(e);
            }
        }
        self.last = Some(want);
        Ok(())
    }

    fn read_back(&mut self) -> Result<Option<ChannelMask>> {
        let mut body = [0u8; dcttech::REPORT_LEN];
        self.dev.get_feature(0, &mut body)?;
        // Believed only when it looks like an answer.
        //
        // The board puts its five-character serial in the first five bytes and
        // its contacts in byte 7, so a reply whose first five bytes are not
        // printable is one that landed at the wrong offset — which is exactly
        // the failure a platform that keeps the report id in byte 0 would
        // produce, and the one thing in the HID layer no test here can catch.
        // Reporting nothing is correct in that case: a read-back is never
        // load-bearing, and a wrong one would have the driver rewriting
        // contacts that were already right.
        let Some((serial, state)) = dcttech::decode(&body) else { return Ok(None) };
        if serial.len() != 5 || !serial.bytes().all(|b| b.is_ascii_graphic()) {
            tracing::debug!("the relay board's read-back did not look like one: {body:02x?}");
            return Ok(None);
        }
        let actual = ChannelMask::from(state) & self.managed;
        // The board disagrees with what was written, so the cache in `apply`
        // is wrong too; without forgetting it, the driver's rewrite would be
        // skipped as "unchanged".
        if self.last.is_some_and(|had| had != actual) {
            self.last = None;
        }
        Ok(Some(actual))
    }

    fn round_trip(&self) -> Duration {
        // A USB control transfer per contact, on a full-speed device. One
        // millisecond is the bus frame; the V-USB firmware is the rest.
        Duration::from_millis(5)
    }

    fn describe(&self) -> String {
        if self.name.is_empty() {
            format!("USB HID relay board at {}", self.key)
        } else {
            format!("{} at {}", self.name, self.key)
        }
    }
}

/// The relay boards on this machine.
///
/// The V-USB id also requires a relay product string, for the reason the module
/// docs give: it is shared with other hobby devices. Noyito modules are matched
/// by their separate id.
pub fn list() -> Vec<RelayDevice> {
    hid::enumerate(dcttech::USB_IDS).into_iter().filter_map(relay_device).collect()
}

fn relay_device(e: hid::HidEntry) -> Option<RelayDevice> {
    match (e.vendor, e.product) {
        dcttech::USB_ID if e.name.contains(dcttech::PRODUCT_PREFIX) => {}
        dcttech::NOYITO_USB_ID => {}
        _ => return None,
    }
    // Zero means unknown: do not infer a board width from its USB id.
    let channels = [dcttech::PRODUCT_PREFIX, dcttech::NOYITO_PRODUCT_PREFIX]
        .into_iter()
        .find_map(|prefix| {
            e.name.rsplit_once(prefix).and_then(|(_, n)| n.trim().parse::<u8>().ok())
        })
        .unwrap_or(0);
    Some(RelayDevice {
        label: if e.name.is_empty() { e.key.clone() } else { e.name },
        key: e.key,
        link: RelayLink::Hid,
        channels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn entry(id: (u16, u16), name: &str) -> hid::HidEntry {
        hid::HidEntry {
            key: "/dev/hidraw3".into(),
            vendor: id.0,
            product: id.1,
            name: name.into(),
            serial: String::new(),
        }
    }

    #[test]
    fn discovery_offers_both_relay_ids_together() {
        let devices: Vec<_> = [
            entry(dcttech::USB_ID, "www.dcttech.com USBRelay8"),
            entry(dcttech::NOYITO_USB_ID, "HIDRelay2"),
        ]
        .into_iter()
        .filter(|e| dcttech::USB_IDS.contains(&(e.vendor, e.product)))
        .filter_map(relay_device)
        .collect();
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].channels, 8);
        assert_eq!(devices[1].channels, 2);
        assert!(devices.iter().all(|d| d.link == RelayLink::Hid));
    }

    #[test]
    fn the_shared_hobby_id_still_requires_a_relay_product_string() {
        assert!(relay_device(entry(dcttech::USB_ID, "Hobby keyboard")).is_none());
        assert!(relay_device(entry(dcttech::USB_ID, "")).is_none());
        assert!(relay_device(entry((0x046d, 0x406d), "USBRelay8")).is_none());
    }

    #[test]
    fn noyito_discovery_does_not_require_a_product_string_or_guess_a_width() {
        for name in ["", "Noyito HID relay module"] {
            let device = relay_device(entry(dcttech::NOYITO_USB_ID, name)).unwrap();
            assert_eq!(device.key, "/dev/hidraw3");
            assert_eq!(device.channels, 0);
            assert_eq!(device.label, if name.is_empty() { "/dev/hidraw3" } else { name });
        }
        assert_eq!(relay_device(entry(dcttech::NOYITO_USB_ID, "USBRelay8")).unwrap().channels, 8);
    }

    /// A board that records what it was told and answers as the real one does.
    #[derive(Default)]
    struct FakeHid {
        sent: Arc<Mutex<Vec<Vec<u8>>>>,
        answer: Vec<u8>,
    }

    impl HidDev for FakeHid {
        fn set_feature(&mut self, id: u8, body: &[u8]) -> Result<()> {
            assert_eq!(id, 0, "these boards use report id 0");
            self.sent.lock().unwrap().push(body.to_vec());
            Ok(())
        }
        fn get_feature(&mut self, _id: u8, body: &mut [u8]) -> Result<()> {
            let n = body.len().min(self.answer.len());
            body[..n].copy_from_slice(&self.answer[..n]);
            Ok(())
        }
        fn write_output(&mut self, _id: u8, _body: &[u8]) -> Result<()> {
            unreachable!("a relay board is commanded with feature reports")
        }
    }

    fn transport(answer: Vec<u8>) -> (DcttechTransport, Arc<Mutex<Vec<Vec<u8>>>>) {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let dev = FakeHid { sent: Arc::clone(&sent), answer };
        (
            DcttechTransport {
                dev: Box::new(dev),
                key: "fake".into(),
                name: "USBRelay2".into(),
                managed: 0b11,
                last: None,
            },
            sent,
        )
    }

    #[test]
    fn both_contacts_are_written_first_and_then_only_what_changed() {
        let (mut t, sent) = transport(Vec::new());
        t.apply(0).unwrap();
        assert_eq!(
            *sent.lock().unwrap(),
            vec![dcttech::set(1, false).to_vec(), dcttech::set(2, false).to_vec()]
        );
        sent.lock().unwrap().clear();

        t.apply(0b01).unwrap();
        assert_eq!(*sent.lock().unwrap(), vec![dcttech::set(1, true).to_vec()]);
        sent.lock().unwrap().clear();

        t.apply(0b01).unwrap();
        assert!(sent.lock().unwrap().is_empty(), "an unchanged state is not written");
    }

    #[test]
    fn a_read_back_yields_the_contacts_the_board_reports() {
        let (mut t, _) = transport(vec![b'A', b'B', b'C', b'D', b'E', 0, 0, 0b11]);
        assert_eq!(t.read_back().unwrap(), Some(0b11));
    }

    #[test]
    fn a_disagreeing_read_back_makes_the_next_apply_rewrite_the_contacts() {
        let (mut t, sent) = transport(vec![b'A', b'B', b'C', b'D', b'E', 0, 0, 0b00]);
        t.apply(0b01).unwrap();
        sent.lock().unwrap().clear();

        assert_eq!(t.read_back().unwrap(), Some(0b00));
        t.apply(0b01).unwrap();
        assert_eq!(
            *sent.lock().unwrap(),
            vec![dcttech::set(1, true).to_vec(), dcttech::set(2, false).to_vec()],
            "a board that lost a write must be told again"
        );
    }

    #[test]
    fn an_agreeing_read_back_keeps_writes_minimal() {
        let (mut t, sent) = transport(vec![b'A', b'B', b'C', b'D', b'E', 0, 0, 0b01]);
        t.apply(0b01).unwrap();
        sent.lock().unwrap().clear();

        assert_eq!(t.read_back().unwrap(), Some(0b01));
        t.apply(0b01).unwrap();
        assert!(sent.lock().unwrap().is_empty());
    }

    /// The defence against the one byte the HID layer cannot be tested on.
    #[test]
    fn a_read_back_at_the_wrong_offset_is_reported_as_nothing() {
        // What a platform that left the report id in byte 0 would hand back:
        // the serial shifted along by one.
        let (mut t, _) = transport(vec![0, b'A', b'B', b'C', b'D', b'E', 0, 0]);
        assert_eq!(
            t.read_back().unwrap(),
            None,
            "a reply that is not a serial must not be believed as a contact state"
        );
    }
}
