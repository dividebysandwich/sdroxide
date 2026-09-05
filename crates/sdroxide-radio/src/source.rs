use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::{Duration, Instant};

use crate::{Complex32, Result};
use sdroxide_types::Mode;

/// Corner frequency of the front-end DC blocker. Low enough to be invisible at
/// any device rate (10 ppm of a 2 Msps span), high enough to settle in ~8 ms.
pub const DC_BLOCK_HZ: f64 = 20.0;

/// Fraction of the span the LO is parked above the VFO on a zero-IF front end.
/// A quarter keeps every channel filter clear of DC while leaving three
/// quarters of the span *above* the VFO, which is where band activity sits.
const LO_OFFSET_FRAC: f64 = 0.25;

/// Below this rate offset tuning costs more span than it is worth, and the span
/// is too narrow to escape DC by a useful margin anyway.
const LO_OFFSET_MIN_RATE: f64 = 1_000_000.0;

/// LO offset for a zero-IF front end — see [`IqSource::lo_offset_hz`].
///
/// `analog_bw` is the front end's filter bandwidth (0 if it reports none):
/// parking the LO further out than the analog filter reaches would just
/// attenuate the signal we moved out there, so such a device gets no offset
/// and relies on the DC blocker alone.
///
/// This lives here rather than beside the SoapySDR device it was written for,
/// because `device.rs` is behind the `soapy` feature and the policy is not: the
/// native PlutoSDR backend is an AD9361, which is zero-IF for exactly the same
/// reasons and wants exactly the same treatment. Keeping one copy also keeps
/// the two from drifting into disagreeing about where the LO should sit.
///
/// The corollary for a backend that sets its own analog filter: set it *wide*.
/// A device left at a filter narrower than a quarter of its span silently gets
/// no offset at all, which looks identical to the offset not working.
pub fn lo_offset_for(rate: f64, analog_bw: f64) -> f64 {
    if rate < LO_OFFSET_MIN_RATE {
        return 0.0;
    }
    let offset = rate * LO_OFFSET_FRAC;
    if analog_bw > 0.0 && offset > analog_bw * 0.45 { 0.0 } else { offset }
}

/// A change a rig reported out-of-band (the operator turned the dial or
/// changed the mode on the radio itself, or a sibling stream moved shared
/// hardware).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControlUpdate {
    Freq(f64),
    Mode(Mode),
    /// TX drive the rig reports, as a 0..1 fraction (see
    /// [`IqSource::commands_tx_power`]).
    TxDrive(f32),
    /// TUNE drive the rig reports, as a 0..1 fraction.
    TuneDrive(f32),
    /// The squelch threshold the rig reports, as a 0..1 fraction of its own
    /// scale (see [`IqSource::commands_squelch`]).
    ///
    /// The radio's own setting arriving, not a request — read when the control
    /// link opens and *adopted*, the same way the antenna below it is.
    Squelch(f32),
    /// The hardware centre (LO) moved out from under this source: on a
    /// shared-LO device (an AD9361's two receive chains share one
    /// synthesiser), a sibling stream retuned it. The engine *adopts* the new
    /// centre — its span simply is somewhere else now — and must not answer
    /// with a correction, or two engines sharing one LO would chase each
    /// other forever.
    Center(f64),
    /// Which antenna socket the radio says its receiver is on, by the name the
    /// device lists in `DeviceCaps::antennas_rx`.
    ///
    /// The radio's own setting arriving, not a request: it lives in the rig,
    /// survives a power cycle, and is read when the control link opens. The
    /// engine *adopts* it — state and remembered preference both — rather than
    /// putting a session file's port back on top of the one the operator left
    /// the radio on.
    Antenna(&'static str),
    /// Whether the radio's separate *receiving* antenna is switched into the
    /// receive path — an IC-7300MK2's RX ANT IN/OUT, an IC-7610's RX ANT.
    ///
    /// The radio's own setting arriving, and *only* ever arriving: the rig
    /// recalls it per band (an IC-7610 keeps antenna memories at
    /// `1A 05 0276~0287`), so it is read and adopted rather than asserted, and
    /// sdroxide writes it exactly when the operator clicks.
    RxAntenna(bool),
    /// The state of the radio's own PTT line — a foot switch, a mic button, or
    /// whatever is wired to the board's PTT input. `true` is keyed.
    ///
    /// This is a *level*, reported whenever it changes, not a request to
    /// transmit: the engine still puts it through the same interlock, band and
    /// capability rails as the on-screen button, and ignores a key-down that
    /// arrives while something else already owns the transmitter.
    Ptt(bool),
    /// The radio in front of us is transmitting **on its own** — its mic
    /// button, foot switch, VOX or keyer — and sdroxide is not driving the
    /// over. `true` is on the air.
    ///
    /// Emphatically not [`ControlUpdate::Ptt`], and the difference is the whole
    /// point. `Ptt` is a line on hardware sdroxide transmits *through*: closing
    /// it starts an sdroxide over, modulator and all. This is a transceiver
    /// that has keyed itself and is putting its own microphone on the air, so
    /// there is nothing here to drive and nothing to send it — running the
    /// transmit chain would push the computer's audio into a radio that is
    /// already modulating somebody's voice, and on a rig whose key-down doubles
    /// as an audio-source switch (a Kenwood's `TX1;` is DATA SEND) it would cut
    /// them off mid-word.
    ///
    /// So the engine *observes* it: the meter follows the over and shows its
    /// SWR, and sdroxide will not key on top of it. It does not transmit.
    RigTx(bool),
}

/// Anything that produces a stream of complex baseband samples: a live
/// SoapySDR RX stream, a recorded IQ file, or a signal generator.
///
/// This is the seam that lets the whole DSP stack run in CI and without
/// hardware attached.
///
/// # Adding a method here
///
/// [`ConvertedSource`] below wraps an arbitrary `IqSource` and forwards every
/// one of these methods. Almost all of them have a default, so a new method
/// that is not forwarded compiles cleanly and then silently reverts to that
/// default for anyone running a frequency converter. Add the forward at the
/// same time.
pub trait IqSource: Send {
    fn sample_rate(&self) -> f64;
    fn center_hz(&self) -> f64;
    fn set_center_hz(&mut self, hz: f64) -> Result<()>;

    /// How far above the operator's VFO this front end wants its LO parked, or
    /// `0.0` to tune the LO straight to the VFO.
    ///
    /// Zero-IF hardware leaves LO leakage, converter offset and flicker noise
    /// piled up at DC, which is exactly where the VFO would otherwise sit. A
    /// DC blocker takes out the static part, but not the flicker noise or the
    /// signal's own IQ image folding onto it — measured on a HackRF One at
    /// 2 Msps, an FM broadcast station on the LO recovers a 19 kHz pilot 12 dB
    /// above the noise floor with the offset removed, against 26 dB once the
    /// station is moved off DC. So the LO is parked clear of the VFO and the
    /// DDC brings the signal back down; the offset is the engine's business
    /// because it owns retuning (see `keep_vfo_in_span`).
    ///
    /// Default: `0.0`, which is right for low-IF and direct-sampling front ends
    /// (RTL-SDR, HPSDR, TCI) and for demod-audio rigs that have no DDC at all.
    fn lo_offset_hz(&self) -> f64 {
        0.0
    }

    /// Blocking read. Returns the number of samples written to `buf`;
    /// 0 means a timeout (caller should just retry).
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize>;

    /// Read only what the front end already has waiting, returning `Ok(0)`
    /// rather than waiting for more.
    ///
    /// This is what a full-duplex over uses. The engine runs on one thread, and
    /// during an over that thread owes the transmitter a block of samples every
    /// 10 ms; whatever it spends inside a receive call comes out of that budget.
    /// [`Self::read`] is allowed to block for as long as the samples take to
    /// arrive — on a slow front end that is far longer than a transmit block —
    /// so the receive side of an over has to ask for what is there and no more.
    ///
    /// Default: [`Self::read`], which is right for every source that either
    /// cannot transmit or paces itself elsewhere (a file, the signal generator,
    /// a network rig with its own queue). Only SoapySDR devices report full
    /// duplex, so only that implementation needs to override this.
    fn read_available(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        self.read(buf)
    }

    /// Human-readable description for logs/UI.
    fn describe(&self) -> String;

