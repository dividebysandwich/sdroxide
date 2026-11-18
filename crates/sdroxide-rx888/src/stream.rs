//! The two threads that own the receiver.
//!
//! # Why two
//!
//! Every other USB backend in the workspace converts samples inline on the
//! thread servicing the endpoint. That works at the RTL-SDR's 4.8 MB/s. This
//! device delivers **129.6 MB/s** at the default rate, and the conversion is not
//! a byte-swap but an 8192-point FFT roughly sixteen thousand times a second. A
//! single thread would stop servicing the endpoint for the duration of each FFT,
//! and the FX3 does not wait.
//!
//! So the USB thread does as little as it possibly can — drain completions,
//! hand the buffer on, resubmit — and a second thread does the arithmetic.
//! Buffers are recycled between the two rather than allocated, because at this
//! rate the allocator is a real cost.

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, TrySendError};
use nusb::MaybeFuture;
use nusb::transfer::{Bulk, In, TransferError};
use rtrb::Producer;
use sdroxide_dsp::{Complex32, WbDdc, WideSpectrum, as_interleaved};

use crate::band::{self, Band};
use crate::convert;
use crate::device::{Device, Settings, Tune};
use crate::error::{Error, Result};
use crate::handle::{Ctrl, Pending, Rx888Handle, RxStats, Shared, WideFrame, push_iq, ring_for};
use crate::protocol::{BULK_EP, BULK_PACKET_HS, BULK_PACKET_SS};

/// Input block size for the downconverter. 8192 gives a 7.9 kHz tuning grid at
/// 64.8 Msps, which is fine enough that the residual mixer never has far to go.
pub const DDC_BLOCK: usize = 8192;
/// Default bins selected, and so the inverse FFT size. 256 of 8192 at
/// 64.8 Msps is a 2.025 Msps output, comfortably inside what the engine's own
/// DDC expects. `Settings::ddc_bins` chooses others — the output, and so the
/// panadapter, is always `adc_rate · bins / 8192`, up to half of `DDC_BLOCK`
/// for a full-Nyquist span.
pub const DEFAULT_DDC_BINS: usize = 256;

/// The bin counts that make sense to offer: a power of two (the inverse FFT
/// runs on every block) from the classic 1/32 up to the full half-spectrum.
pub const DDC_BIN_CHOICES: [usize; 5] = [256, 512, 1024, 2048, 4096];

/// Clamp a configured bin count to something `WbDdc::new` accepts: a power of
/// two, at least a usable channel, at most half the block. A config written by
/// hand (or by an older sdroxide, where the field deserialises as zero) lands
/// on the default rather than a panic.
pub fn sanitize_ddc_bins(bins: usize) -> usize {
    if bins == 0 {
        return DEFAULT_DDC_BINS;
    }
    bins.next_power_of_two().clamp(64, DDC_BLOCK / 2)
}

/// FFT size for the full-band display. A real FFT of 8192 gives 4097 bins over
/// 0–32.4 MHz — 7.9 kHz each, or about two bins per pixel once the engine pools
/// them down to its 2048-bin display width.
pub const WIDE_FFT: usize = 8192;
/// Frames per second for the full-band display.
pub const WIDE_FPS: f64 = 20.0;

/// Bounds on the transfer geometry, so a bad config cannot wedge the stream.
const MIN_TRANSFERS: usize = 4;
const MAX_TRANSFERS: usize = 64;
const MIN_TRANSFER_KIB: usize = 16;
const MAX_TRANSFER_KIB: usize = 1024;

/// How long the USB thread waits for a completion before serving control.
const COMPLETE_TIMEOUT: Duration = Duration::from_millis(5);

