//! The station's clock state, shared by every radio in the process.
//!
//! Whichever radio is listening to a time station publishes its measurement
//! here; the NTP server reads it, and a request to set the computer's clock
//! acts on it. One per process, for the reason the transmit interlock is: the
//! computer has one clock and one port 123, however many radios it drives.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::sync::{Arc, Mutex};

use sdroxide_types::{ClockAuthority, ClockStatus, NtpServerConfig, NtpServerStatus};

use crate::ntp::{NtpServer, Reference};
use crate::os;

/// How long a measurement is good for. A computer's crystal left to itself
/// drifts by tens of parts per million, a few milliseconds a minute at worst;
/// the dispersion the NTP server announces grows to say so, and past this it
/// stops claiming to be synchronised at all.
pub const HOLDOVER_NS: i128 = 2 * 3600 * 1_000_000_000;
/// The age past which a measurement is too old to set the clock from.
pub const STEP_MAX_AGE_NS: i128 = 120 * 1_000_000_000;
/// The scatter past which a measurement is not good enough to set the clock.
pub const STEP_MAX_JITTER_NS: i64 = 50_000_000;
/// Closer than this the clock is already right, and a step would do harm.
pub const STEP_MIN_NS: i64 = 1_000_000;
/// Steps larger than this need the operator's confirmation: ten minutes is
/// further than a running computer's clock wanders, so a measurement saying
/// so is either a clock that was never set or a decoder that is wrong — and
/// the operator is the one who knows which.
pub const STEP_CONFIRM_NS: i64 = 600 * 1_000_000_000;
/// How fast a free-running clock is assumed to drift, as RFC 5905's PHI.
const PHI: f64 = 15e-6;

/// One radio's measurement of the computer's clock.
#[derive(Debug, Clone, PartialEq)]
pub struct Fix {
    /// How far the clock is behind the received time, ns: positive means it
    /// is slow and has to be moved forward.
    pub offset_ns: i64,
    /// The scatter of the measurements behind it, ns.
    pub jitter_ns: i64,
    /// The computer's clock when it was made, Unix ns.
    pub at_ns: i128,
    /// The station's RFC 5905 reference identifier.
    pub refid: [u8; 4],
}

/// Why the clock was not set.
#[derive(Debug, Clone, PartialEq)]
pub enum Refusal {
    NoFix,
    Stale {
        age_s: i64,
    },
    Scattered {
        jitter_ms: f64,
    },
    AlreadyRight {
        offset_ms: f64,
    },
    /// Large enough to need the operator's say-so; resend with `force`.
    NeedsConfirmation {
        offset_s: f64,
    },
    Implausible,
    Busy,
    NoAuthority,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::NoFix => write!(f, "no time has been received yet"),
            Refusal::Stale { age_s } => {
                write!(f, "the last time received is {age_s} s old — wait for the next minute")
            }
            Refusal::Scattered { jitter_ms } => {
                write!(
                    f,
                    "the measurements scatter by ±{jitter_ms:.0} ms; not good enough to set from"
                )
            }
            Refusal::AlreadyRight { offset_ms } => {
                write!(f, "the clock is already right ({offset_ms:+.1} ms)")
            }
            Refusal::NeedsConfirmation { offset_s } => {
                write!(f, "the clock is {offset_s:+.0} s out; confirm to set it")
            }
            Refusal::Implausible => write!(f, "the received time is implausibly far out"),
            Refusal::Busy => write!(f, "a clock change is already waiting for the password"),
            Refusal::NoAuthority => write!(
                f,
                "this program cannot set the clock here — run it as an administrator, or use \
                 the NTP server"
            ),
        }
    }
}

/// Whether `fix` may set the clock at `now_ns`, and by how much.
pub fn check_step(fix: Option<&Fix>, now_ns: i128, force: bool) -> Result<i64, Refusal> {
    let f = fix.ok_or(Refusal::NoFix)?;
    let age = now_ns - f.at_ns;
    if age > STEP_MAX_AGE_NS {
        return Err(Refusal::Stale { age_s: (age / 1_000_000_000) as i64 });
    }
    if f.jitter_ns > STEP_MAX_JITTER_NS {
        return Err(Refusal::Scattered { jitter_ms: f.jitter_ns as f64 / 1e6 });
    }
    if f.offset_ns.unsigned_abs() > os::MAX_STEP_NS as u64 {
        return Err(Refusal::Implausible);
    }
    if f.offset_ns.abs() < STEP_MIN_NS {
        return Err(Refusal::AlreadyRight { offset_ms: f.offset_ns as f64 / 1e6 });
    }
    if f.offset_ns.abs() > STEP_CONFIRM_NS && !force {
        return Err(Refusal::NeedsConfirmation { offset_s: f.offset_ns as f64 / 1e9 });
    }
    Ok(f.offset_ns)
}

