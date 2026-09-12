//! `PacketController` — AX.25 packet radio, on HF and on VHF/UHF.
//!
//! Modem, deframer, codec, the monitor an operator watches, CSMA, and the
//! connected-mode link with the terminal that drives it.
//!
//! # One link, two possible owners
//!
//! There is one radio and one channel, so there is one connected-mode link —
//! and two things that want it: the operator, typing at a node or a BBS from
//! the packet panel, and a Winlink forwarding session driven from the MAIL
//! window. They exclude each other through the lease on
//! [`sdroxide_ax25::PortEndpoint`], which either end may take and the other is
//! then refused, so the second one to ask gets a clear answer rather than the
//! two interleaving on one link and producing traffic neither understands.
//!
//! The lease also decides whether this station **answers** calls: while a
//! session holds the link an incoming connect has nowhere to go, so it is
//! refused with a DM rather than accepted into a link somebody else is using.
//!
//! # Why one controller for two modes
//!
//! [`Mode::Packet`] (VHF/UHF, FM) and [`Mode::PacketHf`] (HF, sideband) are the
//! same link layer over different radios. The waveform differs — Bell 202 tones
//! at 1200, a shaped scrambled baseband at 9600, 200 Hz-shift AFSK at 300 — but
//! above the bit stream every one of them is HDLC and AX.25, so the split lives
//! in the modem this holds, not in the controller or the protocol.
//!
//! The `Mode` split exists because the *radio* differs: FM against sideband
//! decides the filter, the modulator, the mode commanded over CAT and where the
//! band plan puts it. Those are per-`Mode` constants throughout the tree, and
//! one variant with a config field would have to thread that field into every
//! one of them.
//!
//! The modem, the framing and the two rules that make a packet station behave
//! on a shared channel live in [`crate::ax25_channel`], which
//! [`crate::AprsController`] runs as well.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sdroxide_ax25::{
    Addr, Packet, PacketType, PortEndpoint, PortEvent, PortLease, PortRequest, state,
};
use sdroxide_dsp::AfskProfile;
use sdroxide_types::{
    DigiConfig, DigiStatus, Mode, PACKET_HEARD_MAX, PACKET_TERM_LINE_MAX, PACKET_TERM_MAX,
    PacketBaud, PacketHeard, PacketLink, PacketLinkOwner, PacketStatus, PacketTermKind,
    PacketTermLine, QsoStep, TranscriptLine,
    text::{decode_cp1252, encode_cp1252},
};

use crate::DigiEngine;
use crate::ax25_channel::Ax25Channel;
use crate::controller::DigiAction;

/// The speed the modem is built for, given the mode and the operator's choice.
///
/// HF has exactly one speed, so a stale `Vhf9600` left in the config from a VHF
/// session must not follow the operator to 40 metres and produce a receiver
/// that decodes nothing without ever saying why.
fn baud_for(mode: Mode, cfg: &DigiConfig) -> PacketBaud {
    match mode {
        Mode::PacketHf => PacketBaud::Hf300,
        _ => match cfg.packet_baud {
            PacketBaud::Hf300 => PacketBaud::Vhf1200,
            b => b,
        },
    }
}

/// How long to wait for an acknowledgement, given the speed the modem runs at
/// and how big the frames it sends are.
///
/// The state machine ships with three seconds, which is a VHF figure. At 300
/// baud a 128-byte I frame is about 1200 bits with its header — four seconds
/// before the far end has heard the end of it, let alone answered — so T1
/// expires while the frame is still going out, the link retransmits into its
/// own traffic, and N2 runs out. That is why HF connected mode does not work
/// with the shipped value, and this is the whole of the fix.
///
/// The figure is one frame out, one supervisory frame back, plus the flags at
/// each end of both overs, doubled for a round trip and multiplied by the hops
/// — a digipeater repeats the whole frame, so two hops is three transmissions
/// of it. Clamped at both ends: below three seconds nothing on VHF benefits,
/// and above sixty a link that is genuinely dead takes N2 = 10 retries to say
/// so — and `select_t1_value` grows the estimate on each one, so the ceiling
/// is what keeps "gave up" inside a few minutes rather than tens of them.
fn srt_for(baud: PacketBaud, paclen: u16, txdelay_ms: u16, hops: usize) -> Duration {
    // Address field, control, PID, and the FCS: ~20 bytes on top of the
    // information field, and a supervisory frame back is ~18 with no payload.
    let bits = f64::from(paclen) * 8.0 + 20.0 * 8.0 + 18.0 * 8.0;
    let air = bits / baud.baud();
    let keying = 2.0 * f64::from(txdelay_ms) / 1000.0;
    let one_way = air + keying;
    let secs = 2.0 * one_way * (hops as f64 + 1.0);
    Duration::from_secs_f64(secs.clamp(3.0, 60.0))
}

/// The idle probe — T3, the "are you still there?" timer.
///
/// The shipped ten seconds is not just short, it is shorter than a working
/// link's own round trip on HF: an idle station keys its transmitter six times
/// a minute to ask a station that has nothing to say whether it is still
/// listening, and on a shared channel that is somebody else's QSO it is
/// stepping on. Comfortably longer than T1 (6.7.1.3), and floored at half a
/// minute so a quiet link is actually quiet.
fn t3_for(srt: Duration) -> Duration {
    (4 * srt).clamp(Duration::from_secs(30), Duration::from_secs(300))
}

/// A digipeater path as the operator writes it: callsigns nearest hop first,
/// separated by commas or spaces, the way `c CALL v A,B` takes them.
///
/// A bad callsign is named rather than skipped. Dropping a hop silently gives a
/// connect that goes out through the wrong path and fails with nothing on
/// screen to explain it, which is the failure this whole area is full of.
fn parse_via(via: &str) -> Result<Vec<Addr>, String> {
    via.split([',', ' '])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| Addr::new(s).map_err(|e| format!("{s}: {e}")))
        .collect()
}

/// The operator's terminal session: their claim on the link, and who it is with.
struct Term {
    /// Held for as long as the session lives, so a Winlink call is refused
    /// rather than taking the link out from under it. Released on drop.
    _lease: PortLease,
    peer: Addr,
    via: Vec<Addr>,
    /// True when we called them. The connect text goes to a station that
    /// called *us*: announcing ourselves to a BBS we dialled puts a line into
    /// its command parser before its prompt has arrived, and it answers with
    /// an error the operator did not cause.
    dialled: bool,
}

pub struct PacketController {
    mode: Mode,
    cfg: DigiConfig,
    tap_rate: f64,

    /// The modem, the framing and CSMA.
    ch: Ax25Channel,

    /// When the next beacon is due. `None` disables it.
    next_beacon: Option<SystemTime>,

