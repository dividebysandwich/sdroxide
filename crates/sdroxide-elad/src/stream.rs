//! The one blocking thread that owns the device.
//!
//! Queued bulk transfers on endpoint `0x86` via nusb's
//! [`nusb::Endpoint::wait_next_complete`], which blocks with a timeout — so this
//! is a plain `std::thread` with no executor, matching every other backend in
//! the workspace. Control arrives over a crossbeam channel and is coalesced
//! (see [`Pending`]) so a panadapter drag cannot starve the sample stream.
//!
//! # Why the control path matters more here
//!
//! On an FDM-DUO a retune is a control transfer *plus* a CAT write, and the CAT
//! write polls a busy flag in front of it — up to two seconds in the worst case.
//! That is far longer than the transfer queue holds at the higher rates, so the
//! queue is topped up before any control work and again immediately after it.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use nusb::MaybeFuture;
use nusb::transfer::{Bulk, In, TransferError};
use rtrb::Producer;
use sdroxide_types::EladConfig;

use crate::convert::Deconstructor;
use crate::device::Device;
use crate::error::{Error, Result};
use crate::handle::{Ctrl, EladHandle, Pending, RxStats, Shared, push_iq, ring_for};
use crate::protocol::{BULK_EP, Model, TRANSFER_BYTES};
use crate::trace::{self, Trace};

/// How long to wait for a transfer before going back to serve control
/// messages. Short enough that a retune feels immediate, long enough that an
/// idle loop costs nothing.
const COMPLETE_TIMEOUT: Duration = Duration::from_millis(5);

/// How often an FDM-DUO is asked where its VFO now is.
///
/// The window rides on that VFO, so a hand on the knob moves the samples and
/// nothing else says so when there is no CAT serial port. Four times a second
/// keeps a spun dial from being visibly behind, and costs one short control
/// transfer between blocks — the same endpoint a retune already uses, and far
/// less traffic than one retune's busy-wait.
const TUNE_POLL: Duration = Duration::from_millis(250);

/// How many times [`TUNE_POLL`] doubles while the read keeps failing: up to
/// 32×, one attempt every eight seconds.
///
/// A control transfer that is refused outright costs nothing, but one that
/// times out holds this thread for a whole second — and this thread is the one
/// keeping the sample queue fed, which at 192 kHz is 128 ms deep. Retried at
/// the full rate, a firmware that never answers would have the stream dropping
/// samples for most of every second, while still delivering enough that the
/// silence watchdog never steps in. Backed off, it costs one second in eight
/// at worst, and an answer puts the rate straight back.
const TUNE_POLL_BACKOFF_MAX: u32 = 5;

/// How long a stream may deliver nothing at all before it is said out loud.
///
/// This is the one failure this backend can produce with no error anywhere in
/// it — an unloaded FPGA answers every command and sends no samples — and until
/// issue #178 it was completely silent: the source's own watchdog reopened the
/// device every three seconds, for ever, with nothing in the log between two
/// cycles. Comfortably inside that three seconds, so the sentence is raised
/// before the handle carrying it is thrown away.
const FIRST_SAMPLE_GRACE: Duration = Duration::from_millis(1500);

/// Transfers kept in flight.
///
/// At the default 192 kHz this is 128 ms of hardware-side buffering; at
/// 6144 kHz it is 4 ms, which is thin — but the top rate is 49 MB/s and a
/// deeper queue there would be reserving megabytes for a mode almost nobody
/// runs. `gr-elad` uses two.
const IN_FLIGHT: usize = 16;

/// What the thread reports back once the device is up.
pub(crate) struct DeviceInfo {
    pub label: String,
    pub model: Model,
    pub serial: Option<String>,
    pub hw_version: Option<(u8, u8)>,
    pub firmware: Option<(u8, u8)>,
    pub sample_rate_hz: f64,
    pub warnings: Vec<String>,
}

