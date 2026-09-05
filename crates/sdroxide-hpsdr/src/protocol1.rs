//! OpenHPSDR Protocol 1 ("old protocol" / Metis / Ozy) — enough for RX + SSB TX,
//! which covers Hermes-Lite 2 and legacy Metis/Hermes boards.
//!
//! Everything runs over UDP port 1024. Data datagrams are 1032 bytes:
//! `0xEF 0xFE 0x01 <endpoint>` + a 4-byte big-endian sequence, then two 512-byte
//! "OZY" frames. Each frame is `0x7F 0x7F 0x7F` + five command-and-control bytes
//! (C0..C4) + 504 bytes of data (63 samples × 8 bytes for one receiver).
//!
//! - RX (endpoint 6, radio→host): each sample is I (24-bit signed big-endian) +
//!   Q (24-bit) + mic (16-bit) = 8 bytes. EP6 arrives at the DDC rate.
//! - TX (endpoint 2, host→radio): each sample is L + R audio (16-bit) + TX I +
//!   TX Q (16-bit signed) = 8 bytes. We send zero audio and the modulator's I/Q.
//!   EP2 is a **fixed 48 kHz** stream in both directions of the protocol —
//!   the radio drains it at 48 ksps whatever the DDC rate is, so it must be
//!   paced by the clock, not by how fast EP6 comes back.
//!
//! The C0..C4 registers are round-robined on outgoing frames. C0 bit 0 is MOX
//! (PTT); C0 bits 7..1 are the register address. Frequencies are the actual
//! value in **Hz** (32-bit big-endian in C1..C4), not a phase word.
//!
//! Incoming EP6 frames carry C&C the other way: C0 bits 2..0 are the hardware
//! PTT/dash/dot lines and bits 7..3 select which sensor set C1..C4 hold.
//!
//! Offsets follow the g0orx/rustyHPSDR reference and the OpenHPSDR/Hermes-Lite 2
//! protocol docs; verify against hardware before trusting on-air behavior.

use std::collections::VecDeque;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use crate::net::{
    Ctrl, RxStats, SeqTracker, ThreadCtx, board_has_lna_gain, board_is_hermes_lite, hex_head,
    lna_gain_code, push_iq,
};
use crate::protocol2::be24_to_f32;
use sdroxide_types::HpsdrOcPlan;
#[cfg(test)]
use sdroxide_types::{HpsdrFilterBoard, hpsdr_alex_oc as alex_oc, hpsdr_n2adr_oc as n2adr_oc};

const PORT: u16 = 1024;
/// Samples per 512-byte OZY frame for one receiver (504 data bytes / 8).
const SAMPLES_PER_FRAME: usize = 63;
/// Sample-pairs carried by one datagram (two frames).
const SAMPLES_PER_DATAGRAM: usize = SAMPLES_PER_FRAME * 2;
/// Two frames per datagram → 126 sample-pairs = 252 interleaved floats.
const FLOATS_PER_DATAGRAM: usize = SAMPLES_PER_DATAGRAM * 2;
const DATAGRAM_LEN: usize = 8 + 2 * 512;

/// EP2 is a fixed 48 kHz stream, so one datagram is due every
/// 126 / 48000 s = 2.625 ms no matter what the DDC rate is.
const EP2_INTERVAL: Duration = Duration::from_nanos(2_625_000);

/// C0 register bytes (address already shifted into bits 7..1; OR in the MOX
/// bit 0). Values confirmed against the rustyHPSDR reference.
const CC_CONFIG: u8 = 0x00; // frame #1 of every datagram: C1[1:0]=rate, C4=duplex|rx-count
const CC_TX_FREQ: u8 = 0x02; // C1..C4 = TX NCO frequency (Hz)
const CC_RX1_FREQ: u8 = 0x04; // C1..C4 = RX1 NCO frequency (Hz)
const CC_DRIVE: u8 = 0x12; // C1 = TX drive level 0..255
/// Register `0x09`, C2 bit 3 — the register's 32-bit view calls it bit 19.
/// On a Hermes-Lite it switches the **onboard power amplifier** on. With it
/// clear the board still keys (the T/R relay throws, the PTT line and any
/// accessory board follow) and puts out nothing at the antenna jack, which is
/// exactly what "PTT works, no power" looks like from the operating position.
/// Every other Protocol 1 board reads this bit as an Apollo tuner command, so
/// it is only ever sent to a Hermes-Lite.
const HL2_PA_ON: u8 = 0x08;
/// Register `0x09`, C2 bit 2 (bit 18 of the 32-bit view): with the PA off,
/// hold the T/R relay in receive so the antenna connector stays on the
/// receiver — transmit then appears at the low-power RF1 output, which is how
/// an external amplifier is driven.
const HL2_TR_RX_ONLY: u8 = 0x04;
/// Register 0x0A. On a Hermes-Lite 2 its C4 is the AD9866 front-end gain:
/// bit 6 marks the field valid, bits 5..0 are `dB + 12` (so 0 = −12 dB,
/// 60 = +48 dB). Other Protocol 1 boards use this register for Alex and
/// attenuator settings, so it is only ever sent to a Hermes-Lite.
const CC_HL2_GAIN: u8 = 0x14;
/// C4 bit that tells a Hermes-Lite the gain field below it is meaningful.
const HL2_GAIN_VALID: u8 = 0x40;

/// Fixed TX drive; the engine already scales the I/Q amplitude in software.
const TX_DRIVE: u8 = 255;
/// Config C4: duplex on (bit 2) so RX keeps streaming during TX, plus
/// `(receivers - 1) << 3` in bits 5:3 — one receiver → 0.
const CONFIG_C4: u8 = 0x04;

/// Sample-rate code for the config register (48/96/192/384 kHz → 0..3).
fn speed_code(rate_hz: f64) -> u8 {
    match rate_hz.round() as u32 {
        r if r <= 48_000 => 0,
        r if r <= 96_000 => 1,
        r if r <= 192_000 => 2,
        _ => 3,
    }
}