/// Least time between two tuner PLL reprograms.
///
/// On VHF a retune is an I2C conversation that sleeps up to 20 ms waiting for
/// lock, and it happens on this thread — the one servicing the bulk endpoint,
/// which holds about 32 ms of samples. Back-to-back retunes are exactly what
/// dragging the panadapter produces, and they would drop most of the stream.
///
/// So the dial keeps moving on screen and in the downconverter, and the tuner
/// follows it at most this often; the last position always wins, so the dial
/// never ends up somewhere stale. Only *hardware* retunes are rate-limited —
/// sliding the downconverter inside the IF costs nothing and is not delayed.
///
/// 100 ms is a starting figure rather than a measured one. Its failure mode is
/// dropped buffers, which the overrun counter below already reports.
const MIN_HW_RETUNE: Duration = Duration::from_millis(100);

/// How many filled buffers may be in flight to the converter thread.
///
/// Small on purpose: this queue is not a buffer, it is a hand-off. If the
/// converter falls behind, the honest outcome is to drop a buffer and say so,
/// not to grow an unbounded backlog that turns into latency.
const HANDOFF_DEPTH: usize = 8;

/// Open the device and start streaming.
///
/// The device is opened *on the USB thread* so all control stays there; this
/// function blocks on a handshake so the caller still gets a synchronous
/// `Result`.
pub fn spawn(settings: &Settings, center_hz: f64) -> Result<Rx888Handle> {
    let (ctrl_tx, ctrl_rx) = crossbeam_channel::unbounded::<Ctrl>();
    let (ready_tx, ready_rx) = crossbeam_channel::bounded::<Result<DeviceInfo>>(1);

    let shared = Arc::new(Shared {
        alive: AtomicBool::new(true),
        last_rx_ms: AtomicU64::new(0),
        out_rate_milli_hz: AtomicU64::new(0),
        vga_tenth_db: AtomicI64::new(i64::MIN),
        att_tenth_db: AtomicI64::new(i64::MIN),
        dropped: AtomicU64::new(0),
        wide: Mutex::new(None),
        rx_paused: AtomicBool::new(false),
    });

    let out_rate =
        settings.adc_rate_hz * sanitize_ddc_bins(settings.ddc_bins) as f64 / DDC_BLOCK as f64;
    let (rx_prod, rx_cons) = ring_for(out_rate);

    let settings = settings.clone();
    let thread_shared = Arc::clone(&shared);
    let join = std::thread::Builder::new()
        .name("sdroxide-rx888".into())
        .spawn(move || {
            run(settings, center_hz, ctrl_rx, rx_prod, Arc::clone(&thread_shared), ready_tx);
            thread_shared.alive.store(false, Ordering::Relaxed);
        })
        .map_err(|e| Error::Access(format!("could not start the RX-888 thread: {e}")))?;

    match ready_rx.recv() {
        Ok(Ok(info)) => Ok(Rx888Handle::from_parts(
            rx_cons,
            ctrl_tx,
            shared,
            join,
            info.label,
            info.serial,
            info.adc_rate_hz,
            info.out_rate_hz,
            info.bin_hz,
            info.warning,
            info.vhf_capable,
        )),
        Ok(Err(e)) => {
            let _ = join.join();
            Err(e)
        }
        Err(_) => {
            let _ = join.join();
            Err(Error::Access("the RX-888 thread stopped before it opened the device".into()))
        }
    }
}

struct DeviceInfo {
    label: String,
    serial: Option<String>,
    adc_rate_hz: f64,
    out_rate_hz: f64,
    bin_hz: f64,
    warning: Option<String>,
    vhf_capable: bool,
}

/// One buffer travelling between the USB thread and the converter.
struct Filled {
    buf: Vec<u8>,
}

