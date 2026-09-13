//! OpenHPSDR Protocol 2 ("new protocol") wire format: constants and pure
//! packet builders/parsers.
//!
//! Byte offsets follow the g0orx/rustyHPSDR reference and the N4MTT
//! "openhpsdr-e" Wireshark dissector. They are cross-checked but should be
//! verified field-by-field against the TAPR Protocol 2 documentation before
//! trusting on-air behavior; see the notes on individual builders.

use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crossbeam_channel::Receiver;
use rtrb::{Consumer, Producer};

use crate::net::{Ctrl, RxStats, SeqTracker, ThreadCtx, WATCHDOG, hex_head, push_iq};
use sdroxide_types::HpsdrOcPlan;

/// UDP ports. Host→radio use these as the *destination* port; radio→host DDC IQ
/// arrives with a *source* port of [`port::DDC_IQ_BASE`]` + ddc_index`.
pub mod port {
    /// Discovery + general/run/watchdog command (host→radio) and command reply.
    pub const GENERAL: u16 = 1024;
    /// DDC (receiver) configuration command.
    pub const DDC_COMMAND: u16 = 1025;
    /// DUC (transmitter) configuration command.
    pub const DUC_COMMAND: u16 = 1026;
    /// High-priority command: NCO frequencies, PTT/MOX, drive.
    pub const HIGH_PRIORITY: u16 = 1027;
    /// DUC I/Q data out to the radio (TX).
    pub const DUC_IQ: u16 = 1029;
    /// Radio→host: DDC I/Q streams start here (DDC0 = 1035).
    pub const DDC_IQ_BASE: u16 = 1035;
}

/// Master sample clock (Hz) used for the NCO phase-word math on
/// Hermes/Angelia/Orion/Saturn boards.
pub const CLOCK_HZ: f64 = 122_880_000.0;
/// Hermes-Lite 2 (Protocol 1) clock; kept for reference / future P1 support.
#[allow(dead_code)]
pub const CLOCK_HZ_HL2: f64 = 76_800_000.0;

/// Packet sizes (bytes).
pub const GENERAL_LEN: usize = 60;
pub const DDC_COMMAND_LEN: usize = 1444;
pub const DUC_COMMAND_LEN: usize = 60;
pub const HIGH_PRIORITY_LEN: usize = 1444;
/// 4-byte sequence + 240 IQ pairs × 6 bytes.
pub const DUC_IQ_LEN: usize = 4 + DUC_SAMPLES_PER_PKT * 6;
/// I/Q pairs per DUC (TX) datagram (rustyHPSDR `IQ_BUFFER_SIZE`).
pub const DUC_SAMPLES_PER_PKT: usize = 240;
/// Header bytes preceding the I/Q payload in a DDC (RX) datagram.
pub const DDC_IQ_HEADER_LEN: usize = 16;

/// Full-scale for 24-bit samples (2^23). RX divides by this; TX multiplies by
/// (this − 1) to avoid overflow at +1.0.
const FULL_SCALE: f32 = 8_388_608.0;

/// The 32-bit NCO phase word for `freq_hz` at `clock_hz`.
pub fn phase_word(freq_hz: f64, clock_hz: f64) -> u32 {
    let f = freq_hz.max(0.0);
    ((f / clock_hz) * 4_294_967_296.0).round() as u32
}

/// Decode a 24-bit big-endian two's-complement sample.
pub fn be24_to_i32(b: [u8; 3]) -> i32 {
    let u = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | (b[2] as u32);
    if u & 0x0080_0000 != 0 { (u | 0xFF00_0000) as i32 } else { u as i32 }
}

/// Encode a 24-bit big-endian two's-complement sample.
pub fn i32_to_be24(v: i32) -> [u8; 3] {
    let u = (v as u32) & 0x00FF_FFFF;
    [(u >> 16) as u8, (u >> 8) as u8, u as u8]
}

/// `-1.0..=1.0` float → 24-bit BE sample bytes.
pub fn f32_to_be24(x: f32) -> [u8; 3] {
    let v = (x.clamp(-1.0, 1.0) * (FULL_SCALE - 1.0)).round() as i32;
    i32_to_be24(v)
}

/// 24-bit BE sample bytes → `-1.0..=1.0` float.
pub fn be24_to_f32(b: [u8; 3]) -> f32 {
    be24_to_i32(b) as f32 / FULL_SCALE
}

/// Write a 32-bit big-endian value at `buf[off..off+4]`.
fn put_u32_be(buf: &mut [u8], off: usize, v: u32) {
    buf[off..off + 4].copy_from_slice(&v.to_be_bytes());
}

