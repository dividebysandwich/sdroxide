//! The native worker: one thread, which owns the device outright.
//!
//! Nothing is configured but on/off. There is no port to pick — the RC-28 has
//! one USB id and nobody owns two — so the worker opens the first one it finds,
//! and looks again once a second while it has none.

use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, TryRecvError, bounded, unbounded};
use sdroxide_relay::hid::{self, HidDev};
use tracing::{debug, info, warn};

use crate::proto::{self, Decoder, LED_LINK, Report};
use crate::{Rc28Event, Rc28Status};

/// How often to look for a device while there is none.
const RESCAN: Duration = Duration::from_secs(1);
/// How long one read waits for a report. Also how long a control message — a
/// new LED state, a stop — can wait to be noticed, so it is kept short; the
/// device sends every 10 ms while it is moving, so this costs nothing then.
const READ_WAIT: Duration = Duration::from_millis(20);
/// Depth of the event queue. A stalled UI drops knob steps rather than
/// back-pressuring the reader.
const QUEUE: usize = 512;

enum Ctl {
    Enabled(bool),
    /// `None`: leave the LEDs alone. See [`Rc28Handle::release_leds`].
    Leds(Option<u8>),
    Stop,
}

/// The app's view of the RC-28: a worker thread plus two channels.
pub struct Rc28Handle {
    ctl: Sender<Ctl>,
    events: Receiver<Rc28Event>,
    status: Arc<Mutex<Rc28Status>>,
    thread: Option<JoinHandle<()>>,
}

impl Rc28Handle {
    pub fn set_enabled(&self, on: bool) {
        let _ = self.ctl.send(Ctl::Enabled(on));
    }

    /// Light exactly these of TRANSMIT, F-1 and F-2 ([`proto::Key::bit`]).
    /// LINK is the worker's own: lit while the device is open.
    pub fn set_leds(&self, lit: u8) {
        let _ = self.ctl.send(Ctl::Leds(Some(lit & !LED_LINK)));
    }

    /// Stop writing the LEDs, and leave them to whoever sets them next.
    ///
    /// Every radio tab has a worker of its own on the one device, and only the
    /// focused tab's lights are the right ones: a background tab that kept
    /// writing its own would put them out from under it. A worker starts out
    /// released, and [`Self::set_leds`] takes them back — writing at once,
    /// whatever it last wrote, since another tab may have written since.
    pub fn release_leds(&self) {
        let _ = self.ctl.send(Ctl::Leds(None));
    }

    /// Everything that has arrived since the last call. Non-blocking.
    pub fn poll(&mut self) -> Vec<Rc28Event> {
        self.events.try_iter().collect()
    }