fn run(
    settings: Settings,
    center_hz: f64,
    ctrl: Receiver<Ctrl>,
    rx: Producer<f32>,
    shared: Arc<Shared>,
    ready: crossbeam_channel::Sender<Result<DeviceInfo>>,
) {
    let mut dev = match Device::open(&settings, None) {
        Ok(d) => d,
        Err(e) => {
            let _ = ready.send(Err(e));
            return;
        }
    };

    let adc_rate = dev.adc_rate_hz();
    let mut ddc = WbDdc::new(adc_rate, DDC_BLOCK, dev.ddc_bins());
    ddc.set_center_hz(center_hz);
    let out_rate = ddc.out_rate();
    shared.out_rate_milli_hz.store((out_rate * 1000.0) as u64, Ordering::Relaxed);

    let _ = ready.send(Ok(DeviceInfo {
        label: dev.label().to_string(),
        serial: dev.serial().map(str::to_string),
        adc_rate_hz: adc_rate,
        out_rate_hz: out_rate,
        bin_hz: ddc.bin_hz(),
        warning: dev.warning().map(str::to_string),
        vhf_capable: dev.vhf_capable(),
    }));

    tracing::info!(
        "RX-888 streaming: {:.3} Msps real in -> {:.4} Msps complex out ({:.1} MB/s over USB)",
        adc_rate / 1e6,
        out_rate / 1e6,
        adc_rate * 2.0 / 1e6,
    );

    // Hand-off queues: `full` carries data to the converter, `empty` returns
    // the buffers so nothing is allocated in the steady state.
    let (full_tx, full_rx) = crossbeam_channel::bounded::<Filled>(HANDOFF_DEPTH);
    let (empty_tx, empty_rx) = crossbeam_channel::bounded::<Vec<u8>>(HANDOFF_DEPTH + 4);

    // The initial tune has to happen after `Device::open` — which settles the
    // ADC rate, and so the crossover — and before the handle is handed out.
    let initial_tune = dev.set_center_hz(center_hz).unwrap_or_else(|e| {
        tracing::warn!("RX-888: initial tune failed: {e}");
        Tune { band: Band::Hf, lo_dial_hz: 0.0, ddc_center_hz: center_hz, conjugate: false }
    });
    ddc.set_center_hz(initial_tune.ddc_center_hz);

    let wide = WideSpectrum::new(adc_rate, WIDE_FFT, WIDE_FPS);
    let conv_shared = Arc::clone(&shared);
    let randomized = dev.randomized();
    let (ddc_ctrl_tx, ddc_ctrl_rx) = crossbeam_channel::unbounded::<Tune>();
    let converter =
        std::thread::Builder::new().name("sdroxide-rx888-ddc".into()).spawn(move || {
            convert_loop(
                ddc,
                wide,
                randomized,
                adc_rate,
                initial_tune,
                full_rx,
                empty_tx,
                rx,
                conv_shared,
                ddc_ctrl_rx,
            );
        });
    let converter = match converter {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("could not start the RX-888 converter thread: {e}");
            return;
        }
    };

    if let Err(e) = pump(&mut dev, &ctrl, &full_tx, &empty_rx, &ddc_ctrl_tx, &settings) {
        tracing::warn!("RX-888 stream stopped: {e}");
    }

    // Dropping the sender ends the converter's loop.
    drop(full_tx);
    let _ = converter.join();
    dev.shutdown();
}

/// Transfer geometry, clamped and rounded to something the endpoint accepts.
fn geometry(packet: usize) -> (usize, usize) {
    let transfers = 16usize.clamp(MIN_TRANSFERS, MAX_TRANSFERS);
    let kib = 256usize.clamp(MIN_TRANSFER_KIB, MAX_TRANSFER_KIB);
    // A transfer length that is not a whole number of packets is rejected.
    let bytes = (kib * 1024 / packet).max(1) * packet;
    (transfers, bytes)
}

