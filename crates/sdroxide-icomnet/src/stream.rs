//! The reliability layer shared by all three UDP streams.
//!
//! Icom runs control, CI-V and audio as three independent conversations that
//! happen to speak the same language: each has its own socket, its own session
//! ids, its own 16-bit sequence space, and its own idle/ping/retransmit
//! timers. [`Stream`] is that language; the per-stream logic on top of it
//! differs only in what it does with the payload.
//!
//! # Why retransmit at all, over UDP, on a LAN
//!
//! Because the radio asks. It buffers what it sends and expects us to request
//! anything we miss; a client that never asks gets a stream with holes in it
//! and no way to tell a lost packet from silence. The obligation runs both
//! ways — the radio requests our packets back, and a client that cannot supply
//! them stalls the far end's own sequencing.

use std::collections::{BTreeMap, VecDeque};
use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::time::{Duration, Instant};

use crate::protocol::{self as proto, Header, ctrl, timing};
use crate::trace::Trace;

/// What one call to [`Stream::poll`] produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Event {
    /// Nothing arrived before the read timeout.
    Idle,
    /// A packet arrived and the reliability layer dealt with it in full.
    Handled,
    /// The peer answered "are you there" — its session id is now known.
    IAmHere,
    /// The peer answered "are you ready".
    Ready,
    /// The peer is closing the stream.
    Disconnect,
    /// An application packet; the caller's buffer holds the whole datagram.
    Data,
}

pub(crate) struct Stream {
    sock: UdpSocket,
    peer: SocketAddr,
    my_id: u32,
    peer_id: u32,

    /// Our outgoing sequence, stamped into every tracked packet.
    send_seq: u16,
    /// Recently sent tracked packets, oldest first, for retransmit.
    tx_buf: VecDeque<(u16, Vec<u8>)>,

    /// Highest sequence the peer has reached, if it has sent anything.
    rx_high: Option<u16>,
    /// Packets in a row that arrived far behind `rx_high` without being asked
    /// for. A run of them is a peer that restarted its sequence lower down,
    /// not stray duplicates; see [`Stream::accept`].
    behind_run: u8,
    /// Sequences we have not seen yet, and how many times we have asked.
    missing: BTreeMap<u16, u8>,

    ping_seq: u16,
    last_rx: Instant,
    next_idle: Instant,
    next_ping: Instant,
    next_are_you_there: Instant,
    next_retransmit: Instant,
    /// Stops once the peer says "I am here".
    probing: bool,
    /// Whether this stream fills silence with pings. All three streams do —
    /// the established clients (wfview, kappanhang) ping every stream, and a
    /// stream the radio never hears from is one it may time out. This matters
    /// most on the audio stream of a receive-only session, where nothing else
    /// is ever sent after the handshake.
    ping: bool,
    /// Whether this stream fills silence with sequenced idle packets. The
    /// control and CI-V streams do; the audio stream does not — its own data
    /// is its filler, and a receive-only session's silence is covered by the
    /// pings above without putting empty packets into the audio sequence space.
    idle: bool,

    trace: Trace,
    tag: &'static str,
}

