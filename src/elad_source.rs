//! An [`IqSource`] for an ELAD FDM-DUO, FDM-S2 or FDM-S1: wideband I/Q over
//! the native USB driver in `sdroxide-elad`, rig control over the transceiver's
//! CAT port, and transmit audio out through its USB sound card.
//!
//! # One radio, three USB devices
//!
//! An FDM-DUO enumerates three times — ELAD's own vendor interface for the
//! DDC's I/Q, an FTDI bridge for CAT, and a C-Media sound card — and this file
//! is where the three become one interface. That shape is why the pieces here
//! are borrowed rather than rebuilt: `sdroxide_cat` already drives the serial
//! link (with [`CatFamily::Elad`], the same profile `Backend::Cat` uses) and
//! `sdroxide_audio` already carries transmit audio, so what is left is the
//! joining.
//!
//! An FDM-S1 or FDM-S2 has only the first of the three and comes up
//! receive-only, with no control path at all.
//!
//! # The centre is the dial, because on this radio they are one knob
//!
//! This front end hands over a whole DDC window — 192 kHz at the least — but
//! what that window is centred *on* is the transceiver's own VFO. The DDC is
//! not a second receiver alongside the one the radio tunes for itself: move the
//! VFO and the window moves with it, hertz for hertz. There is therefore no
//! arrangement in which the radio's displayed frequency and sdroxide's dial can
//! be different numbers and both be true, so [`IqSource::center_is_dial`] says
//! so and the engine commands the VFO for every tune, exactly as it does for a
//! transceiver sending I/Q down a sound card ([`crate::audio_cat_source`]).
//!
//! CW is the one exception, and it is not really one. The window still rides the
//! VFO — that is the point — but sdroxide's CW dial is a zero-beat and the tone
//! being copied sits a sidetone pitch above it, while the DUO keys its own
//! transmitter on the VFO. So there the VFO goes on the station and the dial
//! stays a pitch under it: [`IqSource::cw_iq_on_vfo`] is this radio saying which
//! of the two CW families it belongs to, and the engine's `rig_cw_offset_hz` is
//! what it costs. Leaving the two equal answered every station a sidetone low
//! ([issue #170]).
//!
//! It used to be the other way round — the VFO was parked on the panadapter
//! centre and the dial moved inside the window in software — and every way that
//! could go wrong, did. The rig's display never agreed with sdroxide's readout;
//! the radio's own audio was demodulating the centre rather than the station
//! being listened to; an over keyed at the *radio* (mic, key, VOX) went out on
//! the centre, which is up to half a window from where the operator could see
//! they were tuned; and a tuning step smaller than half the window commanded
//! nothing at all, so the dial and the mouse wheel simply did not move the
//! radio ([issue #146]).
//!
//! What must **not** come back with that is [issue #111]: the receive dial used
//! to be pushed at the VFO through [`IqSource::set_tx_freq_hz`], which slid the
//! window underneath a panadapter that believed it had not moved, so the station
//! clicked on ran away across the screen instead of being tuned. That hook stays
//! a no-op. The window moves only through [`IqSource::set_center_hz`], which is
//! the one path the engine also relabels the frequency axis from.
//!
//! An FDM-S1 or FDM-S2 has no VFO to command and is an ordinary SDR: its centre
//! is its LO, and the engine tunes inside the span. So is an FDM-DUO whose
//! control port never answered — commanding a dial nothing is listening to is
//! how the panadapter comes to look locked (issue #155), so the claim is made
//! only once the radio has been heard from.
//!
//! [issue #111]: https://github.com/dividebysandwich/sdroxide/issues/111
//! [issue #146]: https://github.com/dividebysandwich/sdroxide/issues/146
//! [issue #170]: https://github.com/dividebysandwich/sdroxide/issues/170
//!
//! # Not verified against hardware
//!
//! Almost nothing here has been run against a radio. One of the two assumptions
//! this file was written on has now been settled the hard way, by an operator
//! with an FDM-DUO: the DDC feeding this USB interface is emphatically *not*
//! independent of the receiver the transceiver tunes for its own audio. The
//! other still wants checking — whether the stream survives a transmit cycle.
//! It is assumed *not* to (the interface is declared half duplex), which is the
//! safe way to be wrong.

use std::time::Duration;

use sdroxide_elad::{EladHandle, Model};
use sdroxide_radio::{Complex32, ControlUpdate, IqSource, Result};
use sdroxide_types::{CatConfig, CatFamily, EladAntenna, EladConfig, Mode, TxTelemetry};

/// How long the device may deliver nothing before the connection counts as
/// dead. Same three seconds as the other native USB backends: this is a local
/// device, so there is no network to be briefly slow.
const SILENCE_BEFORE_REOPEN: Duration = Duration::from_secs(3);