    // Hardware controls — meaningful only for real devices; default no-ops.
    fn set_gain_element(&mut self, _name: &str, _db: f64) -> Result<()> {
        Ok(())
    }
    fn set_antenna(&mut self, _name: &str) -> Result<()> {
        Ok(())
    }
    /// Write one driver-specific setting, by the key the device published for
    /// it (see `DeviceCaps::settings`). A no-op on every backend whose controls
    /// are named fields in its own config block — this exists for SoapySDR,
    /// where the set of controls is whatever the driver says it is.
    fn set_device_setting(&mut self, _key: &str, _value: &str) -> Result<()> {
        Ok(())
    }
    fn current_gains(&self) -> Vec<(String, f64)> {
        Vec::new()
    }
    fn current_antenna(&self) -> String {
        String::new()
    }
    /// The receive antenna sockets the *radio* has since said it has, when that
    /// is not settled at the moment the device is opened.
    ///
    /// `None` — the default — leaves `DeviceCaps::antennas_rx` exactly as it
    /// was built, which is the right answer for every front end whose ports are
    /// a fact about the hardware. A control link is different: every Icom
    /// speaks one dialect, and whether this one has an antenna selector is only
    /// learned when it answers (or NAKs) the read that goes out a round trip
    /// after the port opens. Answering `Some` here re-publishes the list when
    /// it changes — see `Engine::refresh_antennas`, which is the same shape as
    /// `refresh_center_is_dial` and exists for the same reason.
    ///
    /// Borrowed names rather than owned ones, because this is asked on every
    /// pass of the engine loop and a control link's vocabulary is fixed: the
    /// answer is a slice of the family's own constant, and building two short
    /// `String`s a thousand times a second to compare them against the same two
    /// would be the whole cost of the check.
    fn learned_antennas(&self) -> Option<&'static [&'static str]> {
        None
    }
    /// The RX gain stages the front end has **right now**, when their ranges
    /// are not a fact about the hardware alone. `None` — the default — leaves
    /// `DeviceCaps::gains` exactly as it was built.
    ///
    /// Receive stages only, and the whole of them: what comes back replaces
    /// every `Direction::Rx` element in the capabilities, in this order, and
    /// leaves the transmit ones where they are.
    ///
    /// One backend needs this. An RSP's RF gain is a step into a table whose
    /// length is a property of the *band*: an RSPdx has 28 states at 250–420
    /// MHz and 19 below 12 MHz, so a range published once at open is wrong for
    /// most of the spectrum, and a slider built from it spends its top third
    /// moving nothing and snapping back as the driver clamps.
    ///
    /// Asked only where the answer can have changed — after a retune, a port
    /// change or a gain command — rather than on every pass of the engine
    /// loop, because building it allocates.
    fn learned_rx_gains(&self) -> Option<Vec<sdroxide_types::GainElement>> {
        None
    }
    /// Switch the *radio* off, or back on again, over its control link.
    ///
    /// The radio's own power switch, not sdroxide's: an Icom answers `18 00` /
    /// `18 01` on CI-V and keeps the control end of the link alive while the
    /// set is off, which is what lets the second of those reach it (issue
    /// #239). A no-op on every front end whose only power switch is its USB
    /// cable, and the engine does not offer the control there — see
    /// [`Self::commands_rig_power`].
    fn set_rig_power(&mut self, _on: bool) -> Result<()> {
        Ok(())
    }
    /// Whether [`Self::set_rig_power`] reaches this radio, for
    /// `DeviceCaps::commands_rig_power`.
    fn commands_rig_power(&self) -> bool {
        false
    }
    /// Whether this radio has a separate *receiving* antenna connector, and
    /// whether it is switched into the receive path.
    ///
    /// `None` — the default — where it has none, or has not said yet, which is
    /// every SDR: a board's ports are a choice between sockets, not an extra
    /// input switched in alongside the one the transmitter uses. `Some`
    /// becomes `DeviceCaps::has_rx_antenna` (that there is one) and
    /// `RadioState::rx_antenna` (which way it is set).
    ///
    /// Asked on every pass of the engine loop, like [`Self::learned_antennas`],
    /// so it must not allocate or take a lock that anything else holds for
    /// long.
    fn rx_antenna(&self) -> Option<bool> {
        None
    }
    /// Switch that connector into the receive path, or out of it.
    ///
    /// Only ever from an operator's click. The radio recalls the setting per
    /// band on its own, so everywhere else it is read and adopted — see
    /// [`Self::reread_rx_antenna`].
    fn set_rx_antenna(&mut self, _on: bool) -> Result<()> {
        Ok(())
    }
    /// Ask the radio again what its antennas are set to, because something it
    /// recalls per band has just happened — a band change, or a socket
    /// command whose answer belongs to the socket being selected rather than
    /// to the one being left.
    ///
    /// Not an assertion: what comes back is adopted. Free on a front end with
    /// nothing to ask.
    fn reread_rx_antenna(&mut self) {}
    /// Whether the receive port is the source's own to decide, so a remembered
    /// one must not be restored over the top of it.
    ///
    /// The LimeRFE is what this exists for. That front end is one coaxial cable
    /// into one of the LimeSDR's receive sockets, so which socket to listen on
    /// is a fact about the cabling rather than a preference worth carrying
    /// between sessions — and a `session.json` holding the socket some earlier
    /// run happened to land on would keep the radio listening to an empty
    /// connector for good. An operator who wants a different one names it in
    /// the interface's own configuration, which is read at every open.
    fn owns_rx_antenna(&self) -> bool {
        false
    }

    // Transmit path — implemented by transmit-capable devices only.
    // Half-duplex sequencing (pausing RX) is the implementation's job.

    /// Start transmitting: tune the TX LO, apply TX gains, activate the TX
    /// stream. Returns the actual TX sample rate.
    fn tx_begin(&mut self, _center_hz: f64, _rate: f64) -> Result<f64> {
        Err(crate::RadioError::Msg("device is not transmit capable".into()))
    }
    /// Blocking write of complex baseband at the TX rate.
    fn tx_write(&mut self, _samples: &[Complex32]) -> Result<()> {
        Err(crate::RadioError::Msg("device is not transmit capable".into()))
    }
    /// Stop transmitting and restore RX.
    fn tx_end(&mut self) -> Result<()> {
        Ok(())
    }
    /// Discard any RX samples buffered while transmitting, so the first read
    /// after [`Self::tx_end`] returns fresh data instead of a stale backlog.
    /// Default no-op: most sources have no such buffer.
    fn discard_pending_rx(&mut self) {}
    /// Tell a receiver that nobody is going to read it for the length of an
    /// over, and then that somebody is again.
    ///
    /// The engine does not read a half-duplex source while it transmits
    /// ([`DeviceCaps::full_duplex`] false), but a receiver that is not the
    /// transmitter carries on regardless — a network rig streaming its DDC, a
    /// separate SDR lent to a rig as a panadapter, an FDM-DUO whose USB
    /// receiver knows nothing about the PTT line its CAT port just asserted.
    /// Its buffer therefore fills within its own depth of key-down and stays
    /// full until [`Self::discard_pending_rx`] empties it, and every sample
    /// the device delivers in between is discarded. That is the ordinary cost
    /// of transmitting, at exactly the receive rate, for as long as the
    /// operator holds the key — not a host that cannot keep up. Backends that
    /// count discards as overruns need to know which of the two they are
    /// looking at, or a healthy station reports a fault per over and its
    /// running total ends up measuring time on the air.
    ///
    /// Called with `true` on key-down and `false` on key-up, after the backlog
    /// has been discarded. Not called at all for a full-duplex source, which
    /// the engine keeps reading through the over and whose overruns are
    /// therefore all real.
    ///
    /// Default no-op: a source that stops its receiver for the over (every
    /// half-duplex radio that transmits on its own front end) has nothing to
    /// account for, and neither has one that keeps no statistics.
    fn set_rx_paused(&mut self, _paused: bool) {}
    fn set_tx_gain_element(&mut self, _name: &str, _db: f64) -> Result<()> {
        Ok(())
    }
    fn current_tx_gains(&self) -> Vec<(String, f64)> {
        Vec::new()
    }
    /// Select the transmit antenna port. Separate from [`Self::set_antenna`]
    /// because a device with a shared port still lists the two directions
    /// separately, and the names need not match on one that doesn't (a LimeSDR
    /// receives on LNAH/LNAL/LNAW and transmits on BAND1/BAND2).
    fn set_tx_antenna(&mut self, _name: &str) -> Result<()> {
        Ok(())
    }
    fn current_tx_antenna(&self) -> String {
        String::new()
    }
    /// Set the TX drive as a `0..1` fraction on rigs that modulate their own
    /// audio (CAT/TCI), which command output power directly rather than scaling
    /// the transmitted samples. No-op for IQ sources (they apply drive in the
    /// modulator chain instead).
    fn set_tx_drive(&mut self, _frac: f64) {}
    /// A ceiling on transmit drive that the *station* imposes rather than the
    /// operator, as a `0..1` fraction, or `None` where there is none.
    ///
    /// What imposes one is a transverter: its I.F. input takes milliwatts, and
    /// the drive that is right for the radio's own bands destroys it. The
    /// engine holds the operator's setting under this rather than moving the
    /// slider, so the number they set for HF comes back untouched when the dial
    /// leaves the transverter's band (issue #278).
    fn tx_drive_ceiling(&self) -> Option<f32> {
        None
    }
    /// Tell the source where the *dial* is, when that is not where the hardware
    /// is tuned.
    ///
    /// Only a converted source ever calls this, and only an accessory board
    /// cares: a band decoder switching filters and antennas has to follow the
    /// frequency on the air, not the intermediate frequency a transverter left
    /// the radio on. Everything else about a source is stated in hardware
    /// frequencies and stays that way.
    fn set_dial_hz(&mut self, _hz: f64) {}
    /// Set the TUNE drive as a `0..1` fraction (see [`Self::set_tx_drive`]).
    fn set_tune_drive(&mut self, _frac: f64) {}
    /// Whether [`Self::set_tx_drive`] actually commands the rig's output power
    /// (TCI). On such a rig the audio we send is just the modulating signal and
    /// must go out at full scale — the power level is the rig's job, and
    /// attenuating the audio as well would scale the output twice. Sources that
    /// return `false` have no power control, so the audio amplitude *is* the
    /// only drive control (a CAT rig's sound card) or drive is applied in our
    /// own modulator chain (IQ sources).
    fn commands_tx_power(&self) -> bool {
        false
    }
    /// Set the *rig's own* squelch threshold, as a `0..1` fraction of its
    /// scale — `0` open, `1` closed. No-op for sources with no such control.
    fn set_squelch(&mut self, _frac: f32) {}
    /// Whether [`Self::set_squelch`] actually reaches a squelch in the radio.
    ///
    /// True only on a transceiver that hands us audio it has already gated. On
    /// one of those the rig's squelch is the *only* squelch there is: what the
    /// sound card receives has been through it, so a threshold applied on this
    /// side can close further on what got through but can never open what was
    /// shut out — which is how an operator ended up with a squelch control that
    /// could not reach the thing muting their radio (issue #192).
    ///
    /// False on every I/Q front end, where the engine has the whole passband
    /// and its own gate is the honest one.
    fn commands_squelch(&self) -> bool {
        false
    }
    /// Latest forward-power / SWR the rig reported, polled by the engine while
    /// transmitting. `None` when the source has no such sensor or hasn't
    /// produced a reading yet; individual fields inside may also be `None`.
    /// Default: none.
    fn tx_telemetry(&mut self) -> Option<sdroxide_types::TxTelemetry> {
        None
    }
    /// The radio's own temperature in degrees Celsius, where it measures one.
    ///
    /// Polled on the meter tick in receive as well as transmit, because the
    /// number an operator watches is the one *after* the over: what a small PA
    /// does over an afternoon of FT8 is heat up and stay hot, and a reading
    /// that vanished at unkey would hide exactly the part worth seeing
    /// (issue #333).
    ///
    /// Almost nothing reports one — this is the board's own sensor, not a
    /// figure derived from anything on this side — so `None` is the ordinary
    /// answer and the meter simply says nothing. Default: none.
    fn pa_temp_c(&mut self) -> Option<f32> {
        None
    }
    /// The rig's own S-meter in dBm, polled by the engine while receiving.
    ///
    /// For a source that hands us already-demodulated audio (a CAT rig on a
    /// sound card) this is the only signal-strength measurement there is: the
    /// audio arrives after the rig's own filters and AGC, so nothing on this
    /// side of it can tell a strong signal from a weak one. Sources that give
    /// us IQ need none of this — the engine measures the passband itself.
    /// Default: none.
    fn rx_signal_dbm(&mut self) -> Option<f32> {
        None
    }
    /// Absolute gain, in dB, between the antenna socket and the converter that
    /// produced these samples. `None` when the front end cannot say.
    ///
    /// The engine measures the passband in dBFS, which is a level *at the
    /// converter*; the level at the antenna is that minus whatever the front
    /// end put in front of it. Where a source knows that number the engine
    /// takes it off, and what is left is a reading that stays put when the
    /// gain moves — which is the whole point, because on a front end with an
    /// AGC the gain moves continuously and an S-meter that follows it is
    /// measuring the loop rather than the band.
    ///
    /// Default: `None`, which leaves the reading on the old footing — dBFS
    /// plus the operator's fixed calibration offset. That is the right answer
    /// for a front end whose gain nothing reports, and for one whose gain
    /// never moves.
    ///
    /// Must describe the samples being delivered *now*, not a gain that has
    /// been commanded and not yet taken effect: a front end that answers with
    /// the new value while the old samples are still in flight puts a step in
    /// the meter that the antenna never saw.
    fn rx_gain_db(&self) -> Option<f32> {
        None
    }
    /// Offset (Hz) of the operator's VFO from the IQ centre, so a rig that keeps
    /// its own VFO within a wideband IQ stream (TCI) can track the dial while we
    /// tune with a software DDC. No-op for sources whose VFO already equals the
    /// centre or that don't expose a per-VFO offset.
    fn set_if_offset(&mut self, _hz: f64) {}

    /// How long after [`IqSource::set_center_hz`] returns the samples arriving
    /// here are actually on the new centre. Default: no delay worth naming.
    ///
    /// A retune is a command, not an event: the engine learns the new centre
    /// the instant the call returns, but the pipeline behind it is still full
    /// of samples taken at the old one. Label those with the new centre and
    /// they are drawn at the wrong frequency — by the distance the centre moved
    /// in the meantime. Standing still nobody would ever see it, because the
    /// centre stops moving and the picture catches up within one delay. It is a
    /// **drag** that makes it visible: with the view fully zoomed out a pan
    /// sends `SetCenter` once per displayed frame (issue #133), so the label
    /// runs continuously ahead of the data and the whole spectrum sits
    /// displaced by `drag rate × delay` — in the direction of the drag — until
    /// the operator lets go and it snaps back.
    ///
    /// A local USB front end is a millisecond or two of this and nothing to
    /// see. A radio at the end of a socket is not: measured on a SunSDR2DX
    /// through ExpertSDR3's TCI on the loopback interface, **131 ms** at
    /// 192 kHz — of which only 21 ms was sdroxide's own ring, the rest inside
    /// the rig. Nothing on the wire marks it, either: the `dds:` echo comes
    /// back in 0.4 ms, so it acknowledges the command rather than the data.
    ///
    /// Hence a declaration rather than a measurement. Sources that do not
    /// override this are unchanged in every respect — the engine's compensation
    /// is skipped outright at zero.
    fn stream_delay_s(&self) -> f64 {
        0.0
    }

    /// The frequency this radio would transmit on — split, XIT and a satellite
    /// uplink already in it — told to the source *while receiving*, whenever it
    /// changes.
    ///
    /// Distinct from [`Self::tx_begin`], which comes at key-down and is far too
    /// late for the hardware that wants this. Band-switching accessories —
    /// an amplifier, a transverter, a loop antenna's tuner — have to be on the
    /// right band *before* any RF appears, and they cannot work it out for
    /// themselves: the operator changing band is the event they need, not the
    /// operator keying. Default: no-op, which is right for every source with
    /// nothing downstream of it to switch.
    fn set_tx_freq_hz(&mut self, _hz: f64) {}

    // CAT-controlled rigs — meaningful only for the sound-card/CAT source.

    /// Panadapter width (Hz) for a demod-audio source — the engine shows this
    /// slice of the audio band mapped to RF. `None` for normal IQ sources.
    fn display_bandwidth(&self) -> Option<f64> {
        None
    }

    /// A full-band power spectrum, far wider than the IQ this source delivers.
    ///
    /// Fills `out` with dBFS bins ascending from the low edge and returns the
    /// `(centre, span)` in Hz they cover, or `None` when there is no new frame —
    /// which is the answer for every source that has nothing wider to show.
    ///
    /// # Why finished bins rather than samples
    ///
    /// A direct-sampling front end sees its whole Nyquist band at once: 32 MHz
    /// for an RX-888. That cannot cross this trait as IQ — it would be 64 Msps
    /// of complex samples for a picture that is a couple of thousand pixels
    /// wide. The source already has the samples and can analyse a small
    /// fraction of them cheaply, so it hands over the result and the engine
    /// keeps the display policy (pooling to `WIDE_BINS`, the dB window).
    ///
    /// Default: nothing to show.
    fn wide_spectrum_db(&mut self, _out: &mut Vec<f32>) -> Option<(f64, f64)> {
        None
    }

    /// How wide [`IqSource::wide_spectrum_db`]'s window is, in Hz — `0.0` for a
    /// source that publishes none.
    ///
    /// Answered at open, before any frame has been built, because it is what
    /// the client's zoom-out is bounded by: see `DeviceCaps::wide_span_hz`. A
    /// source whose lane is a fixed width knows this from its handshake; one
    /// whose width moves should report the widest it will send.
    ///
    /// Default: no lane.
    fn wide_span_hz(&self) -> f64 {
        0.0
    }

    /// Whether this front end's centre *is* the rig's dial — one synthesiser
    /// doing both jobs.
    ///
    /// True for a transceiver whose I/Q output feeds a sound card and for an
    /// Icom sending its 12 kHz IF: [`Self::set_center_hz`] retunes the radio,
    /// and turning the radio's knob moves the spectrum we capture. It changes
    /// what setting the dial means. On an SDR the window is a resource worth
    /// keeping, so the engine leaves the hardware alone and tunes its DDC until
    /// the VFO would leave the span; here that would leave the radio's readout
    /// and ours showing different numbers, with nothing to reconcile them until
    /// the next report from the rig snapped ours back to its.
    ///
    /// Not the same question as [`crate::DeviceCaps::audio_mode`], which asks
    /// whether there is any IQ at all: a demod-audio rig has no window to tune
    /// within, so it never reaches the distinction this draws.
    ///
    /// May change while the source is open, and is re-asked every block: a rig
    /// whose control port has never answered has no dial for this end to move
    /// (issue #155), and one switched on later gets it back.
    ///
    /// Default: false — a front end that tunes independently of any rig.
    fn center_is_dial(&self) -> bool {
        false
    }

    /// Where a transceiver's I/Q lands when the radio is in CW: on the
    /// frequency its VFO displays (true), or a sidetone pitch below it (false).
    ///
    /// The question exists because a transceiver in CW displays the carrier it
    /// *transmits* while listening a sidetone away from it, and radios settle
    /// that in two different places. Some move their own I.F. by the pitch, so
    /// a station on the displayed frequency arrives already offset and the
    /// stream is a pitch below the readout — the K3's `CW WGHT: VFO OFS`, a QMX
    /// on I/Q. Others hand out the DDC as it is, centred on the VFO, and leave
    /// the offset to whatever demodulates it; an ELAD FDM-DUO does.
    ///
    /// It matters for one thing, and it is not the picture: on the second kind
    /// the frequency being copied is a pitch *above* the VFO, so a radio keying
    /// its own transmitter — a paddle in its socket, text handed to its keyer —
    /// has to be left sitting there and not on our dial, or every over goes out
    /// a sidetone below the station (issue #170).
    ///
    /// Default: false, which is how sdroxide has always treated every rig, so
    /// only a radio this is *known* about should answer otherwise.
    fn cw_iq_on_vfo(&self) -> bool {
        false
    }

    /// Drain any out-of-band changes the rig reported (dial/mode moved on the
    /// radio). Default: none.
    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        Vec::new()
    }
    /// Command the rig's operating mode (CAT). Default: no-op.
    fn set_control_mode(&mut self, _mode: Mode) -> Result<()> {
        Ok(())
    }

    /// Command the rig's own receive filter to the passband the operator picked
    /// here, as audio-band edges in Hz either side of the dial.
    ///
    /// Only a radio that hands over *demodulated audio* needs this, and it needs
    /// it badly: there is no DDC and no demodulator on this side, so the width
    /// control in the panel has nothing to act on — the audio arrives already
    /// filtered, and narrowing it here would only cut what the rig had already
    /// let through, after its AGC had already ridden the interference down.
    /// Passing the width to the radio puts it where it does some good.
    ///
    /// The mode comes with it because every family expresses a filter in terms
    /// of the mode it is in: the same 500 Hz is one index in CW and a different
    /// one in SSB, and some rigs take a width in CW but a pair of slopes in SSB.
    ///
    /// Default: no-op — an SDR's filter is the engine's own, applied to IQ it
    /// has all of.
    fn set_control_filter(&mut self, _mode: Mode, _lo_hz: f64, _hi_hz: f64) {}

    /// The receive offset (RIT) this front end has to carry itself, in Hz —
    /// zero when RIT is off.
    ///
    /// A source with a DDC applies RIT by shifting the DDC and never sees this.
    /// A CAT rig has no DDC: its dial is the only frequency control there is, so
    /// the offset has to go on the dial, which means the source needs to know
    /// how much of the dial is RIT (to subtract it again from what the rig
    /// reports back). The transmit side needs no equivalent — [`Self::tx_begin`]
    /// is already handed the transmit frequency with XIT and split in it.
    ///
    /// Default: no-op.
    fn set_rit_hz(&mut self, _hz: f64) {}

    /// Real receive audio this front end produces *alongside* its I/Q — a
    /// transceiver's own demodulated output, where a second receiver is
    /// supplying the spectrum. Appends to `out` and returns the rate those
    /// samples are at; `None` when there is no such stream.
    ///
    /// Distinct from a demod-audio source ([`DeviceCaps::audio_mode`]), which
    /// has no I/Q at all and delivers its audio through [`Self::read`]. Here
    /// both exist and come from different radios: the engine paints the picture
    /// from the I/Q and plays (and decodes) this instead of what it
    /// demodulated. Announced by [`DeviceCaps::rx_audio_external`], which is
    /// what the engine actually branches on — a source may keep this method
    /// silent for a block without the engine concluding the arrangement is
    /// over.
    ///
    /// Default: nothing, which is right for every ordinary front end.
    fn rx_audio(&mut self, _out: &mut Vec<f32>) -> Option<f64> {
        None
    }

    /// Whether the operator's *receive* mode has to be commanded to this front
    /// end as it changes, and asserted whenever the source is established.
    ///
    /// True where the radio in front of us is the one being worked even though
    /// something else may be doing the demodulating: a panadapter pairing,
    /// whose receiver offset can depend on the mode and whose transceiver's own
    /// display must not disagree with ours. A demod-audio rig needs the same
    /// thing and gets it from [`DeviceCaps::audio_mode`], which the engine has
    /// always keyed this on — the two are kept apart so that path's behaviour
    /// is unchanged.
    ///
    /// False for an SDR, which has no mode of its own, and for a rig that only
    /// transmits for us (TCI): there the mode asserted at key-down is enough.
    fn tracks_rx_mode(&self) -> bool {
        false
    }

    /// Whether an operator's mode change has to be commanded to this front end,
    /// *without* the mode also being imposed when the source is established.
    ///
    /// The weaker half of [`Self::tracks_rx_mode`], for a radio that is the rig
    /// as well as the front end while sdroxide does the demodulating: an Icom
    /// on its LAN port handing us the 12 kHz IF. Its mode still picks the IF
    /// filter the stream comes through, and it is the mode the operator sees on
    /// the radio's own display, so a mode chosen here has to reach it. But the
    /// session must not push one at connect — this backend deliberately adopts
    /// the dial and the mode the transceiver is already sitting on rather than
    /// rearranging somebody's radio out of a config file.
    ///
    /// False everywhere else: an SDR has no mode of its own, and a rig that
    /// only transmits for us is told the mode at key-down.
    fn commands_rx_mode(&self) -> bool {
        false
    }

    /// Whether receive audio must be silenced while this source is
    /// transmitting, without the receiver being stopped.
    ///
    /// Only ever true where the transmitter and the receiver are different
    /// devices, which is the one arrangement where receiving through an over is
    /// both possible and unwanted: the receiver hears our own transmitter.
    /// A half-duplex source ([`DeviceCaps::full_duplex`] false) is not read at
    /// all during an over and needs nothing here.
    ///
    /// Default: false.
    fn mutes_rx_audio_on_tx(&self) -> bool {
        false
    }

    /// Whether receive audio must be silenced while the transceiver in front of
    /// us transmits **under its own control** — someone with their hand on its
    /// microphone ([`ControlUpdate::RigTx`]).
    ///
    /// Separate from [`Self::mutes_rx_audio_on_tx`], and not a spelling of it.
    /// That one answers only for the arrangement where the receiver is read
    /// through an over sdroxide keyed, and is false wherever the engine stops
    /// reading instead — which is how a half-duplex front end goes quiet
    /// without anyone muting anything. Nothing here drives a rig-keyed over, so
    /// the receiver is read right through *every* one of them, and a front end
    /// that would have been stopped for an over of ours has to ask for the
    /// silence explicitly.
    ///
    /// Default: true. An over is an over, and a receiver on the station's own
    /// antenna — or on the rig's I.F. — hears its own transmitter whoever
    /// keyed it. A source with a setting for it says so instead (issue #244).
    fn mutes_rx_audio_on_rig_tx(&self) -> bool {
        true
    }

    /// Write real TX audio to the rig's sound card (used instead of `tx_write`
    /// in demod-audio mode, where the rig does its own modulation).
    fn tx_write_audio(&mut self, _audio: &[f32]) -> Result<()> {
        Err(crate::RadioError::Msg("device has no audio TX path".into()))
    }

    /// How much CW text this radio's own keyer takes in one go, or `None` when
    /// CW is sent the ordinary way — as keyed audio through the transmit chain.
    ///
    /// A transceiver put into CW keys its own transmitter: it does not modulate
    /// what arrives at its sound card, so a keyer's sidetone written there goes
    /// nowhere at all. Such a radio can only send CW from text handed to its
    /// keyer, which is what [`Self::send_cw`] does — and what this reports it
    /// can do. Default: an SDR, whose transmit chain keys the sidetone itself.
    fn cw_text_keying(&self) -> Option<usize> {
        None
    }
    /// True when CW typed at the panel goes out as keyed audio (MCW) through
    /// this radio's transmit sound path, so the radio must be kept on a
    /// sideband rather than put in CW — in CW it would not modulate its sound
    /// card at all (issue #119: a Xiegu G90 switched out of U-D made no power).
    ///
    /// Deliberately not `cw_text_keying().is_none()`: that is also `None` for
    /// an SDR (which keys the sidetone through its own TX chain and has no rig
    /// mode to protect) and for rigs whose keyer sdroxide cannot drive but
    /// that genuinely belong in CW (ELAD paddle CW, rigctld). Only an explicit
    /// "Sound card (MCW)" choice answers true.
    fn cw_audio_keyed(&self) -> bool {
        false
    }
    /// True when this radio's control layer works out for itself which sideband
    /// a mode whose sideband follows the band rides — analog SSTV and RADE, LSB
    /// on 160/80/40 m and USB above (`Mode::sideband_follows_band`).
    ///
    /// The engine translates those into plain LSB before handing them over (see
    /// `Engine::control_mode`), because a family's mode map is a table with no
    /// dial in it. A CAT rig is the exception: there the mode goes through
    /// `digi_mode`, the operator's choice between the plain sideband and the
    /// rig's DATA position, and that choice has to survive on both sidebands —
    /// a translation to plain LSB spends it (issue #313). Such a radio answers
    /// true and is sent [`Mode::Sstv`] or [`Mode::Rade`] itself, dial and all.
    fn resolves_band_sideband(&self) -> bool {
        false
    }
    /// Hand `text` to the radio's own keyer, at most `cw_text_keying()` worth,
    /// and not again until the last lot has been sent.
    ///
    /// Carries no PTT: the radio switches to transmit for the length of the
    /// message on its own, and a transmitter already keyed by CAT is one its
    /// keyer cannot key.
    fn send_cw(&mut self, _text: &str) {}
    /// Stop a message the radio is part way through sending.
    fn abort_cw(&mut self) {}
    /// Tell the radio's keyer what speed to send at. It keys at its own speed,
    /// so until this arrives the panel's WPM is not what goes on the air.
    fn set_cw_wpm(&mut self, _wpm: f32) {}

    /// Block until queued TX audio has been played out, so PTT can be released
    /// without cutting off the tail of a burst. Default: nothing is buffered.
    fn tx_drain(&mut self) {}

    /// How far ahead of real time the engine may fill this source's transmit
    /// ring before it paces production back down to real time, in ms.
    ///
    /// The engine feeds TX audio at real time plus this cushion (see
    /// `pace_tx_block` in `engine.rs`) so the device/network ring downstream
    /// stays near-empty rather than filling to its full depth — that ring's
    /// depth is otherwise wasted as pure latency. The cushion is what absorbs
    /// jitter between one feed and the next before the ring runs dry: a stall
    /// (scheduling, USB round trip, network transit) shorter than this many ms
    /// is invisible; a longer one underruns and is heard as a chopped
    /// transmission. Default `30.0` ms is right for hardware reached directly
    /// (sound card, USB, LAN) whose own jitter is negligible. A source
    /// reached over a link with real jitter — WiFi, a VPN — should report
    /// more, trading transmit-audio and PTT latency for headroom, the same
    /// tradeoff the Icom LAN backend's `tx_latency_ms` setting makes
    /// explicit.
    fn tx_pace_cushion_ms(&self) -> f64 {
        30.0
    }

    /// A user-facing warning captured while opening the source (e.g. the radio
    /// audio device was unavailable, or a mono card was selected for IQ), or
    /// `None` when the source came up cleanly. Surfaced in the UI so a silent
    /// failure doesn't just read as "waiting for spectrum". Default: none.
    fn open_status(&self) -> Option<String> {
        None
    }

    /// Whether this source is a stand-in for a radio that isn't actually
    /// connected — the placeholder used when the configured interface couldn't
    /// be opened at startup, or a network rig whose link has since dropped. The
    /// engine retries the configured interface in the background while this is
    /// true, so a rig that appears (or reappears) attaches on its own instead of
    /// waiting for Settings → Radio → Apply. Default: a real device.
    fn needs_reopen(&self) -> bool {
        false
    }

    /// Give up any hardware held exclusively, ahead of the engine building this
    /// front-end's replacement.
    ///
    /// A USB dongle is claimed by exactly one handle at a time, and the kernel
    /// does not care that the second claim comes from the same process: opening
    /// the replacement while the outgoing source still holds the interface
    /// fails as "device busy", which reaches the operator as the rather
    /// alarming claim that another program has taken their radio. Pressing
    /// Apply in Settings → Radio with an RTL-SDR running did exactly that. So
    /// the engine tells the outgoing source to stand down first.
    ///
    /// Called only on the reopen paths, where the source is on its way out
    /// regardless. Implementations must be idempotent, and must leave the
    /// source inert but still callable — `read` delivering nothing rather than
    /// panicking — and reporting [`IqSource::needs_reopen`], so that a reopen
    /// which then fails leaves the engine retrying in the background rather
    /// than holding a corpse it will never replace.
    ///
    /// Default: nothing to release. That is right for file, signal-generator
    /// and network sources, and it is the better behaviour where it applies —
    /// a replacement built alongside the source it replaces means a bad new
    /// config leaves the working interface on air.
    fn release(&mut self) {}
}

