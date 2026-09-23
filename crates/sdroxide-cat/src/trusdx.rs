//! (tr)uSDX CAT — DL2MAN/PE1NNZ's pocket QRP transceiver and the open uSDX
//! firmware it grew from. A fourth Kenwood dialect, with the audio *inside* the
//! control link.
//!
//! The radio emulates a Kenwood TS-480 and answers `ID;` with `020`, so on
//! paper a Kenwood profile would drive it. On the wire the subset is thin
//! enough that it would also spend most of its frames being refused: the
//! firmware answers `?;` to `SM`, `RM`, `SL`, `SH`, `PC`, `AG` (the bare read),
//! `FB`, `FR`, `FT`, `RA`, `SQ` and the rest, because it has no S-meter, no
//! SWR, no power control, no VFO B, no split and no keyer. What it *does*
//! answer is:
//!
//! | Command | Meaning |
//! |---------|---------|
//! | `FA;` / `FAnnnnnnnnnnn;` | read / set the dial |
//! | `MD;` / `MDn;` | read / set the mode, 1..5 = LSB USB CW FM AM |
//! | `IF;` | status: the dial, and the transmit flag |
//! | `ID;` | `ID020;` |
//! | `PS;` | power on/off state |
//! | `TX;` `TX0;` `TX1;` `TX2;` | key, unkey, key, tune |
//! | `RX;` | unkey |
//! | `AG0;` `FL0;` `RS;` `AI;` `VX;` `RC;` `RT1;` `XT1;` | the handful of |
//! |   | settings the firmware kept |
//!
//! Every one of those was confirmed against a bench radio rather than read off
//! the manual — see the module's own history. The manual's own list is a
//! *superset*: it names commands the firmware's parser does not reach.
//!
//! # The audio is on the same wire
//!
//! This is what makes it a family and not a Kenwood with a note attached. A
//! (tr)uSDX has **no sound card at all**. Receive audio and transmit audio are
//! 8-bit streams carried inside the CAT serial link, switched on with the
//! firmware's own `UA` extension:
//!
//! * `UA1;` turns streaming on with the speaker, `UA2;` with the speaker off,
//!   `UA0;` off. The reply is `UA1;` immediately followed by `US`.
//! * `US` is **not** terminated. Receive audio runs until a CAT command
//!   arrives, at which point the firmware writes a `;`, the reply, and then
//!   `US` again to resume. The host therefore sees
//!   `[audio] ; FA00014031000; US [audio]`, and has to tell the three apart.
//! * The receive rate is ~7812 samples/s (the firmware divides a timer; the
//!   measured figure on the bench was 7812.3, and the published ones — 7812,
//!   7820, 7825 — disagree). Transmit is 11520 samples/s, and the *host*
//!   paces it: the radio does not clock the bytes back.
//! * The firmware escapes a `0x3B` sample byte to `0x3C` on receive, because a
//!   bare `;` would end the stream. The host must do the same on transmit.
//!
//! # Two things about the hardware that the driver has to work around
//!
//! * **Opening the port may reset the radio.** On the common CH340 board the
//!   serial adapter's DTR is wired to the ATmega's reset, and the radio comes
//!   up announcing itself with an unsolicited `IF…;` a moment later. Toggling
//!   DTR resets it again. So DTR is held high for the whole session and never
//!   offered as a keying line — a PTT on DTR would reboot the radio on every
//!   over.
//! * **A CAT command written while the stream is running kills it.** Not
//!   pauses it — kills it. Measured on the bench: a single `FA;` mid-stream
//!   took the rate from ~6 kB/s to zero, and re-sending `UA1;` over the dead
//!   stream made it worse. The stream only comes back when it is stopped
//!   (`UA0;`) and started again. So this profile polls nothing at all
//!   ([`TrUsdx::poll_requests`] is empty), and every control frame is
//!   bracketed `UA0;` … `UA1;` by the serial thread. The radio's own dial and
//!   mode are therefore not followed, and each command costs a brief gap.
//!
//! Written against a bench radio running DL2MAN firmware 2.x, and against the
//! open uSDX firmware (`threeme3/usdx`) for the framing the two share. The
//! transmit-audio path is DL2MAN's and is not in the open firmware, so the
//! transmit rate and the escape are taken from the firmware's own documentation
//! and confirmed only as far as a dummy load allows.
//!
//! # nG, the second generation
//!
//! The same profile also drives DL2MAN's rewritten **nG** firmware
//! ([`TrUsdx::new_ng`], chosen by [`sdroxide_types::CatFamily::TrUsdxNg`]). The
//! receive side and the `UA`/`US` framing are unchanged — 7812.5 B/s, the same
//! receive escape — so the demultiplexer below is the same code. Three things
//! on the transmit side differ, and each is a silent failure if got wrong:
//!
//! * the rate is **4807.69 B/s** (the transmit slot is `20 MHz / (64 × 65)`),
//!   not 2.00x's 11520 — see [`sdroxide_types::TRUSDX_NG_TX_RATE_HZ`];
//! * the delimiter escape is **`0x3B → 0x3A`**, where 2.00x shifts up to
//!   `0x3C` — see [`sdroxide_types::TRUSDX_NG_TX_ESCAPE_TO`];
//! * the stream opens on the first byte **≥ `0x80`**, so a leading `0x80`
//!   (silence) is emitted when the first real sample is low — see
//!   [`sdroxide_types::TRUSDX_NG_TX_START_BYTE`].
//!
//! nG also adds a level extension the profile uses: `AG0nn;` (volume 00–31)
//! and `GTn;` (gain control 0 off / 1 on / 2 DIGI), neither answered nor
//! stored, so they are re-asserted whenever the link opens; and it accepts
//! `UA2;` to switch the radio's own speaker off while it streams.
//!
//! The nG figures come from DL2MAN's own notes for app developers and are
//! **not** confirmed against an nG radio here — the firmware was not available
//! on this bench. They are unit-tested structurally, and the on-air checks are
//! named in the fork's notes.