    /// The connected-mode link, once something has asked for one.
    link: Option<Link>,
    /// The operator's session, when they are the one driving the link.
    term: Option<Term>,
    /// What the terminal has printed, oldest first, capped.
    term_lines: Vec<PacketTermLine>,
    /// The tail of a line that arrived without its terminator — a BBS prompt.
    term_partial: Vec<u8>,
    /// Since when the link has been up with nobody holding the lease. See the
    /// reaper in [`PacketController::poll_link`].
    orphan_since: Option<SystemTime>,
    /// Whether anybody has ever held the lease on the link that is up now.
    ///
    /// The reaper watches for an owner that *went away*, and "nobody ever
    /// claimed it" is not that: a caller may legitimately drive the port
    /// without taking a lease when it knows it is the only one — a test
    /// harness, a single-purpose host — and hanging up on it after two seconds
    /// would be a link that dies for no reason it can see.
    was_claimed: bool,
    /// Frames heard on the air, waiting for a KISS host to collect them.
    /// Capped, because a host that stops reading must not grow this forever.
    air_frames: Vec<Vec<u8>>,

    heard: Vec<PacketHeard>,
    bad_frames: u32,

    queued: Vec<DigiAction>,
    status_dirty: bool,
    last_status: Option<SystemTime>,
}

impl PacketController {
    pub fn new(mode: Mode, cfg: DigiConfig, tap_rate: f64) -> Self {
        let baud = baud_for(mode, &cfg);
        PacketController {
            mode,
            ch: Ax25Channel::new(baud, &cfg, tap_rate),
            cfg,
            tap_rate,
            next_beacon: None,
            link: None,
            term: None,
            term_lines: Vec::new(),
            term_partial: Vec::new(),
            orphan_since: None,
            was_claimed: false,
            air_frames: Vec::new(),
            heard: Vec::new(),
            bad_frames: 0,
            queued: Vec::new(),
            status_dirty: true,
            last_status: None,
        }
    }

    /// Rebuild the modem, after a speed change or a new tap rate.
    fn rebuild(&mut self) {
        self.ch.rebuild(baud_for(self.mode, &self.cfg), &self.cfg, self.tap_rate);
        // The link's timers are derived from the speed, so a modem rebuilt for
        // 300 baud with T1 still set for 1200 is the HF bug all over again.
        self.apply_link_settings();
        self.status_dirty = true;
    }

    /// Queue a frame for the next clear slot.
    ///
    /// Nothing is sent from here — CSMA decides when, on the audio clock. That
    /// separation is the point: a caller that could transmit directly would be
    /// able to key on top of another station.
    pub fn queue_frame(&mut self, frame: Vec<u8>) {
        self.ch.queue(frame);
        self.status_dirty = true;
    }

    /// Our own address, or `None` when the operator has not set one.
    ///
    /// A packet station with no callsign must not transmit — it would be an
    /// unidentified signal, which is illegal everywhere. Every transmit path
    /// goes through this and gives up quietly when it returns `None`.
    fn mycall(&self) -> Option<Addr> {
        let call = self.cfg.packet_mycall.trim();
        if call.is_empty() {
            return None;
        }
        Addr::new(call).ok()
    }

    /// Give the controller the engine end of a link port.
    ///
    /// The engine makes the pair and holds the other end, rather than the
    /// controller handing one out: a mode change destroys the controller, and
    /// the port's lifetime has to be something the engine can reason about.
    pub fn attach_port(&mut self, port: PortEndpoint) {
        self.link = Some(Link::new(port));
        self.apply_link_settings();
        self.refresh_accept();
        self.status_dirty = true;
    }

    /// Put the operator's link settings into the state machine.
    ///
    /// Called wherever any of them can change — the port arriving, the config,
    /// a speed change, a connect that sets the path — because the timers are
    /// derived from the speed *and* the packet length *and* the hop count, and
    /// there is no one place all three settle.
    ///
    /// Without this the config's `paclen` and `maxframe` reach `port_pair` and
    /// stop there, and the timers stay at the state machine's VHF defaults —
    /// which is the whole of why HF connected mode does not work.
    fn apply_link_settings(&mut self) {
        let baud = self.ch.baud();
        let paclen = self.cfg.packet_paclen.clamp(16, 256);
        let maxframe = self.cfg.packet_maxframe;
        let txdelay = self.cfg.packet_txdelay_ms;
        let Some(link) = self.link.as_mut() else { return };
        link.data.mtu(paclen as usize);
        link.data.maxframe(maxframe);
        let srt = srt_for(baud, paclen, txdelay, link.data.via().len());
        link.data.srt_default(srt);
        link.data.t3v(t3_for(srt));
    }

    /// Whether this station should be answering calls right now.
    ///
    /// The operator's setting, and the lease. While a Winlink session holds the
    /// link an incoming connect has nowhere to go: accepting it would drop a
    /// forwarding session into a conversation with a stranger. Refusing with a
    /// DM tells the caller to try later, which is true.
    fn refresh_accept(&mut self) {
        let want = self.cfg.packet_accept_incoming;
        let mine = self.term.is_some();
        let Some(link) = self.link.as_mut() else { return };
        link.data.set_accept_incoming(want && (mine || !link.port.is_claimed()));
    }

    /// Send what the link produced to whoever owns it.
    ///
    /// The seam. While the operator has the link its bytes go to the transcript
    /// and never near the port's bounded channel — which, with the operator
    /// owning it, nothing is draining. A session's bytes go to the port, and an
    /// event the port refuses fails the link rather than being dropped: a gap in
    /// a B2F stream is not noticed until the CRC at the end of the message.
    fn dispatch(&mut self, out: Vec<LinkOut>) {
        for ev in out {
            if self.term.is_some() {
                match ev {
                    LinkOut::Up => {
                        let (peer, via, dialled) = match self.term.as_ref() {
                            Some(t) => (t.peer.call().to_string(), t.via.clone(), t.dialled),
                            None => continue,
                        };
                        let path = if via.is_empty() {
                            String::new()
                        } else {
                            let hops: Vec<_> = via.iter().map(|a| a.call().to_string()).collect();
                            format!(" via {}", hops.join(","))
                        };
                        self.term_note(if dialled {
                            format!("*** connected to {peer}{path}")
                        } else {
                            format!("*** {peer} connected{path}")
                        });
                        // The one place the connect text goes out. `Up` is the
                        // single announce edge, so putting it anywhere else —
                        // where the call is adopted, say — greets the caller
                        // twice, once from each.
                        self.send_connect_text();
                    }
                    LinkOut::Down => {
                        self.term_note("*** disconnected".into());
                        self.end_term();
                    }
                    LinkOut::Failed(why) => {
                        // Said differently from a hangup on purpose: "the link
                        // gave up" and "they hung up" send an operator looking
                        // in completely different places.
                        self.term_note(format!("*** the link gave up — {why}"));
                        self.end_term();
                    }
                    LinkOut::Data(d) => self.term_absorb(&d),
                }
                continue;
            }
            let Some(link) = self.link.as_mut() else { continue };
            let sent = match ev {
                LinkOut::Up => link.port.emit(PortEvent::Connected),
                LinkOut::Down => link.port.emit(PortEvent::Disconnected),
                LinkOut::Failed(why) => link.port.emit(PortEvent::Failed(why)),
                LinkOut::Data(d) => link.port.emit(PortEvent::Data(d)),
            };
            if !sent {
                // The queue is full, so that event is gone. Carrying on would
                // hand the session a byte stream with a hole in it, and B2F
                // does not find out until the CRC at the end of the whole
                // message — so the link dies instead.
                //
                // Latched rather than emitted: a second `emit` would be
                // refused by the same full queue. The failure rides out on the
                // announce edge when the link reaches Disconnected, by which
                // time the session has either drained the queue and will read
                // it, or has gone away and nothing needed saying.
                tracing::warn!("the packet link's session stopped reading; failing the link");
                link.failure = Some("the session stopped reading and the link was closed".into());
                let mut frames = Vec::new();
                let mut more = Vec::new();
                link.handle(&state::Event::Disconnect, &mut frames, &mut more);
                for f in frames {
                    self.queue_frame(f);
                }
                return;
            }
        }
    }

