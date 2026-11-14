//! The ELAD / WoodBoxRadio TMate 2 USB controller as a control surface: a
//! main tuning dial, two small dials (E1, E2) with push switches, and six
//! keys, each delivered as a [`Tmate2Event`] the input runtime binds to an
//! [`Action`](sdroxide_types::Action) exactly as it binds a MIDI control.
//!
//! NATIVE ONLY. The rest of the app sees [`Tmate2Handle`] and nothing of the
//! platform HID layer.
//!
//! The worker owns the device. It looks for one once a second while enabled
//! and none is open, so plugging the TMate 2 in after start-up, or pulling it
//! and putting it back, needs nothing from the operator.
//!
//! The LCD shows what [`Tmate2Handle::set_display`] last gave it — frequency,
//! mode, S-meter and status legends — and the panel's USB LED is lit while
//! sdroxide is driving it.

mod hid;
pub mod protocol;

use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, TryRecvError, bounded, unbounded};
use sdroxide_types::Tmate2Control;
use tracing::{debug, info, warn};

pub use protocol::{Display, ModeIcon};
use protocol::{InputReport, REPORT_LEN};

/// How often a missing device is looked for.
const RESCAN: Duration = Duration::from_secs(1);
/// How long one read waits before the worker checks its mailbox. Reports do
/// not wait on this — a read returns the moment one arrives. Short, because
/// the refresh below has to keep its pace while the panel is idle.
const READ_WAIT: Duration = Duration::from_millis(40);
/// How often the output report is re-sent while connected.
///
/// Not optional: the panel treats the report's refresh-time byte (400 ms) as
/// a watchdog and blanks the backlight and USB LED when no report arrives
/// inside it — on hardware, a 2 s refresh made both blink. ELAD's own console
/// and Tmate2_C both write continuously; well inside the window here.
const REFRESH: Duration = Duration::from_millis(150);
/// Depth of the event queue. A stalled UI drops events rather than letting
/// them pile up behind it.
const QUEUE: usize = 256;

/// What the TMate 2 did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tmate2Event {
    /// A dial turned by `delta` counts (positive clockwise). The hardware's
    /// own acceleration is already in the count.
    Dial { control: Tmate2Control, delta: i32 },
    /// A key or dial push went down or came up.
    Key { control: Tmate2Control, down: bool },
    /// A device was opened; the product name.
    Connected(String),
    /// The open device went away. Anything it held must be released.
    Disconnected,
}

/// Connection state, for the settings dialog.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tmate2Status {
    pub connected: bool,
    pub device: String,
    pub error: Option<String>,
}

enum Ctl {
    Enable(bool),
    Display(Display),
    Shutdown,
}

/// The running worker. Dropping it stops the worker and turns the panel's USB
/// LED off.
pub struct Tmate2Handle {
    ctl: Sender<Ctl>,
    events: Receiver<Tmate2Event>,
    status: Arc<Mutex<Tmate2Status>>,
    thread: Option<JoinHandle<()>>,
}

/// Whether this platform can talk to a TMate 2 at all (Linux and Windows).
pub const SUPPORTED: bool = hid::SUPPORTED;

/// Start the worker. `wake` is called from the worker thread whenever events
/// are queued, so the UI can repaint at once instead of on its idle poll.
pub fn spawn(enabled: bool, wake: impl Fn() + Send + 'static) -> Tmate2Handle {
    let (ctl_tx, ctl_rx) = unbounded();
    let (ev_tx, ev_rx) = bounded(QUEUE);
    let status = Arc::new(Mutex::new(Tmate2Status::default()));
    let st = status.clone();
    let thread = std::thread::Builder::new()
        .name("tmate2".into())
        .spawn(move || worker(enabled, ctl_rx, ev_tx, st, wake))
        .ok();
    Tmate2Handle { ctl: ctl_tx, events: ev_rx, status, thread }
}

impl Tmate2Handle {
    /// Everything queued since the last call.
    pub fn poll(&mut self) -> Vec<Tmate2Event> {
        self.events.try_iter().collect()
    }

    /// What the LCD should show. Cheap to call every frame: the caller is
    /// expected to send only on change, and the worker writes it out at once.
    pub fn set_display(&self, d: Display) {
        let _ = self.ctl.send(Ctl::Display(d));
    }

    pub fn set_enabled(&self, on: bool) {
        let _ = self.ctl.send(Ctl::Enable(on));
    }

