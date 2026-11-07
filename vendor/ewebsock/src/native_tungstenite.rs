//! Native implementation of the WebSocket client using the `tungstenite` crate.

use std::net::TcpStream;
use std::{
    ops::ControlFlow,
    sync::mpsc::{Receiver, TryRecvError},
};

use tungstenite::stream::MaybeTlsStream;
use tungstenite::WebSocket;

use crate::tungstenite_common::into_requester;
use crate::{EventHandler, Options, Result, WsEvent, WsMessage};

/// This is how you send [`WsMessage`]s to the server.
///
/// When the last clone of this is dropped, the connection is closed.
pub struct WsSender {
    tx: Option<std::sync::mpsc::Sender<WsMessage>>,
}

impl Drop for WsSender {
    fn drop(&mut self) {
        self.close();
    }
}

impl WsSender {
    /// Send a message.
    ///
    /// You have to wait for [`WsEvent::Opened`] before you can start sending messages.
    pub fn send(&mut self, msg: WsMessage) {
        if let Some(tx) = &self.tx {
            tx.send(msg).ok();
        }
    }

    /// Close the connection.
    ///
    /// This is called automatically when the sender is dropped.
    pub fn close(&mut self) {
        if self.tx.is_some() {
            log::debug!("Closing WebSocket");
        }
        self.tx = None;
    }

    /// Forget about this sender without closing the connection.
    pub fn forget(mut self) {
        #[allow(clippy::mem_forget)] // intentional
        std::mem::forget(self.tx.take());
    }
}

pub(crate) fn ws_receive_impl(url: String, options: Options, on_event: EventHandler) -> Result<()> {
    std::thread::Builder::new()
        .name("ewebsock".to_owned())
        .spawn(move || {
            if let Err(err) = ws_receiver_blocking(&url, options, &on_event) {
                on_event(WsEvent::Error(err));
            } else {
                log::debug!("WebSocket connection closed.");
            }
        })
        .map_err(|err| format!("Failed to spawn thread: {err}"))?;

    Ok(())
}

/// Connect and call the given event handler on each received event.
///
/// Blocking version of [`crate::ws_receive`], only available on native.
///
/// # Errors
/// All errors are returned to the caller, and NOT reported via `on_event`.
pub fn ws_receiver_blocking(url: &str, options: Options, on_event: &EventHandler) -> Result<()> {
    let uri: tungstenite::http::Uri = url
        .parse()
        .map_err(|err| format!("Failed to parse URL {url:?}: {err}"))?;
    let config = tungstenite::protocol::WebSocketConfig::from(options.clone());
    let max_redirects = 3; // tungstenite default

    let read_timeout = options.read_timeout;
    let (mut socket, response) = match connect(uri, options, config, max_redirects) {
        Ok(result) => result,
        Err(err) => {
            return Err(format!("Connect: {err}"));
        }
    };

    set_read_timeout(&mut socket, read_timeout)?;

    log::debug!("WebSocket HTTP response code: {}", response.status());
    log::trace!(
        "WebSocket response contains the following headers: {:?}",
        response.headers()
    );

    let control = on_event(WsEvent::Opened);
    if control.is_break() {
        log::trace!("Closing connection due to Break");
        return socket
            .close(None)
            .map_err(|err| format!("Failed to close connection: {err}"));
    }

    let mut pending_in_a_row = 0;
    loop {
        let control = read_from_socket(&mut socket, on_event, &mut pending_in_a_row)?;

        if control.is_break() {
            log::trace!("Closing connection due to Break");
            return socket
                .close(None)
                .map_err(|err| format!("Failed to close connection: {err}"));
        }

        std::thread::yield_now();
    }
}

pub(crate) fn ws_connect_impl(
    url: String,
    options: Options,
    on_event: EventHandler,
) -> Result<WsSender> {
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::Builder::new()
        .name("ewebsock".to_owned())
        .spawn(move || {
            if let Err(err) = ws_connect_blocking(&url, options, &on_event, &rx) {
                on_event(WsEvent::Error(err));
            } else {
                log::debug!("WebSocket connection closed.");
            }
        })
        .map_err(|err| format!("Failed to spawn thread: {err}"))?;

    Ok(WsSender { tx: Some(tx) })
}