/// Build the General packet (dest port 1024): the port assignments and the
/// board-wide switches. It carries **no run bit** — that lives in the
/// high-priority packet, which is the only thing that starts and stops a
/// Protocol 2 radio.
///
/// Byte 4 is the *packet type* of the 60-byte datagrams that go to port 1024,
/// and `0x00` is what makes this one a General packet: `0x02` is discovery,
/// `0x03`/`0x04` erase and program, `0x06` sets the radio's IP. A board that
/// reads anything else there has been handed a packet of a type it does not
/// know and drops the whole thing — which is what sdroxide used to send
/// whenever it meant "run", so the port table, the phase-word mode bit, the
/// watchdog and the PA/Alex switches never reached the radio at all. An ANAN
/// G2 (Saturn) answered that by configuring its DDCs and streaming nothing
/// (issue #281).
///
/// Every field left at zero means "use the default", which is what the port
/// assignments want: the defaults are the ports this crate listens on.
///
/// `alex_both` enables the second Alex/filter chain, which the two-ADC boards
/// (Orion2, Saturn) have and the rest do not.
pub fn general_packet(seq: u32, alex_both: bool) -> [u8; GENERAL_LEN] {
    let mut b = [0u8; GENERAL_LEN];
    put_u32_be(&mut b, 0, seq);
    b[4] = 0x00; // packet type: General. Never anything else.
    b[23] = 0x00; // wideband disabled
    b[37] = 0x08; // NCO fields carry phase words, not frequencies
    b[38] = 0x01; // hardware watchdog enable
    b[58] = 0x01; // PA enable
    b[59] = if alex_both { 0x03 } else { 0x01 }; // ALEX 0, and 1 on a two-ADC board
    b
}

/// How many DDCs the Protocol 2 framing carries (DDC0..7, IQ source ports
/// 1035..1042). Boards implement between 2 and all 8 of them.
pub const MAX_DDCS: u8 = 8;

/// Build the DDC command packet (dest port 1025): enable every DDC in `ddcs`
/// at `rate_khz`, 24 bits/sample, ADC0.
///
/// Always the **whole** table: the enable mask and each enabled DDC's
/// descriptor (stride 6 from offset 17) are written from scratch on every
/// call, so the packet states the complete intended configuration rather than
/// editing whatever the board had — a partial write leaving stale descriptors
/// behind is exactly the firmware-dependent hazard to avoid.
pub fn ddc_command_packet(seq: u32, rate_khz: u16, ddcs: &[u8]) -> [u8; DDC_COMMAND_LEN] {
    let mut b = [0u8; DDC_COMMAND_LEN];
    put_u32_be(&mut b, 0, seq);
    b[4] = 1; // ADC count
    for &ddc in ddcs {
        if ddc >= MAX_DDCS {
            continue;
        }
        b[7] |= 1 << ddc; // DDC enable mask
        // Descriptor: ADC sel, rate (kHz, BE u16), bits per sample.
        let off = 17 + ddc as usize * 6;
        b[off] = 0; // ADC0
        b[off + 1] = (rate_khz >> 8) as u8;
        b[off + 2] = (rate_khz & 0xff) as u8;
        b[off + 5] = 24; // bits per sample
    }
    b
}

/// Build the DUC command packet (dest port 1026). Minimal linear-SSB config.
pub fn duc_command_packet(seq: u32) -> [u8; DUC_COMMAND_LEN] {
    let mut b = [0u8; DUC_COMMAND_LEN];
    put_u32_be(&mut b, 0, seq);
    b[5] = 0x00; // mode flags (no CW/keyer)
    b[50] = 0x00; // mic config
    b[51] = 0x00; // line-in / DUC gain
    b
}

/// Byte of the high-priority packet carrying the open-collector outputs, and
/// the shift they sit at inside it.
///
/// Same convention as Protocol 1's C2 (`protocol1::config_cc`): the seven
/// outputs occupy bits 7..1 with bit 0 unused, so the word is written shifted
/// up by one. Offset and shift are from piHPSDR's `new_protocol.c`, which
/// writes `band->OCtx`/`OCrx << 1` here, and from the N4MTT dissector; like
/// every other offset in this file they are **not** hardware-verified.
const HP_OC: usize = 1400;

/// Build the High-Priority command packet (dest port 1027): the run bit, RX/TX
/// NCO frequencies, PTT/MOX, drive level (0..=255) and the open-collector
/// outputs.
///
/// Offsets: DDC-*n* RX NCO @ `buf[9 + 4n .. 13 + 4n]` (the spec's frequency
/// table, 4 bytes per DDC), TX DUC0 NCO @ `buf[329..333]`, drive @ `buf[345]`,
/// open collectors @ `buf[1400]` (see [`HP_OC`]), run/MOX flags @ `buf[4]` —
/// canonical P2 values; DDC0's offset is hardware-verified, the stride is per
/// the TAPR layout.
///
/// `run` is bit 0 of byte 4 and is the *only* thing that starts and stops the
/// radio's streams. Sending this packet with it clear is how a session ends —
/// nothing on port 1024 will do it, and a radio left running goes on answering
/// discovery as "in use" long after the program that started it has gone.
///
/// `rx_phases[n]` is DDC *n*'s phase word; a DDC that is not enabled simply
/// has its slot written as given (zero for an unused one is fine — the board
/// ignores frequencies of disabled DDCs).
///
/// `oc` is the seven open-collector lines as the accessory board sees them
/// (bit 0 = output 1), and is zero on a station that has configured no filter
/// board — which is what every Protocol 2 radio sent before issue #296,
/// because nothing on this path wrote the byte at all.
pub fn high_priority_packet(
    seq: u32,
    rx_phases: &[u32; MAX_DDCS as usize],
    tx_phase: u32,
    run: bool,
    ptt: bool,
    drive: u8,
    oc: u8,
) -> [u8; HIGH_PRIORITY_LEN] {
    let mut b = [0u8; HIGH_PRIORITY_LEN];
    put_u32_be(&mut b, 0, seq);
    b[4] = u8::from(run) | if ptt { 0x02 } else { 0x00 }; // run + MOX
    for (n, &phase) in rx_phases.iter().enumerate() {
        put_u32_be(&mut b, 9 + 4 * n, phase); // DDC-n RX NCO
    }
    put_u32_be(&mut b, 329, tx_phase); // DUC0 TX NCO
    b[345] = drive; // TX drive level 0..255
    b[HP_OC] = (oc & 0x7F) << 1; // open-collector outputs 1..7
    b
}