/// How long a frequency we have commanded stands as an expectation before a
/// disagreeing report is believed instead.
///
/// A radio does not move the instant it is told to, and the answer to a poll
/// sent before our command can arrive after it — so for a moment after every
/// re-centre the rig is honestly reporting a frequency we are in the middle of
/// moving it away from. Adopting that as the window centre is how opening the
/// interface with the radio parked on another band used to drag the dial to the
/// edge of a window it was never in. Bounded rather than held until the
/// expectation is met, because a command the rig quietly refused would
/// otherwise leave every turn of the operator's knob ignored for ever.
const FREQ_SETTLE: Duration = Duration::from_millis(1200);

/// How long a reading from the rig's S-meter stands in for the next one — the
/// same window `AudioCatSource` uses, and for the same reason: a gap between
/// answers is not a signal that went away, but a link that has gone quiet must
/// not leave a needle standing. Follows the configured poll rate, which is what
/// decides how far apart two honest answers are.

/// How the transceiver is reached.
///
/// Three cases and they are genuinely different, which is why this is an enum
/// rather than an `Option`. The serial port is the full link — it reads as well
/// as writes, so the meters, the mode and the operator's own knob all come
/// back. The USB gateway writes only. Nothing at all is an FDM-S.
enum Control {
    /// The rig's CAT serial port.
    Serial(Box<sdroxide_cat::CatHandle>),
    /// The FDM-DUO's CAT gateway on the streaming USB interface.
    ///
    /// A write-only path, and the reason it exists is worth stating: it works
    /// with **no serial cable plugged in**. A DUO on one USB lead can still be
    /// tuned, put in a mode and keyed. What is given up is everything that
    /// needs an answer — the S-meter, the SWR, the transmit power, and any
    /// notice that the operator has touched the front panel.
    Gateway,
    /// An FDM-S1 or FDM-S2: a receiver, with nothing to control.
    None,
}

pub struct EladSource {
    handle: EladHandle,
    control: Control,
    center: f64,
    rx_scratch: Vec<f32>,
    label: String,
    model: Model,

    // TX audio to the rig (interleaved stereo playback ring), exactly as
    // `AudioCatSource` drives it.
    out: Option<(sdroxide_audio::AudioOutput, sdroxide_radio::rtrb::Producer<f32>)>,
    tx_resampler: Option<sdroxide_dsp::MonoResampler>,
    tx_scratch: Vec<f32>,

    /// Mirrors of the front-end switches, so `current_gains` can answer without
    /// a round trip to the stream thread.
    attenuator: bool,
    preselector: bool,

    /// Which of the two antenna sockets the receiver is on.
    ///
    /// A mirror, like the two switches above, but unlike them it starts as a
    /// *guess*: the setting lives in the radio and survives a power cycle, so
    /// what it is when sdroxide arrives is the rig's business. Over the serial
    /// link the guess is corrected within a poll — the port asks `AN;` as it
    /// opens and the answer is adopted. Over the USB gateway nothing can be
    /// asked, so it stays the radio's own default (one antenna, on RTX) until
    /// the operator picks a socket, and picking one is what makes the two
    /// agree.
    antenna: EladAntenna,

    /// The frequency last commanded to the rig, held until the rig reports it
    /// back.
    ///
    /// Without this, our own commands feed back on themselves: we move the
    /// VFO — to re-centre the window, or onto the transmit frequency for an
    /// over — the rig dutifully reports having moved there, and this file,
    /// which cannot tell that report from the operator turning the knob, reads
    /// it as the window having been panned by hand. Suppressing our own
    /// commands coming back — and, for [`FREQ_SETTLE`], whatever the rig says
    /// on its way there — leaves genuine front-panel movements getting through,
    /// which is the whole point of reading the rig at all.
    expect_freq: Option<(f64, std::time::Instant)>,
    /// Whether the transceiver's VFO is something this end can move at all,
    /// and so whether the window centre is the dial ([`IqSource::center_is_dial`]).
    ///
    /// True the moment there is any path to the VFO: the USB gateway always has
    /// one (it is the interface the samples are already arriving on), and a
    /// serial port has one as soon as the radio has answered it. Seeded from the
    /// startup query and latched on by the first thing the rig says afterwards,
    /// so a DUO switched on after sdroxide — or one whose CAT baud the operator
    /// has just corrected — is picked up rather than left out for the session.
    /// Never cleared: a rig that has missed a poll or two is still a rig whose
    /// dial we own, and flipping the panadapter's whole tuning behaviour on a
    /// quiet moment would be worse than either answer.
    ///
    /// False for an FDM-S, which has no VFO, and for a DUO on a serial port
    /// nothing has ever answered — see the module header.
    dial_reachable: bool,
    /// Whether sdroxide is holding the rig keyed.
    ///
    /// For the length of an over the VFO is the *transmit* frequency rather
    /// than the receive window's centre, so what the rig reports about its
    /// frequency means something else and is ignored until [`Self::tx_end`] has
    /// put the centre back.
    keyed: bool,
    /// Latest SWR the rig reported while keyed, and the latest S-meter reading
    /// with the time it arrived. Held for the same reason `AudioCatSource`
    /// holds them: the engine's meter tick is far faster than the rig answers.
    last_telem: Option<TxTelemetry>,
    last_signal: Option<(std::time::Instant, f32)>,
    /// How long that S-meter reading stands in for the next one — derived from
    /// the configured poll rate, which is what decides how far apart two of the
    /// rig's answers are.
    signal_max_age: Duration,
    /// Warnings from the open, plus the one the stream thread can only raise
    /// once samples have been flowing.
    status: Vec<String>,
    /// The stream thread's own warnings, kept once they have been taken.
    ///
    /// Sticky because [`IqSource::open_status`] is asked in two places and only
    /// one of them shows the answer: the engine also asks it to decide whether a
    /// source that wants reopening already has a reason on screen. Taking the
    /// stream thread's sentence there and dropping it is how the one message
    /// that explains a receiver delivering nothing — see `sdroxide_elad::fpga`
    /// and issue #178 — was thrown away on every cycle of the reopen loop.
    late: std::sync::Mutex<Vec<String>>,
}