/// What a clock-setting attempt came to.
#[derive(Debug, Clone, PartialEq)]
pub enum StepOutcome {
    Stepped { by_ns: i64 },
    Failed(String),
}

#[derive(Default)]
struct Ntp {
    server: Option<NtpServer>,
    cfg: NtpServerConfig,
    error: Option<String>,
}

pub struct TimeHub {
    fix: Arc<Mutex<Option<Fix>>>,
    ntp: Mutex<Ntp>,
    step: Mutex<Option<Receiver<StepOutcome>>>,
    last_step: Mutex<Option<String>>,
    /// Bumped on every successful step, with the step stored beside it, so
    /// each radio can notice and forget measurements made against the clock
    /// as it was.
    steps: AtomicU64,
    last_by_ns: Mutex<i64>,
    authority: ClockAuthority,
    sync_daemon: Option<String>,
}

impl TimeHub {
    pub fn new() -> Arc<TimeHub> {
        Arc::new(TimeHub {
            fix: Arc::new(Mutex::new(None)),
            ntp: Mutex::new(Ntp::default()),
            step: Mutex::new(None),
            last_step: Mutex::new(None),
            steps: AtomicU64::new(0),
            last_by_ns: Mutex::new(0),
            authority: os::authority(),
            sync_daemon: os::sync_daemon(),
        })
    }

    pub fn authority(&self) -> ClockAuthority {
        self.authority
    }

    /// A radio's latest measurement.
    pub fn publish(&self, fix: Fix) {
        *self.fix.lock().unwrap() = Some(fix);
    }

    pub fn latest(&self) -> Option<Fix> {
        self.fix.lock().unwrap().clone()
    }

    /// What the NTP server answers with now.
    pub fn reference(&self) -> Reference {
        reference_from(self.fix.lock().unwrap().as_ref(), os::now_ns())
    }

    /// Starts, stops or rebinds the NTP server to match `cfg`.
    pub fn configure_ntp(&self, cfg: &NtpServerConfig) {
        let mut ntp = self.ntp.lock().unwrap();
        let same = ntp.cfg == *cfg && (ntp.server.is_some() == cfg.enabled);
        if same && ntp.error.is_none() {
            return;
        }
        // Close first: the new configuration is often the same port.
        ntp.server = None;
        ntp.error = None;
        ntp.cfg = cfg.clone();
        if !cfg.enabled {
            return;
        }
        let fix = self.fix.clone();
        match NtpServer::start(
            &cfg.addr(),
            Arc::new(move || reference_from(fix.lock().unwrap().as_ref(), os::now_ns())),
        ) {
            Ok(s) => {
                tracing::info!(addr = %s.addr(), "NTP server started");
                ntp.server = Some(s);
            }
            Err(e) => {
                let hint = if e.kind() == std::io::ErrorKind::PermissionDenied && cfg.port < 1024 {
                    " — ports below 1024 need privilege; try 1123"
                } else {
                    ""
                };
                tracing::warn!("NTP server on {}: {e}", cfg.addr());
                ntp.error = Some(format!("{e}{hint}"));
            }
        }
    }

    /// Sets the clock from the latest measurement, on a thread of its own:
    /// the administrator prompt can take as long as the operator likes. The
    /// result arrives through [`TimeHub::poll_step`].
    pub fn request_step(&self, force: bool) -> Result<(), Refusal> {
        let mut pending = self.step.lock().unwrap();
        if pending.is_some() {
            return Err(Refusal::Busy);
        }
        let offset = check_step(self.latest().as_ref(), os::now_ns(), force)?;
        let authority = self.authority;
        if authority == ClockAuthority::Unavailable {
            return Err(Refusal::NoAuthority);
        }
        let (tx, rx) = channel();
        std::thread::Builder::new()
            .name("sdroxide-clock-step".into())
            .spawn(move || {
                let r = match authority {
                    ClockAuthority::Direct => os::step_direct(offset).map_err(|e| e.to_string()),
                    _ => os::step_elevated(offset),
                };
                let _ = tx.send(match r {
                    Ok(()) => StepOutcome::Stepped { by_ns: offset },
                    Err(e) => StepOutcome::Failed(e),
                });
            })
            .map_err(|_| Refusal::Busy)?;
        *pending = Some(rx);
        Ok(())
    }