    pub fn status(&self) -> Tmate2Status {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

impl Drop for Tmate2Handle {
    fn drop(&mut self) {
        let _ = self.ctl.send(Ctl::Shutdown);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

struct Open {
    dev: Box<dyn hid::Device>,
    name: String,
    /// The last report, the baseline the next one is compared with. `None`
    /// until the first arrives: the counters start wherever the device left
    /// them, and the first report must not read as a turn.
    last: Option<InputReport>,
    refreshed: Instant,
}

fn set_status(status: &Mutex<Tmate2Status>, s: Tmate2Status) {
    if let Ok(mut g) = status.lock() {
        *g = s;
    }
}

fn worker(
    mut enabled: bool,
    ctl: Receiver<Ctl>,
    events: Sender<Tmate2Event>,
    status: Arc<Mutex<Tmate2Status>>,
    wake: impl Fn(),
) {
    let mut open: Option<Open> = None;
    let mut last_scan: Option<Instant> = None;
    let mut last_error: Option<String> = None;
    let mut display = Display::default();
    let mut dirty = false;

    let emit = |ev: Tmate2Event| {
        if events.try_send(ev).is_ok() {
            wake();
        }
    };

    loop {
        // ── Mailbox ─────────────────────────────────────────────────────────
        loop {
            match ctl.try_recv() {
                Ok(Ctl::Enable(on)) => enabled = on,
                Ok(Ctl::Display(d)) => {
                    if d != display {
                        display = d;
                        dirty = true;
                    }
                }
                Ok(Ctl::Shutdown) | Err(TryRecvError::Disconnected) => {
                    if let Some(mut o) = open.take() {
                        let _ = o.dev.write(&protocol::closing_output());
                    }
                    return;
                }
                Err(TryRecvError::Empty) => break,
            }
        }

        if !enabled {
            if let Some(mut o) = open.take() {
                let _ = o.dev.write(&protocol::closing_output());
                emit(Tmate2Event::Disconnected);
                set_status(&status, Tmate2Status::default());
            }
            last_error = None;
            std::thread::sleep(READ_WAIT);
            continue;
        }

        // ── Find and open ───────────────────────────────────────────────────
        if open.is_none() {
            if last_scan.is_some_and(|t| t.elapsed() < RESCAN) {
                std::thread::sleep(READ_WAIT);
                continue;
            }
            last_scan = Some(Instant::now());
            match try_open() {
                Ok(Some(o)) => {
                    info!(device = %o.name, "TMate 2 connected");
                    set_status(
                        &status,
                        Tmate2Status { connected: true, device: o.name.clone(), error: None },
                    );
                    emit(Tmate2Event::Connected(o.name.clone()));
                    last_error = None;
                    dirty = true;
                    open = Some(o);
                }
                Ok(None) => {
                    let msg = if hid::SUPPORTED {
                        "No TMate 2 found on USB.".to_string()
                    } else {
                        "TMate 2 is not supported on this platform yet.".to_string()
                    };
                    set_status(&status, Tmate2Status { error: Some(msg), ..Default::default() });
                }
                Err(e) => {
                    let msg = format!("TMate 2 found but could not be opened: {e}");
                    if last_error.as_deref() != Some(msg.as_str()) {
                        warn!("{msg}");
                    }
                    set_status(
                        &status,
                        Tmate2Status { error: Some(msg.clone()), ..Default::default() },
                    );
                    last_error = Some(msg);
                }
            }
            continue;
        }

        // ── Read ────────────────────────────────────────────────────────────
        let Some(o) = open.as_mut() else { continue };
        let mut buf = [0u8; REPORT_LEN];
        match o.dev.read(&mut buf, READ_WAIT) {
            Ok(Some(n)) => {
                if let Some(now) = InputReport::parse(&buf[..n]) {
                    if let Some(before) = o.last {
                        for c in protocol::changes(&before, &now) {
                            emit(match c {
                                protocol::Change::Dial { control, delta } => {
                                    Tmate2Event::Dial { control, delta }
                                }
                                protocol::Change::Key { control, down } => {
                                    Tmate2Event::Key { control, down }
                                }
                            });
                        }
                    } else {
                        debug!(?now, "TMate 2 first report");
                    }
                    o.last = Some(now);
                }
            }
            Ok(None) => {}
            Err(e) => {
                warn!(device = %o.name, "TMate 2 lost: {e}");
                open = None;
                emit(Tmate2Event::Disconnected);
                set_status(
                    &status,
                    Tmate2Status {
                        error: Some(format!("Disconnected: {e}")),
                        ..Default::default()
                    },
                );
                continue;
            }
        }

        if dirty || o.refreshed.elapsed() >= REFRESH {
            dirty = false;
            o.refreshed = Instant::now();
            if let Err(e) = o.dev.write(&protocol::output(&display)) {
                debug!("TMate 2 refresh write failed: {e}");
            }
        }
    }
}

/// Open the first TMate 2 that will open. `Ok(None)` when there is none.
fn try_open() -> std::io::Result<Option<Open>> {
    let found = hid::find();
    let mut last_err = None;
    for f in found {
        match hid::open(&f.key) {
            Ok(mut dev) => {
                // Light the USB LED and set the dial acceleration before the
                // first read. A failure here is not fatal: input still works.
                if let Err(e) = dev.write(&protocol::idle_output()) {
                    warn!(key = %f.key, "TMate 2 initial write failed: {e}");
                }
                return Ok(Some(Open { dev, name: f.name, last: None, refreshed: Instant::now() }));
            }
            Err(e) => last_err = Some(e),
        }
    }
    match last_err {
        Some(e) => Err(e),
        None => Ok(None),
    }
}
