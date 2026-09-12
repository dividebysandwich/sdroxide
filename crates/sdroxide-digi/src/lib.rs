//! Digital-mode engines for sdroxide: the slotted FT8/FT4 and JS8 modes, the
//! keyboard modems, and the image and voice modes.
//!
//! **Licensing:** this crate links `mfsk-core` (GPL-3.0-or-later) and carries
//! the JS8 protocol tables transcribed from JS8Call (also GPL-3.0-or-later).
//! It is the only crate in the workspace that does either, and it is used only
//! in the native binary — the wasm remote client links none of it (all
//! decode/encode runs server-side). Keep it that way: `sdroxide-types` and
//! `sdroxide-dsp` are deliberately free of both.
//!
//! The CW panel additionally links `sdroxide-deepcw`, which is AGPL-3.0-only.
//! That crate is the only AGPL in the tree; see its manifest for what §13 means
//! for anything that links it.

pub mod aprs_controller;
pub mod atchat_controller;
pub(crate) mod ax25_channel;
pub mod clock;
pub mod controller;
pub mod cw_controller;
pub mod fox;
pub mod fsq_controller;
pub mod ft2;
pub mod ft8_eu;
pub mod hell_controller;
pub mod js8;
pub mod js8_controller;
pub mod modem;
pub mod navtex_controller;
mod packet_controller;
pub mod params;
pub mod qso;
pub mod rade_controller;
pub mod rf_paint_controller;
pub mod rifp_controller;
pub mod rifp_object;
pub mod scheduler;
pub mod squelch;
pub mod sstv_controller;
pub mod text_modem;
pub mod wefax_controller;
pub mod wspr;
pub mod wspr_controller;

pub use aprs_controller::AprsController;
pub use atchat_controller::AtChatController;
pub use clock::ClockMonitor;
pub use controller::{DigiAction, DigiController};
pub use cw_controller::CwController;
pub use fox::Fox;
pub use fsq_controller::FsqController;
pub use hell_controller::HellController;
pub use js8_controller::Js8Controller;
pub use modem::{ApHints, Ft8Modem};
pub use navtex_controller::NavtexController;
pub use packet_controller::PacketController;
pub use params::{DECODE_RATE, DigiParams};
pub use qso::QsoMachine;
pub use rade_controller::RadeController;
pub use rf_paint_controller::RfPaintController;
pub use rifp_controller::RifpController;
pub use scheduler::SlotScheduler;
pub use sstv_controller::SstvController;
pub use text_modem::TextModemController;
pub use wefax_controller::WefaxController;
pub use wspr_controller::WsprController;

use std::time::SystemTime;

use sdroxide_types::{DigiConfig, DigiStatus, Mode, SstvMode};

/// The engine-facing digital-mode seam, implemented by the slotted FT8/FT4
/// [`DigiController`] and the continuous-keyboard [`TextModemController`]. The
/// engine holds one as `Box<dyn DigiEngine>` and never branches on the mode.
///
/// Method-syntax note: the FT8 controller keeps inherent methods of the same
/// names, so its trait impl delegates with fully-qualified calls.
pub trait DigiEngine: Send {
    fn mode(&self) -> Mode;
    fn on_rx_audio(&mut self, tap: &[f32]);
    fn poll(&mut self, now: SystemTime, dial_hz: f64) -> Vec<DigiAction>;
    fn tx_burst_active(&self) -> bool;
    fn fill_tx_block(&mut self, out: &mut [f32]) -> bool;

    /// The rate [`fill_tx_block`](Self::fill_tx_block) hands samples back at.
    ///
    /// Not the receive tap's rate, and not always 48 kHz. Nearly every modem
    /// here synthesises its transmit audio at a fixed 48 kHz whatever it is
    /// listening at — the burst synthesisers resample their own 12 kHz work up
    /// to it, and the keyboard modes their 8 kHz — so 48 kHz is the default and
    /// the two AX.25 controllers, whose modem is built at the tap rate, are the
    /// only ones that override it.
    ///
    /// The engine rate-matches this to whatever the radio actually plays, so a
    /// wrong answer here is a burst that goes out at the wrong speed rather
    /// than one that sounds wrong: an Icom on its 12 kHz IF receives at 24 kHz
    /// and takes transmit audio back at 48, and taking the tap for the answer
    /// stretched every FT8/FT4 over to twice its length ([issue #359]) — the
    /// mirror of the packet burst that went out at twice its baud rate when
    /// there was no rate matching at all ([issue #150]).
    ///
    /// [issue #150]: https://github.com/dividebysandwich/sdroxide/issues/150
    /// [issue #359]: https://github.com/dividebysandwich/sdroxide/issues/359
    fn tx_rate(&self) -> f64 {
        48_000.0
    }