impl EladSource {
    /// Open the device, and — on an FDM-DUO — its control link and transmit
    /// audio.
    ///
    /// `cat` is the same `RadioConfig::cat` block every other CAT rig uses; an
    /// empty serial path is how an operator says there is no control cable, and
    /// on a DUO that falls through to the USB gateway rather than to nothing.
    pub fn open(
        cfg: &EladConfig,
        cat: &CatConfig,
        audio_out: Option<&str>,
        center_hz: f64,
    ) -> anyhow::Result<Self> {
        let handle = EladHandle::open(cfg, center_hz)?;
        let model = handle.model;
        let label = handle.label.clone();
        let mut status = handle.warnings.clone();

        // Whether the transceiver's VFO can be moved from here at all, decided
        // alongside the control path because it is the same question: see
        // `IqSource::center_is_dial`.
        let mut dial_reachable = false;
        let control = if model != Model::Duo {
            Control::None
        } else if cat.serial.path.trim().is_empty() {
            status.push(
                "no CAT serial port is set, so the FDM-DUO is being driven through its \
                 USB interface — it can be tuned and keyed, but its S-meter, SWR and \
                 front-panel knob cannot be read. Set the port under Settings → Radio."
                    .to_string(),
            );
            // Write-only, but it is the interface the samples are arriving on,
            // so there is no question of whether it is there.
            dial_reachable = true;
            Control::Gateway
        } else {
            // The family is forced rather than read: this interface *is* an
            // ELAD, and a config carried over from another radio could easily
            // still say Kenwood.
            let mut cat = cat.clone();
            cat.family = CatFamily::Elad;
            // Forcing the family is also what brings the line discipline into
            // line: `sdroxide_cat::spawn` holds an ELAD to the four baud rates
            // and the 8N1 frame its CAT port actually has. Worth saying on
            // screen as well as in the log, because the value that fails this
            // way is the *default* — `RadioConfig::cat` is shared with the CAT
            // interface and its own default baud is 19200, which no FDM-DUO
            // has, so an owner who has never touched Baud starts from a link
            // the radio cannot hear a word of. It is silent in both directions,
            // which is what [issue #146] looked like from the operator's chair:
            // a DUO that received perfectly and would not transmit, on every
            // serial port they tried.
            //
            // [issue #146]: https://github.com/dividebysandwich/sdroxide/issues/146
            let asked = cat.serial.baud;
            let using = sdroxide_types::elad_cat_baud(asked);
            if using != asked {
                status.push(format!(
                    "the FDM-DUO has no {asked} baud CAT setting, so its control port is \
                     being opened at {using} instead — at any rate the radio does not have, \
                     it ignores the dial and refuses to key. Set Baud under Settings → Radio \
                     to whatever menu 70 \"CAT BAUD\" says on the radio.",
                ));
            }
            // Ask the radio one question before commanding it anything, for
            // the same reason `AudioCatSource` does: whether it answers at all
            // is the only evidence there is that there *is* a control link, and
            // the whole shape of tuning hangs on it. A DUO that has not answered
            // by the time the first sample arrives can still say so later —
            // `poll_control` latches this on at the first update.
            dial_reachable = sdroxide_cat::query_once(&cat).is_some();
            if !dial_reachable {
                status.push(format!(
                    "No answer from the FDM-DUO on {} — its VFO cannot be commanded from \
                     here, so tuning stays inside the window the radio is already sending \
                     and its display will not follow. Check the port, and that Baud matches \
                     menu 70 \"CAT BAUD\" on the radio.",
                    sdroxide_cat::link_label(&cat),
                ));
            }
            Control::Serial(Box::new(sdroxide_cat::spawn(cat)))
        };
        let signal_max_age = sdroxide_cat::signal_max_age(cat);

        // TX audio is best-effort and only on the transceiver: a missing device
        // means no transmit audio, not a failed open.
        let out = if model.transmits() {
            if audio_out.is_none() {
                status.push(
                    "no sound card is set for transmit audio — the FDM-DUO's own USB \
                     Audio port should be chosen under Settings → General → Radio audio, \
                     or transmit will go out through whatever the system default is"
                        .to_string(),
                );
            }
            match sdroxide_audio::start_output(audio_out, 48_000) {
                Ok((o, p)) => Some((o, p)),
                Err(e) => {
                    tracing::warn!("ELAD TX audio device unavailable ({e}); receive only");
                    status.push(format!("transmit audio device unavailable ({e})"));
                    None
                }
            }
        } else {
            None
        };
        // `MonoResampler::new` returns None when the rates match.
        let tx_resampler = out
            .as_ref()
            .and_then(|(o, _)| sdroxide_dsp::MonoResampler::new(48_000.0, o.sample_rate));

        tracing::info!("ELAD source ready: {label}, center {center_hz:.0} Hz");
        let mut src = EladSource {
            handle,
            control,
            center: center_hz,
            rx_scratch: Vec::new(),
            label,
            model,
            out,
            tx_resampler,
            tx_scratch: Vec::new(),
            attenuator: cfg.attenuator,
            preselector: cfg.preselector,
            antenna: EladAntenna::default(),
            expect_freq: None,
            dial_reachable,
            keyed: false,
            last_telem: None,
            last_signal: None,
            signal_max_age,
            status,
            late: std::sync::Mutex::new(Vec::new()),
        };
        // Put the transceiver's VFO on the window centre before the first
        // sample is looked at. On a DUO that is where the window *is*, so the
        // two agreeing is what makes the frequency axis true; the device was
        // told the same number as it opened, and setting both leaves nothing
        // resting on which of the two the radio's firmware actually obeys.
        // Nothing to do on an FDM-S, which has no VFO and no way to be told.
        src.command_freq(center_hz);
        Ok(src)
    }