/// Build one DUC I/Q datagram (dest port 1029) from up to
/// [`DUC_SAMPLES_PER_PKT`] interleaved I,Q float pairs. Pads with zeros.
pub fn duc_iq_packet(seq: u32, interleaved_iq: &[f32]) -> [u8; DUC_IQ_LEN] {
    let mut b = [0u8; DUC_IQ_LEN];
    put_u32_be(&mut b, 0, seq);
    let pairs = (interleaved_iq.len() / 2).min(DUC_SAMPLES_PER_PKT);
    for p in 0..pairs {
        let i = f32_to_be24(interleaved_iq[2 * p]);
        let q = f32_to_be24(interleaved_iq[2 * p + 1]);
        let off = 4 + p * 6;
        b[off..off + 3].copy_from_slice(&i);
        b[off + 3..off + 6].copy_from_slice(&q);
    }
    b
}

/// Decode a DDC (RX) I/Q datagram, appending `-1.0..=1.0` interleaved I,Q pairs
/// to `out`. Returns the number of complex samples decoded, or `None` if the
/// packet is too short / malformed.
pub fn decode_ddc_iq(pkt: &[u8], out: &mut Vec<f32>) -> Option<usize> {
    if pkt.len() < DDC_IQ_HEADER_LEN {
        return None;
    }
    // Sample count lives in the header at [14..16] (BE u16); fall back to
    // deriving it from the payload length if the field looks wrong.
    let declared = u16::from_be_bytes([pkt[14], pkt[15]]) as usize;
    let payload = &pkt[DDC_IQ_HEADER_LEN..];
    let max_pairs = payload.len() / 6;
    let pairs = if declared > 0 && declared <= max_pairs { declared } else { max_pairs };
    for p in 0..pairs {
        let off = p * 6;
        let i = be24_to_f32([payload[off], payload[off + 1], payload[off + 2]]);
        let q = be24_to_f32([payload[off + 3], payload[off + 4], payload[off + 5]]);
        out.push(i);
        out.push(q);
    }
    Some(pairs)
}

// ---------------------------------------------------------------------------
// Protocol 2 network thread
// ---------------------------------------------------------------------------

/// Fixed FPGA drive level while keyed. The engine already scales the I/Q by the
/// operator's drive fraction in software, so the FPGA runs at full scale and the
/// I/Q amplitude sets the power (the TX safety rails still apply upstream).
const TX_DRIVE: u8 = 255;

#[derive(Default)]
struct Seq {
    general: u32,
    high_priority: u32,
    ddc: u32,
    duc: u32,
    tx_iq: u32,
}

fn next_seq(v: &mut u32) -> u32 {
    let n = *v;
    *v = v.wrapping_add(1);
    n
}

/// How long an enabled DDC may go without IQ — while not transmitting — before
/// the thread re-states the DDC configuration and run command. A board can
/// stop (or never start) one stream while the connection is otherwise healthy;
/// nothing else would notice, because liveness is per stream and reconnecting
/// the whole radio would take every other radio's DDC down with it.
const DDC_STARVE_AFTER: Duration = Duration::from_secs(2);
/// How often starved DDCs are nudged while they stay silent.
const DDC_KICK_EVERY: Duration = Duration::from_secs(3);

/// Run the Protocol 2 network loop until told to shut down.
pub(crate) fn run(ctx: ThreadCtx) {
    let mut t = P2Thread {
        socket: ctx.socket,
        radio: ctx.radio,
        opened_at: ctx.opened_at,
        conn_id: ctx.conn_id,
        invert_spectrum: ctx.invert_spectrum,
        rate_khz: (ctx.rate_hz / 1000.0) as u16,
        slots: std::collections::HashMap::new(),
        tx: ctx.tx,
        ctrl: ctx.ctrl,
        seq: Seq::default(),
        tx_freq: 7_100_000.0,
        ptt: false,
        oc: ctx.oc,
        band_dial: None,
        last_kick: None,
        kick_logged: false,
        // The two-ADC boards drive a second Alex chain; the rest have one.
        // Same test piHPSDR makes, and it is a board fact, not a setting.
        alex_both: matches!(ctx.board.as_str(), "Saturn" | "Orion2"),
    };
    t.run();
}

/// One attached DDC's state: its ring, its liveness clock (shared with the
/// `HpsdrRx` that reads it), its NCO frequency and its own datagram sequence —
/// Protocol 2 counts per stream, so a shared counter would report phantom
/// gaps whenever the streams interleave.
struct P2Slot {
    ring: Producer<f32>,
    last_rx_ms: crate::net::RxClock,
    freq_hz: f64,
    seq_in: SeqTracker,
    /// Whether this DDC's ring is still holding the backlog of an over: the
    /// engine stops reading its receiver while keyed and drains the ring on
    /// unkey (`discard_pending_rx`), so the discards either side of that
    /// moment belong to the over rather than to a host that fell behind. A
    /// datagram taken is the proof the reader is back.
    tx_backlog: bool,
    /// Set while this DDC's own engine is transmitting and therefore not
    /// reading it — see `HpsdrRx::set_rx_paused`. Separate from `self.ptt`,
    /// which is this *board* keying: a DDC lent to another rig as a panadapter
    /// goes unread for an over that MOX here knows nothing about.
    rx_paused: Arc<AtomicBool>,
}

