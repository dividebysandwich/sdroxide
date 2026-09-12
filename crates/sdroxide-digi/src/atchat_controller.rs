//! `AtChatController` — AtCHAT NET, a 2.7 kHz COFDM multi-station keyboard and
//! file mode.
//!
//! Structurally this is the keyboard-modem controller with a whole NET protocol
//! behind it: dynamic master election, a shared roster, common and directed
//! chat, and block-CRC-ARQ file/image transfer. All of that — the OFDM modem,
//! the protocol state machine, the channel — runs on
//! [`AtChatSession`]'s own thread; this controller only moves 8 kHz PCM across
//! the session's ring buffers and resamples it to and from the radio's rate,
//! so nothing here can stall the engine's audio loop.
//!
//! Two channels are possible and the operator's config picks which: over the
//! radio (`RadioConnector`, real listen-before-transmit off the demodulated
//! carrier) or a `channel_server`-compatible virtual TCP endpoint for
//! radio-less development ([`DigiConfig::atchat_virtual`]).

use std::path::PathBuf;
use std::time::SystemTime;

use sdroxide_atchat::AtChatSession;
use sdroxide_dsp::MonoResampler;
use sdroxide_types::{
    AtChatChatLine, AtChatFile, AtChatRosterEntry, AtChatStatus, AtChatTransfer, DigiConfig,
    DigiStatus, Mode, QsoStep, TranscriptLine,
};

use crate::DigiEngine;
use crate::controller::DigiAction;

/// The modem's sample rate — everything the session produces and consumes.
const MODEM_RATE: f64 = 8_000.0;

/// The centre of the occupied band (~312–2688 Hz); the honest value for the
/// panadapter marker, since the carriers are fixed by the waveform and there is
/// no operator tone offset to move.
const BAND_CENTRE_HZ: f32 = 1_500.0;

pub struct AtChatController {
    cfg: DigiConfig,
    session: AtChatSession,

    /// Radio tap rate → 8 kHz for the session's receive path. `None` when the
    /// tap already runs at 8 kHz.
    rx_rs: Option<MonoResampler>,
    /// Scratch for the resampled receive audio.
    rx_scratch: Vec<f32>,

    /// True while our own frame is on the air, so the received copy of it is
    /// not fed back in as a signal to decode.
    keyed: bool,
    /// Set once the queued burst has drained, so `fill_tx_block` reports the
    /// over finished.
    tx_done: bool,
    /// 8 kHz transmit samples pulled from the session and not yet handed out.
    tx_fifo: Vec<f32>,
    /// Samples handed to the radio for the burst currently on the air — for the
    /// diagnostic note when the burst ends.
    tx_burst_samples: usize,

    last_status: DigiStatus,
    status_dirty: bool,
}

impl AtChatController {
    pub fn new(cfg: DigiConfig, tap_rate: f64) -> Self {
        let virt = cfg.atchat_virtual.then(|| cfg.atchat_virtual_addr.clone());
        let session = AtChatSession::new(&cfg.my_call, virt);
        let rx_rs = MonoResampler::new(tap_rate, MODEM_RATE);
        let last_status = build_status(&cfg, &session);
        AtChatController {
            cfg,
            session,
            rx_rs,
            rx_scratch: Vec::new(),
            keyed: false,
            tx_done: false,
            tx_fifo: Vec::new(),
            tx_burst_samples: 0,
            last_status,
            status_dirty: true,
        }
    }
}

fn build_status(cfg: &DigiConfig, session: &AtChatSession) -> DigiStatus {
    let s = session.snapshot();
    let atchat = AtChatStatus {
        my_call: s.my_call,
        connected: s.connected,
        role: s.role.map(str::to_string),
        master: s.master,
        roster: s
            .roster
            .into_iter()
            .map(|(call, status, age_s)| AtChatRosterEntry {
                call,
                status: status.to_string(),
                age_s,
            })
            .collect(),
        chat: s
            .chat
            .into_iter()
            .map(|c| AtChatChatLine {
                from: c.from,
                dst: c.dst,
                text: c.text,
                own: c.own,
                private: c.private,
                when: c.when,
            })
            .collect(),
        transfers: s
            .transfers
            .into_iter()
            .map(|t| AtChatTransfer {
                id: t.id,
                filename: t.filename,
                peer: t.peer,
                incoming: t.incoming,
                have: t.have as u32,
                total: t.total as u32,
                complete: t.complete,
            })
            .collect(),
        files: s
            .files
            .into_iter()
            .map(|f| AtChatFile {
                from: f.from,
                filename: f.filename,
                path: f.path,
                is_image: f.is_image,
                when: f.when,
            })
            .collect(),
        log: s.log,
        keyed: s.keyed,
        carrier: s.carrier,
        virtual_addr: s.virtual_addr,
    };

    DigiStatus {
        mode: Mode::AtChat,
        step: QsoStep::Idle,
        dx_call: None,
        dx_grid: None,
        tx_next: atchat.keyed,
        tx_pending_msg: None,
        audio_hz: BAND_CENTRE_HZ,
        tx_even: false,
        transmitting: atchat.keyed,
        tx_watchdog: false,
        transcript: Vec::<TranscriptLine>::new(),
        config: cfg.clone(),
        text_rx: String::new(),
        tx_sent: 0,
        fsq_heard: Vec::new(),
        fsq_messages: Vec::new(),
        rade: None,
        packet: None,
        navtex: None,
        aprs: None,
        js8: None,
        atchat: Some(Box::new(atchat)),
        fox_queue: Vec::new(),
        call_queue: Vec::new(),
        clock_offset_s: None,
        cw: None,
        wspr: None,
        qso: None,
    }
}