/// `-1.0..=1.0` float → 16-bit big-endian sample.
fn i16_be(x: f32) -> [u8; 2] {
    ((x.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_be_bytes()
}

/// The config register (C0 address 0). rustyHPSDR sends this as frame #1 of
/// *every* datagram: C1 = sample-rate code, C2 = the open-collector outputs,
/// C4 = duplex | receiver-count.
///
/// `oc` is the seven open-collector lines as the accessory board sees them
/// (bit 0 = output 1). In the Hermes-Lite's 32-bit view of this register they
/// live at bits [23:17], which is C2 bits [7:1] — hence the shift — and the
/// gateware forwards them over I2C to a filter board's MCP23008.
fn config_cc(speed: u8, mox: u8, oc: u8) -> [u8; 5] {
    [CC_CONFIG | mox, speed, (oc & 0x7F) << 1, 0, CONFIG_C4]
}

/// Everything the rotating register slots need. `lna_gain` and `pa` are `None`
/// on boards whose Hermes-Lite-specific register fields we must not touch.
#[derive(Clone, Copy)]
struct Regs {
    rx_freq: u32,
    tx_freq: u32,
    /// Where the operator's dial is, when a transverter has put the radio
    /// somewhere else — the frequency the accessory board's decoder has to
    /// switch for. `None` when nothing is in front of the radio, which is when
    /// the NCO frequency *is* the band.
    band_dial: Option<f64>,
    lna_gain: Option<f64>,
    /// `Some(true)` to run the Hermes-Lite's onboard PA, `Some(false)` to leave
    /// it off and keep the antenna jack on receive, `None` on a board that is
    /// not a Hermes-Lite (its register 0x09 C2 means Apollo/Alex things and is
    /// left at zero, as before).
    pa: Option<bool>,
    /// The seven open-collector outputs, resolved for this connection — a
    /// preset's convention or the operator's own per-band table (issue #296).
    oc: HpsdrOcPlan,
    ptt: bool,
}

/// Rotating-register slots, sent one per datagram in frame #2. At the EP2 rate
/// a full rotation takes ~10 ms, so a changed value is also queued as urgent
/// (see [`Rotation::urge`]) rather than waiting its turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    TxFreq,
    RxFreq,
    Drive,
    LnaGain,
}

impl Regs {
    /// The seven open-collector outputs for the attached accessory board, or 0
    /// when none is configured. Follows the transmit frequency while keyed —
    /// the low-pass filter has to match what is actually going out — and the
    /// receive frequency otherwise.
    fn oc(&self) -> u8 {
        // The dial wins where there is one: with a 2 m transverter in front,
        // the NCO says 28 MHz and the filters, relays and transverter the
        // decoder switches all belong to 144 (issue #278).
        let freq = match self.band_dial {
            Some(hz) => hz,
            None => (if self.ptt { self.tx_freq } else { self.rx_freq }) as f64,
        };
        self.oc.word(freq, self.ptt)
    }

    /// C2 of the drive register (`0x09`). On a Hermes-Lite it carries the PA
    /// and T/R-relay bits; on anything else it stays zero — those bits are
    /// Apollo tuner/filter commands there, and asserting them would operate
    /// hardware the operator never asked about.
    ///
    /// The PA state is asserted continuously rather than only while keyed: the
    /// register rotation reaches this slot every fourth datagram, and the board
    /// must already know the answer at the moment MOX arrives.
    fn drive_c2(&self) -> u8 {
        match self.pa {
            Some(true) => HL2_PA_ON,
            Some(false) => HL2_TR_RX_ONLY,
            None => 0,
        }
    }

    /// The slots this board actually has, in rotation order.
    fn slots(&self) -> &'static [Slot] {
        if self.lna_gain.is_some() {
            &[Slot::TxFreq, Slot::RxFreq, Slot::Drive, Slot::LnaGain]
        } else {
            &[Slot::TxFreq, Slot::RxFreq, Slot::Drive]
        }
    }

    /// Encode one slot as its five C&C bytes, with the MOX bit riding along.
    fn cc(&self, slot: Slot) -> [u8; 5] {
        let mox = if self.ptt { 1 } else { 0 };
        match slot {
            Slot::TxFreq => freq_cc(CC_TX_FREQ, self.tx_freq, mox),
            Slot::RxFreq => freq_cc(CC_RX1_FREQ, self.rx_freq, mox),
            Slot::Drive => [CC_DRIVE | mox, TX_DRIVE, self.drive_c2(), 0, 0],
            Slot::LnaGain => {
                let code = lna_gain_code(self.lna_gain.unwrap_or(0.0));
                [CC_HL2_GAIN | mox, 0, 0, 0, HL2_GAIN_VALID | code]
            }
        }
    }
}

/// Conjugate interleaved I/Q in place — negate every Q — which mirrors the
/// spectrum about the tuned frequency. Applied to receive and transmit alike so
/// the two directions cannot end up on opposite sidebands.
pub(crate) fn conjugate(iq: &mut [f32]) {
    for q in iq.iter_mut().skip(1).step_by(2) {
        *q = -*q;
    }
}

fn freq_cc(addr: u8, freq: u32, mox: u8) -> [u8; 5] {
    let f = freq.to_be_bytes();
    [addr | mox, f[0], f[1], f[2], f[3]]
}

/// Write one 512-byte OZY frame: sync + C&C + 63 TX samples (zero audio + I/Q).
fn write_ozy_frame(frame: &mut [u8], cc: [u8; 5], tx_iq: &[f32]) {
    frame[0] = 0x7F;
    frame[1] = 0x7F;
    frame[2] = 0x7F;
    frame[3..8].copy_from_slice(&cc);
    for s in 0..SAMPLES_PER_FRAME {
        let base = 8 + s * 8;
        // bytes [base..base+4] = L/R audio, left zero.
        let i = tx_iq.get(2 * s).copied().unwrap_or(0.0);
        let q = tx_iq.get(2 * s + 1).copied().unwrap_or(0.0);
        frame[base + 4..base + 6].copy_from_slice(&i16_be(i));
        frame[base + 6..base + 8].copy_from_slice(&i16_be(q));
    }
}

/// Build an EP2 (host→radio) datagram: frame #1 is always the config register,
/// frame #2 carries `cc` — the next rotating register. Both frames also carry
/// 63 TX samples.
fn build_ep2(
    seq: &mut u32,
    speed: u8,
    mox: u8,
    oc: u8,
    cc: [u8; 5],
    tx_iq: &[f32],
) -> [u8; DATAGRAM_LEN] {
    let mut d = [0u8; DATAGRAM_LEN];
    d[0] = 0xEF;
    d[1] = 0xFE;
    d[2] = 0x01;
    d[3] = 0x02; // EP2
    d[4..8].copy_from_slice(&seq.to_be_bytes());
    *seq = seq.wrapping_add(1);

    // Frame #1: config register + TX samples 0..63.
    write_ozy_frame(&mut d[8..520], config_cc(speed, mox, oc), tx_iq);
    // Frame #2: rotating register + TX samples 63..126.
    let chunk = &tx_iq[(SAMPLES_PER_FRAME * 2).min(tx_iq.len())..];
    write_ozy_frame(&mut d[520..1032], cc, chunk);
    d
}

/// Picks which rotating register frame #2 carries. Values the operator just
/// changed jump the queue: at 381 datagrams/s a plain round-robin would take up
/// to ~10 ms to reach the radio, which is felt as lag on the tuning knob.
struct Rotation {
    next: usize,
    urgent: VecDeque<Slot>,
}

impl Rotation {
    fn new() -> Self {
        Rotation { next: 0, urgent: VecDeque::new() }
    }

    /// Send `slot` on the next datagram (de-duplicated: a knob being turned
    /// queues the same slot over and over).
    fn urge(&mut self, slot: Slot) {
        if !self.urgent.contains(&slot) {
            self.urgent.push_back(slot);
        }
    }

    fn take(&mut self, regs: &Regs) -> Slot {
        if let Some(s) = self.urgent.pop_front() {
            return s;
        }
        let slots = regs.slots();
        let s = slots[self.next % slots.len()];
        self.next = self.next.wrapping_add(1);
        s
    }
}

/// What an EP6 datagram told us besides I/Q.
struct Ep6Info {
    /// Datagram sequence number, for loss detection.
    seq: u32,
    /// The radio's own PTT line (hardware PTT, foot switch, mic button).
    ptt: bool,
    /// ADC clipped since the last report — the front-end gain is too high.
    adc_overload: bool,
    /// A Hermes-Lite acknowledging a request we marked RQST: the address it
    /// echoed and the four data bytes with it. Only the I2C tunnel asks for
    /// these (see [`crate::ioboard`]).
    ack: Option<(u8, [u8; 4])>,
    /// `(C1, C2, C3, C4)` of a status-set-0 frame: the firmware/gateware
    /// version bytes. Their exact meaning is board-specific, so they are logged
    /// raw rather than interpreted.
    versions: Option<(u8, u8, u8, u8)>,
    /// AIN5, the first analogue reading of a status-set-1 frame, as the raw
    /// 12-bit converter count. On a Hermes-Lite 2 this input carries the
    /// board's temperature sensor — see [`hl2_temperature_c`].
    ain5: Option<u16>,
}

