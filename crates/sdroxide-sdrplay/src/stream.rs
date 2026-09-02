//! The session thread that owns an RSP, and the callbacks the service feeds.
//!
//! # Ownership and threads
//!
//! Unlike the USB backends there is no endpoint to pump: after
//! `sdrplay_api_Init` the *service* pushes samples into [`stream_cb`] on a
//! thread it owns. The session thread here exists to own everything else —
//! device selection, the parameter block, every `sdrplay_api_Update` — so all
//! control stays on one thread, the same invariant the RX-888 backend keeps
//! for its USB handle.
//!
//! Two rules the safety of this module rests on:
//!
//! * The `DeviceParamsT` pointer from `GetDeviceParams` refers to storage the
//!   service owns, valid from `SelectDevice` to `ReleaseDevice`. It is
//!   dereferenced **only on the session thread** — never in a callback.
//! * The callback context is a `Box` leaked before `Init` and reclaimed only
//!   **after `Uninit` returns**, which is the API's guarantee that no
//!   callback is still running or will run again.
//!
//! Callbacks arrive on a foreign thread, so they must never unwind into the
//! service: every callback body is wrapped in `catch_unwind`, and a panic
//! marks the session dead (the engine reopens it) instead of corrupting the
//! service's stack.
//!
//! # Two tuners
//!
//! An RSPduo asked for diversity runs in the API's dual-tuner mode, where both
//! stream callbacks fire and each carries one tuner. Everything below then
//! happens twice: two parameter blocks are configured, and every
//! `sdrplay_api_Update` is issued once per tuner, because the call names the
//! tuner it applies to and `Tuner_Both` is a selection, not an addressee. The
//! samples are put back together in [`crate::pair`] before they reach the
//! ring.

use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, RecvTimeoutError};
use rtrb::Producer;
use sdroxide_types::{SdrPlayConfig, SdrPlayDuoRole, SdrPlayDuoTuner, SdrPlayModel};

use crate::api;
use crate::device;
use crate::error::{Error, Result};
use crate::ffi;
use crate::handle::{
    Ctrl, DuoMode, Pending, RxStats, SdrPlayDevice, SdrPlayHandle, Shared, TUNERS, push_iq,
};
use crate::pair::{Pairer, Side};

/// 16-bit wire samples to full-scale ±1.0.
const SCALE: f32 = 1.0 / 32768.0;

/// How long the session loop waits for control before housekeeping (overload
/// acknowledgements) runs anyway.
const CTRL_TIMEOUT: Duration = Duration::from_millis(100);

/// How long an announced gain must stand before it is believed without the
/// service having flagged the block that carries it — see [`settle_gain`].
///
/// Comfortably longer than the pipe it is covering for (a few milliseconds of
/// USB and service buffering at any rate this backend offers) and far shorter
/// than an operator can see on a meter.
const GAIN_SETTLE: Duration = Duration::from_millis(25);

/// The tuner's own range: 1 kHz to 2 GHz on every model.
const MIN_RF_HZ: f64 = 1_000.0;
const MAX_RF_HZ: f64 = 2_000_000_000.0;

/// Open the device and start streaming, and hand back the session.
///
/// The device is selected *on the session thread* so all control stays there;
/// this function blocks on a handshake so the caller still gets a synchronous
/// `Result`. Nothing is delivered until a radio claims a tuner — see
/// [`SdrPlayDevice::attach`].
pub fn open(cfg: &SdrPlayConfig, center_hz: f64) -> Result<Arc<SdrPlayDevice>> {
    let (ctrl_tx, ctrl_rx) = crossbeam_channel::unbounded::<Ctrl>();
    let (ready_tx, ready_rx) = crossbeam_channel::bounded::<Result<DeviceInfo>>(1);

    let shared = Arc::new(Shared::new());

    let cfg = cfg.clone();
    let thread_shared = Arc::clone(&shared);
    let join = std::thread::Builder::new()
        .name("sdroxide-sdrplay".into())
        .spawn(move || {
            run(cfg, center_hz, ctrl_rx, Arc::clone(&thread_shared), ready_tx);
            thread_shared.alive.store(false, Ordering::Relaxed);
        })
        .map_err(|e| Error::Api {
            call: "spawn",
            text: format!("could not start the SDRplay thread: {e}"),
        })?;

    match ready_rx.recv() {
        Ok(Ok(info)) => Ok(Arc::new(SdrPlayDevice::from_parts(
            ctrl_tx,
            shared,
            join,
            info.serial,
            info.model,
            info.main,
            info.duo,
            info.out_rate_hz,
            info.analog_bw_hz,
            info.low_if_khz,
        ))),
        Ok(Err(e)) => {
            let _ = join.join();
            Err(e)
        }
        Err(_) => {
            let _ = join.join();
            Err(Error::Api {
                call: "spawn",
                text: "the SDRplay thread stopped before it opened the device".into(),
            })
        }
    }
}

/// Open a device and claim one tuner on it in one step — what a lone radio
/// wants, and what the tests and the example use.
pub fn spawn(cfg: &SdrPlayConfig, center_hz: f64) -> Result<SdrPlayHandle> {
    let dev = open(cfg, center_hz)?;
    let tuner = tuner_idx(cfg.duo_tuner);
    dev.attach(tuner, center_hz)
}

/// The index a configured tuner has in every per-tuner array here: tuner 1 is
/// the API's tuner A.
pub(crate) fn tuner_idx(t: SdrPlayDuoTuner) -> usize {
    match t {
        SdrPlayDuoTuner::Tuner1 => 0,
        SdrPlayDuoTuner::Tuner2 => 1,
    }
}

/// The API's addressee for a tuner index.
fn tuner_sel(idx: usize) -> ffi::TunerSelect {
    if idx == 1 { ffi::TUNER_B } else { ffi::TUNER_A }
}