impl DigiEngine for AtChatController {
    fn mode(&self) -> Mode {
        Mode::AtChat
    }

    fn on_rx_audio(&mut self, tap: &[f32]) {
        if self.keyed {
            return; // our own transmit audio is not a signal to decode
        }
        let pcm = match self.rx_rs.as_mut() {
            Some(rs) => {
                self.rx_scratch.clear();
                rs.push(tap, &mut self.rx_scratch);
                &self.rx_scratch[..]
            }
            None => tap,
        };
        if pcm.is_empty() {
            return;
        }
        let i16s: Vec<i16> =
            pcm.iter().map(|&x| (x * 32767.0).clamp(-32768.0, 32767.0) as i16).collect();
        self.session.push_rx_pcm(&i16s);
    }

    fn poll(&mut self, _now: SystemTime, _dial_hz: f64) -> Vec<DigiAction> {
        let mut actions = Vec::new();

        // A frame the station wants to send is already past its own
        // listen-before-transmit by the time it reaches the session's transmit
        // ring, so keying here is unconditional.
        if !self.keyed && self.session.tx_pending() {
            self.keyed = true;
            self.tx_done = false;
            self.tx_fifo.clear();
            self.tx_burst_samples = 0;
            self.status_dirty = true;
            self.session.note("controller: modem burst ready — keying the transmitter");
            actions.push(DigiAction::KeyTx);
        }

        let status = build_status(&self.cfg, &self.session);
        if self.status_dirty || status.atchat != self.last_status.atchat {
            self.status_dirty = false;
            self.last_status = status.clone();
            actions.push(DigiAction::Status(status));
        }
        actions
    }

    fn tx_burst_active(&self) -> bool {
        self.keyed
    }

    /// The session hands transmit audio back at the modem's 8 kHz; the engine
    /// rate-matches it to whatever the radio plays.
    fn tx_rate(&self) -> f64 {
        MODEM_RATE
    }

    /// The modem peak-normalises every burst to 0.7 of full scale; that 0.7 is
    /// headroom the transmitter may have back (issue #131's rule), so it is
    /// declared here for the chain to divide out.
    fn tx_peak(&self) -> f32 {
        0.7
    }

    fn fill_tx_block(&mut self, out: &mut [f32]) -> bool {
        // Top the FIFO up from the session's 8 kHz transmit ring.
        if self.tx_fifo.len() < out.len() {
            let mut pcm: Vec<i16> = Vec::new();
            let want = out.len().saturating_sub(self.tx_fifo.len());
            self.session.drain_tx_pcm(&mut pcm, want);
            self.tx_fifo.extend(pcm.iter().map(|&s| s as f32 / 32768.0));
        }

        let take = self.tx_fifo.len().min(out.len());
        out[..take].copy_from_slice(&self.tx_fifo[..take]);
        out[take..].fill(0.0);
        self.tx_fifo.drain(..take);
        self.tx_burst_samples += take;

        // The over ends once the session has nothing more queued and the FIFO
        // has played out. Returning true early would clip the tail of a frame,
        // which for a bulk block is its CRC.
        if !self.session.tx_pending() && self.tx_fifo.is_empty() {
            self.tx_done = true;
        }
        self.tx_done
    }

    fn on_burst_done(&mut self) {
        self.session.note(format!(
            "controller: burst finished — {} samples sent to the radio",
            self.tx_burst_samples
        ));
        self.keyed = false;
        self.tx_done = false;
        self.tx_fifo.clear();
        self.tx_burst_samples = 0;
        self.status_dirty = true;
    }

    fn abort(&mut self) {
        self.abort_tx();
    }

    fn abort_tx(&mut self) {
        if self.keyed {
            self.session.note(format!(
                "controller: transmit aborted after {} samples — the TX rails refused or the \
                 over was interrupted",
                self.tx_burst_samples
            ));
        }
        self.keyed = false;
        self.tx_done = true;
        self.tx_fifo.clear();
        self.tx_burst_samples = 0;
        self.status_dirty = true;
    }

    fn set_config(&mut self, cfg: DigiConfig) {
        if cfg.my_call != self.cfg.my_call {
            self.session.set_callsign(&cfg.my_call);
        }
        let was = self.cfg.atchat_virtual.then(|| self.cfg.atchat_virtual_addr.clone());
        let now = cfg.atchat_virtual.then(|| cfg.atchat_virtual_addr.clone());
        if was != now {
            self.session.set_virtual(now);
        }
        self.cfg = cfg;
        self.status_dirty = true;
    }

    /// AtCHAT's carriers are fixed by the waveform — there is no tone offset to
    /// move, so the audio-frequency control does nothing here.
    fn set_audio_hz(&mut self, _hz: f32) {}

    fn audio_hz(&self) -> f32 {
        BAND_CENTRE_HZ
    }

    fn status(&self) -> DigiStatus {
        build_status(&self.cfg, &self.session)
    }

    // --- AtCHAT-specific operator actions ---

    fn atchat_send_chat(&mut self, to: String, text: String) {
        self.session.send_chat(&to, &text);
        self.status_dirty = true;
    }

    fn atchat_send_file(&mut self, to: String, path: PathBuf) {
        self.session.send_file(path, &to);
        self.status_dirty = true;
    }

    fn atchat_drop(&mut self) {
        self.session.drop_link();
        self.status_dirty = true;
    }

    fn atchat_reconnect(&mut self) {
        self.session.reconnect();
        self.status_dirty = true;
    }

    fn clear_rx(&mut self) {
        self.session.clear_chat();
        self.status_dirty = true;
    }
}