fn pump(
    dev: &mut Device,
    ctrl: &Receiver<Ctrl>,
    full: &Sender<Filled>,
    empty: &Receiver<Vec<u8>>,
    ddc_ctrl: &Sender<Tune>,
    settings: &Settings,
) -> Result<()> {
    let packet = match dev.usb().speed() {
        Some(nusb::Speed::Super | nusb::Speed::SuperPlus) => BULK_PACKET_SS,
        _ => BULK_PACKET_HS,
    };
    let (in_flight, xfer_bytes) = geometry(packet);

    // STARTFX3 before the endpoint exists — see `Device::start`.
    dev.start()?;

    let mut ep = dev
        .usb()
        .interface()
        .endpoint::<Bulk, In>(BULK_EP)
        .map_err(|e| Error::Access(format!("cannot open the RX-888 bulk endpoint: {e}")))?;

    for _ in 0..in_flight {
        ep.submit(ep.allocate(xfer_bytes));
    }

    let mut settings = settings.clone();
    let started = Instant::now();
    let mut overruns = 0u64;
    // Last dial the operator asked for but the tuner has not reached yet.
    let mut want_center: Option<f64> = None;
    let mut next_hw_retune = Instant::now();

    loop {
        // 1. Collapse the control channel and apply each field once.
        let mut pending = Pending::default();
        while let Ok(c) = ctrl.try_recv() {
            pending.absorb(c);
        }
        if pending.shutdown {
            break;
        }
        if !pending.is_empty() {
            while ep.pending() < in_flight {
                ep.submit(ep.allocate(xfer_bytes));
            }
            if let Some(hz) = pending.center {
                want_center = Some(hz);
            }
            apply(dev, &pending, &mut settings);
        }

        // Retune outside `apply`, because unlike every other control this one
        // can block for PLL lock and so has to be paced.
        if let Some(hz) = want_center {
            let hw = dev.hardware_retune_needed(hz);
            if !hw || Instant::now() >= next_hw_retune {
                // Top the queue up first: the retune is about to stop
                // servicing it.
                while ep.pending() < in_flight {
                    ep.submit(ep.allocate(xfer_bytes));
                }
                match dev.set_center_hz(hz) {
                    Ok(t) => {
                        let _ = ddc_ctrl.send(t);
                    }
                    Err(e) => tracing::warn!("RX-888: {e}"),
                }
                want_center = None;
                if hw {
                    next_hw_retune = Instant::now() + MIN_HW_RETUNE;
                }
            }
        }

        // 2. Refill before draining, so the queue is never empty while the
        //    device is producing.
        while ep.pending() < in_flight {
            ep.submit(ep.allocate(xfer_bytes));
        }

        // 3. Drain every ready completion, not just one.
        let mut drained = 0usize;
        loop {
            let timeout = if drained == 0 { COMPLETE_TIMEOUT } else { Duration::ZERO };
            let Some(completion) = ep.wait_next_complete(timeout) else {
                break;
            };
            match completion.status {
                Ok(()) => {
                    let data = &completion.buffer[..];
                    if !data.is_empty() {
                        let mut buf = empty.try_recv().unwrap_or_default();
                        buf.clear();
                        buf.extend_from_slice(data);
                        match full.try_send(Filled { buf }) {
                            Ok(()) => {}
                            Err(TrySendError::Full(_)) => {
                                // The converter is behind. Dropping here is
                                // better than stalling the endpoint.
                                overruns += 1;
                                if overruns.is_power_of_two() {
                                    tracing::warn!(
                                        "RX-888: converter behind, dropped {overruns} buffers"
                                    );
                                }
                            }
                            Err(TrySendError::Disconnected(_)) => {
                                return Ok(());
                            }
                        }
                    }
                }
                Err(TransferError::Cancelled) => {}
                Err(TransferError::Disconnected) => {
                    return Err(Error::NotFound("the RX-888 was unplugged".into()));
                }
                Err(TransferError::Stall) => {
                    tracing::warn!("RX-888 endpoint stalled; clearing and resynchronising");
                    let _ = ep.clear_halt().wait();
                }
                Err(e) => tracing::debug!("RX-888 transfer error: {e}"),
            }

            let mut buf = completion.buffer;
            buf.clear();
            ep.submit(buf);

            drained += 1;
            if drained >= in_flight {
                break;
            }
        }
        let _ = started;
    }

    ep.cancel_all();
    let deadline = Instant::now() + Duration::from_millis(500);
    while ep.pending() > 0 && Instant::now() < deadline {
        if ep.wait_next_complete(Duration::from_millis(50)).is_none() {
            break;
        }
    }
    let _ = dev.stop();
    Ok(())
}