    /// Drive the link: whatever its owner asked for, plus the timers.
    fn poll_link(&mut self, now: SystemTime) {
        let mut frames = Vec::new();
        let mut out = Vec::new();
        {
            let Some(link) = self.link.as_mut() else { return };

            for req in link.port.take_requests() {
                match req {
                    PortRequest::Connect { peer, via, ext } => {
                        // The path, which used to be destructured away here.
                        // A gateway two hops out was addressed direct, never
                        // heard us, and the session waited out its whole
                        // timeout against a station that had not been called.
                        link.data.set_via(via);
                        link.handle(
                            &state::Event::Connect { addr: peer, ext },
                            &mut frames,
                            &mut out,
                        );
                    }
                    PortRequest::Data(d) => {
                        link.handle(&state::Event::Data(d), &mut frames, &mut out);
                    }
                    PortRequest::Disconnect => {
                        link.handle(&state::Event::Disconnect, &mut frames, &mut out);
                    }
                }
            }

            // Timers. These belong here rather than on the audio clock: T1 and
            // T3 are seconds, and `poll` runs often enough for seconds even at
            // the 341 ms worst case. The CSMA slot clock is the one that cannot
            // live here — see rule 2.
            if link.data.t1_expired() {
                link.handle(&state::Event::T1, &mut frames, &mut out);
            }
            if link.data.t3_expired() {
                link.handle(&state::Event::T3, &mut frames, &mut out);
            }

            link.announce(&mut out);

            // The reaper. An owner that went away without unwinding — a killed
            // worker — leaves a live link nobody is driving, and the far end
            // waits out T3 × N2 before it works that out. The two seconds is
            // what keeps this away from the gap between a claim and the connect
            // that follows it, and `was_claimed` is what keeps it away from a
            // caller that never took a lease in the first place.
            if link.state.is_state_disconnected() {
                self.was_claimed = false;
            } else if link.port.is_claimed() {
                self.was_claimed = true;
            }
            let orphan =
                !link.state.is_state_disconnected() && !link.port.is_claimed() && self.was_claimed;
            if orphan && self.term.is_none() {
                match self.orphan_since {
                    None => self.orphan_since = Some(now),
                    Some(since) => {
                        if now.duration_since(since).map(|d| d.as_secs() >= 2).unwrap_or(false) {
                            tracing::warn!("packet link left open by its owner; disconnecting");
                            self.orphan_since = None;
                            link.handle(&state::Event::Disconnect, &mut frames, &mut out);
                        }
                    }
                }
            } else {
                self.orphan_since = None;
            }
        }

        for f in frames {
            self.queue_frame(f);
        }
        self.dispatch(out);
        self.refresh_accept();
    }

    /// Hand a received frame to the link, if one is open and it is addressed
    /// to us.
    fn feed_link(&mut self, p: &Packet) {
        let Some(link) = self.link.as_mut() else { return };
        // Compare the **callsigns**, not the addresses.
        //
        // `Addr` carries four more bits than the call — command/response,
        // end-of-address, and two reserved — and they differ between a frame
        // off the air and a freshly parsed `Addr::new`. Comparing whole
        // addresses therefore never matches, and the symptom is a station that
        // hears a SABM addressed to it, files it in the monitor, and never
        // answers. Silent, and indistinguishable from a dead receiver.
        if p.dst().call() != link.port.cfg.me.call() {
            // Somebody else's traffic. It belongs in the monitor, which has
            // already had it, and nowhere near our sequence numbers.
            return;
        }
        // A link belongs to one station.
        //
        // The state machine takes a SABM in the Connected state as a request to
        // reset the link, and never asks who sent it — so any station on the
        // channel can tear down a session in progress by calling us, and the
        // operator watches their BBS vanish with nothing on screen to explain
        // it. While a link is up, only the station it is up with gets a say.
        if let Some(peer) = link.data.peer()
            && !link.state.is_state_disconnected()
            && p.src().call() != peer.call()
        {
            return;
        }
        let mut frames = Vec::new();
        let mut out = Vec::new();
        let cr = p.command_response();
        // Whether this frame is a fresh call *to* us, which is the only thing
        // that hands the link to the operator's terminal. Read before the
        // machine runs, because by the time it has the state has moved.
        let calling_us = matches!(p.packet_type(), PacketType::Sabm(_) | PacketType::Sabme(_))
            && link.state.is_state_disconnected();
        match p.packet_type() {
            PacketType::Sabm(f) => {
                answer_through(link, p);
                link.handle(&state::Event::Sabm(f.clone(), p.src().clone()), &mut frames, &mut out);
            }
            PacketType::Sabme(f) => {
                answer_through(link, p);
                link.handle(
                    &state::Event::Sabme(f.clone(), p.src().clone()),
                    &mut frames,
                    &mut out,
                );
            }
            PacketType::Ua(f) => link.handle(&state::Event::Ua(f.clone()), &mut frames, &mut out),
            PacketType::Dm(f) => link.handle(&state::Event::Dm(f.clone()), &mut frames, &mut out),
            PacketType::Disc(f) => {
                link.handle(&state::Event::Disc(f.clone()), &mut frames, &mut out);
            }
            PacketType::Iframe(f) => {
                link.handle(&state::Event::Iframe(f.clone(), cr), &mut frames, &mut out);
            }
            PacketType::Rr(f) => {
                link.handle(&state::Event::Rr(f.clone(), cr), &mut frames, &mut out);
            }
            PacketType::Rnr(f) => link.handle(&state::Event::Rnr(f.clone()), &mut frames, &mut out),
            PacketType::Rej(f) => link.handle(&state::Event::Rej(f.clone()), &mut frames, &mut out),
            PacketType::Srej(f) => {
                link.handle(&state::Event::Srej(f.clone()), &mut frames, &mut out);
            }
            PacketType::Frmr(f) => {
                link.handle(&state::Event::Frmr(f.clone()), &mut frames, &mut out);
            }
            PacketType::Xid(f) => {
                link.handle(&state::Event::Xid(f.clone(), cr), &mut frames, &mut out);
            }
            PacketType::Test(f) => {
                link.handle(&state::Event::Test(f.clone(), cr), &mut frames, &mut out);
            }
            // UI frames are monitor traffic; they carry no link state.
            PacketType::Ui(_) => {}
        }
        for f in frames {
            self.queue_frame(f);
        }
        // A call *we answered* is now the operator's session, if nobody else
        // holds the link.
        //
        // `calling_us` and not simply "the link is up": a link that came up
        // because our own SABM was answered belongs to whoever asked for it —
        // a Winlink session driving the port — and adopting that one takes the
        // link out from under them, sending the bytes to a transcript nobody
        // is reading while the session waits for a connect event that never
        // arrives.
        //
        // Before the announcement below, so that when the link is announced
        // there is already an owner to announce it to.
        if calling_us
            && self.term.is_none()
            && self.link.as_ref().is_some_and(|l| l.state.is_state_connected())
        {
            self.adopt_incoming();
        }
        if let Some(link) = self.link.as_mut() {
            link.announce(&mut out);
        }
        self.dispatch(out);
        self.refresh_accept();
    }