/// Connect and call the given event handler on each received event.
///
/// This is a blocking variant of [`crate::ws_connect`], only available on native.
///
/// # Errors
/// All errors are returned to the caller, and NOT reported via `on_event`.
pub fn ws_connect_blocking(
    url: &str,
    options: Options,
    on_event: &EventHandler,
    rx: &Receiver<WsMessage>,
) -> Result<()> {
    let config = tungstenite::protocol::WebSocketConfig::from(options.clone());
    let max_redirects = 3; // tungstenite default
    let uri: tungstenite::http::Uri = url
        .parse()
        .map_err(|err| format!("Failed to parse URL {url:?}: {err}"))?;

    let read_timeout = options.read_timeout;
    let (mut socket, response) = match connect(uri, options, config, max_redirects) {
        Ok(result) => result,
        Err(err) => {
            return Err(format!("Connect: {err}"));
        }
    };

    set_read_timeout(&mut socket, read_timeout)?;

    log::debug!("WebSocket HTTP response code: {}", response.status());
    log::trace!(
        "WebSocket response contains the following headers: {:?}",
        response.headers()
    );

    let control = on_event(WsEvent::Opened);
    if control.is_break() {
        log::trace!("Closing connection due to Break");
        return socket
            .close(None)
            .map_err(|err| format!("Failed to close connection: {err}"));
    }

    let mut pending_in_a_row = 0;
    loop {
        match rx.try_recv() {
            Ok(outgoing_message) => {
                let outgoing_message = match outgoing_message {
                    WsMessage::Text(text) => tungstenite::protocol::Message::Text(text),
                    WsMessage::Binary(data) => tungstenite::protocol::Message::Binary(data),
                    WsMessage::Ping(data) => tungstenite::protocol::Message::Ping(data),
                    WsMessage::Pong(data) => tungstenite::protocol::Message::Pong(data),
                    WsMessage::Unknown(_) => panic!("You cannot send WsMessage::Unknown"),
                };
                if let Err(err) = socket.send(outgoing_message) {
                    socket.close(None).ok();
                    socket.flush().ok();
                    return Err(format!("send: {err}"));
                }
            }
            Err(TryRecvError::Disconnected) => {
                log::debug!("WsSender dropped - closing connection.");
                socket.close(None).ok();
                socket.flush().ok();
                return Ok(());
            }
            Err(TryRecvError::Empty) => {}
        };

        let control = read_from_socket(&mut socket, on_event, &mut pending_in_a_row)?;

        if control.is_break() {
            log::trace!("Closing connection due to Break");
            return socket
                .close(None)
                .map_err(|err| format!("Failed to close connection: {err}"));
        }

        std::thread::yield_now();
    }
}

/// Open the socket and run the handshake — tungstenite's own
/// `connect_with_config`, unless [`Options::tls_config`] asks for a TLS
/// configuration of the caller's.
///
/// Added by sdroxide's patch. Without a configuration this is exactly the
/// upstream call, so a connection that does not ask for one is untouched.
fn connect(
    uri: tungstenite::http::Uri,
    options: Options,
    config: tungstenite::protocol::WebSocketConfig,
    max_redirects: u8,
) -> tungstenite::Result<(
    WebSocket<MaybeTlsStream<TcpStream>>,
    tungstenite::handshake::client::Response,
)> {
    #[cfg(feature = "tls")]
    if let Some(tls) = options.tls_config.clone() {
        return connect_with_tls(into_requester(uri, options), config, max_redirects, &tls);
    }
    tungstenite::client::connect_with_config(
        into_requester(uri, options),
        Some(config),
        max_redirects,
    )
}