use crate::{CatUpdate, Protocol};
use sdroxide_types::{
    Mode, TRUSDX_NG_TX_ESCAPE_TO, TRUSDX_NG_TX_RATE_HZ, TRUSDX_NG_TX_START_BYTE, TrUsdxNgAgc,
};
use tracing::{debug, info};

/// Digits in the `FA` frequency field, fixed across the family.
const FREQ_DIGITS: usize = 11;

/// The transmit flag's offset in an `IF;` reply body — the same positional
/// layout the TS-480 emulation copies.
const IF_TX_FLAG: usize = 26;
/// Length of that body, `IF` and `;` excluded.
const IF_BODY_LEN: usize = 35;

/// The nG-only level settings, carried into the profile so it can re-assert
/// them whenever it is asked to: nG answers neither `AG` nor `GT` and stores
/// neither, so the host is the only place they live.
#[derive(Debug, Clone, Copy)]
pub struct NgLevel {
    /// Volume, 0–31 (`AG0nn;`).
    pub volume: u8,
    /// Gain control (`GTn;`).
    pub agc: TrUsdxNgAgc,
    /// Whether the radio's own speaker stays on while streaming (`UA1;`) or is
    /// switched off (`UA2;`).
    pub speaker: bool,
}

pub struct TrUsdx {
    /// Whether the audio rides the CAT link (`TrUsdxAudio::OneCable`) rather
    /// than a sound card. Everything the profile does differently between the
    /// two modes hangs off this one flag: only the in-band mode streams, and
    /// only the in-band mode must send no polls.
    one_cable: bool,
    /// `Some` when driving nG firmware, with the level settings it re-asserts.
    /// `None` is the 2.00x/open-firmware generation; the receive side and the
    /// `UA`/`US` framing are the same for both, so this flag decides only the
    /// transmit rate, the transmit delimiter escape, the stream's opening byte
    /// and whether the `AG`/`GT` level commands are sent.
    ng: Option<NgLevel>,
    /// Bytes arrived and not yet split into whole CAT frames.
    buf: Vec<u8>,
    /// Whether the receive audio stream is running — true between the `US`
    /// that resumes it and the `;` that ends it. Determines how the bytes in
    /// [`Self::buf`] are read, which is the whole of the framing.
    in_audio: bool,
    /// Receive audio pulled out of the stream since it was last drained, as
    /// unsigned 8-bit samples. Held here rather than sent anywhere because the
    /// profile is called from the serial thread, which owns the ring the audio
    /// actually goes into (see `sdroxide_cat::CatHandle::poll_stream_audio`).
    audio: Vec<u8>,
    /// True once the radio has answered the enable with `US`, i.e. the stream
    /// is actually running. Until then the serial thread re-asks, because the
    /// reboot as the port opens can swallow the first `UA1;`.
    stream_on: bool,
    /// nG: a transmit stream has just started and its opening byte has not gone
    /// out yet, so the next encoded block must open on a byte ≥ `0x80`.
    tx_start_pending: bool,
    /// Mode digit from the radio's last `MD;` reply.
    mode_digit: Option<char>,
    /// True once `ID;` has answered `020`, so the model is logged once.
    identified: bool,
    /// The radio answered `?;` since this was last read.
    nak: bool,
}

impl TrUsdx {
    /// `one_cable` is [`sdroxide_types::TrUsdxAudio::OneCable`] — audio inside
    /// the CAT stream rather than over a sound card.
    pub fn new(one_cable: bool) -> Self {
        Self::with_ng(one_cable, None)
    }

    /// The nG generation: the same profile, with nG's transmit rate, delimiter
    /// escape and opening byte, and its `AG`/`GT` level commands.
    pub fn new_ng(one_cable: bool, level: NgLevel) -> Self {
        Self::with_ng(one_cable, Some(level))
    }

    fn with_ng(one_cable: bool, ng: Option<NgLevel>) -> Self {
        TrUsdx {
            one_cable,
            ng,
            buf: Vec::new(),
            in_audio: false,
            audio: Vec::new(),
            stream_on: false,
            tx_start_pending: false,
            mode_digit: None,
            identified: false,
            nak: false,
        }
    }