    /// Take over a call the state machine has just accepted.
    ///
    /// The lease may fail: a Winlink session can have claimed the link in the
    /// window between `refresh_accept` letting the SABM through and the UA
    /// going out. Rare, and the honest answer is to hang up rather than hold
    /// a conversation neither side can see.
    fn adopt_incoming(&mut self) {
        let Some(link) = self.link.as_mut() else { return };
        let Some(peer) = link.data.peer().cloned() else { return };
        let via = link.data.via().to_vec();
        match link.port.try_claim() {
            Some(lease) => {
                self.term = Some(Term { _lease: lease, peer, via, dialled: false });
            }
            None => {
                let mut frames = Vec::new();
                let mut out = Vec::new();
                if let Some(link) = self.link.as_mut() {
                    link.handle(&state::Event::Disconnect, &mut frames, &mut out);
                }
                for f in frames {
                    self.queue_frame(f);
                }
            }
        }
        self.status_dirty = true;
    }

    /// Make the link use the callsign the operator has set.
    ///
    /// The engine bakes the callsign into the port when the mode starts, with
    /// `N0CALL` when there is not one yet — and filling in the station call
    /// *after* selecting packet is the obvious order to do it in. Without this,
    /// beacons go out correctly (they read the live config) while every SABM
    /// goes out as `N0CALL`, and the operator sees a call that nobody answers.
    ///
    /// Only while the link is idle and unclaimed: rebuilding the state machine
    /// under a live session would lose it.
    fn adopt_callsign(&mut self, me: &Addr) {
        let Some(link) = self.link.as_mut() else { return };
        if link.port.cfg.me.call() == me.call() {
            return;
        }
        if self.term.is_some() || link.port.is_claimed() || !link.state.is_state_disconnected() {
            self.term_note(format!(
                "*** the link is busy — it will start using {} once it is free",
                me.call()
            ));
            return;
        }
        link.port.cfg.me = me.clone();
        link.state = state::new();
        link.data = state::Data::new(me.clone());
        link.announced = false;
        link.failure = None;
        self.apply_link_settings();
        self.refresh_accept();
    }

    // ───────────────────────── the operator's terminal ──────────────────

    /// Call a station in connected mode.
    ///
    /// Every refusal comes back as a line in the transcript rather than
    /// vanishing: the operator pressed CONNECT and is looking at this pane, so
    /// this is where "you have no callsign", "that is not a callsign" and "the
    /// MAIL window has the link" belong.
    pub fn term_connect(&mut self, call: &str, via: &str, ext: bool) {
        if self.term.is_some() {
            self.term_note("*** already connected — disconnect first".into());
            return;
        }
        let Some(me) = self.mycall() else {
            self.term_note(
                "*** set a station callsign first — the SETUP button above the monitor pane".into(),
            );
            return;
        };
        let peer = match Addr::new(call.trim()) {
            Ok(p) => p,
            Err(e) => {
                self.term_note(format!("*** {e}"));
                return;
            }
        };
        // The operator's box wins; the setting is what fills in when they leave
        // it empty, which is the case the setting exists for — the path to a
        // local node is the same every time.
        let via = if via.trim().is_empty() { self.cfg.packet_connect_via.trim() } else { via };
        let hops = match parse_via(via) {
            Ok(v) => v,
            Err(e) => {
                self.term_note(format!("*** bad digipeater path — {e}"));
                return;
            }
        };
        if self.link.is_none() {
            self.term_note("*** the packet link is unavailable".into());
            return;
        }
        // The callsign may have been typed after the mode started, in which
        // case the link is still addressed as whatever it was built with.
        self.adopt_callsign(&me);
        let Some(link) = self.link.as_mut() else { return };
        let Some(lease) = link.port.try_claim() else {
            self.term_note("*** the MAIL window is using the link — one session at a time".into());
            return;
        };
        link.data.set_via(hops.clone());
        link.failure = None;
        self.term =
            Some(Term { _lease: lease, peer: peer.clone(), via: hops.clone(), dialled: true });
        // After `set_via`, because the timers count the hops: a call through
        // two digipeaters takes three transmissions of every frame each way.
        self.apply_link_settings();

        let path = if hops.is_empty() {
            String::new()
        } else {
            let names: Vec<_> = hops.iter().map(|a| a.call().to_string()).collect();
            format!(" via {}", names.join(","))
        };
        self.term_note(format!("*** calling {}{path}…", peer.call()));

        let mut frames = Vec::new();
        let mut out = Vec::new();
        if let Some(link) = self.link.as_mut() {
            link.handle(&state::Event::Connect { addr: peer, ext }, &mut frames, &mut out);
        }
        for f in frames {
            self.queue_frame(f);
        }
        self.dispatch(out);
        self.refresh_accept();
        self.status_dirty = true;
    }

    /// Send one line to the connected station.
    pub fn term_send(&mut self, text: String) {
        if self.term.is_none() {
            self.term_note("*** not connected".into());
            return;
        }
        if !self.link.as_ref().is_some_and(|l| l.state.is_state_connected()) {
            self.term_note("*** the link is not up yet".into());
            return;
        }
        let line = text.trim_end_matches(['\r', '\n']).to_string();
        // Carriage return, not newline. AX.25 keyboard traffic, every BBS and
        // every node command line end a line with CR; a station sent LF sits
        // there waiting for a command it has not been given.
        let mut bytes = encode_cp1252(&line);
        bytes.push(b'\r');

        self.term_line(PacketTermKind::Tx, line);
        let mut frames = Vec::new();
        let mut out = Vec::new();
        if let Some(link) = self.link.as_mut() {
            link.handle(&state::Event::Data(bytes), &mut frames, &mut out);
        }
        for f in frames {
            self.queue_frame(f);
        }
        self.dispatch(out);
        self.status_dirty = true;
    }

    /// Hang up: a clean DISC, waited out.
    pub fn term_disconnect(&mut self) {
        if self.term.is_none() {
            return;
        }
        self.term_note("*** disconnecting…".into());
        let mut frames = Vec::new();
        let mut out = Vec::new();
        if let Some(link) = self.link.as_mut() {
            link.handle(&state::Event::Disconnect, &mut frames, &mut out);
        }
        for f in frames {
            self.queue_frame(f);
        }
        self.dispatch(out);
        self.status_dirty = true;
    }