struct P2Thread {
    socket: UdpSocket,
    radio: IpAddr,
    /// Epoch for every slot's liveness clock (see `HpsdrRx::silent_for`).
    opened_at: Instant,
    /// This connection's ownership ticket (see `net::owns_connection`).
    conn_id: u64,
    /// Conjugate I/Q both ways (see `HpsdrConfig::invert_spectrum`).
    invert_spectrum: bool,
    rate_khz: u16,
    /// The attached DDCs, by wire index.
    slots: std::collections::HashMap<u8, P2Slot>,
    tx: Consumer<f32>,
    ctrl: Receiver<Ctrl>,
    seq: Seq,
    tx_freq: f64,
    ptt: bool,
    /// How the seven open-collector outputs are driven (issue #296).
    oc: HpsdrOcPlan,
    /// Where the operator's dial is, when a transverter has put the radio
    /// somewhere else — the frequency the accessory board's decoder has to
    /// switch for. `None` when nothing is in front of the radio, which is when
    /// the DDC frequency *is* the band. Exactly `Regs::band_dial`'s job on the
    /// Protocol 1 side.
    band_dial: Option<f64>,
    /// The starved-DDC watchdog's rate limit and one-shot log flag.
    last_kick: Option<Instant>,
    kick_logged: bool,
    /// Whether this board has the second Alex/filter chain (Orion2, Saturn).
    alex_both: bool,
}

impl P2Thread {
    fn dest(&self, p: u16) -> SocketAddr {
        SocketAddr::new(self.radio, p)
    }

    /// Send the high-priority packet with the run bit set — the packet that
    /// keeps the radio streaming, and the one that starts it.
    fn send_high_priority(&mut self) {
        self.send_high_priority_run(true);
    }

    fn send_high_priority_run(&mut self, run: bool) {
        let seq = next_seq(&mut self.seq.high_priority);
        let mut phases = [0u32; MAX_DDCS as usize];
        for (&ddc, slot) in &self.slots {
            phases[ddc as usize] = phase_word(slot.freq_hz, CLOCK_HZ);
        }
        let tx = phase_word(self.tx_freq, CLOCK_HZ);
        let drive = if self.ptt { TX_DRIVE } else { 0 };
        let oc = self.oc_word();
        let pkt = high_priority_packet(seq, &phases, tx, run, self.ptt, drive, oc);
        tracing::trace!(
            "HPSDR P2: high-priority seq {seq}: run {run}, {} DDC NCO(s), TX phase 0x{tx:08X} \
             ({:.0} Hz), MOX {}, drive {drive}, OC 0x{oc:02X}",
            self.slots.len(),
            self.tx_freq,
            self.ptt
        );
        let _ = self.socket.send_to(&pkt, self.dest(port::HIGH_PRIORITY));
    }

    /// The seven open-collector outputs for wherever this radio is working:
    /// the transmit frequency while keyed — the low-pass filter has to match
    /// what is actually going out — and the receive frequency otherwise.
    ///
    /// The dial wins where there is one: with a 2 m transverter in front, the
    /// DDC says 28 MHz and the filters, relays and transverter the decoder
    /// switches all belong to 144 (issue #278). Without a dial it is DDC 0's
    /// frequency, because DDC 0 is the receiver that owns the transmitter; a
    /// second radio tab on another DDC is a panadapter and does not get to
    /// throw the station's relays.
    fn oc_word(&self) -> u8 {
        let freq = match self.band_dial {
            Some(hz) => hz,
            None if self.ptt => self.tx_freq,
            None => match self.slots.get(&0) {
                Some(s) => s.freq_hz,
                // No DDC 0 attached: the lowest one there is, so a board driven
                // by a panadapter-only tab still switches for what it hears
                // rather than for 7.1 MHz.
                None => self
                    .slots
                    .iter()
                    .min_by_key(|(d, _)| **d)
                    .map_or(self.tx_freq, |(_, s)| s.freq_hz),
            },
        };
        self.oc.word(freq, self.ptt)
    }

    fn send_ddc_command(&mut self) {
        let seq = next_seq(&mut self.seq.ddc);
        let mut ddcs: Vec<u8> = self.slots.keys().copied().collect();
        ddcs.sort_unstable();
        let pkt = ddc_command_packet(seq, self.rate_khz, &ddcs);
        tracing::debug!(
            "HPSDR P2: DDC command seq {seq}: DDCs {ddcs:?} enabled, {} kHz, 24-bit, ADC0 -> \
             port {}",
            self.rate_khz,
            port::DDC_COMMAND
        );
        let _ = self.socket.send_to(&pkt, self.dest(port::DDC_COMMAND));
    }

