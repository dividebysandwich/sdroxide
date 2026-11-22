//! One thread per connected client: pure transport. It completes the WebSocket
//! handshake, writes whatever the hub queues, and hands back what the peer
//! sends. Every protocol decision belongs to the hub, so nothing here inspects
//! a command.
//!
//! The loop mirrors the client-side `NetThread::run`: a short socket read
//! timeout turns a blocking WebSocket into a poll loop that interleaves reads
//! and writes on one thread, which is what lets us avoid an async runtime in
//! the GUI process.

use std::net::TcpStream;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, TryRecvError};
use tracing::{debug, warn};
use tungstenite::{Bytes, HandshakeError, Message, WebSocket};

use crate::protocol as p;

/// Socket read timeout; also the loop's tick.
const READ_TIMEOUT: Duration = Duration::from_millis(20);

/// How long a client has to complete its upgrade. Held as the socket's read
/// timeout for the duration, because [`READ_TIMEOUT`] is far too short to cover
/// a request arriving over anything but the loopback — and a read that expires
/// mid-upgrade is only resumable on Unix (see `crate::ws_handshake`).
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// Realtime frames written per iteration before we go back to reading, so a
/// backlog can drain quickly without starving inbound control.
const WRITE_BUDGET: usize = 32;

/// Anything a client sends that the hub must see.
pub(crate) enum Up {
    Text(u64, String),
    /// Decoded TX audio, already downmixed to 48 kHz mono.
    TxAudio(u64, Vec<f32>),
    Gone(u64),
}

/// The two outbound lanes: text is never dropped, realtime data is.
pub(crate) struct Out {
    pub text_rx: Receiver<String>,
    pub data_rx: Receiver<Bytes>,
}

pub(crate) struct ClientParams {
    pub id: u64,
    pub stream: TcpStream,
    pub out: Out,
    pub up_tx: Sender<Up>,
}

pub(crate) fn spawn(params: ClientParams) -> JoinHandle<()> {
    let id = params.id;
    std::thread::Builder::new()
        .name(format!("sdroxide-tciserv-{id}"))
        .spawn(move || {
            let up_tx = params.up_tx.clone();
            run(params);
            let _ = up_tx.send(Up::Gone(id));
        })
        .expect("spawn TCI server client")
}