/// `connect_with_config` with the TLS connector named rather than defaulted,
/// which upstream tungstenite has no blocking entry point for. The same
/// steps in the same order: resolve, try each address, `TCP_NODELAY`, the
/// handshake, and a redirect followed up to `max_redirects` times.
#[cfg(feature = "tls")]
fn connect_with_tls(
    request: tungstenite::client::ClientRequestBuilder,
    config: tungstenite::protocol::WebSocketConfig,
    max_redirects: u8,
    tls: &crate::TlsConfig,
) -> tungstenite::Result<(
    WebSocket<MaybeTlsStream<TcpStream>>,
    tungstenite::handshake::client::Response,
)> {
    use std::net::ToSocketAddrs;
    use tungstenite::client::IntoClientRequest;
    use tungstenite::error::UrlError;
    use tungstenite::handshake::HandshakeError;
    use tungstenite::stream::Mode;
    use tungstenite::Error;

    let (parts, ()) = request.into_client_request()?.into_parts();
    let mut uri = parts.uri.clone();
    for attempt in 0..=max_redirects {
        let mut builder = tungstenite::http::Request::builder()
            .uri(uri.clone())
            .method(parts.method.clone())
            .version(parts.version);
        if let Some(headers) = builder.headers_mut() {
            headers.clone_from(&parts.headers);
        }
        let request = builder.body(())?;

        let mode = tungstenite::client::uri_mode(&uri)?;
        let host = uri.host().ok_or(Error::Url(UrlError::NoHostName))?;
        let host = host
            .strip_prefix('[')
            .and_then(|h| h.strip_suffix(']'))
            .unwrap_or(host);
        let port = uri.port_u16().unwrap_or(match mode {
            Mode::Plain => 80,
            Mode::Tls => 443,
        });
        let stream = (host, port)
            .to_socket_addrs()?
            .find_map(|addr| TcpStream::connect(addr).ok())
            .ok_or_else(|| Error::Url(UrlError::UnableToConnect(uri.to_string())))?;
        stream.set_nodelay(true)?;

        let connector = tungstenite::Connector::Rustls(tls.0.clone());
        match tungstenite::client_tls_with_config(request, stream, Some(config), Some(connector)) {
            Ok(done) => return Ok(done),
            Err(HandshakeError::Failure(Error::Http(res)))
                if res.status().is_redirection() && attempt < max_redirects =>
            {
                let Some(location) = res.headers().get("Location") else {
                    log::warn!("No `Location` found in redirect");
                    return Err(Error::Http(res));
                };
                uri = location.to_str()?.parse()?;
                log::debug!("Redirecting to {uri:?}");
            }
            Err(HandshakeError::Failure(err)) => return Err(err),
            Err(HandshakeError::Interrupted(_)) => {
                return Err(Error::Io(std::io::ErrorKind::WouldBlock.into()));
            }
        }
    }
    unreachable!("the last attempt either returns or is not redirected")
}

/// Windows' `ERROR_IO_PENDING` / `WSA_IO_PENDING`, which Winsock hands back
/// from a *blocking* `recv` whose `SO_RCVTIMEO` expired while the driver still
/// had the request queued. See `is_read_timeout`.
const WSA_IO_PENDING: i32 = 997;

/// Windows' `WSAEINPROGRESS`, the other shape the same condition arrives in.
const WSAEINPROGRESS: i32 = 10_036;

/// How many of those in a row before the socket is treated as genuinely stuck
/// rather than as having timed out. At the default 10 ms read timeout that is
/// about ten seconds of a socket saying nothing but "already in progress",
/// which no live connection does.
const MAX_PENDING_IN_A_ROW: u32 = 1000;

/// Whether an I/O error from `socket.read()` is the read timeout expiring
/// rather than the connection failing.
///
/// `WouldBlock` is the usual answer and `TimedOut` is what Windows may say
/// instead — and, on Windows, so is `ERROR_IO_PENDING` (997). A blocking
/// `recv` with `SO_RCVTIMEO` set is built on an overlapping receive, and when
/// the timeout fires with the request still queued in the AFD driver, the next
/// call reports the operation as already in progress rather than as having
/// timed out. Rust has no `ErrorKind` for that, so it arrives here
/// uncategorised and used to end the connection: a long-lived socket polled a
/// hundred times a second runs for tens of minutes and then dies of it, which
/// is what sdroxide's remote clients saw as `read: IO error: An overlapping
/// I/O operation is in progress.. (os error 997)`. Nothing has failed and the
/// socket reads normally on the next attempt.
fn is_read_timeout(io_err: &std::io::Error) -> bool {
    io_err.kind() == std::io::ErrorKind::WouldBlock
        || io_err.kind() == std::io::ErrorKind::TimedOut
        || matches!(io_err.raw_os_error(), Some(WSA_IO_PENDING | WSAEINPROGRESS))
}