/// Rate limiter for the two Hermes-Lite transmit faults worth shouting about.
/// Both can be reported on every datagram — 381 of them a second — so each gets
/// its own five-second gate rather than a shared one, which would let a noisy
/// fault hide a quiet one.
#[derive(Default)]
struct TxHealth {
    last_inhibit_warn: Option<Instant>,
    last_fifo_warn: Option<Instant>,
}

impl TxHealth {
    /// Whether `slot` has gone quiet long enough to warn again, stamping it if so.
    fn due(slot: &mut Option<Instant>) -> bool {
        if slot.is_none_or(|t| t.elapsed() >= Duration::from_secs(5)) {
            *slot = Some(Instant::now());
            return true;
        }
        false
    }
}

/// Interpret a Hermes-Lite's status set 0 for the two things that decide
/// whether a transmission is actually going out, and warn when either says it
/// is not. Fields are from the Hermes-Lite 2 protocol document, quoted here in
/// its 32-bit register view: C1 is `RDATA[31:24]`, C3 is `RDATA[15:8]`.
///
/// - `RDATA[25]` — TX inhibited, **active low**: an external inhibit input is
///   holding the transmitter off. The radio still keys and produces nothing.
/// - `RDATA[15]` — TX I/Q FIFO under/overflow recovery, with `RDATA[14]`
///   choosing between them (`0` under, `1` over). Underflow means our EP2
///   stream is not keeping the board's 48 kHz transmit FIFO fed and the signal
///   on the air is being chopped; overflow means we are sending it too fast.
///   Otherwise `RDATA[14:8]` is the FIFO's fill level.
fn hl2_status(c1: u8, c3: u8, keyed: bool, health: &mut TxHealth) {
    if c1 & 0x02 == 0 && TxHealth::due(&mut health.last_inhibit_warn) {
        tracing::warn!(
            "HPSDR P1: the Hermes-Lite reports TRANSMIT INHIBITED — something is holding its \
             external TX-inhibit input low. The board will key and make no power until that is \
             released."
        );
    }
    // The FIFO legitimately sits empty until the first over, so this is only a
    // fault while keyed.
    if keyed && c3 & 0x80 != 0 && TxHealth::due(&mut health.last_fifo_warn) {
        if c3 & 0x40 == 0 {
            tracing::warn!(
                "HPSDR P1: the Hermes-Lite's transmit FIFO UNDERRAN — EP2 datagrams are not \
                 reaching it at 48 kHz, so the transmitted signal is being chopped. A busy host \
                 or a lossy link is the usual cause."
            );
        } else {
            tracing::warn!(
                "HPSDR P1: the Hermes-Lite's transmit FIFO OVERFLOWED — EP2 datagrams are \
                 arriving faster than 48 kHz and samples are being discarded."
            );
        }
    }
}

/// Decode the five C&C bytes at the head of a radio→host frame.
///
/// C0 bit 0 is the radio's PTT line either way. What the rest means depends on
/// C0 bit 7, the Hermes-Lite's ACK flag:
///
/// - **Clear** — the ordinary rotating status. Bits 7..3 select which sensor set
///   C1..C4 carry; only set 0 is interpreted here, holding the ADC-overload flag
///   and the version bytes, the two things a bring-up log needs. The power and
///   voltage sets are board-specific and left to the caller's raw logging.
/// - **Set** — an answer to a request we marked RQST. Bits 6..1 are the address
///   being answered and C1..C4 are its data. Nothing but the I2C tunnel asks for
///   these, so they are handed straight to it.
fn decode_ep6_status(cc: &[u8], info: &mut Ep6Info) {
    info.ptt |= cc[0] & 0x01 != 0;
    if cc[0] & 0x80 != 0 {
        info.ack = Some(((cc[0] >> 1) & 0x3F, [cc[1], cc[2], cc[3], cc[4]]));
        return;
    }
    match (cc[0] >> 3) & 0x1F {
        0 => {
            info.adc_overload |= cc[1] & 0x01 != 0;
            info.versions = Some((cc[1], cc[2], cc[3], cc[4]));
        }
        // Set 1: C1/C2 are AIN5 and C3/C4 AIN1, both big-endian. AIN1 is
        // forward power on a board that has a coupler wired to it, which the
        // stock Hermes-Lite 2 has not — AIN5 is the one that means something on
        // every HL2, and it is the temperature sensor (issue #333).
        1 => info.ain5 = Some(u16::from_be_bytes([cc[1], cc[2]]) & 0x0FFF),
        _ => {}
    }
}

/// Turn a Hermes-Lite 2's AIN5 reading into degrees Celsius.
///
/// The sensor is an MCP9700-class part: 500 mV at 0 °C, 10 mV per degree above
/// it. The gateware's converter is 12-bit against a 3.26 V reference, which
/// makes the whole conversion
///
/// ```text
/// °C = (3.26 × count / 4096 − 0.5) / 0.01
/// ```
///
/// the same arithmetic the other HL2 hosts use, so a temperature read here
/// agrees with one read anywhere else on the same board.
///
/// This is the board, not the PA die: the sensor sits on the PCB near the
/// output stage, so it lags a key-down by a good many seconds and reads well
/// below the transistors themselves. That is what makes it worth showing — it
/// is the number that says whether the *board* is running hot over an afternoon
/// of FT8, which is the failure an HL2 actually has.
///
/// `None` for a reading of zero: the converter has not been sampled yet, and
/// −50 °C is not a temperature this board reports.
fn hl2_temperature_c(count: u16) -> Option<f32> {
    if count == 0 {
        return None;
    }
    Some((3.26 * f32::from(count) / 4096.0 - 0.5) / 0.01)
}

/// Decode an EP6 (radio→host) datagram, appending interleaved I,Q floats.
/// Returns `None` if the datagram is not a valid EP6 frame.
fn decode_ep6(d: &[u8], out: &mut Vec<f32>) -> Option<Ep6Info> {
    if d.len() < DATAGRAM_LEN || d[0] != 0xEF || d[1] != 0xFE || d[2] != 0x01 || d[3] != 0x06 {
        return None;
    }
    let mut info = Ep6Info {
        seq: u32::from_be_bytes([d[4], d[5], d[6], d[7]]),
        ptt: false,
        adc_overload: false,
        ack: None,
        versions: None,
        ain5: None,
    };
    for f in 0..2 {
        let frame = &d[8 + f * 512..8 + f * 512 + 512];
        if frame[0] != 0x7F || frame[1] != 0x7F || frame[2] != 0x7F {
            continue;
        }
        decode_ep6_status(&frame[3..8], &mut info);
        for s in 0..SAMPLES_PER_FRAME {
            let base = 8 + s * 8;
            let i = be24_to_f32([frame[base], frame[base + 1], frame[base + 2]]);
            let q = be24_to_f32([frame[base + 3], frame[base + 4], frame[base + 5]]);
            out.push(i);
            out.push(q);
        }
    }
    Some(info)
}

/// Metis start/stop command: `0xEF 0xFE 0x04 <run>` padded to 64 bytes. `run`
/// bit 0 starts the EP6 I/Q stream.
fn start_command(run: bool) -> [u8; 64] {
    let mut c = [0u8; 64];
    c[0] = 0xEF;
    c[1] = 0xFE;
    c[2] = 0x04;
    c[3] = if run { 0x01 } else { 0x00 };
    c
}

