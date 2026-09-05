//! Serial CAT control for non-SoapySDR rigs (Icom CI-V / Yaesu / Xiegu).
//!
//! NATIVE ONLY — links `serialport`; must never be a dependency of any
//! wasm-targeted crate. The rest of the app talks to it only through the
//! opaque [`CatHandle`] (a background serial thread), so no serial types leak
//! into the engine or UI.

/// Icom CI-V framing and parsing. Public because the Icom LAN backend tunnels
/// the same protocol over UDP and must not carry a second copy of it.
pub mod civ;
/// ELAD framing. Public for the same reason [`civ`] is: the native ELAD backend
/// tunnels these commands through the FDM-DUO's USB interface and must not
/// carry a second copy of them.
pub mod elad;
mod elecraft;
mod flrig;
mod kenwood;
mod qrplabs;
mod rigctld;
mod yaesu;

use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, TryRecvError};
use sdroxide_types::{
    CatConfig, CatFamily, CwKeying, DigiMode, LineState, Mode, ModeControl, Parity, PttMethod,
    SoundFormat, StopBits, TxTelemetry,
};
use tracing::{info, warn};

/// How long after sdroxide keys or unkeys the rig's own transmit state is
/// ignored. Covers a read that was already in flight across the edge, and the
/// rig's own turnaround, without being long enough to miss a real over.
const PTT_SETTLE: Duration = Duration::from_millis(600);

/// How long a rig that has stopped answering the transmit read is still
/// believed to be transmitting. A rig cannot be left keyed on this side of the
/// link by a control port that has gone quiet: the S-meter would stay blanked
/// and every key-down would be refused, with nothing on screen to say why.
const RIG_TX_MAX_AGE: Duration = Duration::from_millis(2000);

/// The fastest the meters are ever asked for, whatever the poll rate is set to.
///
/// Five times a second is already faster than anyone reads a needle, and every
/// reading is a frame on the control port. On the radios where that port shares
/// an internal USB bus with the sound card — an IC-7300 is a four-port hub with
/// a CP2102 and a PCM2901 behind it, both at full speed — frames the operator
/// cannot see the benefit of are paid for in audio the operator can hear the
/// loss of.
const METER_FLOOR: Duration = Duration::from_millis(200);

/// How often the dial and mode are still read on a rig that reports its own
/// changes unasked (CI-V transceive).
///
/// This is a safety net, not the thing that follows the knob: a broadcast that
/// went missing — a collision on a shared bus, a buffer that overflowed — would
/// otherwise leave the readout on a frequency the radio left seconds ago, and
/// nothing else would ever notice. Three seconds is short enough that a missed
/// broadcast is a glitch rather than a wrong dial, and long enough that the
/// poll it replaces is gone as a source of traffic.
const PUSHED_POLL_PERIOD: Duration = Duration::from_secs(3);

/// A scope that has sent nothing for this long has stopped. Sweeps arrive
/// several times a second, so this is a silence, not a slow sweep — and it is
/// long enough to sit through a band change or a menu opened on the radio.
/// Mirrors the LAN backend's watchdog, which exists for the same reasons: the
/// enables are fire-and-forget, and several ordinary things stop the sweeps.
const SCOPE_STALL: Duration = Duration::from_secs(3);
/// How long to wait before asking a rig again which antenna socket it is on,
/// and how many times. See where `antenna_probes_left` is seeded.
const ANTENNA_PROBE_RETRY: Duration = Duration::from_secs(2);
const ANTENNA_PROBE_RETRIES: u8 = 3;

/// How soon after a stall to ask the radio again, and the ceiling the interval
/// backs off to while it stays quiet. The enables are idempotent, but a rig
/// with no scope at all — or one whose CI-V USB port is still linked to
/// [REMOTE], so the waveform never reaches this side — must not be asked twice
/// a second forever on a link where every frame is bus time the audio pays for.
const SCOPE_RETRY: Duration = Duration::from_secs(2);
const SCOPE_RETRY_MAX: Duration = Duration::from_secs(30);

/// One finished sweep of the rig's own spectrum scope: what it covers, and its
/// amplitudes on the radio's 0..=160 scale.
#[derive(Debug, Clone, PartialEq)]
pub struct ScopeFrame {
    pub center_hz: f64,
    /// Full width, low edge to high edge.
    pub span_hz: f64,
    pub bins: Vec<u8>,
}

/// Whether this configuration streams the rig's scope over the serial link.
///
/// Three gates, all needed: the operator asked for it, the family is Icom (the
/// only serial dialect with `27 00`), and the link is fast enough to carry the
/// sweeps at all — below [`sdroxide_types::CAT_SCOPE_MIN_BAUD`] they would
/// bury every poll and PTT, so a slow link silently declines rather than
/// degrading the control channel. The caller surfaces the note for that case.
pub fn scope_active(cfg: &CatConfig) -> bool {
    cfg.scope
        && cfg.family == CatFamily::Icom
        && cfg.serial.baud >= sdroxide_types::CAT_SCOPE_MIN_BAUD
}

/// How many dial polls the mode rides along with one of.
///
/// The dial has to keep up with a hand on the VFO knob. The mode is a discrete
/// setting somebody changes a handful of times in an evening, and asking for it
/// at the same rate spends a frame every time on an answer that is the same one
/// it was last time.
const MODE_POLL_EVERY: u32 = 4;

/// The shortest the mode is ever asked for, whatever [`MODE_POLL_EVERY`] works
/// out to. A fast poll rate is somebody buying a responsive dial; it is not a
/// reason to interrogate a setting that has not moved.
const MODE_POLL_FLOOR: Duration = Duration::from_secs(1);

/// How often the rig is asked what *mode* it is in, at a given poll rate.
///
/// Never slower than the dial poll's own ceiling: at the bottom of the range
/// the two meet and the split stops mattering, which is the right place for it
/// to stop.
fn mode_poll_period(cfg: &CatConfig) -> Duration {
    (poll_period(cfg) * MODE_POLL_EVERY).clamp(MODE_POLL_FLOOR, Duration::from_secs(5))
}

/// How often the rig's meters are asked for while receiving, at a given poll
/// rate — the cadence the serial thread runs. Only the receive side: the
/// transmit meter is the SWR, which stays at [`METER_FLOOR`] whatever the
/// setting says.
fn meter_period(cfg: &CatConfig) -> Duration {
    poll_period(cfg).max(METER_FLOOR)
}

/// How long one of the rig's S-meter readings stands in for the next.
///
/// Three of the rig's own answers, and never less than a second and a half: an
/// ordinary gap between them is covered, and a link that has gone quiet is not
/// — a needle left standing at the last thing a dead port said is worse than no
/// needle. It has to follow the poll rate, because at a low one the gap between
/// two honest answers is longer than any fixed window would allow for and the
/// meter would blank between every pair of them; the floor is there so that
/// raising the poll rate cannot make the needle *more* twitchy than it was.
pub fn signal_max_age(cfg: &CatConfig) -> Duration {
    (meter_period(cfg) * 3).max(Duration::from_millis(1500))
}

/// How often the rig is asked what its dial and mode are, at a given poll rate.
///
/// Clamped at both ends: below 0.2 Hz the readout stops being a readout, and
/// above ~33 Hz there is no room left between frames anyway (see `FRAME_GAP`).
fn poll_period(cfg: &CatConfig) -> Duration {
    Duration::from_secs_f32((1.0 / cfg.poll_hz.max(0.2)).min(5.0))
}

/// A change the rig reported (external dial/mode movement) or that we read back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CatUpdate {
    Freq(f64),
    Mode(Mode),
    /// TX SWR reading (routed to the telemetry channel, not the control channel).
    Swr(f32),
    /// TX ALC reading, 0.0..=1.0 of the rig's own meter. Routed alongside
    /// [`CatUpdate::Swr`]. Distinct from the engine's own drive measurement,
    /// which is what SDRoxide SENDS rather than what the rig does about it.
    Alc(f32),
    /// TX power-output reading as a `0.0..=1.0` fraction of the rig's full
    /// scale (routed to the telemetry channel, like [`CatUpdate::Swr`]).
    Po(f32),
    /// TX forward power the rig has actually *measured*, in watts (routed
    /// alongside [`CatUpdate::Po`]).
    ///
    /// Kept apart from [`CatUpdate::Po`] because the two are different claims,
    /// the same way [`TxTelemetry::fwd_w`] and [`TxTelemetry::po`] are: a
    /// needle position on the rig's own scale is not a wattage, and turning one
    /// into the other invents a calibration the radio never offered. Only a
    /// family whose meter answers in watts sends this — QRP Labs' `PC`, which
    /// on every other ASCII family here is the power *control* instead.
    FwdW(f32),
    /// RX S-meter reading in dBm, from the rig's own meter (routed to the
    /// signal channel, not the control channel).
    Signal(f32),
    /// The transmit power the rig is set to, as a `0..1` fraction of its
    /// maximum. Read once when the port opens, so the panel's Drive slider ends
    /// up where the radio's own power control already is.
    Power(f32),
    /// The squelch threshold the rig is set to, as a `0..1` fraction of its
    /// own scale. Read once when the port opens, and adopted for the same
    /// reason [`Self::Power`] is: the operator set it at the radio.
    Squelch(f32),
    /// Which antenna socket the rig says it is receiving on, by the name that
    /// family gives the port (see [`Protocol::antennas`]).
    ///
    /// Read once when the port opens and never polled, so this is the radio's
    /// own setting being *adopted* — the same shape as [`Self::Power`], and for
    /// the same reason: the operator set it at the front panel, it survived the
    /// power cycle, and imposing a remembered one on top would move an antenna
    /// relay nobody asked to move.
    Antenna(&'static str),
    /// Whether the rig's separate *receiving* antenna connector is switched
    /// into the receive path.
    ///
    /// Only ever the radio reporting itself: read when the port opens and
    /// again after anything the radio recalls per band, and adopted. sdroxide
    /// writes it exactly when the operator clicks, because writing a
    /// receive-only input nobody asked about takes an aerial out of use with
    /// nothing on screen to say so.
    RxAntenna(bool),
    /// The rig is transmitting under its *own* control — a hand on the mic
    /// button, a foot switch, its VOX, its keyer. `true` is keyed.
    ///
    /// Only ever an over sdroxide did not ask for: an over we keyed is one the
    /// engine already knows about, and reporting it back would be an echo. See
    /// the suppression around `ptt_ours` in the serial thread.
    Ptt(bool),
}

/// Enumerate serial ports for the settings UI. USB-style ports (ttyACM/ttyUSB,
/// where CAT rigs like the X6100 appear) are listed first; the many legacy
/// `/dev/ttyS*` entries — which the non-libudev sysfs scan can't filter to
/// only present ones — sort to the end.
pub fn available_ports() -> Vec<String> {
    let mut ports: Vec<String> = serialport::available_ports()
        .map(|ports| ports.into_iter().map(|p| p.port_name).collect())
        .unwrap_or_default();
    let rank = |p: &str| -> u8 {
        if p.contains("ttyACM") || p.contains("ttyUSB") {
            0
        } else if p.contains("ttyS") {
            2
        } else {
            1
        }
    };
    ports.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| a.cmp(b)));
    ports
}

/// Per-family framing. `parse` consumes complete frames from a rolling buffer.
trait Protocol: Send {
    fn set_freq(&mut self, hz: f64) -> Vec<u8>;
    fn set_mode(&mut self, m: Mode) -> Vec<u8>;
    /// CAT-command PTT (only used when `PttMethod::Cat`).
    fn ptt(&self, on: bool) -> Vec<u8>;
    /// Frames that request the rig's current freq + mode — the whole poll.
    fn poll_requests(&self) -> Vec<Vec<u8>>;
    /// The part of [`Self::poll_requests`] that asks for the dial alone, sent
    /// on the polls the mode does not ride along with (see [`MODE_POLL_EVERY`]).
    ///
    /// Defaults to the whole poll, so a family added without a thought spared
    /// for this keeps asking for everything every time — the behaviour every
    /// family had before the split existed.
    fn dial_requests(&self) -> Vec<Vec<u8>> {
        self.poll_requests()
    }
    /// Frames requesting TX telemetry (SWR / power), polled only while keyed.
    /// Empty for families with no such read.
    fn tx_telemetry_requests(&self) -> Vec<Vec<u8>> {
        Vec::new()
    }
    /// Frames requesting the rig's S-meter, polled only while receiving.
    ///
    /// A CAT rig sends us audio it has already demodulated, filtered and
    /// levelled — there is no signal left on this side of the sound card to
    /// measure — so its own meter is the only S-meter the operator can be
    /// shown. Empty for families with no such read, which fall back to the
    /// level of the audio itself.
    fn rx_telemetry_requests(&self) -> Vec<Vec<u8>> {
        Vec::new()
    }
    /// Frames asking whether the rig is transmitting, polled while sdroxide is
    /// *not* the one keying it.
    ///
    /// This is how an over the operator started at the radio — mic button, foot
    /// switch, VOX, the rig's own keyer — becomes something sdroxide knows
    /// about, so the meter can show the SWR of it and nothing here tries to key
    /// on top of it. Empty for families with no such read, which simply never
    /// learn about an over they were not asked for.
    fn tx_state_requests(&self) -> Vec<Vec<u8>> {
        Vec::new()
    }

    /// Frames written once when the link opens, before anything else — for a
    /// profile that must ask what is on the other end before its other frames
    /// mean anything: the model, the power scale, the mode names. Empty — the
    /// default — for every family that needs no introduction.
    fn open_requests(&self) -> Vec<Vec<u8>> {
        Vec::new()
    }

    /// Frames that ask the rig to run its spectrum scope and stream the sweeps
    /// here — written when the link opens, and again by the watchdog whenever
    /// the sweeps stop. All idempotent, for exactly that reason. Empty — the
    /// default — for every family without a streamable scope, which is every
    /// family but Icom.
    fn scope_requests(&self) -> Vec<Vec<u8>> {
        Vec::new()
    }
    /// The sweep [`Protocol::parse`] most recently completed, if one has since
    /// the last call. Sweeps carry no sequence and only the newest can be
    /// drawn, so this is a take of the latest rather than a queue.
    fn take_scope_sweep(&mut self) -> Option<ScopeFrame> {
        None
    }

    /// Frames that switch the rig's *own* RIT, XIT and split off, sent once
    /// when the port opens. sdroxide carries all three on the dial (the rig's
    /// dial is the only frequency control a CAT rig gives us), so anything the
    /// radio is still holding would add to ours unseen. Empty for families with
    /// no such command.
    fn clear_offsets(&self) -> Vec<Vec<u8>> {
        Vec::new()
    }

    /// Frames that put the rig's own *repeater shift* back to simplex, sent
    /// whenever the dial moves to another band.
    ///
    /// The fourth thing that moves a transmit frequency, and the one that
    /// [`Protocol::clear_offsets`] cannot cover: RIT, XIT and split stay where
    /// they are put, but a radio with band stacking registers restores the
    /// duplex setting the band was last left on as soon as the dial crosses
    /// into it. On an IC-9700 that means 70 cm and 23 cm come up on DUP−, and
    /// an operator who wanted simplex had no way to say so — sdroxide carries
    /// the shift on the dial itself, so the radio's own would be added to it
    /// (issue #192). Empty for families with no such command.
    fn clear_duplex(&self) -> Vec<Vec<u8>> {
        Vec::new()
    }

    /// Longest run of CW text this rig's keyer takes in one go, or 0 when it
    /// cannot be keyed from text at all. The caller sends no more than this per
    /// [`Protocol::send_cw`], so nothing has to be truncated on the way out.
    fn cw_chunk_len(&self) -> usize {
        0
    }
    /// Hand `text` to the rig's own keyer. The rig keys itself — this is *not*
    /// wrapped in PTT, and must not be: a transmitter already keyed by CAT is
    /// one the keyer cannot key.
    fn send_cw(&mut self, _text: &str) -> Vec<Vec<u8>> {
        Vec::new()
    }
    /// Stop a message the rig is part way through sending. Empty for families
    /// with no such command — there, an abort can only stop the *next* chunk.
    fn abort_cw(&mut self) -> Vec<Vec<u8>> {
        Vec::new()
    }
    /// Set the rig keyer's speed. The rig keys at its own speed, so the panel's
    /// WPM has no effect on the air until it has been sent here.
    fn set_cw_wpm(&mut self, _wpm: f32) -> Vec<Vec<u8>> {
        Vec::new()
    }

    /// Set the rig's own receive filter, from audio-band edges in Hz either
    /// side of the dial.
    ///
    /// The mode is passed because no family expresses a filter without one: the
    /// same 500 Hz is one index in CW and a different one in SSB, and some rigs
    /// take a width in CW but a pair of slope frequencies in SSB. What each
    /// family can express varies too, so a protocol that cannot say this
    /// particular passband on this particular rig returns nothing — leaving the
    /// radio's filter where the operator last put it, which is a far better
    /// answer than a guess at an index.
    fn set_filter(&mut self, _mode: Mode, _lo_hz: f32, _hi_hz: f32) -> Vec<Vec<u8>> {
        Vec::new()
    }
    /// Whether [`Protocol::set_filter`] reaches this family at all.
    fn commands_filter(&self) -> bool {
        false
    }

    /// Set the transmitter's output power, as a `0..1` fraction of what the rig
    /// can do.
    ///
    /// This is the only transmit level a CAT rig has that means anything in
    /// *every* mode. The level of the audio we put into its sound card is not
    /// one: a rig in CW keys its own transmitter from its own keyer and never
    /// looks at the sound card, and one asked to TUNE holds a carrier the audio
    /// has no part in either. Empty for families with no such command, where
    /// there is nothing but the audio.
    fn set_power(&mut self, _frac: f32) -> Vec<Vec<u8>> {
        Vec::new()
    }
    /// Frames asking what the rig's power is set to, sent once when the port
    /// opens so the panel can *adopt* the rig's own level instead of imposing a
    /// remembered one on it. Empty for families with no such read.
    fn read_power(&self) -> Vec<Vec<u8>> {
        Vec::new()
    }
    /// Whether [`Protocol::set_power`] reaches this family at all.
    fn commands_power(&self) -> bool {
        false
    }

    /// Set the receiver's squelch threshold, as a `0..1` fraction of the rig's
    /// own scale — `0` open, `1` closed, the way the front-panel knob reads.
    ///
    /// On a rig that hands us demodulated audio this is the *only* squelch
    /// there is. What reaches the sound card has already been through the
    /// radio's gate, so a threshold applied on this side can close further on
    /// what got through but can never open what was shut out (issue #192).
    /// Empty for families with no such command.
    fn set_squelch(&mut self, _frac: f32) -> Vec<Vec<u8>> {
        Vec::new()
    }
    /// Frames asking where the rig's squelch is set, sent once when the port
    /// opens so the control *adopts* the radio's own level — the same shape,
    /// and the same reasoning, as [`Protocol::read_power`].
    fn read_squelch(&self) -> Vec<Vec<u8>> {
        Vec::new()
    }

    /// Frames asking whether the rig's break-in is on, sent once when the port
    /// opens.
    ///
    /// Not adopted like the power and the squelch above — this one is *needed*.
    /// Every family here that keys the rig's own keyer has to have break-in on
    /// or the message is accepted and never transmitted, and Yaesu and Kenwood
    /// simply assert it with each message. On CI-V the setting has three
    /// positions rather than two, and knocking an operator who runs QSK down to
    /// semi break-in every time they send is worth one frame at open to avoid.
    /// Empty for families with no such read, which is every other one.
    fn read_break_in(&self) -> Vec<Vec<u8>> {
        Vec::new()
    }
    /// Whether [`Protocol::set_squelch`] reaches this family at all.
    fn commands_squelch(&self) -> bool {
        false
    }