/// A front end with an external frequency converter in its antenna line: an HF
/// upconverter (Ham It Up, SpyVerter) or a receive converter for a band the
/// hardware cannot reach.
///
/// The converter mixes the whole spectrum up by a fixed amount, so `10.1008 MHz`
/// on the air arrives at the receiver as `135.1008 MHz`. This wrapper is the one
/// place that arithmetic happens: everything above it — the engine, the UI, the
/// CAT servers, memories, spots, the logbook — works in the operator's
/// frequency, and everything below it in the hardware's.
///
/// # The transmit path is its own question
///
/// A converter sits between the antenna and the receiver *input*, so what the
/// transmit line does is a separate fact about the station that only its
/// operator knows — and `tx_offset_hz` is where they say it. `None` withdraws
/// transmit ([`shift_caps`] takes the capability away entirely), which is the
/// default because guessing the receive offset would be worse than useless: the
/// licence gate in the engine checks the operator's frequency, which would pass
/// on 30 m and then key the radio 125 MHz up, in an aeronautical band.
///
/// `Some(t)` translates transmit by `t` instead — the same sign rule, so a
/// transverter that converts both ways passes its receive offset and a receive
/// converter with the transmitter on its own antenna passes `0.0`. The latter is
/// the QO-100 station: the 10 GHz downlink arrives through an LNB while the
/// 2.4 GHz uplink leaves the radio direct, and only the receive side is offset.
pub struct ConvertedSource {
    inner: Box<dyn IqSource>,
    plan: ConverterPlan,
    /// The step the dial has selected. Held rather than looked up per call
    /// because [`Self::down`] has only a *hardware* frequency to work from, and
    /// which step took it there is not recoverable from that number — two
    /// transverters on one I.F. hand back the same 28 MHz.
    step: ConverterStep,
}