    /// The peak amplitude [`fill_tx_block`](Self::fill_tx_block) reaches, as a
    /// fraction of full scale.
    ///
    /// Nearly every modem here synthesises its modulating signal at half scale
    /// — `RttyTx`, `SstvTx`, `HellTx`, the AFSK and G3RUH packet modems, the
    /// Olivia/THOR/FSQ tone generators, and the FT8/FT4/JS8/WSPR burst
    /// synthesisers all agree on 0.5 — leaving 6 dB of headroom above the
    /// waveform. That headroom belongs to the modem, not to the transmitter:
    /// the chain divides it out again so a hundred-percent Drive puts the over
    /// on the air at full scale, the same level a TUNE goes out at. Without
    /// that, every digital over sat 6 dB — three quarters of the power — below
    /// what the Drive slider said, and on a rig that modulates the audio we
    /// send it no slider could get that back: the radio was simply never asked
    /// for more than a quarter of its power ([issue #131]).
    ///
    /// Overridden wherever that is not the figure: the modes whose signal is
    /// already at the level it wants ([`CwController`], [`RadeController`],
    /// [`RifpController`]) and the ones that leave a different amount of room
    /// ([`PacketController`] at 9600, [`RfPaintController`], which sums tones).
    /// A mode that gets this wrong transmits quiet or transmits into the
    /// limiter, so it is worth measuring rather than assuming.
    ///
    /// [issue #131]: https://github.com/dividebysandwich/sdroxide/issues/131
    fn tx_peak(&self) -> f32 {
        0.5
    }

    fn on_burst_done(&mut self);
    fn abort(&mut self);
    fn abort_tx(&mut self);
    fn set_config(&mut self, cfg: DigiConfig);
    fn set_audio_hz(&mut self, hz: f32);
    /// Put the transmit tone back where the operator last left it on this band.
    ///
    /// Separate from [`set_audio_hz`](Self::set_audio_hz) because it is the one
    /// move Hold TX does not block. Hold exists to stop the tone drifting on its
    /// own; changing band is the operator's own act, and the figure being
    /// restored is one they chose there themselves. Holding through a band
    /// change would instead carry a licence-edge figure onto a band that does
    /// not want it, or 1500 Hz onto one that cannot have it.
    ///
    /// Defaults to the ordinary route, so a mode with no hold of its own, and
    /// no per-band memory, needs no implementation.
    fn restore_audio_hz(&mut self, hz: f32) {
        self.set_audio_hz(hz);
    }
    fn audio_hz(&self) -> f32;
    fn status(&self) -> DigiStatus;

    // Actions that only some modes use; default to no-ops.
    fn call_cq(&mut self) {}
    fn start_qso(
        &mut self,
        _from: String,
        _grid: Option<String>,
        _snr: i16,
        _audio_hz: f32,
        _wait_for_cq: bool,
    ) {
    }
    fn stop_qso(&mut self) {}
    /// FT8/FT4: pick which message goes out next (the operator's Tx1–Tx6).
    fn set_step(&mut self, _step: sdroxide_types::QsoStep) {}
    /// FT8/FT4: queue a message to send verbatim in the next transmit slot.
    fn send_text(&mut self, _text: String) {}
    /// FT8/FT4: mark a station to work when the sequencer is next free.
    fn queue_add(&mut self, _entry: sdroxide_types::QueuedCall) {}
    /// FT8/FT4: drop a station from the call queue (empty callsign clears it).
    fn queue_remove(&mut self, _call: &str) {}
    /// Continuous keyboard modes: replace the outgoing text buffer.
    fn set_tx_text(&mut self, _text: String) {}
    /// Continuous keyboard modes: enter/leave transmit.
    fn set_tx_active(&mut self, _on: bool) {}
    /// Throw away what has been copied so far, so the operator can start a
    /// fresh page. Only the received text goes: the decoder keeps running, an
    /// over in progress is untouched, and nothing already logged is lost. Inert
    /// for the modes that receive no text at all (the image modes, digital
    /// voice), which is why it defaults to nothing.
    fn clear_rx(&mut self) {}
    /// SSTV: select the mode (`None` = auto-detect on RX, Martin 1 on TX).
    fn set_sstv_mode(&mut self, _mode: Option<SstvMode>) {}
    /// SSTV: queue a composed image (interleaved RGB) and start transmitting.
    fn set_sstv_image(&mut self, _mode: SstvMode, _rgb: Vec<u8>, _w: u16, _h: u16) {}
    /// SSTV: throw away the picture being received and hunt for a header again.
    fn sstv_restart_rx(&mut self) {}