struct DeviceInfo {
    serial: String,
    model: SdrPlayModel,
    main: usize,
    duo: DuoMode,
    out_rate_hz: f64,
    analog_bw_hz: f64,
    low_if_khz: i32,
}

/// What the requested sample rate turns into on this device, which depends on
/// how many tuners are running.
///
/// One tuner samples at baseband and the ADC follows the rate. Two share one
/// ADC at a fixed 6 MHz and hand the service a low IF, from which its own
/// downconverter delivers 2 Msps — decimated further for anything narrower.
/// A rate the second arrangement cannot reach is clamped rather than refused:
/// a configuration carried over from single-tuner operation should still open.
#[derive(Debug, Clone, Copy, PartialEq)]
struct RatePlan {
    dual: bool,
    /// ADC rate to program.
    fs_hz: f64,
    /// The API's decimation factor.
    decim: u8,
    /// What comes out, after that decimation.
    out_hz: f64,
    if_type: ffi::IfType,
}

impl RatePlan {
    /// `dual` is what the device *opened* as, not what was asked for: a
    /// diversity setting left behind by an RSPduo must not put an RSP1A into
    /// a low IF and a 2 Msps ceiling it has no reason to be in.
    fn for_device(cfg: &SdrPlayConfig, dual: bool) -> RatePlan {
        if dual {
            let decim = device::dual_decim(cfg.sample_rate_hz);
            RatePlan {
                dual: true,
                fs_hz: device::DUAL_FS_HZ,
                decim,
                out_hz: device::DUAL_OUT_HZ / f64::from(decim),
                if_type: ffi::IF_1_620,
            }
        } else {
            let (fs, decim) = device::fs_and_decim(cfg.sample_rate_hz);
            RatePlan {
                dual: false,
                fs_hz: fs,
                decim,
                out_hz: fs / f64::from(decim),
                if_type: ffi::IF_ZERO,
            }
        }
    }
}

/// Everything the callbacks touch. Leaked to a raw pointer for the life of
/// the Init..Uninit span; see the module doc.
struct CbCtx {
    shared: Arc<Shared>,
    stream: Mutex<StreamState>,
    epoch: Instant,
}

struct StreamState {
    /// Where each tuner's samples go, or `None` while no radio is reading it.
    /// In [`DuoMode::Paired`] only the main tuner's is ever used: the pair
    /// travels down it together.
    rings: [Option<Producer<f32>>; TUNERS],
    /// Where one tuner's block is converted to `f32` on its way to the ring.
    /// The paired path has no use for it: [`Pairer`] converts as it stages.
    scratch: Vec<f32>,
    /// `Some` while both tuners are being combined — see [`crate::pair`].
    pair: Option<Pairer>,
    stats: [RxStats; TUNERS],
    duo: DuoMode,
    main: usize,
}

/// Mutable session state the apply loop tracks for one tuner between updates.
struct LiveTuner {
    center_hz: f64,
    /// What the operator asked the LNA to be — kept separate from what the
    /// band allows, so retuning out of a restrictive band restores it.
    lna_wanted: u8,
    /// What is actually programmed after the per-band clamp.
    lna_programmed: u8,
    /// The IF gain-reduction floor in force — the API's normal 20 dB, or 0
    /// where the operator has opened the last of the range.
    min_gr: ffi::MinGr,
    antenna: String,
}

/// The same for the device as a whole.
struct Live {
    t: [LiveTuner; TUNERS],
    hdr: bool,
}

/// The parameter block and `Update` addressee for each tuner in use.
///
/// The API's channel blocks are named for the tuners, not for the jobs: `A` is
/// always tuner 1. Which of them a given radio listens on is the operator's
/// choice ([`SdrPlayConfig::duo_tuner`]), so both are resolved once, here, and
/// everything downstream indexes them.
struct Tuners {
    /// Both parameter blocks, in tuner order. The one this session did not
    /// open is `None`.
    ch: [Option<*mut ffi::RxChannelParamsT>; TUNERS],
    /// The tuner the session was opened for.
    main: usize,
}

impl Tuners {
    /// Whether this session is running `idx` at all.
    fn has(&self, idx: usize) -> bool {
        self.ch.get(idx).is_some_and(Option::is_some)
    }

    /// Every tuner this session is running, in order.
    fn live(&self) -> impl Iterator<Item = usize> + '_ {
        (0..TUNERS).filter(|&i| self.has(i))
    }
}