impl ConvertedSource {
    /// One converter in front of the whole receiver: an upconverter, an LNB.
    pub fn new(inner: Box<dyn IqSource>, offset_hz: f64, tx_offset_hz: Option<f64>) -> Self {
        ConvertedSource::with_plan(
            inner,
            ConverterPlan::single(ConverterStep {
                band: None,
                rx_offset_hz: offset_hz,
                tx_offset_hz,
                tx_drive: None,
            }),
        )
    }

    /// A station with more than one box in front of it — see [`ConverterPlan`].
    pub fn with_plan(inner: Box<dyn IqSource>, plan: ConverterPlan) -> Self {
        // Seeded by working *backwards* from where the front end already is.
        // Nothing has told us a dial yet, and the first thing the engine does
        // is read `center_hz()` — so a source that started on a transverter's
        // I.F. has to say so, or the radio comes up on 28 MHz having been
        // opened for 144.
        let step = plan.step_from_hardware(inner.center_hz());
        ConvertedSource { inner, plan, step }
    }

    /// The plan this source is following, for the caller that has to publish
    /// the same tuning ranges — see [`plan_caps`].
    pub fn plan(&self) -> &ConverterPlan {
        &self.plan
    }

    /// Hardware frequency → operator frequency, through the selected step.
    fn down(&self, hz: f64) -> f64 {
        hz - self.step.rx_offset_hz
    }