    pub fn status(&self) -> Rc28Status {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

impl Drop for Rc28Handle {
    fn drop(&mut self) {
        let _ = self.ctl.send(Ctl::Stop);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Start the worker. `wake` is called whenever an event is queued, so the UI
/// can repaint at once rather than waiting out its idle poll.
pub fn spawn(enabled: bool, wake: impl Fn() + Send + Sync + 'static) -> Rc28Handle {
    let (ctl_tx, ctl_rx) = unbounded();
    let (ev_tx, ev_rx) = bounded(QUEUE);
    let status = Arc::new(Mutex::new(Rc28Status::default()));
    let st = Arc::clone(&status);
    let thread = std::thread::Builder::new()
        .name("sdroxide-rc28".into())
        .spawn(move || {
            Worker {
                enabled,
                ev: ev_tx,
                status: st,
                wake: Box::new(wake),
                dev: None,
                decoder: Decoder::default(),
                leds: None,
                leds_sent: None,
                next_scan: Instant::now(),
            }
            .run(ctl_rx)
        })
        .ok();
    Rc28Handle { ctl: ctl_tx, events: ev_rx, status, thread }
}

struct Worker {
    enabled: bool,
    ev: Sender<Rc28Event>,
    status: Arc<Mutex<Rc28Status>>,
    wake: Box<dyn Fn() + Send + Sync>,
    dev: Option<Box<dyn HidDev>>,
    decoder: Decoder,
    /// TRANSMIT/F-1/F-2 LEDs the app wants lit, or `None` while this worker
    /// is not the one driving them.
    leds: Option<u8>,
    /// What this worker last told the device, LINK included.
    leds_sent: Option<u8>,
    next_scan: Instant,
}

impl Worker {
    fn run(mut self, ctl: Receiver<Ctl>) {
        loop {
            // With a device open the read below is what paces the loop, so
            // control messages are only drained here; without one, waiting on
            // them is the pacing.
            let msg = if self.dev.is_some() {
                match ctl.try_recv() {
                    Ok(m) => Some(m),
                    Err(TryRecvError::Empty) => None,
                    Err(TryRecvError::Disconnected) => Some(Ctl::Stop),
                }
            } else {
                let wait = if self.enabled {
                    self.next_scan.saturating_duration_since(Instant::now())
                } else {
                    RESCAN
                };
                match ctl.recv_timeout(wait) {
                    Ok(m) => Some(m),
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => None,
                    Err(crossbeam_channel::RecvTimeoutError::Disconnected) => Some(Ctl::Stop),
                }
            };
            match msg {
                Some(Ctl::Enabled(on)) => {
                    self.enabled = on;
                    if !on {
                        self.close();
                    }
                    self.next_scan = Instant::now();
                    continue;
                }
                Some(Ctl::Leds(lit)) => {
                    if self.leds.is_none() {
                        // Taking the LEDs back: what this worker last wrote
                        // may have been overwritten by another tab's since.
                        self.leds_sent = None;
                    }
                    self.leds = lit;
                }
                Some(Ctl::Stop) => {
                    self.close();
                    return;
                }
                None => {}
            }

            if self.dev.is_none() {
                if self.enabled && Instant::now() >= self.next_scan {
                    self.next_scan = Instant::now() + RESCAN;
                    self.open();
                }
                continue;
            }
            self.write_leds();
            self.read();
        }
    }

    fn set_status(&self, f: impl FnOnce(&mut Rc28Status)) {
        if let Ok(mut s) = self.status.lock() {
            f(&mut s);
        }
    }

    fn emit(&self, e: Rc28Event) {
        if self.ev.try_send(e).is_ok() {
            (self.wake)();
        }
    }

    fn open(&mut self) {
        let Some(entry) = hid::enumerate(&[(proto::VID, proto::PID)]).into_iter().next() else {
            // Not plugged in is not an error; clear a stale one so the panel
            // does not blame permissions on a device that has gone.
            self.set_status(|s| s.error = None);
            return;
        };
        let mut dev = match hid::open(&entry.key) {
            Ok(d) => d,
            Err(e) => {
                let msg = match e {
                    sdroxide_relay::Error::Permission { .. } => format!(
                        "permission denied opening the RC-28 at {} — install the packaged udev \
                         rule (60-sdroxide-rc28.rules) and replug it",
                        entry.key
                    ),
                    other => format!("cannot open the RC-28 at {}: {other}", entry.key),
                };
                warn!("{msg}");
                self.set_status(|s| s.error = Some(msg));
                return;
            }
        };
        if let Err(e) = dev.write_output(0, &proto::firmware_request()) {
            debug!("RC-28 firmware request: {e}");
        }
        let name = if entry.name.is_empty() { "Icom RC-28".to_string() } else { entry.name };
        info!("RC-28 connected: {name} at {}", entry.key);
        self.dev = Some(dev);
        self.decoder.reset();
        self.leds_sent = None;
        let n = name.clone();
        self.set_status(move |s| {
            s.connected = true;
            s.name = n;
            s.firmware.clear();
            s.error = None;
        });
        self.emit(Rc28Event::Connected(name));
    }

    /// Let go of the device — dark, if this worker was the one lighting it.
    /// Silent if there was none.
    fn close(&mut self) {
        let Some(mut dev) = self.dev.take() else { return };
        if self.leds.is_some() {
            // Best effort: an unplugged device cannot be written, and does not
            // need to be.
            let _ = dev.write_output(0, &proto::led_report(0));
        }
        drop(dev);
        self.lost();
    }

    /// The device is gone, however that happened.
    fn lost(&mut self) {
        self.dev = None;
        self.decoder.reset();
        self.leds_sent = None;
        self.set_status(|s| {
            s.connected = false;
            s.firmware.clear();
        });
        self.emit(Rc28Event::Disconnected);
        self.next_scan = Instant::now() + RESCAN;
    }

    fn write_leds(&mut self) {
        let Some(lit) = self.leds else { return };
        let want = lit | LED_LINK;
        if self.leds_sent == Some(want) {
            return;
        }
        let Some(dev) = self.dev.as_mut() else { return };
        match dev.write_output(0, &proto::led_report(want)) {
            Ok(()) => self.leds_sent = Some(want),
            Err(e) => {
                info!("RC-28 went away: {e}");
                self.lost();
            }
        }
    }

    fn read(&mut self) {
        let Some(dev) = self.dev.as_mut() else { return };
        let mut buf = [0u8; 64];
        match dev.read_input(&mut buf, READ_WAIT) {
            Ok(Some(n)) => match Report::parse(&buf[..n]) {
                Some(Report::Firmware(v)) => {
                    info!("RC-28 firmware {v}");
                    self.set_status(|s| s.firmware = v);
                }
                Some(r) => {
                    for i in self.decoder.feed(&r) {
                        self.emit(Rc28Event::Input(i));
                    }
                }
                None => debug!("RC-28 sent an unknown report {:02x?}", &buf[..n.min(8)]),
            },
            Ok(None) => {}
            Err(e) => {
                info!("RC-28 went away: {e}");
                self.lost();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Enumeration must not panic or hang with no device and no hidraw at all
    /// (CI containers have neither), and a worker must stop promptly.
    #[test]
    fn a_worker_with_no_device_starts_and_stops_cleanly() {
        let h = spawn(true, || {});
        std::thread::sleep(Duration::from_millis(50));
        let started = Instant::now();
        drop(h);
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn a_disabled_worker_reports_nothing() {
        let mut h = spawn(false, || {});
        assert!(!h.status().connected);
        assert!(h.poll().is_empty());
    }
}