fn run(pr: ClientParams) {
    let ClientParams { id, stream, out, up_tx } = pr;
    if stream.set_nodelay(true).is_err() {
        debug!(id, "TCI client: could not disable Nagle");
    }
    // The handshake needs a timeout too, or a peer that connects and says
    // nothing would pin this thread forever. Wide enough for the upgrade, then
    // the loop's own tick once there is a WebSocket to poll.
    if stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT)).is_err() {
        warn!(id, "TCI client: could not set read timeout");
        return;
    }
    let Some(mut ws) = accept(id, stream) else { return };
    if ws.get_ref().set_read_timeout(Some(READ_TIMEOUT)).is_err() {
        warn!(id, "TCI client: could not set read timeout");
        return;
    }

    let mut scratch: Vec<f32> = Vec::new();
    loop {
        let mut dirty = false;

        // 1) Status and control text. Never dropped, so drain it all.
        loop {
            match out.text_rx.try_recv() {
                Ok(line) => {
                    if ws.write(Message::Text(line.into())).is_err() {
                        return;
                    }
                    dirty = true;
                }
                Err(TryRecvError::Empty) => break,
                // The hub dropped our lanes: it is closing this client.
                Err(TryRecvError::Disconnected) => {
                    let _ = ws.close(None);
                    let _ = ws.flush();
                    return;
                }
            }
        }

        // 2) Stream frames, bounded so a backlog can't starve the read below.
        for _ in 0..WRITE_BUDGET {
            match out.data_rx.try_recv() {
                Ok(pkt) => {
                    if let Err(e) = ws.write(Message::Binary(pkt)) {
                        debug!(id, "TCI client stream write failed: {e}");
                        return;
                    }
                    dirty = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }

        if dirty && let Err(e) = ws.flush() {
            debug!(id, "TCI client flush failed: {e}");
            return;
        }

        // 3) One inbound message. The read timeout is what paces this loop.
        match ws.read() {
            Ok(Message::Text(t)) => {
                if up_tx.send(Up::Text(id, t.as_str().to_string())).is_err() {
                    return;
                }
            }
            Ok(Message::Binary(b)) => {
                if let Some(mono) = decode_tx_audio(&b, &mut scratch)
                    && up_tx.send(Up::TxAudio(id, mono)).is_err()
                {
                    return;
                }
            }
            Ok(Message::Close(_)) => {
                let _ = ws.close(None);
                let _ = ws.flush();
                return;
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(e))
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(e) => {
                debug!(id, "TCI client read ended: {e}");
                return;
            }
        }
    }
}

/// Complete the server-side WebSocket handshake, retrying while the read
/// timeout interrupts it, and giving up at [`HANDSHAKE_TIMEOUT`] so a peer that
/// connects and says nothing cannot hold the thread.
fn accept(id: u64, stream: TcpStream) -> Option<WebSocket<TcpStream>> {
    let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
    let mut result = tungstenite::accept(stream);
    loop {
        match result {
            Ok(ws) => return Some(ws),
            Err(HandshakeError::Interrupted(mid)) => {
                if Instant::now() > deadline {
                    debug!(id, "TCI client never finished its handshake");
                    return None;
                }
                result = mid.handshake();
            }
            Err(HandshakeError::Failure(e)) => {
                debug!(id, "TCI client handshake failed: {e}");
                return None;
            }
        }
    }
}

/// Decode a client `TxAudio` packet to 48 kHz mono. Returns `None` for any
/// other stream type — a client has no other reason to send us binary.
fn decode_tx_audio(msg: &[u8], scratch: &mut Vec<f32>) -> Option<Vec<f32>> {
    let h = p::parse_header(msg)?;
    if h.dtype != p::DataType::TxAudio {
        return None;
    }
    scratch.clear();
    p::decode_f32_payload(msg, &h, scratch);
    Some(match interleave_of(h.channels, scratch) {
        1 => scratch.clone(),
        n => scratch.chunks(n).map(|f| f.iter().sum::<f32>() / f.len() as f32).collect(),
    })
}

/// How many floats of the payload make one frame.
///
/// **The header field cannot be trusted.** WSJT-X leaves `channels`
/// uninitialised on the transmit audio it sends, so it arrives as a *different*
/// large random number in every packet — 2 771 387 240, then 613 045 248, then
/// 0. Taken at face value, `chunks(n)` over one of those collapses a whole
/// 10 ms packet into a single averaged sample, and what reaches the air is a
/// few hundred scattered samples of the client's waveform: a 15 s FT8 slot
/// going out as a few seconds of chopped signal, which is issue #202.
///
/// So only 1 and 2 are read as channel counts. Anything else is decided from
/// the payload, which says so plainly: transmit audio is mono content, and a
/// client that interleaves it duplicates each sample, so a stereo packet is
/// pairs of identical floats. WSJT-X's are, exactly — `[0.778, 0.778, 0.913,
/// 0.913, …]` — and a genuinely mono packet from a sender that also left the
/// field unset is not halved.
fn interleave_of(channels: u32, payload: &[f32]) -> usize {
    match channels {
        1 => 1,
        2 => 2,
        _ => {
            let pairs = payload.len() / 2;
            // Every pair, not most of them: a duplicated stream is duplicated
            // exactly, and "nearly all" would also be true of a real stereo
            // recording of a quiet room. An empty or odd payload is not paired
            // at all and stays mono.
            let duplicated = pairs > 0
                && payload.len() % 2 == 0
                && payload.chunks_exact(2).all(|c| c[0] == c[1]);
            if duplicated { 2 } else { 1 }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tx_audio_downmix() {
        // Stereo in, mono out: the two channels are averaged.
        let pkt = p::build_binary(p::DataType::TxAudio, 0, 48_000, 2, &[1.0, 0.0, 0.5, 0.5]);
        let mut scratch = Vec::new();
        assert_eq!(decode_tx_audio(&pkt, &mut scratch), Some(vec![0.5, 0.5]));

        // A sender that left `channels` unset but duplicated its samples is
        // stereo whatever the field says.
        let pkt = p::build_binary(p::DataType::TxAudio, 0, 48_000, 0, &[0.25, 0.25, -0.5, -0.5]);
        assert_eq!(decode_tx_audio(&pkt, &mut scratch), Some(vec![0.25, -0.5]));

        // …and one that left it unset and really is mono is not halved.
        let pkt = p::build_binary(p::DataType::TxAudio, 0, 48_000, 0, &[0.25, -0.25]);
        assert_eq!(decode_tx_audio(&pkt, &mut scratch), Some(vec![0.25, -0.25]));

        // Anything that isn't TX audio is not ours to interpret.
        let pkt = p::build_iq(96_000, 0, &[1.0, 1.0]);
        assert_eq!(decode_tx_audio(&pkt, &mut scratch), None);
        assert_eq!(decode_tx_audio(&[0u8; 8], &mut scratch), None);
    }

    /// Issue #202, in the form it actually arrived in: WSJT-X sends
    /// duplicated-stereo transmit audio with the `channels` field left as
    /// whatever was on the stack, a different value in every packet. Trusting
    /// it collapsed each 10 ms packet to a single sample.
    #[test]
    fn a_garbage_channel_count_does_not_destroy_the_packet() {
        let mut scratch = Vec::new();
        // 480 duplicated frames — one 10 ms packet, exactly as WSJT-X sends it.
        let mono: Vec<f32> = (0..480).map(|i| (i as f32 / 480.0) - 0.5).collect();
        let stereo: Vec<f32> = mono.iter().flat_map(|&s| [s, s]).collect();
        for channels in [2_771_387_240u32, 613_045_248, 0, 3_331_189_240, u32::MAX] {
            let pkt = p::build_binary(p::DataType::TxAudio, 0, 48_000, channels, &stereo);
            let got = decode_tx_audio(&pkt, &mut scratch).expect("TX audio");
            assert_eq!(
                got.len(),
                mono.len(),
                "channels={channels} turned a 480-frame packet into {} frame(s)",
                got.len()
            );
            assert_eq!(got, mono, "channels={channels} changed the waveform");
        }
    }
}