    /// Operator frequency → hardware frequency, on the transmit path.
    ///
    /// The step is chosen by the *transmit* frequency, which split and XIT
    /// make a different question from where the receiver is listening.
    fn up_tx(&self, hz: f64) -> Option<f64> {
        self.plan.step_for(hz).tx_offset_hz.map(|t| hz + t)
    }
}

/// One band of a converter plan: what the hardware does while the dial is in it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ConverterStep {
    /// The dial range this step covers, in Hz, or `None` for every frequency —
    /// which is what a converter in front of the whole receiver is.
    pub band: Option<(f64, f64)>,
    /// `hardware = dial + rx_offset_hz`.
    pub rx_offset_hz: f64,
    /// The same for transmit; `None` withdraws transmit on this band.
    pub tx_offset_hz: Option<f64>,
    /// A ceiling on transmit drive while this step is selected, 0.0–1.0.
    /// `None` is no ceiling — the radio's own bands.
    pub tx_drive: Option<f32>,
}

impl ConverterStep {
    /// Whether this step does anything at all: a zero offset that neither
    /// withdraws transmit nor limits drive is the bare radio.
    fn is_identity(&self) -> bool {
        self.rx_offset_hz == 0.0
            && self.tx_offset_hz == Some(0.0)
            && self.tx_drive.is_none()
            && self.band.is_none()
    }

    fn covers(&self, dial_hz: f64) -> bool {
        match self.band {
            None => true,
            Some((lo, hi)) => (lo..=hi).contains(&dial_hz),
        }
    }
}

/// What is in front of the radio, band by band.
///
/// One entry is the ordinary case and the one sdroxide shipped first: an
/// upconverter or an LNB that converts every frequency. A station with
/// transverters has several, each covering the band it works, and the dial
/// decides which applies — a 2 m and a 6 m transverter sharing a 28 MHz I.F.
/// are two steps with different offsets and the same hardware behind them
/// (issue #278).
///
/// Steps are tried in order and the first that covers the dial wins, so a
/// band-limited step must come before the catch-all one. [`Self::from_steps`]
/// does that sorting; nothing else needs to think about it.
///
/// A dial no step covers is *not* converted and *may* transmit: it is the radio
/// on its own bands, which is exactly what a station with a 2 m transverter
/// still has on HF.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ConverterPlan {
    steps: Vec<ConverterStep>,
}

impl ConverterPlan {
    /// The bare radio: nothing in front of it.
    pub fn none() -> ConverterPlan {
        ConverterPlan { steps: Vec::new() }
    }

    /// One converter over the whole dial.
    pub fn single(step: ConverterStep) -> ConverterPlan {
        ConverterPlan { steps: vec![step] }
    }

    /// Build a plan, putting the band-limited steps ahead of any catch-all.
    pub fn from_steps(steps: impl IntoIterator<Item = ConverterStep>) -> ConverterPlan {
        let mut steps: Vec<ConverterStep> = steps.into_iter().collect();
        steps.sort_by_key(|s| u8::from(s.band.is_none()));
        ConverterPlan { steps }
    }

    pub fn steps(&self) -> &[ConverterStep] {
        &self.steps
    }

    /// Whether this plan leaves the radio exactly as it is.
    pub fn is_transparent(&self) -> bool {
        self.steps.iter().all(ConverterStep::is_identity)
    }

    /// The step for a dial frequency — the bare radio where none covers it.
    pub fn step_for(&self, dial_hz: f64) -> ConverterStep {
        self.steps
            .iter()
            .find(|s| s.covers(dial_hz))
            .copied()
            .unwrap_or(ConverterStep { tx_offset_hz: Some(0.0), ..ConverterStep::default() })
    }

    /// `hardware = dial + this`, for a dial.
    pub fn offset_for(&self, dial_hz: f64) -> f64 {
        self.step_for(dial_hz).rx_offset_hz
    }

    /// The step for a *hardware* frequency: the one whose offset puts it back
    /// inside its own band.
    ///
    /// The inverse is not unique — two transverters on one I.F. hand back the
    /// same 28 MHz — so this is only ever used to seed a source from where the
    /// front end was found, before any dial has been commanded. The first
    /// match wins, which is the order the plan lists them in.
    pub fn step_from_hardware(&self, hw_hz: f64) -> ConverterStep {
        self.steps
            .iter()
            .find(|s| s.covers(hw_hz - s.rx_offset_hz))
            .copied()
            .unwrap_or(ConverterStep { tx_offset_hz: Some(0.0), ..ConverterStep::default() })
    }
}

/// Move a device's published tuning ranges into the operator's domain, and
/// either translate its transmit capability or take it away (see
/// [`ConvertedSource`]).
///
/// The edges are clamped at DC and ranges that collapse are dropped: a negative
/// frequency is not one, and it would read as nonsense in the "outside this
/// radio's receive range" notice and in what the rigctld and TCI servers
/// advertise to their clients.
///
/// `tx_offset_hz` of `None` withdraws transmit entirely — no ranges, no
/// antennas, no channel — so the engine's gate refuses a key-down before
/// anything reaches the hardware. `Some(t)` shifts the transmit ranges by `t`
/// instead, leaving the gate to vet the operator's transmit frequency against
/// where the radio can actually put its transmit LO.
pub fn shift_caps(
    mut caps: sdroxide_types::DeviceCaps,
    offset_hz: f64,
    tx_offset_hz: Option<f64>,
) -> sdroxide_types::DeviceCaps {
    fn shift(ranges: Vec<(f64, f64)>, offset_hz: f64) -> Vec<(f64, f64)> {
        ranges
            .into_iter()
            .map(|(lo, hi)| ((lo - offset_hz).max(0.0), hi - offset_hz))
            .filter(|&(lo, hi)| hi > lo)
            .collect()
    }
    caps.freq_ranges_rx = shift(caps.freq_ranges_rx, offset_hz);
    match tx_offset_hz {
        Some(tx) => caps.freq_ranges_tx = shift(caps.freq_ranges_tx, tx),
        None => {
            caps.freq_ranges_tx.clear();
            caps.antennas_tx.clear();
            caps.tx_channels = 0;
        }
    }
    caps
}

/// The tuning ranges a front end behind a converter publishes, from the two
/// answers that exist about them.
///
/// The order is the whole of issue #279. What the *device* says is in the
/// hardware's own domain, so [`shift_caps`] moves it onto the dial; what the
/// *operator* typed is already on the dial, because a transverter owner who
/// writes 144-148 means the band they tune and not the 28-32 MHz I.F. their
/// radio is really sitting on. So the shift goes on first and the stated ranges
/// replace the result — applying the offset to a typed range put the limit a
/// whole offset away from the band it named, and a 2 m transverter came up
/// refusing everything outside 260-264 MHz.
///
/// A stated transmit range cannot put back a transmitter the operator withdrew:
/// with `tx_offset_hz` of `None`, [`shift_caps`] has taken transmit away and it
/// stays away.
pub fn converted_caps(
    caps: sdroxide_types::DeviceCaps,
    offset_hz: f64,
    tx_offset_hz: Option<f64>,
    stated_rx: &[(f64, f64)],
    stated_tx: &[(f64, f64)],
) -> sdroxide_types::DeviceCaps {
    let shifted = shift_caps(caps, offset_hz, tx_offset_hz);
    let mut caps = override_caps_ranges(shifted, stated_rx, stated_tx);
    if tx_offset_hz.is_none() {
        caps.freq_ranges_tx.clear();
    }
    caps
}