    /// The antenna sockets this family can switch the *receiver* between, named
    /// as the rest of sdroxide names ports. Empty — the default — for the
    /// families with no such command, which is all of them but ELAD.
    ///
    /// Receive only, and deliberately: the one rig here with two sockets
    /// transmits out of the same one either way, so there is no transmit port
    /// to choose.
    fn antennas(&self) -> &'static [&'static str] {
        &[]
    }
    /// Put the receiver on `name`, which is one of [`Protocol::antennas`].
    fn set_antenna(&mut self, _name: &str) -> Vec<Vec<u8>> {
        Vec::new()
    }
    /// Frames asking which socket the rig is on, sent when the port opens — and
    /// again whenever the radio may have moved it behind us — so the panel
    /// adopts the radio's own setting (see [`CatUpdate::Antenna`]). Empty for
    /// families with no such read.
    fn read_antenna(&self) -> Vec<Vec<u8>> {
        Vec::new()
    }
    /// Frames putting the rig's separate *receiving* antenna connector into
    /// the receive path, or taking it out. Empty where the family — or this
    /// radio — has none.
    ///
    /// Whether there is such a connector at all is learned rather than claimed,
    /// and not from here: an Icom's antenna reply carries the flag behind the
    /// socket only where the connector exists, so the shape of the answer is
    /// what says so, and it reaches the caller as [`CatUpdate::RxAntenna`]. The
    /// transmit aerial is not affected either way; that is the whole point of
    /// the thing.
    ///
    /// Sent only when the operator asks. The radio recalls this per band on its
    /// own, so everywhere else it is read and adopted rather than asserted:
    /// writing a receive-only input nobody asked about takes an aerial out of
    /// use with nothing on screen to say so.
    fn set_rx_antenna(&mut self, _on: bool) -> Vec<Vec<u8>> {
        Vec::new()
    }
    /// Frames that switch the *radio* off, or back on again — its own power
    /// switch, over the control link (issue #239). `baud` is the port's line
    /// rate, which a power-*on* needs: a sleeping radio's control receiver has
    /// to be woken with a run of bytes long enough to cover the time it takes
    /// to come up, and how many bytes that is depends on how fast they go out.
    ///
    /// Empty — the default — for a family with no such command, and the caller
    /// publishes no power switch for it (see
    /// [`sdroxide_types::DeviceCaps::commands_rig_power`]).
    fn set_rig_power(&mut self, _on: bool, _baud: u32) -> Vec<Vec<u8>> {
        Vec::new()
    }
    /// Whether [`Protocol::set_rig_power`] reaches this family at all. Asked
    /// before a frame has gone out, so it cannot be inferred from the above.
    fn commands_rig_power(&self) -> bool {
        false
    }

    /// Whether [`Protocol::antennas`] is a *claim about the family* or a
    /// *question for the radio*.
    ///
    /// An ELAD FDM-DUO has two sockets and says so from the model name alone.
    /// A CI-V rig does not: the same dialect covers an IC-7610 with a selector
    /// and an IC-705 with one connector, and the only thing that tells them
    /// apart is whether the rig answers the antenna read at all. A family that
    /// answers true here publishes no sockets until it has (see
    /// [`CatHandle::antennas`]), so a radio with one antenna shows no antenna
    /// control rather than a switch that does nothing.
    fn antennas_probed(&self) -> bool {
        false
    }

    /// Whether a mode change can move this rig's *dial*.
    ///
    /// Most radios keep the VFO where it is when the mode changes. Some can be
    /// set not to: they shift the displayed frequency by the CW pitch when
    /// entering or leaving CW, so that zero-beat stays zero-beat, and the
    /// operator's 14.050 becomes 14.050 6 without anything having asked for it.
    /// A protocol that answers true has the frequency re-asserted behind every
    /// mode command it writes — which is what the radio's own documentation
    /// tells applications to do.
    fn mode_moves_dial(&self) -> bool {
        false
    }

    /// True when [`Protocol::parse`] learned something about the rig's framing
    /// that invalidates frames written before it — the Yaesu frequency-field
    /// width. Reported once, to whoever can re-issue the frame.
    fn reframed(&mut self) -> bool {
        false
    }

    /// True when [`Protocol::parse`] saw the rig refuse a command since the
    /// last call. Which command it refused is not in the answer — CI-V's "NG"
    /// carries nothing but itself — so this is only worth reporting where the
    /// caller knows what it just sent. Reported once, then cleared.
    fn refused(&mut self) -> bool {
        false
    }

    /// True while this rig is reporting its own dial and mode changes unasked —
    /// Icom's "CI-V transceive", which every other family here lacks.
    ///
    /// Deliberately proven in both directions rather than assumed or timed out.
    /// It is a setting in the radio's own menu, so nothing is claimed until a
    /// broadcast has actually arrived and parsed, and until then the poll runs
    /// exactly as it always did.
    ///
    /// A timeout is the wrong way to withdraw the claim: at idle — precisely
    /// when the traffic is worth saving — nothing changes, so no broadcasts
    /// arrive to keep it alive, and a timeout would put the poll back on every
    /// rig that simply had nothing to report. What actually disproves it is a
    /// *change* that arrived without one: the safety-net poll answering with a
    /// dial or a mode that nobody here commanded and no broadcast announced.
    /// The rig moved and did not say so, which is the whole of the question.
    fn pushes_updates(&self) -> bool {
        false
    }

    fn parse(&mut self, buf: &mut Vec<u8>) -> Vec<CatUpdate>;

    /// Called with every protocol-generated frame the moment it is actually
    /// written to the link — and only then. What a profile *generates* is not
    /// what goes out: `set_mode` is also called purely to compute a frame for
    /// comparison (see [`ModeMemory`]), and a frame the dedup discards was
    /// never sent. A profile whose replies do not name the request they answer
    /// — flrig's XML-RPC — correlates on this instead of on the reply.
    fn wrote(&mut self, _frame: &[u8]) {}

    /// How many requests are on the wire with no answer back yet, for a family
    /// that can only tell its answers apart by the order they arrive in.
    ///
    /// Where this is not zero the driver keeps exactly one request in flight:
    /// it waits for the answer before writing anything else (see
    /// [`await_answers`]). That is not politeness — it is the only way to
    /// survive a far end that reads its socket in one go and answers only the
    /// first request it finds there, throwing the rest away unanswered. One
    /// request lost that way costs no reading at all; it costs the correlation
    /// itself, and every answer after it is read against the wrong question
    /// until the connection is remade.
    ///
    /// Zero — the default — for every family whose replies say what they
    /// answer: an Icom names the command, rigctld echoes the line it was given,
    /// and a reply lost there is one missing reading and nothing more.
    fn in_flight(&self) -> usize {
        0
    }

    /// A reply arrived that cannot be an answer to the request it was read
    /// against — a frequency read answered with a word, a transmit read
    /// answered with a frequency. Only a family correlating by order can find
    /// itself here, and once it has, nothing it reads means anything: the
    /// caller drops the link, and the fresh one starts the correlation again.
    /// Reported once, like [`Self::refused`].
    fn desynced(&mut self) -> bool {
        false
    }

    /// The link has (re)opened. Anything a profile holds that describes bytes
    /// in flight — a half-read reply, a queue of requests awaiting answers —
    /// is about a connection that no longer exists and must be dropped here.
    /// The protocol object itself outlives reconnects on purpose (what it has
    /// *learned* about the rig is still true); this is only about the wire.
    fn link_opened(&mut self) {}
}

/// CI-V protocol (Icom + Xiegu). `radio` is the CI-V transceiver address.
struct Civ {
    radio: u8,
    /// The `1A` sub-command that switches DATA mode on this model, or `None`
    /// where the model has none — see [`civ::set_mode_frames`].
    data_sub: Option<u8>,
    /// The rig answered "NG" since this was last read (see
    /// [`Protocol::refused`]).
    nak: bool,
    /// The rig has broadcast a change nobody asked for — its transceive setting
    /// is on (see [`Protocol::pushes_updates`]).
    pushed: bool,
    /// When the last broadcast arrived, so a polled answer that disagrees with
    /// [`Self::seen_freq`] can be told from one that merely overtook a knob
    /// still being turned. Turning the VFO produces a broadcast per step, and a
    /// read issued in the middle of that comes back describing a dial that has
    /// already moved on — which is transceive working, not failing.
    last_push: Option<Instant>,
    /// The dial and mode this end has reason to believe the rig is on: whatever
    /// was last broadcast, answered, or commanded from here. A polled answer
    /// that disagrees with these is a change nobody was told about.
    seen_freq: Option<f64>,
    seen_mode: Option<u8>,
    /// Whether this session streams the rig's scope, and the half-span to
    /// command it to (`None` leaves the radio's own span alone). See
    /// [`scope_active`] for the gates.
    scope: bool,
    scope_half_span: Option<f64>,
    /// Reassembles the `27 00` sweeps, which arrive in ~11 fragments over
    /// serial where the LAN delivers them whole.
    scope_assembler: civ::ScopeAssembler,
    /// The newest finished sweep, until [`Protocol::take_scope_sweep`] takes it.
    scope_finished: Option<ScopeFrame>,
    /// What the rig last said its break-in was set to, from the read at open
    /// and from our own writes. `None` until it has answered — a model that
    /// NAKs the read, or a reply still in flight.
    break_in: Option<civ::BreakIn>,
    /// How many aerial sockets this model's selector has — from
    /// `IcomModel::antenna_sockets`, because CI-V cannot be asked.
    sockets: usize,
    /// The socket the rig last said it was on, so the receiving antenna can be
    /// written without moving it. `None` until it has answered.
    socket: Option<u8>,
    /// The receiving antenna's in/out setting, from the rig's own replies and
    /// from our own writes. `None` on a radio whose reply was the socket
    /// alone — which is how it says it has no such connector.
    rx_ant: Option<bool>,
}

/// How long after a broadcast a disagreeing polled answer is put down to the
/// two having crossed on the wire rather than to transceive being off.
///
/// One round trip is all it takes, and a knob being turned produces broadcasts
/// far faster than that; a second is generous in the direction that costs
/// nothing, since guessing wrong here only means polling normally for a while.
const PUSH_CROSSED_WIRES: Duration = Duration::from_secs(1);

impl Civ {
    fn new(radio: u8, data_sub: Option<u8>, sockets: usize) -> Civ {
        Civ {
            radio,
            data_sub,
            nak: false,
            pushed: false,
            last_push: None,
            seen_freq: None,
            seen_mode: None,
            scope: false,
            scope_half_span: None,
            scope_assembler: civ::ScopeAssembler::default(),
            scope_finished: None,
            break_in: None,
            sockets,
            socket: None,
            rx_ant: None,
        }
    }

    /// Stream the rig's scope this session, sweeping `half_span` either side of
    /// the dial (`None` keeps whatever span the radio's own screen is on).
    fn with_scope(mut self, half_span: Option<f64>) -> Civ {
        self.scope = true;
        self.scope_half_span = half_span;
        self
    }

    /// Whether the rig turning up at `now`, where we believed `seen`, disproves
    /// the transceive claim.
    ///
    /// Only a *polled* answer can: a broadcast is the rig reporting itself,
    /// which is the claim holding. Only a disagreeing one can: an answer that
    /// matches what we already believed says nothing either way, and the safety
    /// net produces one of those every few seconds. And only outside
    /// [`PUSH_CROSSED_WIRES`], which is where a read and a broadcast that
    /// crossed on the wire live.
    fn moved_silently<T: PartialEq>(&self, seen: &Option<T>, now: &T, broadcast: bool) -> bool {
        !broadcast
            && self.pushed
            && seen.as_ref().is_some_and(|was| was != now)
            && self.last_push.is_none_or(|at| at.elapsed() > PUSH_CROSSED_WIRES)
    }
}

impl Protocol for Civ {
    fn set_freq(&mut self, hz: f64) -> Vec<u8> {
        // Where we have just put the rig is somewhere it got to without a
        // broadcast, and legitimately so — recording it here is what stops our
        // own tuning reading as the radio moving behind our back.
        self.seen_freq = Some(hz.round());
        civ::set_freq_frame(self.radio, hz)
    }
    fn set_mode(&mut self, m: Mode) -> Vec<u8> {
        self.seen_mode = Some(civ::mode_to_civ(m));
        civ::set_mode_frames(self.radio, m, self.data_sub).concat()
    }
    fn ptt(&self, on: bool) -> Vec<u8> {
        civ::ptt_frame(self.radio, on)
    }
    fn poll_requests(&self) -> Vec<Vec<u8>> {
        vec![civ::read_freq_frame(self.radio), civ::read_mode_frame(self.radio)]
    }
    fn dial_requests(&self) -> Vec<Vec<u8>> {
        vec![civ::read_freq_frame(self.radio)]
    }
    fn tx_telemetry_requests(&self) -> Vec<Vec<u8>> {
        // Three reads per telemetry tick. All are answered on the same command
        // and are told apart by their sub-command byte on the way back in.
        vec![
            civ::read_swr_frame(self.radio),
            civ::read_alc_frame(self.radio),
            civ::read_po_frame(self.radio),
        ]
    }
    fn tx_state_requests(&self) -> Vec<Vec<u8>> {
        vec![civ::read_ptt_frame(self.radio)]
    }
    fn rx_telemetry_requests(&self) -> Vec<Vec<u8>> {
        vec![civ::read_smeter_frame(self.radio)]
    }
    fn clear_offsets(&self) -> Vec<Vec<u8>> {
        civ::clear_offsets_frames(self.radio)
    }
    fn clear_duplex(&self) -> Vec<Vec<u8>> {
        vec![civ::simplex_frame(self.radio)]
    }
    fn cw_chunk_len(&self) -> usize {
        civ::CW_MAX
    }
    fn send_cw(&mut self, text: &str) -> Vec<Vec<u8>> {
        let Some(msg) = civ::send_cw_frame(self.radio, text) else {
            return Vec::new();
        };
        let mut frames = Vec::new();
        // Break-in has to be on or the message is taken and never transmitted.
        // The rig's own reference is explicit about it: a message sent from a
        // PC goes out only while `[TRANSMIT]` is on, an external TX switch is
        // closed, or break-in is on — and an operator working the radio over a
        // network can reach none of the first two. That was issue #282, where
        // CW from the panel did nothing at all on an IC-9700.
        //
        // Semi rather than full, because that is what a message sent from a
        // computer wants — but never *down* from full: an operator who runs QSK
        // has said what they want, and the read at open is there so this end
        // knows. Idempotent otherwise, so it rides with every chunk the way
        // Yaesu's `BI1` and Kenwood's `VX1` do.
        if self.break_in != Some(civ::BreakIn::Full) {
            frames.push(civ::set_break_in_frame(self.radio, civ::BreakIn::Semi));
            self.break_in = Some(civ::BreakIn::Semi);
        }
        frames.push(msg);
        frames
    }
    fn abort_cw(&mut self) -> Vec<Vec<u8>> {
        vec![civ::stop_cw_frame(self.radio)]
    }
    fn set_cw_wpm(&mut self, wpm: f32) -> Vec<Vec<u8>> {
        vec![civ::keyer_speed_frame(self.radio, wpm)]
    }
    fn set_filter(&mut self, mode: Mode, lo_hz: f32, hi_hz: f32) -> Vec<Vec<u8>> {
        civ::set_filter_frame(self.radio, mode, lo_hz, hi_hz).into_iter().collect()
    }
    fn commands_filter(&self) -> bool {
        true
    }
    fn set_power(&mut self, frac: f32) -> Vec<Vec<u8>> {
        vec![civ::set_power_frame(self.radio, frac)]
    }
    fn read_power(&self) -> Vec<Vec<u8>> {
        vec![civ::read_power_frame(self.radio)]
    }
    fn commands_power(&self) -> bool {
        true
    }
    fn set_squelch(&mut self, frac: f32) -> Vec<Vec<u8>> {
        vec![civ::set_squelch_frame(self.radio, frac)]
    }
    fn read_squelch(&self) -> Vec<Vec<u8>> {
        vec![civ::read_squelch_frame(self.radio)]
    }
    fn read_break_in(&self) -> Vec<Vec<u8>> {
        vec![civ::read_break_in_frame(self.radio)]
    }
    fn commands_squelch(&self) -> bool {
        true
    }
    /// ANT1/ANT2, and only once the radio has answered the read below — see
    /// [`Protocol::antennas_probed`] and [`civ::ANTENNAS`].
    ///
    /// Empty on a model with one socket, even though such a radio may answer
    /// the read: one socket is not a choice, and the second position of a chip
    /// offered for it would send `12 01 …`, which an IC-7300MK2 rejects. What
    /// that radio answers the read *for* is the receiving antenna — see
    /// [`Protocol::rx_antenna`].
    fn antennas(&self) -> &'static [&'static str] {
        if self.sockets > 1 { &civ::ANTENNAS[..self.sockets.min(civ::ANTENNAS.len())] } else { &[] }
    }
    fn antennas_probed(&self) -> bool {
        true
    }
    /// `18 00` / `18 01`, with the wake-up run in front of a power-on — see
    /// [`civ::power_frames`].
    fn set_rig_power(&mut self, on: bool, baud: u32) -> Vec<Vec<u8>> {
        civ::power_frames(self.radio, on, civ::wake_bytes_for(baud))
    }
    fn commands_rig_power(&self) -> bool {
        true
    }
    /// The socket, carrying the receiving antenna's setting out with it: the
    /// same command writes both, so a zero here would switch the operator's
    /// receive aerial out of circuit every time the socket was asserted. The
    /// caller re-reads afterwards, because on an IC-7610 the flag belongs to
    /// the socket it arrives with rather than to the radio.
    fn set_antenna(&mut self, name: &str) -> Vec<Vec<u8>> {
        // Against this model's own list, not the whole of `civ::ANTENNAS`: a
        // radio with one socket has no socket to be put on, and `12 01 …` is
        // a frame an IC-7300MK2 rejects.
        if !self.antennas().iter().any(|a| a.eq_ignore_ascii_case(name)) {
            return Vec::new();
        }
        // `self.rx_ant` says both things at once: whether the receive aerial is
        // in circuit, and — by being `None` — whether this radio's `12` has a
        // byte for it at all. A receiver has not, and NAKs the longer form.
        civ::set_antenna_frame(self.radio, name, self.rx_ant).into_iter().collect()
    }
    fn read_antenna(&self) -> Vec<Vec<u8>> {
        vec![civ::read_antenna_frame(self.radio)]
    }
    /// The same command with the socket held still.
    ///
    /// Nothing goes out until the radio has said two things: which socket it is
    /// on — guessing `00` would move the aerial on a two-socket rig sitting on
    /// ANT2 — and that it has a receiving antenna at all, which it says by
    /// answering the read with the flag behind the socket.
    fn set_rx_antenna(&mut self, on: bool) -> Vec<Vec<u8>> {
        let (Some(socket), true) = (self.socket, self.rx_ant.is_some()) else {
            return Vec::new();
        };
        self.rx_ant = Some(on);
        vec![civ::antenna_frame(self.radio, socket, Some(on))]
    }
    /// The same enable sequence the LAN backend sends, because it is the same
    /// scope: run it, stream it here, and — when a span is chosen — put it in
    /// centre mode so it follows the dial, since a scope left in a fixed mode
    /// ignores the span command and sits on a slice of band the dial is not in.
    fn scope_requests(&self) -> Vec<Vec<u8>> {
        if !self.scope {
            return Vec::new();
        }
        let mut out =
            vec![civ::scope_on_frame(self.radio, true), civ::scope_output_frame(self.radio, true)];
        if let Some(half) = self.scope_half_span {
            out.push(civ::scope_mode_frame(self.radio, civ::ScopeMode::Center));
            out.push(civ::scope_span_frame(self.radio, half));
        }
        out
    }
    fn take_scope_sweep(&mut self) -> Option<ScopeFrame> {
        self.scope_finished.take()
    }
    fn refused(&mut self) -> bool {
        std::mem::take(&mut self.nak)
    }
    fn pushes_updates(&self) -> bool {
        self.pushed
    }
    fn parse(&mut self, buf: &mut Vec<u8>) -> Vec<CatUpdate> {
        let mut out = Vec::new();
        for reply in civ::parse_frames(buf) {
            // Ignore our own echoes (controller-sourced frames).
            if reply.from == civ::CONTROLLER_ADDR {
                continue;
            }
            match reply.cmd {
                // The transceive broadcasts (`0x00` frequency, `0x01` mode):
                // what a rig with that setting on sends, unasked, the moment
                // its dial or its mode moves. Same payloads as the answers to
                // the reads below, addressed to nobody in particular.
                //
                // These are only ever *from* the radio. `0x00` and `0x01` are
                // the controller's "set frequency" and "set mode" — but this
                // end sets with `0x05`/`0x06`, and anything from the controller
                // address was already skipped above as our own echo. The LAN
                // backend has folded the same pair together since it was
                // written (`IcomNetSource::on_reply`); this is the serial side
                // of the link catching up with it.
                0x00 | 0x03 => {
                    if let Some(hz) = civ::decode_freq(&reply.data) {
                        let broadcast = reply.cmd == 0x00;
                        self.pushed &= !self.moved_silently(&self.seen_freq, &hz, broadcast);
                        if broadcast {
                            self.pushed = true;
                            self.last_push = Some(Instant::now());
                        }
                        self.seen_freq = Some(hz);
                        out.push(CatUpdate::Freq(hz));
                    }
                }
                0x01 | 0x04 => {
                    if let Some(&b) = reply.data.first() {
                        let broadcast = reply.cmd == 0x01;
                        // Judged on the mode *byte*, not the app's `Mode`: two
                        // app modes share a byte on CI-V (USB and USB-DATA),
                        // and a rig that has moved between them has not moved
                        // as far as this command can see.
                        self.pushed &= !self.moved_silently(&self.seen_mode, &b, broadcast);
                        if broadcast {
                            self.pushed = true;
                            self.last_push = Some(Instant::now());
                        }
                        self.seen_mode = Some(b);
                        if let Some(m) = civ::civ_to_mode(b) {
                            out.push(CatUpdate::Mode(m));
                        }
                    }
                }
                // The antenna (0x12): which socket the rig is on, and behind
                // it the receiving antenna's own in/out setting. A radio with
                // neither NAKs the read instead, which is what keeps both
                // controls off the screen for it — see
                // `Protocol::antennas_probed`.
                0x12 => {
                    if let Some(r) = civ::parse_antenna_reply(&reply.data) {
                        self.socket = Some(r.socket);
                        // A second byte is a radio with the connector; its
                        // absence is a radio saying it has not got one — the
                        // same "the shape of the answer is the signal" rule
                        // the probe itself runs on.
                        if let Some(on) = r.rx_ant {
                            self.rx_ant = Some(on);
                            out.push(CatUpdate::RxAntenna(on));
                        }
                        // Not reported on a radio with one socket: there is no
                        // selector there, and a panel told "ANT1" would draw a
                        // chip whose other position the radio rejects.
                        if self.sockets > 1
                            && let Some(name) = civ::antenna_name(r.socket, self.sockets)
                        {
                            out.push(CatUpdate::Antenna(name));
                        }
                    }
                }
                // Level read (0x14): the transmit power (0x0A) and the
                // squelch (0x03), both asked for only when the port opens. The
                // sub-command byte in the reply is the only thing that says
                // which arrived, which is why each parser checks it and answers
                // None otherwise — the same shape as the meters below.
                0x14 => {
                    if let Some(frac) = civ::parse_power_reply(&reply.data) {
                        out.push(CatUpdate::Power(frac));
                    } else if let Some(frac) = civ::parse_squelch_reply(&reply.data) {
                        out.push(CatUpdate::Squelch(frac));
                    }
                }
                // The "various settings" block (0x16). Only one of its several
                // dozen sub-commands is asked for here — break-in (0x47),
                // once when the port opens — and the sub-command byte in the
                // reply is the only thing that says which arrived, the same
                // shape as the level and meter reads around it.
                //
                // Nothing is surfaced to the app: this is not a control, it is
                // what `send_cw` needs to know before it decides whether to
                // assert semi break-in over an operator's QSK (issue #282).
                0x16 => {
                    if let Some(bk) = civ::parse_break_in_reply(&reply.data) {
                        self.break_in = Some(bk);
                    }
                }
                // Meter read (0x15): while transmitting the SWR sub-meter
                // (0x12), the ALC one (0x13) and the power-output one (0x11);
                // while receiving the S-meter (0x02). The sub-command byte in
                // the reply says which arrived — nothing else does, since all
                // four are answered on the one command, which is why each
                // parser checks it and returns None otherwise.
                0x15 => {
                    if let Some(swr) = civ::parse_swr_reply(&reply.data) {
                        out.push(CatUpdate::Swr(swr));
                    } else if let Some(alc) = civ::parse_alc_reply(&reply.data) {
                        out.push(CatUpdate::Alc(alc));
                    } else if let Some(po) = civ::parse_po_reply(&reply.data) {
                        out.push(CatUpdate::Po(po));
                    } else if let Some(dbm) = civ::parse_smeter_reply(&reply.data) {
                        out.push(CatUpdate::Signal(dbm));
                    }
                }
                // "NG": the rig would not do what it was asked. Plenty of these
                // are expected — every sub-command a given model doesn't
                // implement answers this way — so it is only noted here, for a
                // caller that knows it just sent something that mattered.
                // Transceiver status (0x1C): sub-command 0x00 is the PTT
                // line. Asked for only while sdroxide is not keying, so an
                // answer of "transmitting" means the operator is on the mic.
                0x1C => {
                    if let Some(on) = civ::parse_ptt_reply(&reply.data) {
                        out.push(CatUpdate::Ptt(on));
                    }
                }
                // A scope sweep fragment (`27 00`), unsolicited once the
                // enables have taken. Over serial a sweep spans ~11 frames;
                // only a completed reassembly is worth surfacing, and only the
                // newest — see `Protocol::take_scope_sweep`.
                0x27 => {
                    if let Some((info, bins)) = civ::parse_scope_frame(&reply.data)
                        .and_then(|s| self.scope_assembler.push(s))
                        && !bins.is_empty()
                    {
                        self.scope_finished = Some(ScopeFrame {
                            center_hz: info.center_hz,
                            span_hz: info.span_hz,
                            bins,
                        });
                    }
                }
                civ::NG => self.nak = true,
                _ => {}
            }
        }
        out
    }
}