    /// Empty the transcript. The link is untouched — a connected station stays
    /// connected, the same rule `clear_rx` follows for the monitor.
    pub fn term_clear(&mut self) {
        self.term_lines.clear();
        self.term_partial.clear();
        self.status_dirty = true;
    }

    /// The session is over: let the lease go so Winlink — or the next call —
    /// can have the link.
    fn end_term(&mut self) {
        self.term = None;
        // Anything the far end sent without a terminator is the last thing it
        // said, and it is not going to finish the line now.
        self.flush_partial();
        self.orphan_since = None;
        self.refresh_accept();
        self.status_dirty = true;
    }

    /// Greet a station that called us. See [`Term::dialled`].
    fn send_connect_text(&mut self) {
        if self.term.as_ref().is_none_or(|t| t.dialled) {
            return;
        }
        let text = self.cfg.packet_connect_text.trim().to_string();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            self.term_send(line.to_string());
        }
    }

    /// Bytes off the link become terminal lines.
    ///
    /// A BBS ends a line with CR; some send CR LF and a few send LF alone, so
    /// both are accepted and an LF straight after a CR opens no empty line. The
    /// other C0 controls are dropped: a node that sends a bell or a form feed
    /// should not be able to put a control character into a transcript that
    /// crosses a wire into somebody else's text renderer.
    ///
    /// Whole lines are decoded rather than a running stream, which sidesteps a
    /// UTF-8 sequence split across two I frames entirely.
    fn term_absorb(&mut self, bytes: &[u8]) {
        for &b in bytes {
            match b {
                b'\r' => self.flush_partial(),
                b'\n' => {
                    // Only if it did not just follow a CR — otherwise CR LF
                    // opens a blank line between every real one.
                    if !self.term_partial.is_empty() {
                        self.flush_partial();
                    }
                }
                0x00..=0x1f | 0x7f => {}
                _ => {
                    if self.term_partial.len() < PACKET_TERM_LINE_MAX {
                        self.term_partial.push(b);
                    }
                }
            }
        }
        self.status_dirty = true;
    }

    /// Turn whatever is in the partial line into a transcript line.
    fn flush_partial(&mut self) {
        if self.term_partial.is_empty() {
            return;
        }
        let bytes = std::mem::take(&mut self.term_partial);
        self.term_line(PacketTermKind::Rx, decode_cp1252(&bytes));
    }

    /// sdroxide's own voice in the transcript: the link came up, the call was
    /// refused, the far end gave up.
    fn term_note(&mut self, text: String) {
        self.term_line(PacketTermKind::Note, text);
    }

    fn term_line(&mut self, kind: PacketTermKind, text: String) {
        let at =
            SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
        let mut text = text;
        text.truncate(text.char_indices().nth(PACKET_TERM_LINE_MAX).map_or(text.len(), |(i, _)| i));
        if self.term_lines.len() >= PACKET_TERM_MAX {
            self.term_lines.remove(0);
        }
        self.term_lines.push(PacketTermLine { at, kind, text });
        self.status_dirty = true;
    }

    /// Frames heard since the last call, for a KISS host.
    ///
    /// Drained rather than pushed: the engine already polls this controller, and
    /// a controller that reached out to a socket would have to know one exists.
    pub fn take_air_frames(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.air_frames)
    }

    /// Transmit a frame on a KISS host's behalf.
    ///
    /// It goes through CSMA like everything else — a host cannot key the radio
    /// directly, only ask. The frame is used verbatim: KISS hands over address
    /// through information and the FCS is ours to add, exactly as a TNC's
    /// firmware would.
    pub fn send_from_host(&mut self, frame: Vec<u8>) {
        self.queue_frame(frame);
    }

    /// Send a beacon now, on the operator's command, rather than waiting for
    /// the timer. Still subject to CSMA — nothing here bypasses the channel.
    pub fn queue_beacon_now(&mut self) {
        self.queue_beacon();
    }

    /// Queue an UNPROTO beacon carrying the operator's text.
    fn queue_beacon(&mut self) {
        let (Some(me), Some(dest)) = (self.mycall(), Addr::new("BEACON").ok()) else {
            return;
        };
        let text = self.cfg.packet_beacon_text.trim().to_string();
        if text.is_empty() {
            return;
        }
        let p = Packet::ui(me, dest, text.into_bytes());
        self.queue_frame(p.serialize(false));
    }

    fn note(&mut self, entry: PacketHeard) {
        if self.heard.len() >= PACKET_HEARD_MAX {
            self.heard.remove(0);
        }
        self.heard.push(entry);
        self.status_dirty = true;
    }

    /// Turn a verified frame into a monitor line.
    ///
    /// A frame that fails to parse is still recorded rather than dropped
    /// silently: it passed a 16-bit FCS, so it is almost certainly a real frame
    /// of a kind the codec does not handle, and an operator staring at a quiet
    /// pane deserves to know the difference between "nothing is being heard"
    /// and "something is being heard and not understood".
    fn on_frame(&mut self, bytes: &[u8]) {
        self.note_frame(bytes, false);
    }

    fn note_frame(&mut self, bytes: &[u8], sent: bool) {
        let at =
            SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
        // `None` means "work out the sequence-number width from the reserved
        // bit in the source address", rather than assuming mod-8.
        //
        // A monitor is reading connections it did not join, so it cannot know:
        // an S or I frame has a one-byte control field under mod-8 and a
        // two-byte one under mod-128, and guessing wrong shifts everything
        // after it. The reserved bit is what the Linux stack and `axlisten` use
        // to signal the difference. It is not in the standard, so a mod-128
        // station that does not set it is still mis-read — but nothing short of
        // having seen the SABME can do better, and assuming mod-8 outright is
        // wrong strictly more often.
        match Packet::parse(bytes, None) {
            Ok(p) => {
                self.feed_link(&p);
                let (kind, text) = describe(&p);
                let entry = PacketHeard {
                    at,
                    from: p.src().call().to_string(),
                    to: p.dst().call().to_string(),
                    via: p.digipeaters().iter().map(|a| a.call().to_string()).collect(),
                    kind,
                    text,
                    sent,
                };
                self.note(entry);
            }
            Err(e) => {
                tracing::debug!("packet frame passed its FCS but would not parse: {e}");
                self.note(PacketHeard {
                    at,
                    from: String::new(),
                    to: String::new(),
                    via: Vec::new(),
                    kind: "?".into(),
                    text: format!("unparsed, {} bytes", bytes.len()),
                    sent,
                });
            }
        }
    }

    fn packet_status(&self) -> PacketStatus {
        // Who has the link. `is_claimed` without a `term` of our own means a
        // Winlink session, because those are the only two things that can hold
        // it — and it is the answer to "why was I refused", which an operator
        // otherwise has to guess at.
        let owner = match self.link.as_ref() {
            Some(_) if self.term.is_some() => PacketLinkOwner::Terminal,
            Some(l) if l.port.is_claimed() => PacketLinkOwner::Session,
            _ => PacketLinkOwner::Idle,
        };
        PacketStatus {
            baud: self.ch.baud(),
            dcd: self.ch.dcd,
            level: self.ch.level(),
            heard: self.heard.clone(),
            bad_frames: self.bad_frames,
            link: self.link.as_ref().map(|l| l.status(owner)),
            term: self.term_lines.clone(),
            term_partial: decode_cp1252(&self.term_partial),
        }
    }

    fn digi_status(&self) -> DigiStatus {
        DigiStatus {
            mode: self.mode,
            step: QsoStep::Idle,
            dx_call: None,
            dx_grid: None,
            tx_next: false,
            tx_pending_msg: None,
            audio_hz: self.audio_hz(),
            tx_even: false,
            transmitting: self.ch.keyed,
            tx_watchdog: false,
            transcript: Vec::<TranscriptLine>::new(),
            config: self.cfg.clone(),
            text_rx: String::new(),
            tx_sent: 0,
            fsq_heard: Vec::new(),
            fsq_messages: Vec::new(),
            rade: None,
            packet: Some(self.packet_status()),
            navtex: None,
            aprs: None,
            js8: None,
            atchat: None,
            fox_queue: Vec::new(),
            call_queue: Vec::new(),
            clock_offset_s: None,
            cw: None,
            wspr: None,
            qso: None,
        }
    }
}