/// The tuning ranges a front end behind a whole [`ConverterPlan`] publishes.
///
/// The union of what each step can reach, clipped to the band that step covers,
/// plus — where no step covers everything — the radio's own ranges for the
/// bands nothing is in front of. That last part is what keeps HF on a station
/// whose only converter is a 2 m transverter.
///
/// The stated ranges go on top of the lot, on the dial, for the reason
/// [`converted_caps`] gives.
pub fn plan_caps(
    caps: sdroxide_types::DeviceCaps,
    plan: &ConverterPlan,
    stated_rx: &[(f64, f64)],
    stated_tx: &[(f64, f64)],
) -> sdroxide_types::DeviceCaps {
    /// One step's view of a set of hardware ranges, on the dial.
    fn through(ranges: &[(f64, f64)], offset_hz: f64, band: Option<(f64, f64)>) -> Vec<(f64, f64)> {
        ranges
            .iter()
            .filter_map(|&(lo, hi)| {
                let (mut lo, mut hi) = ((lo - offset_hz).max(0.0), hi - offset_hz);
                if let Some((blo, bhi)) = band {
                    lo = lo.max(blo);
                    hi = hi.min(bhi);
                }
                (hi > lo).then_some((lo, hi))
            })
            .collect()
    }

    let mut out = caps.clone();
    let covers_all = plan.steps().iter().any(|s| s.band.is_none());
    let mut rx: Vec<(f64, f64)> = if covers_all { Vec::new() } else { caps.freq_ranges_rx.clone() };
    let mut tx: Vec<(f64, f64)> = if covers_all { Vec::new() } else { caps.freq_ranges_tx.clone() };
    for step in plan.steps() {
        rx.extend(through(&caps.freq_ranges_rx, step.rx_offset_hz, step.band));
        if let Some(t) = step.tx_offset_hz {
            tx.extend(through(&caps.freq_ranges_tx, t, step.band));
        }
    }
    out.freq_ranges_rx = rx;
    out.freq_ranges_tx = tx;
    // Transmit is withdrawn outright only when *nothing* may transmit: a
    // station that keys on HF and not through its transverter still has a
    // transmitter, and the ranges above are what hold it to the right bands.
    if out.freq_ranges_tx.is_empty() && !caps.freq_ranges_tx.is_empty() {
        out.antennas_tx.clear();
        out.tx_channels = 0;
    }
    let stated_tx: &[(f64, f64)] = if out.tx_channels == 0 { &[] } else { stated_tx };
    override_caps_ranges(out, stated_rx, stated_tx)
}

/// Which hardware frequency to open a converted front end on, for the dial the
/// engine is holding — the current one on a runtime interface change, the
/// restored session's at startup.
///
/// Normally that is the one rule the whole feature is built on: the hardware is
/// tuned to `dial + offset`. The exception is the dial that predates the
/// converter, and every operator who sets one up makes it — they are listening
/// to the converter's output on 739.494 MHz, and *then* tell sdroxide there is
/// a 9750 MHz LNB in front of the radio. That dial is in the hardware's domain,
/// so putting the offset on it asks the front end for −9010.505 MHz.
///
/// Below DC is not a frequency, and it is taken as the proof of what the number
/// is: the dial is read as the hardware frequency it plainly is, the front end
/// opens exactly where it already was, and [`ConvertedSource`] hands the dial
/// back re-labelled — 739.494 MHz becomes 10489.494 MHz, the same signal under
/// the number the operator wanted to see when they said there was an LNB.
///
/// What this replaces is not a smaller inconvenience: several back ends clamp a
/// tune they cannot make (a Pluto asked for a negative frequency lands at
/// 46.875 MHz) and report the number they were *given* rather than the one they
/// took, which left the engine holding a dial outside the converted receive
/// range, a receiver pointed somewhere else entirely, and every subsequent tune
/// refused as "outside this radio's receive range".
pub fn converter_open_hz(dial_hz: f64, offset_hz: f64) -> f64 {
    let hw = dial_hz + offset_hz;
    // `> 0.0` and not `>= 0.0`: DC itself is no more openable than below it.
    // The second clause is the same test on the dial — a dial at or below zero
    // is not a frequency to read as a hardware one either, so that sum goes on
    // down and the back end refuses it rather than this inventing something.
    if hw > 0.0 || dial_hz <= 0.0 { hw } else { dial_hz }
}

/// Replace what a device says about its own tuning ranges with what the
/// operator says, for whichever direction they gave an answer for.
///
/// Both lists are in the hardware's own domain — the same one the device
/// publishes in — so a converter offset is applied to them afterwards, not
/// before. Nonsensical pairs are dropped rather than refused: this runs while
/// the radio is being opened, where the worst outcome is not "a bad range" but
/// "no radio", and the settings dialog has already refused anything malformed
/// at the point it was typed.
///
/// A stated transmit range does not make a receiver into a transceiver — a
/// device with no TX channel has no transmitter to unlock, and this leaves
/// `tx_channels` exactly as the device reported it.
pub fn override_caps_ranges(
    mut caps: sdroxide_types::DeviceCaps,
    rx: &[(f64, f64)],
    tx: &[(f64, f64)],
) -> sdroxide_types::DeviceCaps {
    fn sane(ranges: &[(f64, f64)]) -> Vec<(f64, f64)> {
        ranges
            .iter()
            .copied()
            .filter(|&(lo, hi)| lo.is_finite() && hi.is_finite() && lo >= 0.0 && hi > lo)
            .collect()
    }
    let rx = sane(rx);
    let tx = sane(tx);
    if !rx.is_empty() {
        caps.freq_ranges_rx = rx;
    }
    if !tx.is_empty() {
        caps.freq_ranges_tx = tx;
    }
    caps
}

impl IqSource for ConvertedSource {
    // --- translated ---------------------------------------------------------

    fn center_hz(&self) -> f64 {
        self.down(self.inner.center_hz())
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        // Chosen but not committed: a refused tune must leave the source on the
        // step it was already using, or the *next* `down()` would relabel the
        // frequency the receiver is still sitting on.
        let step = self.plan.step_for(hz);
        let hw = hz + step.rx_offset_hz;
        // Refused here rather than passed down, because several back ends
        // (RTL-SDR, RX-888, HPSDR) answer `Ok` to any frequency and then clamp
        // inside their DDC. That would leave the engine believing the tune
        // succeeded while the receiver sat somewhere else entirely; an error
        // goes through the normal refused-tune path and puts the dial back.
        if hw < 0.0 {
            return Err(crate::RadioError::Msg(format!(
                "{:.6} MHz is below DC once the {:.6} MHz converter offset is applied",
                hz / 1e6,
                step.rx_offset_hz / 1e6
            )));
        }
        self.inner.set_center_hz(hw)?;
        // The band decoder wants the frequency on the *air*, not the I.F. an
        // accessory board would otherwise switch its filters for.
        self.inner.set_dial_hz(hz);
        self.step = step;
        Ok(())
    }

    /// The ceiling the selected transverter puts on transmit drive.
    fn tx_drive_ceiling(&self) -> Option<f32> {
        self.step.tx_drive
    }

    fn describe(&self) -> String {
        let boxes = self.plan.steps().len();
        if boxes > 1 {
            return format!("{} ({boxes} converters)", self.inner.describe());
        }
        format!("{} (+{:.6} MHz converter)", self.inner.describe(), self.step.rx_offset_hz / 1e6)
    }

    fn wide_spectrum_db(&mut self, out: &mut Vec<f32>) -> Option<(f64, f64)> {
        let (center, span) = self.inner.wide_spectrum_db(out)?;
        Some((self.down(center), span))
    }