/// Output power Yaesu and Kenwood rigs are assumed to have at full scale, in
/// watts.
///
/// Both families' `PC` sets a *number of watts*, and neither has a command that
/// says how many the rig has — so a 0..1 fraction can only be turned into one
/// against an assumption. A hundred watts is what all but a handful of the rigs
/// these two dialects cover put out (the FT-891, FT-991A, FTDX10, FTDX101D,
/// FT-710, TS-590, TS-2000, TS-890 are all 100 W), and the exceptions are the
/// *bigger* ones — an FTDX101MP or a TS-480HX simply tops out at half the
/// slider. Erring low is the right way to be wrong about a transmitter.
///
/// The other two families need none of this. Icom's power level spans whatever
/// the radio has, so the fraction is the setting; and Elecraft, whose rigs run
/// from a 12 W KX2 to a 110 W K3, has an `OM` query that says which one is on
/// the other end of the cable.
const ASCII_FULL_POWER_W: f32 = 100.0;

/// Where to put the dial back after a mode change moved it, or `None` when
/// there is nothing to put back.
///
/// What we last *set* comes first: that is where the operator asked to be, and
/// it is right even if the rig has since shifted itself somewhere else. Only
/// when nothing has been set — a session that has done nothing but adopt the
/// radio's own dial — does the last frequency the rig *reported* stand in, and
/// it is the pre-shift one, because the reply carrying the shifted frequency
/// cannot have arrived before the mode command that caused it went out.
fn dial_to_restore(moves: bool, last_sent: Option<f64>, reported: Option<f64>) -> Option<f64> {
    moves.then(|| last_sent.or(reported)).flatten()
}

/// Piecewise-linear interpolation over a rising calibration table, clamped to
/// its ends.
///
/// Every meter a rig reports arrives as a number on a scale its manufacturer
/// chose and only ever published as a handful of points — "reading 130 is S9",
/// "reading 89 is 2:1" — so turning one into a decibel or a ratio is always
/// this: find the segment, interpolate, and refuse to extrapolate past either
/// end, where the curve is not just unknown but often not monotonic.
pub(crate) fn interp(cal: &[(f32, f32)], x: f32) -> f32 {
    if x <= cal[0].0 {
        return cal[0].1;
    }
    for w in cal.windows(2) {
        let ((x0, y0), (x1, y1)) = (w[0], w[1]);
        if x <= x1 {
            return y0 + (y1 - y0) * (x - x0) / (x1 - x0);
        }
    }
    cal[cal.len() - 1].1
}

/// dBm of an S9 signal on HF, the reference every S-meter curve here is
/// expressed against.
pub(crate) const S9_DBM: f32 = -73.0;

/// `PC` — set output power (Yaesu "new CAT" and Kenwood alike). The three-digit
/// field is in watts, and the families' documented minimum is 5 W: a rig cannot
/// be asked for less, so the bottom of the slider is as low as it goes.
fn pc_set_frame(frac: f32) -> Vec<u8> {
    let w = (frac.clamp(0.0, 1.0) * ASCII_FULL_POWER_W).round().clamp(5.0, 999.0) as u32;
    format!("PC{w:03};").into_bytes()
}

/// `PC;` — ask what the power is set to.
fn pc_read_frame() -> Vec<u8> {
    b"PC;".to_vec()
}

/// The payload of a `PC` reply (the digits after `PC`) as a 0..1 fraction, or
/// `None` when it isn't a number.
fn pc_parse(rest: &str) -> Option<f32> {
    let w: u32 = rest.trim().parse().ok()?;
    Some((w as f32 / ASCII_FULL_POWER_W).clamp(0.0, 1.0))
}

fn make_protocol(cfg: &CatConfig) -> Box<dyn Protocol> {
    match cfg.family {
        // A Xiegu speaks the dialect but is not an Icom: none of the model
        // table applies to it, so it gets the plain mode command.
        // A Xiegu is not in the Icom model table, so it takes the two sockets
        // every radio with a selector has at least — which changes nothing,
        // since it NAKs the read.
        CatFamily::Xiegu => Box::new(Civ::new(cfg.icom_radio_id, None, 2)),
        CatFamily::Icom => {
            let civ = Civ::new(
                cfg.icom_radio_id,
                cfg.icom_model.data_mode_sub(),
                cfg.icom_model.antenna_sockets(),
            );
            Box::new(if scope_active(cfg) {
                civ.with_scope(cfg.scope_span.half_span_hz())
            } else {
                civ
            })
        }
        CatFamily::Yaesu => Box::new(yaesu::Yaesu::new()),
        CatFamily::Kenwood => Box::new(kenwood::Kenwood::new(cfg.kenwood_send)),
        CatFamily::Elecraft => Box::new(elecraft::Elecraft::new()),
        CatFamily::Elad => Box::new(elad::Elad::new(cfg.elad_tx_input)),
        // The one family whose framing depends on the *sound* settings: a QMX's
        // USB codec carries either demodulated audio or raw I/Q, and which one
        // it must carry is asserted at the radio when the port opens.
        CatFamily::QrpLabs => Box::new(qrplabs::QrpLabs::new(cfg.format == SoundFormat::Iq)),
        CatFamily::Rigctld => Box::new(rigctld::Rigctld::new()),
        CatFamily::Flrig => Box::new(flrig::Flrig::new(cfg.flrig_addr.trim().to_string())),
    }
}

enum CatCmd {
    Freq(f64),
    Mode(Mode),
    Ptt(bool),
    /// Text for the rig's own keyer to send.
    Cw(String),
    /// Stop a message the rig is part way through.
    CwAbort,
    CwWpm(f32),
    /// Output power as a 0..1 fraction of the rig's maximum.
    Power(f32),
    /// Squelch threshold as a 0..1 fraction of the rig's own scale.
    Squelch(f32),
    /// The rig's own receive filter: the mode it applies to, and the audio-band
    /// edges in Hz.
    Filter(Mode, f32, f32),
    /// Which antenna socket to receive on, by name.
    Antenna(String),
    /// Switch the rig's receiving antenna into the receive path, or out of it.
    RxAntenna(bool),
    /// Ask the rig again what its antenna and receiving antenna are set to,
    /// because something the radio recalls per band has just happened.
    AntennaRead,
    /// Switch the radio itself off, or back on again.
    RigPower(bool),
    Stop,
}

/// What the serial thread publishes back for [`CatHandle`] to read *without*
/// draining a channel.
///
/// Both of these are asked on every pass of the engine loop, so neither can
/// wait behind an event queue — and both are things only the thread that reads
/// the radio's replies can know.
#[derive(Clone)]
struct Learned {
    /// Whether the sockets in `CatHandle::antennas` may be offered yet — see
    /// [`Protocol::antennas_probed`]. `false` from the start on a family whose
    /// list is a question for the radio, and set the moment the rig answers the
    /// antenna read.
    antennas_known: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// The receiving antenna's setting as the radio last reported it: `0`
    /// nothing said, `1` out of circuit, `2` in it.
    rx_antenna: std::sync::Arc<std::sync::atomic::AtomicU8>,
}

/// Opaque handle to the running serial thread.
pub struct CatHandle {
    cmd_tx: Sender<CatCmd>,
    event_rx: Receiver<CatUpdate>,
    telem_rx: Receiver<TxTelemetry>,
    signal_rx: Receiver<f32>,
    /// The newest finished scope sweep, written by the serial thread and taken
    /// by [`CatHandle::take_scope_sweep`]. A slot rather than a channel because
    /// only the latest sweep can be drawn — one falling behind must overwrite,
    /// not queue.
    scope: std::sync::Arc<std::sync::Mutex<Option<ScopeFrame>>>,
    cw_chunk_len: usize,
    commands_power: bool,
    commands_filter: bool,
    commands_squelch: bool,
    antennas: &'static [&'static str],
    commands_rig_power: bool,
    /// What the serial thread has found out about the radio since — see
    /// [`Learned`].
    learned: Learned,
}

impl CatHandle {
    pub fn set_freq(&self, hz: f64) {
        let _ = self.cmd_tx.send(CatCmd::Freq(hz));
    }
    pub fn set_mode(&self, m: Mode) {
        let _ = self.cmd_tx.send(CatCmd::Mode(m));
    }
    pub fn set_ptt(&self, on: bool) {
        let _ = self.cmd_tx.send(CatCmd::Ptt(on));
    }
    /// How much CW text this rig's keyer takes at a time, or `None` if it
    /// cannot be keyed from text.
    pub fn cw_chunk_len(&self) -> Option<usize> {
        (self.cw_chunk_len > 0).then_some(self.cw_chunk_len)
    }
    /// Hand `text` to the rig's keyer. No more than [`Self::cw_chunk_len`] at a
    /// time, and not again until the rig has finished the last lot — a rig part
    /// way through a message has nowhere to put a second one.
    pub fn send_cw(&self, text: String) {
        let _ = self.cmd_tx.send(CatCmd::Cw(text));
    }
    pub fn abort_cw(&self) {
        let _ = self.cmd_tx.send(CatCmd::CwAbort);
    }
    /// Set the rig keyer's speed — but only when the rig is the one doing the
    /// sending. A rig has one keyer and its paddle uses it too, so an operator
    /// who is not keying from here keeps their own speed.
    pub fn set_cw_wpm(&self, wpm: f32) {
        if self.cw_chunk_len == 0 {
            return;
        }
        let _ = self.cmd_tx.send(CatCmd::CwWpm(wpm));
    }
    /// Set the transmitter's output power, as a `0..1` fraction of what the rig
    /// can do. Silently ignored on a family with no power command, where the
    /// level of the audio going into the rig is the only control there is.
    pub fn set_power(&self, frac: f32) {
        if !self.commands_power {
            return;
        }
        let _ = self.cmd_tx.send(CatCmd::Power(frac));
    }
    /// Whether [`Self::set_power`] reaches this rig.
    pub fn commands_power(&self) -> bool {
        self.commands_power
    }
    /// Set the receiver's squelch threshold, as a `0..1` fraction of the rig's
    /// own scale (`0` open, `1` closed). Silently ignored on a family with no
    /// such command.
    pub fn set_squelch(&self, frac: f32) {
        if !self.commands_squelch {
            return;
        }
        let _ = self.cmd_tx.send(CatCmd::Squelch(frac));
    }
    /// Whether [`Self::set_squelch`] reaches this rig.
    pub fn commands_squelch(&self) -> bool {
        self.commands_squelch
    }
    /// Set the rig's own receive filter to the passband the operator chose,
    /// as audio-band edges in Hz. Silently ignored on a family — or a model —
    /// whose filter sdroxide cannot address.
    pub fn set_filter(&self, mode: Mode, lo_hz: f32, hi_hz: f32) {
        if !self.commands_filter {
            return;
        }
        let _ = self.cmd_tx.send(CatCmd::Filter(mode, lo_hz, hi_hz));
    }
    /// The antenna sockets this rig can put its receiver on, or empty where the
    /// family has no such command — and, on a family whose list has to be
    /// asked for ([`Protocol::antennas_probed`]), empty until the radio has
    /// answered. What the caller publishes as `DeviceCaps::antennas_rx`, and
    /// what `IqSource::learned_antennas` re-publishes when the answer lands.
    pub fn antennas(&self) -> &'static [&'static str] {
        if self.learned.antennas_known.load(std::sync::atomic::Ordering::Relaxed) {
            self.antennas
        } else {
            &[]
        }
    }
    /// Switch the radio off, or back on again. Silently ignored on a family
    /// with no such command — see [`Self::commands_rig_power`].
    pub fn set_rig_power(&self, on: bool) {
        if !self.commands_rig_power {
            return;
        }
        let _ = self.cmd_tx.send(CatCmd::RigPower(on));
    }
    /// Whether [`Self::set_rig_power`] reaches this rig.
    pub fn commands_rig_power(&self) -> bool {
        self.commands_rig_power
    }
    /// Put the receiver on `name`, one of [`Self::antennas`]. Silently ignored
    /// on a rig with one socket, and on a name that rig has never heard of.
    pub fn set_antenna(&self, name: &str) {
        if !self.antennas().contains(&name) {
            return;
        }
        let _ = self.cmd_tx.send(CatCmd::Antenna(name.to_string()));
    }
    /// Whether this rig has a separate receiving antenna connector, and whether
    /// it is switched into the receive path. `None` until it has said — which,
    /// on a radio without one, is never.
    pub fn rx_antenna(&self) -> Option<bool> {
        match self.learned.rx_antenna.load(std::sync::atomic::Ordering::Relaxed) {
            0 => None,
            n => Some(n == 2),
        }
    }
    /// Switch that connector into the receive path, or out of it. Silently
    /// ignored on a rig that has not said it has one.
    pub fn set_rx_antenna(&self, on: bool) {
        if self.rx_antenna().is_none() {
            return;
        }
        let _ = self.cmd_tx.send(CatCmd::RxAntenna(on));
    }
    /// Ask the rig again what its antenna and receiving antenna are set to.
    ///
    /// For after a band change, or after a socket change: the radio recalls
    /// both per band on its own, so what it is set to now is a question for it
    /// rather than something this end may assert.
    pub fn reread_antenna(&self) {
        let _ = self.cmd_tx.send(CatCmd::AntennaRead);
    }
    /// Non-blocking drain of rig-reported freq/mode changes.
    pub fn poll(&self) -> Vec<CatUpdate> {
        self.event_rx.try_iter().collect()
    }
    /// Latest TX telemetry (SWR) the rig reported, or `None` if nothing new
    /// arrived since the last call. A default (all-`None`) value is pushed when
    /// PTT drops, so the reading clears on unkey.
    pub fn poll_telemetry(&self) -> Option<TxTelemetry> {
        self.telem_rx.try_iter().last()
    }

    /// The rig's own S-meter in dBm, or `None` if nothing new arrived since the
    /// last call. Only rigs whose family has such a read report one; the rest
    /// never send here at all.
    pub fn poll_signal(&self) -> Option<f32> {
        self.signal_rx.try_iter().last()
    }

    /// The newest finished sweep of the rig's own spectrum scope, or `None`
    /// when nothing new has completed since the last take. Only an Icom with
    /// [`CatConfig::scope`] on and a fast enough link ever produces one — see
    /// [`scope_active`].
    pub fn take_scope_sweep(&self) -> Option<ScopeFrame> {
        self.scope.lock().unwrap_or_else(|e| e.into_inner()).take()
    }
}

impl Drop for CatHandle {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(CatCmd::Stop);
    }
}

/// How long the startup query waits for one answer before giving up on the
/// rest of its questions. Shorter than [`REPLY_TIMEOUT`] on purpose: the app is
/// waiting on this before it shows anything, and a daemon that is not answering
/// should cost a moment here rather than the driver's whole patience — the
/// driver, which has all session to be patient in, opens its own link behind
/// this and adopts the dial the moment it does answer.
const QUERY_WAIT: Duration = Duration::from_millis(300);

/// Blocking one-shot query of the rig's current frequency + mode, used at
/// startup so the app adopts the radio's state instead of overwriting it.
/// Returns `None` if the port can't be opened or the rig doesn't answer.
pub fn query_once(cfg: &CatConfig) -> Option<(Option<f64>, Option<Mode>)> {
    // The third door onto the port, and it has to be held to the same line
    // discipline as the other two ([`with_family_serial_limits`], applied in
    // [`spawn`]). Without this an FDM-DUO left on the shared 19200 default is
    // asked its dial at a rate it has no setting for, answers nothing, and the
    // caller reads that as a radio with no control link at all — while the
    // driver opening behind it snaps to 38400 and talks to the rig perfectly.
    let cfg = &with_family_serial_limits(cfg.clone());
    let mut port = open_link(cfg).ok()?;
    let mut protocol = make_protocol(cfg);
    protocol.link_opened();
    let mut io = Exchange::default();
    let mut last_write = Instant::now() - FRAME_GAP;
    // Through [`write_frame`] rather than straight at the port, for the rule
    // it carries: a family that can only read its answers by their order gets
    // one question at a time here too. Two reads written back to back is
    // exactly what flrig throws half of away, and the half it kept was the
    // dial — which is how a startup query came to adopt a frequency and no
    // mode.
    for req in protocol.poll_requests() {
        if write_frame_within(
            &mut *port,
            &mut *protocol,
            &req,
            &mut last_write,
            &mut io,
            QUERY_WAIT,
        ) {
            break;
        }
    }
    let _ = port.flush();
    let (mut freq, mut mode) = (None, None);
    let deadline = Instant::now() + Duration::from_millis(600);
    while Instant::now() < deadline && (freq.is_none() || mode.is_none()) {
        if io.read_once(&mut *port, &mut *protocol).is_none() {
            break;
        }
        for u in io.updates.drain(..) {
            match u {
                CatUpdate::Freq(hz) => freq = Some(hz),
                CatUpdate::Mode(m) => mode = Some(m),
                // No meter, the power, the squelch, or the transmit state is
                // requested during the startup query.
                CatUpdate::Swr(_)
                | CatUpdate::Alc(_)
                | CatUpdate::Po(_)
                | CatUpdate::FwdW(_)
                | CatUpdate::Signal(_)
                | CatUpdate::Power(_)
                | CatUpdate::Squelch(_)
                | CatUpdate::Antenna(_)
                | CatUpdate::RxAntenna(_)
                | CatUpdate::Ptt(_) => {}
            }
        }
    }
    (freq.is_some() || mode.is_some()).then_some((freq, mode))
}