impl Stream {
    /// Bind an ephemeral local port and aim at `peer`.
    ///
    /// `local_ip` only feeds the session id — the socket binds to all
    /// interfaces, because a radio reached over a VPN or a second NIC answers
    /// from an address the caller cannot predict.
    pub(crate) fn bind(
        local_ip: Ipv4Addr,
        peer: SocketAddrV4,
        local_port: u16,
        idle: bool,
        trace: Trace,
        tag: &'static str,
    ) -> io::Result<Stream> {
        let sock = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, local_port))?;
        // All three streams share one thread, so none of them may block: the
        // thread drains every socket each turn and sleeps only when all three
        // are empty.
        sock.set_nonblocking(true)?;
        let bound = sock.local_addr()?.port();
        let now = Instant::now();
        Ok(Stream {
            sock,
            peer: SocketAddr::V4(peer),
            my_id: proto::session_id(local_ip, bound),
            peer_id: 0,
            send_seq: 0,
            tx_buf: VecDeque::with_capacity(timing::TX_BUFFER),
            rx_high: None,
            behind_run: 0,
            missing: BTreeMap::new(),
            ping_seq: 0,
            last_rx: now,
            next_idle: now,
            next_ping: now,
            next_are_you_there: now,
            next_retransmit: now,
            probing: true,
            ping: true,
            idle,
            trace,
            tag,
        })
    }

    /// Bind a socket without connecting it, purely to reserve a port number.
    ///
    /// The stream request has to name our CI-V and audio ports *before* the
    /// radio replies with its own, so those two ports must be reserved while
    /// the control stream is still talking.
    pub(crate) fn reserve_port() -> io::Result<u16> {
        let s = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))?;
        s.local_addr().map(|a| a.port())
    }

    pub(crate) fn my_id(&self) -> u32 {
        self.my_id
    }

    /// Nothing heard for [`timing::STALE_MS`].
    pub(crate) fn is_stale(&self) -> bool {
        self.last_rx.elapsed() >= Duration::from_millis(timing::STALE_MS)
    }

    /// Whether the peer has ever answered on this stream. False on a socket
    /// whose "are you there" probes are going into a void — which is what a
    /// radio still holding an earlier session looks like on its data ports.
    pub(crate) fn handshaken(&self) -> bool {
        !self.probing
    }

    /// How long since anything at all arrived — handshake answers, pings and
    /// idles included, so this reads as time-connected on a fresh stream.
    pub(crate) fn quiet_for(&self) -> Duration {
        self.last_rx.elapsed()
    }

    /// Send without a sequence number: handshake probes and ping replies, which
    /// the peer never asks to have resent.
    pub(crate) fn send_untracked(&self, kind: u16, seq: u16) {
        let p = proto::control(kind, seq, self.my_id, self.peer_id);
        self.write(&p);
    }

    /// Send with the next sequence number, keeping a copy for retransmit.
    pub(crate) fn send_tracked(&mut self, mut pkt: Vec<u8>) {
        pkt[6..8].copy_from_slice(&self.send_seq.to_le_bytes());
        self.write(&pkt);
        if self.tx_buf.len() >= timing::TX_BUFFER {
            self.tx_buf.pop_front();
        }
        self.tx_buf.push_back((self.send_seq, pkt));
        self.send_seq = self.send_seq.wrapping_add(1);
        // Any traffic postpones the idle filler.
        self.next_idle = Instant::now() + Duration::from_millis(timing::IDLE_PERIOD);
        self.trace.sent();
    }

    /// Build a tracked packet with our current ids. Callers that need the ids
    /// (every builder in `protocol`) go through this rather than reading them.
    pub(crate) fn ids(&self) -> (u32, u32) {
        (self.my_id, self.peer_id)
    }

    /// Run the timers. Call once per loop turn.
    pub(crate) fn tick(&mut self) {
        let now = Instant::now();
        if self.probing && now >= self.next_are_you_there {
            self.next_are_you_there = now + Duration::from_millis(timing::ARE_YOU_THERE_PERIOD);
            self.send_untracked(ctrl::ARE_YOU_THERE, 0);
        }
        if !self.probing {
            if self.ping && now >= self.next_ping {
                self.next_ping = now + Duration::from_millis(timing::PING_PERIOD);
                let p = proto::ping(self.ping_seq, false, uptime_ms(), self.my_id, self.peer_id);
                self.write(&p);
            }
            if self.idle && now >= self.next_idle {
                // `send_tracked` pushes this out again; an idle only goes when
                // nothing else has.
                let p = proto::control(ctrl::IDLE, 0, self.my_id, self.peer_id);
                self.send_tracked(p);
            }
            if now >= self.next_retransmit {
                self.next_retransmit = now + Duration::from_millis(timing::RETRANSMIT_PERIOD);
                self.request_missing();
            }
        }
    }

    /// Read one datagram, handling anything the reliability layer owns.
    ///
    /// `out` is filled only when the return value is [`Event::Data`].
    pub(crate) fn poll(&mut self, out: &mut Vec<u8>) -> Event {
        let mut buf = [0u8; 2048];
        let n = match self.sock.recv_from(&mut buf) {
            Ok((n, from)) if from == self.peer => n,
            // A datagram from anywhere else on our ephemeral port is not ours.
            Ok(_) => return Event::Handled,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Event::Idle,
            Err(e) if e.kind() == io::ErrorKind::TimedOut => return Event::Idle,
            Err(e) => {
                tracing::debug!(target: "icomnet", stream = self.tag, error = %e, "recv failed");
                return Event::Idle;
            }
        };
        let pkt = &buf[..n];
        let Some(h) = Header::parse(pkt) else { return Event::Handled };
        self.last_rx = Instant::now();

        match h.kind {
            ctrl::I_AM_HERE if n == proto::CONTROL_SIZE => {
                self.peer_id = h.sent_id;
                self.probing = false;
                self.trace.note(format!("{}: peer is here (id {:08x})", self.tag, h.sent_id));
                self.send_untracked(ctrl::READY, 0x01);
                return Event::IAmHere;
            }
            ctrl::READY if n == proto::CONTROL_SIZE => {
                // The peer may only learn our id here — it answers with its own.
                if self.peer_id == 0 {
                    self.peer_id = h.sent_id;
                }
                self.probing = false;
                return Event::Ready;
            }
            ctrl::DISCONNECT if n == proto::CONTROL_SIZE => {
                self.trace.note(format!("{}: peer disconnected", self.tag));
                return Event::Disconnect;
            }
            ctrl::PING if n == proto::PING_SIZE => {
                if let Some(p) = proto::parse_ping(pkt) {
                    if p.reply {
                        if p.seq == self.ping_seq {
                            self.ping_seq = self.ping_seq.wrapping_add(1);
                        }
                    } else {
                        // Echo the peer's own timestamp back untouched.
                        let r = proto::ping(p.seq, true, p.time, self.my_id, self.peer_id);
                        self.write(&r);
                    }
                }
                return Event::Handled;
            }
            ctrl::RETRANSMIT => {
                self.serve_retransmit(pkt, h);
                return Event::Handled;
            }
            _ => {}
        }

        // Everything else carries a sequence worth tracking. Pings are excluded
        // above; a zero sequence is the protocol's "unsequenced" marker.
        if h.kind == 0 && h.seq != 0 && !self.accept(h.seq) {
            return Event::Handled;
        }
        if n == proto::CONTROL_SIZE {
            // A bare idle: its only job was to carry the sequence.
            return Event::Handled;
        }
        out.clear();
        out.extend_from_slice(pkt);
        Event::Data
    }

    /// Record an incoming sequence. Returns false for a duplicate, which is
    /// what a retransmit we had already recovered from looks like.
    fn accept(&mut self, seq: u16) -> bool {
        let Some(high) = self.rx_high else {
            self.rx_high = Some(seq);
            return true;
        };
        // Signed distance, so a wrap at 0xFFFF reads as +1 and not -65535.
        let ahead = seq.wrapping_sub(high) as i16;
        if ahead > 0 || self.missing.contains_key(&seq) {
            self.behind_run = 0;
        }
        if ahead > 0 {
            if ahead as usize > timing::MAX_MISSING {
                // Not a gap — a restarted peer or a link that dropped a burst
                // far too large to chase. Start again rather than issue
                // thousands of retransmit requests.
                self.trace.note(format!(
                    "{}: sequence jumped {:#06x} → {:#06x}, resynchronising",
                    self.tag, high, seq
                ));
                self.missing.clear();
                self.rx_high = Some(seq);
                return true;
            }
            for i in 1..ahead as u16 {
                self.missing.entry(high.wrapping_add(i)).or_insert(0);
            }
            self.rx_high = Some(seq);
            true
        } else {
            // At or behind the high-water mark: only interesting if we asked
            // for it.
            if self.missing.remove(&seq).is_some() {
                return true;
            }
            // Far behind, and again and again: the peer started its sequence
            // over lower down (a wfview server restarting its audio does,
            // #904). The jump forward above resynchronises; this is its mirror
            // image, and without it every packet was dropped as a duplicate
            // until the new sequence climbed past the old mark — minutes of
            // silence with the CI-V stream still running. One stray old packet
            // is not enough to move the mark.
            if usize::from(ahead.unsigned_abs()) > timing::MAX_MISSING {
                self.behind_run += 1;
                if self.behind_run >= 3 {
                    self.trace.note(format!(
                        "{}: sequence went back {:#06x} → {:#06x}, resynchronising",
                        self.tag, high, seq
                    ));
                    self.behind_run = 0;
                    self.missing.clear();
                    self.rx_high = Some(seq);
                    return true;
                }
            }
            false
        }
    }

    /// Ask the peer for whatever is still missing.
    fn request_missing(&mut self) {
        if self.missing.is_empty() {
            return;
        }
        if self.missing.len() > timing::MAX_MISSING {
            self.trace.note(format!(
                "{}: {} packets missing, giving up on the backlog",
                self.tag,
                self.missing.len()
            ));
            self.missing.clear();
            return;
        }
        let mut wanted = Vec::new();
        self.missing.retain(|&seq, tries| {
            if *tries >= 4 {
                // Asked four times over 400 ms with no answer; the peer no
                // longer has it, and asking forever wedges the window.
                return false;
            }
            *tries += 1;
            wanted.push(seq);
            true
        });
        if wanted.is_empty() {
            return;
        }
        self.trace.retransmit_out(wanted.len());
        let p = if wanted.len() == 1 {
            proto::control(ctrl::RETRANSMIT, wanted[0], self.my_id, self.peer_id)
        } else {
            proto::retransmit_request(&wanted, self.my_id, self.peer_id)
        };
        self.write(&p);
    }

    /// Resend whatever the peer asked for.
    fn serve_retransmit(&mut self, pkt: &[u8], h: Header) {
        self.trace.retransmit_in();
        let mut wanted: Vec<u16> = Vec::new();
        if pkt.len() == proto::CONTROL_SIZE {
            wanted.push(h.seq);
        } else {
            // Pairs of (first, last); the peer sends the same number twice for
            // a single packet, so walking the pair covers both forms.
            let mut at = proto::HEADER;
            while at + 4 <= pkt.len() {
                let first = proto::u16_at(pkt, at);
                let last = proto::u16_at(pkt, at + 2);
                let span = last.wrapping_sub(first);
                if usize::from(span) <= timing::MAX_MISSING {
                    for i in 0..=span {
                        wanted.push(first.wrapping_add(i));
                    }
                }
                at += 4;
            }
        }
        for seq in wanted {
            if let Some((_, data)) = self.tx_buf.iter().find(|(s, _)| *s == seq) {
                let data = data.clone();
                self.write(&data);
            } else {
                // We no longer hold it. An idle bearing that sequence lets the
                // peer close its own gap instead of asking forever.
                let p = proto::control(ctrl::IDLE, seq, self.my_id, self.peer_id);
                self.write(&p);
            }
        }
    }

    /// Tell the peer we are going away. Best effort: this runs on the way out.
    pub(crate) fn disconnect(&self) {
        self.send_untracked(ctrl::DISCONNECT, 0);
    }

    fn write(&self, pkt: &[u8]) {
        if let Err(e) = self.sock.send_to(pkt, self.peer) {
            tracing::debug!(target: "icomnet", stream = self.tag, error = %e, "send failed");
        }
    }
}