    /// A converter shifts where the lane sits, never how wide it is.
    fn wide_span_hz(&self) -> f64 {
        self.inner.wide_span_hz()
    }

    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        self.inner
            .poll_control()
            .into_iter()
            .map(|u| match u {
                ControlUpdate::Freq(hz) => ControlUpdate::Freq(self.down(hz)),
                // A centre is a hardware frequency like any other the inner
                // source reports; the converter offset comes off it too.
                ControlUpdate::Center(hz) => ControlUpdate::Center(self.down(hz)),
                other => other,
            })
            .collect()
    }

    // --- forwarded verbatim -------------------------------------------------

    fn sample_rate(&self) -> f64 {
        self.inner.sample_rate()
    }

    /// Relative to the VFO, so the converter does not enter into it: the LO ends
    /// up at `(vfo ± lo_offset) + converter offset`, which is what both want.
    fn lo_offset_hz(&self) -> f64 {
        self.inner.lo_offset_hz()
    }

    /// A transverter in front of a rig does not change which knob decides where
    /// the spectrum sits — it is still the rig's.
    fn center_is_dial(&self) -> bool {
        self.inner.center_is_dial()
    }

    /// Nor where the rig puts its own I.F. in CW: that is a property of the
    /// radio, and a converter ahead of it moves both numbers together.
    fn cw_iq_on_vfo(&self) -> bool {
        self.inner.cw_iq_on_vfo()
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        self.inner.read(buf)
    }

    fn read_available(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        self.inner.read_available(buf)
    }

    fn set_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        self.inner.set_gain_element(name, db)
    }

    fn set_antenna(&mut self, name: &str) -> Result<()> {
        self.inner.set_antenna(name)
    }

    fn set_device_setting(&mut self, key: &str, value: &str) -> Result<()> {
        self.inner.set_device_setting(key, value)
    }

    fn current_gains(&self) -> Vec<(String, f64)> {
        self.inner.current_gains()
    }

    fn current_antenna(&self) -> String {
        self.inner.current_antenna()
    }
    fn learned_antennas(&self) -> Option<&'static [&'static str]> {
        self.inner.learned_antennas()
    }
    /// The front end's own stages. A converter shifts the frequency the *radio*
    /// shows, but the tuner behind it is still on the converted one, so its
    /// band-dependent ranges are the inner source's to report.
    fn learned_rx_gains(&self) -> Option<Vec<sdroxide_types::GainElement>> {
        self.inner.learned_rx_gains()
    }
    fn set_rig_power(&mut self, on: bool) -> Result<()> {
        self.inner.set_rig_power(on)
    }
    fn commands_rig_power(&self) -> bool {
        self.inner.commands_rig_power()
    }
    fn rx_antenna(&self) -> Option<bool> {
        self.inner.rx_antenna()
    }
    fn set_rx_antenna(&mut self, on: bool) -> Result<()> {
        self.inner.set_rx_antenna(on)
    }
    fn reread_rx_antenna(&mut self) {
        self.inner.reread_rx_antenna();
    }

    fn owns_rx_antenna(&self) -> bool {
        self.inner.owns_rx_antenna()
    }

    /// Translated by the *transmit* offset, which is a different number from the
    /// receive one — and refused outright when there is none, rather than
    /// falling back on either the receive offset or the operator's frequency.
    ///
    /// The engine's gate stops a key-down long before this, since [`shift_caps`]
    /// has withdrawn transmit; this is the backstop for every other way in (a
    /// CAT client, a digital mode, a future caller) and it must not be the one
    /// that guesses. Keying a Pluto behind a 9750 MHz LNB on the dial frequency
    /// would ask it for 10.489 GHz, which it would clamp to the top of its range
    /// and transmit there.
    fn tx_begin(&mut self, center_hz: f64, rate: f64) -> Result<f64> {
        match self.up_tx(center_hz) {
            Some(hw) => self.inner.tx_begin(hw, rate),
            None => Err(crate::RadioError::Msg(
                "transmit is off while a converter is set — say what is in the transmit line \
                 under Settings → Radio → Transmit"
                    .into(),
            )),
        }
    }

    fn tx_write(&mut self, samples: &[Complex32]) -> Result<()> {
        self.inner.tx_write(samples)
    }

    fn tx_end(&mut self) -> Result<()> {
        self.inner.tx_end()
    }

    /// Both of these are about the receiver behind the converter, which is the
    /// one doing the buffering — a converter shifts frequencies and holds no
    /// samples of its own. Forwarded rather than left to the trait defaults
    /// because the defaults are no-ops: an operator who set a transverter up
    /// would otherwise have every over replayed as stale receive, and their
    /// backend would report the whole transmission as an overrun.
    fn discard_pending_rx(&mut self) {
        self.inner.discard_pending_rx();
    }

    fn set_rx_paused(&mut self, paused: bool) {
        self.inner.set_rx_paused(paused);
    }

    fn set_tx_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        self.inner.set_tx_gain_element(name, db)
    }

    fn current_tx_gains(&self) -> Vec<(String, f64)> {
        self.inner.current_tx_gains()
    }

    fn set_tx_antenna(&mut self, name: &str) -> Result<()> {
        self.inner.set_tx_antenna(name)
    }

    fn current_tx_antenna(&self) -> String {
        self.inner.current_tx_antenna()
    }

    fn set_tx_drive(&mut self, frac: f64) {
        self.inner.set_tx_drive(frac);
    }

    fn set_tune_drive(&mut self, frac: f64) {
        self.inner.set_tune_drive(frac);
    }

    fn commands_tx_power(&self) -> bool {
        self.inner.commands_tx_power()
    }
    fn set_squelch(&mut self, frac: f32) {
        self.inner.set_squelch(frac);
    }
    fn commands_squelch(&self) -> bool {
        self.inner.commands_squelch()
    }

    fn tx_telemetry(&mut self) -> Option<sdroxide_types::TxTelemetry> {
        self.inner.tx_telemetry()
    }

    fn pa_temp_c(&mut self) -> Option<f32> {
        self.inner.pa_temp_c()
    }

    fn rx_signal_dbm(&mut self) -> Option<f32> {
        self.inner.rx_signal_dbm()
    }

    /// The front end's own gain, which is what it was: a converter ahead of it
    /// shifts the frequency and not the level. Its conversion gain or loss is
    /// the operator's to put in the calibration offset, being a property of
    /// their box rather than anything the radio can read.
    fn rx_gain_db(&self) -> Option<f32> {
        self.inner.rx_gain_db()
    }

    /// Relative (VFO minus IQ centre), so untouched.
    fn set_if_offset(&mut self, hz: f64) {
        self.inner.set_if_offset(hz);
    }

    /// A property of the pipeline behind the converter, which a frequency
    /// translation does not change.
    fn stream_delay_s(&self) -> f64 {
        self.inner.stream_delay_s()
    }

    /// A transmit frequency, so it takes the transmit offset: what the hardware
    /// behind the converter actually emits is what its accessory boards switch
    /// bands for. With transmit withdrawn there is no such frequency, and a
    /// board is better left where it is than switched to a band nothing will be
    /// keyed on.
    fn set_tx_freq_hz(&mut self, hz: f64) {
        if let Some(hw) = self.up_tx(hz) {
            self.inner.set_tx_freq_hz(hw);
        }
    }

    fn display_bandwidth(&self) -> Option<f64> {
        self.inner.display_bandwidth()
    }

    fn set_control_mode(&mut self, mode: Mode) -> Result<()> {
        self.inner.set_control_mode(mode)
    }

    /// Audio-band edges either side of the dial, which the converter does not
    /// move.
    fn set_control_filter(&mut self, mode: Mode, lo_hz: f64, hi_hz: f64) {
        self.inner.set_control_filter(mode, lo_hz, hi_hz);
    }

    /// Relative (an offset from the dial), so untouched.
    fn set_rit_hz(&mut self, hz: f64) {
        self.inner.set_rit_hz(hz);
    }

    /// Audio, which no frequency conversion touches.
    fn rx_audio(&mut self, out: &mut Vec<f32>) -> Option<f64> {
        self.inner.rx_audio(out)
    }

    fn tracks_rx_mode(&self) -> bool {
        self.inner.tracks_rx_mode()
    }

    fn commands_rx_mode(&self) -> bool {
        self.inner.commands_rx_mode()
    }

    /// A question about the radio's control layer, which a frequency
    /// translation in front of it does not change.
    fn resolves_band_sideband(&self) -> bool {
        self.inner.resolves_band_sideband()
    }

    fn mutes_rx_audio_on_tx(&self) -> bool {
        self.inner.mutes_rx_audio_on_tx()
    }

    fn mutes_rx_audio_on_rig_tx(&self) -> bool {
        self.inner.mutes_rx_audio_on_rig_tx()
    }

    fn tx_write_audio(&mut self, audio: &[f32]) -> Result<()> {
        self.inner.tx_write_audio(audio)
    }

    /// Keying is text, not a frequency: the converter has nothing to add.
    fn cw_text_keying(&self) -> Option<usize> {
        self.inner.cw_text_keying()
    }
    fn cw_audio_keyed(&self) -> bool {
        self.inner.cw_audio_keyed()
    }
    fn send_cw(&mut self, text: &str) {
        self.inner.send_cw(text);
    }
    fn abort_cw(&mut self) {
        self.inner.abort_cw();
    }
    fn set_cw_wpm(&mut self, wpm: f32) {
        self.inner.set_cw_wpm(wpm);
    }

    fn tx_drain(&mut self) {
        self.inner.tx_drain();
    }

    fn tx_pace_cushion_ms(&self) -> f64 {
        self.inner.tx_pace_cushion_ms()
    }

    fn open_status(&self) -> Option<String> {
        self.inner.open_status()
    }

    fn needs_reopen(&self) -> bool {
        self.inner.needs_reopen()
    }

    fn release(&mut self) {
        self.inner.release();
    }
}

/// Paces reads so a non-hardware source delivers samples in real time.
struct Throttle {
    start: Instant,
    emitted: u64,
    rate: f64,
}

impl Throttle {
    fn new(rate: f64) -> Self {
        Throttle { start: Instant::now(), emitted: 0, rate }
    }

    fn pace(&mut self, n: usize) {
        self.emitted += n as u64;
        let due = self.start + Duration::from_secs_f64(self.emitted as f64 / self.rate);
        let now = Instant::now();
        if due > now {
            std::thread::sleep(due - now);
        }
    }
}

/// Multi-tone signal generator with a noise floor. Real-time paced.
pub struct SigGenSource {
    sample_rate: f64,
    center_hz: f64,
    /// (offset from center in Hz, linear amplitude)
    tones: Vec<(f64, f32)>,
    /// Each tone as a rotating phasor and the per-sample rotation that advances
    /// it, rather than a phase angle to take a sine and a cosine of.
    ///
    /// `(phasor, step)`: one complex multiply a sample instead of a `sin`, a
    /// `cos` and a `fmod`. That arithmetic was measured at about a fifth of the
    /// whole process on a five-tone 1.5 Msps generator — 7.7 million of each
    /// libm call a second — which is a strange thing for a *test* source to
    /// spend, and it made every profile of the real receive chain read high.
    rotors: Vec<(Complex32, Complex32)>,
    /// Samples until the phasors are renormalised. Repeated complex multiplies
    /// drift off the unit circle, slowly (the error is second-order per step)
    /// but without bound, so the amplitude has to be pulled back now and then.
    renorm_in: usize,
    noise_amp: f32,
    rng: u64,
    throttle: Throttle,
}

/// How often the tone phasors are pulled back onto the unit circle.
///
/// A rotation by a complex multiply loses about `eps` of magnitude per step, so
/// a few thousand steps between corrections keeps every tone inside a hundredth
/// of a dB of its nominal amplitude while costing one square root per tone per
/// block rather than per sample.
const SIGGEN_RENORM_SAMPLES: usize = 4096;

impl SigGenSource {
    pub fn new(sample_rate: f64, center_hz: f64, tones: Vec<(f64, f32)>, noise_amp: f32) -> Self {
        let rotors = Self::rotors(&tones, sample_rate);
        SigGenSource {
            sample_rate,
            center_hz,
            tones,
            rotors,
            renorm_in: SIGGEN_RENORM_SAMPLES,
            noise_amp,
            rng: 0x9e3779b97f4a7c15,
            throttle: Throttle::new(sample_rate),
        }
    }

    /// Each tone's starting phasor (at its amplitude) and per-sample rotation.
    ///
    /// The two `sin`/`cos` pairs a tone needs are taken once here, not once per
    /// sample: `e^{i(φ+δ)} = e^{iφ}·e^{iδ}`, so advancing a tone is a complex
    /// multiply by a constant.
    fn rotors(tones: &[(f64, f32)], sample_rate: f64) -> Vec<(Complex32, Complex32)> {
        use std::f64::consts::TAU;
        tones
            .iter()
            .map(|&(offset, amp)| {
                let d = TAU * offset / sample_rate;
                (Complex32::new(amp, 0.0), Complex32::new(d.cos() as f32, d.sin() as f32))
            })
            .collect()
    }