/// Bring the serial parameters into line with what the *radio* actually has,
/// where the family fixes them.
///
/// Only the FDM-DUO so far, and it earns the treatment: its CAT port is
/// asynchronous 8N1 at 9600, 38400, 57600 or 115200 and nothing else (manual
/// v2.6 §6.1). A port opened outside that is not slow or lossy, it is silent in
/// both directions — no command lands and no answer comes back, so what the
/// operator sees is a radio ignoring the dial and refusing to key, on whatever
/// port they try next.
///
/// The reason this is worth doing rather than leaving to the settings dialog is
/// that the failing value is the *default*. [`SerialConfig`]'s own default baud
/// is 19200, which is not one of the four, and the `cat` block is shared between
/// the CAT interface and the native ELAD one — so an ELAD owner who has never
/// touched Baud starts from a link their radio cannot hear a word of. That was
/// [issue #146].
///
/// [`SerialConfig`]: sdroxide_types::SerialConfig
/// [issue #146]: https://github.com/dividebysandwich/sdroxide/issues/146
fn with_family_serial_limits(mut cfg: CatConfig) -> CatConfig {
    if cfg.family != CatFamily::Elad {
        return cfg;
    }
    let asked = cfg.serial.baud;
    cfg.serial.baud = sdroxide_types::elad_cat_baud(asked);
    if cfg.serial.baud != asked {
        warn!(
            asked,
            using = cfg.serial.baud,
            "no FDM-DUO has that CAT baud rate (menu 70 offers four); opening at the \
             factory rate instead"
        );
    }
    cfg.serial.data_bits = 8;
    cfg.serial.parity = Parity::None;
    cfg.serial.stop_bits = StopBits::One;
    cfg
}

/// Spawn the serial CAT thread from a persisted [`CatConfig`].
pub fn spawn(cfg: CatConfig) -> CatHandle {
    let cfg = with_family_serial_limits(cfg);
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    let (event_tx, event_rx) = crossbeam_channel::unbounded();
    let (telem_tx, telem_rx) = crossbeam_channel::unbounded();
    let (signal_tx, signal_rx) = crossbeam_channel::unbounded();
    // Asked of the framing before it goes to the thread, so the keyer can size
    // its chunks to the rig without reaching across the channel to find out.
    let cw_chunk_len =
        if cfg.cw_keying == CwKeying::Cat { make_protocol(&cfg).cw_chunk_len() } else { 0 };
    // Same reason: the engine asks whether this rig's power can be commanded
    // before it commands anything.
    let commands_power = make_protocol(&cfg).commands_power();
    let commands_filter = make_protocol(&cfg).commands_filter();
    let commands_squelch = make_protocol(&cfg).commands_squelch();
    // And the same again for the antenna sockets: the caps this device
    // publishes are built before a single frame has gone out, so the list has
    // to come from the framing rather than from the rig. Where the framing
    // cannot answer for the model — every CI-V rig shares one dialect and only
    // some have a selector — the list is held back until the radio has replied
    // to the opening read.
    let antennas = make_protocol(&cfg).antennas();
    let commands_rig_power = make_protocol(&cfg).commands_rig_power();
    let learned = Learned {
        antennas_known: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
            !make_protocol(&cfg).antennas_probed(),
        )),
        // Nothing said yet — see `CatHandle::rx_antenna` for the three values.
        rx_antenna: std::sync::Arc::new(std::sync::atomic::AtomicU8::new(0)),
    };
    let learned_in = learned.clone();
    let scope = std::sync::Arc::new(std::sync::Mutex::new(None));
    let scope_in = scope.clone();
    std::thread::Builder::new()
        .name("sdroxide-cat".into())
        .spawn(move || {
            serial_thread(cfg, cmd_rx, event_tx, telem_tx, signal_tx, scope_in, learned_in)
        })
        .expect("spawn cat thread");
    CatHandle {
        cmd_tx,
        event_rx,
        telem_rx,
        signal_rx,
        scope,
        cw_chunk_len,
        commands_power,
        commands_filter,
        commands_squelch,
        antennas,
        commands_rig_power,
        learned,
    }
}

fn map_parity(p: Parity) -> serialport::Parity {
    match p {
        Parity::None => serialport::Parity::None,
        Parity::Even => serialport::Parity::Even,
        Parity::Odd => serialport::Parity::Odd,
    }
}
fn map_stop(s: StopBits) -> serialport::StopBits {
    match s {
        StopBits::One => serialport::StopBits::One,
        StopBits::Two => serialport::StopBits::Two,
    }
}
fn map_data_bits(n: u8) -> serialport::DataBits {
    match n {
        5 => serialport::DataBits::Five,
        6 => serialport::DataBits::Six,
        7 => serialport::DataBits::Seven,
        _ => serialport::DataBits::Eight,
    }
}

/// The shortest gap left between two frames written to the rig.
///
/// A transceiver serves its control port with the same processor that runs the
/// radio, and it acts on one command at a time: a frame that arrives while the
/// rig is still working through the previous one can simply be missed, and
/// nothing on the wire says so. The case that matters is key-down, which asserts
/// the mode (and, with split or XIT, the transmit frequency) and then keys —
/// changing mode is among the slowest things a rig does, and a PTT lost behind
/// one is an over that never reaches the air while everything else about the
/// link looks healthy.
///
/// Only consecutive writes wait; a frame sent on its own goes out at once. The
/// whole traffic here is a handful of short frames a second, so this costs
/// nothing that can be noticed — and [`ModeMemory`] keeps the mode off the wire
/// entirely when the rig is already in it, which is what makes key-down a single
/// frame in the ordinary case.
const FRAME_GAP: Duration = Duration::from_millis(30);

/// Shortest gap between two output-power writes. A slider drag is worth one
/// frame every so often, not one per pixel; a hundred milliseconds is faster
/// than anyone can read a wattmeter and slow enough that the queue in front of
/// the next PTT stays empty.
const POWER_GAP: Duration = Duration::from_millis(100);

/// Shortest gap between two receive-filter writes. Longer than the power's: a
/// filter edge is dragged rather than nudged, several frames can go out per
/// change on some families, and nothing about a receive filter is urgent.
const FILTER_GAP: Duration = Duration::from_millis(250);

/// Shortest gap between two squelch writes. The power's rather than the
/// filter's: a squelch is set by ear against the noise it is muting, and a
/// threshold that arrives a quarter of a second after the rail moved is one the
/// operator cannot hear themselves setting.
const SQUELCH_GAP: Duration = Duration::from_millis(100);

/// What has been read from the link but not yet acted on: bytes still short of
/// a whole reply, and the updates the whole ones yielded.
///
/// One of each per connection, shared by the loop's own read and by the waits
/// inside [`await_answers`] — two buffers would cut a reply in half between
/// them, and updates parsed while waiting to write have to reach the loop that
/// dispatches them.
#[derive(Default)]
struct Exchange {
    rx: Vec<u8>,
    updates: Vec<CatUpdate>,
}

impl Exchange {
    /// One read, parsed into whatever it completes. Returns how many bytes it
    /// took, or `None` on a read error — the caller's signal to reconnect.
    fn read_once(&mut self, port: &mut dyn Link, protocol: &mut dyn Protocol) -> Option<usize> {
        let mut buf = [0u8; 256];
        match port.read(&mut buf) {
            Ok(0) if port.closed_on_empty_read() => {
                warn!("the CAT link was closed from the other end");
                None
            }
            Ok(0) => Some(0),
            Ok(n) => {
                self.rx.extend_from_slice(&buf[..n]);
                self.updates.extend(protocol.parse(&mut self.rx));
                Some(n)
            }
            // A read that found nothing in its window. Which of the two kinds
            // arrives is the transport's business — a serial port times out, a
            // socket would block — and neither is an error.
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                Some(0)
            }
            Err(e) => {
                warn!("CAT read error: {e}");
                None
            }
        }
    }
}

/// How long an answer is waited for before the link is given up on.
///
/// Only a family that correlates by order waits at all, and the wait is
/// normally the length of one round trip to a program on the same machine.
/// This is the other end of it: a daemon wedged, gone, or one that lost the
/// request.
///
/// Three seconds because of what the slowest ordinary call costs: flrig's
/// `rig.set_ptt` holds its server for up to a second waiting for the radio to
/// confirm the change, that server is one thread shared by every program
/// pointed at it, and an operator running sdroxide beside fldigi can have two
/// of those queued ahead of the answer being waited for here. Reconnecting
/// through a daemon that was merely busy would cost more than the wait does.
const REPLY_TIMEOUT: Duration = Duration::from_secs(3);

/// Wait until the profile has no request outstanding, reading and parsing
/// whatever arrives meanwhile. Returns true if the answer never came, or the
/// link failed — the caller's signal to reconnect, which is also the only way
/// to put a lost correlation right.
fn await_answers(
    port: &mut dyn Link,
    protocol: &mut dyn Protocol,
    io: &mut Exchange,
    patience: Duration,
) -> bool {
    if protocol.in_flight() == 0 {
        return false;
    }
    let deadline = Instant::now() + patience;
    while protocol.in_flight() > 0 {
        match io.read_once(port, protocol) {
            None => return true,
            // A link at the far end's own timeout paces this loop for us; a
            // socket whose peer has gone answers "nothing" the instant it is
            // asked, forever, and would otherwise spin here until the deadline.
            Some(0) => std::thread::sleep(Duration::from_millis(2)),
            Some(_) => {}
        }
        if protocol.in_flight() > 0 && Instant::now() >= deadline {
            warn!(
                outstanding = protocol.in_flight(),
                "the daemon left a request unanswered for {patience:?}; reconnecting, because \
                 from here on nothing it says can be matched to what was asked"
            );
            return true;
        }
    }
    false
}

/// How long to wait before remaking a link that has just failed, so a daemon
/// that is down is not hammered with a connection per loop.
const RECONNECT_PAUSE: Duration = Duration::from_secs(1);

/// Wait out [`RECONNECT_PAUSE`] on the command channel rather than on a bare
/// sleep, so a thread that is being shut down does not first spend a second
/// waiting to reconnect to something nobody is listening to any more. Returns
/// true if the thread should stop.
fn pause_before_reconnect(cmd_rx: &Receiver<CatCmd>) -> bool {
    matches!(
        cmd_rx.recv_timeout(RECONNECT_PAUSE),
        Ok(CatCmd::Stop) | Err(crossbeam_channel::RecvTimeoutError::Disconnected)
    )
}

/// Write one frame, leaving at least [`FRAME_GAP`] since the last one went out.
/// Returns true on a write error — the caller's signal to reconnect.
///
/// The protocol is told about every frame that goes out ([`Protocol::wrote`])
/// — here, and nowhere else, so that "generated" and "written" cannot drift
/// apart on it. An empty frame is a profile saying "nothing to send" (an
/// unmappable mode, say) and writes nothing, waits for nothing, tells nothing.
///
/// A family that tells its answers apart only by their order gets one question
/// at a time: the answer to the last one is waited for here, before this frame
/// is allowed onto the wire. See [`Protocol::in_flight`] for what is being
/// avoided.
fn write_frame(
    port: &mut dyn Link,
    protocol: &mut dyn Protocol,
    frame: &[u8],
    last_write: &mut Instant,
    io: &mut Exchange,
) -> bool {
    write_frame_within(port, protocol, frame, last_write, io, REPLY_TIMEOUT)
}

/// [`write_frame`] with the caller's own patience for the answer still owed —
/// for the startup query, which somebody is waiting on and must not spend the
/// driver's whole timeout discovering that a daemon is not answering.
fn write_frame_within(
    port: &mut dyn Link,
    protocol: &mut dyn Protocol,
    frame: &[u8],
    last_write: &mut Instant,
    io: &mut Exchange,
    patience: Duration,
) -> bool {
    if frame.is_empty() {
        return false;
    }
    if await_answers(port, protocol, io, patience) {
        return true;
    }
    let since = last_write.elapsed();
    if since < FRAME_GAP {
        std::thread::sleep(FRAME_GAP - since);
    }
    let failed = port.write_all(frame).is_err();
    protocol.wrote(frame);
    *last_write = Instant::now();
    failed
}

/// The lowest frequency a transceiver has a repeater shift on. Below it the
/// only thing that can put a duplex setting back is a band stacking register,
/// which is a band crossing; above it there are auto-repeater functions that
/// arm on the *frequency*, so the assertion has to follow every move.
///
/// 28 MHz rather than 30: 10 m FM repeaters live at 29.6, and they are the
/// lowest ones anybody works.
pub const DUPLEX_FLOOR_HZ: f64 = 28_000_000.0;

/// Whether the rig's own repeater shift needs putting back to simplex, given
/// the dial this end believes it is on and the dial that was last asserted for.
///
/// On HF this is true once per band: the first dial a fresh connection learns,
/// and every crossing after that, because a band stacking register is the only
/// thing down there that can restore a duplex setting and it does so when the
/// dial crosses into the band.
///
/// From [`DUPLEX_FLOOR_HZ`] up it is true on **every** move of the dial. Icom's
/// auto-repeater function (and its equivalents) arms on the frequency, not on
/// the band: tuning from one 2 m channel to another inside the same band puts
/// DUP back without ever crossing a band edge, and the over then goes out a
/// shift away from where the operator asked for it (issue #233). A frame per
/// settled dial position is what that costs; the rig NAKs it if it has no such
/// command, and the poll already carries more traffic than this does.
///
/// `asserted` is updated in place, so a caller that acts on the answer will not
/// be told again until the dial moves somewhere else.
///
/// A dial outside every amateur band is [`sdroxide_types::Band::Gen`], and two
/// such HF frequencies are the same "band" here — a radio whose duplex is
/// re-stacked between two out-of-band frequencies is not one this can see, and
/// there is nothing to transmit there anyway.
fn needs_simplex(dial_hz: Option<f64>, asserted: &mut Option<f64>) -> bool {
    let Some(hz) = dial_hz else { return false };
    let same = match *asserted {
        None => false,
        Some(prev) => {
            prev == hz
                || (prev < DUPLEX_FLOOR_HZ
                    && hz < DUPLEX_FLOOR_HZ
                    && sdroxide_types::Band::containing(prev)
                        == sdroxide_types::Band::containing(hz))
        }
    };
    if same {
        return false;
    }
    *asserted = Some(hz);
    true
}

/// Write an output-power level, unless it is the one the rig was last given.
/// Returns true on a write error — the caller's signal to reconnect.
///
/// Skipping a level already written is what keeps the assertion before every
/// key-down free: the rig is served one frame at a time, and a needless one in
/// front of a PTT delays the transmitter coming up.
fn write_power(
    port: &mut dyn Link,
    protocol: &mut dyn Protocol,
    frac: f32,
    last_sent: &mut PowerMemory,
    last_write: &mut Instant,
    io: &mut Exchange,
) -> bool {
    if last_sent.holds(frac) {
        return false;
    }
    let mut failed = false;
    for f in protocol.set_power(frac) {
        failed |= write_frame(port, protocol, &f, last_write, io);
    }
    if !failed {
        last_sent.set(frac);
    }
    failed
}

/// The output power the rig was last *given* — or last said it was on — so a
/// level it already holds costs no frame in front of the next PTT.
///
/// The catch is that this is a cache of a register the *radio* also writes to.
/// These families keep their output power **per mode**: switch a rig from USB
/// to DATA-U and its transmitter comes up on whatever was last set in DATA-U,
/// and one switched into AM is held to a quarter of its rated power on top of
/// that on most of them. So the moment the mode changes, this no longer
/// describes anything the rig is on, and the level the operator chose has to be
/// written again rather than deduped away.
///
/// It is the I/Q sound format where that bites: there sdroxide demodulates the
/// rig's quadrature stream itself, so the app's mode is nobody's business but
/// ours until the transmitter comes up, and the mode is asserted *at key-down*
/// (see the engine's `sync_tx_state`). Every over therefore moved the rig to
/// another mode — and so to another power register — and the assertion that
/// should have put the Drive level back was skipped as already sent, leaving
/// the over on whatever that mode happened to be left at and the Drive slider
/// apparently doing nothing at all.
#[derive(Default)]
struct PowerMemory(Option<f32>);

impl PowerMemory {
    /// Whether the rig is known to be on this level already.
    fn holds(&self, frac: f32) -> bool {
        self.0 == Some(frac)
    }

    /// Record a level the rig has been given, or has reported it is on.
    fn set(&mut self, frac: f32) {
        self.0 = Some(frac);
    }

    /// The rig changed mode, whoever moved it. Whatever this held describes a
    /// register the transmitter is no longer using, and the next level asked
    /// for has to reach the radio.
    fn forget(&mut self) {
        self.0 = None;
    }
}

/// What mode the rig is in, held as the frame that would put it there — either
/// because it was told so, or because it said so on its last poll.
///
/// Every key-down asserts the mode, so that an over cannot go out in whatever
/// the rig happens to have been left in. Asserting it is not free, though: the
/// rig acts on the command every time, which on an Icom also re-selects filter
/// 1 under an operator who chose another, and leaves the radio busy at exactly
/// the moment the PTT frame arrives behind it. So the command is only written
/// when it would actually change something.
///
/// What is compared is the frame, not the mode: two of the app's modes can be
/// one thing to a rig (DIGU rides on USB, and that is what goes on the wire),
/// and the rig can only report back the one it has.
#[derive(Default)]
struct ModeMemory(Option<Vec<u8>>);

impl ModeMemory {
    /// True when `frame` still needs sending — the rig is in some other mode,
    /// or has not said which. Records it as sent.
    fn needs(&mut self, frame: &[u8]) -> bool {
        if self.0.as_deref() == Some(frame) {
            return false;
        }
        self.0 = Some(frame.to_vec());
        true
    }

    /// The rig reported the mode `frame` would have set. That is where it is,
    /// whoever put it there — a mode the operator selected on the radio itself
    /// is one there is no need to command back onto it.
    fn reported(&mut self, frame: &[u8]) {
        if self.0.as_deref() != Some(frame) {
            self.0 = Some(frame.to_vec());
        }
    }
}

/// The link to the radio: a serial port, or a TCP connection to a `rigctld`.
///
/// Everything above this point is bytes in and bytes out, which is why the
/// network case fits at all — a rigctld speaks a line protocol over a socket
/// exactly as a transceiver speaks a frame protocol over a wire, and the
/// driver's rate limiting, coalescing and reconnection all mean the same thing
/// on both. The one thing that does not carry over is the pair of control
/// lines, which a socket simply does not have.
trait Link: Send {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()>;
    fn flush(&mut self) -> std::io::Result<()>;
    /// Drive RTS, where there is one. A network link has none, so a `PTT
    /// method` of RTS or DTR keys nothing there — see [`PttMethod`].
    fn set_rts(&mut self, _on: bool) {}
    fn set_dtr(&mut self, _on: bool) {}
    /// Whether a read of no bytes means the far end has gone.
    ///
    /// On a socket it does — that is exactly what EOF looks like — and reading
    /// it as "nothing yet" leaves the driver re-reading a connection that has
    /// already been closed until something else notices, which on a family that
    /// waits for its answers is the whole of [`REPLY_TIMEOUT`]. A serial port
    /// reports an idle window as a timeout instead, so there a zero-byte read
    /// says nothing at all.
    fn closed_on_empty_read(&self) -> bool {
        false
    }
}

impl Link for Box<dyn serialport::SerialPort> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        std::io::Read::read(self, buf)
    }
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        std::io::Write::write_all(self, buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        std::io::Write::flush(self)
    }
    fn set_rts(&mut self, on: bool) {
        let _ = self.write_request_to_send(on);
    }
    fn set_dtr(&mut self, on: bool) {
        let _ = self.write_data_terminal_ready(on);
    }
}

impl Link for std::net::TcpStream {
    fn closed_on_empty_read(&self) -> bool {
        true
    }
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        std::io::Read::read(self, buf)
    }
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        std::io::Write::write_all(self, buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        std::io::Write::flush(self)
    }
}

/// How long a read waits before giving the loop its turn back. The same on both
/// transports: the driver polls, so a read that blocks is a driver that cannot
/// write.
const READ_TIMEOUT: Duration = Duration::from_millis(50);

/// Open whichever link this configuration describes.
fn open_link(cfg: &CatConfig) -> std::io::Result<Box<dyn Link>> {
    if cfg.family.is_network() {
        let addr = match cfg.family {
            CatFamily::Flrig => cfg.flrig_addr.trim(),
            _ => cfg.rigctld_addr.trim(),
        };
        let stream = std::net::TcpStream::connect(addr)?;
        stream.set_read_timeout(Some(READ_TIMEOUT))?;
        // Every command here is a handful of bytes that wants to be on the wire
        // now: a key-down waiting on Nagle for another 40 ms is an over that
        // starts late.
        let _ = stream.set_nodelay(true);
        return Ok(Box::new(stream));
    }
    let s = &cfg.serial;
    let port = serialport::new(&s.path, s.baud)
        .data_bits(map_data_bits(s.data_bits))
        .parity(map_parity(s.parity))
        .stop_bits(map_stop(s.stop_bits))
        .timeout(READ_TIMEOUT)
        .open()
        .map_err(std::io::Error::other)?;
    Ok(Box::new(port))
}