fn run(
    cfg: SdrPlayConfig,
    center_hz: f64,
    ctrl: Receiver<Ctrl>,
    shared: Arc<Shared>,
    ready: crossbeam_channel::Sender<Result<DeviceInfo>>,
) {
    let (api, mut dev) = match api::select(&cfg.serial, cfg.duo_tuner, cfg.wants_dual_tuner()) {
        Ok(v) => v,
        Err(e) => {
            if matches!(e, Error::ServiceDown(_)) {
                // Reconnect from scratch next time: the service may have been
                // restarted under us.
                api::reset();
            }
            let _ = ready.send(Err(e));
            return;
        }
    };
    let model = SdrPlayModel::from_hw_ver(dev.hw_ver);
    let serial = dev.serial();
    if let Some(w) = sdroxide_types::SdrPlayDevice::degraded_warning(&serial, model) {
        // Streaming still starts — the operator may know better — but this in
        // the log turns "deaf for days" into a one-line diagnosis.
        tracing::warn!("{w}");
    }

    let mut params_ptr: *mut ffi::DeviceParamsT = std::ptr::null_mut();
    let err = unsafe { (api.get_device_params)(dev.dev, &mut params_ptr) };
    if err != ffi::ERR_SUCCESS || params_ptr.is_null() {
        api::release(&mut dev);
        let _ = ready.send(Err(Error::from_status(&api, "GetDeviceParams", err)));
        return;
    }
    // Service-owned storage; see the module doc for the lifetime rule.
    let dev_params = unsafe { (*params_ptr).dev_params };
    // Believe the device record rather than the configuration: `select` only
    // puts an RSPduo into dual-tuner mode, and a diversity setting left behind
    // by one must not have the driver look for a second tuner on an RSP1A.
    let dual = dev.rsp_duo_mode == ffi::RSPDUO_MODE_DUAL_TUNER;
    let plan = RatePlan::for_device(&cfg, dual);
    let main = tuner_idx(cfg.duo_tuner);
    let duo = match (dual, cfg.duo.role) {
        (false, _) => DuoMode::Single,
        (true, SdrPlayDuoRole::Diversity) => DuoMode::Paired,
        (true, SdrPlayDuoRole::SecondRadio) => DuoMode::Split,
    };
    let (ch_a, ch_b) = unsafe { ((*params_ptr).rx_channel_a, (*params_ptr).rx_channel_b) };
    let tuners = if dual {
        Tuners { ch: [Some(ch_a), Some(ch_b)], main }
    } else {
        // Single-tuner mode: the API gives the selected tuner's block, and
        // the other one is not this session's to touch.
        let only = if dev.tuner == ffi::TUNER_B { 1 } else { 0 };
        let mut ch = [None, None];
        ch[only] = Some(if only == 1 { ch_b } else { ch_a });
        Tuners { ch, main: only }
    };
    if dev_params.is_null() || tuners.live().any(|i| tuners.ch[i].is_some_and(|p| p.is_null())) {
        api::release(&mut dev);
        let _ = ready.send(Err(Error::Api {
            call: "GetDeviceParams",
            text: "the service returned no parameter block for the selected tuner".into(),
        }));
        return;
    }

    let effective = plan.out_hz;
    let bw = device::bw_khz_for(cfg.bw_khz, effective, dual);
    let center = center_hz.clamp(MIN_RF_HZ, MAX_RF_HZ);
    let hiz = device::antenna_is_hiz(model, &cfg.antenna);
    let hdr = cfg.hdr && model.has_hdr();
    let lna_wanted = cfg.lna_state.min(model.max_lna_state());
    let lna_programmed = lna_wanted.min(device::max_lna_state(model, center, hiz, hdr));
    // The second tuner's opening gains. Combining, they are the aerial
    // settings that belong to this radio; split, they are a placeholder until
    // the radio that owns that tuner attaches and sets its own — which it does
    // as soon as it opens.
    let aux_lna_wanted = cfg.duo.lna_state.min(model.max_lna_state());
    // The second tuner has no port choice of its own — its 50 Ω input is all
    // it has — so its clamp never sees a Hi-Z path.
    let aux_lna_programmed = aux_lna_wanted.min(device::max_lna_state(model, center, false, false));

    let mut live = Live {
        t: std::array::from_fn(|i| {
            let is_main = i == main;
            LiveTuner {
                center_hz: center,
                lna_wanted: if is_main { lna_wanted } else { aux_lna_wanted },
                lna_programmed: if is_main { lna_programmed } else { aux_lna_programmed },
                // Both tuners share the operator's choice: they are one board,
                // and a diversity pair gained differently is a pair the filter
                // cannot line up.
                min_gr: if cfg.extended_if_gr { ffi::EXTENDED_MIN_GR } else { ffi::NORMAL_MIN_GR },
                antenna: if is_main { cfg.antenna.clone() } else { String::new() },
            }
        }),
        hdr,
    };

    unsafe {
        let dp = &mut *dev_params;
        dp.ppm = cfg.ppm;
        dp.fs_freq.fs_hz = plan.fs_hz;

        // Every tuner this session opened gets the same span, filter,
        // decimation and notches; only the gains differ. That is not tidiness
        // — two branches of a diversity pair filtered differently are two
        // branches the adaptive filter cannot line up — and for two separate
        // radios it is simply the shared half of one board.
        for i in tuners.live() {
            let ch = &mut *tuners.ch[i].expect("live tuner has a block");
            ch.tuner_params.bw_type = bw;
            ch.tuner_params.if_type = plan.if_type;
            ch.tuner_params.lo_mode = ffi::LO_AUTO;
            ch.tuner_params.rf_freq.rf_hz = center;
            ch.tuner_params.gain.g_rdb = if i == main {
                cfg.if_gr_db.clamp(cfg.if_gr_min(), SdrPlayConfig::IF_GR_MAX)
            } else {
                cfg.duo.if_gr_db.clamp(cfg.if_gr_min(), SdrPlayConfig::IF_GR_MAX)
            };
            ch.tuner_params.gain.lna_state = live.t[i].lna_programmed;
            ch.tuner_params.gain.min_gr =
                if cfg.extended_if_gr { ffi::EXTENDED_MIN_GR } else { ffi::NORMAL_MIN_GR };
            ch.ctrl_params.decimation.enable = (plan.decim > 1) as u8;
            ch.ctrl_params.decimation.decimation_factor = plan.decim;
            ch.ctrl_params.agc.enable = cfg.agc.code() as ffi::AgcControl;
            // Clamped to what this *rate* allows, not to the widest range any
            // rate allows: the API refuses a set point the converter cannot
            // reach, and this backend offers 10 MSPS where the floor is -48.
            let sp = cfg.agc_setpoint_range();
            ch.ctrl_params.agc.set_point_dbfs = cfg.agc_setpoint_dbfs.clamp(*sp.start(), *sp.end());
            if model == SdrPlayModel::RspDuo {
                ch.rsp_duo_tuner_params.rf_notch_enable = cfg.rf_notch as u8;
                ch.rsp_duo_tuner_params.rf_dab_notch_enable = cfg.dab_notch as u8;
            }
        }

        let ch = &mut *tuners.ch[main].expect("the main tuner has a block");
        match model {
            SdrPlayModel::Rsp1a | SdrPlayModel::Rsp1b => {
                dp.rsp1a_params.rf_notch_enable = cfg.rf_notch as u8;
                dp.rsp1a_params.rf_dab_notch_enable = cfg.dab_notch as u8;
                ch.rsp1a_tuner_params.bias_t_enable = cfg.bias_tee as u8;
            }
            SdrPlayModel::Rsp2 => {
                ch.rsp2_tuner_params.bias_t_enable = cfg.bias_tee as u8;
                ch.rsp2_tuner_params.rf_notch_enable = cfg.rf_notch as u8;
                if let Some((ant, port)) = device::rsp2_antenna(&cfg.antenna) {
                    ch.rsp2_tuner_params.antenna_sel = ant;
                    ch.rsp2_tuner_params.am_port_sel = port;
                }
            }
            SdrPlayModel::RspDuo => {
                ch.rsp_duo_tuner_params.bias_t_enable = cfg.bias_tee as u8;
                if let Some(port) = device::rspduo_tuner1_amport(&cfg.antenna) {
                    ch.rsp_duo_tuner_params.tuner1_am_port_sel = port;
                }
            }
            SdrPlayModel::RspDx | SdrPlayModel::RspDxR2 => {
                dp.rsp_dx_params.hdr_enable = hdr as u8;
                dp.rsp_dx_params.bias_t_enable = cfg.bias_tee as u8;
                dp.rsp_dx_params.rf_notch_enable = cfg.rf_notch as u8;
                dp.rsp_dx_params.rf_dab_notch_enable = cfg.dab_notch as u8;
                if let Some(ant) = device::rspdx_antenna(&cfg.antenna) {
                    dp.rsp_dx_params.antenna_sel = ant;
                }
            }
            SdrPlayModel::Rsp1 | SdrPlayModel::Unknown => {}
        }
    }
    for i in tuners.live() {
        shared.t[i].lna_state.store(live.t[i].lna_programmed, Ordering::Relaxed);
    }

    // The callback context: leaked here, reclaimed strictly after Uninit.
    let ctx = Box::into_raw(Box::new(CbCtx {
        shared: Arc::clone(&shared),
        stream: Mutex::new(StreamState {
            rings: [None, None],
            scratch: Vec::new(),
            pair: (duo == DuoMode::Paired).then(|| Pairer::new(effective)),
            stats: [RxStats::new(effective), RxStats::new(effective)],
            duo,
            main,
        }),
        epoch: Instant::now(),
    }));
    // With one tuner running both slots point at the same handler: only one of
    // them ever fires, and which one is the service's business. With two, the
    // slot *is* the tuner — that is the only thing that says which aerial a
    // block came off — so each gets a handler that knows which it is.
    let mut cbfns = if dual {
        ffi::CallbackFnsT {
            stream_a: Some(stream_a_cb),
            stream_b: Some(stream_b_cb),
            event: Some(event_cb),
        }
    } else if tuners.main == 1 {
        ffi::CallbackFnsT {
            stream_a: Some(stream_b_cb),
            stream_b: Some(stream_b_cb),
            event: Some(event_cb),
        }
    } else {
        ffi::CallbackFnsT {
            stream_a: Some(stream_a_cb),
            stream_b: Some(stream_a_cb),
            event: Some(event_cb),
        }
    };

    let err = unsafe { (api.init)(dev.dev, &mut cbfns, ctx.cast::<c_void>()) };
    if err != ffi::ERR_SUCCESS {
        let e = Error::from_status(&api, "Init", err);
        if matches!(e, Error::ServiceDown(_)) {
            api::reset();
        }
        api::release(&mut dev);
        // No callback was registered, so the context comes straight back.
        drop(unsafe { Box::from_raw(ctx) });
        let _ = ready.send(Err(e));
        return;
    }

    let label = format!("SDRplay {} (serial {serial})", model.label());
    tracing::info!(
        "{label} streaming: fs {:.3} Msps / {} -> {:.3} Msps, bw {bw} kHz, IF {} kHz, {} \
         samples/pkt",
        plan.fs_hz / 1e6,
        plan.decim,
        effective / 1e6,
        plan.if_type,
        unsafe { (*dev_params).samples_per_pkt },
    );
    match duo {
        DuoMode::Paired => tracing::info!(
            "both RSPduo tuners are running, combined: {} is the main aerial, {} the second \
             (LNA state {aux_lna_programmed}, IF gain reduction {} dB). Dual-tuner mode fixes \
             the ADC at {:.0} MHz and the widest span at {:.3} Msps.",
            cfg.duo_tuner.short_label(),
            cfg.aux_tuner().short_label(),
            cfg.duo.if_gr_db,
            device::DUAL_FS_HZ / 1e6,
            device::DUAL_OUT_HZ / 1e6,
        ),
        DuoMode::Split => tracing::info!(
            "both RSPduo tuners are running as separate radios, each on its own frequency. \
             They share one ADC clock, so both run at {:.3} Msps — whichever radio opened the \
             board set that, and the other adopts it.",
            effective / 1e6,
        ),
        DuoMode::Single => {}
    }
    let _ = ready.send(Ok(DeviceInfo {
        serial,
        model,
        main,
        duo,
        out_rate_hz: effective,
        analog_bw_hz: bw as f64 * 1000.0,
        low_if_khz: plan.if_type,
    }));

    let mut pending = Pending::default();
    loop {
        match ctrl.recv_timeout(CTRL_TIMEOUT) {
            Ok(c) => {
                pending.absorb(c);
                while let Ok(c) = ctrl.try_recv() {
                    pending.absorb(c);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        // The overload acknowledgement the API requires, moved out of the
        // event callback so no Update ever runs on the service's thread. It
        // goes to the tuner that reported it, and to no other.
        let acks = shared.overload_ack_pending.swap(0, Ordering::Relaxed);
        for i in tuners.live() {
            if acks & ack_bit(tuner_sel(i)) != 0 {
                update(
                    &api,
                    &dev,
                    tuner_sel(i),
                    ffi::UPDATE_CTRL_OVERLOAD_ACK,
                    ffi::UPDATE_EXT1_NONE,
                );
            }
        }
        if pending.shutdown {
            break;
        }
        if shared.removed.load(Ordering::Relaxed) {
            tracing::warn!("SDRplay: device removed, stopping the session");
            break;
        }
        if !pending.is_empty() {
            apply(&api, &dev, dev_params, &tuners, model, &shared, &mut live, &mut pending, ctx);
            pending = Pending::default();
        }
    }

    let err = unsafe { (api.uninit)(dev.dev) };
    if err != ffi::ERR_SUCCESS {
        tracing::debug!("Uninit: {}", api.err_text(err));
    }
    api::release(&mut dev);
    // Only now is the API done with the callbacks; the context can come back.
    drop(unsafe { Box::from_raw(ctx) });
    tracing::debug!("SDRplay session ended");
}

/// Which bit of [`Shared::overload_ack_pending`] belongs to a tuner.
fn ack_bit(tuner: ffi::TunerSelect) -> u8 {
    if tuner == ffi::TUNER_B { 0b10 } else { 0b01 }
}

/// One `sdrplay_api_Update`, with failures logged rather than fatal: a slider
/// tweak that the service refuses should not take the receiver down.
///
/// The tuner is named by the caller rather than read off the device: with both
/// running, `dev.tuner` is `Tuner_Both`, which is how the pair was *selected*
/// and not somewhere an update can be sent.
fn update(
    api: &ffi::Api,
    dev: &ffi::DeviceT,
    tuner: ffi::TunerSelect,
    reason: ffi::Reason,
    ext1: ffi::ReasonExt1,
) {
    if reason == ffi::UPDATE_NONE && ext1 == ffi::UPDATE_EXT1_NONE {
        return;
    }
    let err = unsafe { (api.update)(dev.dev, tuner, reason, ext1) };
    if err != ffi::ERR_SUCCESS {
        tracing::warn!("sdrplay_api_Update({reason:#x},{ext1:#x}) failed: {}", api.err_text(err));
    }
}

/// Apply coalesced control to the parameter blocks. Runs only on the session
/// thread — the one place the params pointers may be dereferenced.
///
/// `ctx` is touched only to hand a newly attached radio its ring, under the
/// same mutex the callbacks take.
#[allow(clippy::too_many_arguments)]
fn apply(
    api: &ffi::Api,
    dev: &ffi::DeviceT,
    dev_params: *mut ffi::DevParamsT,
    tuners: &Tuners,
    model: SdrPlayModel,
    shared: &Shared,
    live: &mut Live,
    p: &mut Pending,
    ctx: *mut CbCtx,
) {
    let dp = unsafe { &mut *dev_params };
    let mut ext1 = ffi::UPDATE_EXT1_NONE;

    // Attachments and detachments first: a radio that has just claimed a tuner
    // is about to be sent its opening frequency and gains, and wants its ring
    // in place before them.
    if !p.attach.is_empty() || p.detach.iter().any(|d| *d) {
        // Safety: the same leaked context the callbacks hold, and this lock is
        // the only thing either side touches it through.
        let cx = unsafe { &*ctx };
        if let Ok(mut st) = cx.stream.lock() {
            for t in 0..TUNERS {
                if p.detach[t] {
                    st.rings[t] = None;
                    if let Some(pair) = st.pair.as_mut() {
                        pair.restart();
                    }
                }
            }
            for (t, ring, center) in p.attach.drain(..) {
                st.rings[t] = Some(ring);
                st.stats[t] = RxStats::new(st.stats[t].nominal_hz());
                shared.t[t]
                    .last_rx_ms
                    .store(cx.epoch.elapsed().as_millis() as u64, Ordering::Relaxed);
                // The frequency the radio opened at, applied like any other
                // retune once the ring is in place.
                if p.t[t].center.is_none() {
                    p.t[t].center = Some(center);
                }
            }
        }
    }

    // The device's own settings, which belong to no tuner. Sent to the first
    // tuner this session is running: the API takes an addressee even for a
    // device-wide change, and on a split RSPduo either radio's trim moves the
    // board.
    let mut dev_reason = ffi::UPDATE_NONE;
    if let Some(ppm) = p.ppm {
        dp.ppm = ppm.clamp(-1000.0, 1000.0);
        dev_reason |= ffi::UPDATE_DEV_PPM;
    }
    if let Some(on) = p.hdr
        && model.has_hdr()
    {
        live.hdr = on;
        dp.rsp_dx_params.hdr_enable = on as u8;
        ext1 |= ffi::UPDATE_RSPDX_HDR_ENABLE;
    }
    // The HDR filter rides its own update reason, and belongs to the tuner
    // block rather than the device one — so it is applied per tuner below,
    // through `hdr_bw_pending`.
    let hdr_bw_pending = p.hdr_bw.filter(|_| model.has_hdr());

    for t in tuners.live() {
        let ch = unsafe { &mut *tuners.ch[t].expect("live tuner has a block") };
        let pt = &mut p.t[t];
        let lv = &mut live.t[t];
        let mut reason = ffi::UPDATE_NONE;

        if let Some(hz) = pt.center {
            lv.center_hz = hz.clamp(MIN_RF_HZ, MAX_RF_HZ);
            ch.tuner_params.rf_freq.rf_hz = lv.center_hz;
            reason |= ffi::UPDATE_TUNER_FRF;
        }
        // The floor first: a reduction sent in the same pass must be clamped
        // to the floor this pass installs, not to the one before it.
        if let Some(on) = pt.extended_if_gr {
            lv.min_gr = if on { ffi::EXTENDED_MIN_GR } else { ffi::NORMAL_MIN_GR };
            ch.tuner_params.gain.min_gr = lv.min_gr;
            // Its own reason: the limits move, not the value.
            reason |= ffi::UPDATE_TUNER_GR_LIMITS;
        }
        if let Some(gr) = pt.if_gr {
            ch.tuner_params.gain.g_rdb = gr.clamp(lv.min_gr, SdrPlayConfig::IF_GR_MAX);
            reason |= ffi::UPDATE_TUNER_GR;
        }
        if let Some(bw) = hdr_bw_pending {
            ch.rsp_dx_tuner_params.hdr_bw = bw;
            ext1 |= ffi::UPDATE_RSPDX_HDR_BW;
        }
        if let Some(lna) = pt.lna {
            lv.lna_wanted = lna.min(model.max_lna_state());
        }
        if let Some(name) = pt.antenna.take() {
            match model {
                SdrPlayModel::Rsp2 => {
                    if let Some((ant, port)) = device::rsp2_antenna(&name) {
                        ch.rsp2_tuner_params.antenna_sel = ant;
                        ch.rsp2_tuner_params.am_port_sel = port;
                        reason |= ffi::UPDATE_RSP2_ANTENNA | ffi::UPDATE_RSP2_AM_PORT;
                        lv.antenna = name;
                    }
                }
                SdrPlayModel::RspDx | SdrPlayModel::RspDxR2 => {
                    if let Some(ant) = device::rspdx_antenna(&name) {
                        dp.rsp_dx_params.antenna_sel = ant;
                        ext1 |= ffi::UPDATE_RSPDX_ANTENNA;
                        lv.antenna = name;
                    }
                }
                SdrPlayModel::RspDuo => {
                    // Only tuner 1 has a port to choose; tuner 2 has its 50 Ω
                    // input and nothing else, and the API would refuse.
                    if t == 0
                        && let Some(port) = device::rspduo_tuner1_amport(&name)
                    {
                        ch.rsp_duo_tuner_params.tuner1_am_port_sel = port;
                        reason |= ffi::UPDATE_RSPDUO_AM_PORT;
                        lv.antenna = name;
                    }
                }
                _ => {}
            }
        }

        // The LNA clamp depends on band, port and HDR path, so re-derive it
        // after any of those may have moved and republish only a real change.
        let hiz = device::antenna_is_hiz(model, &lv.antenna);
        let clamped = lv.lna_wanted.min(device::max_lna_state(model, lv.center_hz, hiz, live.hdr));
        if clamped != lv.lna_programmed || pt.lna.is_some() {
            lv.lna_programmed = clamped;
            ch.tuner_params.gain.lna_state = clamped;
            shared.t[t].lna_state.store(clamped, Ordering::Relaxed);
            reason |= ffi::UPDATE_TUNER_GR;
        }

        if pt.agc.is_some() || pt.agc_setpoint.is_some() {
            if let Some(code) = pt.agc {
                ch.ctrl_params.agc.enable = code as ffi::AgcControl;
            }
            if let Some(sp) = pt.agc_setpoint {
                ch.ctrl_params.agc.set_point_dbfs = sp.clamp(-72, -20);
            }
            reason |= ffi::UPDATE_CTRL_AGC;
        }
        if let Some(on) = pt.bias_tee {
            match model {
                SdrPlayModel::Rsp1a | SdrPlayModel::Rsp1b => {
                    ch.rsp1a_tuner_params.bias_t_enable = on as u8;
                    reason |= ffi::UPDATE_RSP1A_BIAS_T;
                }
                SdrPlayModel::Rsp2 => {
                    ch.rsp2_tuner_params.bias_t_enable = on as u8;
                    reason |= ffi::UPDATE_RSP2_BIAS_T;
                }
                SdrPlayModel::RspDuo => {
                    ch.rsp_duo_tuner_params.bias_t_enable = on as u8;
                    reason |= ffi::UPDATE_RSPDUO_BIAS_T;
                }
                SdrPlayModel::RspDx | SdrPlayModel::RspDxR2 => {
                    dp.rsp_dx_params.bias_t_enable = on as u8;
                    ext1 |= ffi::UPDATE_RSPDX_BIAS_T;
                }
                _ => {}
            }
        }
        if let Some(on) = pt.rf_notch {
            match model {
                SdrPlayModel::Rsp1a | SdrPlayModel::Rsp1b => {
                    dp.rsp1a_params.rf_notch_enable = on as u8;
                    reason |= ffi::UPDATE_RSP1A_RF_NOTCH;
                }
                SdrPlayModel::Rsp2 => {
                    ch.rsp2_tuner_params.rf_notch_enable = on as u8;
                    reason |= ffi::UPDATE_RSP2_RF_NOTCH;
                }
                SdrPlayModel::RspDuo => {
                    ch.rsp_duo_tuner_params.rf_notch_enable = on as u8;
                    reason |= ffi::UPDATE_RSPDUO_RF_NOTCH;
                }
                SdrPlayModel::RspDx | SdrPlayModel::RspDxR2 => {
                    dp.rsp_dx_params.rf_notch_enable = on as u8;
                    ext1 |= ffi::UPDATE_RSPDX_RF_NOTCH;
                }
                _ => {}
            }
        }
        if let Some(on) = pt.dab_notch {
            match model {
                SdrPlayModel::Rsp1a | SdrPlayModel::Rsp1b => {
                    dp.rsp1a_params.rf_dab_notch_enable = on as u8;
                    reason |= ffi::UPDATE_RSP1A_DAB_NOTCH;
                }
                SdrPlayModel::RspDuo => {
                    ch.rsp_duo_tuner_params.rf_dab_notch_enable = on as u8;
                    reason |= ffi::UPDATE_RSPDUO_DAB_NOTCH;
                }
                SdrPlayModel::RspDx | SdrPlayModel::RspDxR2 => {
                    dp.rsp_dx_params.rf_dab_notch_enable = on as u8;
                    ext1 |= ffi::UPDATE_RSPDX_DAB_NOTCH;
                }
                _ => {}
            }
        }

        // The device-wide reasons ride with the first tuner's update, and the
        // extension-1 ones with whichever tuner earned them — they are all
        // RSPdx settings, and an RSPdx has one tuner.
        let (reason, ext) = if t == tuners.main {
            (reason | dev_reason, ext1)
        } else {
            (reason, ffi::UPDATE_EXT1_NONE)
        };
        update(api, dev, tuner_sel(t), reason, ext);
    }
}

/// The service's sample delivery for tuner A. Foreign thread — see the module
/// doc.
unsafe extern "C" fn stream_a_cb(
    xi: *mut i16,
    xq: *mut i16,
    params: *mut ffi::StreamCbParamsT,
    num_samples: u32,
    reset: u32,
    cb_context: *mut c_void,
) {
    unsafe { deliver(0, xi, xq, params, num_samples, reset, cb_context) }
}

/// The same for tuner B.
unsafe extern "C" fn stream_b_cb(
    xi: *mut i16,
    xq: *mut i16,
    params: *mut ffi::StreamCbParamsT,
    num_samples: u32,
    reset: u32,
    cb_context: *mut c_void,
) {
    unsafe { deliver(1, xi, xq, params, num_samples, reset, cb_context) }
}

/// Promote the gain the GainChange event announced to the gain the arriving
/// samples were actually taken with.
///
/// The event and the samples are two streams that do not run together: the
/// service announces a gain as it programs the tuner, while the blocks behind
/// it in the USB pipe are still the old gain's. Adopting the announcement
/// straight away puts a step in the S-meter the antenna never saw, for as long
/// as the pipe takes to drain.
///
/// `grChanged` on a block is the service saying "from here on", and the AGC
/// reference note tells anyone measuring the ADC to wait for it. It is right
/// as far as it goes, and on this hardware it does not go far enough:
/// **measured on an RSPdx against a −73 dBm carrier, the flag is set for
/// host-commanded gain updates and never for the hardware AGC's own.** Fifteen
/// seconds of a settling 50 Hz loop produced six GainChange events — 33.4 dB
/// climbing to 51.4 — and one flagged block, so a reader that waited for the
/// flag sat 18 dB stale and stayed there. That is the case this exists for:
/// the AGC is on by default, and it moves the gain whether or not anybody
/// touched a control.
///
/// So the flag is taken as one of two ways in. The other is age: an
/// announcement that has stood unchanged for [`GAIN_SETTLE`] is adopted
/// anyway, on the grounds that the pipe cannot still be holding samples from
/// before it. What that costs is a wrong reading for the settle window on any
/// step the service does flag late — and what it buys is a gain that tracks
/// the loop at all. The two errors are not the same size: a commanded LNA step
/// is tens of dB and gets the flag, while the AGC's own increments are a few dB
/// and get the timeout.
///
/// # Safety
///
/// `params` is the service's, valid for the duration of the callback, and may
/// be null.
unsafe fn settle_gain(ctx: &CbCtx, tuner: usize, params: *mut ffi::StreamCbParamsT) {
    let t = &ctx.shared.t[tuner];
    let announced = t.ev_curr_gain_milli_db.load(Ordering::Relaxed);
    if announced == i64::MIN {
        return;
    }
    let live = t.rx_curr_gain_milli_db.load(Ordering::Relaxed);
    if live == announced {
        return;
    }
    // How long this announcement has stood. Restarted whenever a different one
    // arrives, so a loop that is still hunting keeps resetting it and only the
    // value it settles on ages into place.
    let now_ms = ctx.epoch.elapsed().as_millis() as u64;
    if t.pending_gain_milli_db.swap(announced, Ordering::Relaxed) != announced {
        t.pending_gain_since_ms.store(now_ms, Ordering::Relaxed);
    }
    let flagged = !params.is_null() && unsafe { (*params).gr_changed } != 0;
    let stood = now_ms.saturating_sub(t.pending_gain_since_ms.load(Ordering::Relaxed));
    // The unset arm is the first reading rather than a change: a session that
    // has not moved its gain yet would otherwise wait out the settle window
    // before the meter said anything at all.
    if flagged || live == i64::MIN || stood >= GAIN_SETTLE.as_millis() as u64 {
        t.rx_curr_gain_milli_db.store(announced, Ordering::Relaxed);
    }
}

/// One block of samples, from whichever tuner. Foreign thread — see the module
/// doc; nothing here may unwind, and nothing here may touch the parameter
/// block.
///
/// # Safety
///
/// The pointers are the service's, valid for `num_samples` for the duration of
/// the call, and `cb_context` is the leaked [`CbCtx`].
unsafe fn deliver(
    tuner: usize,
    xi: *mut i16,
    xq: *mut i16,
    params: *mut ffi::StreamCbParamsT,
    num_samples: u32,
    reset: u32,
    cb_context: *mut c_void,
) {
    let ctx = cb_context.cast::<CbCtx>();
    if ctx.is_null() {
        return;
    }
    let panicked = catch_unwind(AssertUnwindSafe(|| {
        let ctx = unsafe { &*ctx };
        if reset != 0 {
            tracing::debug!("SDRplay stream reset (retune or rate change settled)");
        }
        // Pair the announced system gain with the samples it was taken with,
        // before anything below can return early: a tuner nobody is reading
        // still has a gain, and a block of zero samples still carries the flag
        // that says the new one has arrived.
        unsafe { settle_gain(ctx, tuner, params) };
        let n = num_samples as usize;
        if n == 0 || xi.is_null() || xq.is_null() {
            return;
        }
        let xi = unsafe { std::slice::from_raw_parts(xi, n) };
        let xq = unsafe { std::slice::from_raw_parts(xq, n) };
        let Ok(mut guard) = ctx.stream.lock() else {
            return;
        };
        let st = &mut *guard;
        // Which ring this block is bound for, and whose overrun it would be:
        // its own tuner's, except when the two are being combined and both
        // callbacks feed the main tuner's stream.
        let out = if st.duo == DuoMode::Paired { st.main } else { tuner };
        let paused = ctx.shared.t[out].rx_paused.load(Ordering::Relaxed);
        let Some(ring) = st.rings[out].as_mut() else {
            // Nobody is reading this tuner. Its samples are dropped here
            // rather than piling up, and it is not an overrun: a tuner with no
            // radio on it has nothing to fall behind.
            return;
        };
        let (delivered, dropped) = match (st.duo, st.pair.as_mut()) {
            (DuoMode::Paired, Some(pair)) => {
                // The sample number is the only thing that says which of the
                // other tuner's samples these belong with; a service that does
                // not fill it in is noticed inside the pairing.
                let first =
                    if params.is_null() { 0 } else { unsafe { (*params).first_sample_num } };
                let side = if tuner == st.main { Side::Main } else { Side::Aux };
                pair.push(side, xi, xq, first);
                let out = pair.drain(ring, &mut st.stats[st.main], paused);
                ctx.shared.pair_slips.store(pair.slips(), Ordering::Relaxed);
                ctx.shared.aux_stalled.store(pair.stalled(), Ordering::Relaxed);
                ctx.shared.pair_stamped.store(pair.believes_sample_numbers(), Ordering::Relaxed);
                out
            }
            _ => {
                st.scratch.resize(2 * n, 0.0);
                for i in 0..n {
                    st.scratch[2 * i] = f32::from(xi[i]) * SCALE;
                    st.scratch[2 * i + 1] = f32::from(xq[i]) * SCALE;
                }
                (n, push_iq(ring, &st.scratch[..2 * n], &mut st.stats[out], paused, 2))
            }
        };
        if dropped > 0 {
            ctx.shared.t[out].dropped.fetch_add(dropped as u64, Ordering::Relaxed);
        }
        st.stats[out].tick(out, st.duo == DuoMode::Split);
        // Only what actually reached the ring counts as the receiver being
        // alive. A pairing that produces nothing while blocks keep arriving is
        // a receiver that has stopped, and the watchdog has to be able to see
        // that.
        if delivered > 0 {
            ctx.shared.t[out]
                .last_rx_ms
                .store(ctx.epoch.elapsed().as_millis() as u64, Ordering::Relaxed);
        }
    }))
    .is_err();
    if panicked {
        // Unwinding across the FFI boundary is UB; declare the session dead
        // instead and let the engine's reopen machinery rebuild it.
        let shared = &unsafe { &*ctx }.shared;
        shared.alive.store(false, Ordering::Relaxed);
    }
}

/// The service's event delivery. Foreign thread — see the module doc.
unsafe extern "C" fn event_cb(
    event_id: ffi::EventId,
    tuner: ffi::TunerSelect,
    params: *mut ffi::EventParamsT,
    cb_context: *mut c_void,
) {
    let ctx = cb_context.cast::<CbCtx>();
    if ctx.is_null() {
        return;
    }
    let panicked = catch_unwind(AssertUnwindSafe(|| {
        let ctx = unsafe { &*ctx };
        let shared = &ctx.shared;
        match event_id {
            ffi::EVENT_GAIN_CHANGE => {
                if params.is_null() {
                    return;
                }
                // The tuner the event names, so a split RSPduo's two radios
                // each read their own gain rather than the other's.
                let t = if tuner == ffi::TUNER_B { 1 } else { 0 };
                let g = unsafe { (*params).gain };
                shared.t[t].ev_gr_db.store(g.g_rdb as i64, Ordering::Relaxed);
                shared.t[t].ev_lna_gr_db.store(g.lna_g_rdb as i64, Ordering::Relaxed);
                shared.t[t]
                    .ev_curr_gain_milli_db
                    .store((g.curr_gain * 1000.0) as i64, Ordering::Relaxed);
            }
            ffi::EVENT_POWER_OVERLOAD => {
                if params.is_null() {
                    return;
                }
                let kind = unsafe { (*params).power_overload.power_overload_change_type };
                let over = kind == ffi::OVERLOAD_DETECTED;
                let was = shared.overload.swap(over, Ordering::Relaxed);
                // The API insists every overload message is acknowledged, by
                // the tuner it came from; the session thread does that, not
                // this foreign one. An event naming neither tuner (or both) is
                // marked for both, and the session thread only answers for the
                // ones it is running.
                let bits = match tuner {
                    ffi::TUNER_A => 0b01,
                    ffi::TUNER_B => 0b10,
                    _ => 0b11,
                };
                shared.overload_ack_pending.fetch_or(bits, Ordering::Relaxed);
                if over && !was {
                    tracing::warn!(
                        "SDRplay: RF overload on {} — raise the LNA slider (more \
                         attenuation), lower IF gain, or enable AGC",
                        match tuner {
                            ffi::TUNER_A => "tuner 1",
                            ffi::TUNER_B => "tuner 2",
                            _ => "the receiver",
                        }
                    );
                } else if !over && was {
                    tracing::info!("SDRplay: overload corrected");
                }
            }
            ffi::EVENT_DEVICE_REMOVED | ffi::EVENT_DEVICE_FAILURE => {
                tracing::warn!("SDRplay: the service reports the device gone");
                shared.removed.store(true, Ordering::Relaxed);
                shared.alive.store(false, Ordering::Relaxed);
            }
            ffi::EVENT_RSPDUO_MODE_CHANGE => {
                tracing::debug!("SDRplay: RSPduo mode change event");
            }
            _ => {}
        }
    }))
    .is_err();
    if panicked {
        let shared = &unsafe { &*ctx }.shared;
        shared.alive.store(false, Ordering::Relaxed);
    }
}