    /// Re-state the DDC configuration when an enabled stream has gone silent —
    /// see [`DDC_STARVE_AFTER`]. Skipped while keyed: a board may legitimately
    /// pause RX during its own TX, and unkey refreshes the clocks.
    fn kick_starved_ddcs(&mut self) {
        if self.ptt || self.slots.is_empty() {
            return;
        }
        let now_ms = self.opened_at.elapsed().as_millis() as u64;
        let starved: Vec<u8> = self
            .slots
            .iter()
            .filter(|(_, s)| {
                now_ms.saturating_sub(s.last_rx_ms.load(Ordering::Relaxed))
                    > DDC_STARVE_AFTER.as_millis() as u64
            })
            .map(|(&ddc, _)| ddc)
            .collect();
        if starved.is_empty() {
            self.kick_logged = false;
            self.last_kick = None;
            return;
        }
        if self.last_kick.is_some_and(|t| t.elapsed() < DDC_KICK_EVERY) {
            return;
        }
        if !self.kick_logged {
            self.kick_logged = true;
            tracing::warn!(
                "HPSDR P2: DDC(s) {starved:?} stopped streaming; re-stating the DDC \
                 configuration and run command"
            );
        }
        self.last_kick = Some(Instant::now());
        self.send_general();
        self.send_ddc_command();
        self.send_high_priority();
    }

    fn send_duc_command(&mut self) {
        let seq = next_seq(&mut self.seq.duc);
        let pkt = duc_command_packet(seq);
        tracing::debug!("HPSDR P2: DUC command seq {seq} -> port {}", port::DUC_COMMAND);
        let _ = self.socket.send_to(&pkt, self.dest(port::DUC_COMMAND));
    }

    /// See `protocol1::stop_stream`: a superseded connection leaves the radio
    /// streaming to whichever connection replaced it.
    fn stop_stream(&mut self, why: &str) {
        if crate::net::owns_connection(self.radio, self.conn_id) {
            tracing::info!("HPSDR P2: {why}; stopping the radio's stream");
            // The run bit, cleared. Nothing on the General port stops a
            // Protocol 2 radio, which is why one left running kept answering
            // discovery as "in use" and every reconnect found its own last
            // session in the way (issue #281).
            self.ptt = false;
            self.send_high_priority_run(false);
        } else {
            tracing::info!(
                "HPSDR P2: {why}; another connection has taken over this radio, leaving its \
                 stream running"
            );
        }
    }

    fn send_general(&mut self) {
        let seq = next_seq(&mut self.seq.general);
        let pkt = general_packet(seq, self.alex_both);
        let _ = self.socket.send_to(&pkt, self.dest(port::GENERAL));
    }