/// What the link wants said, once its frames are on the transmit queue.
///
/// Returned rather than emitted, because the destination depends on who owns
/// the link: the operator's transcript, or a session's byte stream. Sending
/// straight to the port would also put the operator's own bytes through a
/// bounded channel that, while they own it, nothing is draining.
#[derive(Debug)]
enum LinkOut {
    Up,
    /// A clean shutdown — DISC/UA, or the peer went away politely.
    Down,
    /// The machine gave up: retries exhausted, the call refused. Carries
    /// something worth putting in front of whoever was using the link.
    Failed(String),
    Data(Vec<u8>),
}

/// A connected-mode link: the vendored state machine plus the port it serves.
///
/// The machine is pure — events in, actions out, no I/O and no clock of its
/// own — so everything about *when* lives here.
struct Link {
    state: Box<dyn state::State>,
    data: state::Data,
    port: PortEndpoint,
    /// Whether the owner has been told the link is up, so `Up` is said exactly
    /// once.
    announced: bool,
    /// Latched when the machine gave up rather than closed.
    ///
    /// Without it a dead link and a clean hangup are the same event — both are
    /// just a return to Disconnected — and neither a Winlink transcript nor an
    /// operator can tell "the far end hung up" from "nobody was there". Read
    /// and cleared where the state edge is noticed.
    failure: Option<String>,
}

impl Link {
    fn new(port: PortEndpoint) -> Link {
        Link {
            state: state::new(),
            data: state::Data::new(port.cfg.me.clone()),
            port,
            announced: false,
            failure: None,
        }
    }

    /// Feed one event and collect what falls out: frames to transmit, and
    /// anything the link's owner needs to hear.
    fn handle(&mut self, ev: &state::Event, frames: &mut Vec<Vec<u8>>, out: &mut Vec<LinkOut>) {
        let (next, returns) = state::handle(&*self.state, &mut self.data, ev);
        if let Some(next) = next {
            self.state = next;
        }
        let ext = self.data.ext();
        for r in returns {
            match &r {
                state::ReturnEvent::Data(state::Res::Some(d)) => out.push(LinkOut::Data(d.clone())),
                state::ReturnEvent::Data(state::Res::EOF) => out.push(LinkOut::Down),
                state::ReturnEvent::Data(state::Res::None) => {}
                state::ReturnEvent::DlError(e) => {
                    // Most link-layer errors are recoverable and the machine
                    // says so by carrying on. These are the ones it does not
                    // come back from — retries exhausted against an
                    // unanswered frame, an enquiry nobody replied to, a
                    // connect that timed out — and they are the difference
                    // between "the BBS hung up" and "the BBS was never there".
                    // The machine signals both the same way, by returning to
                    // Disconnected, so the reason has to be caught here.
                    if matches!(
                        e,
                        state::DlError::G
                            | state::DlError::H
                            | state::DlError::I
                            | state::DlError::T
                            | state::DlError::U
                            | state::DlError::V
                    ) {
                        self.failure = Some(e.to_string());
                    }
                    tracing::debug!("AX.25 link error: {e}");
                }
                state::ReturnEvent::Packet(_) => {}
            }
            if let Some(frame) = r.serialize(ext) {
                frames.push(frame);
            }
        }
    }

    /// Say the link came up, or went down, exactly once.
    ///
    /// Called wherever the state can have moved — a received frame, a timer —
    /// rather than only on the next poll. A link that comes up and is spoken to
    /// in the same over would otherwise print what the far end said *before*
    /// saying it had connected, because the announcement was waiting for a poll
    /// that had not come round yet.
    ///
    /// `is_state_connected` and not `data.peer().is_some()`: the peer is
    /// recorded the moment a connect is *requested*, so testing that announces
    /// a link still waiting for its UA — and the owner then writes into a
    /// machine that refuses with "writing data while not connected".
    fn announce(&mut self, out: &mut Vec<LinkOut>) {
        let up = self.state.is_state_connected();
        if up && !self.announced {
            self.announced = true;
            self.failure = None;
            // Ahead of anything the same frame delivered: "connected" is the
            // line that has to come first.
            out.insert(0, LinkOut::Up);
        } else if !up && self.announced {
            self.announced = false;
            out.push(self.failure.take().map_or(LinkOut::Down, LinkOut::Failed));
        } else if !up && self.failure.is_some() {
            // A call that never came up at all: ten SABMs into silence and a
            // DL-ERROR. There is no announced-to-not-announced edge to catch
            // here, so without this the caller is told nothing and waits out
            // its own timeout — two minutes, for a link that gave up after
            // forty seconds.
            let why = self.failure.take().unwrap_or_default();
            out.push(LinkOut::Failed(why));
        }
    }

    /// The link's state as the panel shows it.
    fn status(&self, owner: PacketLinkOwner) -> PacketLink {
        PacketLink {
            state: self.state.name(),
            peer: self.data.peer().map(|p| p.call().to_string()),
            via: self.data.via().iter().map(|a| a.call().to_string()).collect(),
            ext: self.data.ext(),
            unacked: u16::try_from(self.data.unacked()).unwrap_or(u16::MAX),
            pending: u32::try_from(self.data.pending_out()).unwrap_or(u32::MAX),
            retries: self.data.retries(),
            owner,
        }
    }
}

/// Answer a call through the path it arrived on, reversed.
///
/// A station reached through a node has to be answered through it: a UA sent
/// direct never gets there, the caller retries ten times and gives up, and both
/// ends see a station that heard them and would not talk.
///
/// The whole list is reversed rather than only the hops whose has-been-repeated
/// bit is set, which is what Direwolf and the Linux stack do — a path that got
/// the frame here works in reverse, and reading the bits instead would drop a
/// hop whenever a digipeater set them in a way the standard does not require.
fn answer_through(link: &mut Link, p: &Packet) {
    let mut back = p.digipeaters().to_vec();
    back.reverse();
    link.data.set_via(back);
}

