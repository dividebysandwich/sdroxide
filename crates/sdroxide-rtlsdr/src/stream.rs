//! The one blocking thread that owns the dongle.
//!
//! Queued bulk transfers on endpoint `0x81` via nusb's
//! [`Endpoint::wait_next_complete`], which blocks with a timeout — so this is a
//! plain `std::thread` with no executor, matching every other backend in the
//! workspace. Control arrives over a crossbeam channel and is coalesced (see
//! [`Pending`]) so a panadapter drag cannot starve the sample stream.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use nusb::MaybeFuture;
use nusb::transfer::{Bulk, In, TransferError};
use rtrb::Producer;
use sdroxide_types::RtlSdrConfig;

use crate::device::Device;
use crate::error::{Error, Result};
use crate::handle::{Ctrl, Pending, RtlSdrHandle, RxStats, Shared, push_iq, ring_for};

/// The bulk IN endpoint carrying the sample stream.
const BULK_EP: u8 = 0x81;

/// How long to wait for a transfer before going back to serve control
/// messages. Short enough that a retune feels immediate, long enough that an
/// idle loop costs nothing.
const COMPLETE_TIMEOUT: Duration = Duration::from_millis(5);

/// Bounds on the advanced transfer settings, so a bad config file cannot
/// wedge the stream.
const MIN_TRANSFERS: usize = 4;
const MAX_TRANSFERS: usize = 64;
const MIN_TRANSFER_KIB: usize = 2;
const MAX_TRANSFER_KIB: usize = 256;

/// The endpoint's packet size. Every transfer length must be a multiple of it.
const BULK_PACKET: usize = 512;

/// Open the device and start the stream thread.
///
/// The device is opened *on the thread* so that all register access happens on
/// one thread; this function blocks on a handshake until that has succeeded or
/// failed, so the caller still gets a synchronous `Result`.
pub(crate) fn spawn(cfg: &RtlSdrConfig, center_hz: f64) -> Result<RtlSdrHandle> {
    let (ctrl_tx, ctrl_rx) = crossbeam_channel::unbounded::<Ctrl>();
    // Rendezvous for the open result, so the caller learns about a permission
    // problem or a missing dongle as a normal error rather than as a stream
    // that silently never starts.
    let (ready_tx, ready_rx) = crossbeam_channel::bounded::<Result<DeviceInfo>>(1);

    let shared = Arc::new(Shared {
        alive: AtomicBool::new(true),
        last_rx_ms: AtomicU64::new(0),
        rate_milli_hz: AtomicU64::new(0),
        effective_gain_tenth_db: AtomicI64::new(-1),
        rx_paused: AtomicBool::new(false),
    });

    // The ring is sized for the configured rate; the achieved rate differs by
    // well under a hertz, so it does not need resizing afterwards.
    let (rx_prod, rx_cons) = ring_for(cfg.sample_rate_hz);

    let cfg = cfg.clone();
    let thread_shared = Arc::clone(&shared);
    let join = std::thread::Builder::new()
        .name("sdroxide-rtlsdr".into())
        .spawn(move || {
            run(cfg, center_hz, ctrl_rx, rx_prod, Arc::clone(&thread_shared), ready_tx);
            thread_shared.alive.store(false, Ordering::Relaxed);
        })
        .map_err(|e| Error::Access(format!("could not start the RTL-SDR thread: {e}")))?;

    match ready_rx.recv() {
        Ok(Ok(info)) => Ok(RtlSdrHandle::from_parts(
            rx_cons,
            ctrl_tx,
            shared,
            join,
            info.label,
            info.tuner,
            info.is_blog_v4,
            info.hf_capable,
            info.sample_rate_hz,
            info.gains_db,
        )),
        Ok(Err(e)) => {
            let _ = join.join();
            Err(e)
        }
        // The thread died before reporting; joining surfaces a panic message.
        Err(_) => {
            let _ = join.join();
            Err(Error::Access("the RTL-SDR thread stopped before it opened the device".into()))
        }
    }
}

/// What the thread reports back once the device is up.
struct DeviceInfo {
    label: String,
    tuner: String,
    is_blog_v4: bool,
    hf_capable: bool,
    sample_rate_hz: f64,
    gains_db: Vec<f64>,
}