    fn run(&mut self) {
        tracing::info!(
            "HPSDR P2: stream starting to {} at {} kHz; sending DDC/DUC/high-priority config \
             then run command (ports {}/{}/{}/{})",
            self.radio,
            self.rate_khz,
            port::DDC_COMMAND,
            port::DUC_COMMAND,
            port::HIGH_PRIORITY,
            port::GENERAL,
        );
        // Stop before starting, for the reason `protocol1::run` gives at
        // length: a board left streaming to a host that has gone answers
        // discovery as "in use" and ignores a fresh start, which is the
        // endless connect-and-lose-it of issue #365. One packet on a board
        // that was already idle.
        self.send_high_priority_run(false);
        std::thread::sleep(Duration::from_millis(100));
        // The order and the pauses are the protocol's, not a style: the
        // General packet carries the port table and the board switches and has
        // to land before anything is configured against them, and each step
        // needs a moment in the FPGA before the next one arrives. The run
        // command comes last, once there is something configured to run.
        self.send_general();
        std::thread::sleep(Duration::from_millis(100));
        self.send_ddc_command();
        std::thread::sleep(Duration::from_millis(50));
        self.send_duc_command();
        std::thread::sleep(Duration::from_millis(50));
        self.send_high_priority();
        tracing::debug!(
            "HPSDR P2: run command sent; awaiting DDC I/Q on source port {}..{}",
            port::DDC_IQ_BASE,
            port::DDC_IQ_BASE + 7
        );

        let mut last_watchdog = Instant::now();
        let mut rx_scratch: Vec<f32> = Vec::with_capacity(512);
        let mut tx_scratch: Vec<f32> = Vec::with_capacity(DUC_SAMPLES_PER_PKT * 2);
        let mut buf = [0u8; 2048];
        let mut stats = RxStats::new(2, self.rate_khz as f64 * 1000.0);
        let mut logged_first_rx = false;
        let mut logged_first_tx = false;
        let mut warned_no_rx = false;
        let started = Instant::now();
        // When the run bit was last asserted, so it can be repeated while the
        // board has yet to answer with any I/Q — see the retry below.
        let mut last_run_cmd = Instant::now();

        loop {
            let mut freq_changed = false;
            while let Ok(msg) = self.ctrl.try_recv() {
                match msg {
                    Ctrl::Attach { ddc, ring, last_rx_ms, rx_paused } => {
                        self.slots.insert(
                            ddc,
                            P2Slot {
                                ring,
                                last_rx_ms,
                                freq_hz: 7_100_000.0,
                                seq_in: SeqTracker::new(),
                                tx_backlog: false,
                                rx_paused,
                            },
                        );
                        // The whole table, freshly stated — never an edit.
                        self.send_ddc_command();
                        freq_changed = true;
                        tracing::info!(ddc, "HPSDR P2: DDC attached");
                    }
                    Ctrl::Detach { ddc } => {
                        if self.slots.remove(&ddc).is_some() {
                            if ddc == 0 && self.ptt {
                                // The stream that owns the transmitter is
                                // going away mid-over: unkey rather than
                                // leaving the board on the air unattended.
                                self.ptt = false;
                            }
                            self.send_ddc_command();
                            freq_changed = true;
                            tracing::info!(ddc, "HPSDR P2: DDC detached");
                        }
                    }
                    Ctrl::RxFreq { ddc, hz } => {
                        if let Some(s) = self.slots.get_mut(&ddc) {
                            s.freq_hz = hz;
                            freq_changed = true;
                            tracing::debug!("HPSDR P2: DDC{ddc} NCO -> {hz:.0} Hz");
                        }
                    }
                    // Protocol 2 boards have no front-end gain register this
                    // crate drives; the DDC command carries no gain field.
                    Ctrl::RxGain(_) => {}
                    // The HL2IOBoard is a Hermes-Lite accessory on Protocol 1's
                    // I2C tunnel; nothing here to switch.
                    Ctrl::IoRxInput(_) => {}
                    // The open collectors follow the dial here exactly as they
                    // do on Protocol 1: a band decoder switches for the signal
                    // on the air, not for the I.F. a transverter left the radio
                    // on (issue #278, and #296 for driving them at all).
                    Ctrl::BandDial(hz) => {
                        if self.band_dial != hz {
                            self.band_dial = hz;
                            freq_changed = true;
                        }
                    }
                    // Load the DUC ahead of key-down. Nothing on a Protocol 2
                    // board acts on this until MOX — there is no accessory bus
                    // here of the kind the Hermes-Lite has — but keeping the
                    // register current means key-down needs no retune.
                    Ctrl::TxFreq(hz) => {
                        if self.tx_freq != hz {
                            self.tx_freq = hz;
                            self.send_duc_command();
                            tracing::debug!("HPSDR P2: TX NCO -> {hz:.0} Hz (not keyed)");
                        }
                    }
                    Ctrl::TxOn(hz) => {
                        self.tx_freq = hz;
                        self.ptt = true;
                        self.send_duc_command();
                        freq_changed = true;
                        tracing::info!("HPSDR P2: MOX on, TX NCO {hz:.0} Hz");
                    }
                    Ctrl::TxOff => {
                        self.ptt = false;
                        freq_changed = true;
                        // The watchdog was paused for the over and the board
                        // may legitimately have paused RX during it: every
                        // stream gets a fresh grace period.
                        let now_ms = self.opened_at.elapsed().as_millis() as u64;
                        for slot in self.slots.values_mut() {
                            slot.last_rx_ms.store(now_ms, Ordering::Relaxed);
                        }
                        tracing::info!("HPSDR P2: MOX off");
                    }
                    Ctrl::Shutdown => {
                        self.stop_stream("shutdown requested");
                        return;
                    }
                }
            }
            if freq_changed {
                self.send_high_priority();
            }

            match self.socket.recv_from(&mut buf) {
                Ok((n, src)) => {
                    let p = src.port();
                    let ddc = p.checked_sub(port::DDC_IQ_BASE).filter(|&d| d < MAX_DDCS as u16);
                    // MOX is connection-wide but only DDC 0 owns the
                    // transmitter, so only DDC 0's reader stops reading for the
                    // over. A second radio tab on another DDC keeps receiving
                    // through it, and a full ring there is a real overrun.
                    let ptt = self.ptt && ddc == Some(0);
                    if let Some(slot) = ddc.and_then(|d| self.slots.get_mut(&(d as u8))) {
                        rx_scratch.clear();
                        if let Some(pairs) = decode_ddc_iq(&buf[..n], &mut rx_scratch) {
                            if self.invert_spectrum {
                                crate::protocol1::conjugate(&mut rx_scratch);
                            }
                            stats.on_iq(pairs);
                            slot.last_rx_ms.store(
                                self.opened_at.elapsed().as_millis() as u64,
                                Ordering::Relaxed,
                            );
                            if !logged_first_rx {
                                logged_first_rx = true;
                                let declared = if n >= 16 {
                                    u16::from_be_bytes([buf[14], buf[15]])
                                } else {
                                    0
                                };
                                tracing::info!(
                                    "HPSDR P2: first DDC I/Q from src port {p} — {n} bytes, \
                                     header declares {declared} samples, decoded {pairs} pairs \
                                     [{}]",
                                    hex_head(&buf[..n], 16)
                                );
                            }
                            // Sequence counters are per stream: each DDC's
                            // datagrams count on their own, so interleaved
                            // streams never read as phantom gaps.
                            if n >= 4 {
                                let seq = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
                                stats.on_lost(slot.seq_in.observe(seq) as u64);
                            }
                            let keyed =
                                ptt || slot.rx_paused.load(Ordering::Relaxed) || slot.tx_backlog;
                            slot.tx_backlog =
                                !push_iq(&mut slot.ring, &rx_scratch, &mut stats, keyed) && keyed;
                        } else {
                            stats.on_other();
                            tracing::trace!(
                                "HPSDR P2: undecodable DDC datagram from port {p} ({n} bytes)"
                            );
                        }
                    } else if ddc.is_some() {
                        // A DDC nobody attached — the board streaming one we
                        // just disabled, most likely. Not an error.
                        stats.on_other();
                        tracing::trace!("HPSDR P2: I/Q from unattached DDC port {p} ({n} bytes)");
                    } else {
                        stats.on_other();
                        tracing::trace!(
                            "HPSDR P2: {n}-byte datagram from unexpected src port {p} [{}]",
                            hex_head(&buf[..n], 8)
                        );
                    }
                }
                Err(ref e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => {
                    self.stop_stream(&format!("recv error: {e}"));
                    return;
                }
            }

            // A run bit the board never acted on is worth asserting again
            // before the whole connection is torn down and rebuilt around it:
            // the DDC and DUC configuration is already in the gateware, so a
            // second ask is one packet against a five-second reconnect
            // (issue #365).
            if !logged_first_rx
                && !self.slots.is_empty()
                && last_run_cmd.elapsed() >= Duration::from_secs(1)
            {
                last_run_cmd = Instant::now();
                self.send_high_priority();
                tracing::debug!("HPSDR P2: still no DDC I/Q — run command sent again");
            }

            // Flag a radio that accepted the run command but never streams I/Q.
            if !logged_first_rx
                && !warned_no_rx
                && !self.slots.is_empty()
                && started.elapsed() >= Duration::from_secs(3)
            {
                warned_no_rx = true;
                tracing::warn!(
                    "HPSDR P2: no DDC I/Q datagrams after 3 s, and the run command has been sent \
                     again since. Expected them on source port {}..{}. Check that these UDP \
                     ports are not blocked by a firewall, that the radio is idle (another \
                     program holding it shows as \"in use\" at discovery), and that the \
                     DDC-command offsets match this board.",
                    port::DDC_IQ_BASE,
                    port::DDC_IQ_BASE + 7
                );
            }
            self.kick_starved_ddcs();
            stats.tick();

            if self.ptt {
                while let Ok(v) = self.tx.pop() {
                    tx_scratch.push(v);
                    if tx_scratch.len() >= DUC_SAMPLES_PER_PKT * 2 {
                        if self.invert_spectrum {
                            crate::protocol1::conjugate(&mut tx_scratch);
                        }
                        let seq = next_seq(&mut self.seq.tx_iq);
                        let pkt = duc_iq_packet(seq, &tx_scratch);
                        if !logged_first_tx {
                            logged_first_tx = true;
                            tracing::info!(
                                "HPSDR P2: first DUC I/Q datagram sent — seq {seq}, {} samples \
                                 -> port {}",
                                DUC_SAMPLES_PER_PKT,
                                port::DUC_IQ
                            );
                        }
                        let _ = self.socket.send_to(&pkt, self.dest(port::DUC_IQ));
                        tx_scratch.clear();
                    }
                }
            } else if !tx_scratch.is_empty() {
                tx_scratch.clear();
                logged_first_tx = false;
            }

            if last_watchdog.elapsed() >= WATCHDOG {
                self.send_high_priority();
                self.send_general();
                last_watchdog = Instant::now();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_word_math() {
        // Half the clock → top bit set.
        assert_eq!(phase_word(CLOCK_HZ / 2.0, CLOCK_HZ), 0x8000_0000);
        // DC → 0.
        assert_eq!(phase_word(0.0, CLOCK_HZ), 0);
        // 14.074 MHz on the 122.88 MHz clock (known ballpark).
        let p = phase_word(14_074_000.0, CLOCK_HZ);
        let back = p as f64 / 4_294_967_296.0 * CLOCK_HZ;
        assert!((back - 14_074_000.0).abs() < 1.0, "round-trips within 1 Hz, got {back}");
    }

    #[test]
    fn be24_roundtrip() {
        for v in [0, 1, -1, 8_388_607, -8_388_608, 1234, -4321] {
            assert_eq!(be24_to_i32(i32_to_be24(v)), v, "roundtrip {v}");
        }
        // Big-endian byte order.
        assert_eq!(i32_to_be24(0x123456), [0x12, 0x34, 0x56]);
        // Sign extension.
        assert_eq!(be24_to_i32([0xFF, 0xFF, 0xFF]), -1);
        assert_eq!(be24_to_i32([0x80, 0x00, 0x00]), -8_388_608);
    }

    #[test]
    fn f32_be24_roundtrip() {
        for x in [0.0f32, 0.5, -0.5, 0.999, -0.999] {
            let back = be24_to_f32(f32_to_be24(x));
            assert!((back - x).abs() < 1e-4, "roundtrip {x} -> {back}");
        }
        // Clamps and never overflows at the rails.
        let _ = f32_to_be24(2.0);
        let _ = f32_to_be24(-2.0);
    }

    #[test]
    fn ddc_iq_roundtrip_via_duc_encoder() {
        // Encode two IQ pairs into DUC form, then decode as a DDC packet by
        // prepending a 16-byte header with the right sample count. This exercises
        // both the encoder and decoder against each other.
        let iq = [0.25f32, -0.5, 0.75, -0.125];
        let duc = duc_iq_packet(7, &iq);
        // Build a DDC-shaped packet: 16-byte header (count=2) + the 12 IQ bytes.
        let mut ddc = vec![0u8; DDC_IQ_HEADER_LEN];
        ddc[14] = 0;
        ddc[15] = 2;
        ddc.extend_from_slice(&duc[4..4 + 12]);
        let mut out = Vec::new();
        assert_eq!(decode_ddc_iq(&ddc, &mut out), Some(2));
        assert_eq!(out.len(), 4);
        for (a, b) in out.iter().zip(iq.iter()) {
            assert!((a - b).abs() < 1e-4, "{a} vs {b}");
        }
    }

    #[test]
    fn high_priority_field_placement() {
        let rx0 = phase_word(7_100_000.0, CLOCK_HZ);
        let rx1 = phase_word(14_074_000.0, CLOCK_HZ);
        let mut phases = [0u32; MAX_DDCS as usize];
        phases[0] = rx0;
        phases[1] = rx1;
        let tx = phase_word(7_100_000.0, CLOCK_HZ);
        let b = high_priority_packet(0, &phases, tx, true, true, 200, 0);
        assert_eq!(b[4] & 0x02, 0x02, "MOX bit set");
        // The DDC frequency table: 4 bytes per DDC from offset 9.
        assert_eq!(u32::from_be_bytes([b[9], b[10], b[11], b[12]]), rx0);
        assert_eq!(u32::from_be_bytes([b[13], b[14], b[15], b[16]]), rx1);
        assert_eq!(u32::from_be_bytes([b[17], b[18], b[19], b[20]]), 0, "DDC2 unused");
        assert_eq!(u32::from_be_bytes([b[329], b[330], b[331], b[332]]), tx);
        assert_eq!(b[345], 200);
        assert_eq!(b[HP_OC], 0, "no filter board configured: every output off");
    }

    /// The open-collector byte, which Protocol 2 never used to send at all —
    /// so a Protocol 2 station could not switch a filter board, an antenna
    /// relay or an amplifier's band decoder from sdroxide however the filter
    /// board setting was left (issue #296).
    ///
    /// Same convention as Protocol 1's C2: seven outputs in bits 7..1, bit 0
    /// unused, so the word goes out shifted up by one.
    #[test]
    fn the_open_collector_word_lands_in_the_high_priority_packet() {
        let phases = [0u32; MAX_DDCS as usize];
        // Outputs 1, 3 and 7.
        let b = high_priority_packet(0, &phases, 0, true, false, 0, 0b100_0101);
        assert_eq!(b[HP_OC], 0b1000_1010, "outputs 1, 3 and 7, shifted off bit 0");
        assert_eq!(HP_OC, 1400);
        // Nothing else in the packet moved with it.
        assert_eq!(b[345], 0);
        assert!(b[1401..].iter().all(|&x| x == 0), "nothing written past the OC byte");
        // There is no eighth output: bit 7 of the word cannot reach the wire.
        let b = high_priority_packet(0, &phases, 0, true, false, 0, 0xFF);
        assert_eq!(b[HP_OC], 0xFE);
    }

    /// Byte 4 of a 60-byte datagram to port 1024 is the packet *type*, and
    /// `0x00` is what makes one a General packet. Issue #281: sdroxide put a
    /// run bit there, the radio saw a packet type it did not know and dropped
    /// every one — port table, phase-word mode bit, watchdog and PA/Alex
    /// switches with it.
    #[test]
    fn the_general_packet_is_always_of_the_general_type() {
        for alex_both in [false, true] {
            let b = general_packet(7, alex_both);
            assert_eq!(b[4], 0x00, "packet type must stay General");
            assert_eq!(u32::from_be_bytes([b[0], b[1], b[2], b[3]]), 7, "sequence");
            assert_eq!(b[37], 0x08, "NCO fields carry phase words");
            assert_eq!(b[38], 0x01, "hardware watchdog");
            assert_eq!(b[58], 0x01, "PA enable");
            // Everything the protocol reads as a port number is left at zero,
            // which is what asks for the defaults this crate listens on.
            assert!(b[5..23].iter().all(|&x| x == 0), "port table left at its defaults");
        }
        // The two-ADC boards drive a second filter chain.
        assert_eq!(general_packet(0, false)[59], 0x01);
        assert_eq!(general_packet(0, true)[59], 0x03);
    }

    /// The run bit is the high-priority packet's, and clearing it is the only
    /// way to stop a Protocol 2 radio — a session that never cleared it left
    /// the radio answering discovery as "in use" (issue #281).
    #[test]
    fn the_run_bit_lives_in_the_high_priority_packet() {
        let phases = [0u32; MAX_DDCS as usize];
        let running = high_priority_packet(0, &phases, 0, true, false, 0, 0);
        assert_eq!(running[4] & 0x01, 0x01, "run");
        assert_eq!(running[4] & 0x02, 0x00, "not keyed");
        let stopped = high_priority_packet(1, &phases, 0, false, false, 0, 0);
        assert_eq!(stopped[4], 0x00, "a stop is the run bit cleared, and nothing else");
        // MOX rides beside it either way.
        assert_eq!(high_priority_packet(2, &phases, 0, true, true, 0, 0)[4], 0x03);
    }

    #[test]
    fn ddc_command_encodes_rate() {
        // DDC0 alone: byte-for-byte what the single-receiver code always sent.
        let b = ddc_command_packet(0, 1536, &[0]);
        assert_eq!(b[7], 0x01);
        assert_eq!(u16::from_be_bytes([b[18], b[19]]), 1536);
        assert_eq!(b[22], 24);
    }

    #[test]
    fn ddc_command_encodes_the_whole_table() {
        let b = ddc_command_packet(0, 384, &[0, 2]);
        assert_eq!(b[7], 0b0000_0101, "enable mask: DDC0 + DDC2");
        // DDC0 descriptor at 17, DDC2's at 17 + 2×6 = 29; DDC1's untouched.
        assert_eq!(u16::from_be_bytes([b[18], b[19]]), 384);
        assert_eq!(b[22], 24);
        assert_eq!(u16::from_be_bytes([b[24], b[25]]), 0, "DDC1 rate stays zero");
        assert_eq!(b[28], 0, "DDC1 bits stay zero");
        assert_eq!(u16::from_be_bytes([b[30], b[31]]), 384);
        assert_eq!(b[34], 24);
        // An out-of-range index is ignored, not a corrupted write.
        let b = ddc_command_packet(0, 384, &[9]);
        assert_eq!(b[7], 0, "no DDC enabled");
    }
}