/// Milliseconds since this process started talking to radios, wrapped into the
/// field the radio expects. It only has to advance monotonically — the radio
/// echoes it back untouched and never interprets it.
fn uptime_ms() -> u32 {
    use std::sync::OnceLock;
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stream aimed at a socket nobody reads, which is enough to exercise the
    /// sequence bookkeeping without a peer.
    fn a_stream() -> Stream {
        let peer = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 59_999);
        Stream::bind(Ipv4Addr::LOCALHOST, peer, 0, true, Trace::new(), "test").unwrap()
    }

    #[test]
    fn the_first_sequence_is_accepted_and_repeats_are_not() {
        let mut s = a_stream();
        assert!(s.accept(100));
        assert!(!s.accept(100));
        assert!(s.accept(101));
    }

    #[test]
    fn a_gap_is_recorded_and_closed_by_the_retransmission() {
        let mut s = a_stream();
        assert!(s.accept(10));
        assert!(s.accept(14));
        assert_eq!(s.missing.keys().copied().collect::<Vec<_>>(), vec![11, 12, 13]);
        // The recovered packets are delivered, once each.
        assert!(s.accept(12));
        assert!(!s.accept(12));
        assert_eq!(s.missing.keys().copied().collect::<Vec<_>>(), vec![11, 13]);
    }

    #[test]
    fn the_sequence_wraps_without_declaring_65000_packets_lost() {
        let mut s = a_stream();
        assert!(s.accept(0xfffe));
        assert!(s.accept(0x0001));
        // Two packets missing across the wrap, not the whole space.
        let missing: Vec<u16> = s.missing.keys().copied().collect();
        assert_eq!(missing, vec![0x0000, 0xffff], "sorted, so the wrap reads low-first");
    }

    /// A peer that starts its sequence over lower down is followed once it
    /// has done so for a few packets; one stray old packet moves nothing.
    #[test]
    fn a_sequence_that_restarts_lower_is_followed() {
        let mut s = a_stream();
        for q in 3000..3010 {
            assert!(s.accept(q));
        }
        // A single far-behind packet is a stray: dropped, mark kept.
        assert!(!s.accept(100));
        assert!(s.accept(3010));
        assert_eq!(s.rx_high, Some(3010));
        // A run of them is a restart.
        assert!(!s.accept(1));
        assert!(!s.accept(2));
        assert!(s.accept(3));
        assert_eq!(s.rx_high, Some(3));
        assert!(s.accept(4));
        assert!(s.missing.is_empty());
    }

    #[test]
    fn an_enormous_jump_resynchronises_instead_of_chasing_it() {
        let mut s = a_stream();
        assert!(s.accept(1));
        assert!(s.accept(5000));
        assert!(s.missing.is_empty());
        assert_eq!(s.rx_high, Some(5000));
    }

    #[test]
    fn asking_four_times_for_a_packet_nobody_has_gives_up() {
        let mut s = a_stream();
        s.probing = false;
        s.accept(1);
        s.accept(3); // 2 is missing
        for _ in 0..4 {
            s.request_missing();
        }
        assert!(s.missing.contains_key(&2));
        s.request_missing();
        assert!(s.missing.is_empty(), "a packet nobody will resend must not be chased forever");
    }

    #[test]
    fn tracked_packets_are_stamped_and_kept_for_resend() {
        let mut s = a_stream();
        for _ in 0..3 {
            let (a, b) = s.ids();
            s.send_tracked(proto::control(ctrl::IDLE, 0, a, b));
        }
        assert_eq!(s.send_seq, 3);
        let seqs: Vec<u16> = s.tx_buf.iter().map(|(q, _)| *q).collect();
        assert_eq!(seqs, vec![0, 1, 2]);
        // The sequence is stamped into the packet, not just the bookkeeping.
        assert_eq!(proto::u16_at(&s.tx_buf[2].1, 6), 2);
    }

    #[test]
    fn the_resend_buffer_is_bounded() {
        let mut s = a_stream();
        for _ in 0..(timing::TX_BUFFER + 10) {
            let (a, b) = s.ids();
            s.send_tracked(proto::control(ctrl::IDLE, 0, a, b));
        }
        assert_eq!(s.tx_buf.len(), timing::TX_BUFFER);
    }
}