fn read_from_socket(
    socket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    on_event: &EventHandler,
    pending_in_a_row: &mut u32,
) -> Result<ControlFlow<()>> {
    let control = match socket.read() {
        Ok(incoming_msg) => match incoming_msg {
            tungstenite::protocol::Message::Text(text) => {
                on_event(WsEvent::Message(WsMessage::Text(text)))
            }
            tungstenite::protocol::Message::Binary(data) => {
                on_event(WsEvent::Message(WsMessage::Binary(data)))
            }
            tungstenite::protocol::Message::Ping(data) => {
                on_event(WsEvent::Message(WsMessage::Ping(data)))
            }
            tungstenite::protocol::Message::Pong(data) => {
                on_event(WsEvent::Message(WsMessage::Pong(data)))
            }
            tungstenite::protocol::Message::Close(close) => {
                on_event(WsEvent::Closed);
                log::debug!("WebSocket close received: {close:?}");
                ControlFlow::Break(())
            }
            tungstenite::protocol::Message::Frame(_) => ControlFlow::Continue(()),
        },
        // If we get `WouldBlock`, then the read timed out.
        // Windows may emit `TimedOut`, or `ERROR_IO_PENDING`, instead.
        Err(tungstenite::Error::Io(io_err)) if is_read_timeout(&io_err) => {
            // Only the Windows spelling is counted. `WouldBlock` is the
            // ordinary once-per-poll answer of an idle socket and says nothing
            // is wrong; nothing but "already in progress", for ten seconds
            // together, does — and a caller with no keepalive of its own would
            // otherwise wait on a wedged socket for ever instead of being told.
            if matches!(io_err.raw_os_error(), Some(WSA_IO_PENDING | WSAEINPROGRESS)) {
                *pending_in_a_row += 1;
                if *pending_in_a_row >= MAX_PENDING_IN_A_ROW {
                    return Err(format!("read: {io_err}"));
                }
            } else {
                *pending_in_a_row = 0;
            }
            return Ok(ControlFlow::Continue(())); // Ignore
        }
        Err(err) => {
            return Err(format!("read: {err}"));
        }
    };

    *pending_in_a_row = 0;
    Ok(control)
}

fn set_read_timeout(
    s: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    value: Option<std::time::Duration>,
) -> Result<()> {
    // zero timeout is the same as no timeout
    if value.is_none() || value.is_some_and(|value| value.is_zero()) {
        return Ok(());
    }

    match s.get_mut() {
        MaybeTlsStream::Plain(s) => {
            s.set_read_timeout(value)
                .map_err(|err| format!("failed to set read timeout: {err}"))?;
        }
        #[cfg(feature = "tls")]
        MaybeTlsStream::Rustls(s) => {
            s.get_mut()
                .set_read_timeout(value)
                .map_err(|err| format!("failed to set read timeout: {err}"))?;
        }
        _ => {}
    };

    Ok(())
}

/// Added with the classifier above, and the only test added: what it lets
/// through is the whole of the patch, and getting it wrong either restores the
/// dropped connections or hides real ones.
#[test]
fn a_windows_pending_read_counts_as_a_timeout() {
    use std::io::{Error, ErrorKind};
    assert!(is_read_timeout(&Error::from(ErrorKind::WouldBlock)));
    assert!(is_read_timeout(&Error::from(ErrorKind::TimedOut)));
    assert!(is_read_timeout(&Error::from_raw_os_error(WSA_IO_PENDING)));
    assert!(is_read_timeout(&Error::from_raw_os_error(WSAEINPROGRESS)));
    // A connection that has actually failed still fails.
    assert!(!is_read_timeout(&Error::from(ErrorKind::ConnectionReset)));
    assert!(!is_read_timeout(&Error::from(ErrorKind::ConnectionAborted)));
    assert!(!is_read_timeout(&Error::from(ErrorKind::UnexpectedEof)));
    assert!(!is_read_timeout(&Error::from_raw_os_error(10_054))); // WSAECONNRESET
}

#[test]
fn test_connect() {
    let options = crate::Options::default();
    // see documentation for more options
    let (mut sender, _receiver) = crate::connect("ws://example.com", options).unwrap();
    sender.send(crate::WsMessage::Text("Hello!".into()));
}