/// Open the device and start the stream thread.
///
/// The device is opened *on the thread* so that all control transfers happen on
/// one thread; this function blocks on a handshake until that has succeeded or
/// failed, so the caller still gets a synchronous `Result`.
pub(crate) fn spawn(cfg: &EladConfig, center_hz: f64) -> Result<EladHandle> {
    let (ctrl_tx, ctrl_rx) = crossbeam_channel::unbounded::<Ctrl>();
    // Rendezvous for the open result, so the caller learns about a permission
    // problem or a missing device as a normal error rather than as a stream
    // that silently never starts.
    let (ready_tx, ready_rx) = crossbeam_channel::bounded::<Result<DeviceInfo>>(1);

    let shared = Arc::new(Shared::new());
    let late_warning = Arc::new(std::sync::Mutex::new(None));
    let t = Trace::new();
    trace::remember(&t);

    let (rx_prod, rx_cons) = ring_for(cfg.sample_rate_hz as f64);

    let cfg = cfg.clone();
    let thread_shared = Arc::clone(&shared);
    let thread_trace = t.clone();
    let thread_warning = Arc::clone(&late_warning);
    let join = std::thread::Builder::new()
        .name("sdroxide-elad".into())
        .spawn(move || {
            run(
                cfg,
                center_hz,
                ctrl_rx,
                rx_prod,
                Arc::clone(&thread_shared),
                ready_tx,
                thread_trace,
                thread_warning,
            );
            thread_shared.alive.store(false, Ordering::Relaxed);
        })
        .map_err(|e| Error::Access(format!("could not start the ELAD thread: {e}")))?;

    match ready_rx.recv() {
        Ok(Ok(info)) => {
            Ok(EladHandle::from_parts(rx_cons, ctrl_tx, shared, join, t, late_warning, info))
        }
        Ok(Err(e)) => {
            let _ = join.join();
            Err(e)
        }
        // The thread died before reporting; joining surfaces a panic message.
        Err(_) => {
            let _ = join.join();
            Err(Error::Access("the ELAD thread stopped before it opened the device".into()))
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run(
    cfg: EladConfig,
    center_hz: f64,
    ctrl: crossbeam_channel::Receiver<Ctrl>,
    mut rx: Producer<f32>,
    shared: Arc<Shared>,
    ready: crossbeam_channel::Sender<Result<DeviceInfo>>,
    trace: Trace,
    late_warning: Arc<std::sync::Mutex<Option<String>>>,
) {
    let mut dev = match Device::open(&cfg, center_hz, &trace) {
        Ok(d) => d,
        Err(e) => {
            trace.note(format!("open failed: {e}"));
            let _ = ready.send(Err(e));
            return;
        }
    };

    let _ = ready.send(Ok(DeviceInfo {
        label: dev.describe(),
        model: dev.model(),
        serial: dev.serial.clone(),
        hw_version: dev.hw_version,
        firmware: dev.firmware,
        sample_rate_hz: dev.rate_hz() as f64,
        warnings: dev.warnings.clone(),
    }));

    if let Err(e) = pump(&mut dev, &ctrl, &mut rx, &shared, &trace, &late_warning) {
        tracing::warn!("ELAD stream stopped: {e}");
        trace.note(format!("stream stopped: {e}"));
    }
    dev.shutdown(shared.leaving.load(Ordering::Relaxed));
}

fn pump(
    dev: &mut Device,
    ctrl: &crossbeam_channel::Receiver<Ctrl>,
    rx: &mut Producer<f32>,
    shared: &Arc<Shared>,
    trace: &Trace,
    late_warning: &Arc<std::sync::Mutex<Option<String>>>,
) -> Result<()> {
    let rate = dev.rate_hz();
    tracing::info!(
        "ELAD streaming: {IN_FLIGHT} transfers x {} KiB = {:.0} ms of buffering at {:.0} kSPS",
        TRANSFER_BYTES / 1024,
        (IN_FLIGHT * TRANSFER_BYTES / crate::protocol::sample_bytes(rate)) as f64 / rate as f64
            * 1000.0,
        rate as f64 / 1000.0,
    );

    let mut ep = dev
        .usb()
        .interface()
        .endpoint::<Bulk, In>(BULK_EP)
        .map_err(|e| Error::Access(format!("cannot open the ELAD bulk endpoint: {e}")))?;

    let mut deconstruct = Deconstructor::new(rate);
    deconstruct.set_scale(dev.scale());

    let mut stats = RxStats::new(rate as f64, dev.model());
    let started = Instant::now();
    let mut samples: Vec<f32> = Vec::with_capacity(TRANSFER_BYTES / 2);
    let mut logged_first = false;
    let mut said_silent = false;
    // Far enough back that the first pass asks straight away.
    let mut last_tune_poll = Instant::now() - TUNE_POLL;
    let mut tune_poll_failures = 0u32;
    let mut said_tune_poll_failed = false;
    let mut said_tune_answered = false;

    for _ in 0..IN_FLIGHT {
        ep.submit(ep.allocate(TRANSFER_BYTES));
    }

    // Only now is the device told to produce anything. On a sampler the FIFO
    // start was held back from the open for exactly this: the queue above is
    // what the first block has to land in. See `Device::start_pending`.
    dev.start_pending()?;

    loop {
        // 1. Collapse the whole control channel, then apply each field once.
        let mut pending = Pending::default();
        while let Ok(c) = ctrl.try_recv() {
            pending.absorb(c);
        }
        if pending.shutdown {
            break;
        }
        if !pending.is_empty() {
            // Keep the hardware fed while we talk on endpoint 0 — a DUO retune
            // can block for a good deal longer than the queue holds.
            while ep.pending() < IN_FLIGHT {
                ep.submit(ep.allocate(TRANSFER_BYTES));
            }
            if let Err(e) = apply(dev, &pending) {
                // A setting that cannot be applied is worth saying out loud,
                // but it is not a reason to tear the stream down — the operator
                // can pick another value.
                tracing::warn!("ELAD: {e}");
                trace.note(format!("control change failed: {e}"));
            }
            // The front-end switches move the calibrated scale, so it is
            // re-read rather than tracked field by field.
            deconstruct.set_scale(dev.scale());
        }

        // 1b. Ask the radio where it is, on a timer — when the owner wants it
        //     (`EladHandle::follow_radio_dial`). Only an FDM-DUO answers, and
        //     only its own front panel can make the answer differ from what we
        //     last commanded.
        let tune_poll_every = TUNE_POLL * (1 << tune_poll_failures.min(TUNE_POLL_BACKOFF_MAX));
        // Not on a DUO running a wide image: its read-back then reports VFO B,
        // while the I/Q centre is the one this driver set, and following it
        // walked the centre between the two (AJ3G, 25 Oct 2026).
        if shared.read_dial.load(Ordering::Relaxed)
            && !dev.remote()
            && last_tune_poll.elapsed() >= tune_poll_every
        {
            last_tune_poll = Instant::now();
            match dev.read_tuned() {
                Ok(Some(t)) => {
                    shared.duo_tuned_hz.store(t.hz as u64, Ordering::Relaxed);
                    // The answers themselves are not traced (see
                    // `UsbDev::control_in_polled`), so the report says once that
                    // the read works, and again if it comes back after failing.
                    if !said_tune_answered || tune_poll_failures > 0 {
                        said_tune_answered = true;
                        trace.note(format!(
                            "dial read-back answering: {:.0} Hz, word 0x{:08X}, status 0x{:02x}",
                            t.hz, t.word, t.status
                        ));
                    }
                    tune_poll_failures = 0;
                }
                Ok(None) => {}
                // Not fatal and not worth a line per failure: the samples are
                // still arriving, and the frequency axis simply stays where it
                // was until an answer comes back. Asked less often meanwhile —
                // see `TUNE_POLL_BACKOFF_MAX`.
                Err(e) => {
                    tune_poll_failures = tune_poll_failures.saturating_add(1);
                    if !said_tune_poll_failed {
                        said_tune_poll_failed = true;
                        tracing::debug!("ELAD: reading the radio's dial failed: {e}");
                        trace.note(format!(
                            "dial read-back failed: {e} — asking less often until it answers"
                        ));
                    }
                }
            }
        }

        // 2. Refill before draining, so the queue is never empty while the
        //    device is producing.
        while ep.pending() < IN_FLIGHT {
            ep.submit(ep.allocate(TRANSFER_BYTES));
        }

        // 3. Drain every completion that is ready, not just one. Blocking only
        //    on the first; the rest are polled. Taking one per iteration
        //    starves the stream under a sustained dial drag.
        let mut drained = 0usize;
        loop {
            let timeout = if drained == 0 { COMPLETE_TIMEOUT } else { Duration::ZERO };
            let Some(completion) = ep.wait_next_complete(timeout) else {
                break;
            };
            match completion.status {
                Ok(()) => {
                    let bytes = &completion.buffer[..];
                    samples.clear();
                    deconstruct.push(bytes, &mut samples);

                    if !logged_first && !samples.is_empty() {
                        logged_first = true;
                        // The one line a developer without this hardware cannot
                        // produce: point the receiver at a known carrier and
                        // these bytes settle whether I really does come first.
                        let decoded: Vec<sdroxide_dsp::Complex32> = samples
                            .chunks_exact(2)
                            .take(8)
                            .map(|p| sdroxide_dsp::Complex32::new(p[0], p[1]))
                            .collect();
                        trace.first_samples(bytes, &decoded);
                    }

                    if !samples.is_empty() {
                        stats.on_iq(samples.len() / 2);
                        push_iq(rx, &samples, &mut stats, shared.rx_paused.load(Ordering::Relaxed));
                        shared
                            .last_rx_ms
                            .store(started.elapsed().as_millis() as u64, Ordering::Relaxed);
                    }
                }
                Err(TransferError::Cancelled) => {}
                Err(TransferError::Disconnected) => {
                    return Err(Error::NotFound("the ELAD device was unplugged".into()));
                }
                Err(TransferError::Stall) => {
                    tracing::warn!("ELAD endpoint stalled; clearing and resynchronising");
                    trace.note("bulk endpoint stalled; cleared");
                    let _ = ep.clear_halt().wait();
                    // The stream restarts from a packet boundary, so bytes
                    // carried from before the stall belong to nothing.
                    deconstruct.reset();
                    stats.on_stall();
                }
                Err(e) => {
                    tracing::debug!("ELAD transfer error: {e}");
                    stats.on_error();
                }
            }

            // Reuse the buffer rather than allocating a new one every few ms.
            let mut buf = completion.buffer;
            buf.clear();
            ep.submit(buf);

            drained += 1;
            // Go back and serve control rather than spinning here forever if
            // the device is outrunning us.
            if drained >= IN_FLIGHT {
                break;
            }
        }

        // A stream that has never produced a sample. Not a transfer error, not
        // a stall, not a short read — nothing has come back at all, which is
        // exactly what an unloaded FPGA looks like from here.
        if !logged_first && !said_silent && started.elapsed() >= FIRST_SAMPLE_GRACE {
            said_silent = true;
            // Whatever this process believed it had loaded into the device is
            // not true any more — most likely because it has been unplugged and
            // plugged back in, which empties the FPGA again. Saying so is what
            // makes the next reopen load it rather than trusting the memory.
            crate::fpga::forget();
            let w = crate::fpga::silence_hint(dev.model());
            tracing::warn!("{w}");
            trace.note(&w);
            *late_warning.lock().unwrap_or_else(|e| e.into_inner()) = Some(w);
        }

        // The rate is a guess until the stream has been running long enough to
        // measure. See `RxStats::check_rate`.
        if let Some(mut w) = stats.check_rate(trace) {
            // A DUO read at 192 kHz but streaming at another rate was left on a
            // wide image, most likely by another program. Putting it back on its
            // own image is something this driver can do, once: ask for a reopen,
            // and the reopen restores stand-alone (see `remote::ask_restore`).
            if dev.model() == Model::Duo
                && dev.rate_hz() == crate::remote::STANDALONE_RATE_HZ
                && crate::remote::ask_restore()
            {
                shared.reopen_wanted.store(true, Ordering::Relaxed);
                w = format!("{w} sdroxide is putting the FDM-DUO back in stand-alone at 192 kHz.");
                trace.note("asking for a reopen to restore stand-alone");
            }
            tracing::warn!("{w}");
            *late_warning.lock().unwrap_or_else(|e| e.into_inner()) = Some(w);
        }
        stats.tick(trace);
    }

    trace.note(format!("stream ended: {}", stats.summary()));

    // Cancel outstanding transfers and let them come back before the interface
    // is dropped.
    ep.cancel_all();
    let deadline = Instant::now() + Duration::from_millis(500);
    while ep.pending() > 0 && Instant::now() < deadline {
        if ep.wait_next_complete(Duration::from_millis(50)).is_none() {
            break;
        }
    }
    Ok(())
}

/// Apply coalesced control changes, in dependency order.
///
/// The front-end switches go before the centre on an S2, because its filter
/// code is a function of both and `set_center_hz` re-sends it when the dial
/// crosses a band edge — doing it the other way round would send the code twice
/// for one change. The CAT commands go last: they are the operator's own
/// instructions to the radio and must not be reordered among themselves.
fn apply(dev: &mut Device, p: &Pending) -> Result<()> {
    if let Some(on) = p.attenuator {
        dev.set_attenuator(on)?;
    }
    if let Some(on) = p.preselector {
        dev.set_preselector(on)?;
    }
    if let Some(hz) = p.center {
        dev.set_center_hz(hz)?;
    }
    for cmd in &p.cat {
        dev.cat_write(cmd)?;
    }
    Ok(())
}