/// What to call this link in the log — a port and a baud rate, or a host.
///
/// The rate named is the one the port will actually be opened at, not
/// necessarily the one in the config: see [`with_family_serial_limits`]. A label
/// that repeated a baud rate the radio has no setting for would be pointing at
/// the wrong thing in exactly the moment somebody is reading it to find out why
/// the link is dead.
pub fn link_label(cfg: &CatConfig) -> String {
    match cfg.family {
        CatFamily::Rigctld => format!("rigctld at {}", cfg.rigctld_addr.trim()),
        CatFamily::Flrig => format!("flrig at {}", cfg.flrig_addr.trim()),
        CatFamily::Elad => {
            format!(
                "{} at {} baud",
                cfg.serial.path,
                sdroxide_types::elad_cat_baud(cfg.serial.baud)
            )
        }
        _ => format!("{} at {} baud", cfg.serial.path, cfg.serial.baud),
    }
}

/// Apply a forced control-line level (ignored when `LineState::None`). If a
/// line is used for PTT, PTT owns it instead (handled in the loop).
fn apply_line(port: &mut dyn Link, forced: LineState, rts: bool) {
    let level = match forced {
        LineState::None => return,
        LineState::High => true,
        LineState::Low => false,
    };
    if rts { port.set_rts(level) } else { port.set_dtr(level) }
}

/// What mode to command the rig into for a given app mode. Everything sdroxide
/// modulates through the rig's sound card uses the separate `digi_mode`
/// setting; every other mode obeys `mode_control` (CAT = mirror the selected
/// mode to the rig; Radio = don't touch it).
///
/// `digi_mode` is a choice between two spellings of a *sideband* — the plain
/// one or the rig's DATA position — so it only makes sense for a mode that
/// rides a sideband. The carrier-centred modes (RIFP, VHF packet) frequency-modulate
/// the carrier instead: sending them as USB puts the rig in the wrong
/// modulation entirely, and nothing downstream would say so. Those fall
/// through to `mode_control`, where each protocol's own map answers DATA-FM.
///
/// CW keyed as audio (`CwKeying::Audio`) rides the digi sideband too: a rig
/// put in CW keys its own transmitter and never modulates what arrives at its
/// sound card, so the keyed sidetone (MCW) only reaches the air from USB or
/// DATA-U. Commanding CW there was the issue #119 dead key — a Xiegu G90
/// switched out of U-D made no power at all.
///
/// Analog SSTV rides it as well, on whichever sideband it is being used on:
/// it is the app that modulates the picture, through the same sound card and
/// into the same input, so a rig left in plain USB transmits it off the
/// microphone (issue #313). It used to be excluded here because `digi_mode`
/// names only the *upper* pair — and SSTV is one of the two modes whose
/// sideband follows the band, LSB on 160/80/40 m — which is what `dial_hz`
/// answers. RADE is the other, and for the same reason: it is digital voice
/// worked alongside phone, and an over sent on the wrong sideband arrives
/// inverted and decodes for nobody (issue #317). Without a dial the sideband
/// cannot be worked out, so the upper pair is assumed, as it was for every
/// mode before this took a frequency at all.
fn commanded_mode(cfg: &CatConfig, app_mode: Mode, dial_hz: Option<f64>) -> Option<Mode> {
    let rides_digi_sideband = (app_mode.is_digital() && !app_mode.is_carrier_centered())
        || (app_mode == Mode::Cw && cfg.cw_keying == CwKeying::Audio);
    if rides_digi_sideband {
        let lower = dial_hz.is_some_and(|hz| app_mode.is_lower_sideband_at(hz));
        return match cfg.digi_mode {
            DigiMode::Radio => None,
            DigiMode::Usb => Some(if lower { Mode::Lsb } else { Mode::Usb }),
            DigiMode::Data => Some(if lower { Mode::Digl } else { Mode::Digu }),
        };
    }
    match cfg.mode_control {
        ModeControl::Cat => Some(app_mode),
        ModeControl::Radio => None,
    }
}