    pub fn model(&self) -> Model {
        self.model
    }

    /// Whether the rig can be keyed at all — which is the question the caps'
    /// transmit channel count is really asking.
    pub fn can_transmit(&self) -> bool {
        self.model.transmits() && !matches!(self.control, Control::None)
    }

    /// Whether the control link can answer questions as well as ask them.
    pub fn reads_rig(&self) -> bool {
        matches!(self.control, Control::Serial(_))
    }

    /// Put the rig's own VFO on `hz`, by whichever path is available.
    fn command_freq(&mut self, hz: f64) {
        match &self.control {
            Control::Serial(cat) => cat.set_freq(hz),
            Control::Gateway => self.handle.send_cat(sdroxide_cat::elad::freq_frame(hz)),
            Control::None => return,
        }
        self.expect_freq = Some((hz, std::time::Instant::now()));
    }

    fn command_ptt(&self, on: bool) {
        match &self.control {
            Control::Serial(cat) => cat.set_ptt(on),
            Control::Gateway => self.handle.send_cat(sdroxide_cat::elad::ptt_frame(on)),
            Control::None => {}
        }
    }

    /// Whether this rig has two antenna sockets to choose between — the
    /// transceiver, on either of its two control paths. An FDM-S has one input
    /// and nothing to send a command down.
    pub fn switches_antenna(&self) -> bool {
        self.model == Model::Duo && !matches!(self.control, Control::None)
    }
}

impl IqSource for EladSource {
    /// The engine is transmitting and has stopped reading, or has started
    /// again. Passed straight through to the stream thread, which keeps
    /// receiving either way — this only decides whether a full ring is
    /// reported as an overrun or as the ordinary cost of an over. See
    /// [`IqSource::set_rx_paused`].
    /// The DDC keeps streaming for the whole over — `tx_start` only asserts a
    /// PTT line over CAT, and the receiver on the other USB interface knows
    /// nothing about it — so without this the engine's first read after an
    /// unkey replays a whole transmission's worth of stale I/Q as if it had
    /// just arrived.
    fn discard_pending_rx(&mut self) {
        self.handle.discard_pending_rx();
    }

    fn set_rx_paused(&mut self, paused: bool) {
        self.handle.set_rx_paused(paused);
    }

    fn sample_rate(&self) -> f64 {
        self.handle.sample_rate_hz
    }