    /// Weather fax: begin a picture now rather than waiting for a start tone.
    /// The usual way to catch a chart already under way, which on a
    /// fifteen-minute transmission is most of the time.
    fn wefax_start(&mut self) {}

    /// Weather fax: end the picture in progress, keeping what has arrived.
    fn wefax_stop(&mut self) {}

    /// Weather fax: shift the line alignment by whole pixels, to straighten a
    /// chart whose phasing pulse was missed.
    fn wefax_nudge(&mut self, _pixels: i32) {}
    /// RIFP: queue a composed image (interleaved RGB) and start transmitting.
    /// The controller encodes, chunks and frames it per the operator's config.
    fn set_rifp_image(&mut self, _rgb: Vec<u8>, _w: u16, _h: u16) {}
    /// RIFP: forget an incomplete incoming session by its 16-hex-digit ID, or
    /// all of them when the string is empty.
    fn rifp_drop_session(&mut self, _session: &str) {}
    /// Packet: send one UNPROTO identification frame now.
    fn packet_beacon_now(&mut self) {}
    /// Packet: transmit a frame on a KISS host's behalf. Subject to CSMA like
    /// everything else — a host asks for the channel, it does not take it.
    fn packet_send_frame(&mut self, _frame: Vec<u8>) {}

    /// Packet: call a station in connected mode. `via` is the operator's path
    /// string, parsed here where the callsign validation lives.
    fn packet_connect(&mut self, _call: String, _via: String, _ext: bool) {}

    /// Packet: send one line to the connected station.
    fn packet_send_line(&mut self, _text: String) {}

    /// Packet: hang up the connected-mode link.
    fn packet_disconnect(&mut self) {}

    /// Packet: empty the terminal transcript, leaving the link alone.
    fn packet_term_clear(&mut self) {}
    /// APRS: send one position beacon now. Still subject to CSMA — the
    /// operator asks for the channel, they do not take it.
    fn aprs_beacon_now(&mut self) {}
    /// APRS: send a message, and keep retrying it until it is acknowledged.
    fn aprs_send_message(&mut self, _to: String, _text: String) {}
    /// Packet: frames heard since the last call, for a KISS host. Drained
    /// rather than pushed, so a controller need not know a socket exists.
    fn packet_take_air_frames(&mut self) -> Vec<Vec<u8>> {
        Vec::new()
    }

    /// FSQ image: queue a grayscale image (`w*h` bytes) and start transmitting.
    fn set_image(&mut self, _gray: Vec<u8>, _w: u16, _h: u16) {}

    // --- AtCHAT NET ---
    //
    // A whole NET protocol rather than a keyboard buffer: chat is addressed
    // (common channel or a directed callsign), files go by block-CRC-ARQ, and
    // the link can be dropped and resumed. These are the operator's side of
    // that, and default to inert.

    /// AtCHAT: send a chat line — `to` empty is the common channel, a callsign
    /// is a directed message.
    fn atchat_send_chat(&mut self, _to: String, _text: String) {}

    /// AtCHAT: send a file or image to a station (or the common channel when
    /// `to` is empty).
    fn atchat_send_file(&mut self, _to: String, _path: std::path::PathBuf) {}

    /// AtCHAT: drop the channel link, keeping station state so a reconnect can
    /// resume half-finished transfers.
    fn atchat_drop(&mut self) {}

    /// AtCHAT: rejoin after [`DigiEngine::atchat_drop`] — a JOIN_REQUEST only,
    /// never a master claim while a beacon is heard.
    fn atchat_reconnect(&mut self) {}

    // --- digital voice ---
    //
    // The text and image modes are decoded *from* the receive audio and
    // transmitted *as* a synthesised burst. Digital voice is neither: it
    // produces receive audio of its own, and it transmits the live microphone.
    // These three hooks are the whole difference, and default to inert.

    /// Take decoded speech at 48 kHz, appending to `out`.
    ///
    /// `true` means this mode is producing audio and the engine should play it
    /// instead of the demodulated signal; `false` leaves the normal audio path
    /// alone (so an out-of-sync RADE receiver still passes the raw SSB through,
    /// unless [`DigiEngine::mutes_analog_audio`] says otherwise).
    fn rx_audio_out(&mut self, _out: &mut Vec<f32>) -> bool {
        false
    }