/// The frame type as a monitor prints it, and whatever text it carried.
fn describe(p: &Packet) -> (String, String) {
    let text = |b: &[u8]| String::from_utf8_lossy(b).trim_end_matches('\n').to_string();
    match p.packet_type() {
        PacketType::Ui(f) => ("UI".into(), text(&f.payload)),
        PacketType::Iframe(f) => (format!("I{}{}", f.ns, f.nr), text(&f.payload)),
        PacketType::Sabm(_) => ("SABM".into(), String::new()),
        PacketType::Sabme(_) => ("SABME".into(), String::new()),
        PacketType::Ua(_) => ("UA".into(), String::new()),
        PacketType::Dm(_) => ("DM".into(), String::new()),
        PacketType::Disc(_) => ("DISC".into(), String::new()),
        PacketType::Rr(f) => (format!("RR{}", f.nr), String::new()),
        PacketType::Rnr(f) => (format!("RNR{}", f.nr), String::new()),
        PacketType::Rej(f) => (format!("REJ{}", f.nr), String::new()),
        PacketType::Srej(f) => (format!("SREJ{}", f.nr), String::new()),
        PacketType::Frmr(_) => ("FRMR".into(), String::new()),
        PacketType::Xid(_) => ("XID".into(), String::new()),
        PacketType::Test(f) => ("TEST".into(), text(&f.payload)),
    }
}

impl DigiEngine for PacketController {
    fn mode(&self) -> Mode {
        self.mode
    }

    /// Rule 1: never while our own transmitter is up.
    ///
    /// The gate is here rather than in the engine because the engine has good
    /// reasons to keep the receive chain running through an over — a full-duplex
    /// front end genuinely is receiving, and the panadapter wants it. It is this
    /// mode that must not listen, so this is where the refusal belongs.
    fn on_rx_audio(&mut self, tap: &[f32]) {
        let r = self.ch.on_rx_audio(tap, &self.cfg);
        for f in r.frames {
            // Keep a copy for any KISS host before the codec gets an opinion:
            // a host is entitled to frames we cannot parse, which is most of
            // the point of offering the modem rather than the decoder.
            if self.air_frames.len() < PACKET_HEARD_MAX {
                self.air_frames.push(f.clone());
            }
            self.on_frame(&f);
        }
        if r.bad > 0 {
            self.bad_frames = self.bad_frames.saturating_add(r.bad);
            self.status_dirty = true;
        }
        if r.dcd_changed {
            self.status_dirty = true;
        }
    }

    fn poll(&mut self, now_in: SystemTime, _dial_hz: f64) -> Vec<DigiAction> {
        let mut actions = std::mem::take(&mut self.queued);

        self.poll_link(now_in);

        // Beacon. Scheduled here rather than on the audio clock because minutes
        // are exactly what `poll`'s cadence is good for, and because a beacon
        // that slips a few hundred milliseconds is a beacon nobody notices.
        let every = self.cfg.packet_beacon_minutes;
        if every == 0 || self.cfg.packet_beacon_text.trim().is_empty() {
            self.next_beacon = None;
        } else {
            match self.next_beacon {
                // Wait one interval before the first one: keying the moment the
                // operator selects the mode is startling, and the settings they
                // are about to type are not in yet.
                None => self.next_beacon = Some(now_in + Duration::from_secs(every as u64 * 60)),
                Some(at) if now_in >= at => {
                    self.queue_beacon();
                    self.next_beacon = Some(now_in + Duration::from_secs(every as u64 * 60));
                }
                Some(_) => {}
            }
        }

        // CSMA has cleared us to transmit: key up through the engine's normal
        // PTT path so the station interlock and the band rails apply.
        if let Some(sent) = self.ch.take_over(&self.cfg) {
            // Our own traffic belongs in the monitor too: an operator watching
            // a session needs both halves of it, and a beacon that never
            // appears looks exactly like a beacon that was never sent.
            for f in sent {
                self.note_frame(&f, true);
            }
            self.status_dirty = true;
            actions.push(DigiAction::KeyTx);
        }

        let now = SystemTime::now();
        let due = self
            .last_status
            .map(|t| now.duration_since(t).map(|d| d.as_secs_f32() > 0.2).unwrap_or(true))
            .unwrap_or(true);
        if self.status_dirty || due {
            self.status_dirty = false;
            self.last_status = Some(now);
            actions.push(DigiAction::Status(self.digi_status()));
        }
        actions
    }

    fn tx_burst_active(&self) -> bool {
        self.ch.keyed
    }

    /// Whichever modem this speed uses, its own headroom — the shaped 9600
    /// baseband keeps more of it than the 1200 tone does, and declaring the
    /// tone's figure for both would drive the shaping peaks into the limiter
    /// and close the eye.
    fn tx_peak(&self) -> f32 {
        self.ch.tx_peak()
    }

    fn fill_tx_block(&mut self, out: &mut [f32]) -> bool {
        self.ch.fill_tx_block(out)
    }

    /// The AX.25 modem is built at the tap rate and transmits there, so this
    /// is one of the two modes whose transmit audio is not 48 kHz.
    fn tx_rate(&self) -> f64 {
        self.tap_rate
    }

    fn on_burst_done(&mut self) {
        self.ch.on_burst_done();
        self.status_dirty = true;
    }

    /// The operator's stop. Everything the transmitter holds goes, and a
    /// session of theirs goes with it — including the DISC, which is queued
    /// ahead of the clear so the far end is told rather than left to time out.
    fn abort(&mut self) {
        if self.term.is_some() {
            self.term_disconnect();
        }
        self.ch.abort();
        self.status_dirty = true;
    }

    /// The safety rails refused the key-up, or the operator stopped the over.
    ///
    /// The queue is left alone. A refused key is usually temporary — another
    /// radio holds the station interlock — and throwing the frames away would
    /// turn a moment's contention into lost traffic. CSMA will try again.
    fn abort_tx(&mut self) {
        self.ch.abort_tx();
        self.status_dirty = true;
    }

    fn set_config(&mut self, cfg: DigiConfig) {
        let was = self.ch.baud();
        self.cfg = cfg;
        if baud_for(self.mode, &self.cfg) != was {
            self.rebuild();
        }
        // After the rebuild, because the timers are derived from the speed.
        self.apply_link_settings();
        self.refresh_accept();
        if let Some(me) = self.mycall() {
            self.adopt_callsign(&me);
        }
        self.status_dirty = true;
    }

    /// The operator's audio-frequency control does not apply: on FM the signal
    /// is centred on the dial, and on HF the tone pair is fixed by the standard
    /// rather than by the waterfall cursor.
    fn set_audio_hz(&mut self, _hz: f32) {}