    fn is_ng(&self) -> bool {
        self.ng.is_some()
    }

    /// The stream's enable for this generation and speaker choice: `UA1;` with
    /// the radio's speaker on, `UA2;` with it off (nG only — 2.00x has no
    /// second form).
    fn stream_enable(&self) -> &'static [u8] {
        if self.ng.is_some_and(|n| !n.speaker) { b"UA2;" } else { b"UA1;" }
    }

    /// The `AG0nn;` and `GTn;` frames for the current level settings, empty for
    /// the 2.00x generation. `data_mode` resolves [`TrUsdxNgAgc::Auto`].
    fn level_frames(&self, data_mode: bool) -> Vec<Vec<u8>> {
        let Some(ng) = self.ng else { return Vec::new() };
        vec![
            format!("AG0{:02};", ng.volume.min(31)).into_bytes(),
            format!("GT{};", ng.agc.digit(data_mode)).into_bytes(),
        ]
    }

    /// The app's mode for the radio's mode digit.
    ///
    /// `MD1..5` are LSB, USB, CW, FM, AM — the TS-480's own order, and the one
    /// the firmware's `MD` handler emits (`mode + 1`). There is no DATA flag
    /// and no CW-R, so this is the inverse of [`mode_digit`] over the whole
    /// range.
    fn app_mode(&self) -> Option<Mode> {
        Some(match self.mode_digit? {
            '1' => Mode::Lsb,
            '2' => Mode::Usb,
            '3' => Mode::Cw,
            '4' => Mode::Nfm,
            '5' => Mode::Am,
            _ => return None,
        })
    }
}

impl Default for TrUsdx {
    fn default() -> Self {
        Self::new(true)
    }
}

/// The radio's mode digit for an app mode. There is no DATA position: a digital
/// mode rides a plain sideband and the audio does the rest, exactly as it does
/// on a rig with no DATA switch.
fn mode_digit(m: Mode) -> char {
    match m {
        // A digital mode rides the sideband its name carries, the same way
        // every other family maps it: DIGL is LSB with the audio doing the
        // rest, and sending it to USB would put the over on the wrong
        // sideband.
        Mode::Lsb | Mode::Digl => '1',
        Mode::Cw => '3',
        Mode::Am | Mode::Sam | Mode::Dsb | Mode::Isb | Mode::Drm | Mode::Acars => '5',
        // No (tr)uSDX is a general-coverage FM set, but it has the position and
        // the firmware answers it, so the modes sdroxide maps to FM go there.
        Mode::Nfm
        | Mode::Wfm
        | Mode::Rifp
        | Mode::Packet
        | Mode::Aprs
        | Mode::SstvFm
        | Mode::RttyFm
        | Mode::Adsb
        | Mode::Vdl2
        | Mode::Ais
        | Mode::Hfdl
        | Mode::HdRadio => '4',
        Mode::Usb
        | Mode::Spec
        | Mode::Sstv
        | Mode::Wefax
        | Mode::Navtex
        | Mode::RfPaint
        | Mode::Digu
        | Mode::Ft8
        | Mode::Js8
        | Mode::Wspr
        | Mode::Pi4
        | Mode::Ft4
        | Mode::Ft2
        | Mode::Psk
        | Mode::Rtty
        | Mode::Olivia
        | Mode::Thor
        | Mode::Fsq
        | Mode::Hell
        | Mode::PacketHf
        | Mode::AtChat
        | Mode::Rade => '2',
    }
}

/// Whether the radio is transmitting, from the body of an `IF;` reply.
///
/// Positional, exactly as [`crate::kenwood`] reads it and for the same reason:
/// the layout is the TS-480's. The length is checked rather than indexed into
/// blindly, so a reply of another shape reports nothing at all.
fn if_transmitting(body: &str) -> Option<bool> {
    if body.len() != IF_BODY_LEN {
        return None;
    }
    match body.as_bytes()[IF_TX_FLAG] {
        b'0' => Some(false),
        b'1' => Some(true),
        _ => None,
    }
}

/// Escape a transmit-audio byte for the stream: a `0x3B` would be read as a CAT
/// terminator, so it must not go out as one. The two generations disagree on
/// the substitute — 2.00x shifts it up to `0x3C` (the same value it sends in
/// place of a delimiter on receive), nG shifts it down to `0x3A`. Getting this
/// wrong corrupts one sample in 256, and on nG a wrong substitute of `0x3C` is
/// harmless only by luck (it is a legal sample); a `0x3B` left unmapped ends
/// the stream early.
fn escape_tx(sample: u8, ng: bool) -> u8 {
    if sample == b';' { if ng { TRUSDX_NG_TX_ESCAPE_TO } else { b';' + 1 } } else { sample }
}