    /// The outcome of a step requested earlier, once it is known. Exactly one
    /// caller sees each outcome.
    pub fn poll_step(&self) -> Option<StepOutcome> {
        let mut pending = self.step.lock().unwrap();
        let out = match pending.as_ref()?.try_recv() {
            Ok(out) => out,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => {
                StepOutcome::Failed("the clock helper vanished".into())
            }
        };
        *pending = None;
        // Let go of it before touching anything else, so no two of this
        // hub's locks are ever held at once — two radios' engine threads call
        // in here and into `status` concurrently.
        drop(pending);
        let note = match &out {
            StepOutcome::Stepped { by_ns } => {
                // The clock moved under the measurement: it is now that much
                // less out, and was taken that much earlier by the new clock.
                if let Some(f) = self.fix.lock().unwrap().as_mut() {
                    f.offset_ns -= by_ns;
                    f.at_ns += i128::from(*by_ns);
                }
                *self.last_by_ns.lock().unwrap() = *by_ns;
                self.steps.fetch_add(1, Ordering::SeqCst);
                format!("clock moved {:+.3} s", *by_ns as f64 / 1e9)
            }
            StepOutcome::Failed(e) => e.clone(),
        };
        *self.last_step.lock().unwrap() = Some(note);
        Some(out)
    }

    /// How many times the clock has been stepped, and by how much the last
    /// time — for a radio to notice a step another one asked for.
    pub fn steps(&self) -> (u64, i64) {
        (self.steps.load(Ordering::SeqCst), *self.last_by_ns.lock().unwrap())
    }

    pub fn status(&self) -> ClockStatus {
        // One lock at a time, each released before the next is taken: see
        // `poll_step`. A struct literal would hold every guard it made to the
        // end of the expression.
        let synced = self.reference().synced;
        let last_step = self.last_step.lock().unwrap().clone();
        let step_pending = self.step.lock().unwrap().is_some();
        let ntp = {
            let ntp = self.ntp.lock().unwrap();
            NtpServerStatus {
                running: ntp.server.is_some(),
                addr: ntp.server.as_ref().map_or_else(|| ntp.cfg.addr(), |s| s.addr().to_string()),
                error: ntp.error.clone(),
                requests: ntp.server.as_ref().map_or(0, |s| s.requests()),
                synced: synced && ntp.server.is_some(),
            }
        };
        ClockStatus {
            authority: self.authority,
            last_step,
            step_pending,
            sync_daemon: self.sync_daemon.clone(),
            ntp,
        }
    }
}

fn reference_from(fix: Option<&Fix>, now_ns: i128) -> Reference {
    match fix {
        Some(f) if now_ns - f.at_ns < HOLDOVER_NS => {
            let age_s = ((now_ns - f.at_ns) as f64 / 1e9).max(0.0);
            Reference {
                synced: true,
                refid: f.refid,
                offset_ns: f.offset_ns,
                ref_ns: f.at_ns + i128::from(f.offset_ns),
                dispersion_s: f.jitter_ns as f64 / 1e9 + 0.001 + PHI * age_s,
            }
        }
        _ => Reference::UNSYNCED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i128 = 1_792_327_020_000_000_000;

    fn fix(offset_ms: f64) -> Fix {
        Fix {
            offset_ns: (offset_ms * 1e6) as i64,
            jitter_ns: 500_000,
            at_ns: NOW - 1_000_000_000,
            refid: *b"DCF\0",
        }
    }

    #[test]
    fn a_fresh_small_offset_is_stepped() {
        assert_eq!(check_step(Some(&fix(250.0)), NOW, false), Ok(250_000_000));
    }

    #[test]
    fn nothing_to_set_from_is_refused() {
        assert_eq!(check_step(None, NOW, true), Err(Refusal::NoFix));
        let old = Fix { at_ns: NOW - 600_000_000_000, ..fix(250.0) };
        assert_eq!(check_step(Some(&old), NOW, true), Err(Refusal::Stale { age_s: 600 }));
        let noisy = Fix { jitter_ns: 80_000_000, ..fix(250.0) };
        assert!(matches!(check_step(Some(&noisy), NOW, true), Err(Refusal::Scattered { .. })));
    }

    #[test]
    fn a_clock_already_right_is_left_alone() {
        assert!(matches!(
            check_step(Some(&fix(0.4)), NOW, true),
            Err(Refusal::AlreadyRight { .. })
        ));
    }

    #[test]
    fn a_large_step_waits_for_the_operator() {
        let f = fix(-3_600_000.0);
        assert!(matches!(check_step(Some(&f), NOW, false), Err(Refusal::NeedsConfirmation { .. })));
        assert_eq!(check_step(Some(&f), NOW, true), Ok(-3_600_000_000_000));
        let absurd = fix(500.0 * 86_400_000.0);
        assert_eq!(check_step(Some(&absurd), NOW, true), Err(Refusal::Implausible));
    }

    #[test]
    fn the_ntp_reference_ages_and_then_gives_up() {
        let f = fix(250.0);
        let r = reference_from(Some(&f), NOW);
        assert!(r.synced);
        assert_eq!(r.offset_ns, 250_000_000);
        let later = reference_from(Some(&f), NOW + 3600 * 1_000_000_000);
        assert!(later.dispersion_s > r.dispersion_s + 0.05);
        assert!(!reference_from(Some(&f), NOW + HOLDOVER_NS + 1).synced);
        assert!(!reference_from(None, NOW).synced);
    }
}