fn run(
    cfg: RtlSdrConfig,
    center_hz: f64,
    ctrl: crossbeam_channel::Receiver<Ctrl>,
    mut rx: Producer<f32>,
    shared: Arc<Shared>,
    ready: crossbeam_channel::Sender<Result<DeviceInfo>>,
) {
    let mut dev = match Device::open(&cfg, center_hz) {
        Ok(d) => d,
        Err(e) => {
            let _ = ready.send(Err(e));
            return;
        }
    };

    let rate = dev.sample_rate();
    shared.rate_milli_hz.store((rate * 1000.0) as u64, Ordering::Relaxed);
    publish_gain(&shared, &dev);
    let _ = ready.send(Ok(DeviceInfo {
        label: dev.label().to_string(),
        tuner: dev.tuner_name().to_string(),
        is_blog_v4: dev.is_blog_v4(),
        hf_capable: dev.hf_capable(),
        sample_rate_hz: rate,
        gains_db: dev.gains_db(),
    }));

    if let Err(e) = pump(&mut dev, &ctrl, &mut rx, &shared, &cfg) {
        tracing::warn!("RTL-SDR stream stopped: {e}");
    }
    dev.shutdown();
}

/// Transfer geometry, clamped and rounded to something the endpoint accepts.
fn geometry(cfg: &RtlSdrConfig) -> (usize, usize) {
    let transfers = (cfg.transfers as usize).clamp(MIN_TRANSFERS, MAX_TRANSFERS);
    let kib = (cfg.transfer_kib as usize).clamp(MIN_TRANSFER_KIB, MAX_TRANSFER_KIB);
    // Round down to a whole number of packets: a transfer length that is not a
    // multiple of 512 is rejected outright.
    let bytes = (kib * 1024 / BULK_PACKET).max(1) * BULK_PACKET;
    (transfers, bytes)
}

fn pump(
    dev: &mut Device,
    ctrl: &crossbeam_channel::Receiver<Ctrl>,
    rx: &mut Producer<f32>,
    shared: &Arc<Shared>,
    cfg: &RtlSdrConfig,
) -> Result<()> {
    let (in_flight, xfer_bytes) = geometry(cfg);
    tracing::info!(
        "RTL-SDR streaming: {} transfers x {} KiB = {:.0} ms of buffering at {:.3} Msps",
        in_flight,
        xfer_bytes / 1024,
        (in_flight * xfer_bytes) as f64 / (dev.sample_rate() * 2.0) * 1000.0,
        dev.sample_rate() / 1e6,
    );

    let mut ep = dev
        .usb()
        .interface()
        .endpoint::<Bulk, In>(BULK_EP)
        .map_err(|e| Error::Access(format!("cannot open the RTL-SDR bulk endpoint: {e}")))?;

    let lut = sample_lut();

    let mut stats = RxStats::new(dev.sample_rate());
    let started = Instant::now();
    // A bulk transfer can complete short, and on an *odd* byte count. Dropping
    // that byte would swap I and Q for the rest of the session, so it is
    // carried into the next transfer instead. librtlsdr assumes even lengths
    // and does not do this.
    let mut odd_carry: Option<u8> = None;
    let mut scratch: Vec<f32> = Vec::with_capacity(xfer_bytes);
    // Copy of each completed transfer in ordinary (cacheable) memory
    let mut staging: Vec<u8> = Vec::with_capacity(xfer_bytes);

    dev.reset_buffer()?;
    for _ in 0..in_flight {
        ep.submit(ep.allocate(xfer_bytes));
    }

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
            // Keep the hardware fed while we talk on endpoint 0.
            while ep.pending() < in_flight {
                ep.submit(ep.allocate(xfer_bytes));
            }
            if let Err(e) = apply(dev, &pending, &mut odd_carry) {
                // A setting that cannot be applied is worth saying out loud,
                // but it is not a reason to tear the stream down — the
                // operator can pick another value.
                tracing::warn!("RTL-SDR: {e}");
            }
            publish_gain(shared, dev);
        }

        // 2. Refill before draining, so the queue is never empty while the
        //    device is producing.
        while ep.pending() < in_flight {
            ep.submit(ep.allocate(xfer_bytes));
        }

        // 3. Drain every completion that is ready, not just one.
        //
        // Draining a single transfer per iteration is the obvious shape and it
        // does not work: an applied retune costs ~25 ms of I2C, during which
        // the device produces ~7 transfers' worth of samples. Taking one per
        // iteration drains 16 KiB per 25 ms against a device delivering
        // 4.8 MB/s, so under a sustained drag the stream starves completely —
        // measured at literally zero samples over a minute before this loop
        // was added. Block only on the first; poll the rest.
        let mut drained = 0usize;
        loop {
            let timeout = if drained == 0 { COMPLETE_TIMEOUT } else { Duration::ZERO };
            let Some(completion) = ep.wait_next_complete(timeout) else {
                break;
            };
            match completion.status {
                Ok(()) => {
                    // On a host that is not cache-coherent - a Raspberry Pi 5 / CM5,
                    // on its RP1 xHCI ports and its dwc2 port alike - the kernel maps
                    // that memory non-cacheable, and `convert` read it one byte at
                    // a time, then paid a full trip to RAM per byte: ~2.3 ms per 16 KiB
                    // transfer on a CM5, two thirds of a core at 2.4 MB/s.
                    // This fix brings down CPU work significantly.
                    staging.clear();
                    staging.extend_from_slice(&completion.buffer[..]);
                    convert(&staging, &lut, &mut odd_carry, &mut scratch);
                    
                    if !scratch.is_empty() {
                        stats.on_iq(scratch.len() / 2);
                        push_iq(rx, &scratch, &mut stats, shared.rx_paused.load(Ordering::Relaxed));
                        shared
                            .last_rx_ms
                            .store(started.elapsed().as_millis() as u64, Ordering::Relaxed);
                    }
                }
                Err(TransferError::Cancelled) => {}
                Err(TransferError::Disconnected) => {
                    return Err(Error::NotFound("the RTL-SDR was unplugged".into()));
                }
                Err(TransferError::Stall) => {
                    tracing::warn!("RTL-SDR endpoint stalled; clearing and resynchronising");
                    let _ = ep.clear_halt().wait();
                    dev.reset_buffer()?;
                    // The stream restarts from a packet boundary, so a byte
                    // carried from before the stall is meaningless.
                    odd_carry = None;
                    stats.on_error();
                }
                Err(e) => {
                    tracing::debug!("RTL-SDR transfer error: {e}");
                    stats.on_error();
                }
            }

            // Reuse the buffer rather than allocating a new one every 3.4 ms.
            let mut buf = completion.buffer;
            buf.clear();
            ep.submit(buf);

            drained += 1;
            // Go back and serve control rather than spinning here forever if
            // the device is outrunning us.
            if drained >= in_flight {
                break;
            }
        }

        stats.tick();
    }

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