/// A mono `-1.0..=1.0` sample as the radio's unsigned 8-bit value (mid = 128).
fn sample_byte(s: f32) -> u8 {
    ((s.clamp(-1.0, 1.0) * 127.0) as i32 + 128).clamp(0, 255) as u8
}

impl Protocol for TrUsdx {
    fn set_freq(&mut self, hz: f64) -> Vec<u8> {
        let hz = hz.round().clamp(0.0, 99_999_999_999.0) as u64;
        format!("FA{hz:0FREQ_DIGITS$};").into_bytes()
    }

    fn set_mode(&mut self, m: Mode) -> Vec<u8> {
        let mut out = format!("MD{};", mode_digit(m)).into_bytes();
        // nG's gain control follows the mode when it is `Auto`, and `GT` is not
        // stored, so a mode change is the moment to re-assert it — appended to
        // the mode frame, which the radio parses as two commands back to back.
        if let Some(ng) = self.ng
            && ng.agc.is_auto()
        {
            out.extend_from_slice(format!("GT{};", ng.agc.digit(m.is_digital())).as_bytes());
        }
        out
    }

    fn ptt(&self, on: bool) -> Vec<u8> {
        // `TX;` keys and `RX;` unkeys. Not `TX0;` — the manual lists it as a
        // *transmit* form, and the firmware's own `TX` handler treats the bare
        // command as the toggle; `RX;` is the documented unkey either way.
        if on { b"TX;".to_vec() } else { b"RX;".to_vec() }
    }

    /// The dial and the mode — but only when the audio is on a sound card.
    ///
    /// In the in-band mode (`TrUsdxAudio::OneCable`) this is **nothing**, and
    /// that is the heart of how that mode is driven. The firmware cannot take a
    /// CAT command while its audio stream is running: writing one *into* the
    /// stream does not pause it, it kills it, and the stream does not come back
    /// — measured on the bench, a single `FA;` mid-stream took the rate from
    /// ~6 kB/s to zero and the radio stayed silent until it was stopped and
    /// re-enabled by hand. So a poll there, which is a CAT command every
    /// half-second for the whole session, would leave a radio that streams for a
    /// moment and then never again. Control frames only go out when the operator
    /// asks for something, each bracketed with the stream's pause and resume
    /// (see [`Protocol::stream_pause`]), and the cost is that the radio's own
    /// knob and mode are not followed.
    ///
    /// With the audio on a sound card there is no stream to protect, so the poll
    /// is an ordinary one and the rig's own controls are followed like any other
    /// CAT rig's.
    fn poll_requests(&self) -> Vec<Vec<u8>> {
        if self.one_cable { Vec::new() } else { vec![b"FA;".to_vec(), b"MD;".to_vec()] }
    }

    fn dial_requests(&self) -> Vec<Vec<u8>> {
        if self.one_cable { Vec::new() } else { vec![b"FA;".to_vec()] }
    }

    /// The transmit-state read, sound-card mode only: `IF;` mid-stream is the
    /// same poison the poll is.
    fn tx_state_requests(&self) -> Vec<Vec<u8>> {
        if self.one_cable { Vec::new() } else { vec![b"IF;".to_vec()] }
    }

    /// Suspend the stream before a control command is written. The firmware
    /// only accepts a CAT frame with the stream stopped, and only resumes when
    /// it is asked to.
    fn stream_pause(&self) -> Vec<u8> {
        b"UA0;".to_vec()
    }

    fn stream_resume(&self) -> Vec<u8> {
        self.stream_enable().to_vec()
    }

    /// Stop any auto-information the radio may have been left in, and switch off
    /// the in-band audio stream in case a previous session — or one of the
    /// community streaming drivers — left it running, which would pour audio
    /// bytes into a parser expecting `;`-framed replies.
    ///
    /// In the in-band mode the *enable* is deliberately not here: the opening
    /// sequence has other frames after this (`clear_offsets`'s `RC;`), and a CAT
    /// command written once the stream is running kills it, so the enable goes
    /// out last from [`Self::stream_start`] on the serial thread's retry, by
    /// which time no other frame is owed.
    fn open_requests(&self) -> Vec<Vec<u8>> {
        let mut out = vec![b"AI0;".to_vec(), b"UA0;".to_vec()];
        // nG's level commands go out here, while the stream is stopped: it
        // answers neither and stores neither, so the link opening is the one
        // moment they can be set. `data_mode` is false at open — the rig's own
        // mode is adopted, not commanded, so `Auto` starts at ON and corrects
        // on the first mode frame we send.
        out.extend(self.level_frames(false));
        out
    }

    /// Clear the clarifier and switch RIT/XIT off. sdroxide carries RIT on the
    /// dial — the radio's dial is the only frequency control a CAT rig gives us
    /// — so an offset the radio is still holding would add to ours unseen.
    /// `RC;` clears the offset; the `RT0;`/`XT0;` that would switch it off are
    /// not in this firmware, so only the clear is sent.
    fn clear_offsets(&self) -> Vec<Vec<u8>> {
        vec![b"RC;".to_vec()]
    }

