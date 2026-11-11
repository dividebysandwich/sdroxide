//! The thread that reads the VITA-49 UDP socket.
//!
//! **Why a thread of its own.** The data loop used to read the socket with a 2 ms
//! timeout, because that timeout is also what paces the transmit audio pump. On
//! Windows a receive that times out is cancelled, and a datagram completing at
//! that instant can be lost — Winsock's own documentation says the socket is in
//! an "indeterminate state" after a timeout. On a Hermes-Lite 2 read the same way,
//! a Wireshark capture showed every datagram reaching the network card while the
//! application lost about one in 250, and the loss vanished once the read moved
//! to a blocking thread (issue #553). This is the same read, so it gets the same
//! fix; `sdroxide-hpsdr` has its own copy (`net::start_receiver`) of this code,
//! and the two should change together.
//!
//! Here the only timeout is [`POLL`], long enough that it fires only when the
//! radio has gone quiet — when there is nothing to lose — and exists so the
//! thread notices its stop flag. The data loop waits on the channel instead,
//! whose timeout is the loop's own and touches no socket.
//!
//! Errors are passed on rather than acted on: the loop already knows which ones
//! are an ICMP unreachable for a datagram we sent (not fatal) and which end the
//! session, and it is the one that stops this thread.

use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crossbeam_channel::{Receiver, TrySendError, bounded};

/// What the thread hands to the data loop: a datagram and where it came from,
/// or the error the socket raised.
pub(crate) type Datagram = io::Result<(Vec<u8>, SocketAddr)>;

/// Datagrams held for a loop that has stalled. Past that the loop is gone for
/// good, and the VITA-49 packet counter reports the hole, so a full queue drops.
const QUEUE: usize = 4096;

/// How long the socket waits before the thread looks at its stop flag.
const POLL: Duration = Duration::from_millis(100);

/// Largest datagram taken: comfortably over a 192 kHz IQ packet, and room for a
/// jumbo-frame LAN.
const MAX_DATAGRAM: usize = 16 * 1024;

/// Sets a flag when dropped, so the thread stops however the data loop returns.
pub(crate) struct StopOnDrop(Arc<AtomicBool>);

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

/// Start reading `socket` on a thread of its own. The socket's read timeout is
/// one option for every handle, so from here on it belongs to that thread: the
/// loop must only send on its handle.
pub(crate) fn start(
    socket: &UdpSocket,
    name: &str,
) -> io::Result<(Receiver<Datagram>, StopOnDrop)> {
    let stop = Arc::new(AtomicBool::new(false));
    let guard = StopOnDrop(stop.clone());
    let rx = spawn(socket.try_clone()?, stop, POLL, name)?;
    Ok((rx, guard))
}

fn spawn(
    socket: UdpSocket,
    stop: Arc<AtomicBool>,
    poll: Duration,
    name: &str,
) -> io::Result<Receiver<Datagram>> {
    socket.set_read_timeout(Some(poll))?;
    let (tx, rx) = bounded(QUEUE);
    std::thread::Builder::new().name(name.to_string()).spawn(move || {
        let mut buf = vec![0u8; MAX_DATAGRAM];
        while !stop.load(Ordering::Relaxed) {
            let msg = match socket.recv_from(&mut buf) {
                Ok((n, from)) => Ok((buf[..n].to_vec(), from)),
                Err(ref e)
                    if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) =>
                {
                    continue;
                }
                Err(e) => Err(e),
            };
            let failed = msg.is_err();
            if let Err(TrySendError::Disconnected(_)) = tx.try_send(msg) {
                return;
            }
            if failed {
                // A socket that keeps failing must not spin while the loop
                // decides what the error means.
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    })?;
    Ok(rx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_datagram_arrives_in_order_with_its_source_and_the_thread_stops() {
        let rx_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let addr = rx_socket.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let rx = spawn(rx_socket, stop.clone(), Duration::from_millis(20), "test-rx").unwrap();
        let tx = UdpSocket::bind("127.0.0.1:0").unwrap();
        let from = tx.local_addr().unwrap();
        for seq in 0u32..300 {
            let mut d = vec![0u8; 1032];
            d[4..8].copy_from_slice(&seq.to_be_bytes());
            tx.send_to(&d, addr).unwrap();
            if seq % 50 == 49 {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        for want in 0u32..300 {
            match rx.recv_timeout(Duration::from_secs(2)) {
                Ok(Ok((d, src))) => {
                    assert_eq!(d.len(), 1032);
                    assert_eq!(src, from);
                    assert_eq!(u32::from_be_bytes([d[4], d[5], d[6], d[7]]), want);
                }
                Ok(Err(e)) => panic!("socket error: {e}"),
                Err(e) => panic!("datagram {want} never came: {e}"),
            }
        }
        // A quiet socket is not an error: the poll timeout just comes round.
        assert!(rx.recv_timeout(Duration::from_millis(80)).is_err());
        // And the thread ends once told to, which closes the channel.
        stop.store(true, Ordering::Relaxed);
        assert!(matches!(
            rx.recv_timeout(Duration::from_secs(2)),
            Err(crossbeam_channel::RecvTimeoutError::Disconnected)
        ));
    }
}