/// Publish the gain the tuner actually settled on, so the UI slider can snap to
/// a value the hardware can produce instead of the one that was requested.
fn publish_gain(shared: &Arc<Shared>, dev: &Device) {
    let v = dev.effective_gain_db().map(|db| (db * 10.0).round() as i64).unwrap_or(-1);
    shared.effective_gain_tenth_db.store(v, Ordering::Relaxed);
}

/// Apply coalesced control changes, in dependency order.
///
/// ppm first because the rate and every frequency are computed against the
/// corrected crystals; then the rate, because it moves the tuner's IF; then the
/// HF path, which may re-init the tuner; then the centre; then gain, which a
/// tuner re-init would otherwise have reset; then the bias tee, which depends
/// on nothing.
fn apply(dev: &mut Device, p: &Pending, odd_carry: &mut Option<u8>) -> Result<()> {
    if let Some(ppm) = p.ppm {
        dev.set_ppm(ppm)?;
    }
    if let Some(mode) = p.hf_mode {
        dev.set_hf_mode(mode)?;
        *odd_carry = None;
    }
    if let Some(hz) = p.center {
        dev.set_center(hz)?;
    }
    if let Some(agc) = p.agc {
        dev.set_agc(agc)?;
    }
    if let Some(db) = p.gain {
        // A gain request while the tuner's own loop is running would be
        // overwritten by it; ignore rather than fight.
        dev.set_gain_db(db)?;
    }
    if let Some(on) = p.bias_tee {
        dev.set_bias_tee(on)?;
    }
    Ok(())
}

/// Conversion lookup: 8-bit unsigned to f32. 127.4 rather than 127.5 is the
/// measured DC centre of the RTL2832's ADC output, and is what librtlsdr and
/// SoapyRTLSDR both use; 1/128 puts full scale just inside ±1.0. A table rather
/// than arithmetic because this runs 4.8 million times a second.
///
/// Shared with the `rtl_tcp` backend: the bytes on that socket are the ones
/// this endpoint produced, so they convert by the same table.
pub(crate) fn sample_lut() -> [f32; 256] {
    core::array::from_fn(|i| (i as f32 - 127.4) * (1.0 / 128.0))
}