fn serial_thread(
    cfg: CatConfig,
    cmd_rx: Receiver<CatCmd>,
    event_tx: Sender<CatUpdate>,
    telem_tx: Sender<TxTelemetry>,
    signal_tx: Sender<f32>,
    scope_out: std::sync::Arc<std::sync::Mutex<Option<ScopeFrame>>>,
    learned: Learned,
) {
    let mut protocol = make_protocol(&cfg);
    let poll_period = poll_period(&cfg);
    // The meters follow the poll rate too. They used to run at a fixed 5 Hz,
    // which made the setting a half-measure: an operator turning the control
    // traffic down to quieten a shared USB bus took away the dial poll and left
    // the meter poll — the same number of frames — running underneath it.
    //
    // Only while receiving. The transmit side stays at `METER_FLOOR` whatever
    // the setting says: that reading is the SWR, the protection trip counts on
    // it arriving, and it only runs for the length of an over.
    let rx_meter_period = meter_period(&cfg);
    // The mode rides only every `MODE_POLL_EVERY`th dial poll — see
    // `mode_poll_period`.
    let mode_period = mode_poll_period(&cfg);
    // What the log was last told about the transceive stand-down. Kept per
    // process rather than per connection: `protocol` — and so what it has
    // learned about this rig — outlives a reconnect.
    let mut announced_push = false;
    // The latest of each TX meter, because they arrive in separate replies and
    // the consumer keeps only the last message sent. See the send site.
    let mut last_swr: Option<f32> = None;
    let mut last_alc: Option<f32> = None;
    let mut last_po: Option<f32> = None;
    let mut last_fwd: Option<f32> = None;
    // See `commanded_mode` for the app-mode → rig-mode policy. The dial comes
    // in because one mode's sideband follows the band — see there.
    let mode_cmd = |app_mode: Mode, dial: Option<f64>| -> Option<Mode> {
        commanded_mode(&cfg, app_mode, dial)
    };

    loop {
        // (Re)open the port, retrying on failure.
        let mut port = match open_link(&cfg) {
            Ok(p) => {
                // The PTT method belongs in this line: a rig that answers every
                // read and still refuses to key is nearly always one being asked
                // to key some way it isn't set up for, and this is where that
                // shows.
                info!(
                    link = %link_label(&cfg),
                    family = cfg.family.label(),
                    ptt = cfg.ptt.label(),
                    "CAT link open"
                );
                p
            }
            Err(e) => {
                warn!(link = %link_label(&cfg), "CAT open failed: {e}");
                // Wait, but still honor a Stop.
                match cmd_rx.recv_timeout(Duration::from_secs(2)) {
                    Ok(CatCmd::Stop) | Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                        return;
                    }
                    _ => continue,
                }
            }
        };
        // Forced control lines (unless the line is the PTT method).
        if cfg.ptt != PttMethod::Rts {
            apply_line(&mut *port, cfg.serial.force_rts, true);
        }
        if cfg.ptt != PttMethod::Dtr {
            apply_line(&mut *port, cfg.serial.force_dtr, false);
        }
        // Deassert PTT line at start.
        match cfg.ptt {
            PttMethod::Rts => port.set_rts(false),
            PttMethod::Dtr => port.set_dtr(false),
            _ => {}
        }
        // When the last frame went out, so consecutive writes can be spaced
        // (see `FRAME_GAP`). Backdated: the first write waits for nothing.
        let mut last_write = Instant::now() - FRAME_GAP;
        // Everything read from this connection and not yet acted on. Declared
        // with it and dropped with it: half a reply belongs to the link it
        // came from and to nothing else.
        let mut io = Exchange::default();
        // A fresh connection: whatever the protocol still holds about bytes in
        // flight on the old one is now about nothing.
        protocol.link_opened();
        // The opening sequence, and the one place a failed write is worth
        // stopping for rather than pressing on through: everything here is
        // asked once per connection, so a link that cannot carry the first
        // frame cannot carry the rest either — and on a family that waits for
        // its answers, pressing on means spending the whole timeout again per
        // frame while the operator waits to find out the daemon is not there.
        // The opening sequence, and the one place a failed write is worth
        // stopping for rather than pressing on through: everything here is
        // asked once per connection, so a link that cannot carry the first
        // frame will not carry the rest — and on a family that waits for its
        // answers, pressing on spends the whole patience again per frame while
        // the operator waits to be told the daemon is not there.
        let opening_failed = 'opening: {
            // Introductions first, for the profile that needs them: the answers
            // to these (a power scale, a mode list) are what its later frames
            // are interpreted against, and write order is reply order.
            for f in protocol.open_requests() {
                if write_frame(&mut *port, &mut *protocol, &f, &mut last_write, &mut io) {
                    break 'opening true;
                }
            }
            // Don't force a mode on connect — adopt the rig's current mode
            // (read via `query_once`/poll); the app commands mode only when the
            // operator picks one. RIT/XIT/split are the exception: those we do
            // own, so clear the rig's own copies rather than let them offset us
            // invisibly.
            for f in protocol.clear_offsets() {
                if write_frame(&mut *port, &mut *protocol, &f, &mut last_write, &mut io) {
                    break 'opening true;
                }
            }
            // The transmit power is the rig's, not ours: ask what it is set to
            // and let the panel adopt it, the same way the dial and the mode
            // are adopted. Imposing a remembered level instead would put an
            // operator who has never touched the slider on the air at whatever
            // it defaults to — on a radio they had already set the power on.
            for f in protocol.read_power() {
                if write_frame(&mut *port, &mut *protocol, &f, &mut last_write, &mut io) {
                    break 'opening true;
                }
            }
            // And where the squelch is set, adopted the same way and for the
            // same reason. On a rig sending demodulated audio this is the gate
            // the operator actually hears, so the control has to start on the
            // radio's own level rather than somewhere the audio would not
            // match.
            for f in protocol.read_squelch() {
                if write_frame(&mut *port, &mut *protocol, &f, &mut last_write, &mut io) {
                    break 'opening true;
                }
            }
            // Whether break-in is on, which on a CI-V rig is what decides
            // whether a CW message handed to its keyer is transmitted at all.
            // Not adopted — asserted at the moment of sending — but read here
            // so an operator running full break-in is not knocked down to semi
            // by their own first message (issue #282).
            for f in protocol.read_break_in() {
                if write_frame(&mut *port, &mut *protocol, &f, &mut last_write, &mut io) {
                    break 'opening true;
                }
            }
            // Which socket the receiver is on, asked the same
            // once-per-connection way and adopted the same way. The rig
            // remembers it across power cycles, so this is the only moment the
            // panel can find out what the operator left it on.
            //
            // Each group is generated where it is written rather than all four
            // up front: on a family that waits, the answers to one group are in
            // hand before the next is built, and a profile is entitled to use
            // them.
            for f in protocol.read_antenna() {
                if write_frame(&mut *port, &mut *protocol, &f, &mut last_write, &mut io) {
                    break 'opening true;
                }
            }
            false
        };
        if opening_failed {
            warn!(link = %link_label(&cfg), "CAT link failed while opening; reconnecting");
            if pause_before_reconnect(&cmd_rx) {
                return;
            }
            continue;
        }
        // Start the rig's scope streaming, where this session wants it. The
        // enables are fire-and-forget — a rig without a scope answers NG and
        // that is the end of it — and the watchdog below re-sends them when
        // the sweeps stop, so a lost enable is a delay rather than a session
        // with no picture.
        let scope_wanted = {
            let reqs = protocol.scope_requests();
            for f in &reqs {
                if write_frame(&mut *port, &mut *protocol, f, &mut last_write, &mut io) {
                    break;
                }
            }
            !reqs.is_empty()
        };
        let mut last_sweep = Instant::now();
        let mut next_scope_nudge = Instant::now() + SCOPE_STALL;
        let mut scope_retry = SCOPE_RETRY;
        // The antenna read is the one opening question whose *absence of an
        // answer* is itself meaningful — it is how a rig with one socket says
        // so — which makes a lost reply indistinguishable from a rig without a
        // selector, and leaves the panel with no antenna control until the next
        // reconnect (issue #258). So it is asked again, a few times, and then
        // let go: three more chances at a reply cost four frames on a radio
        // that has no selector, and rescue the far commoner case of an answer
        // that collided with the opening burst.
        let mut antenna_probes_left =
            if protocol.antennas_probed() { ANTENNA_PROBE_RETRIES } else { 0 };
        let mut next_antenna_probe = Instant::now() + ANTENNA_PROBE_RETRY;

        let mut next_poll = Instant::now();
        // Backdated so the first poll of a connection carries the mode: the app
        // adopts the rig's mode rather than commanding one, and waiting a mode
        // period to find out what it is would leave the panel wrong meanwhile.
        let mut next_mode_poll = Instant::now();
        // Which meter is asked for depends on what the rig is doing: SWR while
        // keyed, S-meter while receiving.
        //
        // Started half a period behind the dial poll. Started together the two
        // fire together, and four frames go out back to back with nothing but
        // `FRAME_GAP` between them — the worst shape this traffic can have on a
        // radio whose control port shares a USB bus with its sound card, where
        // what the audio needs is the gaps rather than a lower average. Same
        // number of frames, spread out.
        //
        // Only the starting phase: both timers re-arm from the moment they
        // fire, so the two wander relative to each other over a long session.
        // Worth having anyway — the cost is one addition, and the worst it can
        // decay to is the lockstep it starts out avoiding.
        let mut next_meter = Instant::now() + rx_meter_period / 2;
        let mut ptt = false;
        // When a CAT key-down was last written, so the rig's refusal of one can
        // be told from the refusals its unimplemented sub-commands answer with.
        let mut ptt_written: Option<Instant> = None;
        let mut pending_freq: Option<f64> = None;
        // Where the rig is tuned, as best this end knows: what the operator has
        // just asked for, else what the radio last reported. Only the mode
        // policy reads it, and only for the one mode whose sideband follows the
        // band (see `commanded_mode`) — a stale answer there costs a mode
        // command that the rig's own report then corrects, which is the same
        // settling the sideband logic upstairs already relies on.
        let mut dial_hz: Option<f64> = None;
        let mut last_sent_freq: Option<f64> = None;
        let mut freq_deadline = Instant::now();
        // Output power, coalesced and rate-limited exactly as the frequency is:
        // dragging the Drive slider produces a command per pixel, and a rig
        // whose control port is served one frame at a time must not be handed
        // hundreds of them — the PTT behind that queue is the thing that would
        // suffer. Only a level that differs from the last one written goes out,
        // so the assertion before every key-down is free once it has settled —
        // until the rig changes mode, which changes the register the level
        // lives in and makes the memory stale (see [`PowerMemory`]).
        let mut pending_power: Option<f32> = None;
        let mut last_sent_power = PowerMemory::default();
        let mut power_deadline = Instant::now();
        // The rig's own receive filter, on the same rate limit and the same
        // only-on-change rule. Unlike the power it is never asserted before a
        // key-down: it is a receive setting, and the one moment it must not
        // compete for the bus is the moment the transmitter is coming up.
        let mut pending_filter: Option<(Mode, f32, f32)> = None;
        let mut last_sent_filter: Option<(Mode, f32, f32)> = None;
        let mut filter_deadline = Instant::now();
        // The squelch, on the same rate limit and the same only-on-change rule
        // as the filter, and skipped during an over for the same reason: it is
        // a receive setting, and the bus belongs to the meters while keyed.
        let mut pending_squelch: Option<f32> = None;
        let mut last_sent_squelch: Option<f32> = None;
        let mut squelch_deadline = Instant::now();
        // The socket this end has put the receiver on since the port opened, so
        // the opening read's answer can be told from the truth (see where it is
        // parsed). `None` until something is commanded, which is where a rig
        // nobody has switched stays.
        let mut last_sent_antenna: Option<&'static str> = None;
        // The receiving antenna's setting as last forwarded, so the re-reads
        // after a band change do not re-notify the engine with what it already
        // has. `None` on a fresh connection, so the first answer always goes.
        let mut emit_rx_antenna: Option<bool> = None;
        let mut mode_memory = ModeMemory::default();
        // Only forward genuine changes so the engine isn't re-notified every poll.
        let mut emit_freq: Option<f64> = None;
        let mut emit_mode: Option<Mode> = None;
        // The dial this end has already put the rig's own repeater shift back
        // to simplex for (see [`Protocol::clear_duplex`] and [`needs_simplex`]).
        // `None` on a fresh connection, so the first dial the rig reports — the
        // opening poll's answer, a round trip after the port opens — asserts it
        // once, and a move after that re-asserts it.
        let mut simplex_dial: Option<f64> = None;
        // The rig's own transmit state as last reported upwards, and when its
        // last answer arrived. `None` = never answered, which is where a family
        // with no such read stays for good.
        let mut emit_rig_tx: Option<bool> = None;
        let mut last_tx_reply = Instant::now();
        // When sdroxide last keyed or unkeyed. A read already on the wire when
        // that happened comes back describing the other side of the edge, so
        // answers are ignored for a moment either side of one — otherwise every
        // unkey ends with one frame's worth of "the operator is on the mic".
        let mut ptt_edge = Instant::now() - PTT_SETTLE;

        let broke = 'io: loop {
            // Drain commands.
            loop {
                match cmd_rx.try_recv() {
                    Ok(CatCmd::Freq(hz)) => {
                        pending_freq = Some(hz); // coalesce
                        dial_hz = Some(hz);
                    }
                    Ok(CatCmd::Mode(m)) => {
                        if let Some(mm) = mode_cmd(m, dial_hz) {
                            let f = protocol.set_mode(mm);
                            if mode_memory.needs(&f) {
                                if write_frame(
                                    &mut *port,
                                    &mut *protocol,
                                    &f,
                                    &mut last_write,
                                    &mut io,
                                ) {
                                    break 'io true;
                                }
                                // The transmitter is now on another mode's
                                // output-power register, so what this end
                                // believes about the level is about a register
                                // the rig has left. The engine asserts the
                                // level that applies immediately behind this
                                // command at every key-down; forgetting is what
                                // lets that assertion through instead of
                                // deduping it away against a stale memory.
                                last_sent_power.forget();
                                // On a rig that shifts its VFO with the mode,
                                // that command has just moved the dial out from
                                // under the operator. Put it back — before the
                                // next poll reads the shifted frequency and
                                // walks the app's dial to it, and before any
                                // key-down, which is the moment being a pitch
                                // off frequency actually costs something.
                                if let Some(hz) = dial_to_restore(
                                    protocol.mode_moves_dial(),
                                    last_sent_freq,
                                    emit_freq,
                                ) {
                                    last_sent_freq = None; // past the dedup
                                    pending_freq = Some(hz);
                                    freq_deadline = Instant::now();
                                }
                            }
                        }
                    }
                    Ok(CatCmd::Ptt(on)) => {
                        // Key-down has to land at the level the engine asserted
                        // for it — the drive, or the tune level under TUNE — so
                        // the rate limit below is not allowed to leave that
                        // sitting in the queue while the transmitter comes up.
                        if on
                            && let Some(frac) = pending_power.take()
                            && write_power(
                                &mut *port,
                                &mut *protocol,
                                frac,
                                &mut last_sent_power,
                                &mut last_write,
                                &mut io,
                            )
                        {
                            break 'io true;
                        }
                        // Key-down has to land on the transmit frequency. With
                        // XIT or split the engine queues the transmit dial
                        // immediately before PTT, and the debounce below would
                        // otherwise let the first moment of the over go out
                        // where we were listening — so flush it first.
                        if on
                            && let Some(hz) = pending_freq.take()
                            && last_sent_freq != Some(hz)
                        {
                            let f = protocol.set_freq(hz);
                            if write_frame(&mut *port, &mut *protocol, &f, &mut last_write, &mut io)
                            {
                                break 'io true;
                            }
                            last_sent_freq = Some(hz);
                            emit_freq = Some(hz); // suppress the poll echo
                            freq_deadline = Instant::now() + Duration::from_millis(50);
                        }
                        // …and on a shift the rig may have re-armed on that
                        // frequency by itself (issue #233). Only when the dial
                        // actually moved for this over — `needs_simplex` says
                        // no for the ordinary case where transmit lands where
                        // we listen, so a digital burst still keys with nothing
                        // queued in front of it.
                        if on {
                            let mut failed = false;
                            if needs_simplex(emit_freq, &mut simplex_dial) {
                                for f in protocol.clear_duplex() {
                                    failed |= write_frame(
                                        &mut *port,
                                        &mut *protocol,
                                        &f,
                                        &mut last_write,
                                        &mut io,
                                    );
                                }
                            }
                            if failed {
                                break 'io true;
                            }
                        }
                        let failed = match cfg.ptt {
                            PttMethod::Vox => false,
                            PttMethod::Rts => {
                                port.set_rts(on);
                                false
                            }
                            PttMethod::Dtr => {
                                port.set_dtr(on);
                                false
                            }
                            PttMethod::Cat => {
                                let f = protocol.ptt(on);
                                ptt_written = on.then(Instant::now);
                                write_frame(
                                    &mut *port,
                                    &mut *protocol,
                                    &f,
                                    &mut last_write,
                                    &mut io,
                                )
                            }
                        };
                        if failed {
                            break 'io true;
                        }
                        ptt = on;
                        ptt_edge = Instant::now();
                        // Ask the meter that belongs to the new state straight
                        // away, rather than showing the other one's last reading
                        // for the rest of the current period.
                        next_meter = Instant::now();
                        if !on {
                            // Clear the readings so the meters drop on unkey.
                            // Both held values go too, or the next over would
                            // open carrying the last one from the previous one.
                            last_swr = None;
                            last_alc = None;
                            last_po = None;
                            last_fwd = None;
                            // Clear the readings so the meters drop on unkey,
                            // here as well as at the receiver: a stale SWR held
                            // locally would be re-sent beside the next over's
                            // first PO reading and briefly look current.
                            let _ = telem_tx.send(TxTelemetry::default());
                        }
                    }
                    // CW the rig keys itself. Deliberately outside the PTT
                    // interlock above: the rig switches to transmit for the
                    // length of the message on its own, and asserting CAT PTT
                    // around it would hold a carrier the keyer cannot key.
                    Ok(CatCmd::Cw(text)) => {
                        // Nothing else asserts the level for this over: the rig
                        // keys itself, so there is no PTT here to hang it off —
                        // and CW is the mode where the rig's power is the only
                        // transmit control there is, the sound card having no
                        // part in it at all.
                        if let Some(frac) = pending_power.take()
                            && write_power(
                                &mut *port,
                                &mut *protocol,
                                frac,
                                &mut last_sent_power,
                                &mut last_write,
                                &mut io,
                            )
                        {
                            break 'io true;
                        }
                        for f in protocol.send_cw(&text) {
                            if write_frame(&mut *port, &mut *protocol, &f, &mut last_write, &mut io)
                            {
                                break 'io true;
                            }
                        }
                    }
                    Ok(CatCmd::CwAbort) => {
                        for f in protocol.abort_cw() {
                            if write_frame(&mut *port, &mut *protocol, &f, &mut last_write, &mut io)
                            {
                                break 'io true;
                            }
                        }
                    }
                    Ok(CatCmd::CwWpm(wpm)) => {
                        for f in protocol.set_cw_wpm(wpm) {
                            if write_frame(&mut *port, &mut *protocol, &f, &mut last_write, &mut io)
                            {
                                break 'io true;
                            }
                        }
                    }
                    // Coalesced exactly as the power is, and for the same
                    // reason: dragging a filter edge produces a command per
                    // pixel, and a rig served one frame at a time must not be
                    // handed hundreds of them.
                    Ok(CatCmd::Filter(m, lo, hi)) => pending_filter = Some((m, lo, hi)),
                    Ok(CatCmd::Power(frac)) => pending_power = Some(frac), // coalesce
                    // Coalesced like the power: this one is a slider too.
                    Ok(CatCmd::Squelch(frac)) => pending_squelch = Some(frac),
                    // Not coalesced and not rate limited: an antenna is a click
                    // on a two-entry list, not a slider being dragged, and it
                    // is one frame either way.
                    Ok(CatCmd::Antenna(name)) => {
                        let frames = protocol.set_antenna(&name);
                        // A name the family does not have produces no frames,
                        // and must not be recorded as where the receiver is.
                        if !frames.is_empty()
                            && let Some(&port) = protocol.antennas().iter().find(|&&a| a == name)
                        {
                            last_sent_antenna = Some(port);
                        }
                        let mut failed = false;
                        for f in frames {
                            failed |= write_frame(
                                &mut *port,
                                &mut *protocol,
                                &f,
                                &mut last_write,
                                &mut io,
                            );
                        }
                        if failed {
                            break 'io true;
                        }
                    }
                    // The receiving antenna, in or out of circuit. One frame,
                    // and only ever from a click — see
                    // [`Protocol::set_rx_antenna`].
                    Ok(CatCmd::RxAntenna(on)) => {
                        let mut failed = false;
                        for f in protocol.set_rx_antenna(on) {
                            failed |= write_frame(
                                &mut *port,
                                &mut *protocol,
                                &f,
                                &mut last_write,
                                &mut io,
                            );
                        }
                        if failed {
                            break 'io true;
                        }
                    }
                    // Asking the rig where its antennas stand, after something
                    // it recalls per band. The suppression `last_sent_antenna`
                    // does is written for the *opening* read crossing a command
                    // on the wire; an answer we have gone and asked for is not
                    // that, and swallowing it would leave the panel showing a
                    // socket the radio has since left.
                    Ok(CatCmd::AntennaRead) => {
                        last_sent_antenna = None;
                        let mut failed = false;
                        for f in protocol.read_antenna() {
                            failed |= write_frame(
                                &mut *port,
                                &mut *protocol,
                                &f,
                                &mut last_write,
                                &mut io,
                            );
                        }
                        if failed {
                            break 'io true;
                        }
                    }
                    // The radio's own power switch. Written straight out
                    // rather than debounced: it is a button an operator
                    // presses, and the wake-up run in front of a power-on is
                    // the one frame here that is *meant* to be long.
                    Ok(CatCmd::RigPower(on)) => {
                        let mut failed = false;
                        for f in protocol.set_rig_power(on, cfg.serial.baud) {
                            failed |= write_frame(
                                &mut *port,
                                &mut *protocol,
                                &f,
                                &mut last_write,
                                &mut io,
                            );
                        }
                        if failed {
                            break 'io true;
                        }
                        // Switching the radio off takes the link with it: the
                        // reads that follow go unanswered, and everything this
                        // end believes about the rig is about to be about a
                        // radio that is not there. Forget it, so a radio
                        // switched back on is re-asserted from scratch rather
                        // than from a cache of how it used to be.
                        if !on {
                            last_sent_freq = None;
                            last_sent_power.forget();
                            last_sent_squelch = None;
                            last_sent_antenna = None;
                            emit_rx_antenna = None;
                            simplex_dial = None;
                            mode_memory = ModeMemory::default();
                        }
                    }
                    Ok(CatCmd::Stop) => return,
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => return,
                }
            }

            // Rate-limited power write (only on change), on the same principle
            // as the frequency below it.
            if let Some(frac) = pending_power {
                let now = Instant::now();
                if now >= power_deadline {
                    pending_power = None;
                    power_deadline = now + POWER_GAP;
                    if write_power(
                        &mut *port,
                        &mut *protocol,
                        frac,
                        &mut last_sent_power,
                        &mut last_write,
                        &mut io,
                    ) {
                        break 'io true;
                    }
                }
            }

            // Rate-limited filter write, on the same principle — and skipped
            // entirely while transmitting, where a receive setting has nothing
            // to do and the bus belongs to the meter.
            if let Some(f) = pending_filter
                && !ptt
            {
                let now = Instant::now();
                if now >= filter_deadline {
                    pending_filter = None;
                    filter_deadline = now + FILTER_GAP;
                    if last_sent_filter != Some(f) {
                        let mut failed = false;
                        for frame in protocol.set_filter(f.0, f.1, f.2) {
                            failed |= write_frame(
                                &mut *port,
                                &mut *protocol,
                                &frame,
                                &mut last_write,
                                &mut io,
                            );
                        }
                        if failed {
                            break 'io true;
                        }
                        last_sent_filter = Some(f);
                    }
                }
            }

            // Rate-limited squelch write, on the same terms as the filter above.
            if let Some(frac) = pending_squelch
                && !ptt
            {
                let now = Instant::now();
                if now >= squelch_deadline {
                    pending_squelch = None;
                    squelch_deadline = now + SQUELCH_GAP;
                    if last_sent_squelch != Some(frac) {
                        let mut failed = false;
                        for frame in protocol.set_squelch(frac) {
                            failed |= write_frame(
                                &mut *port,
                                &mut *protocol,
                                &frame,
                                &mut last_write,
                                &mut io,
                            );
                        }
                        if failed {
                            break 'io true;
                        }
                        last_sent_squelch = Some(frac);
                    }
                }
            }

            // Debounced frequency write (rate-limit to ~50 ms, only on change).
            if let Some(hz) = pending_freq {
                let now = Instant::now();
                if last_sent_freq != Some(hz) && now >= freq_deadline {
                    let f = protocol.set_freq(hz);
                    if write_frame(&mut *port, &mut *protocol, &f, &mut last_write, &mut io) {
                        break 'io true;
                    }
                    last_sent_freq = Some(hz);
                    emit_freq = Some(hz); // suppress the poll echo of our own set
                    pending_freq = None;
                    freq_deadline = now + Duration::from_millis(50);
                }
            }

            // The rig's own repeater shift, put back to simplex whenever the
            // dial has moved to another band — because that is exactly when a
            // radio with band stacking registers puts it back on (issue #192).
            //
            // Driven off `emit_freq` rather than off the write above, so it
            // covers a band changed at the radio's own front panel as well as
            // one commanded from here: that variable is the dial this end has
            // reason to believe the rig is on, however it got there.
            //
            // Never while keyed. Nothing about a shift matters mid-over — the
            // transmit frequency went out with the key-down — and the one
            // moment the bus must not carry a receive setting is the one the
            // meters are being read in.
            if !ptt && needs_simplex(emit_freq, &mut simplex_dial) {
                let mut failed = false;
                for f in protocol.clear_duplex() {
                    failed |= write_frame(&mut *port, &mut *protocol, &f, &mut last_write, &mut io);
                }
                if failed {
                    break 'io true;
                }
            }

            // Poll the rig for external changes — unless it is already telling
            // us about them. A rig with CI-V transceive switched on broadcasts
            // its dial and its mode the moment either moves, which is both
            // faster than any poll and free, so the poll drops back to the
            // safety net that catches a broadcast gone missing.
            if Instant::now() >= next_poll {
                let pushes = protocol.pushes_updates();
                // Both edges are said out loud, because neither is visible from
                // anywhere else: the stand-down arms on the radio volunteering a
                // broadcast and disarms on it moving without one, both of which
                // happen whenever they happen, and each changes how much traffic
                // this thread puts on the wire. Without these lines, anyone
                // measuring the control traffic against audio dropouts cannot
                // tell which of the two rates they were measuring.
                if pushes != announced_push {
                    announced_push = pushes;
                    if pushes {
                        info!(
                            poll_s = PUSHED_POLL_PERIOD.as_secs(),
                            "the radio reports its own dial and mode (CI-V transceive is on); \
                             standing the dial poll down to a safety net"
                        );
                    } else {
                        info!(
                            poll_hz = cfg.poll_hz,
                            "the radio moved without reporting it (CI-V transceive is off); \
                             polling the dial at the configured rate again"
                        );
                    }
                }
                let period = if pushes { PUSHED_POLL_PERIOD } else { poll_period };
                next_poll = Instant::now() + period.max(poll_period);
                // The mode rides along every so often; the rest of the time the
                // poll is the dial on its own. Not while the rig is reporting
                // itself: that poll is already down to one every few seconds,
                // and splitting a frame off something that small buys nothing.
                let with_mode = pushes || Instant::now() >= next_mode_poll;
                if with_mode {
                    next_mode_poll = Instant::now() + mode_period;
                }
                let reqs =
                    if with_mode { protocol.poll_requests() } else { protocol.dial_requests() };
                for req in reqs {
                    if write_frame(&mut *port, &mut *protocol, &req, &mut last_write, &mut io) {
                        break 'io true;
                    }
                }
            }

            // Poll the meter that applies right now: the SWR while keyed, the
            // rig's S-meter while receiving. Both ride the same command on CI-V
            // and only one of them is meaningful at a time, so they take turns
            // rather than sharing the bus.
            //
            // Which one applies is not only our own PTT: an over the operator
            // started at the radio is just as much a transmission, and the
            // meter that matters during it is the same one.
            //
            // The rate is the operator's while receiving and `METER_FLOOR`
            // while transmitting — the SWR feeds the protection trip, and an
            // over is short enough that its traffic is not what an afternoon of
            // receiving costs.
            if Instant::now() >= next_meter {
                let on_air = ptt || emit_rig_tx == Some(true);
                next_meter = Instant::now() + if on_air { METER_FLOOR } else { rx_meter_period };
                let mut reqs = if on_air {
                    protocol.tx_telemetry_requests()
                } else {
                    protocol.rx_telemetry_requests()
                };
                // ...and ask whether the rig has keyed itself. Not while we are
                // the ones keying it: the answer would be our own key-down
                // coming back, and the engine already knows about that.
                if !ptt {
                    reqs.extend(protocol.tx_state_requests());
                }
                for req in reqs {
                    if write_frame(&mut *port, &mut *protocol, &req, &mut last_write, &mut io) {
                        break 'io true;
                    }
                }
            }

            // Start the scope again when its sweeps stop. Several ordinary
            // things stop them — the enable lost on the wire, the radio's own
            // scope screen closed, a menu opened — and nothing reports it, so
            // the strip would otherwise sit dead until a reconnect. The
            // enables are idempotent; the backoff keeps a rig that will never
            // sweep (no scope, or its CI-V USB port still linked to [REMOTE])
            // from being asked twice a second forever.
            if scope_wanted {
                let now = Instant::now();
                if ptt || emit_rig_tx == Some(true) {
                    // A rig does not sweep while it transmits, and an over is
                    // not a stall: hold the clock instead of nudging through it.
                    last_sweep = now;
                } else if now.duration_since(last_sweep) > SCOPE_STALL && now >= next_scope_nudge {
                    next_scope_nudge = now + scope_retry;
                    scope_retry = (scope_retry * 2).min(SCOPE_RETRY_MAX);
                    for f in protocol.scope_requests() {
                        if write_frame(&mut *port, &mut *protocol, &f, &mut last_write, &mut io) {
                            break 'io true;
                        }
                    }
                }
            }

            // Ask again for the socket while the rig has not answered at all.
            // See where `antenna_probes_left` is seeded.
            if antenna_probes_left > 0 && Instant::now() >= next_antenna_probe {
                if learned.antennas_known.load(std::sync::atomic::Ordering::Relaxed) {
                    antenna_probes_left = 0;
                } else {
                    antenna_probes_left -= 1;
                    next_antenna_probe = Instant::now() + ANTENNA_PROBE_RETRY;
                    for f in protocol.read_antenna() {
                        if write_frame(&mut *port, &mut *protocol, &f, &mut last_write, &mut io) {
                            break 'io true;
                        }
                    }
                }
            }

            // A rig that has stopped answering is not a rig that is still
            // transmitting. Without this, one lost reply at the wrong moment
            // would leave the app believing an over is in progress for as long
            // as the session lasts — S-meter blanked, transmit refused.
            if emit_rig_tx == Some(true) && last_tx_reply.elapsed() > RIG_TX_MAX_AGE {
                warn!("the radio stopped answering the transmit read; assuming it is receiving");
                emit_rig_tx = Some(false);
                let _ = event_tx.send(CatUpdate::Ptt(false));
            }

            // Act on whatever has arrived — from the read here and from the
            // waits inside `write_frame`, which leave their bytes and their
            // updates in the same exchange.
            if io.read_once(&mut *port, &mut *protocol).is_none() {
                break 'io true;
            }
            if let Some(sweep) = protocol.take_scope_sweep() {
                last_sweep = Instant::now();
                scope_retry = SCOPE_RETRY;
                *scope_out.lock().unwrap_or_else(|e| e.into_inner()) = Some(sweep);
            }
            // A reply can teach the framing how this particular rig addresses its frequency (see
            // `Protocol::reframed`). What we sent before that was refused and the rig never moved,
            // so the operator's last dial has to go out again — now in terms the rig accepts.
            if protocol.reframed()
                && let Some(hz) = last_sent_freq.take()
            {
                pending_freq = Some(hz);
                freq_deadline = Instant::now();
                // The frequency in this same batch is where the refused set left the rig — not
                // somewhere the operator asked to be. Reporting it would walk the app's dial back
                // to it for the moment before the re-issue lands.
                io.updates.retain(|u| !matches!(u, CatUpdate::Freq(_)));
            }
            // A refusal on its own says nothing — rigs answer that way for every sub-command they
            // don't have, and the offsets cleared at open collect a few. One arriving on the heels
            // of a key-down is worth saying out loud: the operator is looking at a transmitter that
            // did not key, with no other sign of why.
            if protocol.refused()
                && ptt_written.is_some_and(|t| t.elapsed() < Duration::from_millis(500))
            {
                ptt_written = None;
                warn!(
                    "the radio refused a command at key-down — if it did not transmit, \
                     check its CI-V settings, or the PTT method in Settings → Radio"
                );
            }
            // A profile that reads its answers by the order they come back has just proved they no
            // longer line up with the questions. Nothing it has parsed describes this radio and
            // nothing it parses later will either, so the batch in hand goes with the link.
            if protocol.desynced() {
                break 'io true;
            }
            for u in io.updates.drain(..) {
                // The meters are telemetry, not control changes: they go to their own channels and
                // skip the freq/mode dedup below — a reading that repeats is still current, and
                // dropping it would freeze the meter.
                // ⛔ SWR and ALC arrive as SEPARATE replies, and the consumer keeps only the LAST
                // message (`poll_telemetry` is `try_iter().last()`). So sending one field at a time
                // would make each reading blank the other, and the casualty would be the SWR guard:
                // it reads `None` as "the rig has not said anything yet" and would sit at that
                // forever, never tripping. Both are therefore held here and sent together, so every
                // message carries the latest of each.
                let send = |swr, alc, po, fwd_w| {
                    let _ = telem_tx.send(TxTelemetry { fwd_w, swr, alc, po });
                };
                if let CatUpdate::Swr(v) = u {
                    last_swr = Some(v);
                    send(last_swr, last_alc, last_po, last_fwd);
                    continue;
                }
                if let CatUpdate::Alc(v) = u {
                    last_alc = Some(v);
                    send(last_swr, last_alc, last_po, last_fwd);
                    continue;
                }
                if let CatUpdate::Po(v) = u {
                    last_po = Some(v);
                    send(last_swr, last_alc, last_po, last_fwd);
                    continue;
                }
                // Watts the rig measured, on the families whose meter answers
                // in them. Held beside the rest for the same reason they are:
                // it arrives in its own reply, and the consumer keeps only the
                // last message sent.
                if let CatUpdate::FwdW(v) = u {
                    last_fwd = Some(v);
                    send(last_swr, last_alc, last_po, last_fwd);
                    continue;
                }
                if let CatUpdate::Signal(dbm) = u {
                    let _ = signal_tx.send(dbm);
                    continue;
                }
                // The rig's own transmit state. Deduped like the dial (a level re-reported five
                // times a second is not five key-downs), and dropped entirely across one of our own
                // PTT edges, where the answer in hand describes whichever side of the edge the read
                // was issued on.
                if let CatUpdate::Ptt(on) = u {
                    last_tx_reply = Instant::now();
                    if ptt || ptt_edge.elapsed() < PTT_SETTLE {
                        // Still worth recording: this is what the poll above switches meters on,
                        // and after our own over it must not be left saying "keyed".
                        emit_rig_tx = Some(false);
                        continue;
                    }
                    if emit_rig_tx != Some(on) {
                        emit_rig_tx = Some(on);
                        let _ = event_tx.send(u);
                    }
                    continue;
                }
                // The power the rig reports goes straight out: it is asked for once per connection,
                // and the engine adopts it without answering, so there is nothing here to dedup and
                // no loop to break. Recording it as sent is what makes the adoption free — the
                // engine asserts the level it adopted before the first key-down, and that is now a
                // level the rig is already on.
                if let CatUpdate::Power(frac) = u {
                    last_sent_power.set(frac);
                    let _ = event_tx.send(u);
                    continue;
                }
                // The squelch the rig reports, asked for once per connection.
                // Suppressed once this end has set one: the only answer that
                // can arrive after that is the opening read crossing the
                // command on the wire, and adopting it would drag the slider
                // back to where the radio was before the operator moved it.
                if let CatUpdate::Squelch(frac) = u {
                    if last_sent_squelch.is_none() {
                        last_sent_squelch = Some(frac);
                        let _ = event_tx.send(u);
                    }
                    continue;
                }
                // The socket the rig says it is on. Forwarded whole — it is asked for once per
                // connection, so there are no repeats to dedup — unless it disagrees with a socket
                // this end has already commanded on this connection, in which case it is the answer
                // to the opening read crossing that command on the wire. Adopting it there would
                // put the panel back on the port the operator just left, and leave it disagreeing
                // with the radio.
                // The receiving antenna's setting. Held in the atomic the
                // engine reads on every pass, and forwarded on a change only —
                // the re-reads after a band change answer with the same value
                // most of the time. No `last_sent_antenna` suppression: this
                // one is never commanded except by a click, and a click's own
                // echo arriving is the confirmation, not a contradiction.
                if let CatUpdate::RxAntenna(on) = u {
                    // The rig answered the antenna read, so whatever list it
                    // has — including the empty one a single-socket radio
                    // gets — is settled and the probes can stop.
                    learned.antennas_known.store(true, std::sync::atomic::Ordering::Relaxed);
                    learned
                        .rx_antenna
                        .store(1 + u8::from(on), std::sync::atomic::Ordering::Relaxed);
                    if emit_rx_antenna != Some(on) {
                        emit_rx_antenna = Some(on);
                        let _ = event_tx.send(u);
                    }
                    continue;
                }
                if let CatUpdate::Antenna(a) = u {
                    // The radio answered at all, so it has a selector: the
                    // sockets can be offered now (see
                    // [`Protocol::antennas_probed`]). Marked whether or not the
                    // report itself is forwarded — a rig that has one is a rig
                    // that has one, however the answer crossed our command.
                    learned.antennas_known.store(true, std::sync::atomic::Ordering::Relaxed);
                    if last_sent_antenna.is_none_or(|w| w == a) {
                        let _ = event_tx.send(u);
                    }
                    continue;
                }
                // Forward only genuine changes (poll repeats otherwise).
                let changed = match u {
                    CatUpdate::Freq(hz) => {
                        // Where the radio says it is, which is where it is —
                        // including the dial the opening poll finds it on,
                        // before this end has commanded anything.
                        dial_hz = Some(hz);
                        let c = emit_freq.map(|f| (f - hz).abs() >= 1.0).unwrap_or(true);
                        if c {
                            emit_freq = Some(hz);
                        }
                        c
                    }
                    CatUpdate::Mode(m) => {
                        // Also where the rig's mode is learned: what it reports is the truth about
                        // what it is in, and anything that isn't what we last set means the next
                        // mode command has to go out for real.
                        mode_memory.reported(&protocol.set_mode(m));
                        let c = emit_mode != Some(m);
                        if c {
                            emit_mode = Some(m);
                            // The operator turned the mode knob, and the output power went with
                            // it: these families hold a level per mode. What this end last wrote
                            // went into the register the rig has just left, so it is no longer an
                            // answer about the one the next over will transmit on — see
                            // [`PowerMemory`].
                            last_sent_power.forget();
                        }
                        c
                    }
                    // The meters, the power, the squelch and the transmit state are
                    // handled above.
                    CatUpdate::Swr(_)
                    | CatUpdate::Alc(_)
                    | CatUpdate::Po(_)
                    | CatUpdate::FwdW(_)
                    | CatUpdate::Signal(_)
                    | CatUpdate::Power(_)
                    | CatUpdate::Squelch(_)
                    // Both antenna reports are handled above, where they carry
                    // their own dedup and their own suppression.
                    | CatUpdate::Antenna(_)
                    | CatUpdate::RxAntenna(_)
                    | CatUpdate::Ptt(_) => false,
                };
                if changed {
                    let _ = event_tx.send(u);
                }
            }

            std::thread::sleep(Duration::from_millis(5));
        };

        if broke {
            warn!("CAT link error; reconnecting");
            if pause_before_reconnect(&cmd_rx) {
                return;
            }
        } else {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::CatFamily;

    /// The rig's own repeater shift is put back to simplex once per band on HF,
    /// not once per connection: an IC-9700 keeps a band stacking register per
    /// band and restores that band's duplex the moment the dial crosses into
    /// it, which is what left 70 cm and 23 cm stuck on DUP− (issue #192).
    #[test]
    fn the_duplex_is_re_asserted_on_every_band_change() {
        let mut asserted: Option<f64> = None;
        // Nothing is claimed about a rig that has not said where it is.
        assert!(!needs_simplex(None, &mut asserted));
        assert_eq!(asserted, None);
        // The first dial a fresh connection learns arms it once…
        assert!(needs_simplex(Some(14_074_000.0), &mut asserted));
        // …and on HF only once: a dial that moves inside the band changes
        // nothing, and neither does the same frequency arriving on every poll.
        assert!(!needs_simplex(Some(14_074_000.0), &mut asserted));
        assert!(!needs_simplex(Some(14_200_000.0), &mut asserted));
        // A band crossing re-asserts it.
        assert!(needs_simplex(Some(7_074_000.0), &mut asserted));
        // And back again — a band already visited is still a band change.
        assert!(needs_simplex(Some(14_074_000.0), &mut asserted));
    }

    /// Above the VHF floor it is every *move*, not every band: Icom's
    /// auto-repeater arms on the frequency, so tuning between two 2 m channels
    /// puts DUP back without ever crossing a band edge (issue #233).
    #[test]
    fn a_vhf_dial_re_asserts_the_duplex_wherever_it_moves() {
        let mut asserted: Option<f64> = None;
        assert!(needs_simplex(Some(147_000_000.0), &mut asserted));
        // The same frequency arriving on every poll is not a move.
        assert!(!needs_simplex(Some(147_000_000.0), &mut asserted));
        // Another channel in the same band is.
        assert!(needs_simplex(Some(146_875_000.0), &mut asserted));
        assert!(needs_simplex(Some(147_000_000.0), &mut asserted));
        // 70 cm and 23 cm are where the IC-9700 puts DUP− back.
        assert!(needs_simplex(Some(432_500_000.0), &mut asserted));
        assert!(needs_simplex(Some(1_297_000_000.0), &mut asserted));
        // 10 m FM repeaters are above the floor too.
        assert!(needs_simplex(Some(29_680_000.0), &mut asserted));
        assert!(needs_simplex(Some(29_620_000.0), &mut asserted));
    }

    /// `FE FE 00 94 00 …` — the rig telling the bus its dial has moved.
    fn broadcast_freq(hz: f64) -> Vec<u8> {
        let mut b = vec![0xFE, 0xFE, 0x00, 0x94, 0x00];
        b.extend_from_slice(&civ::encode_freq(hz));
        b.push(0xFD);
        b
    }

    /// `FE FE E0 94 03 …` — the rig answering the read this end just sent.
    fn polled_freq(hz: f64) -> Vec<u8> {
        let mut b = vec![0xFE, 0xFE, civ::CONTROLLER_ADDR, 0x94, 0x03];
        b.extend_from_slice(&civ::encode_freq(hz));
        b.push(0xFD);
        b
    }

    fn icom() -> Box<dyn Protocol> {
        make_protocol(&CatConfig {
            family: CatFamily::Icom,
            icom_radio_id: 0x94,
            icom_model: sdroxide_types::IcomModel::Ic7300,
            ..CatConfig::default()
        })
    }

    fn icom_with_scope(baud: u32) -> Box<dyn Protocol> {
        make_protocol(&CatConfig {
            family: CatFamily::Icom,
            icom_radio_id: 0x94,
            icom_model: sdroxide_types::IcomModel::Ic7300,
            scope: true,
            serial: sdroxide_types::SerialConfig { baud, ..Default::default() },
            ..CatConfig::default()
        })
    }

    /// One `27 00` fragment as the rig sends it over serial: division `div` of
    /// `divs`, with `body` carrying the wave information (division 1) or bins.
    fn sweep_fragment(div: u8, divs: u8, body: &[u8]) -> Vec<u8> {
        let mut b = vec![0xFE, 0xFE, civ::CONTROLLER_ADDR, 0x94, 0x27, 0x00, 0x00, div, divs];
        b.extend_from_slice(body);
        b.push(0xFD);
        b
    }

    #[test]
    fn the_scope_is_asked_for_only_on_a_link_its_sweeps_fit_down() {
        // 115200 carries the sweeps; the enable sequence is the LAN backend's:
        // scope on, output on, centre mode, span.
        let fast = icom_with_scope(115_200);
        let reqs = fast.scope_requests();
        assert_eq!(reqs.len(), 4);
        assert_eq!(reqs[0], vec![0xFE, 0xFE, 0x94, 0xE0, 0x27, 0x10, 0x01, 0xFD]);
        assert_eq!(reqs[1], vec![0xFE, 0xFE, 0x94, 0xE0, 0x27, 0x11, 0x01, 0xFD]);
        assert_eq!(&reqs[2][4..7], &[0x27, 0x14, 0x00]);
        assert_eq!(&reqs[3][4..6], &[0x27, 0x15]);
        // 19200 cannot: the sweeps would bury every poll and PTT, so the box
        // being ticked must not put them on the wire.
        assert!(icom_with_scope(19_200).scope_requests().is_empty());
        // And a rig that was never asked sends nothing either.
        assert!(icom().scope_requests().is_empty());
    }

    #[test]
    fn a_fragmented_sweep_reassembles_and_is_taken_once() {
        let mut p = icom_with_scope(115_200);
        // Division 1 carries the wave information and no bins — centre mode,
        // dial at 14.074 MHz, ±50 kHz — exactly as the serial transport splits
        // a sweep the LAN would deliver whole.
        let mut info = vec![0x00];
        info.extend_from_slice(&civ::encode_freq(14_074_000.0));
        info.extend_from_slice(&civ::encode_freq(50_000.0));
        info.push(0x00);
        let mut buf = sweep_fragment(1, 3, &info);
        assert!(p.parse(&mut buf).is_empty());
        assert!(p.take_scope_sweep().is_none(), "half a sweep must not be drawn");

        let mut buf = sweep_fragment(2, 3, &[10u8; 200]);
        assert!(p.parse(&mut buf).is_empty());
        let mut buf = sweep_fragment(3, 3, &[20u8; 275]);
        assert!(p.parse(&mut buf).is_empty());

        let sweep = p.take_scope_sweep().expect("a finished sweep");
        assert_eq!(sweep.center_hz, 14_074_000.0);
        assert_eq!(sweep.span_hz, 100_000.0, "a centred half-span reads as the full width");
        assert_eq!(sweep.bins.len(), 475);
        assert!(sweep.bins[..200].iter().all(|&b| b == 10));
        assert!(sweep.bins[200..].iter().all(|&b| b == 20));
        // A take is a take: the same sweep must not be drawn twice.
        assert!(p.take_scope_sweep().is_none());
    }

    /// The rig's mode is asserted on every key-down. Writing it every time is
    /// what this guards against: the rig acts on each one — re-selecting its
    /// filter, and busy for as long as it takes — with the PTT frame right
    /// behind it.
    #[test]
    fn the_mode_is_only_commanded_when_it_would_change_something() {
        let mut p = icom();
        let mut m = ModeMemory::default();
        // Nothing is known about the rig yet, so the mode goes out.
        assert!(m.needs(&p.set_mode(Mode::Usb)));
        // Asserting the same mode again — every subsequent key-down — does not.
        assert!(!m.needs(&p.set_mode(Mode::Usb)));
        // DIGU shares USB's mode byte on this family but not its DATA switch,
        // so it is a change and has to go out (issue #313).
        assert!(m.needs(&p.set_mode(Mode::Digu)));
        assert!(!m.needs(&p.set_mode(Mode::Digu)));
        // A mode that really is different is written.
        assert!(m.needs(&p.set_mode(Mode::Cw)));
    }

    #[test]
    fn what_the_rig_reports_is_where_the_rig_is() {
        let mut p = icom();
        let mut m = ModeMemory::default();
        assert!(m.needs(&p.set_mode(Mode::Cw)));
        // The operator turns the mode knob on the radio itself. The app follows
        // it there, and commanding it back onto a mode it is already in is
        // exactly the wasted write this avoids.
        m.reported(&p.set_mode(Mode::Lsb));
        assert!(!m.needs(&p.set_mode(Mode::Lsb)));
        // And a mode the rig is *not* in still goes out.
        assert!(m.needs(&p.set_mode(Mode::Usb)));
    }

    /// A mode change on an Elecraft can shift the dial by the CW pitch, so the
    /// frequency is re-asserted behind every mode command. Which frequency is
    /// the whole question: the operator's, not the rig's.
    #[test]
    fn the_dial_put_back_after_a_mode_change_is_the_one_that_was_asked_for() {
        // What we last set wins — that is where the operator asked to be.
        assert_eq!(
            dial_to_restore(true, Some(14_050_000.0), Some(14_050_600.0)),
            Some(14_050_000.0)
        );
        // With nothing set this session, the last frequency the rig reported
        // stands in. It is the pre-shift one: the reply carrying the shifted
        // frequency cannot have arrived before the mode command that caused it.
        assert_eq!(dial_to_restore(true, None, Some(7_030_000.0)), Some(7_030_000.0));
        // Nothing known at all is nothing to put back.
        assert_eq!(dial_to_restore(true, None, None), None);
        // And a family whose dial does not move is not written to at all — a
        // needless frame in front of the next key-down is exactly what the
        // rate limiting elsewhere in this file exists to avoid.
        assert_eq!(dial_to_restore(false, Some(14_050_000.0), Some(14_050_600.0)), None);
    }

    /// Only the family whose radios document the behaviour asks for it.
    #[test]
    fn only_elecraft_re_asserts_the_dial_after_a_mode_change() {
        let family = |f| make_protocol(&CatConfig { family: f, ..CatConfig::default() });
        assert!(family(CatFamily::Elecraft).mode_moves_dial());
        for f in [
            CatFamily::Icom,
            CatFamily::Xiegu,
            CatFamily::Yaesu,
            CatFamily::Kenwood,
            CatFamily::QrpLabs,
        ] {
            assert!(!family(f).mode_moves_dial(), "{f:?}");
        }
    }

    /// Issue #119: a rig put in CW keys its own transmitter and ignores its
    /// sound card, so CW keyed as audio (MCW) must ride the digi sideband.
    /// Commanding CW anyway was a dead key on a Xiegu G90 — switched out of
    /// U-D, it made no power at all.
    #[test]
    fn cw_keyed_as_audio_rides_the_digi_sideband() {
        let cfg = |digi| CatConfig {
            family: CatFamily::Xiegu,
            cw_keying: CwKeying::Audio,
            mode_control: ModeControl::Cat,
            digi_mode: digi,
            ..CatConfig::default()
        };
        let m40 = Some(7_040_000.0);
        assert_eq!(commanded_mode(&cfg(DigiMode::Radio), Mode::Cw, m40), None);
        assert_eq!(commanded_mode(&cfg(DigiMode::Usb), Mode::Cw, m40), Some(Mode::Usb));
        assert_eq!(commanded_mode(&cfg(DigiMode::Data), Mode::Cw, m40), Some(Mode::Digu));
    }

    /// The rig's own keyer can only send with the rig *in* CW, so that route
    /// still commands it — the Icom/Yaesu/Kenwood/Elecraft text keying path
    /// must not change shape.
    #[test]
    fn cw_keyed_by_the_rig_is_still_commanded_as_cw() {
        let cfg =
            |mc| CatConfig { cw_keying: CwKeying::Cat, mode_control: mc, ..CatConfig::default() };
        assert_eq!(commanded_mode(&cfg(ModeControl::Cat), Mode::Cw, None), Some(Mode::Cw));
        assert_eq!(commanded_mode(&cfg(ModeControl::Radio), Mode::Cw, None), None);
    }

    /// DIGU picked at the panel is a rig mode, not a decode layer: it obeys
    /// Mode control verbatim rather than the digi sideband mapping.
    #[test]
    fn an_on_screen_digu_still_obeys_mode_control_not_digi_mode() {
        let cfg = CatConfig { digi_mode: DigiMode::Radio, ..CatConfig::default() };
        assert_eq!(commanded_mode(&cfg, Mode::Digu, Some(7_040_000.0)), Some(Mode::Digu));
    }

    /// The digital modes' sideband choice is not disturbed by how CW is keyed.
    #[test]
    fn ft8_ignores_the_cw_keying_setting() {
        for k in CwKeying::ALL {
            let cfg = CatConfig { cw_keying: k, digi_mode: DigiMode::Data, ..CatConfig::default() };
            // On 40 m as much as on 20 m: only the phone modes' sideband
            // follows the band, and FT8 is not one of them.
            assert_eq!(
                commanded_mode(&cfg, Mode::Ft8, Some(7_074_000.0)),
                Some(Mode::Digu),
                "{k:?}"
            );
        }
    }

    /// SSTV on VHF rides an FM carrier, so the rig has to be put in FM — not on
    /// the digital modes' sideband, whatever `digi_mode` says.
    ///
    /// This is the whole of the "2 m SSTV in FM" complaint: SSTV commanded USB
    /// on every band, and there was no way to ask for FM at all (issue #192).
    /// [`Mode::SstvFm`] is a mode of its own for exactly this reason — every
    /// family's map has to answer differently for it.
    #[test]
    fn sstv_on_fm_is_commanded_as_fm_and_not_on_the_digi_sideband() {
        for digi in [DigiMode::Radio, DigiMode::Usb, DigiMode::Data] {
            let cfg = CatConfig { digi_mode: digi, ..CatConfig::default() };
            let at = Some(145_500_000.0);
            assert_eq!(commanded_mode(&cfg, Mode::SstvFm, at), Some(Mode::SstvFm), "{digi:?}");
        }
        // And what each family actually writes for it is FM: an Icom's own
        // mode byte, and the data-over-FM spelling everywhere the family has
        // one. The two SSTV modes must not land on the same byte.
        assert_eq!(civ::mode_to_civ(Mode::SstvFm), civ::mode_to_civ(Mode::Nfm));
        assert_ne!(civ::mode_to_civ(Mode::SstvFm), civ::mode_to_civ(Mode::Sstv));
        // `Mode control: Radio` still means hands off, as it does for every
        // other mode — an operator who has said not to touch the rig's mode is
        // not overruled by the picture they chose.
        let hands_off = CatConfig { mode_control: ModeControl::Radio, ..CatConfig::default() };
        assert_eq!(commanded_mode(&hands_off, Mode::SstvFm, None), None);
    }

    /// Issue #313: the picture goes out of the same sound card every other
    /// digital mode uses, so analog SSTV obeys `digi_mode` like all of them —
    /// and on the sideband it is actually being used on, which for this one
    /// mode follows the band.
    ///
    /// It used to be left out of that mapping and commanded as `Mode::Sstv`,
    /// which every family but Icom writes as plain phone USB: the over then
    /// left the rig through the microphone input with its speech processing in
    /// the path, or on a radio whose SSB modulation source is the microphone,
    /// with no audio at all.
    #[test]
    fn sstv_rides_the_digi_sideband_on_the_band_it_is_used_on() {
        let cfg = |digi| CatConfig { digi_mode: digi, ..CatConfig::default() };
        // 20 m and up: the upper sideband, like every other digital mode.
        for at in [14_230_000.0, 21_340_000.0, 28_680_000.0] {
            let at = Some(at);
            assert_eq!(commanded_mode(&cfg(DigiMode::Data), Mode::Sstv, at), Some(Mode::Digu));
            assert_eq!(commanded_mode(&cfg(DigiMode::Usb), Mode::Sstv, at), Some(Mode::Usb));
            assert_eq!(commanded_mode(&cfg(DigiMode::Radio), Mode::Sstv, at), None);
        }
        // 160/80/40 m: SSTV is a phone emission and follows phone practice
        // there, so the same choice is between the *lower* pair.
        for at in [1_890_000.0, 3_730_000.0, 7_171_000.0] {
            let at = Some(at);
            assert_eq!(commanded_mode(&cfg(DigiMode::Data), Mode::Sstv, at), Some(Mode::Digl));
            assert_eq!(commanded_mode(&cfg(DigiMode::Usb), Mode::Sstv, at), Some(Mode::Lsb));
            assert_eq!(commanded_mode(&cfg(DigiMode::Radio), Mode::Sstv, at), None);
        }
        // `Mode control` does not get a say, exactly as it does not for FT8:
        // an operator who set the digital modes' sideband meant it.
        let hands_off = CatConfig {
            mode_control: ModeControl::Radio,
            digi_mode: DigiMode::Data,
            ..cfg(DigiMode::Data)
        };
        assert_eq!(commanded_mode(&hands_off, Mode::Sstv, Some(7_171_000.0)), Some(Mode::Digl));
        // And what an Icom writes for the DATA pair really is its DATA switch —
        // the mode byte alone is the plain sideband on that family.
        assert_eq!(civ::set_mode_frames(0x94, Mode::Digu, Some(0x06))[1][6], 0x01);
        assert_eq!(civ::set_mode_frames(0x94, Mode::Digl, Some(0x06))[1][6], 0x01);
        assert_eq!(civ::mode_to_civ(Mode::Digl), civ::mode_to_civ(Mode::Lsb));
    }

    /// Issue #317: RADE is digital voice worked in the phone segments, so it
    /// keeps phone practice the way SSTV does — LSB on 160, 80 and 40 m, USB on
    /// 60 m, 30 m and everything above. It reaches the rig through the same
    /// sound card as every other digital mode, so the choice is still
    /// `digi_mode`'s; what the band decides is only which pair.
    ///
    /// An over sent on the wrong sideband is not merely hard to copy: the
    /// autoencoder's carriers arrive mirrored and the far end decodes nothing
    /// at all.
    #[test]
    fn rade_rides_the_digi_sideband_on_the_band_it_is_used_on() {
        let cfg = |digi| CatConfig { digi_mode: digi, ..CatConfig::default() };
        // The FreeDV calling frequencies on the bands worked upper sideband —
        // 60 m among them, low band or not.
        for at in [5_357_000.0, 10_130_000.0, 14_236_000.0, 21_313_000.0, 28_330_000.0] {
            let at = Some(at);
            assert_eq!(commanded_mode(&cfg(DigiMode::Data), Mode::Rade, at), Some(Mode::Digu));
            assert_eq!(commanded_mode(&cfg(DigiMode::Usb), Mode::Rade, at), Some(Mode::Usb));
            assert_eq!(commanded_mode(&cfg(DigiMode::Radio), Mode::Rade, at), None);
        }
        // And the three bands where phone is worked the other way up.
        for at in [1_890_000.0, 3_625_000.0, 7_177_000.0] {
            let at = Some(at);
            assert_eq!(commanded_mode(&cfg(DigiMode::Data), Mode::Rade, at), Some(Mode::Digl));
            assert_eq!(commanded_mode(&cfg(DigiMode::Usb), Mode::Rade, at), Some(Mode::Lsb));
            assert_eq!(commanded_mode(&cfg(DigiMode::Radio), Mode::Rade, at), None);
        }
    }

    /// `PC` is watts, and the families' documented floor is 5 W — a rig cannot
    /// be asked for nothing, so the bottom of the slider is as low as it goes
    /// rather than a `PC000;` the radio would reject outright.
    #[test]
    fn the_ascii_families_send_power_as_three_digits_of_watts() {
        let sent = |frac: f32| String::from_utf8(pc_set_frame(frac)).unwrap();
        assert_eq!(sent(1.0), "PC100;");
        assert_eq!(sent(0.5), "PC050;");
        assert_eq!(sent(0.05), "PC005;");
        assert_eq!(sent(0.0), "PC005;");
        // A slider cannot ask for more than the assumed full scale.
        assert_eq!(sent(2.0), "PC100;");
    }

    /// The power memory exists to save a frame in front of a PTT, and it is
    /// only allowed to do that while it is still true. A mode change makes it
    /// false: the level lives in a per-mode register on these families, so
    /// after one the operator's Drive level has to reach the radio again.
    ///
    /// This is the whole of the I/Q transmit bug — there the app demodulates
    /// the rig's quadrature itself, so the mode is asserted at key-down, and
    /// every over moved the rig to a register the deduped `PC` never wrote.
    #[test]
    fn a_mode_change_costs_the_power_memory() {
        let mut p = PowerMemory::default();
        assert!(!p.holds(1.0), "nothing has been written yet");
        p.set(1.0);
        assert!(p.holds(1.0), "a level just written need not be written again");
        p.forget();
        assert!(!p.holds(1.0), "the rig changed mode: that register is not the one it is on");
        // And a level adopted from the rig counts as written, which is what
        // makes the opening read free.
        p.set(0.25);
        assert!(p.holds(0.25));
        assert!(!p.holds(1.0));
    }

    /// The control traffic is what the setting is for, so all of it has to be
    /// under the setting. The meters used to run at a fixed 5 Hz underneath the
    /// dial poll, which meant an operator turning the rate down to quieten a
    /// shared USB bus removed half the frames and left the other half running.
    #[test]
    fn the_meters_follow_the_poll_rate_the_operator_set() {
        let at =
            |hz: f32| meter_period(&CatConfig { poll_hz: hz, ..CatConfig::default() }).as_millis();
        // Turning the rate down turns the meter poll down with it.
        assert_eq!(at(1.0), 1000);
        assert_eq!(at(0.5), 2000);
        // ...but never faster than a needle can be read, however high the
        // setting goes: past this the frames buy nothing and cost bus time.
        assert_eq!(at(5.0), METER_FLOOR.as_millis());
        assert_eq!(at(20.0), METER_FLOOR.as_millis());
    }

    /// A reading has to stand in until the next one can arrive, and at a low
    /// poll rate that is a long way off — a fixed window would blank the needle
    /// between every pair of honest answers.
    #[test]
    fn a_meter_reading_outlives_the_gap_to_the_next_one() {
        let at = |hz: f32| signal_max_age(&CatConfig { poll_hz: hz, ..CatConfig::default() });
        for hz in [0.5, 1.0, 2.0, 5.0, 20.0] {
            let cfg = CatConfig { poll_hz: hz, ..CatConfig::default() };
            assert!(at(hz) > meter_period(&cfg), "{hz} Hz");
        }
        // And raising the rate cannot make the needle twitchier than it was.
        assert_eq!(at(20.0), Duration::from_millis(1500));
    }

    /// CI-V transceive: a rig with that setting on broadcasts its dial and its
    /// mode the moment either moves, which is both faster than any poll and
    /// free. Reading those broadcasts is what lets the poll stand down.
    #[test]
    fn an_icom_that_reports_its_own_dial_is_noticed() {
        let mut p = icom();
        // Nothing is claimed until a broadcast has actually arrived: the
        // setting lives in the radio's menu and is off as often as it is on.
        assert!(!p.pushes_updates());
        // An answer to a poll is not a broadcast, however much it looks like
        // one — cmd 0x03 is the reply to the read this end just sent, and it
        // comes back addressed to the controller rather than to the bus.
        assert_eq!(p.parse(&mut polled_freq(14_074_000.0)), vec![CatUpdate::Freq(14_074_000.0)]);
        assert!(!p.pushes_updates());
        // The broadcast is cmd 0x00, addressed to nobody in particular, and
        // carries the same five BCD bytes.
        assert_eq!(p.parse(&mut broadcast_freq(7_055_000.0)), vec![CatUpdate::Freq(7_055_000.0)]);
        assert!(p.pushes_updates());
    }

    /// The mode has its own broadcast (cmd 0x01), and it counts for the same
    /// reason — a rig that reports one reports the other.
    #[test]
    fn a_broadcast_mode_change_counts_as_the_rig_reporting_itself() {
        let mut p = icom();
        let mut buf = vec![0xFE, 0xFE, 0x00, 0x94, 0x01, civ::mode_to_civ(Mode::Cw), 0xFD];
        assert_eq!(p.parse(&mut buf), vec![CatUpdate::Mode(Mode::Cw)]);
        assert!(p.pushes_updates());
    }

    /// Our own frames come back on the bus. A rig address of `E0` is this end
    /// talking, and treating that echo as the radio reporting itself would have
    /// every family stand its poll down on the strength of its own commands.
    #[test]
    fn our_own_echo_is_not_the_rig_reporting_itself() {
        let mut p = icom();
        // `FE FE <radio> E0 05 …` — the set this end just wrote, seen again.
        let mut buf = civ::set_freq_frame(0x94, 14_074_000.0);
        assert!(p.parse(&mut buf).is_empty());
        assert!(!p.pushes_updates());
    }

    /// The claim has to be withdrawable, or an operator who switches Transceive
    /// off mid-session keeps the three-second safety net until they restart.
    /// What withdraws it is the rig turning up somewhere nobody sent it and no
    /// broadcast announced.
    #[test]
    fn a_rig_that_moves_without_saying_so_loses_the_stand_down() {
        let mut p = icom();
        assert_eq!(p.parse(&mut broadcast_freq(14_074_000.0)), vec![CatUpdate::Freq(14_074_000.0)]);
        assert!(p.pushes_updates());
        // The operator switches Transceive off and turns the knob. Nothing is
        // broadcast; the safety-net poll is what finds out, and finding out that
        // way is the proof.
        std::thread::sleep(PUSH_CROSSED_WIRES + Duration::from_millis(50));
        assert_eq!(p.parse(&mut polled_freq(14_080_000.0)), vec![CatUpdate::Freq(14_080_000.0)]);
        assert!(!p.pushes_updates());
    }

    /// ...but a polled answer that agrees with what we already believed says
    /// nothing either way. Every one of them would otherwise be evidence, and
    /// the safety net answers one every three seconds.
    #[test]
    fn a_poll_that_confirms_what_we_knew_is_not_evidence_of_anything() {
        let mut p = icom();
        p.parse(&mut broadcast_freq(14_074_000.0));
        std::thread::sleep(PUSH_CROSSED_WIRES + Duration::from_millis(50));
        for _ in 0..3 {
            p.parse(&mut polled_freq(14_074_000.0));
            assert!(p.pushes_updates());
        }
    }

    /// Nor is our own tuning. We move the rig with `set_freq` and the answer
    /// comes back saying so — a change that arrived without a broadcast, and
    /// entirely legitimately, because this end is the one that caused it.
    #[test]
    fn our_own_tuning_is_not_the_radio_moving_behind_our_back() {
        let mut p = icom();
        p.parse(&mut broadcast_freq(14_074_000.0));
        std::thread::sleep(PUSH_CROSSED_WIRES + Duration::from_millis(50));
        let _ = p.set_freq(18_100_000.0);
        p.parse(&mut polled_freq(18_100_000.0));
        assert!(p.pushes_updates());
    }

    /// A knob being turned broadcasts every step, so a read issued in the
    /// middle of one comes back describing a dial that has already moved on.
    /// That is transceive working, not failing.
    #[test]
    fn a_poll_that_crossed_a_broadcast_on_the_wire_is_excused() {
        let mut p = icom();
        p.parse(&mut broadcast_freq(14_074_000.0));
        // No sleep: the broadcast is still warm, so the stale answer behind it
        // is put down to the two crossing rather than to a silent rig.
        p.parse(&mut polled_freq(14_073_500.0));
        assert!(p.pushes_updates());
    }

    /// And it re-arms: the operator switches Transceive back on, the rig
    /// volunteers a broadcast, and the poll stands down again without a
    /// reconnect.
    #[test]
    fn the_stand_down_comes_back_when_the_rig_starts_reporting_again() {
        let mut p = icom();
        p.parse(&mut broadcast_freq(14_074_000.0));
        std::thread::sleep(PUSH_CROSSED_WIRES + Duration::from_millis(50));
        p.parse(&mut polled_freq(14_080_000.0));
        assert!(!p.pushes_updates());
        p.parse(&mut broadcast_freq(14_090_000.0));
        assert!(p.pushes_updates());
    }

    /// The dial has to keep up with a hand on the VFO knob; the mode is a
    /// setting somebody changes a few times an evening. Every family splits the
    /// two, and the dial half is the frequency read on its own.
    #[test]
    fn the_mode_does_not_ride_along_with_every_dial_poll() {
        for f in CatFamily::ALL {
            let p = make_protocol(&CatConfig { family: f, ..CatConfig::default() });
            let (full, dial) = (p.poll_requests(), p.dial_requests());
            // The dial poll is strictly smaller, and it is the front of the
            // full one — the frequency read, with the mode left off the back.
            assert!(dial.len() < full.len(), "{f:?}");
            assert_eq!(dial, full[..dial.len()], "{f:?}");
        }
    }

    /// The mode's own cadence, which is the dial's divided down and then held
    /// inside a band at both ends.
    #[test]
    fn the_mode_is_read_a_fraction_as_often_as_the_dial() {
        let at = |hz: f32| mode_poll_period(&CatConfig { poll_hz: hz, ..CatConfig::default() });
        // The default: the dial twice a second, the mode every two.
        assert_eq!(at(2.0), Duration::from_secs(2));
        // A fast dial is somebody buying a responsive readout, not a reason to
        // interrogate a setting that has not moved.
        assert_eq!(at(20.0), MODE_POLL_FLOOR);
        // And at the quiet end the two meet rather than the mode running away.
        assert_eq!(at(0.5), Duration::from_secs(5));
        for hz in [0.2, 0.5, 1.0, 2.0, 5.0, 20.0] {
            let cfg = CatConfig { poll_hz: hz, ..CatConfig::default() };
            assert!(mode_poll_period(&cfg) >= poll_period(&cfg), "{hz} Hz");
        }
    }

    /// Issue #282: an IC-9700 worked over a network took the CW the panel sent
    /// it and transmitted nothing, because a message from a PC keys the
    /// transmitter only while `[TRANSMIT]` is on, an external TX switch is
    /// closed, or break-in is on — and the first two are front-panel things.
    #[test]
    fn a_civ_rig_is_put_in_break_in_before_it_is_handed_cw() {
        let civ = || make_protocol(&CatConfig { family: CatFamily::Icom, ..CatConfig::default() });

        // Nothing has been read back, so the switch is asserted: break-in
        // first, the message second, and in that order.
        let mut p = civ();
        let frames = p.send_cw("cq de w1aw");
        assert_eq!(frames.len(), 2, "{frames:?}");
        assert_eq!(frames[0][4..7], [0x16, 0x47, 0x01], "semi break-in");
        assert_eq!(frames[1][4], 0x17, "the message");

        // A rig that has answered "full break-in" is left in it: an operator
        // running QSK has said what they want, and knocking them down to semi
        // on every message would be this end arguing with the front panel.
        // A reply *from* the radio (0x70) *to* the controller (0xE0); the
        // other way round is our own echo, which `parse` skips.
        let reply = |bk: u8| vec![0xFE, 0xFE, 0xE0, 0x70, 0x16, 0x47, bk, 0xFD];
        let mut p = civ();
        p.parse(&mut reply(0x02));
        let frames = p.send_cw("cq de w1aw");
        assert_eq!(frames.len(), 1, "{frames:?}");
        assert_eq!(frames[0][4], 0x17);

        // But one that has answered "off" is exactly the radio the issue was
        // reported on, and it gets the switch.
        let mut p = civ();
        p.parse(&mut reply(0x00));
        assert_eq!(p.send_cw("cq").len(), 2);

        // Nothing sendable is still no frames at all — a message that is only
        // punctuation must not leave the radio keyed in break-in for nothing.
        let mut p = civ();
        assert!(p.send_cw("  <>  ").is_empty());

        // And the read that makes the distinction possible goes out at open.
        assert_eq!(civ().read_break_in().len(), 1);
        for f in [CatFamily::Yaesu, CatFamily::Kenwood, CatFamily::Elecraft, CatFamily::QrpLabs] {
            let p = make_protocol(&CatConfig { family: f, ..CatConfig::default() });
            assert!(p.read_break_in().is_empty(), "{f:?} asserts break-in without asking");
        }
    }

    /// The antenna reply carries two things, and the receiving antenna's half
    /// of it has to ride back out with the next socket command — otherwise
    /// picking ANT2 on an IC-7610 switches the operator's receive aerial out
    /// of circuit (issue #229).
    #[test]
    fn a_receiving_antenna_report_is_carried_back_out_with_the_socket() {
        let mut p = make_protocol(&CatConfig {
            family: CatFamily::Icom,
            icom_model: sdroxide_types::IcomModel::Ic7610,
            ..CatConfig::default()
        });
        // A reply *from* the radio to the controller: ANT2, receiving antenna
        // in circuit.
        let mut reply = vec![0xFE, 0xFE, 0xE0, 0x98, 0x12, 0x01, 0x01, 0xFD];
        assert_eq!(
            p.parse(&mut reply),
            vec![CatUpdate::RxAntenna(true), CatUpdate::Antenna("ANT2")]
        );
        // Now go back to ANT1. The flag the radio reported goes with it.
        let frames = p.set_antenna("ANT1");
        assert_eq!(frames.len(), 1, "{frames:?}");
        assert_eq!(frames[0][4..7], [0x12, 0x00, 0x01], "the receiving antenna was dropped");
        // And switching the receiving antenna itself holds the socket still.
        let frames = p.set_rx_antenna(false);
        assert_eq!(frames[0][4..7], [0x12, 0x01, 0x00]);
    }

    /// A radio with one socket is offered no selector, even though it answers
    /// the read: on an IC-7300MK2 `0x12` is the receiving antenna and nothing
    /// else, and the second position of a chip offered for it would send
    /// `12 01 …`, which that radio rejects.
    #[test]
    fn a_one_socket_model_offers_no_selector() {
        let model = |m| {
            make_protocol(&CatConfig {
                family: CatFamily::Icom,
                icom_model: m,
                ..CatConfig::default()
            })
        };
        let mut p = model(sdroxide_types::IcomModel::Ic7300Mk2);
        assert!(p.antennas().is_empty());
        let mut reply = vec![0xFE, 0xFE, 0xE0, 0xB6, 0x12, 0x00, 0x01, 0xFD];
        assert_eq!(p.parse(&mut reply), vec![CatUpdate::RxAntenna(true)]);
        // Nothing this end can do moves an aerial on it…
        assert!(p.set_antenna("ANT1").is_empty());
        assert!(p.set_antenna("ANT2").is_empty());
        // …and the one control it does have works.
        assert_eq!(p.set_rx_antenna(false)[0][4..7], [0x12, 0x00, 0x00]);

        // Every other model keeps the two it has always been offered.
        assert_eq!(model(sdroxide_types::IcomModel::Ic7610).antennas(), &["ANT1", "ANT2"]);
        assert_eq!(model(sdroxide_types::IcomModel::Ic7300).antennas(), &["ANT1", "ANT2"]);
    }

    /// A radio whose reply is the socket alone has no receiving antenna
    /// connector, and says so by that absence — nothing is reported and the
    /// control never appears.
    #[test]
    fn a_socket_only_reply_reports_no_receiving_antenna() {
        let mut p = make_protocol(&CatConfig {
            family: CatFamily::Icom,
            icom_model: sdroxide_types::IcomModel::Ic7610,
            ..CatConfig::default()
        });
        let mut reply = vec![0xFE, 0xFE, 0xE0, 0x98, 0x12, 0x01, 0xFD];
        assert_eq!(p.parse(&mut reply), vec![CatUpdate::Antenna("ANT2")]);
        // Nothing to switch, so nothing goes out — and nothing this end
        // believes about a connector the radio never mentioned.
        assert!(p.set_rx_antenna(true).is_empty());
        // And the socket goes out ALONE. Sending a byte for a connector the
        // radio has just said it has not got is a frame it answers NAK to, and
        // the aerial does not move: an IC-R8600 owner could read which socket
        // they were on and never change it (issue #334). The frame is the
        // command byte, the socket, and the terminator.
        assert_eq!(p.set_antenna("ANT1")[0][4..], [0x12, 0x00, 0xFD]);
    }

    /// No other family has anything like it, and none of them may claim to.
    #[test]
    fn only_civ_rigs_report_themselves() {
        let family = |f| make_protocol(&CatConfig { family: f, ..CatConfig::default() });
        for f in [
            CatFamily::Yaesu,
            CatFamily::Kenwood,
            CatFamily::Elecraft,
            CatFamily::QrpLabs,
            CatFamily::Rigctld,
            CatFamily::Flrig,
        ] {
            assert!(!family(f).pushes_updates(), "{f:?}");
        }
    }

    /// An FDM-DUO's CAT port is 8N1 at one of four rates, and the default this
    /// config block ships with is none of them — so the link an ELAD owner gets
    /// before touching anything is one the radio cannot hear a word of. Issue
    /// #146: a DUO receiving perfectly and refusing to transmit, on every
    /// serial port its owner tried.
    #[test]
    fn an_elad_is_held_to_the_serial_settings_its_port_has() {
        let elad = |serial: sdroxide_types::SerialConfig| {
            with_family_serial_limits(CatConfig {
                family: CatFamily::Elad,
                serial,
                ..CatConfig::default()
            })
            .serial
        };
        // The untouched default, which is the case that matters.
        let fixed = elad(sdroxide_types::SerialConfig::default());
        assert_eq!(fixed.baud, sdroxide_types::ELAD_DEFAULT_CAT_BAUD);
        assert_eq!(fixed.data_bits, 8);
        assert_eq!(fixed.parity, Parity::None);
        assert_eq!(fixed.stop_bits, StopBits::One);
        // A frame carried over from a rig that wanted something else.
        let fixed = elad(sdroxide_types::SerialConfig {
            baud: 115_200,
            data_bits: 7,
            parity: Parity::Even,
            stop_bits: StopBits::Two,
            ..Default::default()
        });
        assert_eq!(fixed.baud, 115_200, "a rate the radio has is the operator's own");
        assert_eq!(fixed.data_bits, 8);
        assert_eq!(fixed.parity, Parity::None);
        assert_eq!(fixed.stop_bits, StopBits::One);
    }

    /// And nobody else is touched: every other family takes whatever the
    /// operator has set at the radio, including the frames an ELAD cannot use.
    #[test]
    fn no_other_family_has_its_serial_settings_rewritten() {
        let odd = sdroxide_types::SerialConfig {
            baud: 4800,
            data_bits: 7,
            parity: Parity::Even,
            stop_bits: StopBits::Two,
            ..Default::default()
        };
        for f in CatFamily::ALL {
            if f == CatFamily::Elad {
                continue;
            }
            let cfg = with_family_serial_limits(CatConfig {
                family: f,
                serial: odd.clone(),
                ..CatConfig::default()
            });
            assert_eq!(cfg.serial, odd, "{f:?}");
        }
    }

    #[test]
    fn a_pc_reply_reads_back_as_the_fraction_that_set_it() {
        assert_eq!(pc_parse("100"), Some(1.0));
        assert_eq!(pc_parse("050"), Some(0.5));
        assert_eq!(pc_parse("005"), Some(0.05));
        // A rig that puts out more than the assumed full scale reports more
        // watts than we would ever ask for; the slider still tops out at 1.
        assert_eq!(pc_parse("200"), Some(1.0));
        // Not a number is not a power.
        assert_eq!(pc_parse(""), None);
        assert_eq!(pc_parse("abc"), None);
    }
}