    fn audio_hz(&self) -> f32 {
        match self.ch.baud() {
            // Centred on the carrier: there is no audio offset to report.
            _ if self.mode == Mode::Packet => 0.0,
            PacketBaud::Hf300 => AfskProfile::Hf300.centre_hz() as f32,
            _ => AfskProfile::Vhf1200.centre_hz() as f32,
        }
    }

    fn packet_beacon_now(&mut self) {
        self.queue_beacon();
    }

    fn packet_send_frame(&mut self, frame: Vec<u8>) {
        self.send_from_host(frame);
    }

    fn packet_take_air_frames(&mut self) -> Vec<Vec<u8>> {
        self.take_air_frames()
    }

    fn packet_connect(&mut self, call: String, via: String, ext: bool) {
        self.term_connect(&call, &via, ext);
    }

    fn packet_send_line(&mut self, text: String) {
        self.term_send(text);
    }

    fn packet_disconnect(&mut self) {
        self.term_disconnect();
    }

    fn packet_term_clear(&mut self) {
        self.term_clear();
    }

    /// Empty the monitor. The bad-frame counter goes with it — it is the
    /// monitor's own tally of what it could not read, and a count left over
    /// from a cleared page describes traffic that is no longer on it. The link
    /// is untouched: a connected station stays connected.
    fn clear_rx(&mut self) {
        self.heard.clear();
        self.bad_frames = 0;
        self.status_dirty = true;
    }

    fn status(&self) -> DigiStatus {
        self.digi_status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rule 1, pinned from the first commit rather than after the first
    /// mysterious self-acknowledgement on the air. The moment a transmitter
    /// lands behind this gate the bug it prevents is silent.
    ///
    /// The gate itself lives in [`Ax25Channel`] and is tested there; this is
    /// the controller half — that nothing reaches the monitor either.
    #[test]
    fn a_keyed_station_does_not_listen_to_itself() {
        let mut c = PacketController::new(Mode::Packet, DigiConfig::default(), 48_000.0);
        c.ch.keyed = true;
        c.on_rx_audio(&[0.5; 480]);
        assert!(c.heard.is_empty(), "a keyed station decoded something");
        assert!(!c.ch.dcd, "a keyed station must not read its own signal as a busy channel");
    }

    /// An idle packet station holds the channel for nobody.
    #[test]
    fn an_empty_over_ends_immediately() {
        let mut c = PacketController::new(Mode::Packet, DigiConfig::default(), 48_000.0);
        let mut block = [1.0f32; 480];
        assert!(c.fill_tx_block(&mut block), "an over with nothing to send must end");
        assert!(block.iter().all(|s| *s == 0.0), "silence, not stale audio");
    }

    /// The AX.25 modem is built at the tap rate, so this is one of the two
    /// controllers whose transmit audio is not 48 kHz — and the engine
    /// rate-matches from whatever this says. Answering 48 kHz here would send a
    /// burst at twice its baud rate on a radio that receives at 24 (issue
    /// #150); the FT8 side of the same seam is issue #359.
    #[test]
    fn the_transmit_rate_follows_the_tap() {
        use crate::DigiEngine;

        for tap in [24_000.0, 48_000.0] {
            let c = PacketController::new(Mode::Packet, DigiConfig::default(), tap);
            assert_eq!(DigiEngine::tx_rate(&c), tap);
        }
    }

    #[test]
    fn each_packet_mode_reports_its_own_name() {
        for mode in [Mode::Packet, Mode::PacketHf] {
            let c = PacketController::new(mode, DigiConfig::default(), 48_000.0);
            assert_eq!(c.mode(), mode);
            assert_eq!(c.status().mode, mode);
        }
    }

    /// HF runs at one speed. A `Vhf9600` left in the config from a VHF session
    /// must not follow the operator to 40 metres and produce a receiver that
    /// decodes nothing without ever saying why.
    #[test]
    fn hf_ignores_a_vhf_speed_left_in_the_config() {
        let cfg = DigiConfig { packet_baud: PacketBaud::Vhf9600, ..Default::default() };
        let c = PacketController::new(Mode::PacketHf, cfg, 48_000.0);
        assert_eq!(c.ch.baud(), PacketBaud::Hf300);
        assert_eq!(c.status().packet.unwrap().baud, PacketBaud::Hf300);
    }

    /// ...and the reverse: HF's speed is meaningless on VHF.
    #[test]
    fn vhf_falls_back_when_the_config_says_hf() {
        let cfg = DigiConfig { packet_baud: PacketBaud::Hf300, ..Default::default() };
        let c = PacketController::new(Mode::Packet, cfg, 48_000.0);
        assert_eq!(c.ch.baud(), PacketBaud::Vhf1200);
    }

    /// A station with no callsign must not transmit. Unidentified transmission
    /// is illegal everywhere, and an empty `packet_mycall` is the state the
    /// config ships in — so this is the default path, not an edge case.
    #[test]
    fn a_station_with_no_callsign_never_beacons() {
        let cfg = DigiConfig {
            packet_beacon_text: "sdroxide test".into(),
            packet_beacon_minutes: 1,
            packet_mycall: String::new(),
            ..Default::default()
        };
        let mut c = PacketController::new(Mode::Packet, cfg, 48_000.0);
        c.queue_beacon();
        assert_eq!(c.ch.queued(), 0, "beaconed without a callsign");
    }

    /// ...and with one, the beacon is a real frame addressed from that call.
    #[test]
    fn a_beacon_carries_the_operators_call_and_text() {
        let cfg = DigiConfig {
            packet_beacon_text: "sdroxide test".into(),
            packet_beacon_minutes: 1,
            packet_mycall: "OE3JJS-10".into(),
            packet_persist: 255,
            packet_slottime_ms: 1,
            ..Default::default()
        };
        let mut c = PacketController::new(Mode::Packet, cfg.clone(), 48_000.0);
        c.queue_beacon();
        // Out through the channel the way it actually goes, rather than by
        // reading a private queue: a clear channel, a few blocks of audio, and
        // the over the CSMA lets through.
        for _ in 0..4 {
            c.ch.on_rx_audio(&[0.0; 480], &cfg);
        }
        let sent = c.ch.take_over(&cfg).expect("no beacon queued");
        let p = Packet::parse(&sent[0], None).expect("beacon does not parse");
        assert_eq!(p.src().call(), "OE3JJS-10");
        assert_eq!(p.dst().call(), "BEACON");
        match p.packet_type() {
            PacketType::Ui(ui) => assert_eq!(ui.payload, b"sdroxide test"),
            other => panic!("a beacon must be a UI frame, got {other:?}"),
        }
    }

    /// Changing speed rebuilds the modem. Without this the operator picks 9600,
    /// nothing decodes, and there is no clue as to why.
    #[test]
    fn a_speed_change_rebuilds_the_modem() {
        let mut c = PacketController::new(Mode::Packet, DigiConfig::default(), 48_000.0);
        assert_eq!(c.ch.baud(), PacketBaud::Vhf1200);
        c.set_config(DigiConfig { packet_baud: PacketBaud::Vhf9600, ..Default::default() });
        assert_eq!(c.ch.baud(), PacketBaud::Vhf9600);
    }
}