    fn refused(&mut self) -> bool {
        std::mem::take(&mut self.nak)
    }

    /// A fresh link means a rebooted radio (the reset is on the port open), so
    /// the stream is off again whatever it was before and has to be re-asked.
    fn link_opened(&mut self) {
        self.buf.clear();
        self.in_audio = false;
        self.stream_on = false;
    }

    /// DTR is the reset line on the common board — see the module comment.
    fn holds_dtr_high(&self) -> bool {
        true
    }

    /// Whether this profile carries audio inside the byte stream, so the serial
    /// thread routes bytes to [`Self::take_stream_audio`] rather than to a sound
    /// card. True only in the in-band mode.
    fn streams_audio(&self) -> bool {
        self.one_cable
    }

    /// Stop the stream and start it again — deliberately, and never `UA1;`
    /// alone.
    ///
    /// Re-asserting `UA1;` over a stream that is already running is the exact
    /// shape that kills this firmware: the stream starts, the duplicate enable
    /// arrives, and the radio goes silent for good. Stopping first makes the
    /// call idempotent whatever state the radio is in — safe on a stream that
    /// is running, on one that never started, and on one that has died.
    fn stream_start(&self) -> Vec<u8> {
        let mut out = b"UA0;".to_vec();
        out.extend_from_slice(self.stream_enable());
        out
    }

    fn stream_active(&self) -> bool {
        self.stream_on
    }

    /// The receive audio pulled out of the stream since the last call.
    fn take_stream_audio(&mut self, out: &mut Vec<u8>) {
        out.append(&mut self.audio);
    }

    /// Encode a block of transmit audio — mono, `-1.0..=1.0` — as the unsigned
    /// 8-bit stream the radio takes, escaping the delimiter byte.
    ///
    /// On the first block of an over this also opens an nG stream correctly: nG
    /// takes the first byte with the top bit set as the first sample and reads
    /// everything before it as commands, so if the block's own first sample is
    /// low a single `0x80` (the stream's silence) goes out ahead of it.
    fn encode_tx_audio(&mut self, samples: &[f32], out: &mut Vec<u8>) {
        if samples.is_empty() {
            return;
        }
        out.reserve(samples.len() + 1);
        if std::mem::take(&mut self.tx_start_pending) && self.is_ng() {
            let first = escape_tx(sample_byte(samples[0]), true);
            if first < TRUSDX_NG_TX_START_BYTE {
                out.push(TRUSDX_NG_TX_START_BYTE);
            }
        }
        for &s in samples {
            out.push(escape_tx(sample_byte(s), self.is_ng()));
        }
    }

    /// nG reads transmit audio at its own rate; 2.00x's is the other one. The
    /// serial thread paces the stream by this, so a family that returns nG's
    /// rate feeds the radio at the rate it actually reads.
    fn tx_audio_rate_hz(&self) -> u32 {
        if self.is_ng() { TRUSDX_NG_TX_RATE_HZ } else { sdroxide_types::TRUSDX_TX_RATE_HZ }
    }

    /// A key-down starts a fresh transmit stream, so the opening byte is owed
    /// again on the next block.
    fn on_tx_stream_start(&mut self) {
        if self.is_ng() {
            self.tx_start_pending = true;
        }
    }

    fn parse(&mut self, buf: &mut Vec<u8>) -> Vec<CatUpdate> {
        self.buf.append(buf);
        let mut out = Vec::new();
        // The stream is a byte soup, not lines: audio runs until a `;`, a CAT
        // reply sits between the `;` that ends it and the `;` that ends *it*,
        // and `US` resumes the audio. Which of those a `;` is depends on the
        // state, so the buffer is walked rather than split.
        loop {
            if self.in_audio {
                // Running audio. A `;` is the firmware pausing the stream to
                // answer a command; everything before it is samples. With no
                // `;` yet, everything buffered is samples and the rest waits.
                match self.buf.iter().position(|&b| b == b';') {
                    Some(pos) => {
                        for &b in &self.buf[..pos] {
                            self.audio.push(unescape(b));
                        }
                        self.buf.drain(..=pos);
                        self.in_audio = false;
                    }
                    None => {
                        for b in self.buf.drain(..) {
                            self.audio.push(unescape(b));
                        }
                        break;
                    }
                }
            } else {
                // Between frames. `US` puts the stream back into audio; a
                // complete `…;` frame is parsed; anything short of either waits.
                if self.buf.starts_with(b"US") {
                    self.buf.drain(..2);
                    self.in_audio = true;
                    self.stream_on = true;
                    continue;
                }
                match self.buf.iter().position(|&b| b == b';') {
                    Some(end) => {
                        let frame = String::from_utf8_lossy(&self.buf[..end]).into_owned();
                        self.buf.drain(..=end);
                        out.extend(self.parse_frame(&frame));
                    }
                    None => break,
                }
            }
        }
        out
    }
}

/// Undo the firmware's receive escape: it sends a sample of `0x3B` as `0x3C`
/// so a bare `;` cannot end the stream, and the host turns it back.
fn unescape(b: u8) -> u8 {
    if b == b';' + 1 { b';' } else { b }
}