/// Leave the stream in the right state on the way out. A connection that has
/// been superseded — Apply/reconnect opened a replacement while this one was
/// still running — must not send the stop command: the radio is already
/// streaming to its successor, and stopping it here is what used to leave the
/// board dead until sdroxide was restarted.
fn stop_stream(socket: &UdpSocket, dest: SocketAddr, radio: IpAddr, conn_id: u64, why: &str) {
    if crate::net::owns_connection(radio, conn_id) {
        tracing::info!("HPSDR P1: {why}; stopping the radio's stream");
        let _ = socket.send_to(&start_command(false), dest);
    } else {
        tracing::info!(
            "HPSDR P1: {why}; another connection has taken over this radio, leaving its \
             stream running"
        );
    }
}

/// Run the Protocol 1 network loop until told to shut down.
pub(crate) fn run(ctx: ThreadCtx) {
    let ThreadCtx {
        socket,
        radio,
        opened_at,
        conn_id,
        board,
        rate_hz,
        lna_gain_db,
        oc,
        invert_spectrum,
        pa_enable,
        io_rx_input,
        radio_ptt: ptt_line,
        temp_centi_c,
        mut tx,
        ctrl,
    } = ctx;
    // Protocol 1 has exactly one receiver in this framing: DDC 0's stream,
    // attached and detached like any other ([`Ctrl::Attach`]). EP6 keeps
    // arriving either way — the radio streams as long as it runs — so with no
    // stream attached the samples fall on the floor and the status bytes are
    // still read.
    let mut slot: Option<(
        rtrb::Producer<f32>,
        crate::net::RxClock,
        std::sync::Arc<std::sync::atomic::AtomicBool>,
    )> = None;
    let dest = SocketAddr::new(radio, PORT);
    let speed = speed_code(rate_hz);
    let hermes_lite = board_is_hermes_lite(&board);
    let has_lna = board_has_lna_gain(&board);

    let mut out_seq: u32 = 0;
    let mut rot = Rotation::new();
    // Only a Hermes-Lite has the I2C tunnel this rides on. The board itself is
    // looked for on the bus rather than configured: it either answers or it
    // does not, and an operator should not have to tell us what is plugged in.
    let mut io_board = hermes_lite.then(|| crate::ioboard::IoBoard::new(io_rx_input));
    let mut regs = Regs {
        rx_freq: 7_100_000,
        tx_freq: 7_100_000,
        band_dial: None,
        lna_gain: has_lna.then_some(lna_gain_db),
        pa: hermes_lite.then_some(pa_enable),
        oc,
        ptt: false,
    };

    tracing::info!(
        "HPSDR P1: stream starting to {dest} at {rate_hz:.0} Hz (speed code {speed}), \
         EP2 at 48000 Hz; {}; {}; priming registers then sending run command",
        match regs.lna_gain {
            Some(g) => format!("LNA gain {g:+.0} dB"),
            None => format!("board \"{board}\" has no LNA gain register we can drive"),
        },
        match regs.pa {
            Some(true) => "PA on".to_string(),
            Some(false) => "PA off (RF1 output, T/R relay held in receive)".to_string(),
            None => format!("board \"{board}\" has no PA register we can drive"),
        }
    );

    // Prime the registers — two full rotations so every slot lands, including
    // the front-end gain — then start the EP6 I/Q stream. That order is the one
    // rustyHPSDR uses, so the radio begins with its rate, NCO and gain loaded.
    for _ in 0..(2 * regs.slots().len()) {
        let cc = regs.cc(rot.take(&regs));
        let d = build_ep2(&mut out_seq, speed, 0, regs.oc(), cc, &[]);
        let _ = socket.send_to(&d, dest);
    }
    let _ = socket.send_to(&start_command(true), dest);
    tracing::debug!("HPSDR P1: sent priming EP2 datagrams + run command; awaiting EP6 stream");

    let mut buf = [0u8; 2048];
    let mut rx_scratch: Vec<f32> = Vec::with_capacity(FLOATS_PER_DATAGRAM);
    let mut tx_scratch: Vec<f32> = Vec::with_capacity(FLOATS_PER_DATAGRAM);
    let mut next_ep2 = Instant::now();
    let mut stats = RxStats::new(1, rate_hz);
    let mut seq_in = SeqTracker::new();
    let mut logged_first_rx = false;
    let mut logged_versions = false;
    let mut warned_no_rx = false;
    let mut radio_ptt = false;
    // See the `push_iq` call: the RX ring is still full for the moment
    // between unkey and the engine draining it.
    let mut tx_backlog = false;
    let mut tx_health = TxHealth::default();
    let mut overloads: u64 = 0;
    let mut last_overload_warn: Option<Instant> = None;
    let started = Instant::now();

    loop {
        // 1) Control messages.
        while let Ok(msg) = ctrl.try_recv() {
            match msg {
                Ctrl::Attach { ddc, ring, last_rx_ms, rx_paused } => {
                    // The device refuses any DDC but 0 on a Protocol 1 board;
                    // this arm is belt and braces.
                    if ddc == 0 {
                        slot = Some((ring, last_rx_ms, rx_paused));
                        tracing::info!("HPSDR P1: receiver attached");
                    }
                }
                Ctrl::Detach { ddc } => {
                    if ddc == 0 && slot.take().is_some() {
                        if regs.ptt {
                            // The stream that owns the transmitter is going
                            // away mid-over: unkey rather than leaving the
                            // board on the air unattended.
                            regs.ptt = false;
                            while tx.pop().is_ok() {}
                        }
                        tracing::info!("HPSDR P1: receiver detached");
                    }
                }
                Ctrl::RxFreq { ddc, hz } => {
                    if ddc == 0 {
                        regs.rx_freq = hz.max(0.0) as u32;
                        rot.urge(Slot::RxFreq);
                        tracing::debug!("HPSDR P1: RX NCO -> {} Hz", regs.rx_freq);
                    }
                }
                Ctrl::BandDial(hz) => {
                    let hz = hz.filter(|h| h.is_finite() && *h > 0.0);
                    if regs.band_dial != hz {
                        regs.band_dial = hz;
                        tracing::debug!(
                            "HPSDR P1: accessory-board band follows the dial at {:?} Hz",
                            hz
                        );
                    }
                }
                Ctrl::RxGain(db) => {
                    if has_lna {
                        regs.lna_gain = Some(db);
                        rot.urge(Slot::LnaGain);
                        tracing::debug!(
                            "HPSDR P1: LNA gain -> {db:+.0} dB (code {})",
                            lna_gain_code(db)
                        );
                    }
                }
                Ctrl::TxFreq(hz) => {
                    let hz = hz.max(0.0) as u32;
                    if regs.tx_freq != hz {
                        regs.tx_freq = hz;
                        rot.urge(Slot::TxFreq);
                        tracing::debug!("HPSDR P1: TX NCO -> {hz} Hz (not keyed)");
                    }
                }
                Ctrl::TxOn(hz) => {
                    regs.tx_freq = hz.max(0.0) as u32;
                    regs.ptt = true;
                    rot.urge(Slot::TxFreq);
                    tracing::info!("HPSDR P1: MOX on, TX NCO {} Hz", regs.tx_freq);
                }
                Ctrl::TxOff => {
                    regs.ptt = false;
                    // Drop whatever the modulator left queued: it belongs to the
                    // transmission that just ended, and playing it out at the
                    // start of the next one would key up with a stale tail.
                    while tx.pop().is_ok() {}
                    tracing::info!("HPSDR P1: MOX off");
                }
                Ctrl::Shutdown => {
                    // Leave no stale "keyed" behind: a handle that outlives
                    // this thread must not read a PTT line nobody is watching.
                    ptt_line.store(false, Ordering::Relaxed);
                    stop_stream(&socket, dest, radio, conn_id, "shutdown requested");
                    return;
                }
            }
        }

        // 2) One inbound datagram (EP6 RX I/Q).
        match socket.recv_from(&mut buf) {
            Ok((n, _src)) => {
                rx_scratch.clear();
                if let Some(info) = decode_ep6(&buf[..n], &mut rx_scratch) {
                    if invert_spectrum {
                        conjugate(&mut rx_scratch);
                    }
                    let pairs = rx_scratch.len() / 2;
                    stats.on_iq(pairs);
                    if let Some((_, clock, _)) = slot.as_ref() {
                        clock.store(opened_at.elapsed().as_millis() as u64, Ordering::Relaxed);
                    }
                    stats.on_lost(seq_in.observe(info.seq) as u64);
                    if !logged_first_rx {
                        logged_first_rx = true;
                        tracing::info!(
                            "HPSDR P1: first EP6 datagram received — {n} bytes, {pairs} I/Q \
                             samples [{}]",
                            hex_head(&buf[..n], 8)
                        );
                    }
                    if let Some((c1, c2, c3, c4)) = info.versions {
                        if !logged_versions {
                            logged_versions = true;
                            tracing::info!(
                                "HPSDR P1: radio status set 0 — C1 {c1:02X} C2 {c2:02X} \
                                 C3 {c3:02X} C4 {c4:02X}{}",
                                if hermes_lite {
                                    format!(" (Hermes-Lite gateware version {c4})")
                                } else {
                                    " (firmware/gateware version bytes)".to_string()
                                }
                            );
                        }
                        if hermes_lite {
                            hl2_status(c1, c3, regs.ptt, &mut tx_health);
                        }
                    }
                    // An answer to an I2C request: the only thing that asks for
                    // one is the accessory-board driver.
                    if let Some((raddr, data)) = info.ack
                        && let Some(b) = io_board.as_mut()
                    {
                        b.on_ack(raddr, data, Instant::now());
                    }
                    // The radio's own PTT input (a Hermes-Lite's CN4 ring, a
                    // foot switch, a mic button). Published as a level for
                    // `HpsdrRx::radio_ptt`, which is what carries it up to the
                    // engine and keys the transmitter — logging it and going no
                    // further is what left a foot switch doing nothing.
                    if info.ptt != radio_ptt {
                        radio_ptt = info.ptt;
                        ptt_line.store(radio_ptt, Ordering::Relaxed);
                        tracing::info!(
                            "HPSDR P1: radio reports PTT {}",
                            if radio_ptt { "closed" } else { "open" }
                        );
                    }
                    // The board's own temperature, where the gateware sampled
                    // it this datagram. Published as a level, like the PTT line
                    // above, so a caller polling on the meter tick reads
                    // whatever the last frame said rather than having to catch
                    // one (issue #333).
                    if hermes_lite
                        && let Some(c) = info.ain5.and_then(hl2_temperature_c)
                    {
                        temp_centi_c.store((c * 100.0) as i32, Ordering::Relaxed);
                    }
                    // An overloaded ADC is the classic "the signal looks weird"
                    // fault: everything intermodulates and the noise floor
                    // jumps. Rate-limit the warning, it can fire every datagram.
                    if info.adc_overload {
                        overloads += 1;
                        if last_overload_warn.is_none_or(|t| t.elapsed() >= Duration::from_secs(5))
                        {
                            last_overload_warn = Some(Instant::now());
                            tracing::warn!(
                                "HPSDR P1: ADC OVERLOAD reported by the radio ({overloads} so \
                                 far). The front end is clipping — lower the {} gain in \
                                 Settings → Device (currently {:+.0} dB).",
                                crate::net::LNA_GAIN_ELEMENT,
                                regs.lna_gain.unwrap_or(0.0),
                            );
                        }
                    }
                    if let Some((ring, _, rx_paused)) = slot.as_mut() {
                        // Protocol 1 has one receiver and it owns the
                        // transmitter, so `regs.ptt` is exactly "the reader of
                        // this ring is keyed". It stays true a moment longer
                        // than MOX does: the ring is still full at unkey until
                        // the engine drains it (`discard_pending_rx`), and
                        // those last discards belong to the over that just
                        // ended, not to a host that fell behind. A datagram
                        // taken is the proof the reader is back.
                        // Or the engine simply is not reading: this board can
                        // be lent to another rig as a panadapter, and then
                        // nothing on this connection is keyed while the ring
                        // still goes unread for somebody else's over.
                        let keyed = regs.ptt || rx_paused.load(Ordering::Relaxed) || tx_backlog;
                        tx_backlog = !push_iq(ring, &rx_scratch, &mut stats, keyed) && keyed;
                    }
                } else {
                    stats.on_other();
                    if n >= 4 && buf[0] == 0xEF && buf[1] == 0xFE {
                        tracing::trace!(
                            "HPSDR P1: non-EP6 datagram (sync ok, endpoint 0x{:02X}, {n} bytes)",
                            buf[3]
                        );
                    } else {
                        tracing::trace!(
                            "HPSDR P1: unrecognized {n}-byte datagram [{}]",
                            hex_head(&buf[..n], 8)
                        );
                    }
                }
            }
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(e) => {
                ptt_line.store(false, Ordering::Relaxed);
                stop_stream(&socket, dest, radio, conn_id, &format!("recv error: {e}"));
                return;
            }
        }

        // Flag a radio that accepted the run command but never streams I/Q — the
        // usual symptom of a wrong sample-rate/endpoint offset or a firewall.
        if !logged_first_rx && !warned_no_rx && started.elapsed() >= Duration::from_secs(3) {
            warned_no_rx = true;
            tracing::warn!(
                "HPSDR P1: no EP6 I/Q datagrams after 3 s. Check that UDP port {PORT} is not \
                 blocked, that the radio is idle (not held by another program), and that the \
                 board actually speaks Protocol 1."
            );
        }
        stats.tick();

        // 3) EP2 on its own 48 kHz clock — one datagram per 126 TX samples,
        //    independent of the DDC rate. Pacing it off EP6 instead (which
        //    arrives eight times as fast at 384 kHz) overruns the radio's TX
        //    FIFO and plays the modulator's audio out at the wrong speed.
        let now = Instant::now();
        if now >= next_ep2 {
            tx_scratch.clear();
            if regs.ptt {
                // Whole I/Q pairs only, taken atomically: consuming an odd
                // number of floats would leave the ring one slot out of step and
                // swap I with Q from the next datagram on, inverting the
                // transmitted sideband for the rest of the over.
                while tx_scratch.len() < FLOATS_PER_DATAGRAM {
                    let Ok(pair) = tx.read_chunk(2) else { break };
                    let (a, b) = pair.as_slices();
                    tx_scratch.extend_from_slice(a);
                    tx_scratch.extend_from_slice(b);
                    pair.commit_all();
                }
                if invert_spectrum {
                    conjugate(&mut tx_scratch);
                }
            }
            let mox = if regs.ptt { 1 } else { 0 };
            // An accessory board on the I2C tunnel gets frame #2 when it has
            // something to say, which is only when the transmit frequency
            // moves; the register rotation keeps the slot the rest of the time
            // and never loses its turn (`rot.take` is not reached here).
            let cc = io_board
                .as_mut()
                .and_then(|b| b.next_request(now, regs.tx_freq, regs.rx_freq, mox))
                .unwrap_or_else(|| regs.cc(rot.take(&regs)));
            let d = build_ep2(&mut out_seq, speed, mox, regs.oc(), cc, &tx_scratch);
            let _ = socket.send_to(&d, dest);
            next_ep2 += EP2_INTERVAL;
            // After a scheduler stall, resync instead of firing off the backlog
            // in a burst the radio's FIFO would only drop.
            if next_ep2 + EP2_INTERVAL < now {
                next_ep2 = now + EP2_INTERVAL;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol2::f32_to_be24;

    #[test]
    fn speed_codes() {
        assert_eq!(speed_code(48_000.0), 0);
        assert_eq!(speed_code(96_000.0), 1);
        assert_eq!(speed_code(192_000.0), 2);
        assert_eq!(speed_code(384_000.0), 3);
    }

    #[test]
    fn freq_cc_is_hz_big_endian() {
        let cc = freq_cc(CC_RX1_FREQ, 14_074_000, 1);
        assert_eq!(cc[0], CC_RX1_FREQ | 1); // MOX bit set
        assert_eq!(u32::from_be_bytes([cc[1], cc[2], cc[3], cc[4]]), 14_074_000);
    }

    #[test]
    fn ep6_decode_roundtrip() {
        // Hand-build an EP6 datagram with one non-zero sample in each frame,
        // using the 24-bit encoder, and confirm decode recovers it.
        let mut d = [0u8; DATAGRAM_LEN];
        d[0] = 0xEF;
        d[1] = 0xFE;
        d[2] = 0x01;
        d[3] = 0x06;
        for f in 0..2 {
            let frame = &mut d[8 + f * 512..8 + f * 512 + 512];
            frame[0] = 0x7F;
            frame[1] = 0x7F;
            frame[2] = 0x7F;
            // sample 0: I=0.5, Q=-0.25
            frame[8..11].copy_from_slice(&f32_to_be24(0.5));
            frame[11..14].copy_from_slice(&f32_to_be24(-0.25));
        }
        let mut out = Vec::new();
        let info = decode_ep6(&d, &mut out).expect("valid EP6");
        // 2 frames × 63 samples × 2 floats.
        assert_eq!(out.len(), FLOATS_PER_DATAGRAM);
        assert!((out[0] - 0.5).abs() < 1e-4);
        assert!((out[1] + 0.25).abs() < 1e-4);
        // All-zero C&C: no PTT, no overload, and status set 0 was seen.
        assert!(!info.ptt);
        assert!(!info.adc_overload);
        assert_eq!(info.versions, Some((0, 0, 0, 0)));
    }

    #[test]
    fn ep6_status_bits() {
        let mut info =
            Ep6Info { seq: 0, ptt: false, adc_overload: false, ack: None, versions: None, ain5: None };
        // Status set 0, PTT closed, ADC overloaded, versions in C2..C4.
        decode_ep6_status(&[0x01, 0x01, 0x11, 0x22, 0x33], &mut info);
        assert!(info.ptt);
        assert!(info.adc_overload);
        assert_eq!(info.versions, Some((0x01, 0x11, 0x22, 0x33)));

        // A different status set carries power/voltage, not versions: the
        // overload flag and version bytes must not be read out of it.
        let mut other =
            Ep6Info { seq: 0, ptt: false, adc_overload: false, ack: None, versions: None, ain5: None };
        // Set 2 (power/voltage): not versions, and not the temperature either.
        decode_ep6_status(&[0x10, 0xFF, 0xFF, 0xFF, 0xFF], &mut other);
        assert!(!other.adc_overload);
        assert_eq!(other.versions, None);
        assert_eq!(other.ain5, None);
    }

    /// Issue #333: a Hermes-Lite 2 reports its board temperature on AIN5, in
    /// status set 1 — the frames this decoder used to skip past.
    #[test]
    fn a_hermes_lite_reports_its_temperature_on_status_set_one() {
        // C0: set 1 in bits 7..3, ACK clear, PTT clear.
        let cc = [1u8 << 3, 0x07, 0x8B, 0x00, 0x00];
        let mut info =
            Ep6Info { seq: 0, ptt: false, adc_overload: false, ack: None, versions: None, ain5: None };
        decode_ep6_status(&cc, &mut info);
        assert_eq!(info.ain5, Some(0x078B));
        // 3.26 × 1931 / 4096 = 1.537 V; less the sensor's 500 mV offset, over
        // 10 mV per degree, is a shade under 104 °C… which is what this count
        // means and not a temperature a healthy board reaches. The arithmetic
        // is what is under test.
        let c = hl2_temperature_c(0x078B).expect("a reading");
        assert!((c - 103.7).abs() < 0.5, "{c} °C");

        // A room-temperature board: 25 °C is 750 mV, which is count 942.
        let c = hl2_temperature_c(942).expect("a reading");
        assert!((c - 25.0).abs() < 0.5, "{c} °C");

        // Nothing sampled yet is not −50 °C.
        assert_eq!(hl2_temperature_c(0), None);

        // And set 0 still means what it meant: versions, not a temperature.
        let mut info =
            Ep6Info { seq: 0, ptt: false, adc_overload: false, ack: None, versions: None, ain5: None };
        decode_ep6_status(&[0x00, 0x01, 0x02, 0x03, 0x04], &mut info);
        assert!(info.ain5.is_none());
        assert_eq!(info.versions, Some((0x01, 0x02, 0x03, 0x04)));
        assert!(info.adc_overload);
    }

    /// A converter puts the radio on an intermediate frequency, and the
    /// accessory board's decoder — which is in the antenna line, ahead of the
    /// converter — has to switch for the band on the *air* rather than for
    /// that (issue #278).
    ///
    /// The example is an upconverter, because that is the one where the two
    /// answers differ inside these boards' tables: a Ham It Up presents 80 m to
    /// the receiver at 128.7 MHz, and the filter the antenna needs is 80 m's.
    #[test]
    fn the_accessory_board_follows_the_dial_through_a_converter() {
        let mut regs = Regs {
            rx_freq: 128_700_000,
            tx_freq: 128_700_000,
            band_dial: None,
            lna_gain: None,
            pa: None,
            oc: HpsdrOcPlan::preset(HpsdrFilterBoard::Alex),
            ptt: false,
        };
        // Without a dial, the I.F. is all there is.
        assert_eq!(regs.oc(), alex_oc(128_700_000.0));
        regs.band_dial = Some(3_700_000.0);
        assert_eq!(regs.oc(), alex_oc(3_700_000.0), "the band code has to follow the dial");
        assert_ne!(regs.oc(), alex_oc(128_700_000.0));
        // Putting it back is the radio on its own bands again.
        regs.band_dial = None;
        assert_eq!(regs.oc(), alex_oc(128_700_000.0));
    }

    #[test]
    fn ep2_datagram_shape() {
        let mut seq = 0u32;
        let regs = Regs {
            rx_freq: 7_074_000,
            tx_freq: 7_074_000,
            band_dial: None,
            lna_gain: None,
            pa: None,
            oc: HpsdrOcPlan::none(),
            ptt: false,
        };
        let mut rot = Rotation::new();
        let d = build_ep2(&mut seq, 0, 0, regs.oc(), regs.cc(rot.take(&regs)), &[]);
        assert_eq!(d.len(), 1032);
        assert_eq!(&d[0..4], &[0xEF, 0xFE, 0x01, 0x02]);
        assert_eq!(seq, 1); // advanced
        // Both frames start with the OZY sync.
        assert_eq!(&d[8..11], &[0x7F, 0x7F, 0x7F]);
        assert_eq!(&d[520..523], &[0x7F, 0x7F, 0x7F]);
        // Frame #1 is the config register (address 0); frame #2 is rotating
        // slot 0 = the TX-frequency register (C0 = 0x02).
        assert_eq!(d[11], CC_CONFIG); // frame #1 C0
        assert_eq!(d[8 + 512 + 3], CC_TX_FREQ); // frame #2 C0
        // Config C4 carries the duplex/receiver-count field.
        assert_eq!(d[8 + 7], CONFIG_C4); // frame #1 C4
    }

    #[test]
    fn mox_bit_rides_registers() {
        assert_eq!(config_cc(0, 1, 0)[0] & 1, 1);
        assert_eq!(config_cc(0, 0, 0)[0] & 1, 0);
        let keyed = Regs {
            rx_freq: 7_000_000,
            tx_freq: 7_000_000,
            band_dial: None,
            lna_gain: None,
            pa: None,
            oc: HpsdrOcPlan::none(),
            ptt: true,
        };
        let idle = Regs { ptt: false, ..keyed };
        for slot in [Slot::TxFreq, Slot::RxFreq, Slot::Drive] {
            assert_eq!(keyed.cc(slot)[0] & 1, 1);
            assert_eq!(idle.cc(slot)[0] & 1, 0);
        }
    }

    /// The regression behind "Hermes-Lite 2 keys but makes no power": register
    /// `0x09` went out with C2 = 0, which is the Hermes-Lite's "PA off". The
    /// board keys anyway — the T/R relay throws and the PTT line follows — so
    /// nothing upstream of the antenna jack looks wrong.
    #[test]
    fn drive_register_switches_the_hermes_lite_pa_on() {
        let hl2 = Regs {
            rx_freq: 14_074_000,
            tx_freq: 14_074_000,
            band_dial: None,
            lna_gain: Some(20.0),
            pa: Some(true),
            oc: HpsdrOcPlan::none(),
            ptt: false,
        };
        let cc = hl2.cc(Slot::Drive);
        assert_eq!(cc[0], CC_DRIVE, "register 0x09");
        assert_eq!(cc[1], TX_DRIVE, "drive level unchanged");
        assert_eq!(cc[2], HL2_PA_ON, "bit 19: onboard PA on");
        assert_eq!(cc[2] & HL2_TR_RX_ONLY, 0, "the T/R relay must be free to switch");

        // PA off is a real operating choice — an external amplifier driven from
        // the low-power RF1 output — and then the antenna jack is deliberately
        // held on the receiver rather than left connected to a dead PA.
        let off = Regs { pa: Some(false), ..hl2 }.cc(Slot::Drive);
        assert_eq!(off[2], HL2_TR_RX_ONLY);
        assert_eq!(off[2] & HL2_PA_ON, 0);

        // On any other Protocol 1 board those bits are Apollo tuner/filter
        // commands, so the byte stays exactly as it always was.
        assert_eq!(Regs { pa: None, ..hl2 }.cc(Slot::Drive)[2], 0);

        // The PA state rides every drive frame, keyed or not: the board has to
        // already know it when MOX arrives.
        for ptt in [false, true] {
            assert_eq!(Regs { ptt, ..hl2 }.cc(Slot::Drive)[2], HL2_PA_ON);
        }
    }

    /// The Hermes-Lite's own account of why a transmission is not going out,
    /// from status set 0. Both fields sit in the `RDATA` bits the protocol
    /// document quotes, which is what makes them easy to misplace: `RDATA[25]`
    /// is C1 bit 1 and `RDATA[15:14]` are the top two bits of C3.
    #[test]
    fn hermes_lite_status_reports_tx_faults() {
        // C1 bit 1 is TX-inhibit *active low*: set means "not inhibited".
        let mut health = TxHealth::default();
        hl2_status(0x1E, 0x00, false, &mut health);
        assert!(health.last_inhibit_warn.is_none(), "0x1E is a healthy transmitter");
        hl2_status(0x1C, 0x00, false, &mut health);
        assert!(health.last_inhibit_warn.is_some(), "inhibit line pulled low");

        // FIFO under/overflow only counts while keyed — the transmit FIFO
        // legitimately sits empty until the first over.
        let mut health = TxHealth::default();
        hl2_status(0x1E, 0x80, false, &mut health);
        assert!(health.last_fifo_warn.is_none(), "an idle radio has an empty TX FIFO");
        hl2_status(0x1E, 0x80, true, &mut health);
        assert!(health.last_fifo_warn.is_some(), "underflow while keyed");

        // A plain fill level (bit 15 clear) is not a fault however full it is.
        let mut health = TxHealth::default();
        hl2_status(0x1E, 0x7F, true, &mut health);
        assert!(health.last_fifo_warn.is_none());

        // Both warnings are rate-limited independently, so a chattering fault
        // cannot crowd the other one out of the log.
        let mut health = TxHealth::default();
        hl2_status(0x1C, 0x00, true, &mut health);
        let first = health.last_inhibit_warn;
        hl2_status(0x1C, 0xC0, true, &mut health);
        assert_eq!(health.last_inhibit_warn, first, "second inhibit warning suppressed");
        assert!(health.last_fifo_warn.is_some(), "overflow still reported");
    }

    #[test]
    fn lna_gain_register_encodes_hl2_range() {
        let regs = Regs {
            rx_freq: 7_000_000,
            tx_freq: 7_000_000,
            band_dial: None,
            lna_gain: Some(0.0),
            pa: None,
            oc: HpsdrOcPlan::none(),
            ptt: false,
        };
        let cc = regs.cc(Slot::LnaGain);
        assert_eq!(cc[0], CC_HL2_GAIN);
        // 0 dB → code 12, with the "field is valid" bit set.
        assert_eq!(cc[4], HL2_GAIN_VALID | 12);
        // The rails: −12 dB → 0, +48 dB → 60, and both are clamped beyond that.
        assert_eq!(Regs { lna_gain: Some(-12.0), ..regs }.cc(Slot::LnaGain)[4], HL2_GAIN_VALID);
        assert_eq!(Regs { lna_gain: Some(48.0), ..regs }.cc(Slot::LnaGain)[4], HL2_GAIN_VALID | 60);
        assert_eq!(Regs { lna_gain: Some(-40.0), ..regs }.cc(Slot::LnaGain)[4], HL2_GAIN_VALID);
        assert_eq!(Regs { lna_gain: Some(99.0), ..regs }.cc(Slot::LnaGain)[4], HL2_GAIN_VALID | 60);
        // The gain code never runs into the valid bit.
        for db in [-12.0, 0.0, 20.0, 48.0] {
            assert_eq!(lna_gain_code(db) & HL2_GAIN_VALID, 0);
        }
    }

    #[test]
    fn conjugation_negates_only_q() {
        let mut iq = [1.0f32, 2.0, -3.0, 4.0, 5.0, -6.0];
        conjugate(&mut iq);
        assert_eq!(iq, [1.0, -2.0, -3.0, -4.0, 5.0, 6.0]);
        // Applying it twice is the identity, so RX and TX cannot drift apart.
        conjugate(&mut iq);
        assert_eq!(iq, [1.0, 2.0, -3.0, 4.0, 5.0, -6.0]);
        // An odd tail (never produced by the pair-safe paths) is left alone
        // rather than panicking.
        let mut odd = [1.0f32, 2.0, 3.0];
        conjugate(&mut odd);
        assert_eq!(odd, [1.0, -2.0, 3.0]);
    }

    #[test]
    fn n2adr_filter_selection_is_one_hot_per_band() {
        // Bit assignment from the board's own documentation: 0 = 160 m,
        // 1 = 80 m, 2 = 60/40 m, 3 = 30/20 m, 4 = 17/15 m, 5 = 12/10 m,
        // 6 = the 3 MHz receive high-pass.
        const HPF: u8 = 1 << 6;
        assert_eq!(n2adr_oc(1_840_000.0), 1 << 0, "160 m: LPF only, no high-pass");
        assert_eq!(n2adr_oc(3_573_000.0), (1 << 1) | HPF, "80 m");
        assert_eq!(n2adr_oc(7_074_000.0), (1 << 2) | HPF, "40 m");
        assert_eq!(n2adr_oc(5_357_000.0), (1 << 2) | HPF, "60 m shares the 40 m filter");
        assert_eq!(n2adr_oc(10_136_000.0), (1 << 3) | HPF, "30 m");
        assert_eq!(n2adr_oc(14_074_000.0), (1 << 3) | HPF, "20 m shares the 30 m filter");
        assert_eq!(n2adr_oc(18_100_000.0), (1 << 4) | HPF, "17 m");
        assert_eq!(n2adr_oc(21_074_000.0), (1 << 4) | HPF, "15 m shares the 17 m filter");
        assert_eq!(n2adr_oc(24_915_000.0), (1 << 5) | HPF, "12 m");
        assert_eq!(n2adr_oc(28_074_000.0), (1 << 5) | HPF, "10 m shares the 12 m filter");

        // Exactly one low-pass relay is ever selected, and the byte always fits
        // the seven lines the header actually has.
        for f in [500_000.0, 1_840_000.0, 7_074_000.0, 14_074_000.0, 28_074_000.0, 50_000_000.0] {
            let oc = n2adr_oc(f);
            assert_eq!(oc & !HPF, (oc & !HPF).next_power_of_two(), "one-hot LPF at {f} Hz");
            assert_eq!(oc & 0x80, 0, "only seven outputs exist");
        }
        // Below the high-pass corner it is left out, so broadcast-band and
        // 160 m listening is not attenuated by it.
        assert_eq!(n2adr_oc(600_000.0) & HPF, 0);
        assert_eq!(n2adr_oc(3_500_000.0) & HPF, HPF);
    }

    #[test]
    fn alex_filter_selection_is_the_band_as_a_four_bit_code() {
        // The table an ANAN/Hermes Alex board, a Zeus SDR, a HiQSDR and Quisk
        // all share, quoted band for band from issue #196.
        for (hz, want) in [
            (1_840_000.0, 0x01), // 160 m
            (3_573_000.0, 0x02), // 80 m
            (5_357_000.0, 0x00), // 60 m — deliberately no pins
            (7_074_000.0, 0x03), // 40 m
            (10_136_000.0, 0x04),
            (14_074_000.0, 0x05),
            (18_100_000.0, 0x06),
            (21_074_000.0, 0x07),
            (24_915_000.0, 0x08),
            (28_074_000.0, 0x09),
            (50_313_000.0, 0x0A), // 6 m
        ] {
            assert_eq!(alex_oc(hz), want, "{hz} Hz");
        }
        // Everything below 160 m rides the 160 m code and everything above 6 m
        // the 6 m one, so a listener off the ham bands still gets a filter.
        assert_eq!(alex_oc(198_000.0), 0x01);
        assert_eq!(alex_oc(70_200_000.0), 0x0A);
        // The code is four bits: outputs 5-7 carry no part of it, and the byte
        // fits the seven lines the header has.
        for hz in [198_000.0, 5_000_000.0, 9_000_000.0, 27_555_000.0, 50_313_000.0] {
            assert_eq!(alex_oc(hz) & !0x0F, 0, "outputs 5-7 stay off at {hz} Hz");
        }
    }

    #[test]
    fn open_collector_bits_land_in_c2_and_are_off_without_a_board() {
        // The seven lines sit at bits [23:17] of the register's 32-bit view,
        // which is C2 bits [7:1].
        assert_eq!(config_cc(3, 0, 0b0100_1000)[2], 0b1001_0000);
        // No configured board means every output stays off.
        let regs = Regs {
            rx_freq: 14_074_000,
            tx_freq: 14_074_000,
            band_dial: None,
            lna_gain: Some(20.0),
            pa: None,
            oc: HpsdrOcPlan::none(),
            ptt: false,
        };
        assert_eq!(regs.oc(), 0);
        assert_eq!(config_cc(3, 0, regs.oc())[2], 0);
        // With a board, the filter follows the transmit frequency while keyed —
        // the low-pass has to match what is actually going out.
        let split = Regs {
            rx_freq: 14_074_000,
            tx_freq: 7_074_000,
            band_dial: None,
            oc: HpsdrOcPlan::preset(HpsdrFilterBoard::N2adr),
            ..regs
        };
        assert_eq!(split.oc(), n2adr_oc(14_074_000.0), "receiving: follows RX");
        assert_eq!(Regs { ptt: true, ..split }.oc(), n2adr_oc(7_074_000.0), "keyed: follows TX");
        // And the same rule on the band-code preset.
        let alex = Regs { oc: HpsdrOcPlan::preset(HpsdrFilterBoard::Alex), ..split };
        assert_eq!(alex.oc(), 0x05, "receiving on 20 m");
        assert_eq!(Regs { ptt: true, ..alex }.oc(), 0x03, "keyed on 40 m");
        assert_eq!(config_cc(3, 0, alex.oc())[2], 0x05 << 1);
    }

    /// The operator's own table, on Protocol 1: their words, on their bands,
    /// with receive and transmit told apart (issue #296). The register path is
    /// unchanged — what is new is where the word comes from.
    #[test]
    fn a_custom_table_reaches_the_same_register() {
        let cfg = sdroxide_types::HpsdrConfig {
            filter_board: HpsdrFilterBoard::Custom,
            oc_table: vec![sdroxide_types::HpsdrOcRow {
                band: sdroxide_types::Band::M20,
                rx: 0x12,
                tx: 0x52,
            }],
            ..sdroxide_types::HpsdrConfig::default()
        };
        let regs = Regs {
            rx_freq: 14_074_000,
            tx_freq: 14_074_000,
            band_dial: None,
            lna_gain: None,
            pa: None,
            oc: cfg.oc_plan(),
            ptt: false,
        };
        assert_eq!(regs.oc(), 0x12);
        assert_eq!(config_cc(3, 0, regs.oc())[2], 0x12 << 1);
        assert_eq!(Regs { ptt: true, ..regs }.oc(), 0x52, "keyed takes the transmit word");
        // A band the operator said nothing about asserts nothing, rather than
        // the nearest band's word.
        assert_eq!(Regs { rx_freq: 7_074_000, ..regs }.oc(), 0);
    }

    #[test]
    fn rotation_covers_every_slot_and_honours_urgency() {
        let hl2 = Regs {
            rx_freq: 7_000_000,
            tx_freq: 7_000_000,
            band_dial: None,
            lna_gain: Some(20.0),
            pa: None,
            oc: HpsdrOcPlan::none(),
            ptt: false,
        };
        let mut rot = Rotation::new();
        // A Hermes-Lite rotation visits all four slots; a board without the gain
        // register visits three and never emits the gain slot.
        let seen: Vec<Slot> = (0..4).map(|_| rot.take(&hl2)).collect();
        assert!(seen.contains(&Slot::LnaGain));
        let plain = Regs { lna_gain: None, ..hl2 };
        let mut rot = Rotation::new();
        for _ in 0..9 {
            assert_ne!(rot.take(&plain), Slot::LnaGain);
        }
        // An urgent slot jumps the queue, and repeats collapse to one entry.
        let mut rot = Rotation::new();
        rot.urge(Slot::RxFreq);
        rot.urge(Slot::RxFreq);
        assert_eq!(rot.take(&hl2), Slot::RxFreq);
        assert_ne!(rot.take(&hl2), Slot::RxFreq);
    }
}