    /// A default test scene: carriers at various offsets over a noise floor.
    /// One tone sits 700 Hz above center so the default USB tune is audible
    /// immediately.
    pub fn demo(sample_rate: f64, center_hz: f64) -> Self {
        Self::new(
            sample_rate,
            center_hz,
            vec![
                (-sample_rate * 0.30, 0.02),
                (-sample_rate * 0.11, 0.10),
                (700.0, 0.05),
                (sample_rate * 0.07, 0.30),
                (sample_rate * 0.23, 0.05),
            ],
            0.001,
        )
    }

    fn white(&mut self) -> f32 {
        // xorshift64* — cheap, deterministic, good enough for a noise floor.
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        let v = (self.rng.wrapping_mul(0x2545F4914F6CDD1D) >> 40) as i32;
        v as f32 / (1 << 23) as f32 - 1.0
    }
}

impl IqSource for SigGenSource {
    fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    fn center_hz(&self) -> f64 {
        self.center_hz
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center_hz = hz;
        Ok(())
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        for s in buf.iter_mut() {
            let mut acc =
                Complex32::new(self.noise_amp * self.white(), self.noise_amp * self.white());
            for (phasor, step) in self.rotors.iter_mut() {
                acc += *phasor;
                *phasor *= *step;
            }
            *s = acc;
        }
        // Pull the phasors back onto their circle now and then — see
        // [`SIGGEN_RENORM_SAMPLES`]. Per block rather than per sample, and only
        // when one is due, so the cost is a rounding error on the loop above.
        self.renorm_in = self.renorm_in.saturating_sub(buf.len());
        if self.renorm_in == 0 {
            self.renorm_in = SIGGEN_RENORM_SAMPLES;
            for ((phasor, _), &(_, amp)) in self.rotors.iter_mut().zip(&self.tones) {
                let mag = phasor.norm();
                if mag > 1e-6 {
                    *phasor *= amp / mag;
                }
            }
        }
        self.throttle.pace(buf.len());
        Ok(buf.len())
    }

    fn describe(&self) -> String {
        format!("signal generator ({} tones, {:.3} Msps)", self.tones.len(), self.sample_rate / 1e6)
    }
}

/// Raw interleaved CF32 (little-endian f32 I,Q) file playback, looped,
/// real-time paced.
pub struct FileSource {
    reader: BufReader<File>,
    path: String,
    sample_rate: f64,
    center_hz: f64,
    /// Where the samples begin: past a WAV header, or zero for a raw stream.
    /// The loop at the end of the file goes back to *this*, not to the start.
    data_start: u64,
    throttle: Throttle,
}

impl FileSource {
    /// Play a raw interleaved CF32 stream — or one of sdroxide's own I/Q WAV
    /// captures, whose header says what the caller would otherwise have to
    /// type: a capture made by the REC popup plays back at the rate and on the
    /// frequency it was made, with `--file` and nothing else (issue #217).
    ///
    /// A rate or a centre given on the command line still wins; that is what
    /// `--rate` and `--freq` are for, and a header is not an instruction.
    pub fn open(path: impl AsRef<Path>, sample_rate: f64, center_hz: f64) -> Result<Self> {
        Self::open_with(path, sample_rate, center_hz, false, false)
    }

    /// [`FileSource::open`], told which of the two the operator named
    /// themselves so the header does not overrule them.
    pub fn open_with(
        path: impl AsRef<Path>,
        sample_rate: f64,
        center_hz: f64,
        rate_given: bool,
        center_given: bool,
    ) -> Result<Self> {
        let path_str = path.as_ref().display().to_string();
        let wav = crate::iq_wav::probe(path.as_ref());
        let sample_rate = match &wav {
            Some(w) if !rate_given => w.rate_hz,
            _ => sample_rate,
        };
        let center_hz = match &wav {
            Some(w) if !center_given => w.center_hz.unwrap_or(center_hz),
            _ => center_hz,
        };
        let mut reader = BufReader::new(File::open(path)?);
        let start = wav.as_ref().map_or(0, |w| w.data_start);
        if start > 0 {
            reader.seek(SeekFrom::Start(start))?;
        }
        Ok(FileSource {
            reader,
            path: path_str,
            sample_rate,
            center_hz,
            data_start: start,
            throttle: Throttle::new(sample_rate),
        })
    }
}

impl IqSource for FileSource {
    fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    fn center_hz(&self) -> f64 {
        self.center_hz
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center_hz = hz;
        Ok(())
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let mut raw = vec![0u8; buf.len() * 8];
        let mut filled = 0;
        while filled < raw.len() {
            let n = self.reader.read(&mut raw[filled..])?;
            if n == 0 {
                // Loop — back to the first *sample*, which on a WAV is past the
                // header. Restarting at zero would play the header as signal.
                self.reader.seek(SeekFrom::Start(self.data_start))?;
                continue;
            }
            filled += n;
        }
        for (s, chunk) in buf.iter_mut().zip(raw.chunks_exact(8)) {
            let i = f32::from_le_bytes(chunk[0..4].try_into().unwrap());
            let q = f32::from_le_bytes(chunk[4..8].try_into().unwrap());
            *s = Complex32::new(i, q);
        }
        self.throttle.pace(buf.len());
        Ok(buf.len())
    }

    fn describe(&self) -> String {
        format!("IQ file {} ({:.3} Msps)", self.path, self.sample_rate / 1e6)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The zero-IF LO-offset policy, shared by every front end that has a DC
    /// spike where the operator's VFO would otherwise sit — the SoapySDR
    /// devices it was written for, and the native PlutoSDR backend.
    #[test]
    fn lo_offset_wants_span_and_an_analog_filter_wide_enough_to_reach_it() {
        // A HackRF at the rate it settles on: the 1.75 MHz baseband filter
        // passes a 500 kHz offset with room to spare.
        assert_eq!(lo_offset_for(2_000_000.0, 1_750_000.0), 500_000.0);
        // A front end whose filter is narrower than the offset would only
        // attenuate what we moved out there — DC blocker alone.
        assert_eq!(lo_offset_for(2_000_000.0, 200_000.0), 0.0);
        // Too narrow a stream to spend a quarter of on an offset.
        assert_eq!(lo_offset_for(768_000.0, 768_000.0), 0.0);
        // A driver that reports no filter bandwidth: go by the rate.
        assert_eq!(lo_offset_for(8_000_000.0, 0.0), 2_000_000.0);
    }

    /// The Pluto backend sets the AD9361's analog filter itself, to 0.9 of the
    /// sample rate. That number is chosen against this function: any narrower
    /// and the offset it is trying to enable would be silently discarded.
    #[test]
    fn an_analog_filter_at_nine_tenths_of_the_rate_keeps_the_offset() {
        for rate in [1_000_000.0, 2_000_000.0, 3_840_000.0, 5_000_000.0f64] {
            let offset = lo_offset_for(rate, rate * 0.9);
            assert_eq!(offset, rate * 0.25, "the offset was dropped at {rate} sps");
        }
    }
}

#[cfg(test)]
mod siggen_tests {
    use super::*;

    /// The generator's tones stay where they are put, at the amplitude they
    /// were given, for as long as anyone runs it.
    ///
    /// It advances each tone by multiplying a phasor rather than by taking a
    /// sine of a growing angle, which is an order of magnitude cheaper and
    /// would be worthless if the amplitude wandered: repeated complex
    /// multiplies drift off the unit circle without bound. This runs far past
    /// the renormalisation interval and measures what actually comes out.
    #[test]
    fn the_tones_hold_their_amplitude_and_frequency() {
        let rate = 1_536_000.0;
        let mut siggen = SigGenSource::new(rate, 14_200_000.0, vec![(100_000.0, 0.30)], 0.0);

        // Long enough to cross the renormalisation boundary many times over.
        let mut buf = vec![Complex32::default(); 16_384];
        for _ in 0..40 {
            siggen.read(&mut buf).unwrap();
        }

        // Amplitude: a single tone with no noise, so every sample is the
        // phasor itself.
        let peak = buf.iter().map(|c| c.norm()).fold(0.0f32, f32::max);
        let floor = buf.iter().map(|c| c.norm()).fold(f32::MAX, f32::min);
        assert!(
            (peak - 0.30).abs() < 0.003 && (floor - 0.30).abs() < 0.003,
            "amplitude drifted to {floor}..{peak}, expected 0.30"
        );

        // Frequency: count the phase advance per sample and turn it back into
        // hertz. A step that had been mis-derived would show up here.
        let mut adv = 0.0f64;
        for w in buf.windows(2).take(1000) {
            let d = (w[1] * w[0].conj()).arg() as f64;
            adv += d;
        }
        let hz = adv / 1000.0 / std::f64::consts::TAU * rate;
        assert!((hz - 100_000.0).abs() < 50.0, "tone read {hz:.0} Hz, expected 100000");
    }

    /// Several tones at once still sum to what was asked for, which is what
    /// every test that uses `demo()` as a stand-in for a radio depends on.
    #[test]
    fn every_tone_of_a_scene_is_present() {
        let rate = 1_536_000.0;
        let want = vec![(-200_000.0, 0.10), (700.0, 0.05), (300_000.0, 0.30)];
        let mut siggen = SigGenSource::new(rate, 14_200_000.0, want.clone(), 0.0);
        let mut buf = vec![Complex32::default(); 8192];
        for _ in 0..8 {
            siggen.read(&mut buf).unwrap();
        }

        // Correlate against each tone's own rotation: a tone that is present at
        // amplitude a leaves |mean| = a, and the others average away.
        for &(offset, amp) in &want {
            let step = std::f64::consts::TAU * offset / rate;
            let mut acc = Complex32::default();
            for (i, s) in buf.iter().enumerate() {
                let ph = -step * i as f64;
                acc += *s * Complex32::new(ph.cos() as f32, ph.sin() as f32);
            }
            let got = acc.norm() / buf.len() as f32;
            assert!(
                (got - amp).abs() < amp * 0.05,
                "tone at {offset} Hz read {got}, expected {amp}"
            );
        }
    }
}