    fn center_hz(&self) -> f64 {
        self.center
    }

    /// Move the window, which on a transceiver means moving its VFO — and on
    /// this one, since the VFO carries the window, moving the dial with it.
    ///
    /// Both are commanded to the same frequency: the DDC tuning word through
    /// the streaming interface, and the rig's VFO through whichever control
    /// path there is. On an FDM-S only the first exists. On a DUO the VFO is
    /// the one that decides — see the module header — and sending the tuning
    /// word as well costs one control transfer and keeps this working whichever
    /// way round it turns out to be.
    ///
    /// Not the VFO while keyed, though: it is carrying the transmit frequency
    /// for the length of the over, and [`Self::tx_end`] is what puts it back.
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        self.handle.set_center_hz(hz);
        if !self.keyed {
            self.command_freq(hz);
        }
        Ok(())
    }

    /// On an FDM-DUO the window centre *is* the dial: the DDC rides on the
    /// transceiver's VFO, so the two cannot be different numbers, and the engine
    /// commands the VFO for every tune rather than sliding a software dial
    /// around inside the span. See the module header.
    ///
    /// False where there is no VFO to command — an FDM-S, or a DUO whose control
    /// port has never answered. Claiming a dial nothing hears is how a
    /// panadapter comes to look locked (issue #155): every gesture relabels the
    /// span around spectrum the radio is not sending, and the station clicked on
    /// walks out of the receiver instead of into it.
    fn center_is_dial(&self) -> bool {
        self.dial_reachable
    }

    /// The DDC on this USB interface is the receiver's, not the demodulator's:
    /// it is handed out centred on the VFO whatever mode the rig is in, and the
    /// sidetone offset lives in whatever demodulates it — the DUO's own audio
    /// stage, or ours.
    ///
    /// Which means that in CW the station being copied is a pitch *above* the
    /// VFO, and the VFO is what the rig keys its own transmitter on. Issue #170
    /// is what that costs when the two are left equal: a signal copied
    /// perfectly at 700 Hz, answered 700 Hz below, and nobody coming back.
    fn cw_iq_on_vfo(&self) -> bool {
        true
    }

    /// The transceiver in front of us owns its mode, and a mode chosen here has
    /// to reach it.
    ///
    /// Without this the engine's "command the rig's mode" gate never fired — it
    /// is demod audio, a tracked mode, or this — so the mode travelled rig→app
    /// on the `MD;` poll and never the other way. Change mode on the radio and
    /// sdroxide followed perfectly; change it in sdroxide and the radio sat
    /// there on the old one until the next key-down, where `tx_begin` asserts it
    /// anyway — which is the wrong moment to find out the rig was still on the
    /// other sideband ([issue #146]). The same fault, in the same shape, as the
    /// one `AudioCatSource::commands_rx_mode` was added for.
    ///
    /// [`IqSource::commands_rx_mode`] rather than [`IqSource::tracks_rx_mode`]:
    /// nothing is imposed when the interface opens. The rig's own `MD;` reply is
    /// adopted instead, because rearranging somebody's radio out of a restored
    /// session is not what connecting to it should do.
    ///
    /// The receive *filter* does not travel with it. The engine guards that
    /// separately on whether the rig is the one doing the receiving, and here it
    /// is not — sdroxide demodulates the DDC window itself, and the width the
    /// operator sets belongs to its own demodulator, not to the radio's IF.
    ///
    /// [issue #146]: https://github.com/dividebysandwich/sdroxide/issues/146
    fn commands_rx_mode(&self) -> bool {
        !matches!(self.control, Control::None)
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let need = buf.len() * 2;
        if self.rx_scratch.len() < need {
            self.rx_scratch.resize(need, 0.0);
        }
        let n = self.handle.rx_read(&mut self.rx_scratch[..need]);
        let pairs = n / 2;
        if pairs == 0 {
            // Nothing yet — brief nap so the DSP loop doesn't spin hot.
            std::thread::sleep(Duration::from_millis(2));
            return Ok(0);
        }
        for (p, out) in buf.iter_mut().enumerate().take(pairs) {
            *out = Complex32::new(self.rx_scratch[2 * p], self.rx_scratch[2 * p + 1]);
        }
        Ok(pairs)
    }

    fn describe(&self) -> String {
        self.label.clone()
    }

    /// The attenuator, plus a pseudo-element carrying the pre-selection filter
    /// switch.
    ///
    /// Routing a switch through `SetGain` rather than adding a `Command`
    /// variant keeps `Command`, `DeviceCaps` and the engine untouched for a
    /// setting only this backend has — the same trick the HackRF and Airspy HF+
    /// backends use, with the names living on [`EladConfig`] so the two ends
    /// cannot drift apart.
    fn set_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        match name {
            EladConfig::ATT_ELEMENT => {
                // A gain, so negative is attenuation: anything below half the
                // pad's depth is the pad in.
                self.attenuator = db <= -sdroxide_types::ELAD_ATTENUATOR_DB / 2.0;
                self.handle.set_attenuator(self.attenuator);
            }
            EladConfig::LPF_ELEMENT => {
                self.preselector = db >= 0.5;
                self.handle.set_preselector(self.preselector);
            }
            _ => {}
        }
        Ok(())
    }

    fn current_gains(&self) -> Vec<(String, f64)> {
        vec![(
            EladConfig::ATT_ELEMENT.to_string(),
            if self.attenuator { -sdroxide_types::ELAD_ATTENUATOR_DB } else { 0.0 },
        )]
    }

    /// Put the receiver on one of the transceiver's two antenna sockets.
    ///
    /// The rig's `AN` command, which is published as *how many* antennas are in
    /// use rather than as a port selector — and that is what it does: one
    /// antenna puts receive and transmit both on the RTX socket, two moves
    /// receive to the RX-only socket and leaves transmit on RTX. So there is no
    /// transmit port to choose here, and `set_tx_antenna` stays a no-op.
    ///
    /// It moves the whole receiver, this stream included: the DDC behind this
    /// USB interface is fed from the same front end as the audio the rig
    /// demodulates for itself.
    fn set_antenna(&mut self, name: &str) -> Result<()> {
        // Names from another radio are dropped rather than guessed at — a
        // session file remembers the port of whatever interface was last on
        // this radio, and that is one an ELAD has never heard of.
        let Some(ant) = EladAntenna::from_label(name) else {
            return Ok(());
        };
        match &self.control {
            Control::Serial(cat) => cat.set_antenna(name),
            Control::Gateway => self.handle.send_cat(sdroxide_cat::elad::antenna_frame(ant)),
            Control::None => return Ok(()),
        }
        self.antenna = ant;
        Ok(())
    }

    fn current_antenna(&self) -> String {
        // Nothing at all on a receiver with one input: the engine records what
        // this answers in `session.json`, and a port name from a device that
        // has no ports would be a claim about hardware that isn't there.
        if !self.switches_antenna() {
            return String::new();
        }
        self.antenna.label().to_string()
    }

    // ── Rig control ──────────────────────────────────────────────────────────

    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        let Control::Serial(cat) = &self.control else {
            return Vec::new();
        };
        let updates = cat.poll();
        // A radio that has said anything at all is a radio whose VFO this end
        // can move — the point of the latch is a DUO switched on, or a baud rate
        // corrected, after sdroxide started. See `IqSource::center_is_dial`.
        self.dial_reachable |= !updates.is_empty();
        let mut out = Vec::new();
        for u in updates {
            match u {
                sdroxide_cat::CatUpdate::Freq(hz) => {
                    // Nothing the rig says about its frequency means anything
                    // while we are keying it: the VFO is the transmit frequency
                    // for the length of the over, and reading that as the
                    // window having moved would slide the panadapter sideways
                    // on every key-down. `tx_end` puts it back, and the report
                    // that follows is the honest one.
                    if self.keyed {
                        continue;
                    }
                    match self.expect_freq {
                        // The rig arriving where we sent it. Retire the
                        // expectation and say nothing: the centre is already
                        // the number we commanded.
                        Some((want, _)) if (want - hz).abs() < 1.0 => self.expect_freq = None,
                        // Something else, with a command of ours still in
                        // flight — the rig on its way, or an answer that
                        // crossed our command on the wire. See `FREQ_SETTLE`.
                        Some((_, at)) if at.elapsed() < FREQ_SETTLE => {}
                        // The operator's knob, turned by hand. On this radio
                        // the VFO carries the window and the dial together, so
                        // this is both: the centre moves here (the engine reads
                        // it straight back out of `center_hz` when it takes the
                        // `Freq`, and the axis has to be relabelled against the
                        // same number the samples are now arriving on), and the
                        // dial goes up as `Freq` so the readout follows the
                        // radio instead of sitting where it was left.
                        //
                        // Nothing goes back at the VFO over the control link:
                        // the only write here is the DDC's own tuning word,
                        // which keeps the device's record of the centre in step
                        // with ours. So the operator's hand and this end cannot
                        // fight over the knob.
                        _ => {
                            self.expect_freq = None;
                            self.center = hz;
                            self.handle.set_center_hz(hz);
                            out.push(ControlUpdate::Freq(hz));
                        }
                    }
                }
                sdroxide_cat::CatUpdate::Mode(m) => out.push(ControlUpdate::Mode(m)),
                // Which socket the rig came up on, read once as the port
                // opened. Adopted rather than overridden: it is the operator's
                // own front-panel setting, and the serial thread has already
                // dropped an answer that crossed a command from this end.
                sdroxide_cat::CatUpdate::Antenna(name) => {
                    if let Some(a) = EladAntenna::from_label(name) {
                        self.antenna = a;
                        out.push(ControlUpdate::Antenna(name));
                    }
                }
                // The power the rig came up on, read once when the port opened.
                sdroxide_cat::CatUpdate::Power(frac) => out.push(ControlUpdate::TxDrive(frac)),
                // The ELAD family asks for no transmit state (see
                // `Protocol::tx_state_requests`), so this never arrives — but
                // the day it does, it means the same thing here as anywhere.
                sdroxide_cat::CatUpdate::Ptt(on) => out.push(ControlUpdate::RigTx(on)),
                // The meters arrive on their own telemetry channels, not here;
                // and the ELAD dialect has no squelch command, so that read is
                // never asked for either.
                //
                // Nor has an FDM-DUO a separate receiving antenna: its two
                // sockets are a choice between them, which is `Antenna` above.
                sdroxide_cat::CatUpdate::Swr(_)
                | sdroxide_cat::CatUpdate::Alc(_)
                | sdroxide_cat::CatUpdate::Po(_)
                | sdroxide_cat::CatUpdate::FwdW(_)
                | sdroxide_cat::CatUpdate::Squelch(_)
                | sdroxide_cat::CatUpdate::RxAntenna(_)
                | sdroxide_cat::CatUpdate::Signal(_) => {}
            }
        }
        out
    }

    fn set_control_mode(&mut self, mode: Mode) -> Result<()> {
        match &self.control {
            Control::Serial(cat) => cat.set_mode(mode),
            Control::Gateway => self.handle.send_cat(sdroxide_cat::elad::mode_frame(mode)),
            Control::None => {}
        }
        Ok(())
    }

    /// Only over the CAT link, which puts the mode through `digi_mode` and the
    /// dial (see `commanded_mode`). The gateway writes a mode frame straight to
    /// the radio from a table with no frequency in it, so there SSTV still has
    /// to arrive already translated to the sideband it rides.
    fn resolves_sstv_sideband(&self) -> bool {
        matches!(self.control, Control::Serial(_))
    }

    /// The rig's own receive filter.
    ///
    /// Sent even though sdroxide demodulates the wideband stream itself and has
    /// its own filter in front of the operator: the rig's filter is what shapes
    /// the audio it *transmits*, and on a radio that is also usable standalone
    /// it is the one the operator sees on the front panel. Only over the serial
    /// link — the USB gateway would carry it, but with no readback there is no
    /// way to know the rig took it.
    fn set_control_filter(&mut self, mode: Mode, lo_hz: f64, hi_hz: f64) {
        if let Control::Serial(cat) = &self.control {
            cat.set_filter(mode, lo_hz as f32, hi_hz as f32);
        }
    }

    /// Deliberately nothing, on a radio whose VFO is carrying the receive
    /// window.
    ///
    /// The engine offers this so a rig can be moved onto the transmit frequency
    /// *while still receiving* — an amplifier, a transverter or an antenna
    /// tuner downstream has to be on the right band before any RF appears. This
    /// radio has none of that behind it, only its own filter bank, and taking
    /// the offer up is what caused [issue #111]: the VFO carries the DDC
    /// window, so every dial move slid the window by the same amount and a
    /// click on a station 30 kHz down the band walked that station 30 kHz up
    /// the screen instead of tuning it.
    ///
    /// The transmit frequency is asserted at key-down instead — see
    /// [`IqSource::tx_begin`], and [`IqSource::tx_end`] for the way back.
    ///
    /// [issue #111]: https://github.com/dividebysandwich/sdroxide/issues/111
    fn set_tx_freq_hz(&mut self, _hz: f64) {}

    fn set_tx_drive(&mut self, frac: f64) {
        if let Control::Serial(cat) = &self.control {
            cat.set_power(frac as f32);
        }
    }

    /// Nothing to set: the rig has one power register, and the engine commands
    /// the tune level through [`Self::set_tx_drive`] for as long as TUNE holds
    /// the transmitter.
    fn set_tune_drive(&mut self, _frac: f64) {}

    fn commands_tx_power(&self) -> bool {
        match &self.control {
            Control::Serial(cat) => cat.commands_power(),
            _ => false,
        }
    }

    // The FDM-DUO has no text keyer — `SW` plays a message stored in the radio
    // and nothing hands it text — so `cw_text_keying` stays at its default of
    // `None` and CW is keyed by the operator's own key or paddle. See the
    // module header of `sdroxide_cat::elad`.

    // ── Transmit ─────────────────────────────────────────────────────────────

    fn tx_begin(&mut self, center_hz: f64, _rate: f64) -> Result<f64> {
        // The VFO has been sitting on the dial, which is the receive window's
        // centre, so ordinarily this asserts the frequency it is already on.
        // Under split or XIT it does not, and then it is a real retune that has
        // to land before the key does: the VFO is the only thing that decides
        // what frequency the over goes out on.
        self.keyed = true;
        self.command_freq(center_hz);
        self.command_ptt(true);
        Ok(self.out.as_ref().map(|(o, _)| o.sample_rate).unwrap_or(48_000.0))
    }

    fn tx_end(&mut self) -> Result<()> {
        self.command_ptt(false);
        // Give the receive window its centre back, in that order: the VFO is
        // moved once the transmitter is off, never while it is on. Without
        // this the panadapter would spend the rest of the session looking at
        // wherever the last over went out.
        self.keyed = false;
        self.command_freq(self.center);
        self.last_telem = None; // drop the stale SWR reading on unkey
        Ok(())
    }

    fn tx_write_audio(&mut self, audio: &[f32]) -> Result<()> {
        let Some((_, producer)) = self.out.as_mut() else {
            return Ok(()); // no TX audio device — PTT still keyed the rig
        };
        // Resample 48 kHz → card rate, then interleave to stereo (both
        // channels), with backpressure so the engine's TX loop is paced to real
        // time rather than generating a whole burst at CPU speed.
        self.tx_scratch.clear();
        match self.tx_resampler.as_mut() {
            Some(rs) => rs.push(audio, &mut self.tx_scratch),
            None => self.tx_scratch.extend_from_slice(audio),
        }
        for &s in &self.tx_scratch {
            for _ in 0..2 {
                let mut v = s;
                let mut tries = 0u32;
                while let Err(sdroxide_radio::rtrb::PushError::Full(x)) = producer.push(v) {
                    v = x;
                    tries += 1;
                    if tries > 200 {
                        break; // output device stalled — drop rather than hang TX
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        }
        Ok(())
    }

    fn tx_drain(&mut self) {
        // The output ring holds about a second; wait for it to play out before
        // PTT is released so the tail of a burst — which FT8 decoding depends
        // on — is not cut off.
        if let Some((_, producer)) = self.out.as_ref() {
            let cap = producer.buffer().capacity();
            for _ in 0..1000 {
                let buffered = cap.saturating_sub(producer.slots());
                if buffered <= cap / 40 {
                    break;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        }
    }

    fn tx_telemetry(&mut self) -> Option<TxTelemetry> {
        if let Control::Serial(cat) = &self.control
            && let Some(t) = cat.poll_telemetry()
        {
            self.last_telem = Some(t);
        }
        self.last_telem
    }

    fn rx_signal_dbm(&mut self) -> Option<f32> {
        // The rig's own meter, latched between its ~5 Hz answers and dropped
        // once it goes stale — a link that has stopped answering must not leave
        // a needle standing. With no serial link there is no meter, and the
        // engine falls back to measuring the I/Q itself, which for this
        // interface is a perfectly good answer.
        if let Control::Serial(cat) = &self.control
            && let Some(dbm) = cat.poll_signal()
        {
            self.last_signal = Some((std::time::Instant::now(), dbm));
        }
        self.last_signal.filter(|(at, _)| at.elapsed() < self.signal_max_age).map(|(_, dbm)| dbm)
    }

    // ── Lifecycle ────────────────────────────────────────────────────────────

    /// A device that has been unplugged, or whose thread has died, is reported
    /// as needing a reopen so the engine reconnects on its own — which is what
    /// makes replugging one Just Work rather than needing Apply pressed.
    fn needs_reopen(&self) -> bool {
        !self.handle.is_alive() || self.handle.silent_for() >= SILENCE_BEFORE_REOPEN
    }

    /// Hand the device back before the engine opens its replacement. Without
    /// this, changing anything in Settings → Radio on a running ELAD fails with
    /// "held by another program" — the other program being us.
    fn release(&mut self) {
        self.handle.release();
    }

    /// Surface what an operator needs to know but cannot see.
    fn open_status(&self) -> Option<String> {
        // The sample-rate check and the nothing-has-arrived warning both need a
        // second or two of stream before they can say anything, so they arrive
        // long after the open did — and once taken they are kept, because the
        // caller that asked first is not always the one that displays it.
        let mut late = self.late.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(w) = self.handle.take_late_warning()
            && !late.contains(&w)
        {
            late.push(w);
        }
        let mut parts = self.status.clone();
        parts.extend(late.iter().cloned());
        parts.push(
            "ELAD support is new and has not been verified against real hardware. \
             If it misbehaves, Settings → Radio has a Copy diagnostic report button."
                .to_string(),
        );
        Some(parts.join(" — "))
    }
}