/// Convert a transfer's 8-bit unsigned I/Q into interleaved `f32`.
///
/// `carry` holds a leftover byte from a previous short transfer so that I/Q
/// pairing survives an odd-length completion — or, over `rtl_tcp`, a socket
/// read that ended between an I and its Q, which happens constantly.
pub(crate) fn convert(bytes: &[u8], lut: &[f32; 256], carry: &mut Option<u8>, out: &mut Vec<f32>) {
    out.clear();
    out.reserve(bytes.len() + 1);

    let mut rest = bytes;
    if let Some(c) = carry.take() {
        if let Some((first, tail)) = rest.split_first() {
            out.push(lut[c as usize]);
            out.push(lut[*first as usize]);
            rest = tail;
        } else {
            // Nothing arrived to pair it with; keep holding it.
            *carry = Some(c);
            return;
        }
    }

    let pairs = rest.len() / 2;
    for chunk in rest[..pairs * 2].chunks_exact(2) {
        out.push(lut[chunk[0] as usize]);
        out.push(lut[chunk[1] as usize]);
    }
    if rest.len() % 2 == 1 {
        *carry = Some(rest[rest.len() - 1]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lut() -> [f32; 256] {
        sample_lut()
    }

    #[test]
    fn lut_matches_the_scalar_formula_everywhere() {
        let l = lut();
        for i in 0..256 {
            let expect = (i as f32 - 127.4) * (1.0 / 128.0);
            assert_eq!(l[i], expect, "byte {i}");
        }
        // Mid-scale is very close to zero, and full scale sits just inside ±1.
        assert!(l[127].abs() < 0.01, "mid-scale should be near DC: {}", l[127]);
        assert!(l[0] > -1.0 && l[255] < 1.0, "full scale must not exceed unity");
    }

    /// The hazard this defends against: a bulk transfer completing on an odd
    /// byte count. Split the same stream into odd-length chunks and the output
    /// must be identical to processing it in one go — otherwise I and Q swap
    /// for the rest of the session.
    #[test]
    fn odd_length_transfers_do_not_swap_i_and_q() {
        let l = lut();
        let stream: Vec<u8> = (0..64u8).collect();

        let mut carry = None;
        let mut whole = Vec::new();
        let mut out = Vec::new();
        convert(&stream, &l, &mut carry, &mut out);
        whole.extend_from_slice(&out);
        assert_eq!(carry, None, "an even stream leaves nothing carried");

        let mut carry = None;
        let mut chunked = Vec::new();
        let mut pos = 0;
        for len in [3usize, 5, 7, 1, 9, 2, 11, 6, 13, 7] {
            let end = (pos + len).min(stream.len());
            if pos >= end {
                break;
            }
            convert(&stream[pos..end], &l, &mut carry, &mut out);
            chunked.extend_from_slice(&out);
            pos = end;
        }
        if pos < stream.len() {
            convert(&stream[pos..], &l, &mut carry, &mut out);
            chunked.extend_from_slice(&out);
        }

        assert_eq!(whole, chunked, "odd-length chunking must not shift the I/Q phase");
    }

    #[test]
    fn a_lone_byte_is_held_until_a_partner_arrives() {
        let l = lut();
        let mut carry = None;
        let mut out = Vec::new();

        convert(&[0x10], &l, &mut carry, &mut out);
        assert!(out.is_empty(), "a single byte cannot form a pair");
        assert_eq!(carry, Some(0x10));

        // An empty completion must not discard the carried byte.
        convert(&[], &l, &mut carry, &mut out);
        assert!(out.is_empty());
        assert_eq!(carry, Some(0x10), "an empty transfer must not drop the carry");

        convert(&[0x20], &l, &mut carry, &mut out);
        assert_eq!(out, vec![l[0x10], l[0x20]]);
        assert_eq!(carry, None);
    }

    #[test]
    fn conversion_output_is_always_an_even_number_of_floats() {
        let l = lut();
        let mut carry = None;
        let mut out = Vec::new();
        for len in 0..32usize {
            let bytes: Vec<u8> = (0..len as u8).collect();
            convert(&bytes, &l, &mut carry, &mut out);
            assert_eq!(out.len() % 2, 0, "odd float count from a {len}-byte transfer");
        }
    }

    /// Transfer lengths must be whole packets, and the clamps must hold
    /// against a hand-edited config file.
    #[test]
    fn geometry_rounds_to_whole_packets_and_clamps() {
        let mut cfg = RtlSdrConfig::default();
        let (n, bytes) = geometry(&cfg);
        assert_eq!(n, 16);
        assert_eq!(bytes, 16 * 1024);
        assert_eq!(bytes % BULK_PACKET, 0);

        // Absurd values are clamped rather than believed.
        cfg.transfers = 0;
        cfg.transfer_kib = 0;
        let (n, bytes) = geometry(&cfg);
        assert_eq!(n, MIN_TRANSFERS);
        assert!(bytes >= BULK_PACKET && bytes % BULK_PACKET == 0);

        cfg.transfers = 255;
        cfg.transfer_kib = 4096;
        let (n, bytes) = geometry(&cfg);
        assert_eq!(n, MAX_TRANSFERS);
        assert_eq!(bytes, MAX_TRANSFER_KIB * 1024);

        // Any KiB value still yields whole packets.
        for kib in 1..=64u16 {
            cfg.transfer_kib = kib;
            let (_, bytes) = geometry(&cfg);
            assert_eq!(bytes % BULK_PACKET, 0, "{kib} KiB is not a whole number of packets");
        }
    }
}