impl TrUsdx {
    /// One `;`-delimited CAT frame, without its delimiters.
    fn parse_frame(&mut self, msg: &str) -> Vec<CatUpdate> {
        let msg = msg.trim();
        let mut out = Vec::new();
        if let Some(rest) = msg.strip_prefix("FA") {
            if rest.len() == FREQ_DIGITS
                && let Ok(hz) = rest.parse::<u64>()
            {
                out.push(CatUpdate::Freq(hz as f64));
            }
        } else if let Some(rest) = msg.strip_prefix("MD") {
            if let Some(d) = rest.chars().next() {
                self.mode_digit = Some(d);
                if let Some(m) = self.app_mode() {
                    out.push(CatUpdate::Mode(m));
                }
            }
        } else if let Some(rest) = msg.strip_prefix("IF") {
            if let Some(on) = if_transmitting(rest) {
                out.push(CatUpdate::Ptt(on));
            }
        } else if let Some(rest) = msg.strip_prefix("ID") {
            if !self.identified && rest.trim() == "020" {
                self.identified = true;
                info!("(tr)uSDX CAT: radio identified (TS-480 emulation)");
            }
        } else if msg == "UA0" {
            // The stream has been stopped; it is no longer running whatever it
            // was, so the retry below may re-arm it.
            self.stream_on = false;
        } else if msg == "?" {
            self.nak = true;
            debug!("(tr)uSDX CAT: radio rejected a command (?)");
        } else if msg == "E" || msg == "O" {
            debug!("(tr)uSDX CAT: serial error from radio ({msg})");
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(p: &mut TrUsdx, s: &[u8]) -> Vec<CatUpdate> {
        let mut b = s.to_vec();
        p.parse(&mut b)
    }

    #[test]
    fn the_frames_are_the_documented_shape() {
        let mut p = TrUsdx::new(true);
        assert_eq!(p.set_freq(14_031_000.0), b"FA00014031000;".to_vec());
        assert_eq!(p.set_mode(Mode::Usb), b"MD2;".to_vec());
        assert_eq!(p.set_mode(Mode::Cw), b"MD3;".to_vec());
        assert_eq!(p.ptt(true), b"TX;".to_vec());
        assert_eq!(p.ptt(false), b"RX;".to_vec());
        // Never `TX0;` — that is a transmit form on this family.
        assert_ne!(p.ptt(true), b"TX0;".to_vec());
    }

    /// A digital mode goes to the sideband its name carries. There is no DATA
    /// position on this firmware, so DIGL is plain LSB — not USB, which would
    /// put the over on the wrong sideband.
    #[test]
    fn a_digital_mode_keeps_its_own_sideband() {
        let mut p = TrUsdx::new(true);
        assert_eq!(p.set_mode(Mode::Digl), b"MD1;".to_vec());
        assert_eq!(p.set_mode(Mode::Lsb), b"MD1;".to_vec());
        assert_eq!(p.set_mode(Mode::Digu), b"MD2;".to_vec());
        assert_eq!(p.set_mode(Mode::Ft8), b"MD2;".to_vec());
    }

    #[test]
    fn only_the_commands_the_firmware_answers_are_claimed() {
        let mut p = TrUsdx::new(true);
        // No meters, no power, no filter, no squelch, no keyer.
        assert!(p.tx_telemetry_requests().is_empty());
        assert!(p.rx_telemetry_requests().is_empty());
        assert!(p.set_power(1.0).is_empty());
        assert!(!p.commands_power());
        assert!(p.set_filter(Mode::Usb, 300.0, 2700.0).is_empty());
        assert!(!p.commands_filter());
        assert!(p.set_squelch(0.5).is_empty());
        assert!(!p.commands_squelch());
        assert_eq!(p.cw_chunk_len(), 0);
        assert!(!p.commands_rig_power());
    }

    /// The stream is a family of its own, and its enable is deliberately *not*
    /// among the frames the opening sequence writes: those are followed by
    /// `clear_offsets`, and a CAT frame written once the stream is running kills
    /// it. The enable is the serial thread's retry instead, sent last and only
    /// while the stream is idle.
    #[test]
    fn the_stream_is_enabled_after_the_opening_frames_not_during_them() {
        let p = TrUsdx::new(true);
        let open: Vec<String> =
            p.open_requests().iter().map(|f| String::from_utf8_lossy(f).into_owned()).collect();
        assert!(!open.iter().any(|f| f.contains("UA1")), "the enable must not open: {open:?}");
        assert!(p.streams_audio());
        // The retry stops and starts, so it is safe over a live stream too.
        assert_eq!(p.stream_start(), b"UA0;UA1;".to_vec());
        assert_eq!(p.stream_pause(), b"UA0;".to_vec());
        assert_eq!(p.stream_resume(), b"UA1;".to_vec());
        // The in-band mode asks for nothing of its own — a poll would kill the
        // stream — and it asserts `UA0;` at open all the same, to clear any
        // stream a previous session left running.
        assert!(p.poll_requests().is_empty());
        assert!(p.dial_requests().is_empty());
        assert!(p.tx_state_requests().is_empty());
    }

    /// The other half of the choice: with the audio on a sound card there is no
    /// stream to protect, so the profile polls like any other CAT rig and the
    /// serial thread sets up no in-band rings at all.
    #[test]
    fn sound_card_mode_polls_and_does_not_stream() {
        let p = TrUsdx::new(false);
        assert!(!p.streams_audio());
        assert_eq!(p.poll_requests(), vec![b"FA;".to_vec(), b"MD;".to_vec()]);
        assert_eq!(p.dial_requests(), vec![b"FA;".to_vec()]);
        assert_eq!(p.tx_state_requests(), vec![b"IF;".to_vec()]);
        // Both modes still switch any leftover stream off at open.
        let open: Vec<String> =
            p.open_requests().iter().map(|f| String::from_utf8_lossy(f).into_owned()).collect();
        assert!(open.contains(&"UA0;".to_string()), "{open:?}");
    }

    #[test]
    fn a_frequency_and_mode_reply_are_read() {
        let mut p = TrUsdx::new(true);
        assert_eq!(feed(&mut p, b";FA00014031000;"), vec![CatUpdate::Freq(14_031_000.0)]);
        assert_eq!(feed(&mut p, b";MD3;"), vec![CatUpdate::Mode(Mode::Cw)]);
    }

    /// The transmit flag, out of the bench radio's own `IF;` reply.
    #[test]
    fn the_transmit_flag_comes_out_of_if_by_position() {
        let mut p = TrUsdx::new(true);
        assert_eq!(
            feed(&mut p, b";IF0001403100000000+000000000030000000;"),
            vec![CatUpdate::Ptt(false)]
        );
        // A keyed body: the flag at offset 26 set to 1.
        let mut body = b"0001403100000000+000000000030000000".to_vec();
        body[IF_TX_FLAG] = b'1';
        let mut frame = b";IF".to_vec();
        frame.extend_from_slice(&body);
        frame.push(b';');
        assert_eq!(feed(&mut p, &frame), vec![CatUpdate::Ptt(true)]);
    }

    /// The whole point of the framing: audio and CAT replies share the stream.
    ///
    /// The bench radio's own interleave is `[audio] ; FA00014031000; US
    /// [audio]`, so the parser has to hand the frequency up *and* keep the
    /// audio on both sides of it — without the `US` or the frame ending up in
    /// the audio.
    #[test]
    fn audio_and_a_cat_reply_are_told_apart_in_one_stream() {
        let mut p = TrUsdx::new(true);
        // The stream always starts with the `US` that `UA1;` is answered with.
        let mut stream = b"US".to_vec();
        stream.extend_from_slice(&[0x80, 0x81, 0x82]);
        stream.extend_from_slice(b";FA00014031000;US");
        stream.extend_from_slice(&[0x83, 0x84]);
        let out = feed(&mut p, &stream);
        assert_eq!(out, vec![CatUpdate::Freq(14_031_000.0)]);

        let mut audio = Vec::new();
        p.take_stream_audio(&mut audio);
        assert_eq!(audio, vec![0x80, 0x81, 0x82, 0x83, 0x84], "audio must survive the reply");
    }

    /// The firmware escapes `0x3B` to `0x3C` on receive, so a `0x3C` in the
    /// stream is a sample of 59 and not a stray byte.
    #[test]
    fn the_receive_escape_is_undone() {
        let mut p = TrUsdx::new(true);
        feed(&mut p, b"US");
        feed(&mut p, &[0x3C, 0x40, 0x3C]);
        let mut audio = Vec::new();
        p.take_stream_audio(&mut audio);
        assert_eq!(audio, vec![0x3B, 0x40, 0x3B]);
    }

    /// And the same substitution the other way on transmit.
    #[test]
    fn the_transmit_escape_is_applied() {
        let mut p = TrUsdx::new(true);
        // A sample that lands on 0x3B (59) must go out as 0x3C.
        let mut out = Vec::new();
        p.encode_tx_audio(&[(59.0 - 128.0) / 127.0], &mut out);
        assert_eq!(out, vec![0x3C]);
    }

    /// A frame split across two reads is still one frame, and the audio before
    /// it is not lost.
    #[test]
    fn a_reply_split_across_reads_is_still_one_reply() {
        let mut p = TrUsdx::new(true);
        feed(&mut p, b"US");
        assert!(feed(&mut p, &[0x80, 0x81]).is_empty());
        assert!(feed(&mut p, b";FA0001403").is_empty());
        assert_eq!(feed(&mut p, b"1000;"), vec![CatUpdate::Freq(14_031_000.0)]);
        let mut audio = Vec::new();
        p.take_stream_audio(&mut audio);
        assert_eq!(audio, vec![0x80, 0x81]);
    }

    // ---- nG: the generation that differs on the transmit side ----

    fn ng(volume: u8, agc: TrUsdxNgAgc, speaker: bool) -> TrUsdx {
        TrUsdx::new_ng(true, NgLevel { volume, agc, speaker })
    }

    /// nG reads transmit audio at its own rate, and the serial thread paces by
    /// it — so the profile must report the nG figure, not 2.00x's.
    #[test]
    fn ng_paces_transmit_at_the_ng_rate() {
        assert_eq!(TrUsdx::new(true).tx_audio_rate_hz(), 11_520, "2.00x is unchanged");
        assert_eq!(ng(20, TrUsdxNgAgc::Auto, true).tx_audio_rate_hz(), 4_808);
    }

    /// The two generations disagree on which way to shift the forbidden `;`
    /// sample on transmit: 2.00x up to `0x3C`, nG down to `0x3A`.
    #[test]
    fn ng_escapes_the_delimiter_down_where_legacy_escapes_up() {
        let low = [(59.0 - 128.0) / 127.0];
        let mut legacy = Vec::new();
        let mut legacy_p = TrUsdx::new(true);
        legacy_p.encode_tx_audio(&low, &mut legacy);
        assert_eq!(legacy, vec![0x3C]);
        let mut out = Vec::new();
        ng(20, TrUsdxNgAgc::Auto, true).encode_tx_audio(&low, &mut out);
        assert_eq!(out, vec![0x3A]);
    }

    /// nG starts its transmit stream on the first byte with the top bit set and
    /// reads everything before it as a command, so a block whose first sample
    /// is low must open with `0x80` — once, on the first block after key-down.
    #[test]
    fn an_ng_stream_opens_on_a_high_byte() {
        let mut p = ng(20, TrUsdxNgAgc::Auto, true);
        p.on_tx_stream_start();
        let mut out = Vec::new();
        p.encode_tx_audio(&[-1.0, 0.0], &mut out);
        assert_eq!(out[0], 0x80, "the stream must open on a byte >= 0x80: {out:?}");
        assert_eq!(out.len(), 3, "one leading silent sample, then the two real ones");
        // The next block is ordinary: no second opening byte.
        let mut more = Vec::new();
        p.encode_tx_audio(&[-1.0], &mut more);
        assert_eq!(more.len(), 1, "only the first block after key-down is opened");

        // A block that already opens high is left alone.
        let mut p = ng(20, TrUsdxNgAgc::Auto, true);
        p.on_tx_stream_start();
        let mut out = Vec::new();
        p.encode_tx_audio(&[0.5], &mut out);
        assert_eq!(out.len(), 1);
        assert!(out[0] >= 0x80);

        // 2.00x has no such rule and is never given a leading byte.
        let mut p = TrUsdx::new(true);
        p.on_tx_stream_start();
        let mut out = Vec::new();
        p.encode_tx_audio(&[-1.0], &mut out);
        assert_eq!(out, vec![1], "the sample alone, no opening byte");
    }

    /// The nG level commands are re-asserted at open (nG stores neither), and
    /// `Auto` follows the mode on the mode frame.
    #[test]
    fn ng_level_commands_go_out_at_open_and_follow_the_mode() {
        let p = ng(7, TrUsdxNgAgc::Auto, true);
        let open: Vec<String> =
            p.open_requests().iter().map(|f| String::from_utf8_lossy(f).into_owned()).collect();
        assert!(open.contains(&"AG007;".to_string()), "{open:?}");
        assert!(
            open.contains(&"GT1;".to_string()),
            "Auto is ON until a mode says otherwise: {open:?}"
        );

        // Auto: a digital mode asks for DIGI, an SSB mode for ON.
        let mut p = ng(7, TrUsdxNgAgc::Auto, true);
        assert_eq!(p.set_mode(Mode::Ft8), b"MD2;GT2;".to_vec());
        assert_eq!(p.set_mode(Mode::Usb), b"MD2;GT1;".to_vec());
        // A fixed setting never rides the mode frame.
        let mut p = ng(7, TrUsdxNgAgc::Digi, true);
        assert_eq!(p.set_mode(Mode::Usb), b"MD2;".to_vec());
        // 2.00x sends no level commands at all.
        let p = TrUsdx::new(true);
        assert!(
            p.open_requests().iter().all(|f| !f.starts_with(b"AG") && !f.starts_with(b"GT")),
            "2.00x has no AG/GT"
        );
    }

    /// `UA2;` switches the radio's own speaker off; 2.00x had only `UA1;`.
    #[test]
    fn ng_can_switch_the_radios_own_speaker_off() {
        let off = ng(20, TrUsdxNgAgc::Auto, false);
        assert_eq!(off.stream_start(), b"UA0;UA2;".to_vec());
        assert_eq!(off.stream_resume(), b"UA2;".to_vec());
        let on = ng(20, TrUsdxNgAgc::Auto, true);
        assert_eq!(on.stream_start(), b"UA0;UA1;".to_vec());
        assert_eq!(on.stream_resume(), b"UA1;".to_vec());
    }
}