/// Apply everything except the dial, which `pump` paces separately.
fn apply(dev: &mut Device, p: &Pending, settings: &mut Settings) {
    // The range before the gain, so a pass carrying both programs the gain in
    // the range it was asked for.
    if let Some(r) = p.vga_range
        && let Err(e) = dev.set_vga_range(r)
    {
        tracing::warn!("RX-888: {e}");
    }
    if let Some(db) = p.vga {
        settings.vga_db = db;
        if let Err(e) = dev.set_vga_db(db) {
            tracing::warn!("RX-888: {e}");
        }
    }
    if let Some(db) = p.att {
        settings.attenuator_db = db;
        if let Err(e) = dev.set_attenuator_db(db) {
            tracing::warn!("RX-888: {e}");
        }
    }
    if let Some(on) = p.dither {
        settings.dither = on;
        if let Err(e) = dev.set_dither(on) {
            tracing::warn!("RX-888: {e}");
        }
    }
    if let Some(on) = p.bias_tee {
        settings.bias_tee_hf = on;
        if let Err(e) = dev.set_bias_tee(on) {
            tracing::warn!("RX-888: {e}");
        }
    }
    if let Some(on) = p.pga {
        settings.pga = on;
        if let Err(e) = dev.set_pga(on) {
            tracing::warn!("RX-888: {e}");
        }
    }
    if let Some(db) = p.tuner_gain {
        settings.tuner_gain_db = db;
        if let Err(e) = dev.set_tuner_gain_db(db) {
            tracing::warn!("RX-888: {e}");
        }
    }
    if let Some(on) = p.tuner_agc {
        settings.tuner_agc = on;
        if let Err(e) = dev.set_tuner_agc(on) {
            tracing::warn!("RX-888: {e}");
        }
    }
    if let Some(on) = p.bias_tee_vhf {
        settings.bias_tee_vhf = on;
        if let Err(e) = dev.set_bias_tee_vhf(on) {
            tracing::warn!("RX-888: {e}");
        }
    }
}

/// Cut the analyser's frame down to the axis the front end is actually on.
///
/// On HF that is the whole frame unchanged. On VHF the analyser is looking at
/// the tuner's IF, so the display is the slice covering the IF filter, reversed
/// — ascending IF is descending RF — and carrying the RF axis it belongs on.
fn project_wide(frame: &[f32], m: &band::WideMap) -> WideFrame {
    let hi = m.hi_bin.min(frame.len());
    let lo = m.lo_bin.min(hi);
    let mut bins = frame[lo..hi].to_vec();
    if m.reverse {
        bins.reverse();
    }
    WideFrame { bins, center_hz: m.center_hz, span_hz: m.span_hz }
}