    /// True when the mode wants the demodulated (analog) audio silenced rather
    /// than passed through, so only what it decodes is audible. Consulted only
    /// where [`DigiEngine::rx_audio_out`] declined — decoded audio always wins.
    fn mutes_analog_audio(&self) -> bool {
        false
    }

    /// True for modes that transmit live microphone audio, so the engine keeps
    /// the mic alive during transmit instead of discarding it.
    fn wants_mic(&self) -> bool {
        false
    }

    /// Microphone audio at 48 kHz, delivered only while [`DigiEngine::wants_mic`].
    fn on_tx_mic(&mut self, _mic_48k: &[f32]) {}
}

#[cfg(test)]
mod dispatch_tests {
    use super::*;
    use sdroxide_types::Js8Speed;

    /// Mirrors `Engine::make_digi`'s predicate chain. Kept here so the ordering
    /// can be exercised without standing up an engine.
    fn pick(mode: Mode) -> &'static str {
        if mode == Mode::Cw {
            "cw"
        } else if mode.is_rade() {
            "rade"
        } else if mode.is_atchat() {
            "atchat"
        } else if mode.is_sstv() {
            "sstv"
        } else if mode.is_wefax() {
            "wefax"
        } else if mode.is_rifp() {
            "rifp"
        } else if mode.is_aprs() {
            "aprs"
        } else if mode.is_packet() {
            "packet"
        } else if mode.is_rf_paint() {
            "rfpaint"
        } else if mode.is_fsq() {
            "fsq"
        } else if mode.is_hell() {
            "hell"
        } else if mode.is_text_modem() {
            "text"
        } else if mode.is_js8() {
            "js8"
        } else if mode.is_wspr() {
            "wspr"
        } else {
            "ft8"
        }
    }

    #[test]
    fn every_digital_mode_reaches_its_own_controller() {
        // The dangerous case is JS8: it is slotted like FT8, so a missing
        // branch hands it an FT8 decoder and nothing downstream notices — no
        // error, no log line, just a receiver listening for another protocol.
        assert_eq!(pick(Mode::Js8), "js8");
        // WSPR is the same trap as JS8, and worse: it is slotted, it is 4-FSK
        // in the same passband, and a controller handed FT8's decoder would sit
        // there finding nothing for ever without a word.
        assert_eq!(pick(Mode::Wspr), "wspr");
        assert_eq!(pick(Mode::Ft8), "ft8");
        assert_eq!(pick(Mode::Ft4), "ft8");
        assert_eq!(pick(Mode::Ft2), "ft8");
        assert_eq!(pick(Mode::Fsq), "fsq");
        assert_eq!(pick(Mode::Hell), "hell");
        assert_eq!(pick(Mode::Psk), "text");
        assert_eq!(pick(Mode::Rade), "rade");
        // AtCHAT is neither slotted nor a keyboard modem; the fall-through
        // would hand it an FT8 decoder and its NET station would never join.
        assert_eq!(pick(Mode::AtChat), "atchat");
        assert_eq!(pick(Mode::Wefax), "wefax");
        // Both packet modes reach the one packet controller. HF packet is the
        // quiet trap of the pair: it is a keyboard-shaped mode on a sideband,
        // so `is_text_modem` further down would look like a plausible home and
        // the operator would get a PSK decoder listening to AX.25.
        assert_eq!(pick(Mode::Packet), "packet");
        assert_eq!(pick(Mode::PacketHf), "packet");
        // APRS is the same trap one level down: it is AX.25 over the same
        // 1200 baud modem, so the packet branch would decode the channel
        // perfectly and hand the operator a monitor with no map, no messages
        // and no beacon. `Mode::is_packet` deliberately excludes it, and this
        // is what says so.
        assert_eq!(pick(Mode::Aprs), "aprs");
        assert!(!Mode::Aprs.is_packet(), "APRS must not reach the packet controller");
        // CW is not a digital mode at all, so it has to be picked off before
        // the chain rather than by it — the fall-through would hand it an FT8
        // decoder, which would sit there decoding nothing for ever.
        assert_eq!(pick(Mode::Cw), "cw");
        assert!(!Mode::Cw.is_digital(), "CW must stay an analog mode");
    }

    #[test]
    fn a_js8_controller_reports_js8_at_every_speed() {
        for speed in Js8Speed::ALL {
            let cfg = DigiConfig { js8_speed: speed, ..Default::default() };
            let c = Js8Controller::new(cfg, 48_000.0);
            assert_eq!(c.mode(), Mode::Js8, "{}", speed.label());
            let s = c.status();
            assert_eq!(s.mode, Mode::Js8);
            assert_eq!(s.js8.expect("js8 status").speed, speed);
        }
    }
}