/// The arithmetic half: bytes in, complex baseband out.
#[allow(clippy::too_many_arguments)]
fn convert_loop(
    mut ddc: WbDdc,
    mut wide: WideSpectrum,
    randomized: bool,
    adc_rate_hz: f64,
    initial: Tune,
    full: Receiver<Filled>,
    empty: Sender<Vec<u8>>,
    mut rx: Producer<f32>,
    shared: Arc<Shared>,
    retune: Receiver<Tune>,
) {
    let mut stats = RxStats::new(ddc.out_rate());
    let mut conjugate = initial.conjugate;
    let mut wmap = band::wide_map(initial.band, initial.lo_dial_hz, adc_rate_hz, WIDE_FFT / 2 + 1);
    let mut carry: Option<u8> = None;
    let mut real: Vec<f32> = Vec::with_capacity(1 << 20);
    let mut cplx: Vec<Complex32> = Vec::with_capacity(1 << 16);
    let started = Instant::now();

    while let Ok(filled) = full.recv() {
        // Last retune wins. On HF this is free; on VHF the expensive half has
        // already happened on the USB thread and what arrives here is only
        // where to point the downconverter.
        let mut want = None;
        while let Ok(t) = retune.try_recv() {
            want = Some(t);
        }
        if let Some(t) = want {
            ddc.set_center_hz(t.ddc_center_hz);
            conjugate = t.conjugate;
            wmap = band::wide_map(t.band, t.lo_dial_hz, adc_rate_hz, WIDE_FFT / 2 + 1);
        }

        convert::to_f32(&filled.buf, randomized, &mut carry, &mut real);

        // The full-band display analyses about 2 % of these samples, so it sits
        // on the same thread as the downconverter rather than earning one of
        // its own.
        wide.process(&real);
        if let Some((frame, mut slot)) = wide.take().zip(shared.wide.lock().ok()) {
            *slot = Some(project_wide(&frame, &wmap));
        }

        cplx.clear();
        ddc.process(&real, &mut cplx);

        if !cplx.is_empty() {
            // The R828D's LO sits above the wanted signal, so its IF runs
            // backwards and the whole VHF spectrum arrives mirrored. Negating Q
            // is the conjugate, which puts it the right way round — measured on
            // the bench, see `crate::band`. Without it every VHF signal tunes
            // the wrong way and every SSB sideband is the other one.
            //
            // In place, and skipped entirely on HF: the ring carries the same
            // bytes a complex block is already made of, so there is nothing to
            // pack for it — see `sdroxide_dsp::as_interleaved`.
            if conjugate {
                for v in cplx.iter_mut() {
                    v.im = -v.im;
                }
            }
            let dropped = push_iq(
                &mut rx,
                as_interleaved(&cplx),
                &mut stats,
                shared.rx_paused.load(Ordering::Relaxed),
            );
            if dropped > 0 {
                shared.dropped.fetch_add(dropped as u64, Ordering::Relaxed);
            }
            shared.last_rx_ms.store(started.elapsed().as_millis() as u64, Ordering::Relaxed);
        }

        // Give the buffer back. A full return queue just means the USB thread
        // has plenty, so dropping it is harmless.
        let _ = empty.try_send(filled.buf);
        stats.tick();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_lengths_are_whole_packets() {
        for packet in [BULK_PACKET_HS, BULK_PACKET_SS] {
            let (n, bytes) = geometry(packet);
            assert!((MIN_TRANSFERS..=MAX_TRANSFERS).contains(&n));
            assert_eq!(bytes % packet, 0, "a partial packet is rejected outright");
            assert!(bytes >= MIN_TRANSFER_KIB * 1024);
        }
    }

    #[test]
    fn the_ddc_geometry_gives_the_expected_output_rate() {
        // 64.8 Msps real in, 256 of 8192 bins out.
        let out = 64.8e6 * DEFAULT_DDC_BINS as f64 / DDC_BLOCK as f64;
        assert!((out - 2_025_000.0).abs() < 1.0, "{out}");
        // And the widest choice is the whole half-spectrum.
        let full = 64.8e6 * DDC_BIN_CHOICES[4] as f64 / DDC_BLOCK as f64;
        assert!((full - 32_400_000.0).abs() < 1.0, "{full}");
    }

    #[test]
    fn bin_counts_are_sanitised_to_what_the_converter_accepts() {
        assert_eq!(sanitize_ddc_bins(0), DEFAULT_DDC_BINS, "an old config deserialises as zero");
        assert_eq!(sanitize_ddc_bins(256), 256);
        assert_eq!(sanitize_ddc_bins(300), 512, "rounded up to a power of two");
        assert_eq!(sanitize_ddc_bins(1 << 20), DDC_BLOCK / 2, "never a whole block or more");
        for b in DDC_BIN_CHOICES {
            assert_eq!(sanitize_ddc_bins(b), b, "every offered choice passes through untouched");
        }
    }
}
